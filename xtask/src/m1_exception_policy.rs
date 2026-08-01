use super::m1_bootinfo::{validate_candidate_pin, FORGE_SHA256, THERMITE_COMMIT};
use super::m1_elf::{compare_file, read_json, require_file, verify_bundle, write_output};
use super::{
    check_forge_skill, copy_tree, forge_binary, json_string, require_output_fragments, run_checked,
    run_expect_failure, sha256sum, workspace_root,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE: &str = "thermite/platform/exception_policy.th";
const SHELL: &str = "tests/m1/exception_policy_shell.rs";
const CONSUMER: &str = "tests/m1/exception_policy_consumer.rs";
const FREESTANDING: &str = "tests/m1/exception_policy_freestanding.rs";
const CRATE_NAME: &str = "tmk_exception_policy";
const ARTIFACT: &str = "artifact/libtmk_exception_policy.rlib";
const RECEIPT_SCHEMA: &str = "thermite.verified-composition-receipt.v1";
const RUNTIME_MARKER: &str = "M1_EXCEPTION_POLICY_OK observation=262143 scenarios=18";

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let forge = forge_binary()?;
    validate_candidate_pin(&forge)?;
    check_forge_skill(&forge)?;
    for (relative, label) in [
        (SOURCE, "exception-dispatch Thermite policy"),
        (SHELL, "exception-dispatch direct-Verus shell"),
        (CONSUMER, "exception-dispatch runtime consumer"),
        (FREESTANDING, "exception-dispatch freestanding consumer"),
    ] {
        require_file(&root.join(relative), label)?;
    }

    let work = root.join("build/m1-exception-policy");
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
            &format!("M1 exception-policy composition build {}", index + 1),
        )?;
        write_output(
            &work.join(format!("build-{}.txt", index + 1)),
            &output,
            "M1 exception-policy build evidence",
        )?;
    }

    let primary = &bundles[0];
    let receipt = read_json(
        primary.join("receipt.json").as_path(),
        "exception-policy receipt",
    )?;
    validate_receipt(&root, primary, &receipt)?;
    let binding_sha =
        json_string(&receipt, "/binding_sha256", "exception-policy binding")?.to_string();
    let artifact_sha = json_string(
        &receipt,
        "/binding/artifact/sha256",
        "exception-policy artifact",
    )?
    .to_string();
    let combined_source_sha = json_string(
        &receipt,
        "/binding/composition/combined_source_sha256",
        "exception-policy combined source",
    )?
    .to_string();

    let validation = verify_bundle(&forge, &root, primary, false)?;
    let replay = verify_bundle(&forge, &root, primary, true)?;
    fs::write(
        work.join("verify.json"),
        serde_json::to_vec_pretty(&validation)
            .map_err(|error| format!("serialize exception-policy validation: {error}"))?,
    )
    .map_err(|error| format!("write exception-policy validation: {error}"))?;
    fs::write(
        work.join("replay.json"),
        serde_json::to_vec_pretty(&replay)
            .map_err(|error| format!("serialize exception-policy replay: {error}"))?,
    )
    .map_err(|error| format!("write exception-policy replay: {error}"))?;
    for (report, replayed, label) in [
        (&validation, false, "validation"),
        (&replay, true, "replay"),
    ] {
        if report.get("replayed").and_then(Value::as_bool) != Some(replayed)
            || json_string(report, "/binding_sha256", label)? != binding_sha
            || json_string(report, "/artifact_sha256", label)? != artifact_sha
        {
            return Err(format!(
                "M1 exception-policy {label} does not match the receipt"
            ));
        }
    }

    for relative in [
        "receipt.json",
        "evidence/source.verus.rs",
        ARTIFACT,
        "artifact/deps/libvstd.rlib",
        "evidence/kernel-vstd-link.rs",
    ] {
        let expected = fs::read(primary.join(relative))
            .map_err(|error| format!("read primary exception-policy `{relative}`: {error}"))?;
        for bundle in bundles.iter().skip(1) {
            compare_file(
                bundle,
                relative,
                &expected,
                "exception-policy reproducible artifact",
            )?;
        }
    }
    for bundle in bundles.iter().skip(1) {
        verify_bundle(&forge, &root, bundle, false)?;
    }

    let toolchain = read_json(
        &primary.join("evidence/toolchain.json"),
        "exception-policy toolchain",
    )?;
    validate_toolchain(primary, &toolchain)?;
    let rustc = PathBuf::from(json_string(
        &toolchain,
        "/artifact_codegen/rustc_path",
        "exception-policy codegen rustc",
    )?);
    require_file(&rustc, "exception-policy codegen rustc")?;
    let consumer_sha = run_consumer(&root, &work, primary, &rustc)?;
    let (freestanding_rlib_sha, freestanding_elf_sha, platform_object_sha, linked_memcpy_sha) =
        run_freestanding(&root, &work, primary, &rustc)?;
    run_negative_builds(&forge, &root, &work)?;
    run_bundle_tamper_negatives(&forge, &root, &work, primary)?;

    let report = format!(
        "M1_EXCEPTION_POLICY_OK\ncomponent_verified=true\nrelease_eligible=false\ncandidate_pin_verified=true\ndispatcher_machine_actions_executed=false\nreceipt_validated=true\nreceipt_replayed=true\nsource_sha256={}\nshell_sha256={}\nconsumer_source_sha256={}\nfreestanding_source_sha256={}\ncombined_source_sha256={combined_source_sha}\nreceipt_sha256={}\nbinding_sha256={binding_sha}\nartifact_sha256={artifact_sha}\nconsumer_sha256={consumer_sha}\nfreestanding_rlib_sha256={freestanding_rlib_sha}\nfreestanding_elf_sha256={freestanding_elf_sha}\nplatform_primitive_object_sha256={platform_object_sha}\nlinked_memcpy_sha256={linked_memcpy_sha}\nforge_source_identity={THERMITE_COMMIT}\nforge_sha256={FORGE_SHA256}\nreproducibility_builds=3\nmutation_battery=64/64\nscenarios=user-page-fault,user-terminate,corrupt-page-fault,kernel-page-fault,double-fault,timer,reschedule,bound-irq,unbound-irq,new-shootdown,stale-shootdown,stop-ipi,spurious,bad-frame,bad-vector,missing-thread,counter-overflow,latched-panic\nfreestanding_links=rlib,elf64-x86-64\nfreestanding_dependencies=verified-m0-memcpy\nruntime_marker={RUNTIME_MARKER}\nnegative_cases=wrong-observation,wrong-page-access,wrong-irq-mask,wrong-stop-reason,receipt-tamper,vstd-source-tamper,vstd-rlib-tamper\n",
        sha256sum(&root.join(SOURCE))?,
        sha256sum(&root.join(SHELL))?,
        sha256sum(&root.join(CONSUMER))?,
        sha256sum(&root.join(FREESTANDING))?,
        sha256sum(&primary.join("receipt.json"))?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M1 exception-policy report: {error}"))?;
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
        .args(["--level", "l3", "--compose-export", "exception_policy_step"])
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
        "M1 exception-policy Thermite audit",
    )?;
    write_output(&work.join("audit.txt"), &audit, "M1 exception-policy audit")?;
    require_output_fragments(
        &audit.stdout,
        "M1 exception-policy audit",
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
            .args(["battery", SOURCE, "exception_policy_step"]),
        "M1 exception-policy battery",
    )?;
    write_output(
        &work.join("battery.txt"),
        &battery,
        "M1 exception-policy battery",
    )?;
    require_output_fragments(
        &battery.stdout,
        "M1 exception-policy battery",
        &["non-vacuous", "64/64"],
    )
}

fn validate_receipt(root: &Path, bundle: &Path, receipt: &Value) -> Result<(), String> {
    if json_string(receipt, "/schema", "exception-policy receipt schema")? != RECEIPT_SCHEMA
        || json_string(
            receipt,
            "/binding/schema",
            "exception-policy binding schema",
        )? != RECEIPT_SCHEMA
        || json_string(receipt, "/binding/scope", "exception-policy scope")? != "end_to_end"
        || json_string(receipt, "/binding/target", "exception-policy target")? != "kernel"
        || json_string(receipt, "/binding/crate_name", "exception-policy crate")? != CRATE_NAME
    {
        return Err("M1 exception-policy receipt identity is not accepted".to_string());
    }
    let gates = receipt
        .pointer("/binding/strict_gates")
        .and_then(Value::as_array)
        .ok_or_else(|| "M1 exception-policy receipt has no strict gates".to_string())?;
    for gate in [
        "direct-verus-source-policy",
        "combined-source-inventory",
        "whole-crate-no-cheating",
        "verus-codegen",
        "cryptographic-binding",
    ] {
        if !gates.iter().any(|value| value.as_str() == Some(gate)) {
            return Err(format!(
                "M1 exception-policy receipt is missing gate `{gate}`"
            ));
        }
    }
    for (relative, expected, label) in [
        (
            "evidence/input.th",
            sha256sum(&root.join(SOURCE))?,
            "Thermite source",
        ),
        (
            "evidence/direct-verus/00-exception_policy_shell.rs",
            sha256sum(&root.join(SHELL))?,
            "direct-Verus shell",
        ),
        (
            "evidence/source.verus.rs",
            json_string(
                receipt,
                "/binding/composition/combined_source_sha256",
                "exception-policy combined source",
            )?
            .to_string(),
            "combined source",
        ),
        (
            ARTIFACT,
            json_string(
                receipt,
                "/binding/artifact/sha256",
                "exception-policy artifact",
            )?
            .to_string(),
            "artifact",
        ),
    ] {
        let actual = sha256sum(&bundle.join(relative))?;
        if actual != expected {
            return Err(format!(
                "M1 exception-policy {label} digest is {actual}, expected {expected}"
            ));
        }
    }
    let source = fs::read_to_string(bundle.join("evidence/source.verus.rs"))
        .map_err(|error| format!("read exception-policy source: {error}"))?;
    for forbidden in ["external_body", "assume(", "admit(", "decreases *"] {
        if source.contains(forbidden) {
            return Err(format!("M1 exception-policy source contains `{forbidden}`"));
        }
    }
    for required in [
        "pub(crate) fn exception_policy_step",
        "pub fn exception_policy_observation",
        "ExceptionAction::DeliverFault",
        "ExceptionAction::NotifyIrq",
        "ExceptionAction::TlbShootdown",
        "ExceptionAction::Panic",
    ] {
        if !source.contains(required) {
            return Err(format!(
                "M1 exception-policy source is missing `{required}`"
            ));
        }
    }
    Ok(())
}

fn validate_toolchain(bundle: &Path, toolchain: &Value) -> Result<(), String> {
    if json_string(
        toolchain,
        "/forge_source_identity",
        "exception-policy Forge identity",
    )? != THERMITE_COMMIT
        || json_string(
            toolchain,
            "/forge_executable_sha256",
            "exception-policy Forge digest",
        )? != FORGE_SHA256
    {
        return Err("M1 exception-policy is not bound to the candidate Forge pin".to_string());
    }
    let model = toolchain
        .pointer("/kernel_vstd_model")
        .and_then(Value::as_object)
        .ok_or_else(|| "M1 exception-policy has no kernel vstd model".to_string())?;
    let link_source_sha = model
        .get("link_source_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| "M1 exception-policy has no vstd link-source digest".to_string())?;
    let link_rlib_sha = model
        .get("link_rlib_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| "M1 exception-policy has no vstd link-rlib digest".to_string())?;
    if sha256sum(&bundle.join("evidence/kernel-vstd-link.rs"))? != link_source_sha
        || sha256sum(&bundle.join("artifact/deps/libvstd.rlib"))? != link_rlib_sha
    {
        return Err("M1 exception-policy kernel vstd evidence is inconsistent".to_string());
    }
    Ok(())
}

fn run_consumer(root: &Path, work: &Path, bundle: &Path, rustc: &Path) -> Result<String, String> {
    let executable = work.join("exception-policy-consumer");
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
        "compile M1 exception-policy runtime consumer",
    )?;
    let runtime = run_checked(
        Command::new(&executable).current_dir(root),
        "execute M1 exception-policy runtime consumer",
    )?;
    write_output(
        &work.join("runtime.txt"),
        &runtime,
        "M1 exception-policy runtime evidence",
    )?;
    require_output_fragments(
        &runtime.stdout,
        "M1 exception-policy runtime",
        &[RUNTIME_MARKER],
    )?;
    sha256sum(&executable)
}

fn run_freestanding(
    root: &Path,
    work: &Path,
    bundle: &Path,
    rustc: &Path,
) -> Result<(String, String, String, String), String> {
    let primitives = root.join("build/m0-platform-primitives/objects/platform-primitives.o");
    let platform_report = root.join("build/m0-platform-primitives/report.txt");
    let linker = root.join("tests/m0/global_allocator_kernel.ld");
    validate_platform_primitives(root, &primitives, &platform_report, &linker)?;
    let platform_object_sha = sha256sum(&primitives)?;
    let rlib = work.join("libexception-policy-freestanding.rlib");
    let common_args = |command: &mut Command| {
        command
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
            .args(["-C", "panic=abort"]);
    };
    let mut low = Command::new(rustc);
    common_args(&mut low);
    low.args(["--crate-type=rlib", "-o"]).arg(&rlib);
    run_checked(&mut low, "link M1 exception-policy freestanding rlib")?;

    let executable = work.join("exception-policy-freestanding");
    let mut high = Command::new(rustc);
    common_args(&mut high);
    high.args(["-C", "link-arg=-nostartfiles"])
        .args(["-C", "code-model=kernel"])
        .args(["-C", "link-arg=-no-pie"])
        .args(["-C", "link-arg=-static"])
        .arg("-C")
        .arg(format!("link-arg={}", primitives.display()))
        .arg("-C")
        .arg(format!("link-arg=-T{}", linker.display()))
        .args(["-C", "link-arg=-Wl,--build-id=none", "-o"])
        .arg(&executable);
    run_checked(&mut high, "link M1 exception-policy freestanding ELF")?;
    let undefined = run_checked(
        Command::new("nm").arg("-u").arg(&executable),
        "audit M1 exception-policy undefined symbols",
    )?;
    if !undefined.stdout.is_empty() {
        return Err(format!(
            "M1 exception-policy freestanding ELF has undefined symbols:\n{}",
            String::from_utf8_lossy(&undefined.stdout)
        ));
    }
    let linked = work.join("linked-primitives");
    super::platform_primitives::audit_linked_composition_primitives(&executable, &linked)?;
    let linked_memcpy_sha = sha256sum(&linked.join("memcpy.bin"))?;
    let header = run_checked(
        Command::new("readelf").arg("-h").arg(&executable),
        "inspect M1 exception-policy freestanding ELF",
    )?;
    write_output(
        &work.join("freestanding-readelf.txt"),
        &header,
        "M1 exception-policy ELF evidence",
    )?;
    require_output_fragments(
        &header.stdout,
        "M1 exception-policy freestanding ELF",
        &["ELF64", "Advanced Micro Devices X86-64"],
    )?;
    Ok((
        sha256sum(&rlib)?,
        sha256sum(&executable)?,
        platform_object_sha,
        linked_memcpy_sha,
    ))
}

fn validate_platform_primitives(
    root: &Path,
    primitives: &Path,
    report: &Path,
    linker: &Path,
) -> Result<(), String> {
    for (path, label) in [
        (primitives, "verified platform primitive object"),
        (report, "platform primitive acceptance report"),
        (linker, "accepted higher-half linker script"),
    ] {
        require_file(path, label)?;
    }
    let text = fs::read_to_string(report)
        .map_err(|error| format!("read platform primitive acceptance report: {error}"))?;
    let checks = [
        ("component_verified", "true".to_string()),
        ("linked_primitives_verified", "true".to_string()),
        ("verus_verified", "39".to_string()),
        ("model_reproducibility_builds", "3".to_string()),
        ("primitive_object_sha256", sha256sum(primitives)?),
        ("linker_script_sha256", sha256sum(linker)?),
        (
            "model_source_sha256",
            sha256sum(&root.join("verus/machine-model/platform_primitives_capsule.rs"))?,
        ),
        (
            "adapter_source_sha256",
            sha256sum(&root.join("kernel-host/platform/global_allocator.rs"))?,
        ),
        (
            "auditor_sha256",
            sha256sum(&root.join("xtask/src/platform_primitives.rs"))?,
        ),
        (
            "memcpy_capsule_sha256",
            sha256sum(&root.join("build/m0-platform-primitives/emitted/memcpy.bin"))?,
        ),
    ];
    for (key, expected) in checks {
        let actual = report_field(&text, key)?;
        if actual != expected {
            return Err(format!(
                "platform primitive report field `{key}` is `{actual}`, expected `{expected}`"
            ));
        }
    }
    Ok(())
}

fn report_field<'a>(report: &'a str, key: &str) -> Result<&'a str, String> {
    report
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("platform primitive report has no `{key}` field"))
}

fn run_negative_builds(forge: &Path, root: &Path, work: &Path) -> Result<(), String> {
    let shell = fs::read_to_string(root.join(SHELL))
        .map_err(|error| format!("read exception-policy shell: {error}"))?;
    let cases = [
        (
            "wrong-observation",
            shell.replacen("    262143\n}", "    262142\n}", 1),
        ),
        (
            "wrong-page-access",
            shell.replacen("assert(access == 1);", "assert(access == 2);", 1),
        ),
        (
            "wrong-irq-mask",
            shell.replacen("assert(masked);", "assert(!masked);", 1),
        ),
        (
            "wrong-stop-reason",
            shell.replacen(
                "ExceptionAction::Panic { reason } => assert(reason == 9),",
                "ExceptionAction::Panic { reason } => assert(reason == 10),",
                1,
            ),
        ),
    ];
    for (name, mutated) in cases {
        if mutated == shell {
            return Err(format!(
                "could not construct exception-policy `{name}` negative"
            ));
        }
        let shell_path = work.join(format!("{name}.rs"));
        fs::write(&shell_path, mutated)
            .map_err(|error| format!("write exception-policy {name}: {error}"))?;
        let bundle = work.join(format!("{name}.verified"));
        let output = run_expect_failure(
            &mut build_command(forge, root, &shell_path, &bundle),
            &format!("M1 exception-policy {name} negative"),
        )?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !combined.contains("whole-crate-verus") || bundle.exists() {
            return Err(format!(
                "M1 exception-policy {name} did not fail atomically"
            ));
        }
        write_output(
            &work.join(format!("negative-{name}.txt")),
            &output,
            "M1 exception-policy negative evidence",
        )?;
    }
    Ok(())
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
    let mut receipt = read_json(&receipt_path, "exception-policy tamper receipt")?;
    let digest = json_string(
        &receipt,
        "/binding_sha256",
        "exception-policy tamper digest",
    )?
    .to_string();
    let replacement = format!(
        "{}{}",
        if digest.starts_with('0') { '1' } else { '0' },
        &digest[1..]
    );
    *receipt
        .pointer_mut("/binding_sha256")
        .ok_or_else(|| "exception-policy receipt has no binding digest".to_string())? =
        Value::String(replacement);
    fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("serialize exception-policy receipt: {error}"))?,
    )
    .map_err(|error| format!("write exception-policy receipt: {error}"))?;
    reject_bundle(forge, root, work, "receipt-tamper", &receipt_bundle)?;

    for (name, relative) in [
        ("vstd-source-tamper", "evidence/kernel-vstd-link.rs"),
        ("vstd-rlib-tamper", "artifact/deps/libvstd.rlib"),
    ] {
        let bundle = work.join(format!("{name}.verified"));
        copy_tree(primary, &bundle)?;
        let path = bundle.join(relative);
        let mut bytes =
            fs::read(&path).map_err(|error| format!("read exception-policy {name}: {error}"))?;
        *bytes
            .first_mut()
            .ok_or_else(|| format!("exception-policy {name} target is empty"))? ^= 1;
        fs::write(&path, bytes)
            .map_err(|error| format!("write exception-policy {name}: {error}"))?;
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
        &format!("M1 exception-policy {name} rejection"),
    )?;
    write_output(
        &work.join(format!("negative-{name}.txt")),
        &output,
        "M1 exception-policy tamper evidence",
    )
}
