use super::m1_bootinfo::{validate_candidate_pin, FORGE_SHA256, THERMITE_COMMIT};
use super::m1_elf::{compare_file, read_json, require_file, verify_bundle, write_output};
use super::{
    canonical_json, check_forge_skill, copy_tree, forge_binary, json_string, require_exact_bytes,
    require_output_fragments, run_checked, run_expect_failure, sha256sum, workspace_root,
    write_combined_output,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE: &str = "thermite/platform/exception_policy.th";
const SHELL: &str = "tests/m1/exception_scalar_shell.rs";
const CONSUMER: &str = "tests/m1/exception_scalar_consumer.rs";
const ENTRY_SOURCE: &str = "verus/machine-model/exception_scalar_entry_capsule.rs";
const ENTRY_CONSUMER: &str = "tests/m1/exception_scalar_entry_consumer.rs";
const LINKER: &str = "kernel-host/link/m1_exception_scalar_entry_capsule.ld";
const CRATE_NAME: &str = "tmk_exception_scalar";
const ARTIFACT: &str = "artifact/libtmk_exception_scalar.rlib";
const ENTRY_CRATE: &str = "tmk_exception_scalar_entry";
const ENTRY_RLIB: &str = "libtmk_exception_scalar_entry.rlib";
const RECEIPT_SCHEMA: &str = "thermite.verified-composition-receipt.v1";
const CORE_MARKER: &str = "M1_EXCEPTION_SCALAR_OK scenarios=11 observation=2047 controls=return,schedule,fail-stop actions=fault,terminate,timer,irq,tlb,quarantine,panic";
const ENTRY_MARKER: &str = "M1_EXCEPTION_SCALAR_ENTRY_OK bytes=11 controls=return,schedule,fail-stop rejected=4 core=ffffffff80011300";
const ENTRY_BYTES: &[u8] = &[
    0x49, 0x89, 0xfa, 0x48, 0x89, 0xdf, 0xe9, 0xf5, 0x00, 0x00, 0x00,
];

struct Tools {
    verus: PathBuf,
    rustc: PathBuf,
    ld: PathBuf,
    objcopy: PathBuf,
    objdump: PathBuf,
    readelf: PathBuf,
    nm: PathBuf,
}

impl Tools {
    fn pinned() -> Result<Self, String> {
        let tools = Self {
            verus: PathBuf::from("/opt/verus/0.2026.05.24.ecee80a/verus"),
            rustc: PathBuf::from(
                "/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc",
            ),
            ld: PathBuf::from("/usr/sbin/ld"),
            objcopy: PathBuf::from("/usr/sbin/objcopy"),
            objdump: PathBuf::from("/usr/sbin/objdump"),
            readelf: PathBuf::from("/usr/sbin/readelf"),
            nm: PathBuf::from("/usr/sbin/nm"),
        };
        for (path, expected, label) in [
            (
                tools.verus.as_path(),
                "c5911ee43c7a92c49a48d2c8646c604d252a38c71c87bda88ad4d33eb9e7e0fc",
                "Verus",
            ),
            (
                tools.rustc.as_path(),
                "bff349e72704ff70bc08a234a3847338e797065bbedde5e556808bc87b7bf7c6",
                "Rust 1.95 codegen compiler",
            ),
            (
                tools.ld.as_path(),
                "6cf122245638eb45fd981c75bf3a116675b3f9c7510ae2e3b386aa6738e46505",
                "GNU ld",
            ),
            (
                tools.objcopy.as_path(),
                "8f09a5b2d8e2b34aebf269fffef2308492a022dcfedf87b49489592e838129b4",
                "GNU objcopy",
            ),
            (
                tools.objdump.as_path(),
                "c7c3f8c5c0ed23b2330e148e58624f8d798f1673f4c9fb126ee81096f05e3653",
                "GNU objdump",
            ),
            (
                tools.readelf.as_path(),
                "59d345f2a2b47f5617e8f53c72f6db5169c723c11d3e809a9e6e3c5673f2420c",
                "GNU readelf",
            ),
            (
                tools.nm.as_path(),
                "988d8ded768c4e59284a44f641e92db6c0c8dd222547c32ce432577ff3cb9cc6",
                "GNU nm",
            ),
        ] {
            require_file(path, label)?;
            let actual = sha256sum(path)?;
            if actual != expected {
                return Err(format!("{label} digest is {actual}, expected {expected}"));
            }
        }
        Ok(tools)
    }
}

struct LinkedCapsule {
    elf: PathBuf,
    bytes: PathBuf,
}

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let forge = forge_binary()?;
    let tools = Tools::pinned()?;
    validate_candidate_pin(&forge)?;
    check_forge_skill(&forge)?;
    for (relative, label) in [
        (SOURCE, "accepted exception policy"),
        (SHELL, "scalar policy/action shell"),
        (CONSUMER, "scalar policy/action consumer"),
        (ENTRY_SOURCE, "scalar-entry Verus source"),
        (ENTRY_CONSUMER, "scalar-entry consumer"),
        (LINKER, "scalar-entry linker"),
    ] {
        require_file(&root.join(relative), label)?;
    }
    let shell_text = read(&root.join(SHELL))?;
    let entry_text = read(&root.join(ENTRY_SOURCE))?;
    audit_sources(
        &shell_text,
        &read(&root.join(CONSUMER))?,
        &entry_text,
        &read(&root.join(ENTRY_CONSUMER))?,
        &read(&root.join(LINKER))?,
    )?;

    let work = root.join("build/m1-exception-scalar");
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
            &format!("M1 exception-scalar composition build {}", index + 1),
        )?;
        write_output(
            &work.join(format!("core-build-{}.txt", index + 1)),
            &output,
            "exception-scalar build evidence",
        )?;
    }
    let primary = &bundles[0];
    let receipt = read_json(&primary.join("receipt.json"), "exception-scalar receipt")?;
    validate_receipt(&root, primary, &receipt)?;
    let binding_sha = json_string(&receipt, "/binding_sha256", "exception-scalar binding")?;
    let artifact_sha = json_string(
        &receipt,
        "/binding/artifact/sha256",
        "exception-scalar artifact",
    )?;
    let combined_source_sha = json_string(
        &receipt,
        "/binding/composition/combined_source_sha256",
        "exception-scalar combined source",
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
                "M1 exception-scalar {label} does not match the receipt"
            ));
        }
    }
    fs::write(
        work.join("verify.json"),
        serde_json::to_vec_pretty(&validation)
            .map_err(|error| format!("serialize exception-scalar validation: {error}"))?,
    )
    .map_err(|error| format!("write exception-scalar validation: {error}"))?;
    fs::write(
        work.join("replay.json"),
        serde_json::to_vec_pretty(&replay)
            .map_err(|error| format!("serialize exception-scalar replay: {error}"))?,
    )
    .map_err(|error| format!("write exception-scalar replay: {error}"))?;
    for relative in [
        "receipt.json",
        "evidence/source.verus.rs",
        ARTIFACT,
        "artifact/deps/libvstd.rlib",
        "evidence/kernel-vstd-link.rs",
    ] {
        let expected = fs::read(primary.join(relative))
            .map_err(|error| format!("read primary exception-scalar `{relative}`: {error}"))?;
        for bundle in bundles.iter().skip(1) {
            compare_file(
                bundle,
                relative,
                &expected,
                "exception-scalar reproducible artifact",
            )?;
        }
    }
    for bundle in bundles.iter().skip(1) {
        verify_bundle(&forge, &root, bundle, false)?;
    }
    let core_consumers = run_core_consumers(&tools, &root, &work, &bundles)?;
    let core_consumer_sha = same_digest(&core_consumers, "exception-scalar consumer")?;

    let model_dirs = [
        work.join("entry-model-primary"),
        work.join("entry-model-repro-a"),
        work.join("entry-model-repro-b"),
    ];
    let mut models = Vec::new();
    for (index, directory) in model_dirs.iter().enumerate() {
        models.push(build_entry_model(
            &tools,
            &root.join(ENTRY_SOURCE),
            directory,
            index == 0,
        )?);
    }
    let entry_model_sha = same_digest(&models, "scalar-entry model")?;
    let mut entry_consumers = Vec::new();
    let mut emitted = Vec::new();
    for (index, model) in models.iter().enumerate() {
        let executable = work.join(format!("entry-consumer-{}", index + 1));
        let bytes = work.join(format!("scalar-entry-{}.bin", index + 1));
        compile_entry_consumer(&tools, &root, model, &executable)?;
        let runtime = run_checked(
            Command::new(&executable).current_dir(&root).arg(&bytes),
            "execute scalar-entry model and emit bytes",
        )?;
        require_output_fragments(&runtime.stdout, "scalar-entry runtime", &[ENTRY_MARKER])?;
        require_exact_bytes(&bytes, ENTRY_BYTES, "emitted scalar-entry capsule")?;
        write_combined_output(
            &work.join(format!("entry-runtime-{}.txt", index + 1)),
            &runtime,
            "scalar-entry runtime evidence",
        )?;
        entry_consumers.push(executable);
        emitted.push(bytes);
    }
    let entry_consumer_sha = same_digest(&entry_consumers, "scalar-entry consumer")?;
    let emitted_sha = same_digest(&emitted, "emitted scalar-entry capsule")?;

    let link_dirs = [
        work.join("link-primary"),
        work.join("link-repro-a"),
        work.join("link-repro-b"),
    ];
    let mut linked = Vec::new();
    for (directory, bytes) in link_dirs.iter().zip(emitted.iter()) {
        linked.push(link_capsule(&tools, &root.join(LINKER), bytes, directory)?);
    }
    let linked_bytes_sha = same_digest(
        &linked
            .iter()
            .map(|item| item.bytes.clone())
            .collect::<Vec<_>>(),
        "linked scalar-entry capsule",
    )?;
    let linked_elf_sha = same_digest(
        &linked
            .iter()
            .map(|item| item.elf.clone())
            .collect::<Vec<_>>(),
        "linked scalar-entry ELF",
    )?;
    audit_linked(&tools, &linked[0], &work)?;
    run_core_proof_negatives(&forge, &root, &work, &shell_text)?;
    run_bundle_tamper_negatives(&forge, &root, &work, primary)?;
    run_entry_proof_negatives(&tools, &entry_text, &work)?;
    run_link_negatives(&tools, &root.join(LINKER), &emitted[0], &work)?;

    let report = format!(
        "M1_EXCEPTION_SCALAR_OK\ncomponent_verified=true\nrelease_eligible=false\ncandidate_pin_verified=true\nhardware_executed=false\npolicy_action_model_executed=true\nfixed_address_scalar_entry_present=true\ncr2_retained_in_r10=true\nper_cpu_lookup_wrapper_present=false\nscalar_core_fixed_address_linked=false\nreceipt_validated=true\nreceipt_replayed=true\nsource_sha256={}\nshell_sha256={}\nconsumer_source_sha256={}\ncombined_source_sha256={combined_source_sha}\nreceipt_sha256={}\nbinding_sha256={binding_sha}\nartifact_sha256={artifact_sha}\ncore_consumer_sha256={core_consumer_sha}\nentry_source_sha256={}\nentry_consumer_source_sha256={}\nentry_linker_sha256={}\nentry_model_sha256={entry_model_sha}\nentry_consumer_sha256={entry_consumer_sha}\nemitted_entry_sha256={emitted_sha}\nlinked_entry_sha256={linked_bytes_sha}\nlinked_entry_elf_sha256={linked_elf_sha}\nforge_source_identity={THERMITE_COMMIT}\nforge_sha256={FORGE_SHA256}\ncore_reproducibility_builds=3\nentry_model_reproducibility_builds=3\nentry_consumer_reproducibility_builds=3\npost_link_reproducibility_builds=3\nmutation_battery=64/64\nverus_entry_verified=12\nscalar_entry_virtual=ffffffff80011200\nscalar_core_seam_virtual=ffffffff80011300\ncommon_continuation_virtual=ffffffff80011038\nscalar_entry_bytes=11\nscalar_entry_instruction=mov-rdi-r10;mov-rbx-rdi;tail-jump\ncontrol_values=return:0,schedule:1,fail-stop:2\ncore_runtime_marker={CORE_MARKER}\nentry_runtime_marker={ENTRY_MARKER}\ncore_runtime_scenarios=11\nentry_runtime_controls=3\nentry_runtime_rejections=4\nnegative_cases=argument-binding,policy-rollback,control-map,snapshot-reason,core-bad-assume,receipt-tamper,vstd-source-tamper,vstd-rlib-tamper,entry-frame-argument,entry-tail-transfer,entry-return-target,entry-bad-assume,byte-mutation,extra-byte,unregistered-executable\n",
        sha256sum(&root.join(SOURCE))?,
        sha256sum(&root.join(SHELL))?,
        sha256sum(&root.join(CONSUMER))?,
        sha256sum(&primary.join("receipt.json"))?,
        sha256sum(&root.join(ENTRY_SOURCE))?,
        sha256sum(&root.join(ENTRY_CONSUMER))?,
        sha256sum(&root.join(LINKER))?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M1 exception-scalar report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
}

fn audit_sources(
    shell: &str,
    consumer: &str,
    entry: &str,
    entry_consumer: &str,
    linker: &str,
) -> Result<(), String> {
    for (text, label) in [(shell, "scalar shell"), (entry, "scalar-entry source")] {
        for forbidden in [
            "external_body",
            "assume(",
            "admit(",
            "axiom fn",
            "decreases *",
            "unsafe ",
            "asm!",
            "global_asm!",
        ] {
            if text.contains(forbidden) {
                return Err(format!("{label} contains forbidden `{forbidden}`"));
            }
        }
    }
    for required in [
        "pub fn scalar_arguments_match",
        "pub fn normalize_exception_event",
        "pub fn execute_exception_action",
        "pub fn scalar_dispatch_checked",
        "cpu.unique_state_token",
        "cpu.interrupts_masked",
        "latch_bridge_failure(prior_state, machine, 103, true, true)",
        "result.action_code == 0 ==>",
        "result.policy_state.irq_deliveries == prior_state.irq_deliveries",
        "CONTROL_RETURN",
        "CONTROL_SCHEDULE",
        "CONTROL_FAIL_STOP",
    ] {
        if !shell.contains(required) {
            return Err(format!("scalar shell is missing `{required}`"));
        }
    }
    for required in [
        "M1_EXCEPTION_SCALAR_OK scenarios=11",
        "backend.policy_state.irq_deliveries, 0",
        "overflow.policy_state.last_tlb_epoch, 0",
        "observation, 2047",
    ] {
        if !consumer.contains(required) {
            return Err(format!("scalar consumer is missing `{required}`"));
        }
    }
    for required in [
        "pub const SCALAR_ENTRY_VIRTUAL: u64 = 0xffff_ffff_8001_1200;",
        "pub const SCALAR_CORE_VIRTUAL: u64 = 0xffff_ffff_8001_1300;",
        "pub const REGISTERED_QWORD: u64 = 0xf5e9_df89_48fa_8949;",
        "pub const REGISTERED_TAIL: u32 = 0;",
        "pub open spec fn scalar_entry_precondition",
        "pub fn decode_execute",
        "result.arguments.frame == state.rbx_frame",
        "result.arguments.cr2 == state.rdi_cr2",
        "result.core_r10_cr2 == state.rdi_cr2",
        "result.stack_neutral_tail_jump",
        "result.post_rip == COMMON_CONTINUATION",
        "ensures result == 511",
    ] {
        if !entry.contains(required) {
            return Err(format!("scalar-entry source is missing `{required}`"));
        }
    }
    for required in [
        "to_le_bytes()",
        "state(CONTROL_RETURN)",
        "state(CONTROL_SCHEDULE)",
        "state(CONTROL_FAIL_STOP)",
        "core_registered: false",
        "core_preserves_frame: false",
        "M1_EXCEPTION_SCALAR_ENTRY_OK bytes=11",
    ] {
        if !entry_consumer.contains(required) {
            return Err(format!("scalar-entry consumer is missing `{required}`"));
        }
    }
    for required in [
        "ENTRY(tmk_exception_scalar_entry)",
        ". = 0xffffffff80011200;",
        ".text.tmk_exception_scalar_entry",
        "tmk_exception_scalar_core = 0xffffffff80011300;",
        "SIZEOF(.text.tmk_exception_scalar_entry) == 11",
    ] {
        if !linker.contains(required) {
            return Err(format!("scalar-entry linker is missing `{required}`"));
        }
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
        "M1 exception-scalar Thermite audit",
    )?;
    require_output_fragments(
        &audit.stdout,
        "M1 exception-scalar audit",
        &[
            "\"level\": \"L3\"",
            "\"mutants_killed\": \"64/64\"",
            "\"kind\": \"end_to_end\"",
            "\"slag\": false",
        ],
    )?;
    write_output(&work.join("audit.txt"), &audit, "exception-scalar audit")?;
    let battery = run_checked(
        Command::new(forge)
            .current_dir(root)
            .args(["battery", SOURCE, "exception_policy_step"]),
        "M1 exception-scalar policy battery",
    )?;
    require_output_fragments(
        &battery.stdout,
        "M1 exception-scalar policy battery",
        &["non-vacuous", "64/64"],
    )?;
    write_output(
        &work.join("battery.txt"),
        &battery,
        "exception-scalar battery",
    )
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

fn validate_receipt(root: &Path, bundle: &Path, receipt: &Value) -> Result<(), String> {
    if json_string(receipt, "/schema", "exception-scalar receipt schema")? != RECEIPT_SCHEMA
        || json_string(
            receipt,
            "/binding/schema",
            "exception-scalar binding schema",
        )? != RECEIPT_SCHEMA
        || json_string(receipt, "/binding/scope", "exception-scalar scope")? != "end_to_end"
        || json_string(receipt, "/binding/target", "exception-scalar target")? != "kernel"
        || json_string(receipt, "/binding/crate_name", "exception-scalar crate")? != CRATE_NAME
    {
        return Err("M1 exception-scalar receipt identity is not accepted".to_string());
    }
    let toolchain = read_json(
        &bundle.join("evidence/toolchain.json"),
        "exception-scalar toolchain",
    )?;
    if json_string(
        &toolchain,
        "/forge_source_identity",
        "exception-scalar Forge identity",
    )? != THERMITE_COMMIT
        || json_string(
            &toolchain,
            "/forge_executable_sha256",
            "exception-scalar Forge digest",
        )? != FORGE_SHA256
    {
        return Err("M1 exception-scalar is not bound to the candidate Forge pin".to_string());
    }
    for (relative, expected, label) in [
        (
            "evidence/input.th",
            sha256sum(&root.join(SOURCE))?,
            "Thermite source",
        ),
        (
            "evidence/direct-verus/00-exception_scalar_shell.rs",
            sha256sum(&root.join(SHELL))?,
            "direct-Verus shell",
        ),
        (
            ARTIFACT,
            json_string(
                receipt,
                "/binding/artifact/sha256",
                "exception-scalar artifact",
            )?
            .to_string(),
            "artifact",
        ),
    ] {
        let actual = sha256sum(&bundle.join(relative))?;
        if actual != expected {
            return Err(format!(
                "M1 exception-scalar {label} digest is {actual}, expected {expected}"
            ));
        }
    }
    let combined = read(&bundle.join("evidence/source.verus.rs"))?;
    for forbidden in ["external_body", "assume(", "admit(", "decreases *"] {
        if combined.contains(forbidden) {
            return Err(format!(
                "exception-scalar combined source contains `{forbidden}`"
            ));
        }
    }
    Ok(())
}

fn run_core_consumers(
    tools: &Tools,
    root: &Path,
    work: &Path,
    bundles: &[PathBuf; 3],
) -> Result<Vec<PathBuf>, String> {
    let mut executables = Vec::new();
    for (index, bundle) in bundles.iter().enumerate() {
        let executable = work.join(format!("core-consumer-{}", index + 1));
        run_checked(
            Command::new(&tools.rustc)
                .current_dir(root)
                .env("SOURCE_DATE_EPOCH", "0")
                .args(["--edition=2021"])
                .arg(CONSUMER)
                .arg("--extern")
                .arg(format!("{CRATE_NAME}={}", bundle.join(ARTIFACT).display()))
                .arg("-L")
                .arg(format!(
                    "dependency={}",
                    bundle.join("artifact/deps").display()
                ))
                .args(["-C", "panic=abort"])
                .args(["-C", "codegen-units=1"])
                .arg(format!("--remap-path-prefix={}=.", root.display()))
                .arg("-o")
                .arg(&executable),
            "compile exception-scalar runtime consumer",
        )?;
        let runtime = run_checked(
            Command::new(&executable).current_dir(root),
            "execute exception-scalar runtime consumer",
        )?;
        require_output_fragments(&runtime.stdout, "exception-scalar runtime", &[CORE_MARKER])?;
        write_combined_output(
            &work.join(format!("core-runtime-{}.txt", index + 1)),
            &runtime,
            "exception-scalar runtime evidence",
        )?;
        executables.push(executable);
    }
    Ok(executables)
}

fn verus_command(tools: &Tools, directory: &Path, compile: bool) -> Command {
    let mut command = Command::new(&tools.verus);
    command
        .current_dir(directory)
        .env("SOURCE_DATE_EPOCH", "0")
        .args([
            "--output-json",
            "--no-vstd",
            "--no-cheating",
            "--multiple-errors",
            "20",
        ]);
    if compile {
        command.arg("--compile");
    }
    command
        .args(["--rlimit", "120"])
        .args(["--smt-option", "smt.random_seed=1"])
        .args(["-C", "panic=abort"])
        .args(["-C", "overflow-checks=off"])
        .args(["-C", "relocation-model=static"])
        .args(["-C", "no-redzone=yes"])
        .arg(format!("--remap-path-prefix={}=.", directory.display()))
        .arg(format!("{ENTRY_CRATE}.rs"));
    command
}

fn build_entry_model(
    tools: &Tools,
    source: &Path,
    directory: &Path,
    retain_result: bool,
) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create scalar-entry model path: {error}"))?;
    fs::copy(source, directory.join(format!("{ENTRY_CRATE}.rs")))
        .map_err(|error| format!("stage scalar-entry model: {error}"))?;
    let output = run_checked(
        &mut verus_command(tools, directory, true),
        "Verus scalar-entry proof and codegen",
    )?;
    require_output_fragments(
        &output.stdout,
        "Verus scalar-entry proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 12",
            "\"errors\": 0",
            "\"is-verifying-entire-crate\": true",
            "\"version\": \"0.2026.05.24.ecee80a\"",
        ],
    )?;
    if retain_result {
        fs::write(
            directory.join("verus-result.json"),
            canonical_json(&output.stdout, "scalar-entry Verus result")?,
        )
        .map_err(|error| format!("write scalar-entry Verus result: {error}"))?;
    }
    let artifact = directory.join(ENTRY_RLIB);
    require_file(&artifact, "compiled scalar-entry model")?;
    Ok(artifact)
}

fn compile_entry_consumer(
    tools: &Tools,
    root: &Path,
    artifact: &Path,
    executable: &Path,
) -> Result<(), String> {
    run_checked(
        Command::new(&tools.rustc)
            .current_dir(root)
            .env("SOURCE_DATE_EPOCH", "0")
            .args(["--edition=2021"])
            .arg(ENTRY_CONSUMER)
            .arg("--extern")
            .arg(format!("{ENTRY_CRATE}={}", artifact.display()))
            .args(["-L", "dependency=/opt/verus/0.2026.05.24.ecee80a"])
            .args(["-C", "panic=abort"])
            .args(["-C", "relocation-model=static"])
            .args(["-C", "codegen-units=1"])
            .arg(format!("--remap-path-prefix={}=.", root.display()))
            .arg("-o")
            .arg(executable),
        "compile scalar-entry consumer",
    )?;
    Ok(())
}

fn same_digest(paths: &[PathBuf], label: &str) -> Result<String, String> {
    let expected = sha256sum(&paths[0])?;
    for path in paths.iter().skip(1) {
        let actual = sha256sum(path)?;
        if actual != expected {
            return Err(format!(
                "{label} {} is {actual}, expected {expected}",
                path.display()
            ));
        }
    }
    Ok(expected)
}

fn wrap_bytes(
    tools: &Tools,
    directory: &Path,
    name: &str,
    section: &str,
) -> Result<PathBuf, String> {
    let object = directory.join(format!("{name}.o"));
    run_checked(
        Command::new(&tools.objcopy)
            .current_dir(directory)
            .args(["-I", "binary", "-O", "elf64-x86-64", "-B", "i386:x86-64"])
            .arg("--rename-section")
            .arg(format!(".data={section},alloc,load,readonly,code,contents"))
            .arg(format!("{name}.bin"))
            .arg(format!("{name}.o")),
        "wrap scalar-entry bytes",
    )?;
    Ok(object)
}

fn link_capsule(
    tools: &Tools,
    linker: &Path,
    bytes: &Path,
    directory: &Path,
) -> Result<LinkedCapsule, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create scalar-entry link path: {error}"))?;
    fs::copy(bytes, directory.join("scalar.bin"))
        .map_err(|error| format!("stage scalar-entry bytes: {error}"))?;
    let object = wrap_bytes(
        tools,
        directory,
        "scalar",
        ".text.tmk_exception_scalar_entry",
    )?;
    let elf = directory.join("scalar.elf");
    run_checked(
        Command::new(&tools.ld)
            .current_dir(directory)
            .args([
                "-m",
                "elf_x86_64",
                "--build-id=none",
                "-nostdlib",
                "-static",
            ])
            .arg("-T")
            .arg(linker)
            .args(["-o", "scalar.elf"])
            .arg(object),
        "link scalar-entry ELF",
    )?;
    let linked_bytes = directory.join("linked.bin");
    run_checked(
        Command::new(&tools.objcopy)
            .current_dir(directory)
            .arg("--dump-section")
            .arg(format!(
                ".text.tmk_exception_scalar_entry={}",
                linked_bytes.display()
            ))
            .arg(&elf),
        "extract linked scalar-entry bytes",
    )?;
    require_exact_bytes(&linked_bytes, ENTRY_BYTES, "linked scalar-entry capsule")?;
    Ok(LinkedCapsule {
        elf,
        bytes: linked_bytes,
    })
}

fn audit_linked(tools: &Tools, linked: &LinkedCapsule, work: &Path) -> Result<(), String> {
    let header = run_checked(
        Command::new(&tools.readelf).args(["-hW"]).arg(&linked.elf),
        "inspect scalar-entry ELF header",
    )?;
    require_output_fragments(
        &header.stdout,
        "scalar-entry ELF header",
        &[
            "ELF64",
            "Advanced Micro Devices X86-64",
            "0xffffffff80011200",
        ],
    )?;
    let sections = run_checked(
        Command::new(&tools.readelf).args(["-SW"]).arg(&linked.elf),
        "inspect scalar-entry sections",
    )?;
    let sections_text = String::from_utf8_lossy(&sections.stdout);
    let executable: Vec<_> = sections_text
        .lines()
        .filter(|line| line.contains(" AX "))
        .collect();
    if executable.len() != 1
        || !executable[0].contains(".text.tmk_exception_scalar_entry")
        || !executable[0].contains("ffffffff80011200")
        || !executable[0].contains("00000b")
    {
        return Err(format!(
            "scalar-entry executable-section audit failed: {executable:?}"
        ));
    }
    let relocations = run_checked(
        Command::new(&tools.readelf).args(["-rW"]).arg(&linked.elf),
        "inspect scalar-entry relocations",
    )?;
    require_output_fragments(
        &relocations.stdout,
        "scalar-entry relocations",
        &["There are no relocations in this file."],
    )?;
    let symbols = run_checked(
        Command::new(&tools.nm).arg("-n").arg(&linked.elf),
        "inspect scalar-entry symbols",
    )?;
    require_output_fragments(
        &symbols.stdout,
        "scalar-entry symbols",
        &[
            "ffffffff80011200 T tmk_exception_scalar_entry",
            "ffffffff80011300 A tmk_exception_scalar_core",
        ],
    )?;
    let disassembly = run_checked(
        Command::new(&tools.objdump).arg("-d").arg(&linked.elf),
        "disassemble scalar-entry capsule",
    )?;
    require_output_fragments(
        &disassembly.stdout,
        "scalar-entry disassembly",
        &[
            "mov    %rdi,%r10",
            "mov    %rbx,%rdi",
            "jmp    ffffffff80011300",
        ],
    )?;
    for (name, output) in [
        ("linked-header.txt", &header),
        ("linked-sections.txt", &sections),
        ("linked-relocations.txt", &relocations),
        ("linked-symbols.txt", &symbols),
        ("linked-disassembly.txt", &disassembly),
    ] {
        write_combined_output(&work.join(name), output, "scalar-entry linked evidence")?;
    }
    Ok(())
}

fn run_core_proof_negatives(
    forge: &Path,
    root: &Path,
    work: &Path,
    shell: &str,
) -> Result<(), String> {
    let cases = [
        (
            "argument-binding",
            shell.replacen("args.cr2 == words@[14]", "args.cr2 == words@[13]", 1),
        ),
        (
            "policy-rollback",
            shell.replacen(
                "result.policy_state.irq_deliveries == prior_state.irq_deliveries",
                "result.policy_state.irq_deliveries == prior_state.irq_deliveries + 1",
                1,
            ),
        ),
        (
            "control-map",
            shell.replacen(
                "result.action_code == 6 || result.action_code == 7 ==>\n            result.control == CONTROL_RETURN,",
                "result.action_code == 6 || result.action_code == 7 ==>\n            result.control == CONTROL_SCHEDULE,",
                1,
            ),
        ),
        (
            "snapshot-reason",
            shell.replacen(
                "result.machine.crash_reason == 100,",
                "result.machine.crash_reason == 102,",
                1,
            ),
        ),
        (
            "core-bad-assume",
            shell.replacen(
                "spec_snapshot_valid(cpu, context) && spec_scalar_arguments_match(words, args)\n            ==> result.policy_invoked,\n{\n    if !(",
                "spec_snapshot_valid(cpu, context) && spec_scalar_arguments_match(words, args)\n            ==> result.policy_invoked,\n{\n    assume(false);\n    if !(",
                1,
            ),
        ),
    ];
    for (name, mutated) in cases {
        if mutated == shell {
            return Err(format!(
                "could not construct exception-scalar `{name}` negative"
            ));
        }
        let path = work.join(format!("core-negative-{name}.rs"));
        fs::write(&path, mutated)
            .map_err(|error| format!("write exception-scalar {name} negative: {error}"))?;
        let bundle = work.join(format!("core-negative-{name}.verified"));
        let output = run_expect_failure(
            &mut build_command(forge, root, &path, &bundle),
            &format!("M1 exception-scalar {name} negative"),
        )?;
        if bundle.exists() {
            return Err(format!("exception-scalar {name} negative emitted a bundle"));
        }
        write_output(
            &work.join(format!("negative-{name}.txt")),
            &output,
            "exception-scalar negative evidence",
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
    let mut receipt = read_json(&receipt_path, "exception-scalar tamper receipt")?;
    let digest = json_string(
        &receipt,
        "/binding_sha256",
        "exception-scalar tamper digest",
    )?;
    let replacement = format!(
        "{}{}",
        if digest.starts_with('0') { '1' } else { '0' },
        &digest[1..]
    );
    *receipt
        .pointer_mut("/binding_sha256")
        .ok_or_else(|| "exception-scalar receipt has no binding digest".to_string())? =
        Value::String(replacement);
    fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("serialize exception-scalar receipt: {error}"))?,
    )
    .map_err(|error| format!("write exception-scalar receipt: {error}"))?;
    reject_bundle(forge, root, work, "receipt-tamper", &receipt_bundle)?;
    for (name, relative) in [
        ("vstd-source-tamper", "evidence/kernel-vstd-link.rs"),
        ("vstd-rlib-tamper", "artifact/deps/libvstd.rlib"),
    ] {
        let bundle = work.join(format!("{name}.verified"));
        copy_tree(primary, &bundle)?;
        let path = bundle.join(relative);
        let mut bytes =
            fs::read(&path).map_err(|error| format!("read exception-scalar {name}: {error}"))?;
        *bytes
            .first_mut()
            .ok_or_else(|| format!("exception-scalar {name} target is empty"))? ^= 1;
        fs::write(&path, bytes)
            .map_err(|error| format!("write exception-scalar {name}: {error}"))?;
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
        &format!("M1 exception-scalar {name} rejection"),
    )?;
    write_output(
        &work.join(format!("negative-{name}.txt")),
        &output,
        "exception-scalar tamper evidence",
    )
}

fn run_entry_proof_negatives(tools: &Tools, source: &str, work: &Path) -> Result<(), String> {
    let cases = [
        (
            "entry-frame-argument",
            source.replacen(
                "result.accepted ==> result.arguments.frame == state.rbx_frame,",
                "result.accepted ==> result.arguments.frame == state.rdi_cr2,",
                1,
            ),
        ),
        (
            "entry-tail-transfer",
            source.replacen(
                "result.accepted ==> result.stack_neutral_tail_jump,",
                "result.accepted ==> !result.stack_neutral_tail_jump,",
                1,
            ),
        ),
        (
            "entry-return-target",
            source.replacen(
                "result.accepted && result.returns_to_common ==> result.post_rip == COMMON_CONTINUATION,",
                "result.accepted && result.returns_to_common ==> result.post_rip == SCALAR_CORE_VIRTUAL,",
                1,
            ),
        ),
        (
            "entry-bad-assume",
            source.replacen(
                "pub fn scalar_entry_observation() -> (result: u64)\n    ensures result == 511,\n{",
                "pub fn scalar_entry_observation() -> (result: u64)\n    ensures result == 511,\n{\n    assume(false);",
                1,
            ),
        ),
    ];
    for (name, mutated) in cases {
        if mutated == source {
            return Err(format!(
                "could not construct scalar-entry `{name}` negative"
            ));
        }
        let directory = work.join(format!("proof-negative-{name}"));
        fs::create_dir_all(&directory)
            .map_err(|error| format!("create scalar-entry negative path: {error}"))?;
        fs::write(directory.join(format!("{ENTRY_CRATE}.rs")), mutated)
            .map_err(|error| format!("write scalar-entry {name} negative: {error}"))?;
        let output = run_expect_failure(
            &mut verus_command(tools, &directory, false),
            &format!("scalar-entry {name} proof negative"),
        )?;
        write_combined_output(
            &work.join(format!("negative-{name}.txt")),
            &output,
            "scalar-entry proof-negative evidence",
        )?;
    }
    Ok(())
}

fn run_link_negatives(
    tools: &Tools,
    linker: &Path,
    emitted: &Path,
    work: &Path,
) -> Result<(), String> {
    let mut mutated = fs::read(emitted)
        .map_err(|error| format!("read scalar-entry bytes for mutation: {error}"))?;
    mutated[0] ^= 1;
    let mutation_path = work.join("mutated-entry.bin");
    fs::write(&mutation_path, mutated)
        .map_err(|error| format!("write scalar-entry mutation: {error}"))?;
    match require_exact_bytes(&mutation_path, ENTRY_BYTES, "mutated scalar-entry capsule") {
        Ok(()) => return Err("scalar-entry byte mutation was accepted".to_string()),
        Err(error) => fs::write(work.join("negative-byte-mutation.txt"), error)
            .map_err(|write_error| format!("write byte-mutation evidence: {write_error}"))?,
    }

    let extra = work.join("link-negative-extra-byte");
    fs::create_dir_all(&extra).map_err(|error| format!("create extra-byte path: {error}"))?;
    let mut extra_bytes = ENTRY_BYTES.to_vec();
    extra_bytes.push(0x90);
    fs::write(extra.join("scalar.bin"), extra_bytes)
        .map_err(|error| format!("write extra-byte capsule: {error}"))?;
    let object = wrap_bytes(tools, &extra, "scalar", ".text.tmk_exception_scalar_entry")?;
    let output = run_expect_failure(
        Command::new(&tools.ld)
            .current_dir(&extra)
            .args([
                "-m",
                "elf_x86_64",
                "--build-id=none",
                "-nostdlib",
                "-static",
            ])
            .arg("-T")
            .arg(linker)
            .args(["-o", "scalar.elf"])
            .arg(object),
        "scalar-entry extra-byte link negative",
    )?;
    write_combined_output(
        &work.join("negative-extra-byte.txt"),
        &output,
        "scalar-entry extra-byte evidence",
    )?;

    let unregistered = work.join("link-negative-unregistered-executable");
    fs::create_dir_all(&unregistered)
        .map_err(|error| format!("create unregistered-executable path: {error}"))?;
    fs::copy(emitted, unregistered.join("scalar.bin"))
        .map_err(|error| format!("stage scalar-entry negative bytes: {error}"))?;
    fs::write(unregistered.join("extra.bin"), [0x90])
        .map_err(|error| format!("write unregistered executable byte: {error}"))?;
    let scalar = wrap_bytes(
        tools,
        &unregistered,
        "scalar",
        ".text.tmk_exception_scalar_entry",
    )?;
    let extra_object = wrap_bytes(tools, &unregistered, "extra", ".text.unregistered")?;
    let elf = unregistered.join("scalar.elf");
    run_checked(
        Command::new(&tools.ld)
            .current_dir(&unregistered)
            .args([
                "-m",
                "elf_x86_64",
                "--build-id=none",
                "-nostdlib",
                "-static",
            ])
            .arg("-T")
            .arg(linker)
            .args(["-o", "scalar.elf"])
            .arg(scalar)
            .arg(extra_object),
        "link scalar-entry unregistered-executable negative",
    )?;
    let negative = LinkedCapsule {
        elf,
        bytes: emitted.to_path_buf(),
    };
    match audit_linked(tools, &negative, &unregistered) {
        Ok(()) => return Err("unregistered scalar-entry executable section was accepted".into()),
        Err(error) => fs::write(work.join("negative-unregistered-executable.txt"), error).map_err(
            |write_error| format!("write unregistered-executable evidence: {write_error}"),
        )?,
    }
    Ok(())
}
