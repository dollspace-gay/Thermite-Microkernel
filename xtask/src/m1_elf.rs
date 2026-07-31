use super::{
    check_forge_skill, copy_tree, forge_binary, json_string, require_output_fragments, run_checked,
    run_expect_failure, sha256sum, workspace_root,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SOURCE: &str = "thermite/boot/elf_policy.th";
const SHELL: &str = "tests/m1/elf_policy_shell.rs";
const CONSUMER: &str = "tests/m1/elf_policy_consumer.rs";
const CRATE_NAME: &str = "tmk_elf_policy";
const EXPORT: &str = "elf_policy_step";
const ARTIFACT: &str = "artifact/libtmk_elf_policy.rlib";
const RECEIPT_SCHEMA: &str = "thermite.verified-composition-receipt.v1";
const RUNTIME_MARKER: &str = "M1_ELF_POLICY_OK observation=127";

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let forge = forge_binary()?;
    check_forge_skill(&forge)?;
    for (relative, label) in [
        (SOURCE, "ELF Thermite policy"),
        (SHELL, "ELF direct-Verus shell"),
        (CONSUMER, "ELF runtime consumer"),
    ] {
        require_file(&root.join(relative), label)?;
    }

    let work = root.join("build/m1-elf");
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
            &format!("M1 ELF Forge composition build {}", index + 1),
        )?;
        write_output(
            &work.join(format!("build-{}.txt", index + 1)),
            &output,
            "ELF build evidence",
        )?;
    }

    let primary = &bundles[0];
    let receipt = read_json(&primary.join("receipt.json"), "M1 ELF receipt")?;
    validate_receipt(&root, primary, &receipt)?;
    let binding_sha = json_string(&receipt, "/binding_sha256", "ELF binding")?.to_string();
    let artifact_sha =
        json_string(&receipt, "/binding/artifact/sha256", "ELF artifact digest")?.to_string();
    let source_sha = json_string(
        &receipt,
        "/binding/composition/combined_source_sha256",
        "ELF combined source digest",
    )?
    .to_string();

    let validation = verify_bundle(&forge, &root, primary, false)?;
    let replay = verify_bundle(&forge, &root, primary, true)?;
    fs::write(
        work.join("verify.json"),
        serde_json::to_vec_pretty(&validation)
            .map_err(|error| format!("serialize ELF validation: {error}"))?,
    )
    .map_err(|error| format!("write ELF validation: {error}"))?;
    fs::write(
        work.join("replay.json"),
        serde_json::to_vec_pretty(&replay)
            .map_err(|error| format!("serialize ELF replay: {error}"))?,
    )
    .map_err(|error| format!("write ELF replay: {error}"))?;
    for (report, replayed, label) in [
        (&validation, false, "validation"),
        (&replay, true, "replay"),
    ] {
        if report.get("replayed").and_then(Value::as_bool) != Some(replayed)
            || json_string(report, "/binding_sha256", label)? != binding_sha
            || json_string(report, "/artifact_sha256", label)? != artifact_sha
        {
            return Err(format!("ELF {label} does not match the accepted receipt"));
        }
    }

    let expected_receipt = fs::read(primary.join("receipt.json"))
        .map_err(|error| format!("read primary ELF receipt: {error}"))?;
    let expected_artifact = fs::read(primary.join(ARTIFACT))
        .map_err(|error| format!("read primary ELF artifact: {error}"))?;
    let expected_source = fs::read(primary.join("evidence/source.verus.rs"))
        .map_err(|error| format!("read primary ELF combined source: {error}"))?;
    for bundle in bundles.iter().skip(1) {
        compare_file(bundle, "receipt.json", &expected_receipt, "receipt")?;
        compare_file(bundle, ARTIFACT, &expected_artifact, "artifact")?;
        compare_file(
            bundle,
            "evidence/source.verus.rs",
            &expected_source,
            "combined source",
        )?;
        verify_bundle(&forge, &root, bundle, false)?;
    }

    let toolchain = read_json(
        &primary.join("evidence/toolchain.json"),
        "ELF toolchain evidence",
    )?;
    let rustc = PathBuf::from(json_string(
        &toolchain,
        "/artifact_codegen/rustc_path",
        "ELF codegen rustc path",
    )?);
    require_file(&rustc, "ELF codegen rustc")?;
    let consumer_sha = run_consumer(&root, &work, primary, &rustc)?;
    run_negative_builds(&forge, &root, &work)?;
    run_tamper_negative(&forge, &root, &work, primary)?;

    let report = format!(
        "M1_ELF_OK\ncomponent_verified=true\nrelease_eligible=false\nreceipt_validated=true\nreceipt_replayed=true\nsource_sha256={}\nshell_sha256={}\ncombined_source_sha256={source_sha}\nreceipt_sha256={}\nbinding_sha256={binding_sha}\nartifact_sha256={artifact_sha}\nconsumer_sha256={consumer_sha}\nreproducibility_builds=3\nruntime_marker={RUNTIME_MARKER}\npositive_scenarios=valid-header,text-load,data-load,gnu-relro,nonexec-gnu-stack,entry-covered\nnegative_scenarios=bad-digest,wx-load,dynamic-segment,executable-stack,entry-uncovered,overlap,file-overrun,wrong-observation,wrong-dynamic-expectation,receipt-tamper\n",
        sha256sum(&root.join(SOURCE))?,
        sha256sum(&root.join(SHELL))?,
        sha256sum(&primary.join("receipt.json"))?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M1 ELF report: {error}"))?;
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
        .args(["--level", "l3", "--compose-export", EXPORT])
        .arg("--compose-shell")
        .arg(shell)
        .args(["--crate-name", CRATE_NAME, "--target", "kernel"])
        .arg("--out")
        .arg(out);
    command
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
        "M1 ELF Thermite audit",
    )?;
    write_output(&work.join("audit.txt"), &audit, "M1 ELF audit")?;
    let audit_text = String::from_utf8_lossy(&audit.stdout);
    for required in [
        "\"level\": \"L3\"",
        "\"mutants_killed\": \"64/64\"",
        "\"kind\": \"end_to_end\"",
        "\"slag\": false",
    ] {
        if !audit_text.contains(required) {
            return Err(format!("M1 ELF audit does not contain `{required}`"));
        }
    }
    let battery = run_checked(
        Command::new(forge)
            .current_dir(root)
            .args(["battery", SOURCE, EXPORT]),
        "M1 ELF Thermite battery",
    )?;
    write_output(&work.join("battery.txt"), &battery, "M1 ELF battery")?;
    require_output_fragments(&battery.stdout, "M1 ELF battery", &["non-vacuous", "64/64"])
}

fn validate_receipt(root: &Path, bundle: &Path, receipt: &Value) -> Result<(), String> {
    if json_string(receipt, "/schema", "ELF receipt schema")? != RECEIPT_SCHEMA
        || json_string(receipt, "/binding/schema", "ELF binding schema")? != RECEIPT_SCHEMA
        || json_string(receipt, "/binding/scope", "ELF assurance scope")? != "end_to_end"
        || json_string(receipt, "/binding/target", "ELF target")? != "kernel"
        || json_string(receipt, "/binding/crate_name", "ELF crate name")? != CRATE_NAME
    {
        return Err("M1 ELF receipt identity is not the accepted kernel composition".to_string());
    }
    let strict = receipt
        .pointer("/binding/strict_gates")
        .and_then(Value::as_array)
        .ok_or_else(|| "M1 ELF receipt has no strict gate inventory".to_string())?;
    for gate in [
        "direct-verus-source-policy",
        "combined-source-inventory",
        "whole-crate-no-cheating",
        "verus-codegen",
        "cryptographic-binding",
    ] {
        if !strict.iter().any(|value| value.as_str() == Some(gate)) {
            return Err(format!("M1 ELF receipt is missing strict gate `{gate}`"));
        }
    }
    for (relative, expected, label) in [
        (
            "evidence/input.th",
            sha256sum(&root.join(SOURCE))?,
            "Thermite source",
        ),
        (
            "evidence/direct-verus/00-elf_policy_shell.rs",
            sha256sum(&root.join(SHELL))?,
            "direct-Verus shell",
        ),
        (
            "evidence/source.verus.rs",
            json_string(
                receipt,
                "/binding/composition/combined_source_sha256",
                "combined source",
            )?
            .to_string(),
            "combined source",
        ),
        (
            ARTIFACT,
            json_string(receipt, "/binding/artifact/sha256", "artifact")?.to_string(),
            "artifact",
        ),
    ] {
        let actual = sha256sum(&bundle.join(relative))?;
        if actual != expected {
            return Err(format!(
                "M1 ELF {label} digest is {actual}, expected {expected}"
            ));
        }
    }
    let source = fs::read_to_string(bundle.join("evidence/source.verus.rs"))
        .map_err(|error| format!("read M1 ELF combined source: {error}"))?;
    for forbidden in ["external_body", "assume(", "admit(", "decreases *"] {
        if source.contains(forbidden) {
            return Err(format!("M1 ELF combined source contains `{forbidden}`"));
        }
    }
    for required in [
        "pub(crate) fn elf_policy_step",
        "pub mod elf_policy_shell",
        "pub fn elf_policy_observation",
    ] {
        if !source.contains(required) {
            return Err(format!(
                "M1 ELF combined source does not contain `{required}`"
            ));
        }
    }
    Ok(())
}

fn verify_bundle(forge: &Path, root: &Path, bundle: &Path, replay: bool) -> Result<Value, String> {
    let mut command = Command::new(forge);
    command.current_dir(root).arg("verify-build").arg(bundle);
    if replay {
        command.arg("--replay");
    }
    command.arg("--json");
    let output = run_checked(
        &mut command,
        if replay {
            "M1 ELF receipt replay"
        } else {
            "M1 ELF receipt validation"
        },
    )?;
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("parse M1 ELF verification JSON: {error}"))
}

fn run_consumer(root: &Path, work: &Path, bundle: &Path, rustc: &Path) -> Result<String, String> {
    let output_path = work.join("elf-policy-consumer");
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
            .arg(&output_path),
        "compile M1 ELF runtime consumer",
    )?;
    let runtime = run_checked(
        Command::new(&output_path).current_dir(root),
        "execute M1 ELF runtime consumer",
    )?;
    write_output(
        &work.join("runtime.txt"),
        &runtime,
        "M1 ELF runtime evidence",
    )?;
    require_output_fragments(&runtime.stdout, "M1 ELF runtime", &[RUNTIME_MARKER])?;
    sha256sum(&output_path)
}

fn run_negative_builds(forge: &Path, root: &Path, work: &Path) -> Result<(), String> {
    let shell = fs::read_to_string(root.join(SHELL))
        .map_err(|error| format!("read M1 ELF shell for negatives: {error}"))?;
    let wrong_observation = shell.replacen("    127\n}", "    126\n}", 1);
    if wrong_observation == shell {
        return Err("could not construct wrong-observation ELF negative".to_string());
    }
    reject_shell(
        forge,
        root,
        work,
        "wrong-observation",
        &wrong_observation,
        "whole-crate-verus",
    )?;

    let dynamic_clause = "ElfPolicyAction::Reject { code } => assert(code == 13),";
    let wrong_dynamic = shell.replacen(
        dynamic_clause,
        "ElfPolicyAction::MetadataAccepted => {},",
        1,
    );
    if wrong_dynamic == shell {
        return Err("could not construct wrong-dynamic ELF negative".to_string());
    }
    reject_shell(
        forge,
        root,
        work,
        "wrong-dynamic-expectation",
        &wrong_dynamic,
        "whole-crate-verus",
    )
}

fn reject_shell(
    forge: &Path,
    root: &Path,
    work: &Path,
    name: &str,
    source: &str,
    expected: &str,
) -> Result<(), String> {
    let shell = work.join(format!("{name}.rs"));
    fs::write(&shell, source).map_err(|error| format!("write {name} shell: {error}"))?;
    let bundle = work.join(format!("{name}.verified"));
    let output = run_expect_failure(
        &mut build_command(forge, root, &shell, &bundle),
        &format!("M1 ELF {name} negative"),
    )?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !combined.contains(expected) || bundle.exists() {
        return Err(format!(
            "M1 ELF {name} negative did not fail atomically at `{expected}`"
        ));
    }
    write_output(
        &work.join(format!("negative-{name}.txt")),
        &output,
        "M1 ELF negative evidence",
    )
}

fn run_tamper_negative(
    forge: &Path,
    root: &Path,
    work: &Path,
    primary: &Path,
) -> Result<(), String> {
    let tampered = work.join("receipt-tampered.verified");
    copy_tree(primary, &tampered)?;
    let receipt = tampered.join("receipt.json");
    let mut value = read_json(&receipt, "M1 ELF receipt for tamper")?;
    let digest = json_string(&value, "/binding_sha256", "M1 ELF tamper digest")?.to_string();
    let replacement = format!(
        "{}{}",
        if digest.starts_with('0') { '1' } else { '0' },
        &digest[1..]
    );
    *value
        .pointer_mut("/binding_sha256")
        .ok_or_else(|| "M1 ELF receipt has no mutable binding digest".to_string())? =
        Value::String(replacement);
    fs::write(
        &receipt,
        serde_json::to_vec_pretty(&value)
            .map_err(|error| format!("serialize M1 ELF receipt tamper: {error}"))?,
    )
    .map_err(|error| format!("write M1 ELF receipt tamper: {error}"))?;
    let output = run_expect_failure(
        Command::new(forge)
            .current_dir(root)
            .arg("verify-build")
            .arg(&tampered)
            .arg("--json"),
        "M1 ELF receipt tamper rejection",
    )?;
    write_output(
        &work.join("negative-receipt-tamper.txt"),
        &output,
        "M1 ELF receipt tamper evidence",
    )
}

fn compare_file(bundle: &Path, relative: &str, expected: &[u8], label: &str) -> Result<(), String> {
    let actual = fs::read(bundle.join(relative))
        .map_err(|error| format!("read reproduced M1 ELF {label}: {error}"))?;
    if actual != expected {
        return Err(format!("M1 ELF {label} differs across independent builds"));
    }
    Ok(())
}

fn read_json(path: &Path, label: &str) -> Result<Value, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("read {label} {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {label}: {error}"))
}

fn require_file(path: &Path, label: &str) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("{label} is missing: {}", path.display()));
    }
    Ok(())
}

fn write_output(path: &Path, output: &Output, label: &str) -> Result<(), String> {
    let mut bytes = output.stdout.clone();
    bytes.extend_from_slice(&output.stderr);
    fs::write(path, bytes).map_err(|error| format!("write {label} {}: {error}", path.display()))
}
