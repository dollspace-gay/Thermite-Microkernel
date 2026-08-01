use super::m1_elf::{compare_file, read_json, require_file, verify_bundle, write_output};
use super::{
    check_forge_skill, copy_tree, forge_binary, json_string, require_output_fragments, run_checked,
    run_expect_failure, sha256sum, workspace_root,
};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE: &str = "thermite/boot/boot_policy.th";
const SHELL: &str = "tests/m1/bootinfo_shell.rs";
const CONSUMER: &str = "tests/m1/bootinfo_consumer.rs";
const FREESTANDING: &str = "tests/m1/bootinfo_freestanding.rs";
const CRATE_NAME: &str = "tmk_bootinfo";
const ARTIFACT: &str = "artifact/libtmk_bootinfo.rlib";
const RECEIPT_SCHEMA: &str = "thermite.verified-composition-receipt.v1";
pub(super) const THERMITE_COMMIT: &str = "1fb0a799071d35493815ba99b9ca26af9a22eb1c";
pub(super) const FORGE_SHA256: &str =
    "12240457546220ebefba7c7a5e3ab2d127acaf9b592543a8d0394bf0c8253b74";
const RUNTIME_MARKER: &str = "M1_BOOTINFO_OK ranges=2 last=0000000000a00000 bsp=7 negatives=12";

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let forge = forge_binary()?;
    validate_candidate_pin(&forge)?;
    check_forge_skill(&forge)?;
    for (relative, label) in [
        (SOURCE, "BootInfo Thermite policy"),
        (SHELL, "BootInfo direct-Verus decoder"),
        (CONSUMER, "BootInfo runtime consumer"),
        (FREESTANDING, "BootInfo freestanding consumer"),
    ] {
        require_file(&root.join(relative), label)?;
    }

    let work = root.join("build/m1-bootinfo");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;
    run_source_assurance(&forge, &root, &work)?;

    let bundles = [
        work.join("primary.verified"),
        work.join("repro-a.verified"),
        work.join("repro-b.verified"),
    ];
    for (index, bundle) in bundles.iter().enumerate() {
        let output = run_checked(
            &mut build_command(&forge, &root, &root.join(SHELL), bundle),
            &format!("M1 BootInfo composition build {}", index + 1),
        )?;
        write_output(
            &work.join(format!("build-{}.txt", index + 1)),
            &output,
            "M1 BootInfo build evidence",
        )?;
    }

    let primary = &bundles[0];
    let receipt = read_json(&primary.join("receipt.json"), "M1 BootInfo receipt")?;
    validate_receipt(&root, primary, &receipt)?;
    let binding_sha = json_string(&receipt, "/binding_sha256", "BootInfo binding")?.to_string();
    let artifact_sha =
        json_string(&receipt, "/binding/artifact/sha256", "BootInfo artifact")?.to_string();
    let combined_source_sha = json_string(
        &receipt,
        "/binding/composition/combined_source_sha256",
        "BootInfo combined source",
    )?
    .to_string();

    let validation = verify_bundle(&forge, &root, primary, false)?;
    let replay = verify_bundle(&forge, &root, primary, true)?;
    fs::write(
        work.join("verify.json"),
        serde_json::to_vec_pretty(&validation)
            .map_err(|error| format!("serialize BootInfo validation: {error}"))?,
    )
    .map_err(|error| format!("write BootInfo validation: {error}"))?;
    fs::write(
        work.join("replay.json"),
        serde_json::to_vec_pretty(&replay)
            .map_err(|error| format!("serialize BootInfo replay: {error}"))?,
    )
    .map_err(|error| format!("write BootInfo replay: {error}"))?;
    for (report, replayed, label) in [
        (&validation, false, "validation"),
        (&replay, true, "replay"),
    ] {
        if report.get("replayed").and_then(Value::as_bool) != Some(replayed)
            || json_string(report, "/binding_sha256", label)? != binding_sha
            || json_string(report, "/artifact_sha256", label)? != artifact_sha
        {
            return Err(format!(
                "M1 BootInfo {label} does not match the accepted receipt"
            ));
        }
    }

    let reproducible = [
        "receipt.json",
        "evidence/source.verus.rs",
        ARTIFACT,
        "artifact/deps/libvstd.rlib",
        "evidence/kernel-vstd-link.rs",
    ];
    for relative in reproducible {
        let expected = fs::read(primary.join(relative))
            .map_err(|error| format!("read primary BootInfo `{relative}`: {error}"))?;
        for bundle in bundles.iter().skip(1) {
            compare_file(
                bundle,
                relative,
                &expected,
                "BootInfo reproducible artifact",
            )?;
        }
    }
    for bundle in bundles.iter().skip(1) {
        verify_bundle(&forge, &root, bundle, false)?;
    }

    let toolchain = read_json(
        &primary.join("evidence/toolchain.json"),
        "BootInfo toolchain evidence",
    )?;
    let rustc = PathBuf::from(json_string(
        &toolchain,
        "/artifact_codegen/rustc_path",
        "BootInfo codegen rustc",
    )?);
    require_file(&rustc, "BootInfo codegen rustc")?;
    let vstd = validate_kernel_vstd(primary, &toolchain)?;
    let consumer_sha = run_consumer(&root, &work, primary, &rustc)?;
    let (freestanding_rlib_sha, freestanding_elf_sha) =
        run_freestanding(&root, &work, primary, &rustc)?;
    run_negative_builds(&forge, &root, &work)?;
    run_bundle_tamper_negatives(&forge, &root, &work, primary)?;

    let report = format!(
        "M1_BOOTINFO_OK\ncomponent_verified=true\nrelease_eligible=false\ncandidate_pin_verified=true\nreceipt_validated=true\nreceipt_replayed=true\nsource_sha256={}\nshell_sha256={}\nconsumer_source_sha256={}\nfreestanding_source_sha256={}\ncombined_source_sha256={combined_source_sha}\nreceipt_sha256={}\nbinding_sha256={binding_sha}\nartifact_sha256={artifact_sha}\nconsumer_sha256={consumer_sha}\nfreestanding_rlib_sha256={freestanding_rlib_sha}\nfreestanding_elf_sha256={freestanding_elf_sha}\nkernel_vstd_vir_sha256={}\nkernel_vstd_source_sha256={}\nkernel_vstd_link_source_sha256={}\nkernel_vstd_link_rlib_sha256={}\nforge_source_identity={}\nreproducibility_builds=3\nverified_success_contract=header,checksum,digests,framebuffer,map-bounds,range-content,range-order,reserved-zero,last-end,bsp-apic-id\nfreestanding_links=rlib,elf64-x86-64\nruntime_marker={RUNTIME_MARKER}\nruntime_negative_cases=12\nnegative_cases=wrong-byte-model,map-overrun,reserved-tail-omission,receipt-tamper,vstd-source-tamper,vstd-rlib-tamper\n",
        sha256sum(&root.join(SOURCE))?,
        sha256sum(&root.join(SHELL))?,
        sha256sum(&root.join(CONSUMER))?,
        sha256sum(&root.join(FREESTANDING))?,
        sha256sum(&primary.join("receipt.json"))?,
        vstd.vir_sha,
        vstd.source_sha,
        vstd.link_source_sha,
        vstd.link_rlib_sha,
        json_string(&toolchain, "/forge_source_identity", "Forge source identity")?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M1 BootInfo report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn build_command(forge: &Path, root: &Path, shell: &Path, out: &Path) -> Command {
    let mut command = Command::new(forge);
    command
        .current_dir(root)
        .arg("build")
        .arg(SOURCE)
        .args(["--level", "l3", "--compose-export", "boot_policy_step"])
        .arg("--compose-shell")
        .arg(shell)
        .args(["--crate-name", CRATE_NAME, "--target", "kernel"])
        .arg("--out")
        .arg(out);
    command
}

pub(super) fn validate_candidate_pin(forge: &Path) -> Result<(), String> {
    let actual_forge_sha = sha256sum(forge)?;
    if actual_forge_sha != FORGE_SHA256 {
        return Err(format!(
            "M1 BootInfo Forge digest is {actual_forge_sha}, expected candidate {FORGE_SHA256}"
        ));
    }
    let thermite_root = env::var_os("THERMITE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/doll/Thermite"));
    let revision = run_checked(
        Command::new("git")
            .arg("-C")
            .arg(&thermite_root)
            .args(["rev-parse", "HEAD"]),
        "M1 BootInfo Thermite revision",
    )?;
    let revision = String::from_utf8(revision.stdout)
        .map_err(|error| format!("decode M1 BootInfo Thermite revision: {error}"))?;
    if revision.trim() != THERMITE_COMMIT {
        return Err(format!(
            "M1 BootInfo Thermite revision is {}, expected candidate {THERMITE_COMMIT}",
            revision.trim()
        ));
    }
    let status = run_checked(
        Command::new("git")
            .arg("-C")
            .arg(&thermite_root)
            .args(["status", "--porcelain"]),
        "M1 BootInfo Thermite worktree status",
    )?;
    if !status.stdout.is_empty() {
        return Err("M1 BootInfo Thermite candidate worktree is dirty".to_string());
    }
    Ok(())
}

fn run_source_assurance(forge: &Path, root: &Path, work: &Path) -> Result<(), String> {
    let audit = run_checked(
        Command::new(forge).current_dir(root).args([
            "audit",
            SOURCE,
            "--json",
            "--meaning",
            "--metrics",
        ]),
        "M1 BootInfo Thermite audit",
    )?;
    write_output(&work.join("audit.txt"), &audit, "M1 BootInfo audit")?;
    require_output_fragments(
        &audit.stdout,
        "M1 BootInfo audit",
        &[
            "\"level\": \"L3\"",
            "\"mutants_killed\": \"64/64\"",
            "\"kind\": \"end_to_end\"",
            "\"slag\": false",
        ],
    )?;
    let battery = run_checked(
        Command::new(forge)
            .current_dir(root)
            .args(["battery", SOURCE, "boot_policy_step"]),
        "M1 BootInfo Thermite battery",
    )?;
    write_output(&work.join("battery.txt"), &battery, "M1 BootInfo battery")?;
    require_output_fragments(
        &battery.stdout,
        "M1 BootInfo battery",
        &["non-vacuous", "64/64"],
    )
}

fn validate_receipt(root: &Path, bundle: &Path, receipt: &Value) -> Result<(), String> {
    if json_string(receipt, "/schema", "BootInfo receipt schema")? != RECEIPT_SCHEMA
        || json_string(receipt, "/binding/schema", "BootInfo binding schema")? != RECEIPT_SCHEMA
        || json_string(receipt, "/binding/scope", "BootInfo scope")? != "end_to_end"
        || json_string(receipt, "/binding/target", "BootInfo target")? != "kernel"
        || json_string(receipt, "/binding/crate_name", "BootInfo crate")? != CRATE_NAME
    {
        return Err("M1 BootInfo receipt identity is not accepted".to_string());
    }
    let gates = receipt
        .pointer("/binding/strict_gates")
        .and_then(Value::as_array)
        .ok_or_else(|| "M1 BootInfo receipt has no strict gates".to_string())?;
    for gate in [
        "direct-verus-source-policy",
        "combined-source-inventory",
        "whole-crate-no-cheating",
        "verus-codegen",
        "cryptographic-binding",
    ] {
        if !gates.iter().any(|value| value.as_str() == Some(gate)) {
            return Err(format!("M1 BootInfo receipt is missing gate `{gate}`"));
        }
    }
    for (relative, expected, label) in [
        (
            "evidence/input.th",
            sha256sum(&root.join(SOURCE))?,
            "Thermite source",
        ),
        (
            "evidence/direct-verus/00-bootinfo_shell.rs",
            sha256sum(&root.join(SHELL))?,
            "direct-Verus shell",
        ),
        (
            "evidence/source.verus.rs",
            json_string(
                receipt,
                "/binding/composition/combined_source_sha256",
                "BootInfo combined source",
            )?
            .to_string(),
            "combined source",
        ),
        (
            ARTIFACT,
            json_string(receipt, "/binding/artifact/sha256", "BootInfo artifact")?.to_string(),
            "artifact",
        ),
    ] {
        let actual = sha256sum(&bundle.join(relative))?;
        if actual != expected {
            return Err(format!(
                "M1 BootInfo {label} digest is {actual}, expected {expected}"
            ));
        }
    }
    let source = fs::read_to_string(bundle.join("evidence/source.verus.rs"))
        .map_err(|error| format!("read BootInfo combined source: {error}"))?;
    for forbidden in ["external_body", "assume(", "admit(", "decreases *"] {
        if source.contains(forbidden) {
            return Err(format!("M1 BootInfo source contains `{forbidden}`"));
        }
    }
    for required in [
        "pub(crate) fn boot_policy_step",
        "use vstd::prelude::*;",
        "pub open spec fn bootinfo_accepted",
        "pub fn validate_bootinfo",
        "reserved_zero: reserved0 == 0 && reserved1 == 0",
        "result.code == 0 ==> bootinfo_accepted(bytes, &result)",
    ] {
        if !source.contains(required) {
            return Err(format!("M1 BootInfo source is missing `{required}`"));
        }
    }
    let plan = read_json(
        &bundle.join("evidence/artifact-plan.v1"),
        "BootInfo artifact plan",
    )?;
    let args = plan
        .pointer("/expected_verus_args")
        .and_then(Value::as_array)
        .ok_or_else(|| "M1 BootInfo artifact plan has no Verus arguments".to_string())?;
    for expected in [
        "--no-vstd",
        "--no-cheating",
        "vstd=<KERNEL_VSTD_VIR>",
        "vstd=<KERNEL_VSTD_RLIB>",
    ] {
        if !args
            .iter()
            .any(|argument| argument.as_str() == Some(expected))
        {
            return Err(format!(
                "M1 BootInfo artifact plan is missing Verus argument `{expected}`"
            ));
        }
    }
    Ok(())
}

struct KernelVstdEvidence {
    vir_sha: String,
    source_sha: String,
    link_source_sha: String,
    link_rlib_sha: String,
}

fn validate_kernel_vstd(bundle: &Path, toolchain: &Value) -> Result<KernelVstdEvidence, String> {
    if json_string(
        toolchain,
        "/forge_source_identity",
        "BootInfo Forge source identity",
    )? != THERMITE_COMMIT
        || json_string(
            toolchain,
            "/forge_executable_sha256",
            "BootInfo Forge executable digest",
        )? != FORGE_SHA256
    {
        return Err("M1 BootInfo receipt is not bound to the candidate Forge pin".to_string());
    }
    let model = toolchain
        .pointer("/kernel_vstd_model")
        .and_then(Value::as_object)
        .ok_or_else(|| "BootInfo toolchain has no kernel vstd model".to_string())?;
    let digest = |name: &str| -> Result<String, String> {
        let value = model
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("BootInfo kernel vstd model has no `{name}`"))?;
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "BootInfo kernel vstd `{name}` is not a SHA-256 digest"
            ));
        }
        Ok(value.to_string())
    };
    let evidence = KernelVstdEvidence {
        vir_sha: digest("vir_sha256")?,
        source_sha: digest("source_sha256")?,
        link_source_sha: digest("link_source_sha256")?,
        link_rlib_sha: digest("link_rlib_sha256")?,
    };
    if model
        .get("source_file_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0
        || model.get("link_source_name").and_then(Value::as_str) != Some("kernel-vstd-link.rs")
        || sha256sum(&bundle.join("evidence/kernel-vstd-link.rs"))? != evidence.link_source_sha
        || sha256sum(&bundle.join("artifact/deps/libvstd.rlib"))? != evidence.link_rlib_sha
    {
        return Err("M1 BootInfo kernel vstd evidence is not self-consistent".to_string());
    }
    Ok(evidence)
}

fn run_consumer(root: &Path, work: &Path, bundle: &Path, rustc: &Path) -> Result<String, String> {
    let executable = work.join("bootinfo-consumer");
    run_checked(
        Command::new(rustc)
            .current_dir(root)
            .args(["--edition=2021"])
            .arg(CONSUMER)
            .arg("--extern")
            .arg(format!("{CRATE_NAME}={}", bundle.join(ARTIFACT).display()))
            .arg("-L")
            .arg(format!(
                "dependency={}",
                bundle.join("artifact/deps").display()
            ))
            .args(["-C", "panic=abort", "-o"])
            .arg(&executable),
        "compile M1 BootInfo runtime consumer",
    )?;
    let runtime = run_checked(
        Command::new(&executable).current_dir(root),
        "execute M1 BootInfo runtime consumer",
    )?;
    write_output(
        &work.join("runtime.txt"),
        &runtime,
        "M1 BootInfo runtime evidence",
    )?;
    require_output_fragments(&runtime.stdout, "M1 BootInfo runtime", &[RUNTIME_MARKER])?;
    sha256sum(&executable)
}

fn run_freestanding(
    root: &Path,
    work: &Path,
    bundle: &Path,
    rustc: &Path,
) -> Result<(String, String), String> {
    let rlib = work.join("libbootinfo-freestanding.rlib");
    run_checked(
        Command::new(rustc)
            .current_dir(root)
            .args(["--edition=2021"])
            .arg(FREESTANDING)
            .args(["--crate-type=rlib", "--extern"])
            .arg(format!("{CRATE_NAME}={}", bundle.join(ARTIFACT).display()))
            .arg("-L")
            .arg(format!(
                "dependency={}",
                bundle.join("artifact/deps").display()
            ))
            .args(["-C", "panic=abort", "-o"])
            .arg(&rlib),
        "link M1 BootInfo freestanding rlib consumer",
    )?;
    let executable = work.join("bootinfo-freestanding");
    run_checked(
        Command::new(rustc)
            .current_dir(root)
            .args(["--edition=2021"])
            .arg(FREESTANDING)
            .arg("--extern")
            .arg(format!("{CRATE_NAME}={}", bundle.join(ARTIFACT).display()))
            .arg("-L")
            .arg(format!(
                "dependency={}",
                bundle.join("artifact/deps").display()
            ))
            .args(["-C", "panic=abort", "-C", "link-arg=-nostartfiles", "-o"])
            .arg(&executable),
        "link M1 BootInfo freestanding ELF consumer",
    )?;
    let header = run_checked(
        Command::new("readelf").arg("-h").arg(&executable),
        "inspect M1 BootInfo freestanding ELF",
    )?;
    write_output(
        &work.join("freestanding-readelf.txt"),
        &header,
        "M1 BootInfo freestanding ELF evidence",
    )?;
    require_output_fragments(
        &header.stdout,
        "M1 BootInfo freestanding ELF",
        &["ELF64", "Advanced Micro Devices X86-64"],
    )?;
    Ok((sha256sum(&rlib)?, sha256sum(&executable)?))
}

fn run_negative_builds(forge: &Path, root: &Path, work: &Path) -> Result<(), String> {
    let shell = fs::read_to_string(root.join(SHELL))
        .map_err(|error| format!("read BootInfo shell for negatives: {error}"))?;
    let cases = [
        (
            "wrong-byte-model",
            shell.replacen("(bytes@[offset] as u32)", "(bytes@[offset + 1] as u32)", 1),
        ),
        (
            "map-overrun",
            shell.replacen(
                "let offset = 256usize + index as usize * 32usize;",
                "let offset = 257usize + index as usize * 32usize;",
                1,
            ),
        ),
        (
            "reserved-tail-omission",
            shell.replacen(
                "reserved_zero: reserved0 == 0 && reserved1 == 0,",
                "reserved_zero: reserved0 == 0,",
                1,
            ),
        ),
    ];
    for (name, mutated) in cases {
        if mutated == shell {
            return Err(format!("could not construct BootInfo `{name}` negative"));
        }
        reject_shell(forge, root, work, name, &mutated)?;
    }
    Ok(())
}

fn reject_shell(
    forge: &Path,
    root: &Path,
    work: &Path,
    name: &str,
    source: &str,
) -> Result<(), String> {
    let shell = work.join(format!("{name}.rs"));
    fs::write(&shell, source).map_err(|error| format!("write {name} shell: {error}"))?;
    let bundle = work.join(format!("{name}.verified"));
    let output = run_expect_failure(
        &mut build_command(forge, root, &shell, &bundle),
        &format!("M1 BootInfo {name} negative"),
    )?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !combined.contains("whole-crate-verus") || bundle.exists() {
        return Err(format!(
            "M1 BootInfo {name} did not fail atomically in whole-crate Verus"
        ));
    }
    write_output(
        &work.join(format!("negative-{name}.txt")),
        &output,
        "M1 BootInfo negative evidence",
    )
}

fn run_bundle_tamper_negatives(
    forge: &Path,
    root: &Path,
    work: &Path,
    primary: &Path,
) -> Result<(), String> {
    let receipt_bundle = work.join("receipt-tampered.verified");
    copy_tree(primary, &receipt_bundle)?;
    let receipt_path = receipt_bundle.join("receipt.json");
    let mut receipt = read_json(&receipt_path, "BootInfo receipt for tamper")?;
    let digest = json_string(&receipt, "/binding_sha256", "BootInfo tamper digest")?;
    let replacement = format!(
        "{}{}",
        if digest.starts_with('0') { '1' } else { '0' },
        &digest[1..]
    );
    *receipt
        .pointer_mut("/binding_sha256")
        .ok_or_else(|| "BootInfo receipt has no mutable binding digest".to_string())? =
        Value::String(replacement);
    fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("serialize BootInfo receipt tamper: {error}"))?,
    )
    .map_err(|error| format!("write BootInfo receipt tamper: {error}"))?;
    reject_bundle(forge, root, work, "receipt-tamper", &receipt_bundle)?;

    for (name, relative) in [
        ("vstd-source-tamper", "evidence/kernel-vstd-link.rs"),
        ("vstd-rlib-tamper", "artifact/deps/libvstd.rlib"),
    ] {
        let bundle = work.join(format!("{name}.verified"));
        copy_tree(primary, &bundle)?;
        let path = bundle.join(relative);
        let mut bytes =
            fs::read(&path).map_err(|error| format!("read BootInfo {name} target: {error}"))?;
        let first = bytes
            .first_mut()
            .ok_or_else(|| format!("BootInfo {name} target is empty"))?;
        *first ^= 1;
        fs::write(&path, bytes)
            .map_err(|error| format!("write BootInfo {name} target: {error}"))?;
        reject_bundle(forge, root, work, name, &bundle)?;
    }
    Ok(())
}

fn reject_bundle(
    forge: &Path,
    root: &Path,
    work: &Path,
    name: &str,
    bundle: &Path,
) -> Result<(), String> {
    let output = run_expect_failure(
        Command::new(forge)
            .current_dir(root)
            .arg("verify-build")
            .arg(bundle)
            .arg("--json"),
        &format!("M1 BootInfo {name} rejection"),
    )?;
    write_output(
        &work.join(format!("negative-{name}.txt")),
        &output,
        "M1 BootInfo bundle-tamper evidence",
    )
}
