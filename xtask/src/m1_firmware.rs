use super::m1_elf::{compare_file, read_json, require_file, verify_bundle, write_output};
use super::{
    check_forge_skill, copy_tree, forge_binary, json_string, require_output_fragments, run_checked,
    run_expect_failure, sha256sum, workspace_root,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE: &str = "thermite/boot/firmware_policy.th";
const SHELL: &str = "tests/m1/firmware_policy_shell.rs";
const CONSUMER: &str = "tests/m1/firmware_policy_consumer.rs";
const CRATE_NAME: &str = "tmk_firmware_policy";
const ARTIFACT: &str = "artifact/libtmk_firmware_policy.rlib";
const RECEIPT_SCHEMA: &str = "thermite.verified-composition-receipt.v1";
const RUNTIME_MARKER: &str = "M1_FIRMWARE_POLICY_OK observation=255";

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let forge = forge_binary()?;
    check_forge_skill(&forge)?;
    for (relative, label) in [
        (SOURCE, "firmware Thermite policy"),
        (SHELL, "firmware direct-Verus shell"),
        (CONSUMER, "firmware runtime consumer"),
    ] {
        require_file(&root.join(relative), label)?;
    }

    let work = root.join("build/m1-firmware");
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
            &format!("M1 firmware composition build {}", index + 1),
        )?;
        write_output(
            &work.join(format!("build-{}.txt", index + 1)),
            &output,
            "M1 firmware build evidence",
        )?;
    }

    let primary = &bundles[0];
    let receipt = read_json(&primary.join("receipt.json"), "M1 firmware receipt")?;
    validate_receipt(&root, primary, &receipt)?;
    let binding_sha = json_string(&receipt, "/binding_sha256", "firmware binding")?.to_string();
    let artifact_sha = json_string(
        &receipt,
        "/binding/artifact/sha256",
        "firmware artifact digest",
    )?
    .to_string();
    let combined_source_sha = json_string(
        &receipt,
        "/binding/composition/combined_source_sha256",
        "firmware combined source digest",
    )?
    .to_string();

    let validation = verify_bundle(&forge, &root, primary, false)?;
    let replay = verify_bundle(&forge, &root, primary, true)?;
    fs::write(
        work.join("verify.json"),
        serde_json::to_vec_pretty(&validation)
            .map_err(|error| format!("serialize firmware validation: {error}"))?,
    )
    .map_err(|error| format!("write firmware validation: {error}"))?;
    fs::write(
        work.join("replay.json"),
        serde_json::to_vec_pretty(&replay)
            .map_err(|error| format!("serialize firmware replay: {error}"))?,
    )
    .map_err(|error| format!("write firmware replay: {error}"))?;
    for (report, replayed, label) in [
        (&validation, false, "validation"),
        (&replay, true, "replay"),
    ] {
        if report.get("replayed").and_then(Value::as_bool) != Some(replayed)
            || json_string(report, "/binding_sha256", label)? != binding_sha
            || json_string(report, "/artifact_sha256", label)? != artifact_sha
        {
            return Err(format!(
                "M1 firmware {label} does not match the accepted receipt"
            ));
        }
    }

    let expected_receipt = fs::read(primary.join("receipt.json"))
        .map_err(|error| format!("read primary firmware receipt: {error}"))?;
    let expected_artifact = fs::read(primary.join(ARTIFACT))
        .map_err(|error| format!("read primary firmware artifact: {error}"))?;
    let expected_source = fs::read(primary.join("evidence/source.verus.rs"))
        .map_err(|error| format!("read primary firmware source: {error}"))?;
    for bundle in bundles.iter().skip(1) {
        compare_file(
            bundle,
            "receipt.json",
            &expected_receipt,
            "firmware receipt",
        )?;
        compare_file(bundle, ARTIFACT, &expected_artifact, "firmware artifact")?;
        compare_file(
            bundle,
            "evidence/source.verus.rs",
            &expected_source,
            "firmware combined source",
        )?;
        verify_bundle(&forge, &root, bundle, false)?;
    }

    let toolchain = read_json(
        &primary.join("evidence/toolchain.json"),
        "firmware toolchain evidence",
    )?;
    let rustc = PathBuf::from(json_string(
        &toolchain,
        "/artifact_codegen/rustc_path",
        "firmware codegen rustc",
    )?);
    require_file(&rustc, "firmware codegen rustc")?;
    let consumer_sha = run_consumer(&root, &work, primary, &rustc)?;
    run_negative_builds(&forge, &root, &work)?;
    run_tamper_negative(&forge, &root, &work, primary)?;

    let report = format!(
        "M1_FIRMWARE_OK\ncomponent_verified=true\nrelease_eligible=false\nreceipt_validated=true\nreceipt_replayed=true\nsource_sha256={}\nshell_sha256={}\ncombined_source_sha256={combined_source_sha}\nreceipt_sha256={}\nbinding_sha256={binding_sha}\nartifact_sha256={artifact_sha}\nconsumer_sha256={consumer_sha}\nreproducibility_builds=3\nruntime_marker={RUNTIME_MARKER}\npositive_scenarios=normalized-map,usable-range,mmio-range,buffer-resize,map-accept,stale-key-reacquire,fresh-key-exit\nnegative_scenarios=bad-descriptor-size,overlap,runtime-mismatch,unknown-attributes,no-usable-range,oversize-map,device-error,map-attempt-limit,exit-retry-limit,wrong-key,zero-key,wrong-observation,wrong-retry-expectation,receipt-tamper\n",
        sha256sum(&root.join(SOURCE))?,
        sha256sum(&root.join(SHELL))?,
        sha256sum(&primary.join("receipt.json"))?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M1 firmware report: {error}"))?;
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
        .args([
            "--level",
            "l3",
            "--compose-export",
            "memory_map_step",
            "--compose-export",
            "firmware_exit_step",
        ])
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
        "M1 firmware Thermite audit",
    )?;
    write_output(&work.join("audit.txt"), &audit, "M1 firmware audit")?;
    let text = String::from_utf8_lossy(&audit.stdout);
    if text.matches("\"mutants_killed\": \"64/64\"").count() != 2
        || text.matches("\"kind\": \"end_to_end\"").count() < 2
        || text.contains("\"slag\": true")
        || text.contains("\"boundary\": true")
    {
        return Err("M1 firmware audit is not two clean L3/end-to-end transitions".to_string());
    }
    for function in ["memory_map_step", "firmware_exit_step"] {
        let battery = run_checked(
            Command::new(forge)
                .current_dir(root)
                .args(["battery", SOURCE, function]),
            &format!("M1 firmware battery {function}"),
        )?;
        write_output(
            &work.join(format!("battery-{function}.txt")),
            &battery,
            "M1 firmware battery",
        )?;
        require_output_fragments(
            &battery.stdout,
            "M1 firmware battery",
            &["non-vacuous", "64/64"],
        )?;
    }
    Ok(())
}

fn validate_receipt(root: &Path, bundle: &Path, receipt: &Value) -> Result<(), String> {
    if json_string(receipt, "/schema", "firmware receipt schema")? != RECEIPT_SCHEMA
        || json_string(receipt, "/binding/schema", "firmware binding schema")? != RECEIPT_SCHEMA
        || json_string(receipt, "/binding/scope", "firmware assurance scope")? != "end_to_end"
        || json_string(receipt, "/binding/target", "firmware target")? != "kernel"
        || json_string(receipt, "/binding/crate_name", "firmware crate")? != CRATE_NAME
    {
        return Err("M1 firmware receipt identity is not accepted".to_string());
    }
    let gates = receipt
        .pointer("/binding/strict_gates")
        .and_then(Value::as_array)
        .ok_or_else(|| "M1 firmware receipt has no strict gates".to_string())?;
    for gate in [
        "direct-verus-source-policy",
        "combined-source-inventory",
        "whole-crate-no-cheating",
        "verus-codegen",
        "cryptographic-binding",
    ] {
        if !gates.iter().any(|value| value.as_str() == Some(gate)) {
            return Err(format!("M1 firmware receipt is missing gate `{gate}`"));
        }
    }
    for (relative, expected, label) in [
        (
            "evidence/input.th",
            sha256sum(&root.join(SOURCE))?,
            "Thermite source",
        ),
        (
            "evidence/direct-verus/00-firmware_policy_shell.rs",
            sha256sum(&root.join(SHELL))?,
            "direct-Verus shell",
        ),
        (
            "evidence/source.verus.rs",
            json_string(
                receipt,
                "/binding/composition/combined_source_sha256",
                "firmware combined source",
            )?
            .to_string(),
            "combined source",
        ),
        (
            ARTIFACT,
            json_string(receipt, "/binding/artifact/sha256", "firmware artifact")?.to_string(),
            "artifact",
        ),
    ] {
        let actual = sha256sum(&bundle.join(relative))?;
        if actual != expected {
            return Err(format!(
                "M1 firmware {label} digest is {actual}, expected {expected}"
            ));
        }
    }
    let source = fs::read_to_string(bundle.join("evidence/source.verus.rs"))
        .map_err(|error| format!("read firmware combined source: {error}"))?;
    for forbidden in ["external_body", "assume(", "admit(", "decreases *"] {
        if source.contains(forbidden) {
            return Err(format!("M1 firmware source contains `{forbidden}`"));
        }
    }
    for required in [
        "pub(crate) fn memory_map_step",
        "pub(crate) fn firmware_exit_step",
        "pub fn firmware_policy_observation",
    ] {
        if !source.contains(required) {
            return Err(format!("M1 firmware source is missing `{required}`"));
        }
    }
    Ok(())
}

fn run_consumer(root: &Path, work: &Path, bundle: &Path, rustc: &Path) -> Result<String, String> {
    let executable = work.join("firmware-policy-consumer");
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
        "compile M1 firmware runtime consumer",
    )?;
    let runtime = run_checked(
        Command::new(&executable).current_dir(root),
        "execute M1 firmware runtime consumer",
    )?;
    write_output(
        &work.join("runtime.txt"),
        &runtime,
        "M1 firmware runtime evidence",
    )?;
    require_output_fragments(&runtime.stdout, "M1 firmware runtime", &[RUNTIME_MARKER])?;
    sha256sum(&executable)
}

fn run_negative_builds(forge: &Path, root: &Path, work: &Path) -> Result<(), String> {
    let shell = fs::read_to_string(root.join(SHELL))
        .map_err(|error| format!("read firmware shell for negatives: {error}"))?;
    let wrong_observation = shell.replacen("    255\n}", "    254\n}", 1);
    if wrong_observation == shell {
        return Err("could not construct firmware observation negative".to_string());
    }
    reject_shell(forge, root, work, "wrong-observation", &wrong_observation)?;

    let wrong_retry = shell.replacen(
        "FirmwareExitAction::Reacquire => {},",
        "FirmwareExitAction::ExitAccepted => {},",
        1,
    );
    if wrong_retry == shell {
        return Err("could not construct firmware retry negative".to_string());
    }
    reject_shell(forge, root, work, "wrong-retry-expectation", &wrong_retry)
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
        &format!("M1 firmware {name} negative"),
    )?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !combined.contains("whole-crate-verus") || bundle.exists() {
        return Err(format!(
            "M1 firmware {name} did not fail atomically in whole-crate Verus"
        ));
    }
    write_output(
        &work.join(format!("negative-{name}.txt")),
        &output,
        "M1 firmware negative evidence",
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
    let path = tampered.join("receipt.json");
    let mut value = read_json(&path, "firmware receipt for tamper")?;
    let digest = json_string(&value, "/binding_sha256", "firmware tamper digest")?.to_string();
    let replacement = format!(
        "{}{}",
        if digest.starts_with('0') { '1' } else { '0' },
        &digest[1..]
    );
    *value
        .pointer_mut("/binding_sha256")
        .ok_or_else(|| "firmware receipt has no binding digest".to_string())? =
        Value::String(replacement);
    fs::write(
        &path,
        serde_json::to_vec_pretty(&value)
            .map_err(|error| format!("serialize firmware receipt tamper: {error}"))?,
    )
    .map_err(|error| format!("write firmware receipt tamper: {error}"))?;
    let output = run_expect_failure(
        Command::new(forge)
            .current_dir(root)
            .arg("verify-build")
            .arg(&tampered)
            .arg("--json"),
        "M1 firmware receipt tamper rejection",
    )?;
    write_output(
        &work.join("negative-receipt-tamper.txt"),
        &output,
        "M1 firmware receipt tamper evidence",
    )
}
