use super::m0_uefi::{build_fat_image, build_pe, run_qemu, run_structural_negative_cases, Tools};
use super::m1_bootinfo::{validate_candidate_pin, FORGE_SHA256, THERMITE_COMMIT};
use super::{
    canonical_json, check_forge_skill, direct_verus_command, forge_binary, require_file,
    require_output_fragments, run_checked, run_expect_failure, sha256sum, uefi, workspace_root,
    write_combined_output,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MODEL_SOURCE: &str = "verus/machine-model/uefi_boot_services_gateway.rs";
const CONSUMER_SOURCE: &str = "tests/m1/uefi_boot_services_gateway_consumer.rs";
const LINKER: &str = "kernel-host/link/m1_uefi_boot_services_gateway.ld";
const MODEL_CRATE: &str = "tmk_uefi_boot_services_gateway";
const MODEL_RLIB: &str = "libtmk_uefi_boot_services_gateway.rlib";
const MARKER: &[u8; 20] = b"TMK_M1_UEFI_GATE_OK\n";
const CONSUMER_MARKER: &str = "M1_UEFI_GATEWAY_MODEL_OK bytes=308 scenarios=15 rejected=14 call=get-memory-map args=5 shadow=32";
const ENTRY_SHA256: &str = "31ba989f27b7ca424ffbc214db0a98186781f429327186a17019ee3d08f7353b";

struct ModelBuild {
    rlib: PathBuf,
    consumer: PathBuf,
    entry: PathBuf,
    verification: PathBuf,
}

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let tools = Tools::pinned()?;
    let forge = forge_binary()?;
    validate_candidate_pin(&forge)?;
    check_forge_skill(&forge)?;
    for (relative, label) in [
        (MODEL_SOURCE, "UEFI boot-services gateway Verus model"),
        (CONSUMER_SOURCE, "UEFI boot-services gateway consumer"),
        (LINKER, "UEFI boot-services gateway linker script"),
    ] {
        require_file(&root.join(relative), label)?;
    }
    audit_sources(&root)?;

    let work = root.join("build/m1-uefi-gateway");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;

    let model_dirs = [
        work.join("model-primary"),
        work.join("model-repro-a"),
        work.join("model-repro-b"),
    ];
    let mut models = Vec::new();
    for directory in &model_dirs {
        models.push(build_model(&tools, &root, directory)?);
    }
    let model_sha = same_digest(
        &models
            .iter()
            .map(|model| model.rlib.clone())
            .collect::<Vec<_>>(),
        "UEFI gateway model",
    )?;
    let consumer_sha = same_digest(
        &models
            .iter()
            .map(|model| model.consumer.clone())
            .collect::<Vec<_>>(),
        "UEFI gateway consumer",
    )?;
    let entry_sha = same_digest(
        &models
            .iter()
            .map(|model| model.entry.clone())
            .collect::<Vec<_>>(),
        "UEFI gateway entry",
    )?;
    let verification_sha = same_digest(
        &models
            .iter()
            .map(|model| model.verification.clone())
            .collect::<Vec<_>>(),
        "UEFI gateway verification result",
    )?;
    if entry_sha != ENTRY_SHA256 {
        return Err(format!(
            "UEFI gateway entry digest is {entry_sha}, expected {ENTRY_SHA256}"
        ));
    }
    let entry = fs::read(&models[0].entry)
        .map_err(|error| format!("read registered UEFI gateway: {error}"))?;
    if entry.len() != 308 {
        return Err(format!(
            "registered UEFI gateway is {} bytes, expected 308",
            entry.len()
        ));
    }

    run_verus_negative_cases(&tools, &root, &work)?;

    let pe_dirs = [
        work.join("pe-primary"),
        work.join("pe-repro-a"),
        work.join("pe-repro-b"),
    ];
    let mut pe_paths = Vec::new();
    for directory in &pe_dirs {
        let pe = build_pe(&tools, &root.join(LINKER), &models[0].entry, directory)?;
        let bytes = fs::read(&pe).map_err(|error| format!("read gateway PE: {error}"))?;
        uefi::audit_pe(&bytes, &entry)?;
        pe_paths.push(pe);
    }
    let pe_sha = same_digest(&pe_paths, "UEFI gateway PE")?;
    let pe_bytes = fs::read(&pe_paths[0]).map_err(|error| format!("read primary PE: {error}"))?;
    let pe_audit = uefi::audit_pe(&pe_bytes, &entry)?;
    audit_disassembly(&tools, &pe_paths[0], &work)?;

    let image_dirs = [
        work.join("image-primary"),
        work.join("image-repro-a"),
        work.join("image-repro-b"),
    ];
    let mut images = Vec::new();
    for directory in &image_dirs {
        let image = build_fat_image(&tools, &pe_paths[0], directory)?;
        let bytes = fs::read(&image).map_err(|error| format!("read gateway FAT image: {error}"))?;
        let extracted = uefi::extract_bootx64(&bytes)?;
        if extracted.bytes != pe_bytes {
            return Err("gateway FAT BOOTX64.EFI differs from audited PE".to_string());
        }
        images.push(image);
    }
    let image_sha = same_digest(&images, "UEFI gateway FAT image")?;
    let disk_bytes = fs::read(&images[0]).map_err(|error| format!("read primary FAT: {error}"))?;
    let boot = uefi::extract_bootx64(&disk_bytes)?;

    run_qemu(&tools, &work, &images[0], "tcg", "tcg", MARKER, true)?;
    run_qemu(&tools, &work, &images[0], "kvm", "kvm", MARKER, true)?;
    run_structural_negative_cases(&entry, &pe_bytes, pe_audit, &disk_bytes, &boot, &work)?;
    run_gateway_artifact_negatives(&tools, &root, &work, &entry, &pe_bytes, pe_audit)?;

    let malformed = work.join("malformed-gateway.img");
    let mut malformed_bytes = disk_bytes;
    malformed_bytes[boot.first_data_offset] ^= 0xff;
    fs::write(&malformed, malformed_bytes)
        .map_err(|error| format!("write malformed gateway image: {error}"))?;
    run_qemu(
        &tools,
        &work,
        &malformed,
        "malformed-pe",
        "tcg",
        MARKER,
        false,
    )?;

    let report = format!(
        "M1_UEFI_GATEWAY_OK\ncomponent_verified=true\nrelease_eligible=false\ncandidate_pin_verified=true\nforge_skill_current=true\nhardware_executed=true\nqemu_executed=true\ntcg=true\nkvm=true\nuefi_spec_version=2.11\nefiapi=x86_64\nsystem_table_boot_services_offset=96\nboot_services_get_memory_map_offset=56\nsystem_table_required_bytes=104\nboot_services_required_bytes=64\ncall_frame_bytes=104\nshadow_space_bytes=32\ncall_site_stack_aligned=true\nnonvolatile_registers_preserved=true\nreturn_address_preserved=true\ndereference_footprint_conditional=true\nget_memory_map_called=true\nget_memory_map_arguments=5\ndescriptor_buffer_null=true\nrequired_size_observed=true\nrequired_size_limit=1048576\nexit_boot_services_called=false\nraw_descriptors_decoded=false\nenvironmental_assumption=OVMF-implements-UEFI-2.x-boot-services\nmodel_source_sha256={}\nconsumer_source_sha256={}\nlinker_script_sha256={}\nmodel_artifact_sha256={model_sha}\nverification_result_sha256={verification_sha}\nconsumer_sha256={consumer_sha}\nentry_sha256={entry_sha}\npe_sha256={pe_sha}\nimage_sha256={image_sha}\novmf_code_sha256={}\novmf_vars_sha256={}\nforge_source_identity={THERMITE_COMMIT}\nforge_sha256={FORGE_SHA256}\nverus_verified=16\nmodel_reproducibility_builds=3\nmodel_consumer_executions=3\nentry_reproducibility_emissions=3\npe_reproducibility_builds=3\nimage_reproducibility_builds=3\ntcg_marker=TMK_M1_UEFI_GATE_OK\\n\nkvm_marker=TMK_M1_UEFI_GATE_OK\\n\nnegative_cases=bad-semantics,bad-assume,bad-environment,bad-image,system-pointer,system-signature,system-header,boot-pointer,boot-signature,boot-header,target,status,size-zero,size-large,pe-byte,pe-timestamp,pe-subsystem,fat-byte,fat-path,call-opcode,extra-byte,firmware-malformed-pe\n",
        sha256sum(&root.join(MODEL_SOURCE))?,
        sha256sum(&root.join(CONSUMER_SOURCE))?,
        sha256sum(&root.join(LINKER))?,
        sha256sum(&tools.ovmf_code)?,
        sha256sum(&tools.ovmf_vars)?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M1 UEFI gateway report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn audit_sources(root: &Path) -> Result<(), String> {
    let model = fs::read_to_string(root.join(MODEL_SOURCE))
        .map_err(|error| format!("read gateway model: {error}"))?;
    for forbidden in [
        "assume(",
        "admit(",
        "axiom fn",
        "external_body",
        "unsafe ",
        "asm!",
        "decreases_by",
        "decreases_when",
    ] {
        if model.contains(forbidden) {
            return Err(format!(
                "UEFI gateway model contains forbidden `{forbidden}`"
            ));
        }
    }
    if root.join("scratch/uefi_gateway_probe.S").exists() {
        return Err("temporary UEFI gateway assembly remains in the repository".to_string());
    }
    Ok(())
}

fn build_model(tools: &Tools, root: &Path, directory: &Path) -> Result<ModelBuild, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create model build {}: {error}", directory.display()))?;
    let staged = directory.join("tmk_uefi_boot_services_gateway.rs");
    fs::copy(root.join(MODEL_SOURCE), &staged)
        .map_err(|error| format!("stage gateway model: {error}"))?;
    if sha256sum(&staged)? != sha256sum(&root.join(MODEL_SOURCE))? {
        return Err("staged UEFI gateway model differs from canonical source".to_string());
    }
    let output = run_checked(
        &mut direct_verus_command(
            &tools.verus,
            directory,
            "tmk_uefi_boot_services_gateway.rs",
            true,
        ),
        "verify and compile exact-byte UEFI gateway model",
    )?;
    require_output_fragments(
        &output.stdout,
        "UEFI gateway Verus result",
        &[
            "\"success\": true",
            "\"verified\": 16",
            "\"errors\": 0",
            "\"version\": \"0.2026.05.24.ecee80a\"",
            "\"toolchain\": \"1.95.0-x86_64-unknown-linux-gnu\"",
        ],
    )?;
    let verification = directory.join("verus-result.json");
    fs::write(
        &verification,
        canonical_json(&output.stdout, "UEFI gateway Verus result")?,
    )
    .map_err(|error| format!("write gateway verification result: {error}"))?;
    let rlib = directory.join(MODEL_RLIB);
    require_file(&rlib, "compiled UEFI gateway model")?;
    let consumer = directory.join("uefi-gateway-consumer");
    run_checked(
        Command::new(&tools.rustc)
            .current_dir(root)
            .args(["--edition=2021"])
            .arg(root.join(CONSUMER_SOURCE))
            .arg("--extern")
            .arg(format!("{MODEL_CRATE}={}", rlib.display()))
            .arg("-L")
            .arg("dependency=/opt/verus/0.2026.05.24.ecee80a")
            .args(["-C", "panic=abort"])
            .arg("-o")
            .arg(&consumer),
        "compile UEFI gateway model consumer",
    )?;
    let entry = directory.join("gateway.bin");
    let runtime = run_checked(
        Command::new(&consumer).current_dir(root).arg(&entry),
        "execute UEFI gateway model consumer",
    )?;
    require_output_fragments(
        &runtime.stdout,
        "UEFI gateway consumer runtime",
        &[CONSUMER_MARKER],
    )?;
    write_combined_output(
        &directory.join("consumer-runtime.txt"),
        &runtime,
        "gateway consumer",
    )?;
    Ok(ModelBuild {
        rlib,
        consumer,
        entry,
        verification,
    })
}

fn run_verus_negative_cases(tools: &Tools, root: &Path, work: &Path) -> Result<(), String> {
    let canonical = fs::read_to_string(root.join(MODEL_SOURCE))
        .map_err(|error| format!("read gateway model for negatives: {error}"))?;
    let bad_semantics = canonical.replacen(
        "rax: if success { 0 } else { EFI_LOAD_ERROR },",
        "rax: if success { 1 } else { EFI_LOAD_ERROR },",
        1,
    );
    if bad_semantics == canonical {
        return Err("gateway semantic mutation target was not found".to_string());
    }
    fs::write(work.join("bad-semantics.rs"), bad_semantics)
        .map_err(|error| format!("write gateway semantic mutation: {error}"))?;
    let output = run_expect_failure(
        &mut direct_verus_command(&tools.verus, work, "bad-semantics.rs", false),
        "Verus rejects gateway result mutation",
    )?;
    let diagnostics = combined_output(&output);
    require_output_fragments(
        &diagnostics,
        "gateway semantic rejection",
        &["postcondition not satisfied"],
    )?;
    fs::write(work.join("bad-semantics-result.txt"), diagnostics)
        .map_err(|error| format!("write gateway semantic rejection: {error}"))?;

    let bad_assume = canonical.replacen(
        "    if image_is_registered(&image)",
        "    assume(false);\n    if image_is_registered(&image)",
        1,
    );
    if bad_assume == canonical {
        return Err("gateway assume mutation target was not found".to_string());
    }
    fs::write(work.join("bad-assume.rs"), bad_assume)
        .map_err(|error| format!("write gateway assume mutation: {error}"))?;
    let output = run_expect_failure(
        &mut direct_verus_command(&tools.verus, work, "bad-assume.rs", false),
        "Verus no-cheating rejects gateway assume",
    )?;
    let diagnostics = combined_output(&output);
    require_output_fragments(
        &diagnostics,
        "gateway assume rejection",
        &["assume/admit not allowed with --no-cheating"],
    )?;
    fs::write(work.join("bad-assume-result.txt"), diagnostics)
        .map_err(|error| format!("write gateway assume rejection: {error}"))?;
    Ok(())
}

fn audit_disassembly(tools: &Tools, pe: &Path, work: &Path) -> Result<(), String> {
    let headers = run_checked(
        Command::new(&tools.objdump)
            .args(["-f", "-p", "-h"])
            .arg(pe),
        "audit gateway PE headers",
    )?;
    require_output_fragments(
        &headers.stdout,
        "gateway PE headers",
        &[
            "file format pei-x86-64",
            "EFI application",
            ".text",
            "00000134",
        ],
    )?;
    fs::write(work.join("pe-headers.txt"), &headers.stdout)
        .map_err(|error| format!("write gateway PE headers: {error}"))?;
    let disassembly = run_checked(
        Command::new(&tools.objdump).arg("-d").arg(pe),
        "disassemble exact UEFI gateway",
    )?;
    require_output_fragments(
        &disassembly.stdout,
        "UEFI gateway disassembly",
        &[
            "mov    0x60(%rdx),%r11",
            "mov    0x38(%r11),%r10",
            "sub    $0x68,%rsp",
            "mov    %rax,0x20(%rsp)",
            "call   *%r10",
            "movabs $0x8000000000000005,%r10",
            "cmp    $0x100000,%rax",
            "add    $0x68,%rsp",
            "movabs $0x8000000000000001,%rax",
        ],
    )?;
    fs::write(work.join("gateway-disassembly.txt"), &disassembly.stdout)
        .map_err(|error| format!("write gateway disassembly: {error}"))?;
    Ok(())
}

fn run_gateway_artifact_negatives(
    tools: &Tools,
    root: &Path,
    work: &Path,
    entry: &[u8],
    pe: &[u8],
    audit: uefi::PeAudit,
) -> Result<(), String> {
    let mut results = String::new();
    let mut bad_call = pe.to_vec();
    bad_call[audit.text_file_offset + 0xba] ^= 1;
    let diagnostic = uefi::audit_pe(&bad_call, entry)
        .err()
        .ok_or_else(|| "mutated gateway call opcode unexpectedly passed".to_string())?;
    if !diagnostic.contains("PE .text bytes differ") {
        return Err(format!(
            "gateway call mutation diagnostic is `{diagnostic}`"
        ));
    }
    results.push_str(&format!("call-opcode: {diagnostic}\n"));

    let extra = work.join("extra-byte.bin");
    let mut extra_bytes = entry.to_vec();
    extra_bytes.push(0xcc);
    fs::write(&extra, extra_bytes).map_err(|error| format!("write extra-byte gateway: {error}"))?;
    let result = build_pe(
        tools,
        &root.join(LINKER),
        &extra,
        &work.join("extra-byte-pe"),
    );
    let diagnostic = result
        .err()
        .ok_or_else(|| "extra-byte gateway unexpectedly linked".to_string())?;
    if !diagnostic.contains("link deterministic minimal UEFI PE image") {
        return Err(format!("extra-byte gateway diagnostic is `{diagnostic}`"));
    }
    results.push_str("extra-byte: linker size assertion rejected 309-byte gateway\n");
    fs::write(work.join("gateway-artifact-negative-results.txt"), results)
        .map_err(|error| format!("write gateway artifact negatives: {error}"))?;
    Ok(())
}

fn same_digest(paths: &[PathBuf], label: &str) -> Result<String, String> {
    let first = paths
        .first()
        .ok_or_else(|| format!("no {label} paths were supplied"))?;
    let expected = sha256sum(first)?;
    for path in paths.iter().skip(1) {
        let actual = sha256sum(path)?;
        if actual != expected {
            return Err(format!(
                "{label} {} digest is {actual}, expected {expected}",
                path.display()
            ));
        }
    }
    Ok(expected)
}

fn combined_output(output: &Output) -> Vec<u8> {
    let mut bytes = output.stdout.clone();
    bytes.extend_from_slice(&output.stderr);
    bytes
}
