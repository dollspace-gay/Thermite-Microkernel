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
const SHELL: &str = "tests/m1/exception_frame_shell.rs";
const CONSUMER: &str = "tests/m1/exception_frame_consumer.rs";
const FREESTANDING: &str = "tests/m1/exception_frame_freestanding.rs";
const CRATE_NAME: &str = "tmk_exception_frame";
const ARTIFACT: &str = "artifact/libtmk_exception_frame.rlib";
const RECEIPT_SCHEMA: &str = "thermite.verified-composition-receipt.v1";
const RUNTIME_MARKER: &str = "M1_EXCEPTION_FRAME_OK words=21/23 scenarios=12 observation=4095";

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let forge = forge_binary()?;
    validate_candidate_pin(&forge)?;
    check_forge_skill(&forge)?;
    for (relative, label) in [
        (SOURCE, "accepted exception policy"),
        (SHELL, "exception-frame direct-Verus shell"),
        (CONSUMER, "exception-frame runtime consumer"),
        (FREESTANDING, "exception-frame freestanding consumer"),
    ] {
        require_file(&root.join(relative), label)?;
    }

    let work = root.join("build/m1-exception-frame");
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
            &format!("M1 exception-frame composition build {}", index + 1),
        )?;
        write_output(
            &work.join(format!("build-{}.txt", index + 1)),
            &output,
            "M1 exception-frame build evidence",
        )?;
    }

    let primary = &bundles[0];
    let receipt = read_json(
        primary.join("receipt.json").as_path(),
        "exception-frame receipt",
    )?;
    validate_receipt(&root, primary, &receipt)?;
    let binding_sha = json_string(&receipt, "/binding_sha256", "exception-frame binding")?;
    let artifact_sha = json_string(
        &receipt,
        "/binding/artifact/sha256",
        "exception-frame artifact",
    )?;
    let combined_source_sha = json_string(
        &receipt,
        "/binding/composition/combined_source_sha256",
        "exception-frame combined source",
    )?;

    let validation = verify_bundle(&forge, &root, primary, false)?;
    let replay = verify_bundle(&forge, &root, primary, true)?;
    for (report, replayed, label) in [
        (&validation, false, "validation"),
        (&replay, true, "replay"),
    ] {
        if report.get("replayed").and_then(Value::as_bool) != Some(replayed)
            || json_string(report, "/binding_sha256", label)? != binding_sha
            || json_string(report, "/artifact_sha256", label)? != artifact_sha
        {
            return Err(format!(
                "M1 exception-frame {label} does not match the receipt"
            ));
        }
    }
    fs::write(
        work.join("verify.json"),
        serde_json::to_vec_pretty(&validation)
            .map_err(|error| format!("serialize exception-frame validation: {error}"))?,
    )
    .map_err(|error| format!("write exception-frame validation: {error}"))?;
    fs::write(
        work.join("replay.json"),
        serde_json::to_vec_pretty(&replay)
            .map_err(|error| format!("serialize exception-frame replay: {error}"))?,
    )
    .map_err(|error| format!("write exception-frame replay: {error}"))?;

    for relative in [
        "receipt.json",
        "evidence/source.verus.rs",
        ARTIFACT,
        "artifact/deps/libvstd.rlib",
        "evidence/kernel-vstd-link.rs",
    ] {
        let expected = fs::read(primary.join(relative))
            .map_err(|error| format!("read primary exception-frame `{relative}`: {error}"))?;
        for bundle in bundles.iter().skip(1) {
            compare_file(
                bundle,
                relative,
                &expected,
                "exception-frame reproducible artifact",
            )?;
        }
    }
    for bundle in bundles.iter().skip(1) {
        verify_bundle(&forge, &root, bundle, false)?;
    }

    let toolchain = read_json(
        &primary.join("evidence/toolchain.json"),
        "exception-frame toolchain",
    )?;
    validate_toolchain(primary, &toolchain)?;
    let rustc = PathBuf::from(json_string(
        &toolchain,
        "/artifact_codegen/rustc_path",
        "exception-frame codegen rustc",
    )?);
    require_file(&rustc, "exception-frame codegen rustc")?;
    let consumer_sha = run_consumer(&root, &work, primary, &rustc)?;
    let (freestanding_rlib_sha, freestanding_elf_sha, platform_object_sha, linked_memcpy_sha) =
        run_freestanding(&root, &work, primary, &rustc)?;
    run_negative_builds(&forge, &root, &work)?;
    run_bundle_tamper_negatives(&forge, &root, &work, primary)?;

    let report = format!(
        "M1_EXCEPTION_FRAME_OK\ncomponent_verified=true\nrelease_eligible=false\ncandidate_pin_verified=true\nraw_pointer_bridge_present=false\ndispatcher_machine_actions_executed=false\nreceipt_validated=true\nreceipt_replayed=true\nsource_sha256={}\nshell_sha256={}\nconsumer_source_sha256={}\nfreestanding_source_sha256={}\ncombined_source_sha256={combined_source_sha}\nreceipt_sha256={}\nbinding_sha256={binding_sha}\nartifact_sha256={artifact_sha}\nconsumer_sha256={consumer_sha}\nfreestanding_rlib_sha256={freestanding_rlib_sha}\nfreestanding_elf_sha256={freestanding_elf_sha}\nplatform_primitive_object_sha256={platform_object_sha}\nlinked_memcpy_sha256={linked_memcpy_sha}\nforge_source_identity={THERMITE_COMMIT}\nforge_sha256={FORGE_SHA256}\nreproducibility_builds=3\nmutation_battery=64/64\nframe_layout_words=21,23\nframe_layout_bytes=168,184\nframe_offsets=r15:0,cr2:112,rax:120,vector:128,error:136,rip:144,cs:152,rflags:160,rsp:168,ss:176\nruntime_marker={RUNTIME_MARKER}\nruntime_scenarios=12\nfreestanding_links=rlib,elf64-x86-64\nfreestanding_dependencies=verified-m0-memcpy\nnegative_cases=prefix-length,user-selector,short-vector,panic-reason,receipt-tamper,vstd-source-tamper,vstd-rlib-tamper\n",
        sha256sum(&root.join(SOURCE))?,
        sha256sum(&root.join(SHELL))?,
        sha256sum(&root.join(CONSUMER))?,
        sha256sum(&root.join(FREESTANDING))?,
        sha256sum(&primary.join("receipt.json"))?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M1 exception-frame report: {error}"))?;
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
        "M1 exception-frame Thermite audit",
    )?;
    write_output(&work.join("audit.txt"), &audit, "M1 exception-frame audit")?;
    require_output_fragments(
        &audit.stdout,
        "M1 exception-frame audit",
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
        "M1 exception-frame policy battery",
    )?;
    write_output(
        &work.join("battery.txt"),
        &battery,
        "M1 exception-frame policy battery",
    )?;
    require_output_fragments(
        &battery.stdout,
        "M1 exception-frame policy battery",
        &["non-vacuous", "64/64"],
    )
}

fn validate_receipt(root: &Path, bundle: &Path, receipt: &Value) -> Result<(), String> {
    if json_string(receipt, "/schema", "exception-frame receipt schema")? != RECEIPT_SCHEMA
        || json_string(receipt, "/binding/schema", "exception-frame binding schema")?
            != RECEIPT_SCHEMA
        || json_string(receipt, "/binding/scope", "exception-frame scope")? != "end_to_end"
        || json_string(receipt, "/binding/target", "exception-frame target")? != "kernel"
        || json_string(receipt, "/binding/crate_name", "exception-frame crate")? != CRATE_NAME
    {
        return Err("M1 exception-frame receipt identity is not accepted".to_string());
    }
    let gates = receipt
        .pointer("/binding/strict_gates")
        .and_then(Value::as_array)
        .ok_or_else(|| "M1 exception-frame receipt has no strict gates".to_string())?;
    for gate in [
        "direct-verus-source-policy",
        "combined-source-inventory",
        "whole-crate-no-cheating",
        "verus-codegen",
        "cryptographic-binding",
    ] {
        if !gates.iter().any(|value| value.as_str() == Some(gate)) {
            return Err(format!(
                "M1 exception-frame receipt is missing gate `{gate}`"
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
            "evidence/direct-verus/00-exception_frame_shell.rs",
            sha256sum(&root.join(SHELL))?,
            "direct-Verus shell",
        ),
        (
            "evidence/source.verus.rs",
            json_string(
                receipt,
                "/binding/composition/combined_source_sha256",
                "exception-frame combined source",
            )?
            .to_string(),
            "combined source",
        ),
        (
            ARTIFACT,
            json_string(
                receipt,
                "/binding/artifact/sha256",
                "exception-frame artifact",
            )?
            .to_string(),
            "artifact",
        ),
    ] {
        let actual = sha256sum(&bundle.join(relative))?;
        if actual != expected {
            return Err(format!(
                "M1 exception-frame {label} digest is {actual}, expected {expected}"
            ));
        }
    }
    let source = fs::read_to_string(bundle.join("evidence/source.verus.rs"))
        .map_err(|error| format!("read exception-frame source: {error}"))?;
    for forbidden in ["external_body", "assume(", "admit(", "decreases *"] {
        if source.contains(forbidden) {
            return Err(format!("M1 exception-frame source contains `{forbidden}`"));
        }
    }
    for required in [
        "pub fn exception_frame_valid",
        "pub fn normalize_exception_event",
        "pub fn dispatch_exception_frame",
        "words[14]",
        "words[16]",
        "words[22]",
    ] {
        if !source.contains(required) {
            return Err(format!("M1 exception-frame source is missing `{required}`"));
        }
    }
    Ok(())
}

fn validate_toolchain(bundle: &Path, toolchain: &Value) -> Result<(), String> {
    if json_string(
        toolchain,
        "/forge_source_identity",
        "exception-frame Forge identity",
    )? != THERMITE_COMMIT
        || json_string(
            toolchain,
            "/forge_executable_sha256",
            "exception-frame Forge digest",
        )? != FORGE_SHA256
    {
        return Err("M1 exception-frame is not bound to the candidate Forge pin".to_string());
    }
    let model = toolchain
        .pointer("/kernel_vstd_model")
        .and_then(Value::as_object)
        .ok_or_else(|| "M1 exception-frame has no kernel vstd model".to_string())?;
    let link_source_sha = model
        .get("link_source_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| "M1 exception-frame has no vstd link-source digest".to_string())?;
    let link_rlib_sha = model
        .get("link_rlib_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| "M1 exception-frame has no vstd link-rlib digest".to_string())?;
    if sha256sum(&bundle.join("evidence/kernel-vstd-link.rs"))? != link_source_sha
        || sha256sum(&bundle.join("artifact/deps/libvstd.rlib"))? != link_rlib_sha
    {
        return Err("M1 exception-frame kernel vstd evidence is inconsistent".to_string());
    }
    Ok(())
}

fn run_consumer(root: &Path, work: &Path, bundle: &Path, rustc: &Path) -> Result<String, String> {
    let executable = work.join("exception-frame-consumer");
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
        "compile M1 exception-frame runtime consumer",
    )?;
    let runtime = run_checked(
        Command::new(&executable).current_dir(root),
        "execute M1 exception-frame runtime consumer",
    )?;
    write_output(
        &work.join("runtime.txt"),
        &runtime,
        "M1 exception-frame runtime evidence",
    )?;
    require_output_fragments(
        &runtime.stdout,
        "M1 exception-frame runtime",
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
    super::m1_exception_policy::validate_platform_primitives(
        root,
        &primitives,
        &platform_report,
        &linker,
    )?;
    let platform_object_sha = sha256sum(&primitives)?;
    let rlib = work.join("libexception-frame-freestanding.rlib");
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
    run_checked(&mut low, "link M1 exception-frame freestanding rlib")?;

    let executable = work.join("exception-frame-freestanding");
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
    run_checked(&mut high, "link M1 exception-frame freestanding ELF")?;
    let undefined = run_checked(
        Command::new("nm").arg("-u").arg(&executable),
        "audit M1 exception-frame undefined symbols",
    )?;
    if !undefined.stdout.is_empty() {
        return Err(format!(
            "M1 exception-frame freestanding ELF has undefined symbols:\n{}",
            String::from_utf8_lossy(&undefined.stdout)
        ));
    }
    let linked = work.join("linked-primitives");
    super::platform_primitives::audit_linked_composition_primitives(&executable, &linked)?;
    let linked_memcpy_sha = sha256sum(&linked.join("memcpy.bin"))?;
    let header = run_checked(
        Command::new("readelf").arg("-h").arg(&executable),
        "inspect M1 exception-frame freestanding ELF",
    )?;
    write_output(
        &work.join("freestanding-readelf.txt"),
        &header,
        "M1 exception-frame ELF evidence",
    )?;
    require_output_fragments(
        &header.stdout,
        "M1 exception-frame freestanding ELF",
        &[
            "ELF64",
            "Advanced Micro Devices X86-64",
            "0xffffffff80000000",
        ],
    )?;
    Ok((
        sha256sum(&rlib)?,
        sha256sum(&executable)?,
        platform_object_sha,
        linked_memcpy_sha,
    ))
}

fn run_negative_builds(forge: &Path, root: &Path, work: &Path) -> Result<(), String> {
    let shell = fs::read_to_string(root.join(SHELL))
        .map_err(|error| format!("read exception-frame shell: {error}"))?;
    let cases = [
        (
            "prefix-length",
            shell.replacen(
                "words.len() == EXCEPTION_PREFIX_WORDS &&",
                "words.len() >= EXCEPTION_PREFIX_WORDS &&",
                1,
            ),
        ),
        (
            "user-selector",
            shell.replacen(
                "words@[22] == USER_DATA_SELECTOR",
                "words@[22] == KERNEL_CODE_SELECTOR",
                1,
            ),
        ),
        (
            "short-vector",
            shell.replacen("result.vector == 256,", "result.vector == 255,", 1),
        ),
        (
            "panic-reason",
            shell.replacen(
                "if state.panic_latched { 1u32 } else { 2u32 }",
                "if state.panic_latched { 1u32 } else { 3u32 }",
                1,
            ),
        ),
    ];
    for (name, mutated) in cases {
        if mutated == shell {
            return Err(format!(
                "could not construct exception-frame `{name}` negative"
            ));
        }
        let shell_path = work.join(format!("{name}.rs"));
        fs::write(&shell_path, mutated)
            .map_err(|error| format!("write exception-frame {name}: {error}"))?;
        let bundle = work.join(format!("{name}.verified"));
        let output = run_expect_failure(
            &mut build_command(forge, root, &shell_path, &bundle),
            &format!("M1 exception-frame {name} negative"),
        )?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !combined.contains("whole-crate-verus") || bundle.exists() {
            return Err(format!("M1 exception-frame {name} did not fail atomically"));
        }
        write_output(
            &work.join(format!("negative-{name}.txt")),
            &output,
            "M1 exception-frame negative evidence",
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
    let mut receipt = read_json(&receipt_path, "exception-frame tamper receipt")?;
    let digest = json_string(&receipt, "/binding_sha256", "exception-frame tamper digest")?;
    let replacement = format!(
        "{}{}",
        if digest.starts_with('0') { '1' } else { '0' },
        &digest[1..]
    );
    *receipt
        .pointer_mut("/binding_sha256")
        .ok_or_else(|| "exception-frame receipt has no binding digest".to_string())? =
        Value::String(replacement);
    fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("serialize exception-frame receipt: {error}"))?,
    )
    .map_err(|error| format!("write exception-frame receipt: {error}"))?;
    reject_bundle(forge, root, work, "receipt-tamper", &receipt_bundle)?;

    for (name, relative) in [
        ("vstd-source-tamper", "evidence/kernel-vstd-link.rs"),
        ("vstd-rlib-tamper", "artifact/deps/libvstd.rlib"),
    ] {
        let bundle = work.join(format!("{name}.verified"));
        copy_tree(primary, &bundle)?;
        let path = bundle.join(relative);
        let mut bytes =
            fs::read(&path).map_err(|error| format!("read exception-frame {name}: {error}"))?;
        *bytes
            .first_mut()
            .ok_or_else(|| format!("exception-frame {name} target is empty"))? ^= 1;
        fs::write(&path, bytes)
            .map_err(|error| format!("write exception-frame {name}: {error}"))?;
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
        &format!("M1 exception-frame {name} rejection"),
    )?;
    write_output(
        &work.join(format!("negative-{name}.txt")),
        &output,
        "M1 exception-frame tamper evidence",
    )
}
