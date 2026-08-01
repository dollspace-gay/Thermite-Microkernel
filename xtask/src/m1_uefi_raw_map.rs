use super::m0_uefi::{build_fat_image, build_pe, run_qemu, run_structural_negative_cases, Tools};
use super::m1_bootinfo::{validate_candidate_pin, FORGE_SHA256, THERMITE_COMMIT};
use super::{
    canonical_json, check_forge_skill, forge_binary, require_file, require_output_fragments,
    run_checked, run_expect_failure, sha256sum, uefi, workspace_root, write_combined_output,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MODEL_SOURCE: &str = "verus/machine-model/uefi_raw_memory_map_capsule.rs";
const CONSUMER_SOURCE: &str = "tests/m1/uefi_raw_memory_map_capsule_consumer.rs";
const LINKER: &str = "kernel-host/link/m1_uefi_raw_memory_map_capsule.ld";
const MODEL_CRATE: &str = "tmk_uefi_raw_memory_map_capsule";
const MODEL_RLIB: &str = "libtmk_uefi_raw_memory_map_capsule.rlib";
const MARKER: &[u8; 11] = b"TMK_MAP_OK\n";
const CONSUMER_MARKER: &str =
    "M1_UEFI_RAW_MAP_MODEL_OK bytes=1016 scenarios=33 rejected=32 calls=4 descriptors=4 free=all-paths";
const ENTRY_SHA256: &str = "2d6649e99a08d6c561eb26f3003d9e2f16fa9bf29190214646c1ece0e6ab9278";

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
        (MODEL_SOURCE, "UEFI raw-memory-map Verus model"),
        (CONSUMER_SOURCE, "UEFI raw-memory-map model consumer"),
        (LINKER, "UEFI raw-memory-map linker script"),
    ] {
        require_file(&root.join(relative), label)?;
    }
    audit_sources(&root)?;

    let work = root.join("build/m1-uefi-raw-map");
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
        "UEFI raw-map model",
    )?;
    let consumer_sha = same_digest(
        &models
            .iter()
            .map(|model| model.consumer.clone())
            .collect::<Vec<_>>(),
        "UEFI raw-map consumer",
    )?;
    let entry_sha = same_digest(
        &models
            .iter()
            .map(|model| model.entry.clone())
            .collect::<Vec<_>>(),
        "UEFI raw-map entry",
    )?;
    let verification_sha = same_digest(
        &models
            .iter()
            .map(|model| model.verification.clone())
            .collect::<Vec<_>>(),
        "UEFI raw-map verification result",
    )?;
    if entry_sha != ENTRY_SHA256 {
        return Err(format!(
            "UEFI raw-map entry digest is {entry_sha}, expected {ENTRY_SHA256}"
        ));
    }
    let entry = fs::read(&models[0].entry)
        .map_err(|error| format!("read registered UEFI raw-map entry: {error}"))?;
    if entry.len() != 1016 {
        return Err(format!(
            "registered UEFI raw-map entry is {} bytes, expected 1016",
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
        let bytes = fs::read(&pe).map_err(|error| format!("read raw-map PE: {error}"))?;
        uefi::audit_pe(&bytes, &entry)?;
        pe_paths.push(pe);
    }
    let pe_sha = same_digest(&pe_paths, "UEFI raw-map PE")?;
    let pe_bytes = fs::read(&pe_paths[0]).map_err(|error| format!("read primary PE: {error}"))?;
    let pe_audit = uefi::audit_pe(&pe_bytes, &entry)?;
    if pe_bytes.len() != 1536 {
        return Err(format!(
            "raw-map PE is {} bytes, expected 1536",
            pe_bytes.len()
        ));
    }
    audit_disassembly(&tools, &pe_paths[0], &work)?;

    let image_dirs = [
        work.join("image-primary"),
        work.join("image-repro-a"),
        work.join("image-repro-b"),
    ];
    let mut images = Vec::new();
    for directory in &image_dirs {
        let image = build_fat_image(&tools, &pe_paths[0], directory)?;
        let bytes = fs::read(&image).map_err(|error| format!("read raw-map FAT image: {error}"))?;
        let extracted = uefi::extract_bootx64(&bytes)?;
        if extracted.bytes != pe_bytes {
            return Err("raw-map FAT BOOTX64.EFI differs from audited PE".to_string());
        }
        images.push(image);
    }
    let image_sha = same_digest(&images, "UEFI raw-map FAT image")?;
    let disk_bytes = fs::read(&images[0]).map_err(|error| format!("read primary FAT: {error}"))?;
    let boot = uefi::extract_bootx64(&disk_bytes)?;

    run_qemu(&tools, &work, &images[0], "tcg", "tcg", MARKER, true)?;
    run_qemu(&tools, &work, &images[0], "kvm", "kvm", MARKER, true)?;
    run_structural_negative_cases(&entry, &pe_bytes, pe_audit, &disk_bytes, &boot, &work)?;
    run_artifact_negatives(&tools, &root, &work, &entry, &pe_bytes, pe_audit)?;

    let malformed = work.join("malformed-raw-map.img");
    let mut malformed_bytes = disk_bytes;
    malformed_bytes[boot.first_data_offset] ^= 0xff;
    fs::write(&malformed, malformed_bytes)
        .map_err(|error| format!("write malformed raw-map image: {error}"))?;
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
        "M1_UEFI_RAW_MAP_OK\ncomponent_verified=true\nrelease_eligible=false\ncandidate_pin_verified=true\nforge_skill_current=true\nhardware_executed=true\nqemu_executed=true\ntcg=true\nkvm=true\nuefi_spec_version=2.11\nefiapi=x86_64\nsystem_table_boot_services_offset=96\nboot_services_get_memory_map_offset=56\nboot_services_allocate_pool_offset=64\nboot_services_free_pool_offset=72\nsystem_table_required_bytes=104\nboot_services_required_bytes=80\ncall_frame_bytes=168\nshadow_space_bytes=32\ncall_site_stack_aligned=true\nnonvolatile_registers_preserved=true\nreturn_address_preserved=true\ndereference_footprint_conditional=true\nprobe_get_memory_map_called=true\nallocate_pool_called=true\nallocation_pool_type=EfiLoaderData\nallocation_margin=512\nsecond_get_memory_map_called=true\nraw_descriptors_scanned=true\nfree_pool_all_post_allocation_paths=true\nmap_key_observed=true\nmap_key_retained=false\nexit_boot_services_called=false\nrequired_size_limit=1048064\nallocated_size_limit=1048576\ndescriptor_size_min=40\ndescriptor_size_max=256\ndescriptor_count_limit=4096\nuefi_unaccepted_memory_type=reserved\nruntime_mmio_accepted=true\nenvironmental_assumption=OVMF-implements-UEFI-2.x-boot-services\nmodel_source_sha256={}\nconsumer_source_sha256={}\nlinker_script_sha256={}\nauditor_sha256={}\nmodel_artifact_sha256={model_sha}\nverification_result_sha256={verification_sha}\nconsumer_sha256={consumer_sha}\nentry_sha256={entry_sha}\npe_sha256={pe_sha}\nimage_sha256={image_sha}\novmf_code_sha256={}\novmf_vars_sha256={}\nforge_source_identity={THERMITE_COMMIT}\nforge_sha256={FORGE_SHA256}\nverus_verified=21\nmodel_reproducibility_builds=3\nmodel_consumer_executions=3\nentry_reproducibility_emissions=3\npe_reproducibility_builds=3\nimage_reproducibility_builds=3\ntcg_marker=TMK_MAP_OK\\n\nkvm_marker=TMK_MAP_OK\\n\nnegative_cases=bad-semantics,bad-assume,bad-image,bad-environment,pe-byte,pe-timestamp,pe-subsystem,fat-byte,fat-path,probe-call-byte,allocate-call-byte,second-call-byte,free-call-byte,extra-byte,firmware-malformed-pe\n",
        sha256sum(&root.join(MODEL_SOURCE))?,
        sha256sum(&root.join(CONSUMER_SOURCE))?,
        sha256sum(&root.join(LINKER))?,
        sha256sum(&root.join("xtask/src/uefi.rs"))?,
        sha256sum(&tools.ovmf_code)?,
        sha256sum(&tools.ovmf_vars)?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M1 UEFI raw-map report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn audit_sources(root: &Path) -> Result<(), String> {
    let model = fs::read_to_string(root.join(MODEL_SOURCE))
        .map_err(|error| format!("read raw-map model: {error}"))?;
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
                "UEFI raw-map model contains forbidden `{forbidden}`"
            ));
        }
    }
    for temporary in [
        "scratch/uefi_raw_map_probe.S",
        "scratch/uefi_raw_map_probe.ld",
    ] {
        if root.join(temporary).exists() {
            return Err(format!("temporary derivation source {temporary} remains"));
        }
    }
    Ok(())
}

fn build_model(tools: &Tools, root: &Path, directory: &Path) -> Result<ModelBuild, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create model build {}: {error}", directory.display()))?;
    let staged = directory.join("tmk_uefi_raw_memory_map_capsule.rs");
    fs::copy(root.join(MODEL_SOURCE), &staged)
        .map_err(|error| format!("stage UEFI raw-map model: {error}"))?;
    if sha256sum(&staged)? != sha256sum(&root.join(MODEL_SOURCE))? {
        return Err("staged UEFI raw-map model differs from canonical source".to_string());
    }
    let output = run_checked(
        &mut raw_map_verus_command(
            &tools.verus,
            directory,
            "tmk_uefi_raw_memory_map_capsule.rs",
            true,
        ),
        "verify and compile exact-byte UEFI raw-map model",
    )?;
    require_output_fragments(
        &output.stdout,
        "UEFI raw-map Verus result",
        &[
            "\"success\": true",
            "\"verified\": 21",
            "\"errors\": 0",
            "\"version\": \"0.2026.05.24.ecee80a\"",
            "\"toolchain\": \"1.95.0-x86_64-unknown-linux-gnu\"",
        ],
    )?;
    let verification = directory.join("verus-result.json");
    fs::write(
        &verification,
        canonical_json(&output.stdout, "UEFI raw-map Verus result")?,
    )
    .map_err(|error| format!("write raw-map verification result: {error}"))?;
    let rlib = directory.join(MODEL_RLIB);
    require_file(&rlib, "compiled UEFI raw-map model")?;
    let consumer = directory.join("uefi-raw-map-consumer");
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
            .args(["-C", "relocation-model=static"])
            .arg("-o")
            .arg(&consumer),
        "compile UEFI raw-map model consumer",
    )?;
    let entry = directory.join("raw-map.bin");
    let runtime = run_checked(
        Command::new(&consumer).current_dir(root).arg(&entry),
        "execute UEFI raw-map model consumer",
    )?;
    require_output_fragments(&runtime.stdout, "UEFI raw-map consumer", &[CONSUMER_MARKER])?;
    write_combined_output(
        &directory.join("consumer-runtime.txt"),
        &runtime,
        "UEFI raw-map consumer",
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
        .map_err(|error| format!("read raw-map model for negatives: {error}"))?;
    let bad_semantics = canonical.replacen(
        "marker_bytes: if success { 11 } else { 0 },",
        "marker_bytes: if success { 10 } else { 0 },",
        1,
    );
    if bad_semantics == canonical {
        return Err("raw-map semantic mutation target was not found".to_string());
    }
    fs::write(work.join("bad-semantics.rs"), bad_semantics)
        .map_err(|error| format!("write raw-map semantic mutation: {error}"))?;
    let output = run_expect_failure(
        &mut raw_map_verus_command(&tools.verus, work, "bad-semantics.rs", false),
        "Verus rejects raw-map result mutation",
    )?;
    let diagnostics = combined_output(&output);
    require_output_fragments(
        &diagnostics,
        "raw-map semantic rejection",
        &["postcondition not satisfied"],
    )?;
    fs::write(work.join("bad-semantics-result.txt"), diagnostics)
        .map_err(|error| format!("write raw-map semantic rejection: {error}"))?;

    let bad_assume = canonical.replacen(
        "    if raw_map_image_is_registered(&image)",
        "    assume(false);\n    if raw_map_image_is_registered(&image)",
        1,
    );
    if bad_assume == canonical {
        return Err("raw-map assume mutation target was not found".to_string());
    }
    fs::write(work.join("bad-assume.rs"), bad_assume)
        .map_err(|error| format!("write raw-map assume mutation: {error}"))?;
    let output = run_expect_failure(
        &mut raw_map_verus_command(&tools.verus, work, "bad-assume.rs", false),
        "Verus no-cheating rejects raw-map assume",
    )?;
    let diagnostics = combined_output(&output);
    require_output_fragments(
        &diagnostics,
        "raw-map assume rejection",
        &["assume/admit not allowed with --no-cheating"],
    )?;
    fs::write(work.join("bad-assume-result.txt"), diagnostics)
        .map_err(|error| format!("write raw-map assume rejection: {error}"))?;
    Ok(())
}

fn audit_disassembly(tools: &Tools, pe: &Path, work: &Path) -> Result<(), String> {
    let headers = run_checked(
        Command::new(&tools.objdump)
            .args(["-f", "-p", "-h"])
            .arg(pe),
        "audit raw-map PE headers",
    )?;
    require_output_fragments(
        &headers.stdout,
        "raw-map PE headers",
        &[
            "file format pei-x86-64",
            "EFI application",
            ".text",
            "000003f8",
        ],
    )?;
    fs::write(work.join("pe-headers.txt"), &headers.stdout)
        .map_err(|error| format!("write raw-map PE headers: {error}"))?;
    let disassembly = run_checked(
        Command::new(&tools.objdump).arg("-d").arg(pe),
        "disassemble exact UEFI raw-map capsule",
    )?;
    require_output_fragments(
        &disassembly.stdout,
        "UEFI raw-map disassembly",
        &[
            "sub    $0xa8,%rsp",
            "call   *0x38(%r11)",
            "call   *0x40(%r11)",
            "div    %rcx",
            "cmp    $0xf,%ecx",
            "call   *0x48(%r11)",
            "add    $0xa8,%rsp",
            "movabs $0x8000000000000001,%rax",
        ],
    )?;
    fs::write(work.join("raw-map-disassembly.txt"), &disassembly.stdout)
        .map_err(|error| format!("write raw-map disassembly: {error}"))?;
    Ok(())
}

fn run_artifact_negatives(
    tools: &Tools,
    root: &Path,
    work: &Path,
    entry: &[u8],
    pe: &[u8],
    audit: uefi::PeAudit,
) -> Result<(), String> {
    let mut results = String::new();
    for (name, offset) in [
        ("probe-call-byte", 0xdfusize),
        ("allocate-call-byte", 0x12f),
        ("second-call-byte", 0x1a6),
        ("free-call-byte", 0x3a3),
    ] {
        let mut bad = pe.to_vec();
        bad[audit.text_file_offset + offset] ^= 1;
        let diagnostic = uefi::audit_pe(&bad, entry)
            .err()
            .ok_or_else(|| format!("{name} unexpectedly passed"))?;
        if !diagnostic.contains("PE .text bytes differ") {
            return Err(format!("{name} diagnostic is `{diagnostic}`"));
        }
        results.push_str(&format!("{name}: {diagnostic}\n"));
    }
    let extra = work.join("extra-byte.bin");
    let mut extra_bytes = entry.to_vec();
    extra_bytes.push(0xcc);
    fs::write(&extra, extra_bytes)
        .map_err(|error| format!("write extra-byte raw-map entry: {error}"))?;
    let diagnostic = build_pe(
        tools,
        &root.join(LINKER),
        &extra,
        &work.join("extra-byte-pe"),
    )
    .err()
    .ok_or_else(|| "extra-byte raw-map entry unexpectedly linked".to_string())?;
    if !diagnostic.contains("link deterministic minimal UEFI PE image") {
        return Err(format!("extra-byte raw-map diagnostic is `{diagnostic}`"));
    }
    results.push_str("extra-byte: linker size assertion rejected 1017-byte entry\n");
    fs::write(work.join("raw-map-artifact-negative-results.txt"), results)
        .map_err(|error| format!("write raw-map artifact negatives: {error}"))?;
    Ok(())
}

fn same_digest(paths: &[PathBuf], label: &str) -> Result<String, String> {
    let first = paths
        .first()
        .ok_or_else(|| format!("no {label} paths supplied"))?;
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

fn raw_map_verus_command(verus: &Path, work: &Path, source_name: &str, compile: bool) -> Command {
    let mut command = Command::new(verus);
    command
        .current_dir(work)
        .env("SOURCE_DATE_EPOCH", "0")
        .args(["--output-json", "--no-cheating"]);
    if compile {
        command.arg("--compile");
    }
    command
        .args(["--rlimit", "20"])
        .args(["--smt-option", "smt.random_seed=1"])
        .args(["-C", "panic=abort"])
        .args(["-C", "overflow-checks=off"])
        .args(["-C", "relocation-model=static"])
        .args(["-C", "no-redzone=yes"])
        .arg(format!("--remap-path-prefix={}=.", work.display()))
        .arg(source_name);
    command
}
