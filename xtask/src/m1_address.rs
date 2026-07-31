use super::m1_elf::{compare_file, read_json, require_file, verify_bundle, write_output};
use super::{
    check_forge_skill, copy_tree, forge_binary, json_string, require_output_fragments, run_checked,
    run_expect_failure, sha256sum, workspace_root,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE: &str = "thermite/platform/address_space_policy.th";
const SHELL: &str = "tests/m1/address_space_policy_shell.rs";
const CONSUMER: &str = "tests/m1/address_space_policy_consumer.rs";
const CRATE_NAME: &str = "tmk_address_space_policy";
const EXPORT: &str = "address_plan_step";
const ARTIFACT: &str = "artifact/libtmk_address_space_policy.rlib";
const RECEIPT_SCHEMA: &str = "thermite.verified-composition-receipt.v1";
const RUNTIME_MARKER: &str = "M1_ADDRESS_SPACE_POLICY_OK observation=511";

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let forge = forge_binary()?;
    check_forge_skill(&forge)?;
    for (relative, label) in [
        (SOURCE, "address-space Thermite policy"),
        (SHELL, "address-space direct-Verus shell"),
        (CONSUMER, "address-space runtime consumer"),
    ] {
        require_file(&root.join(relative), label)?;
    }

    let work = root.join("build/m1-address");
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
            &format!("M1 address-space composition build {}", index + 1),
        )?;
        write_output(
            &work.join(format!("build-{}.txt", index + 1)),
            &output,
            "M1 address-space build evidence",
        )?;
    }

    let primary = &bundles[0];
    let receipt = read_json(&primary.join("receipt.json"), "M1 address-space receipt")?;
    validate_receipt(&root, primary, &receipt)?;
    let binding_sha =
        json_string(&receipt, "/binding_sha256", "address-space binding")?.to_string();
    let artifact_sha = json_string(
        &receipt,
        "/binding/artifact/sha256",
        "address-space artifact digest",
    )?
    .to_string();
    let combined_source_sha = json_string(
        &receipt,
        "/binding/composition/combined_source_sha256",
        "address-space combined source digest",
    )?
    .to_string();

    let validation = verify_bundle(&forge, &root, primary, false)?;
    let replay = verify_bundle(&forge, &root, primary, true)?;
    fs::write(
        work.join("verify.json"),
        serde_json::to_vec_pretty(&validation)
            .map_err(|error| format!("serialize address-space validation: {error}"))?,
    )
    .map_err(|error| format!("write address-space validation: {error}"))?;
    fs::write(
        work.join("replay.json"),
        serde_json::to_vec_pretty(&replay)
            .map_err(|error| format!("serialize address-space replay: {error}"))?,
    )
    .map_err(|error| format!("write address-space replay: {error}"))?;
    for (report, replayed, label) in [
        (&validation, false, "validation"),
        (&replay, true, "replay"),
    ] {
        if report.get("replayed").and_then(Value::as_bool) != Some(replayed)
            || json_string(report, "/binding_sha256", label)? != binding_sha
            || json_string(report, "/artifact_sha256", label)? != artifact_sha
        {
            return Err(format!(
                "M1 address-space {label} does not match the accepted receipt"
            ));
        }
    }

    let expected_receipt = fs::read(primary.join("receipt.json"))
        .map_err(|error| format!("read primary address-space receipt: {error}"))?;
    let expected_artifact = fs::read(primary.join(ARTIFACT))
        .map_err(|error| format!("read primary address-space artifact: {error}"))?;
    let expected_source = fs::read(primary.join("evidence/source.verus.rs"))
        .map_err(|error| format!("read primary address-space source: {error}"))?;
    for bundle in bundles.iter().skip(1) {
        compare_file(
            bundle,
            "receipt.json",
            &expected_receipt,
            "address-space receipt",
        )?;
        compare_file(
            bundle,
            ARTIFACT,
            &expected_artifact,
            "address-space artifact",
        )?;
        compare_file(
            bundle,
            "evidence/source.verus.rs",
            &expected_source,
            "address-space combined source",
        )?;
        verify_bundle(&forge, &root, bundle, false)?;
    }

    let toolchain = read_json(
        &primary.join("evidence/toolchain.json"),
        "address-space toolchain evidence",
    )?;
    let rustc = PathBuf::from(json_string(
        &toolchain,
        "/artifact_codegen/rustc_path",
        "address-space codegen rustc",
    )?);
    require_file(&rustc, "address-space codegen rustc")?;
    let consumer_sha = run_consumer(&root, &work, primary, &rustc)?;
    run_negative_builds(&forge, &root, &work)?;
    run_tamper_negative(&forge, &root, &work, primary)?;

    let report = format!(
        "M1_ADDRESS_SPACE_OK\ncomponent_verified=true\nrelease_eligible=false\nreceipt_validated=true\nreceipt_replayed=true\nsource_sha256={}\nshell_sha256={}\ncombined_source_sha256={combined_source_sha}\nreceipt_sha256={}\nbinding_sha256={binding_sha}\nartifact_sha256={artifact_sha}\nconsumer_sha256={consumer_sha}\nreproducibility_builds=3\nruntime_marker={RUNTIME_MARKER}\npositive_scenarios=direct-window,heap-window,guarded-stack,text-rx,rodata-r-nx,data-rw-nx,complete-plan\nnegative_scenarios=kernel-image-alias,direct-offset,write-execute,missing-stack-guard,segment-order,misalignment,low-guard,physical-gap,virtual-gap,virtual-overlap,incomplete-plan,wrong-observation,wrong-alias-expectation,receipt-tamper\n",
        sha256sum(&root.join(SOURCE))?,
        sha256sum(&root.join(SHELL))?,
        sha256sum(&primary.join("receipt.json"))?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M1 address-space report: {error}"))?;
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
        "M1 address-space Thermite audit",
    )?;
    write_output(&work.join("audit.txt"), &audit, "M1 address-space audit")?;
    let text = String::from_utf8_lossy(&audit.stdout);
    for required in [
        "\"level\": \"L3\"",
        "\"mutants_killed\": \"64/64\"",
        "\"kind\": \"end_to_end\"",
        "\"slag\": false",
    ] {
        if !text.contains(required) {
            return Err(format!(
                "M1 address-space audit does not contain `{required}`"
            ));
        }
    }
    if text.contains("\"slag\": true") || text.contains("\"boundary\": true") {
        return Err("M1 address-space audit contains a trust downgrade".to_string());
    }
    let battery = run_checked(
        Command::new(forge)
            .current_dir(root)
            .args(["battery", SOURCE, EXPORT]),
        "M1 address-space Thermite battery",
    )?;
    write_output(
        &work.join("battery.txt"),
        &battery,
        "M1 address-space battery",
    )?;
    require_output_fragments(
        &battery.stdout,
        "M1 address-space battery",
        &["non-vacuous", "64/64"],
    )
}

fn validate_receipt(root: &Path, bundle: &Path, receipt: &Value) -> Result<(), String> {
    if json_string(receipt, "/schema", "address-space receipt schema")? != RECEIPT_SCHEMA
        || json_string(receipt, "/binding/schema", "address-space binding schema")?
            != RECEIPT_SCHEMA
        || json_string(receipt, "/binding/scope", "address-space scope")? != "end_to_end"
        || json_string(receipt, "/binding/target", "address-space target")? != "kernel"
        || json_string(receipt, "/binding/crate_name", "address-space crate")? != CRATE_NAME
    {
        return Err("M1 address-space receipt identity is not accepted".to_string());
    }
    let gates = receipt
        .pointer("/binding/strict_gates")
        .and_then(Value::as_array)
        .ok_or_else(|| "M1 address-space receipt has no strict gates".to_string())?;
    for gate in [
        "direct-verus-source-policy",
        "combined-source-inventory",
        "whole-crate-no-cheating",
        "verus-codegen",
        "cryptographic-binding",
    ] {
        if !gates.iter().any(|value| value.as_str() == Some(gate)) {
            return Err(format!("M1 address-space receipt is missing gate `{gate}`"));
        }
    }
    for (relative, expected, label) in [
        (
            "evidence/input.th",
            sha256sum(&root.join(SOURCE))?,
            "Thermite source",
        ),
        (
            "evidence/direct-verus/00-address_space_policy_shell.rs",
            sha256sum(&root.join(SHELL))?,
            "direct-Verus shell",
        ),
        (
            "evidence/source.verus.rs",
            json_string(
                receipt,
                "/binding/composition/combined_source_sha256",
                "address-space combined source",
            )?
            .to_string(),
            "combined source",
        ),
        (
            ARTIFACT,
            json_string(
                receipt,
                "/binding/artifact/sha256",
                "address-space artifact",
            )?
            .to_string(),
            "artifact",
        ),
    ] {
        let actual = sha256sum(&bundle.join(relative))?;
        if actual != expected {
            return Err(format!(
                "M1 address-space {label} digest is {actual}, expected {expected}"
            ));
        }
    }
    let source = fs::read_to_string(bundle.join("evidence/source.verus.rs"))
        .map_err(|error| format!("read address-space combined source: {error}"))?;
    for forbidden in ["external_body", "assume(", "admit(", "decreases *"] {
        if source.contains(forbidden) {
            return Err(format!("M1 address-space source contains `{forbidden}`"));
        }
    }
    for required in [
        "pub(crate) fn address_plan_step",
        "pub mod address_space_policy_shell",
        "pub fn address_space_policy_observation",
    ] {
        if !source.contains(required) {
            return Err(format!("M1 address-space source is missing `{required}`"));
        }
    }
    Ok(())
}

fn run_consumer(root: &Path, work: &Path, bundle: &Path, rustc: &Path) -> Result<String, String> {
    let executable = work.join("address-space-policy-consumer");
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
        "compile M1 address-space runtime consumer",
    )?;
    let runtime = run_checked(
        Command::new(&executable).current_dir(root),
        "execute M1 address-space runtime consumer",
    )?;
    write_output(
        &work.join("runtime.txt"),
        &runtime,
        "M1 address-space runtime evidence",
    )?;
    require_output_fragments(
        &runtime.stdout,
        "M1 address-space runtime",
        &[RUNTIME_MARKER],
    )?;
    sha256sum(&executable)
}

fn run_negative_builds(forge: &Path, root: &Path, work: &Path) -> Result<(), String> {
    let shell = fs::read_to_string(root.join(SHELL))
        .map_err(|error| format!("read address-space shell for negatives: {error}"))?;
    let wrong_observation = shell.replacen("    511\n}", "    510\n}", 1);
    if wrong_observation == shell {
        return Err("could not construct address-space observation negative".to_string());
    }
    reject_shell(forge, root, work, "wrong-observation", &wrong_observation)?;

    let alias_clause = "AddressPlanAction::Reject { code } => assert(code == 13),";
    let wrong_alias = shell.replacen(
        alias_clause,
        "AddressPlanAction::RegionAccepted { kind } => assert(kind == 1),",
        1,
    );
    if wrong_alias == shell {
        return Err("could not construct address-space alias negative".to_string());
    }
    reject_shell(forge, root, work, "wrong-alias-expectation", &wrong_alias)
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
        &format!("M1 address-space {name} negative"),
    )?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !combined.contains("whole-crate-verus") || bundle.exists() {
        return Err(format!(
            "M1 address-space {name} did not fail atomically in whole-crate Verus"
        ));
    }
    write_output(
        &work.join(format!("negative-{name}.txt")),
        &output,
        "M1 address-space negative evidence",
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
    let mut value = read_json(&path, "address-space receipt for tamper")?;
    let digest = json_string(&value, "/binding_sha256", "address-space tamper digest")?.to_string();
    let replacement = format!(
        "{}{}",
        if digest.starts_with('0') { '1' } else { '0' },
        &digest[1..]
    );
    *value
        .pointer_mut("/binding_sha256")
        .ok_or_else(|| "address-space receipt has no binding digest".to_string())? =
        Value::String(replacement);
    fs::write(
        &path,
        serde_json::to_vec_pretty(&value)
            .map_err(|error| format!("serialize address-space receipt tamper: {error}"))?,
    )
    .map_err(|error| format!("write address-space receipt tamper: {error}"))?;
    let output = run_expect_failure(
        Command::new(forge)
            .current_dir(root)
            .arg("verify-build")
            .arg(&tampered)
            .arg("--json"),
        "M1 address-space receipt tamper rejection",
    )?;
    write_output(
        &work.join("negative-receipt-tamper.txt"),
        &output,
        "M1 address-space receipt tamper evidence",
    )
}
