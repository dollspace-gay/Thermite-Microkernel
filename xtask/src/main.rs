mod composition;
mod idl;
mod m0_uefi;
mod m1_address;
mod m1_bootinfo;
mod m1_cr3;
mod m1_descriptor_install;
mod m1_descriptors;
mod m1_elf;
mod m1_exception_common;
mod m1_exception_dispatcher_front;
mod m1_exception_entry_dispatcher_join;
mod m1_exception_frame;
mod m1_exception_policy;
mod m1_exception_scalar;
mod m1_exception_scalar_core_wrapper;
mod m1_exception_stubs;
mod m1_firmware;
mod m1_firmware_raw_map;
mod m1_page_tables;
mod m1_uefi_gateway;
mod m1_uefi_raw_map;
mod manifest;
mod platform_primitives;
mod uefi;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const EXPECTED_RUNTIME_MARKER: &str = "M0_FORGE_PROBE_OK:5aa512cb9889ff00";

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("m0-composition") if args.next().is_none() => composition::run(),
        Some("m0-composition-source-check") if args.next().is_none() => {
            m0_composition_source_check()
        }
        Some("m0-idl") if args.next().is_none() => m0_idl(),
        Some("m0-manifest") if args.next().is_none() => m0_manifest(),
        Some("m0-platform-primitives") if args.next().is_none() => platform_primitives::run(),
        Some("m0-uefi") if args.next().is_none() => m0_uefi::run(),
        Some("m0-forge-probe") if args.next().is_none() => m0_forge_probe(),
        Some("m0-forge-tamper") if args.next().is_none() => m0_forge_tamper(),
        Some("m0-host-link") if args.next().is_none() => m0_host_link(),
        Some("m0-verus-allocator") if args.next().is_none() => m0_verus_allocator(),
        Some("m0-verus-byte-allocator") if args.next().is_none() => m0_verus_byte_allocator(),
        Some("m1-elf") if args.next().is_none() => m1_elf::run(),
        Some("m1-firmware") if args.next().is_none() => m1_firmware::run(),
        Some("m1-firmware-raw-map") if args.next().is_none() => m1_firmware_raw_map::run(),
        Some("m1-uefi-gateway") if args.next().is_none() => m1_uefi_gateway::run(),
        Some("m1-uefi-raw-map") if args.next().is_none() => m1_uefi_raw_map::run(),
        Some("m1-address") if args.next().is_none() => m1_address::run(),
        Some("m1-bootinfo") if args.next().is_none() => m1_bootinfo::run(),
        Some("m1-page-tables") if args.next().is_none() => m1_page_tables::run(),
        Some("m1-cr3") if args.next().is_none() => m1_cr3::run(),
        Some("m1-descriptor-install") if args.next().is_none() => m1_descriptor_install::run(),
        Some("m1-descriptors") if args.next().is_none() => m1_descriptors::run(),
        Some("m1-exception-stubs") if args.next().is_none() => m1_exception_stubs::run(),
        Some("m1-exception-common") if args.next().is_none() => m1_exception_common::run(),
        Some("m1-exception-dispatcher-front") if args.next().is_none() => {
            m1_exception_dispatcher_front::run()
        }
        Some("m1-exception-entry-dispatcher-join") if args.next().is_none() => {
            m1_exception_entry_dispatcher_join::run()
        }
        Some("m1-exception-frame") if args.next().is_none() => m1_exception_frame::run(),
        Some("m1-exception-policy") if args.next().is_none() => m1_exception_policy::run(),
        Some("m1-exception-scalar") if args.next().is_none() => m1_exception_scalar::run(),
        Some("m1-exception-scalar-core-wrapper") if args.next().is_none() => {
            m1_exception_scalar_core_wrapper::run()
        }
        Some("m0-verus-capsule") if args.next().is_none() => m0_verus_capsule(),
        Some("toolchain-check") if args.next().is_none() => toolchain_check(),
        _ => Err(
            "usage: cargo run -p xtask -- <toolchain-check|m0-idl|m0-manifest|m0-uefi|m0-forge-probe|m0-forge-tamper|m0-composition-source-check|m0-composition|m0-verus-allocator|m0-verus-byte-allocator|m0-verus-capsule|m0-platform-primitives|m0-host-link|m1-elf|m1-firmware|m1-firmware-raw-map|m1-uefi-gateway|m1-uefi-raw-map|m1-address|m1-bootinfo|m1-page-tables|m1-cr3|m1-descriptors|m1-descriptor-install|m1-exception-stubs|m1-exception-common|m1-exception-dispatcher-front|m1-exception-entry-dispatcher-join|m1-exception-policy|m1-exception-frame|m1-exception-scalar|m1-exception-scalar-core-wrapper>"
                .to_string(),
        ),
    }
}

fn toolchain_check() -> Result<(), String> {
    let root = workspace_root()?;
    let checksums = root.join("toolchain/SHA256SUMS");
    let output = run_checked(
        Command::new("sha256sum").arg("--check").arg(&checksums),
        "toolchain binary digest verification",
    )?;
    print!("{}", String::from_utf8_lossy(&output.stdout));

    let thermite_root = env::var_os("THERMITE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/doll/Thermite"));
    let output = run_checked(
        Command::new("git")
            .arg("-C")
            .arg(&thermite_root)
            .args(["rev-parse", "HEAD"]),
        "Thermite source revision check",
    )?;
    let actual = String::from_utf8_lossy(&output.stdout);
    let expected = "845d684f00e829491ee4c537818fba2689bcaefc";
    if actual.trim() != expected {
        return Err(format!(
            "Thermite revision is {}, expected {expected}",
            actual.trim()
        ));
    }

    run_checked(
        Command::new(thermite_root.join("target/debug/forge")).args([
            "skill",
            "--check",
            "/home/doll/.codex/skills/thermite/references/language.md",
        ]),
        "Thermite generated-skill freshness",
    )?;

    for (path, required, label) in [
        (
            "/home/doll/.rustup/toolchains/1.96.0-x86_64-unknown-linux-gnu/bin/rustc",
            "release: 1.96.0",
            "host rustc",
        ),
        (
            "/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc",
            "release: 1.95.0",
            "Forge codegen rustc",
        ),
    ] {
        let output = run_checked(Command::new(path).arg("-vV"), label)?;
        let version = String::from_utf8_lossy(&output.stdout);
        if !version.contains(required) {
            return Err(format!("{label} output does not contain `{required}`"));
        }
    }

    let output = run_checked(
        Command::new("/opt/verus/0.2026.05.24.ecee80a/verus").arg("--version"),
        "Verus version check",
    )?;
    let verus = String::from_utf8_lossy(&output.stdout);
    for required in [
        "Version: 0.2026.05.24.ecee80a",
        "Toolchain: 1.95.0-x86_64-unknown-linux-gnu",
    ] {
        if !verus.contains(required) {
            return Err(format!("Verus output does not contain `{required}`"));
        }
    }

    println!("M0_TOOLCHAIN_OK");
    Ok(())
}

fn m0_idl() -> Result<(), String> {
    let root = workspace_root()?;
    let rustc =
        PathBuf::from("/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc");
    let cc = PathBuf::from("/usr/sbin/cc");
    for (path, expected, label) in [
        (
            rustc.as_path(),
            "bff349e72704ff70bc08a234a3847338e797065bbedde5e556808bc87b7bf7c6",
            "IDL Rust compiler",
        ),
        (
            cc.as_path(),
            "1ce580ecfabf35747bc550481621e2f2c04fd8fc23b8182779f33b82d07856d0",
            "IDL C compiler",
        ),
    ] {
        require_file(path, label)?;
        let actual = sha256sum(path)?;
        if actual != expected {
            return Err(format!("{label} digest is {actual}, expected {expected}"));
        }
    }

    let source = root.join("abi/kernel.idl");
    let rust_consumer_source = root.join("tests/m0/idl_rust_consumer.rs");
    let nostd_consumer_source = root.join("tests/m0/idl_nostd_consumer.rs");
    let c_consumer_source = root.join("tests/m0/idl_c_consumer.c");
    for (path, label) in [
        (&source, "kernel IDL"),
        (&rust_consumer_source, "IDL Rust consumer"),
        (&nostd_consumer_source, "IDL no-std consumer"),
        (&c_consumer_source, "IDL C consumer"),
    ] {
        require_file(path, label)?;
    }
    let source_text = fs::read_to_string(&source)
        .map_err(|error| format!("read kernel IDL {}: {error}", source.display()))?;
    let source_value: serde_json::Value = serde_json::from_str(&source_text)
        .map_err(|error| format!("parse kernel IDL {}: {error}", source.display()))?;
    let source_sha = sha256sum(&source)?;
    let generator_sha = sha256sum(&root.join("xtask/src/idl.rs"))?;

    let work = root.join("build/m0-idl");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;

    let generated_dir = work.join("generated");
    let generated = idl::generate_file(&source, &generated_dir)?;
    idl::check_outputs(&generated, &generated_dir)?;

    let rust_sha = sha256sum(&generated_dir.join("kernel_abi.rs"))?;
    let c_sha = sha256sum(&generated_dir.join("kernel_abi.h"))?;
    let canonical_sha = sha256sum(&generated_dir.join("kernel_abi.canonical.json"))?;
    for name in ["repro-a", "repro-b"] {
        let repro_dir = work.join(name);
        let reproduced = idl::generate_file(&source, &repro_dir)?;
        idl::check_outputs(&reproduced, &repro_dir)?;
        for (file, expected) in [
            ("kernel_abi.rs", rust_sha.as_str()),
            ("kernel_abi.h", c_sha.as_str()),
            ("kernel_abi.canonical.json", canonical_sha.as_str()),
        ] {
            let actual = sha256sum(&repro_dir.join(file))?;
            if actual != expected {
                return Err(format!(
                    "IDL generation in {name} produced {file} digest {actual}, expected {expected}"
                ));
            }
        }
    }

    let generated_rust = generated_dir.join("kernel_abi.rs");
    let rust_consumer = work.join("idl-rust-consumer");
    run_checked(
        Command::new(&rustc)
            .current_dir(&root)
            .env("TMK_IDL_RS", &generated_rust)
            .args(["--edition=2021"])
            .arg(&rust_consumer_source)
            .args(["-C", "panic=abort"])
            .arg("-o")
            .arg(&rust_consumer),
        "compile generated Rust ABI consumer",
    )?;
    let rust_runtime = run_checked(
        Command::new(&rust_consumer).current_dir(&root),
        "execute generated Rust ABI consumer",
    )?;
    let expected_rust_marker = "M0_IDL_RUST_OK:1024:536:680:0001123400560204";
    require_output_fragments(
        &rust_runtime.stdout,
        "generated Rust ABI runtime",
        &[expected_rust_marker],
    )?;
    write_combined_output(
        &work.join("rust-runtime.txt"),
        &rust_runtime,
        "generated Rust ABI runtime",
    )?;

    let nostd_consumer = work.join("libtmk_idl_nostd.rlib");
    run_checked(
        Command::new(&rustc)
            .current_dir(&root)
            .env("TMK_IDL_RS", &generated_rust)
            .args([
                "--edition=2021",
                "--crate-name",
                "tmk_idl_nostd",
                "--crate-type",
                "rlib",
            ])
            .arg(&nostd_consumer_source)
            .args(["-C", "panic=abort"])
            .arg("-o")
            .arg(&nostd_consumer),
        "compile generated ABI in no-std consumer",
    )?;

    let c_consumer = work.join("idl-c-consumer");
    run_checked(
        Command::new(&cc)
            .current_dir(&root)
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-I"])
            .arg(&generated_dir)
            .arg(&c_consumer_source)
            .arg("-o")
            .arg(&c_consumer),
        "compile generated C ABI consumer",
    )?;
    let c_runtime = run_checked(
        Command::new(&c_consumer).current_dir(&root),
        "execute generated C ABI consumer",
    )?;
    let expected_c_marker = "M0_IDL_C_OK:1024:536:680:0001123400560204";
    require_output_fragments(
        &c_runtime.stdout,
        "generated C ABI runtime",
        &[expected_c_marker],
    )?;
    write_combined_output(
        &work.join("c-runtime.txt"),
        &c_runtime,
        "generated C ABI runtime",
    )?;

    let mut negative_results = String::new();
    for (label, pointer, replacement, expected) in [
        (
            "duplicate-syscall-number",
            "/syscalls/1/number",
            serde_json::json!(0),
            "duplicate name or number",
        ),
        (
            "bad-struct-offset",
            "/structs/1/fields/7/offset",
            serde_json::json!(25),
            "offset is 25, expected 24",
        ),
        (
            "overlapping-bitfield",
            "/bitfields/0/fields/1/lsb",
            serde_json::json!(7),
            "overlaps, leaves a gap",
        ),
        (
            "unknown-wire-type",
            "/structs/0/fields/0/type",
            serde_json::json!("u128"),
            "unknown or forward type `u128`",
        ),
    ] {
        let diagnostic = reject_idl_mutation(&source_value, pointer, replacement, label, expected)?;
        negative_results.push_str(&format!("{label}: {diagnostic}\n"));
    }

    let tampered_dir = work.join("tampered-generated");
    copy_tree(&generated_dir, &tampered_dir)?;
    append_tamper_byte(&tampered_dir.join("kernel_abi.rs"))?;
    let tamper_diagnostic = match idl::check_outputs(&generated, &tampered_dir) {
        Ok(()) => return Err("tampered generated Rust ABI was accepted".to_string()),
        Err(diagnostic) => diagnostic,
    };
    if !tamper_diagnostic.contains("kernel_abi.rs differs") {
        return Err(format!(
            "generated-output tamper rejection had unexpected diagnostic: {tamper_diagnostic}"
        ));
    }
    negative_results.push_str(&format!("generated-output-tamper: {tamper_diagnostic}\n"));
    fs::write(work.join("negative-results.txt"), &negative_results)
        .map_err(|error| format!("write IDL negative-case results: {error}"))?;

    let rust_consumer_sha = sha256sum(&rust_consumer)?;
    let nostd_consumer_sha = sha256sum(&nostd_consumer)?;
    let c_consumer_sha = sha256sum(&c_consumer)?;
    let negative_sha = sha256sum(&work.join("negative-results.txt"))?;
    let report = format!(
        "M0_IDL_OK\ngenerator_validated=true\nrelease_eligible=false\nsource_sha256={source_sha}\ngenerator_sha256={generator_sha}\nrust_output_sha256={rust_sha}\nc_output_sha256={c_sha}\ncanonical_output_sha256={canonical_sha}\nreproducibility_builds=3\nrust_consumer_sha256={rust_consumer_sha}\nnostd_consumer_sha256={nostd_consumer_sha}\nc_consumer_sha256={c_consumer_sha}\nnegative_results_sha256={negative_sha}\nrust_runtime_marker={expected_rust_marker}\nc_runtime_marker={expected_c_marker}\nnegative_cases=duplicate-syscall-number,bad-struct-offset,overlapping-bitfield,unknown-wire-type,generated-output-tamper\n"
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write IDL report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn reject_idl_mutation(
    source: &serde_json::Value,
    pointer: &str,
    replacement: serde_json::Value,
    label: &str,
    expected: &str,
) -> Result<String, String> {
    let mut mutated = source.clone();
    let target = mutated
        .pointer_mut(pointer)
        .ok_or_else(|| format!("IDL mutation `{label}` target `{pointer}` was not found"))?;
    *target = replacement;
    let diagnostic = idl::generate_value(&mutated)
        .err()
        .ok_or_else(|| format!("IDL mutation `{label}` unexpectedly generated outputs"))?;
    if !diagnostic.contains(expected) {
        return Err(format!(
            "IDL mutation `{label}` diagnostic `{diagnostic}` does not contain `{expected}`"
        ));
    }
    Ok(diagnostic)
}

fn m0_manifest() -> Result<(), String> {
    let root = workspace_root()?;
    let openssl = PathBuf::from("/usr/sbin/openssl");
    let openssl_expected = "633e965ce973575b80b845ebcc8c28cef14b2096d3093a12a242828bcc699609";
    require_file(&openssl, "OpenSSL manifest signer/verifier")?;
    let openssl_actual = sha256sum(&openssl)?;
    if openssl_actual != openssl_expected {
        return Err(format!(
            "OpenSSL digest is {openssl_actual}, expected {openssl_expected}"
        ));
    }

    let schema_path = root.join("release/manifest.schema.json");
    let private_key_source = root.join("release/keys/m0-development-private.der.hex");
    let public_key = root.join("release/keys/m0-development-public.pem");
    for (path, label) in [
        (&schema_path, "release manifest schema"),
        (&private_key_source, "M0 development private-key encoding"),
        (&public_key, "M0 development public key"),
    ] {
        require_file(path, label)?;
    }
    let expected_private_sha = "45d4c3dc2826ef09e3e1bdf2cd5fee7286cbbafb3f250fff830459655da3a9a5";
    let expected_public_sha = "c40b867d852bc86bb825aceb2600ffe03ea18cfb1a046108e23b2cfd1c47ea7b";
    for (path, expected, label) in [
        (
            &private_key_source,
            expected_private_sha,
            "M0 development private-key encoding",
        ),
        (
            &public_key,
            expected_public_sha,
            "M0 development public key",
        ),
    ] {
        let actual = sha256sum(path)?;
        if actual != expected {
            return Err(format!("{label} digest is {actual}, expected {expected}"));
        }
    }

    let schema_text = fs::read_to_string(&schema_path)
        .map_err(|error| format!("read manifest schema {}: {error}", schema_path.display()))?;
    let schema: serde_json::Value = serde_json::from_str(&schema_text)
        .map_err(|error| format!("parse manifest schema: {error}"))?;

    let required_artifacts = [
        "build/m0-idl/generated/kernel_abi.rs",
        "build/m0-byte-allocator/libtmk_byte_allocator.rlib",
        "build/m0-capsule/capsule.elf",
        "build/m0-capsule/capsule.bin",
        "build/m0-capsule/linked-capsule.bin",
        "build/m0-composition/composition-kernel-high-half",
        "build/m0-composition/composition-kernel-low",
        "build/m0-composition/composition-consumer",
        "build/m0-composition/final-link-receipt.json",
        "build/m0-composition/linked-primitives/memcpy.bin",
        "build/m0-composition/primary.verified/artifact/libtmk_composition_probe.rlib",
        "build/m0-composition/primary.verified/evidence/direct-verus/00-composition_shell.rs",
        "build/m0-composition/primary.verified/evidence/source.verus.rs",
        "build/m0-composition/primary.verified/receipt.json",
        "build/m0-composition/report.txt",
        "build/m0-host/host.elf",
        "build/m0-host/libtmk_panic_host.rlib",
        "build/m0-platform-primitives/adapter-primary/libtmk_global_allocator.rlib",
        "build/m0-platform-primitives/emitted/alloc.bin",
        "build/m0-platform-primitives/emitted/memcpy.bin",
        "build/m0-platform-primitives/emitted/memset.bin",
        "build/m0-platform-primitives/emitted/seal.bin",
        "build/m0-platform-primitives/global-allocator-consumer",
        "build/m0-platform-primitives/global-allocator-high-half",
        "build/m0-platform-primitives/global-allocator-kernel-consumer",
        "build/m0-platform-primitives/linked/alloc.bin",
        "build/m0-platform-primitives/linked/memcpy.bin",
        "build/m0-platform-primitives/linked/memset.bin",
        "build/m0-platform-primitives/linked/seal.bin",
        "build/m0-platform-primitives/model-primary/libtmk_platform_primitives.rlib",
        "build/m0-platform-primitives/model-primary/verus-result.json",
        "build/m0-platform-primitives/objects/platform-primitives.o",
        "build/m0-platform-primitives/report.txt",
        "build/m0/probe.verified/artifact/libtmk_probe.rlib",
        "build/m0/probe.verified/receipt.json",
        "build/m0-uefi/entry.bin",
        "build/m0-uefi/image-primary/thermite-microkernel-m0.img",
        "build/m0-uefi/libtmk_uefi_capsule.rlib",
        "build/m0-uefi/pe-primary/BOOTX64.EFI",
        "build/m0-uefi/qemu-kvm-debugcon.log",
        "build/m0-uefi/qemu-tcg-debugcon.log",
        "build/m0-uefi/report.txt",
        "build/m0-uefi/verus-result.json",
    ];
    for path in required_artifacts {
        require_file(&root.join(path), &format!("manifest input `{path}`"))?;
    }

    let receipt_path = root.join("build/m0/probe.verified/receipt.json");
    let receipt_text = fs::read_to_string(&receipt_path)
        .map_err(|error| format!("read standalone Forge receipt: {error}"))?;
    let receipt: serde_json::Value = serde_json::from_str(&receipt_text)
        .map_err(|error| format!("parse standalone Forge receipt: {error}"))?;
    let receipt_schema = json_string(&receipt, "/schema", "Forge receipt schema")?;
    let receipt_binding = json_string(&receipt, "/binding_sha256", "Forge receipt binding digest")?;
    if receipt_schema != "thermite.verified-build-receipt.v1" {
        return Err(format!(
            "standalone receipt schema is `{receipt_schema}`, expected verified-build v1"
        ));
    }
    let probe_source_sha = sha256sum(&root.join("thermite/core/probe.th"))?;
    let bound_probe_source = json_string(
        &receipt,
        "/binding/raw_source_sha256",
        "Forge receipt source digest",
    )?;
    if probe_source_sha != bound_probe_source {
        return Err(format!(
            "standalone receipt binds source {bound_probe_source}, canonical probe is {probe_source_sha}"
        ));
    }
    let probe_artifact_path = root.join("build/m0/probe.verified/artifact/libtmk_probe.rlib");
    let probe_artifact_sha = sha256sum(&probe_artifact_path)?;
    let bound_probe_artifact = json_string(
        &receipt,
        "/binding/artifact/sha256",
        "Forge receipt artifact digest",
    )?;
    if probe_artifact_sha != bound_probe_artifact {
        return Err(format!(
            "standalone receipt binds artifact {bound_probe_artifact}, staged artifact is {probe_artifact_sha}"
        ));
    }

    let composition = validate_m0_composition_inputs(&root)?;

    let byte_result = sha256sum(&root.join("build/m0-byte-allocator/verus-result.json"))?;
    let panic_result = sha256sum(&root.join("build/m0-host/verus-result.json"))?;
    let capsule_result = sha256sum(&root.join("build/m0-capsule/verus-result.json"))?;
    let idl_result = sha256sum(&root.join("build/m0-idl/report.txt"))?;
    let byte_report = sha256sum(&root.join("build/m0-byte-allocator/report.txt"))?;
    let capsule_report = sha256sum(&root.join("build/m0-capsule/report.txt"))?;
    let forge_report = sha256sum(&root.join("build/m0/forge-probe-report.txt"))?;
    let host_report = sha256sum(&root.join("build/m0-host/report.txt"))?;
    let platform_result_path =
        root.join("build/m0-platform-primitives/model-primary/verus-result.json");
    let platform_report_path = root.join("build/m0-platform-primitives/report.txt");
    let platform_result = sha256sum(&platform_result_path)?;
    let platform_report = sha256sum(&platform_report_path)?;
    let platform_report_text = fs::read_to_string(&platform_report_path)
        .map_err(|error| format!("read platform-primitives report: {error}"))?;
    for (key, expected) in [
        ("component_verified", "true".to_string()),
        ("release_eligible", "false".to_string()),
        ("linked_primitives_verified", "true".to_string()),
        ("verus_verified", "39".to_string()),
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
            "linker_script_sha256",
            sha256sum(&root.join("tests/m0/global_allocator_kernel.ld"))?,
        ),
        (
            "model_artifact_sha256",
            sha256sum(&root.join(
                "build/m0-platform-primitives/model-primary/libtmk_platform_primitives.rlib",
            ))?,
        ),
        (
            "adapter_artifact_sha256",
            sha256sum(&root.join(
                "build/m0-platform-primitives/adapter-primary/libtmk_global_allocator.rlib",
            ))?,
        ),
        (
            "primitive_object_sha256",
            sha256sum(&root.join("build/m0-platform-primitives/objects/platform-primitives.o"))?,
        ),
        (
            "hosted_consumer_sha256",
            sha256sum(&root.join("build/m0-platform-primitives/global-allocator-consumer"))?,
        ),
        (
            "freestanding_consumer_sha256",
            sha256sum(&root.join("build/m0-platform-primitives/global-allocator-kernel-consumer"))?,
        ),
        (
            "high_half_consumer_sha256",
            sha256sum(&root.join("build/m0-platform-primitives/global-allocator-high-half"))?,
        ),
        ("model_reproducibility_builds", "3".to_string()),
        ("adapter_reproducibility_builds", "3".to_string()),
        ("freestanding_reproducibility_links", "3".to_string()),
        ("high_half_reproducibility_links", "3".to_string()),
        (
            "alloc_capsule_sha256",
            sha256sum(&root.join("build/m0-platform-primitives/emitted/alloc.bin"))?,
        ),
        (
            "seal_capsule_sha256",
            sha256sum(&root.join("build/m0-platform-primitives/emitted/seal.bin"))?,
        ),
        (
            "memcpy_capsule_sha256",
            sha256sum(&root.join("build/m0-platform-primitives/emitted/memcpy.bin"))?,
        ),
        (
            "memset_capsule_sha256",
            sha256sum(&root.join("build/m0-platform-primitives/emitted/memset.bin"))?,
        ),
        (
            "runtime_marker",
            "M0_GLOBAL_ALLOC_OK:box:vec:reject:sealed".to_string(),
        ),
        ("freestanding_runtime", "fail-stop-timeout-124".to_string()),
        ("high_half_link_base", "ffffffff80000000".to_string()),
        (
            "negative_cases",
            "alloc-byte,alloc-semantics,assume,arena-layout,code-model".to_string(),
        ),
    ] {
        let actual = report_field(&platform_report_text, key)?;
        if actual != expected {
            return Err(format!(
                "platform-primitives report field `{key}` is `{actual}`, expected `{expected}`"
            ));
        }
    }
    let receipt_sha = sha256sum(&receipt_path)?;
    let uefi_result = sha256sum(&root.join("build/m0-uefi/verus-result.json"))?;
    let uefi_report = sha256sum(&root.join("build/m0-uefi/report.txt"))?;
    let uefi_entry = fs::read(root.join("build/m0-uefi/entry.bin"))
        .map_err(|error| format!("read manifest UEFI entry bytes: {error}"))?;
    let uefi_loader_path = root.join("build/m0-uefi/pe-primary/BOOTX64.EFI");
    let uefi_loader = fs::read(&uefi_loader_path)
        .map_err(|error| format!("read manifest UEFI loader: {error}"))?;
    uefi::audit_pe(&uefi_loader, &uefi_entry)?;
    let uefi_image_path = root.join("build/m0-uefi/image-primary/thermite-microkernel-m0.img");
    let uefi_image = fs::read(&uefi_image_path)
        .map_err(|error| format!("read manifest UEFI boot image: {error}"))?;
    let embedded_loader = uefi::extract_bootx64(&uefi_image)?;
    if embedded_loader.bytes != uefi_loader {
        return Err("manifest UEFI boot image does not contain the audited loader".to_string());
    }
    for accelerator in ["tcg", "kvm"] {
        let log = fs::read(root.join(format!("build/m0-uefi/qemu-{accelerator}-debugcon.log")))
            .map_err(|error| format!("read manifest QEMU {accelerator} observation: {error}"))?;
        if log != b"TMK_M0_UEFI_OK!\n" {
            return Err(format!(
                "manifest QEMU {accelerator} observation is not the exact UEFI marker"
            ));
        }
    }
    let repository_revision = git_output(&root, &["rev-parse", "HEAD"], "TMK revision")?;
    let repository_dirty = !git_output(&root, &["status", "--porcelain"], "TMK status")?.is_empty();

    let mut host_bindings = vec![
        byte_result.clone(),
        capsule_result.clone(),
        panic_result.clone(),
    ];
    host_bindings.sort();
    let mut manifest_value = serde_json::json!({
        "schema": "tmk.release-manifest.v1",
        "manifest_id": "tmk-m0-development",
        "release": {
            "project": "Thermite Microkernel",
            "version": "0.0.0-m0",
            "source_date_epoch": 0,
            "development": true,
            "release_eligible": false,
            "assurance_headline": "l3"
        },
        "platform": {
            "architecture": "x86_64",
            "machine": "q35",
            "firmware": "uefi",
            "target_triple": "x86_64-unknown-none",
            "bsp_cores": 1,
            "smp_ready": true,
            "cpu_features": ["apic", "long_mode", "nx", "sse2", "syscall"]
        },
        "repositories": [
            {
                "name": "thermite",
                "url": "https://github.com/dollspace-gay/Thermite",
                "revision": "845d684f00e829491ee4c537818fba2689bcaefc",
                "dirty": false
            },
            {
                "name": "thermite-microkernel",
                "url": "https://github.com/dollspace-gay/Thermite-Microkernel",
                "revision": repository_revision,
                "dirty": repository_dirty
            }
        ],
        "tools": [
            {
                "name": "ar",
                "version": "2.44",
                "path": "/usr/sbin/ar",
                "sha256": "a21151402078c113fd801d16e0a0d2659ee44cee1b9828474f937bbf097b0df6",
                "classification": "trusted_tool"
            },
            {
                "name": "cc",
                "version": "15.2.1",
                "path": "/usr/sbin/cc",
                "sha256": "1ce580ecfabf35747bc550481621e2f2c04fd8fc23b8182779f33b82d07856d0",
                "classification": "trusted_tool"
            },
            {
                "name": "forge",
                "version": "thermite-v0.0.2-845d684f",
                "path": "/home/doll/Thermite/target/debug/forge",
                "sha256": "3fad9e2b328367ad0169b297ea03165664edc854f6a026fcb08bcfcb814f35d4",
                "classification": "trusted_tool"
            },
            {
                "name": "ld",
                "version": "2.44",
                "path": "/usr/sbin/ld",
                "sha256": "6cf122245638eb45fd981c75bf3a116675b3f9c7510ae2e3b386aa6738e46505",
                "classification": "trusted_tool"
            },
            {
                "name": "mcopy",
                "version": "4.0.49",
                "path": "/usr/sbin/mcopy",
                "sha256": "92d837c9b2ad562e5597a1881b7cdd7828e9c0e8ccfbc874fb396eee22fcebf3",
                "classification": "trusted_tool"
            },
            {
                "name": "mkfs-fat",
                "version": "4.2",
                "path": "/usr/sbin/mkfs.fat",
                "sha256": "7075f676c8dd292015f8f72d3574eb024c5ab5e545c3b031b8ef5355a5701093",
                "classification": "trusted_tool"
            },
            {
                "name": "nm",
                "version": "2.44",
                "path": "/usr/sbin/nm",
                "sha256": "988d8ded768c4e59284a44f641e92db6c0c8dd222547c32ce432577ff3cb9cc6",
                "classification": "trusted_tool"
            },
            {
                "name": "objcopy",
                "version": "2.44",
                "path": "/usr/sbin/objcopy",
                "sha256": "8f09a5b2d8e2b34aebf269fffef2308492a022dcfedf87b49489592e838129b4",
                "classification": "trusted_tool"
            },
            {
                "name": "objdump",
                "version": "2.44",
                "path": "/usr/sbin/objdump",
                "sha256": "c7c3f8c5c0ed23b2330e148e58624f8d798f1673f4c9fb126ee81096f05e3653",
                "classification": "trusted_tool"
            },
            {
                "name": "openssl",
                "version": "3.2.6",
                "path": "/usr/sbin/openssl",
                "sha256": openssl_actual,
                "classification": "trusted_tool"
            },
            {
                "name": "ovmf-code",
                "version": "sha256-pinned",
                "path": "/usr/share/edk2/ovmf/OVMF_CODE.fd",
                "sha256": "4e87e4be6bb9cdced848ec0b43adab3c7f15623e36055525d0691d137eb74af9",
                "classification": "environmental"
            },
            {
                "name": "ovmf-vars",
                "version": "sha256-pinned",
                "path": "/usr/share/edk2/ovmf/OVMF_VARS.fd",
                "sha256": "6ed987af3a3c155be71665f510eae3e007eda9b8b94afd59d45e91c4a11565cc",
                "classification": "environmental"
            },
            {
                "name": "qemu",
                "version": "9.2.4",
                "path": "/usr/bin/qemu-system-x86_64",
                "sha256": "8294f7d61d86167076194e834c3e4c592923f1812709a46edf4bb8f76e55ec7e",
                "classification": "environmental"
            },
            {
                "name": "readelf",
                "version": "2.44",
                "path": "/usr/sbin/readelf",
                "sha256": "59d345f2a2b47f5617e8f53c72f6db5169c723c11d3e809a9e6e3c5673f2420c",
                "classification": "trusted_tool"
            },
            {
                "name": "rustc-codegen",
                "version": "1.95.0",
                "path": "/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc",
                "sha256": "bff349e72704ff70bc08a234a3847338e797065bbedde5e556808bc87b7bf7c6",
                "classification": "trusted_tool"
            },
            {
                "name": "timeout",
                "version": "9.6",
                "path": "/usr/bin/timeout",
                "sha256": "350001cc47ad731c4e797532fe46a999477aba359692e2de3e93f316b4021dab",
                "classification": "trusted_tool"
            },
            {
                "name": "touch",
                "version": "9.6",
                "path": "/usr/bin/touch",
                "sha256": "22c0c7439c659dff1d88dbe7e096d5f4f6fc12d82673395304815626e240934f",
                "classification": "trusted_tool"
            },
            {
                "name": "verus",
                "version": "0.2026.05.24.ecee80a",
                "path": "/opt/verus/0.2026.05.24.ecee80a/verus",
                "sha256": "c5911ee43c7a92c49a48d2c8646c604d252a38c71c87bda88ad4d33eb9e7e0fc",
                "classification": "trusted_tool"
            }
        ],
        "functions": [
            {
                "semantic_address": "capsule::global_alloc_shim",
                "origin": "capsule",
                "assurance": "capsule_refinement",
                "scope": "exact_bytes",
                "source_sha256": sha256sum(&root.join("verus/machine-model/platform_primitives_capsule.rs"))?,
                "artifact_name": "m0-global-allocator-adapter"
            },
            {
                "semantic_address": "capsule::hlt_register",
                "origin": "capsule",
                "assurance": "capsule_refinement",
                "scope": "exact_bytes",
                "source_sha256": sha256sum(&root.join("verus/machine-model/hlt_register_capsule.rs"))?,
                "artifact_name": "m0-capsule"
            },
            {
                "semantic_address": "capsule::platform_alloc",
                "origin": "capsule",
                "assurance": "capsule_refinement",
                "scope": "exact_bytes",
                "source_sha256": sha256sum(&root.join("verus/machine-model/platform_primitives_capsule.rs"))?,
                "artifact_name": "m0-platform-primitives"
            },
            {
                "semantic_address": "capsule::platform_memcpy",
                "origin": "capsule",
                "assurance": "capsule_refinement",
                "scope": "exact_bytes",
                "source_sha256": sha256sum(&root.join("verus/machine-model/platform_primitives_capsule.rs"))?,
                "artifact_name": "m0-platform-primitives"
            },
            {
                "semantic_address": "capsule::platform_memset",
                "origin": "capsule",
                "assurance": "capsule_refinement",
                "scope": "exact_bytes",
                "source_sha256": sha256sum(&root.join("verus/machine-model/platform_primitives_capsule.rs"))?,
                "artifact_name": "m0-platform-primitives"
            },
            {
                "semantic_address": "capsule::platform_seal",
                "origin": "capsule",
                "assurance": "capsule_refinement",
                "scope": "exact_bytes",
                "source_sha256": sha256sum(&root.join("verus/machine-model/platform_primitives_capsule.rs"))?,
                "artifact_name": "m0-platform-primitives"
            },
            {
                "semantic_address": "capsule::uefi_debug_return",
                "origin": "capsule",
                "assurance": "capsule_refinement",
                "scope": "exact_bytes",
                "source_sha256": sha256sum(&root.join("verus/machine-model/uefi_debug_exit_capsule.rs"))?,
                "artifact_name": "m0-uefi-loader"
            },
            {
                "semantic_address": "thermite::composition_step",
                "origin": "thermite",
                "assurance": "l3",
                "scope": "end_to_end",
                "source_sha256": sha256sum(&root.join("thermite/core/composition_probe.th"))?,
                "artifact_name": "m0-composition-core"
            },
            {
                "semantic_address": "thermite::transition_probe",
                "origin": "thermite",
                "assurance": "l3",
                "scope": "end_to_end",
                "source_sha256": probe_source_sha,
                "artifact_name": "thermite-probe"
            },
            {
                "semantic_address": "verus::allocate_bytes",
                "origin": "direct_verus",
                "assurance": "direct_verus",
                "scope": "whole_body",
                "source_sha256": sha256sum(&root.join("verus/platform/byte_allocator.rs"))?,
                "artifact_name": "m0-byte-allocator"
            },
            {
                "semantic_address": "verus::composition_shell",
                "origin": "direct_verus",
                "assurance": "direct_verus",
                "scope": "whole_body",
                "source_sha256": sha256sum(&root.join("tests/m0/composition_shell.rs"))?,
                "artifact_name": "m0-composition-core"
            },
            {
                "semantic_address": "verus::panic_fail_stop",
                "origin": "direct_verus",
                "assurance": "direct_verus",
                "scope": "whole_body",
                "source_sha256": sha256sum(&root.join("verus/platform/panic_host.rs"))?,
                "artifact_name": "m0-host"
            }
        ],
        "forge_receipts": [
            {
                "name": "composition-probe",
                "kind": "composition",
                "schema": composition.receipt_schema,
                "binding_sha256": composition.binding_sha.clone(),
                "receipt_sha256": composition.receipt_sha.clone(),
                "artifact_name": "m0-composition-core",
                "assurance": "l3",
                "scope": "end_to_end",
                "replay_passed": true
            },
            {
                "name": "standalone-probe",
                "kind": "standalone",
                "schema": receipt_schema,
                "binding_sha256": receipt_binding,
                "receipt_sha256": receipt_sha,
                "artifact_name": "thermite-probe",
                "assurance": "l3",
                "scope": "end_to_end",
                "replay_passed": true
            }
        ],
        "direct_verus": [
            {
                "name": "byte-allocator",
                "source_sha256": sha256sum(&root.join("verus/platform/byte_allocator.rs"))?,
                "result_sha256": byte_result,
                "artifact_name": "m0-byte-allocator",
                "artifact_sha256": sha256sum(&root.join("build/m0-byte-allocator/libtmk_byte_allocator.rlib"))?,
                "verified_queries": 18,
                "errors": 0,
                "no_cheating": true
            },
            {
                "name": "panic-host",
                "source_sha256": sha256sum(&root.join("verus/platform/panic_host.rs"))?,
                "result_sha256": panic_result,
                "artifact_name": "m0-host",
                "artifact_sha256": sha256sum(&root.join("build/m0-host/host.elf"))?,
                "verified_queries": 2,
                "errors": 0,
                "no_cheating": true
            },
            {
                "name": "platform-primitives",
                "source_sha256": sha256sum(&root.join("verus/machine-model/platform_primitives_capsule.rs"))?,
                "result_sha256": platform_result.clone(),
                "artifact_name": "m0-platform-model",
                "artifact_sha256": sha256sum(&root.join("build/m0-platform-primitives/model-primary/libtmk_platform_primitives.rlib"))?,
                "verified_queries": 39,
                "errors": 0,
                "no_cheating": true
            },
            {
                "name": "uefi-entry-model",
                "source_sha256": sha256sum(&root.join("verus/machine-model/uefi_debug_exit_capsule.rs"))?,
                "result_sha256": uefi_result.clone(),
                "artifact_name": "m0-uefi-entry-model",
                "artifact_sha256": sha256sum(&root.join("build/m0-uefi/libtmk_uefi_capsule.rlib"))?,
                "verified_queries": 3,
                "errors": 0,
                "no_cheating": true
            }
        ],
        "capsules": [
            {
                "name": "composition-memcpy",
                "model_source_sha256": sha256sum(&root.join("verus/machine-model/platform_primitives_capsule.rs"))?,
                "proof_result_sha256": platform_result.clone(),
                "emitted_sha256": sha256sum(&root.join("build/m0-platform-primitives/emitted/memcpy.bin"))?,
                "linked_sha256": sha256sum(&root.join("build/m0-composition/linked-primitives/memcpy.bin"))?,
                "semantic_claim": "the composition final link selects the exact registered memcpy bytes and discards the unselected platform primitives",
                "artifact_name": "m0-composition-final-link"
            },
            {
                "name": "hlt-register",
                "model_source_sha256": sha256sum(&root.join("verus/machine-model/hlt_register_capsule.rs"))?,
                "proof_result_sha256": capsule_result,
                "emitted_sha256": sha256sum(&root.join("build/m0-capsule/capsule.bin"))?,
                "linked_sha256": sha256sum(&root.join("build/m0-capsule/linked-capsule.bin"))?,
                "semantic_claim": "mov rax,rdi; hlt preserves all modeled state except RAX and halted",
                "artifact_name": "m0-capsule"
            },
            {
                "name": "platform-alloc",
                "model_source_sha256": sha256sum(&root.join("verus/machine-model/platform_primitives_capsule.rs"))?,
                "proof_result_sha256": platform_result.clone(),
                "emitted_sha256": sha256sum(&root.join("build/m0-platform-primitives/emitted/alloc.bin"))?,
                "linked_sha256": sha256sum(&root.join("build/m0-platform-primitives/linked/alloc.bin"))?,
                "semantic_claim": "exact registered bump-allocation bytes implement aligned bounded boot-arena allocation and null-on-failure semantics",
                "artifact_name": "m0-platform-primitives"
            },
            {
                "name": "platform-memcpy",
                "model_source_sha256": sha256sum(&root.join("verus/machine-model/platform_primitives_capsule.rs"))?,
                "proof_result_sha256": platform_result.clone(),
                "emitted_sha256": sha256sum(&root.join("build/m0-platform-primitives/emitted/memcpy.bin"))?,
                "linked_sha256": sha256sum(&root.join("build/m0-platform-primitives/linked/memcpy.bin"))?,
                "semantic_claim": "exact registered memcpy bytes implement pointwise copy for valid non-overlapping ranges under the DF-clear invariant",
                "artifact_name": "m0-platform-primitives"
            },
            {
                "name": "platform-memset",
                "model_source_sha256": sha256sum(&root.join("verus/machine-model/platform_primitives_capsule.rs"))?,
                "proof_result_sha256": platform_result.clone(),
                "emitted_sha256": sha256sum(&root.join("build/m0-platform-primitives/emitted/memset.bin"))?,
                "linked_sha256": sha256sum(&root.join("build/m0-platform-primitives/linked/memset.bin"))?,
                "semantic_claim": "exact registered memset bytes implement pointwise byte fill for valid ranges under the DF-clear invariant",
                "artifact_name": "m0-platform-primitives"
            },
            {
                "name": "platform-seal",
                "model_source_sha256": sha256sum(&root.join("verus/machine-model/platform_primitives_capsule.rs"))?,
                "proof_result_sha256": platform_result.clone(),
                "emitted_sha256": sha256sum(&root.join("build/m0-platform-primitives/emitted/seal.bin"))?,
                "linked_sha256": sha256sum(&root.join("build/m0-platform-primitives/linked/seal.bin"))?,
                "semantic_claim": "exact registered seal bytes preserve arena contents and cursor while permanently refusing later allocations",
                "artifact_name": "m0-platform-primitives"
            },
            {
                "name": "uefi-debug-return",
                "model_source_sha256": sha256sum(&root.join("verus/machine-model/uefi_debug_exit_capsule.rs"))?,
                "proof_result_sha256": uefi_result.clone(),
                "emitted_sha256": sha256sum(&root.join("build/m0-uefi/entry.bin"))?,
                "linked_sha256": sha256sum(&root.join("build/m0-uefi/entry.bin"))?,
                "semantic_claim": "exact registered bytes emit TMK_M0_UEFI_OK to port 0xe9, preserve modeled firmware state, return EFI_SUCCESS, and survive PE/FAT link unchanged",
                "artifact_name": "m0-uefi-loader"
            }
        ],
        "artifacts": [
            manifest_artifact(&root, "kernel-abi-rust", "generated_source", "build/m0-idl/generated/kernel_abi.rs", vec![idl_result.clone()], false)?,
            manifest_artifact(&root, "m0-byte-allocator", "kernel_rlib", "build/m0-byte-allocator/libtmk_byte_allocator.rlib", vec![byte_result.clone()], false)?,
            manifest_artifact(&root, "m0-capsule", "capsule_elf", "build/m0-capsule/capsule.elf", vec![capsule_result.clone()], true)?,
            manifest_artifact(&root, "m0-composition-core", "kernel_rlib", "build/m0-composition/primary.verified/artifact/libtmk_composition_probe.rlib", vec![composition.binding_sha.clone()], false)?,
            manifest_artifact(&root, "m0-composition-final-link", "kernel_elf", "build/m0-composition/composition-kernel-high-half", vec![composition.binding_sha.clone(), composition.report_sha.clone(), platform_result.clone(), platform_report.clone()], true)?,
            manifest_artifact(&root, "m0-final-link-receipt", "link_receipt", "build/m0-composition/final-link-receipt.json", vec![composition.binding_sha.clone(), composition.report_sha.clone(), platform_result.clone(), platform_report.clone()], false)?,
            manifest_artifact(&root, "m0-global-allocator-adapter", "kernel_rlib", "build/m0-platform-primitives/adapter-primary/libtmk_global_allocator.rlib", vec![platform_result.clone(), platform_report.clone()], false)?,
            manifest_artifact(&root, "m0-global-allocator-high-half", "kernel_elf", "build/m0-platform-primitives/global-allocator-high-half", vec![platform_result.clone(), platform_report.clone()], true)?,
            manifest_artifact(&root, "m0-host", "kernel_elf", "build/m0-host/host.elf", host_bindings, true)?,
            manifest_artifact(&root, "m0-platform-model", "kernel_rlib", "build/m0-platform-primitives/model-primary/libtmk_platform_primitives.rlib", vec![platform_result.clone()], false)?,
            manifest_artifact(&root, "m0-platform-primitives", "kernel_object", "build/m0-platform-primitives/objects/platform-primitives.o", vec![platform_result.clone(), platform_report.clone()], true)?,
            manifest_artifact(&root, "m0-uefi-boot-image", "boot_image", "build/m0-uefi/image-primary/thermite-microkernel-m0.img", vec![uefi_result.clone(), uefi_report.clone()], false)?,
            manifest_artifact(&root, "m0-uefi-entry-model", "kernel_rlib", "build/m0-uefi/libtmk_uefi_capsule.rlib", vec![uefi_result.clone()], false)?,
            manifest_artifact(&root, "m0-uefi-loader", "uefi_loader", "build/m0-uefi/pe-primary/BOOTX64.EFI", vec![uefi_result.clone(), uefi_report.clone()], true)?,
            manifest_artifact(&root, "thermite-probe", "kernel_rlib", "build/m0/probe.verified/artifact/libtmk_probe.rlib", vec![receipt_binding.to_string()], false)?
        ],
        "tests": [
            { "name": "byte-allocator", "status": "pass", "result_sha256": byte_report, "passed": 4, "failed": 0, "skipped": 0 },
            { "name": "capsule", "status": "pass", "result_sha256": capsule_report, "passed": 5, "failed": 0, "skipped": 0 },
            { "name": "composition", "status": "pass", "result_sha256": composition.report_sha.clone(), "passed": 24, "failed": 0, "skipped": 0 },
            { "name": "forge-probe", "status": "pass", "result_sha256": forge_report, "passed": 6, "failed": 0, "skipped": 0 },
            { "name": "host-link", "status": "pass", "result_sha256": host_report, "passed": 4, "failed": 0, "skipped": 0 },
            { "name": "kernel-idl", "status": "pass", "result_sha256": idl_result, "passed": 8, "failed": 0, "skipped": 0 },
            { "name": "platform-primitives", "status": "pass", "result_sha256": platform_report, "passed": 10, "failed": 0, "skipped": 0 },
            { "name": "uefi-image", "status": "pass", "result_sha256": uefi_report.clone(), "passed": 16, "failed": 0, "skipped": 0 }
        ],
        "assumptions": [
            { "id": "firmware", "class": "environmental", "statement": "Pinned OVMF implements the UEFI image loading and return behavior exercised by the M0 probe." },
            { "id": "hardware", "class": "environmental", "statement": "The modeled x86_64 architectural behavior and memory ordering hold." },
            { "id": "linker", "class": "trusted_tool", "statement": "Pinned GNU binutils preserve verified object semantics outside exact-byte capsule checks." },
            { "id": "rust-codegen", "class": "trusted_tool", "statement": "Pinned rustc and LLVM preserve the semantics of verified source." }
        ],
        "versions": {
            "abi_major": 1,
            "abi_minor": 0,
            "service_protocol_major": 1,
            "filesystem_format_major": 1
        },
        "known_limitations": [
            "M0 UEFI image remains a separate debug-return probe; it is not the M1 loader or the composed kernel ELF.",
            "Production release signing requires an external key and a complete release input set."
        ],
        "signing": {
            "algorithm": "ed25519",
            "key_id": "m0-development-test-key",
            "payload_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
            "signature": "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "public_key_sha256": expected_public_sha
        }
    });

    let work = root.join("build/m0-manifest");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;

    let private_key = work.join("m0-development-private.der");
    let private_key_hex = fs::read_to_string(&private_key_source)
        .map_err(|error| format!("read M0 development private-key encoding: {error}"))?;
    fs::write(&private_key, hex_decode(private_key_hex.trim())?)
        .map_err(|error| format!("materialize M0 development private key: {error}"))?;
    let derived_public_key = work.join("derived-development-public.pem");
    run_checked(
        Command::new(&openssl)
            .args(["pkey", "-inform", "DER", "-in"])
            .arg(&private_key)
            .args(["-pubout", "-out"])
            .arg(&derived_public_key),
        "derive M0 development public key",
    )?;
    let derived_public_sha = sha256sum(&derived_public_key)?;
    if derived_public_sha != expected_public_sha {
        return Err(format!(
            "development private key derives public digest {derived_public_sha}, expected {expected_public_sha}"
        ));
    }

    let primary = work.join("primary");
    sign_manifest(&openssl, &private_key, &mut manifest_value, &primary)?;
    manifest::validate(&schema, &manifest_value)?;
    validate_manifest_artifact_files(&root, &manifest_value)?;
    verify_manifest_signature(
        &openssl,
        &public_key,
        expected_public_sha,
        &manifest_value,
        &primary,
        "verify primary M0 development manifest signature",
    )?;
    let primary_manifest = primary.join("manifest.json");
    let primary_manifest_sha = sha256sum(&primary_manifest)?;
    let primary_signature_sha = sha256sum(&primary.join("signature.bin"))?;
    let primary_payload_sha = json_string(
        &manifest_value,
        "/signing/payload_sha256",
        "manifest payload digest",
    )?
    .to_string();

    for name in ["repro-a", "repro-b"] {
        let mut reproduced = manifest_value.clone();
        let reproduced_dir = work.join(name);
        sign_manifest(&openssl, &private_key, &mut reproduced, &reproduced_dir)?;
        manifest::validate(&schema, &reproduced)?;
        verify_manifest_signature(
            &openssl,
            &public_key,
            expected_public_sha,
            &reproduced,
            &reproduced_dir,
            &format!("verify M0 manifest signature in {name}"),
        )?;
        let reproduced_sha = sha256sum(&reproduced_dir.join("manifest.json"))?;
        let signature_sha = sha256sum(&reproduced_dir.join("signature.bin"))?;
        if reproduced_sha != primary_manifest_sha || signature_sha != primary_signature_sha {
            return Err(format!(
                "manifest signing in {name} was not reproducible: manifest {reproduced_sha}, signature {signature_sha}"
            ));
        }
    }

    let mut negative_results = String::new();
    let mut unknown_property = manifest_value.clone();
    unknown_property
        .as_object_mut()
        .ok_or_else(|| "manifest root disappeared".to_string())?
        .insert("unbound_claim".to_string(), serde_json::json!(true));
    record_manifest_rejection(
        &schema,
        &unknown_property,
        "unknown-property",
        "unknown property `unbound_claim`",
        &mut negative_results,
    )?;

    let mut capsule_drift = manifest_value.clone();
    *capsule_drift
        .pointer_mut("/capsules/0/linked_sha256")
        .ok_or_else(|| "capsule drift mutation target missing".to_string())? =
        serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    record_manifest_rejection(
        &schema,
        &capsule_drift,
        "capsule-byte-drift",
        "emitted and linked digests must match",
        &mut negative_results,
    )?;

    let mut unknown_binding = manifest_value.clone();
    *unknown_binding
        .pointer_mut("/artifacts/0/source_bindings/0")
        .ok_or_else(|| "source-binding mutation target missing".to_string())? =
        serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    record_manifest_rejection(
        &schema,
        &unknown_binding,
        "unknown-source-binding",
        "is not supplied by a receipt, proof, capsule, or test result",
        &mut negative_results,
    )?;

    let mut mismatched_verus_artifact = manifest_value.clone();
    *mismatched_verus_artifact
        .pointer_mut("/direct_verus/0/artifact_sha256")
        .ok_or_else(|| "direct-Verus artifact mutation target missing".to_string())? =
        serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    record_manifest_rejection(
        &schema,
        &mismatched_verus_artifact,
        "direct-verus-artifact-mismatch",
        "artifact digest does not match artifact",
        &mut negative_results,
    )?;

    let mut artifact_file_drift = manifest_value.clone();
    *artifact_file_drift
        .pointer_mut("/artifacts/0/sha256")
        .ok_or_else(|| "artifact file mutation target missing".to_string())? =
        serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    manifest::validate(&schema, &artifact_file_drift)?;
    let file_diagnostic = validate_manifest_artifact_files(&root, &artifact_file_drift)
        .err()
        .ok_or_else(|| "artifact file digest mutation unexpectedly passed".to_string())?;
    if !file_diagnostic.contains("file digest") {
        return Err(format!(
            "artifact-file-drift diagnostic `{file_diagnostic}` is unexpected"
        ));
    }
    negative_results.push_str(&format!("artifact-file-drift: {file_diagnostic}\n"));

    let mut platform_file_drift = manifest_value.clone();
    let platform_artifact = platform_file_drift
        .pointer_mut("/artifacts")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|artifacts| {
            artifacts.iter_mut().find(|artifact| {
                artifact.get("name").and_then(serde_json::Value::as_str)
                    == Some("m0-platform-primitives")
            })
        })
        .ok_or_else(|| "platform-primitives file mutation target missing".to_string())?;
    *platform_artifact
        .get_mut("sha256")
        .ok_or_else(|| "platform-primitives digest mutation target missing".to_string())? =
        serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    manifest::validate(&schema, &platform_file_drift)?;
    let platform_file_diagnostic = validate_manifest_artifact_files(&root, &platform_file_drift)
        .err()
        .ok_or_else(|| {
            "platform-primitives file digest mutation unexpectedly passed".to_string()
        })?;
    if !platform_file_diagnostic.contains("m0-platform-primitives")
        || !platform_file_diagnostic.contains("file digest")
    {
        return Err(format!(
            "platform-primitives-file-drift diagnostic `{platform_file_diagnostic}` is unexpected"
        ));
    }
    negative_results.push_str(&format!(
        "platform-primitives-file-drift: {platform_file_diagnostic}\n"
    ));

    let mut composition_file_drift = manifest_value.clone();
    let composition_artifact = composition_file_drift
        .pointer_mut("/artifacts")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|artifacts| {
            artifacts.iter_mut().find(|artifact| {
                artifact.get("name").and_then(serde_json::Value::as_str)
                    == Some("m0-composition-core")
            })
        })
        .ok_or_else(|| "composition artifact file mutation target missing".to_string())?;
    *composition_artifact
        .get_mut("sha256")
        .ok_or_else(|| "composition artifact digest mutation target missing".to_string())? =
        serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    manifest::validate(&schema, &composition_file_drift)?;
    let composition_file_diagnostic =
        validate_manifest_artifact_files(&root, &composition_file_drift)
            .err()
            .ok_or_else(|| "composition artifact file drift unexpectedly passed".to_string())?;
    if !composition_file_diagnostic.contains("m0-composition-core")
        || !composition_file_diagnostic.contains("file digest")
    {
        return Err(format!(
            "composition-artifact-file-drift diagnostic `{composition_file_diagnostic}` is unexpected"
        ));
    }
    negative_results.push_str(&format!(
        "composition-artifact-file-drift: {composition_file_diagnostic}\n"
    ));

    let mut final_link_receipt_file_drift = manifest_value.clone();
    let final_link_receipt_artifact = final_link_receipt_file_drift
        .pointer_mut("/artifacts")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|artifacts| {
            artifacts.iter_mut().find(|artifact| {
                artifact.get("name").and_then(serde_json::Value::as_str)
                    == Some("m0-final-link-receipt")
            })
        })
        .ok_or_else(|| "final-link receipt file mutation target missing".to_string())?;
    *final_link_receipt_artifact
        .get_mut("sha256")
        .ok_or_else(|| "final-link receipt digest mutation target missing".to_string())? =
        serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    manifest::validate(&schema, &final_link_receipt_file_drift)?;
    let final_link_receipt_diagnostic =
        validate_manifest_artifact_files(&root, &final_link_receipt_file_drift)
            .err()
            .ok_or_else(|| "final-link receipt file drift unexpectedly passed".to_string())?;
    if !final_link_receipt_diagnostic.contains("m0-final-link-receipt")
        || !final_link_receipt_diagnostic.contains("file digest")
    {
        return Err(format!(
            "final-link-receipt-file-drift diagnostic `{final_link_receipt_diagnostic}` is unexpected"
        ));
    }
    negative_results.push_str(&format!(
        "final-link-receipt-file-drift: {final_link_receipt_diagnostic}\n"
    ));

    let mut boot_image_file_drift = manifest_value.clone();
    let boot_artifact = boot_image_file_drift
        .pointer_mut("/artifacts")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|artifacts| {
            artifacts.iter_mut().find(|artifact| {
                artifact.get("name").and_then(serde_json::Value::as_str)
                    == Some("m0-uefi-boot-image")
            })
        })
        .ok_or_else(|| "boot-image file mutation target missing".to_string())?;
    *boot_artifact
        .get_mut("sha256")
        .ok_or_else(|| "boot-image digest mutation target missing".to_string())? =
        serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    manifest::validate(&schema, &boot_image_file_drift)?;
    let boot_file_diagnostic = validate_manifest_artifact_files(&root, &boot_image_file_drift)
        .err()
        .ok_or_else(|| "boot-image file digest mutation unexpectedly passed".to_string())?;
    if !boot_file_diagnostic.contains("m0-uefi-boot-image")
        || !boot_file_diagnostic.contains("file digest")
    {
        return Err(format!(
            "boot-image-file-drift diagnostic `{boot_file_diagnostic}` is unexpected"
        ));
    }
    negative_results.push_str(&format!("boot-image-file-drift: {boot_file_diagnostic}\n"));

    let mut reordered = manifest_value.clone();
    reordered
        .pointer_mut("/tools")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| "tool ordering mutation target missing".to_string())?
        .swap(0, 1);
    record_manifest_rejection(
        &schema,
        &reordered,
        "noncanonical-order",
        "strictly sorted and unique",
        &mut negative_results,
    )?;

    let mut dev_key_masquerade = manifest_value.clone();
    *dev_key_masquerade
        .pointer_mut("/release/release_eligible")
        .ok_or_else(|| "release eligibility mutation target missing".to_string())? =
        serde_json::json!(true);
    record_manifest_rejection(
        &schema,
        &dev_key_masquerade,
        "development-key-release",
        "development key cannot authorize",
        &mut negative_results,
    )?;

    let mut missing_composition = manifest_value.clone();
    *missing_composition
        .pointer_mut("/release/development")
        .ok_or_else(|| "development mutation target missing".to_string())? =
        serde_json::json!(false);
    *missing_composition
        .pointer_mut("/release/release_eligible")
        .ok_or_else(|| "release mutation target missing".to_string())? = serde_json::json!(true);
    *missing_composition
        .pointer_mut("/signing/key_id")
        .ok_or_else(|| "key mutation target missing".to_string())? =
        serde_json::json!("external-production-key");
    for repository in missing_composition
        .pointer_mut("/repositories")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| "repository mutation target missing".to_string())?
    {
        *repository
            .get_mut("dirty")
            .ok_or_else(|| "repository dirty field missing".to_string())? =
            serde_json::json!(false);
    }
    let composition_receipt = missing_composition
        .pointer_mut("/forge_receipts")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|receipts| {
            receipts.iter_mut().find(|receipt| {
                receipt.get("kind").and_then(serde_json::Value::as_str) == Some("composition")
            })
        })
        .ok_or_else(|| "missing-composition mutation target missing".to_string())?;
    *composition_receipt
        .get_mut("kind")
        .ok_or_else(|| "composition receipt kind disappeared".to_string())? =
        serde_json::json!("standalone");
    *composition_receipt
        .get_mut("schema")
        .ok_or_else(|| "composition receipt schema disappeared".to_string())? =
        serde_json::json!("thermite.verified-build-receipt.v1");
    record_manifest_rejection(
        &schema,
        &missing_composition,
        "missing-composition-release",
        "requires a composition receipt",
        &mut negative_results,
    )?;

    let mut missing_boot_image = manifest_value.clone();
    *missing_boot_image
        .pointer_mut("/release/development")
        .ok_or_else(|| "boot-image release development target missing".to_string())? =
        serde_json::json!(false);
    *missing_boot_image
        .pointer_mut("/release/release_eligible")
        .ok_or_else(|| "boot-image release eligibility target missing".to_string())? =
        serde_json::json!(true);
    *missing_boot_image
        .pointer_mut("/signing/key_id")
        .ok_or_else(|| "boot-image release key target missing".to_string())? =
        serde_json::json!("external-production-key");
    for repository in missing_boot_image
        .pointer_mut("/repositories")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| "boot-image release repository target missing".to_string())?
    {
        *repository
            .get_mut("dirty")
            .ok_or_else(|| "boot-image release repository dirty field missing".to_string())? =
            serde_json::json!(false);
    }
    let boot_artifact = missing_boot_image
        .pointer_mut("/artifacts")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|artifacts| {
            artifacts.iter_mut().find(|artifact| {
                artifact.get("name").and_then(serde_json::Value::as_str)
                    == Some("m0-uefi-boot-image")
            })
        })
        .ok_or_else(|| "boot-image release artifact target missing".to_string())?;
    *boot_artifact
        .get_mut("kind")
        .ok_or_else(|| "boot-image release kind target missing".to_string())? =
        serde_json::json!("filesystem_image");
    record_manifest_rejection(
        &schema,
        &missing_boot_image,
        "missing-boot-image-release",
        "requires exactly one boot image",
        &mut negative_results,
    )?;

    let mut missing_final_link_receipt = manifest_value.clone();
    *missing_final_link_receipt
        .pointer_mut("/release/development")
        .ok_or_else(|| "final-link release development target missing".to_string())? =
        serde_json::json!(false);
    *missing_final_link_receipt
        .pointer_mut("/release/release_eligible")
        .ok_or_else(|| "final-link release eligibility target missing".to_string())? =
        serde_json::json!(true);
    *missing_final_link_receipt
        .pointer_mut("/signing/key_id")
        .ok_or_else(|| "final-link release key target missing".to_string())? =
        serde_json::json!("external-production-key");
    for repository in missing_final_link_receipt
        .pointer_mut("/repositories")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| "final-link release repository target missing".to_string())?
    {
        *repository
            .get_mut("dirty")
            .ok_or_else(|| "final-link release repository dirty field missing".to_string())? =
            serde_json::json!(false);
    }
    let final_link_artifact = missing_final_link_receipt
        .pointer_mut("/artifacts")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|artifacts| {
            artifacts.iter_mut().find(|artifact| {
                artifact.get("name").and_then(serde_json::Value::as_str)
                    == Some("m0-final-link-receipt")
            })
        })
        .ok_or_else(|| "final-link release artifact target missing".to_string())?;
    *final_link_artifact
        .get_mut("kind")
        .ok_or_else(|| "final-link release kind target missing".to_string())? =
        serde_json::json!("manifest");
    record_manifest_rejection(
        &schema,
        &missing_final_link_receipt,
        "missing-final-link-receipt-release",
        "requires exactly one final-link receipt artifact",
        &mut negative_results,
    )?;

    let mut loose_schema = schema.clone();
    *loose_schema
        .pointer_mut("/additionalProperties")
        .ok_or_else(|| "schema strictness mutation target missing".to_string())? =
        serde_json::json!(true);
    let loose_diagnostic = manifest::validate(&loose_schema, &manifest_value)
        .err()
        .ok_or_else(|| "loosened manifest schema unexpectedly passed validation".to_string())?;
    if !loose_diagnostic.contains("must reject additional properties") {
        return Err(format!(
            "schema-loosening diagnostic `{loose_diagnostic}` is unexpected"
        ));
    }
    negative_results.push_str(&format!("schema-loosening: {loose_diagnostic}\n"));

    let mut signature_mutation = manifest_value.clone();
    let signature = signature_mutation
        .pointer("/signing/signature")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "signature mutation target missing".to_string())?
        .to_string();
    let replacement = if signature.starts_with('0') { '1' } else { '0' };
    let mutated_signature = format!("{replacement}{}", &signature[1..]);
    *signature_mutation
        .pointer_mut("/signing/signature")
        .ok_or_else(|| "signature mutation target disappeared".to_string())? =
        serde_json::json!(mutated_signature);
    manifest::validate(&schema, &signature_mutation)?;
    let signature_failure = verify_manifest_signature_expect_failure(
        &openssl,
        &public_key,
        &signature_mutation,
        &work.join("bad-signature"),
        "reject mutated manifest signature",
    )?;
    negative_results.push_str(&format!("signature-mutation: {signature_failure}\n"));

    let mut payload_mutation = manifest_value.clone();
    *payload_mutation
        .pointer_mut("/known_limitations/0")
        .ok_or_else(|| "payload mutation target missing".to_string())? =
        serde_json::json!("A forged limitation changed after signing.");
    let bad_payload_dir = work.join("bad-payload");
    fs::create_dir(&bad_payload_dir)
        .map_err(|error| format!("create bad-payload evidence directory: {error}"))?;
    let bad_payload = manifest::canonical_payload(&payload_mutation)?;
    fs::write(bad_payload_dir.join("payload.json"), bad_payload)
        .map_err(|error| format!("write bad manifest payload: {error}"))?;
    let bad_payload_sha = sha256sum(&bad_payload_dir.join("payload.json"))?;
    *payload_mutation
        .pointer_mut("/signing/payload_sha256")
        .ok_or_else(|| "payload digest mutation target missing".to_string())? =
        serde_json::json!(bad_payload_sha);
    manifest::validate(&schema, &payload_mutation)?;
    let payload_failure = verify_manifest_signature_expect_failure(
        &openssl,
        &public_key,
        &payload_mutation,
        &bad_payload_dir,
        "reject manifest payload changed after signing",
    )?;
    negative_results.push_str(&format!("payload-mutation: {payload_failure}\n"));

    fs::write(work.join("negative-results.txt"), &negative_results)
        .map_err(|error| format!("write manifest negative results: {error}"))?;
    let negative_sha = sha256sum(&work.join("negative-results.txt"))?;
    let schema_sha = sha256sum(&schema_path)?;
    let validator_sha = sha256sum(&root.join("xtask/src/manifest.rs"))?;
    let orchestrator_sha = sha256sum(&root.join("xtask/src/main.rs"))?;
    let report = format!(
        "M0_MANIFEST_OK\nschema_validated=true\nsignature_verified=true\nartifact_files_replayed=true\ncomposition_receipt_validated=true\ncomposition_receipt_replayed=true\nfinal_link_receipt_validated=true\nrelease_eligible=false\ncomposition_binding_sha256={}\nfinal_link_receipt_sha256={}\nschema_sha256={schema_sha}\nvalidator_sha256={validator_sha}\norchestrator_sha256={orchestrator_sha}\nmanifest_sha256={primary_manifest_sha}\npayload_sha256={primary_payload_sha}\nsignature_sha256={primary_signature_sha}\npublic_key_sha256={expected_public_sha}\nreproducibility_builds=3\nnegative_results_sha256={negative_sha}\nnegative_cases=unknown-property,capsule-byte-drift,unknown-source-binding,direct-verus-artifact-mismatch,artifact-file-drift,platform-primitives-file-drift,composition-artifact-file-drift,final-link-receipt-file-drift,boot-image-file-drift,noncanonical-order,development-key-release,missing-composition-release,missing-boot-image-release,missing-final-link-receipt-release,schema-loosening,signature-mutation,payload-mutation\n",
        composition.binding_sha, composition.final_link_receipt_sha
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write manifest report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

struct ManifestCompositionEvidence {
    receipt_schema: String,
    binding_sha: String,
    receipt_sha: String,
    report_sha: String,
    final_link_receipt_sha: String,
}

fn validate_m0_composition_inputs(root: &Path) -> Result<ManifestCompositionEvidence, String> {
    let bundle = root.join("build/m0-composition/primary.verified");
    let receipt_path = bundle.join("receipt.json");
    let receipt_bytes =
        fs::read(&receipt_path).map_err(|error| format!("read composition receipt: {error}"))?;
    let receipt: serde_json::Value = serde_json::from_slice(&receipt_bytes)
        .map_err(|error| format!("parse composition receipt: {error}"))?;
    let receipt_schema = json_string(&receipt, "/schema", "composition receipt schema")?;
    if receipt_schema != "thermite.verified-composition-receipt.v1"
        || json_string(&receipt, "/binding/schema", "composition binding schema")? != receipt_schema
        || json_string(&receipt, "/binding/assurance", "composition assurance")? != "L3"
        || json_string(&receipt, "/binding/scope", "composition scope")? != "end_to_end"
    {
        return Err("composition receipt is not an end-to-end L3 composition binding".to_string());
    }

    let binding_sha =
        json_string(&receipt, "/binding_sha256", "composition binding digest")?.to_string();
    let receipt_sha = sha256sum(&receipt_path)?;
    let artifact_sha = validate_json_file_record(
        &bundle,
        receipt
            .pointer("/binding/artifact")
            .ok_or_else(|| "composition receipt has no artifact record".to_string())?,
        "artifact/libtmk_composition_probe.rlib",
        "composition artifact",
    )?;
    for (relative, label) in [
        ("evidence/input.th", "composition Thermite input"),
        (
            "evidence/direct-verus/00-composition_shell.rs",
            "composition direct-Verus shell",
        ),
        ("evidence/source.verus.rs", "composition combined source"),
        ("evidence/toolchain.json", "composition toolchain evidence"),
        (
            "evidence/translation-validation.json",
            "composition translation validation",
        ),
        ("evidence/verus-result.json", "composition Verus result"),
    ] {
        validate_receipt_inventory_file(&bundle, &receipt, relative, label)?;
    }

    let source_sha = sha256sum(&root.join("thermite/core/composition_probe.th"))?;
    let shell_sha = sha256sum(&root.join("tests/m0/composition_shell.rs"))?;
    let combined_source_sha = sha256sum(&bundle.join("evidence/source.verus.rs"))?;
    if json_string(
        &receipt,
        "/binding/raw_source_sha256",
        "composition raw-source digest",
    )? != source_sha
        || json_string(
            &receipt,
            "/binding/composition/combined_source_sha256",
            "composition combined-source digest",
        )? != combined_source_sha
        || json_string(
            &receipt,
            "/binding/verus_source_sha256",
            "composition Verus-source digest",
        )? != combined_source_sha
    {
        return Err(
            "composition receipt source binding does not match canonical inputs".to_string(),
        );
    }
    let bound_shell = receipt
        .pointer("/binding/files")
        .and_then(serde_json::Value::as_array)
        .and_then(|files| {
            files.iter().find(|record| {
                record.get("path").and_then(serde_json::Value::as_str)
                    == Some("evidence/direct-verus/00-composition_shell.rs")
            })
        })
        .and_then(|record| record.get("sha256"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "composition receipt has no direct-Verus shell digest".to_string())?;
    if bound_shell != shell_sha {
        return Err(format!(
            "composition receipt shell digest is {bound_shell}, canonical shell is {shell_sha}"
        ));
    }

    let forge = forge_binary()?;
    check_forge_skill(&forge)?;
    let forge_sha = sha256sum(&forge)?;
    if forge_sha != "3fad9e2b328367ad0169b297ea03165664edc854f6a026fcb08bcfcb814f35d4" {
        return Err(format!(
            "composition Forge digest is unexpected: {forge_sha}"
        ));
    }
    let thermite_revision_output = run_checked(
        Command::new("git").args(["-C", "/home/doll/Thermite", "rev-parse", "HEAD"]),
        "manifest composition Thermite revision",
    )?;
    let thermite_revision = String::from_utf8(thermite_revision_output.stdout)
        .map_err(|error| format!("Thermite revision is not UTF-8: {error}"))?;
    let thermite_revision = thermite_revision.trim();
    if thermite_revision != "845d684f00e829491ee4c537818fba2689bcaefc" {
        return Err(format!(
            "composition Thermite revision is {thermite_revision}, expected the manifest pin"
        ));
    }
    let thermite_status = run_checked(
        Command::new("git").args([
            "-C",
            "/home/doll/Thermite",
            "status",
            "--porcelain",
            "--untracked-files=all",
        ]),
        "manifest composition Thermite cleanliness",
    )?;
    if !thermite_status.stdout.is_empty() {
        return Err("composition Thermite repository is dirty".to_string());
    }

    let replay_output = run_checked(
        Command::new(&forge)
            .current_dir(root)
            .args(["verify-build"])
            .arg(&bundle)
            .args(["--replay", "--json"]),
        "manifest replay of composition receipt",
    )?;
    let replay: serde_json::Value = serde_json::from_slice(&replay_output.stdout)
        .map_err(|error| format!("parse composition replay result: {error}"))?;
    if replay.get("replayed").and_then(serde_json::Value::as_bool) != Some(true)
        || json_string(&replay, "/binding_sha256", "composition replay binding")? != binding_sha
        || json_string(&replay, "/artifact_sha256", "composition replay artifact")? != artifact_sha
    {
        return Err("composition replay result does not match the consumed receipt".to_string());
    }

    let final_link_receipt_sha =
        validate_m0_final_link_receipt(root, &receipt, &binding_sha, &receipt_sha)?;
    let report_path = root.join("build/m0-composition/report.txt");
    let report = fs::read_to_string(&report_path)
        .map_err(|error| format!("read composition acceptance report: {error}"))?;
    let report_sha = sha256sum(&report_path)?;
    if report.lines().next() != Some("M0_COMPOSITION_OK") {
        return Err("composition report has no success headline".to_string());
    }
    for (key, expected) in [
        ("component_verified", "true".to_string()),
        ("release_eligible", "false".to_string()),
        ("receipt_validated", "true".to_string()),
        ("receipt_replayed", "true".to_string()),
        ("final_link_receipted", "true".to_string()),
        ("linked_primitives_verified", "true".to_string()),
        ("selected_primitives", "memcpy".to_string()),
        ("positive_gates", "13".to_string()),
        ("forge_revision", thermite_revision.to_string()),
        ("forge_sha256", forge_sha),
        (
            "skill_sha256",
            sha256sum(&PathBuf::from(
                "/home/doll/.codex/skills/thermite/references/language.md",
            ))?,
        ),
        ("source_sha256", source_sha),
        ("shell_sha256", shell_sha),
        ("combined_source_sha256", combined_source_sha),
        ("receipt_sha256", receipt_sha.clone()),
        ("binding_sha256", binding_sha.clone()),
        ("artifact_sha256", artifact_sha),
        (
            "platform_primitive_object_sha256",
            sha256sum(
                &root.join("build/m0-platform-primitives/objects/platform-primitives.o"),
            )?,
        ),
        (
            "final_link_receipt_sha256",
            final_link_receipt_sha.clone(),
        ),
        (
            "hosted_consumer_sha256",
            sha256sum(&root.join("build/m0-composition/composition-consumer"))?,
        ),
        (
            "low_static_consumer_sha256",
            sha256sum(&root.join("build/m0-composition/composition-kernel-low"))?,
        ),
        (
            "high_half_consumer_sha256",
            sha256sum(&root.join("build/m0-composition/composition-kernel-high-half"))?,
        ),
        ("composition_reproducibility_builds", "3".to_string()),
        ("low_static_reproducibility_links", "3".to_string()),
        ("high_half_reproducibility_links", "3".to_string()),
        ("absolute_path_reproducibility_roots", "2".to_string()),
        (
            "hosted_runtime_marker",
            "M0_COMPOSITION_OK:store:reject:1".to_string(),
        ),
        ("freestanding_runtime", "fail-stop-timeout-124".to_string()),
        ("high_half_link_base", "ffffffff80000000".to_string()),
        (
            "negative_cases",
            "artifact-tamper,binding-tamper,certificate-l2,external-body,extra-file,host-rustc,post-plan-shell,private-export,rich-standalone-export,shell-tamper,tv-nonpass".to_string(),
        ),
    ] {
        let actual = report_field(&report, key)?;
        if actual != expected {
            return Err(format!(
                "composition report field `{key}` is `{actual}`, expected `{expected}`"
            ));
        }
    }

    Ok(ManifestCompositionEvidence {
        receipt_schema: receipt_schema.to_string(),
        binding_sha,
        receipt_sha,
        report_sha,
        final_link_receipt_sha,
    })
}

fn validate_m0_final_link_receipt(
    root: &Path,
    composition_receipt: &serde_json::Value,
    composition_binding_sha: &str,
    composition_receipt_sha: &str,
) -> Result<String, String> {
    let path = root.join("build/m0-composition/final-link-receipt.json");
    let bytes = fs::read(&path).map_err(|error| format!("read final-link receipt: {error}"))?;
    if canonical_json(&bytes, "final-link receipt")? != bytes {
        return Err("final-link receipt is not canonical JSON".to_string());
    }
    let receipt: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse final-link receipt: {error}"))?;
    ensure_json_object_keys(
        &receipt,
        "",
        &["composition", "link_plan", "outputs", "platform", "schema"],
        "final-link receipt",
    )?;
    if json_string(&receipt, "/schema", "final-link receipt schema")? != "tmk.final-link-receipt.v1"
        || json_string(
            &receipt,
            "/composition/binding_sha256",
            "final-link composition binding",
        )? != composition_binding_sha
        || json_string(
            &receipt,
            "/composition/receipt_sha256",
            "final-link composition receipt digest",
        )? != composition_receipt_sha
        || receipt
            .pointer("/composition/replay_passed")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
    {
        return Err("final-link receipt does not bind the replayed composition".to_string());
    }

    validate_json_file_record(
        root,
        receipt
            .pointer("/composition/artifact")
            .ok_or_else(|| "final-link receipt has no composition artifact".to_string())?,
        "build/m0-composition/primary.verified/artifact/libtmk_composition_probe.rlib",
        "final-link composition artifact",
    )?;
    let receipt_dependencies = receipt
        .pointer("/composition/dependencies")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "final-link receipt has no dependency array".to_string())?;
    let composition_dependencies: Vec<_> = composition_receipt
        .pointer("/binding/files")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "composition receipt has no file inventory".to_string())?
        .iter()
        .filter(|record| {
            record
                .get("path")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|path| path.starts_with("artifact/deps/"))
        })
        .cloned()
        .collect();
    if receipt_dependencies != composition_dependencies.as_slice()
        || receipt_dependencies.len() != 4
    {
        return Err(
            "final-link dependency allowlist differs from the composition receipt".to_string(),
        );
    }
    let bundle = root.join("build/m0-composition/primary.verified");
    for record in receipt_dependencies {
        let relative = record
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "final-link dependency has no path".to_string())?;
        validate_json_file_record(&bundle, record, relative, "final-link dependency")?;
    }

    for (pointer, relative, label) in [
        (
            "/platform/object",
            "build/m0-platform-primitives/objects/platform-primitives.o",
            "final-link platform object",
        ),
        (
            "/platform/acceptance_report",
            "build/m0-platform-primitives/report.txt",
            "final-link platform report",
        ),
        (
            "/platform/linked_primitive",
            "build/m0-composition/linked-primitives/memcpy.bin",
            "final-link linked primitive",
        ),
        (
            "/link_plan/consumer",
            "tests/m0/composition_kernel_consumer.rs",
            "final-link consumer",
        ),
        (
            "/link_plan/linker_script",
            "tests/m0/global_allocator_kernel.ld",
            "final-link linker script",
        ),
        (
            "/outputs/low_static",
            "build/m0-composition/composition-kernel-low",
            "final-link low image",
        ),
        (
            "/outputs/higher_half",
            "build/m0-composition/composition-kernel-high-half",
            "final-link higher-half image",
        ),
    ] {
        validate_json_file_record(
            root,
            receipt
                .pointer(pointer)
                .ok_or_else(|| format!("{label} record is missing"))?,
            relative,
            label,
        )?;
    }
    if json_string(
        &receipt,
        "/link_plan/orchestrator_source_sha256",
        "final-link orchestrator digest",
    )? != sha256sum(&root.join("xtask/src/composition.rs"))?
        || json_u64(
            &receipt,
            "/link_plan/undefined_symbols",
            "final-link undefined-symbol count",
        )? != 0
        || json_string(&receipt, "/outputs/higher_half_entry", "higher-half entry")?
            != "ffffffff80000000"
        || json_u64(
            &receipt,
            "/outputs/low_static_reproducibility_links",
            "low-link reproducibility count",
        )? != 3
        || json_u64(
            &receipt,
            "/outputs/higher_half_reproducibility_links",
            "higher-half-link reproducibility count",
        )? != 3
        || json_u64(
            &receipt,
            "/outputs/absolute_path_reproducibility_roots",
            "absolute-path reproducibility root count",
        )? != 2
        || json_string(
            &receipt,
            "/outputs/freestanding_runtime",
            "freestanding composition runtime",
        )? != "fail-stop-timeout-124"
    {
        return Err("final-link receipt policy fields are inconsistent".to_string());
    }

    let selected = receipt
        .pointer("/link_plan/selected_symbols")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "final-link receipt has no selected-symbol array".to_string())?;
    let selected_expected = serde_json::json!([
        "memcpy",
        "tmk_composition_probe::composition_shell::boot_observation",
        "tmk_composition_probe::composition_step"
    ]);
    if selected != selected_expected.as_array().expect("literal array") {
        return Err("final-link selected-symbol allowlist is unexpected".to_string());
    }
    let discarded = receipt
        .pointer("/link_plan/discarded_platform_symbols")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "final-link receipt has no discarded-symbol array".to_string())?;
    let discarded_expected = serde_json::json!(["memset", "tmk_alloc_capsule", "tmk_seal_capsule"]);
    if discarded != discarded_expected.as_array().expect("literal array") {
        return Err("final-link discarded-symbol allowlist is unexpected".to_string());
    }

    let tools = receipt
        .pointer("/link_plan/tools")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "final-link receipt has no tool array".to_string())?;
    let expected_tools = [
        (
            "cc",
            "/usr/sbin/cc",
            "1ce580ecfabf35747bc550481621e2f2c04fd8fc23b8182779f33b82d07856d0",
        ),
        (
            "ld",
            "/usr/sbin/ld",
            "6cf122245638eb45fd981c75bf3a116675b3f9c7510ae2e3b386aa6738e46505",
        ),
        (
            "nm",
            "/usr/sbin/nm",
            "988d8ded768c4e59284a44f641e92db6c0c8dd222547c32ce432577ff3cb9cc6",
        ),
        (
            "objcopy",
            "/usr/sbin/objcopy",
            "8f09a5b2d8e2b34aebf269fffef2308492a022dcfedf87b49489592e838129b4",
        ),
        (
            "readelf",
            "/usr/sbin/readelf",
            "59d345f2a2b47f5617e8f53c72f6db5169c723c11d3e809a9e6e3c5673f2420c",
        ),
        (
            "rustc-codegen",
            "/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc",
            "bff349e72704ff70bc08a234a3847338e797065bbedde5e556808bc87b7bf7c6",
        ),
        (
            "timeout",
            "/usr/bin/timeout",
            "350001cc47ad731c4e797532fe46a999477aba359692e2de3e93f316b4021dab",
        ),
    ];
    if tools.len() != expected_tools.len() {
        return Err("final-link tool allowlist has the wrong length".to_string());
    }
    for (record, (name, tool_path, expected_sha)) in tools.iter().zip(expected_tools) {
        if record.get("name").and_then(serde_json::Value::as_str) != Some(name)
            || record.get("path").and_then(serde_json::Value::as_str) != Some(tool_path)
            || record.get("sha256").and_then(serde_json::Value::as_str) != Some(expected_sha)
            || sha256sum(Path::new(tool_path))? != expected_sha
        {
            return Err(format!(
                "final-link tool `{name}` does not match its live pin"
            ));
        }
    }

    let high = root.join("build/m0-composition/composition-kernel-high-half");
    let undefined = run_checked(
        Command::new("/usr/sbin/nm").arg("-u").arg(&high),
        "manifest final-link undefined-symbol audit",
    )?;
    if !undefined.stdout.is_empty() {
        return Err("manifest final-link image has undefined symbols".to_string());
    }
    let symbols = run_checked(
        Command::new("/usr/sbin/nm").arg("-C").arg(&high),
        "manifest final-link symbol audit",
    )?;
    let symbols = String::from_utf8_lossy(&symbols.stdout);
    for symbol in selected_expected
        .as_array()
        .expect("literal array")
        .iter()
        .filter_map(serde_json::Value::as_str)
    {
        if !symbols.contains(symbol) {
            return Err(format!(
                "manifest final-link image lacks selected symbol `{symbol}`"
            ));
        }
    }
    for symbol in discarded_expected
        .as_array()
        .expect("literal array")
        .iter()
        .filter_map(serde_json::Value::as_str)
    {
        if symbols
            .lines()
            .any(|line| line.split_whitespace().last() == Some(symbol))
        {
            return Err(format!(
                "manifest final-link image retains discarded symbol `{symbol}`"
            ));
        }
    }
    let header = run_checked(
        Command::new("/usr/sbin/readelf").args(["-hW"]).arg(&high),
        "manifest final-link ELF-header audit",
    )?;
    require_output_fragments(
        &header.stdout,
        "manifest final-link ELF header",
        &["Entry point address:               0xffffffff80000000"],
    )?;

    let audit = root.join("build/m0-manifest-link-audit");
    if audit.exists() {
        fs::remove_dir_all(&audit)
            .map_err(|error| format!("remove stale manifest link audit: {error}"))?;
    }
    platform_primitives::audit_linked_composition_primitives(&high, &audit)?;
    let extracted = fs::read(audit.join("memcpy.bin"))
        .map_err(|error| format!("read manifest-extracted memcpy: {error}"))?;
    let registered = fs::read(root.join("build/m0-platform-primitives/emitted/memcpy.bin"))
        .map_err(|error| format!("read registered memcpy: {error}"))?;
    let receipted = fs::read(root.join("build/m0-composition/linked-primitives/memcpy.bin"))
        .map_err(|error| format!("read receipted linked memcpy: {error}"))?;
    if extracted != registered || extracted != receipted {
        return Err(
            "manifest-extracted memcpy does not match the receipted registered bytes".to_string(),
        );
    }
    fs::remove_dir_all(&audit).map_err(|error| format!("remove manifest link audit: {error}"))?;
    sha256sum(&path)
}

fn validate_receipt_inventory_file(
    bundle: &Path,
    receipt: &serde_json::Value,
    relative: &str,
    label: &str,
) -> Result<String, String> {
    let record = receipt
        .pointer("/binding/files")
        .and_then(serde_json::Value::as_array)
        .and_then(|records| {
            records.iter().find(|record| {
                record.get("path").and_then(serde_json::Value::as_str) == Some(relative)
            })
        })
        .ok_or_else(|| format!("composition receipt inventory omits `{relative}`"))?;
    validate_json_file_record(bundle, record, relative, label)
}

fn validate_json_file_record(
    root: &Path,
    record: &serde_json::Value,
    expected_relative: &str,
    label: &str,
) -> Result<String, String> {
    let relative = record
        .get("path")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{label} has no path"))?;
    let expected_size = record
        .get("length")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("{label} has no length"))?;
    let expected_sha = record
        .get("sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{label} has no digest"))?;
    if relative != expected_relative
        || relative.starts_with('/')
        || relative
            .split('/')
            .any(|part| part.is_empty() || part == "..")
    {
        return Err(format!(
            "{label} path `{relative}` is not the expected normalized path"
        ));
    }
    let path = root.join(relative);
    require_file(&path, label)?;
    let actual_size = fs::metadata(&path)
        .map_err(|error| format!("stat {label}: {error}"))?
        .len();
    let actual_sha = sha256sum(&path)?;
    if actual_size != expected_size || actual_sha != expected_sha {
        return Err(format!(
            "{label} file is {actual_size}/{actual_sha}, receipt binds {expected_size}/{expected_sha}"
        ));
    }
    Ok(actual_sha)
}

fn ensure_json_object_keys(
    value: &serde_json::Value,
    pointer: &str,
    expected: &[&str],
    label: &str,
) -> Result<(), String> {
    let object = if pointer.is_empty() {
        value.as_object()
    } else {
        value
            .pointer(pointer)
            .and_then(serde_json::Value::as_object)
    }
    .ok_or_else(|| format!("{label} is not an object at `{pointer}`"))?;
    let mut actual: Vec<_> = object.keys().map(String::as_str).collect();
    let mut expected = expected.to_vec();
    actual.sort_unstable();
    expected.sort_unstable();
    if actual != expected {
        return Err(format!(
            "{label} has keys {actual:?}, expected exactly {expected:?}"
        ));
    }
    Ok(())
}

fn json_u64(value: &serde_json::Value, pointer: &str, label: &str) -> Result<u64, String> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("{label} is missing or is not an unsigned integer at `{pointer}`"))
}

fn manifest_artifact(
    root: &Path,
    name: &str,
    kind: &str,
    relative_path: &str,
    mut source_bindings: Vec<String>,
    executable: bool,
) -> Result<serde_json::Value, String> {
    let path = root.join(relative_path);
    require_file(&path, &format!("manifest artifact `{name}`"))?;
    source_bindings.sort();
    source_bindings.dedup();
    let size = fs::metadata(&path)
        .map_err(|error| {
            format!(
                "read manifest artifact metadata {}: {error}",
                path.display()
            )
        })?
        .len();
    Ok(serde_json::json!({
        "name": name,
        "kind": kind,
        "path": relative_path,
        "sha256": sha256sum(&path)?,
        "size": size,
        "source_bindings": source_bindings,
        "executable": executable
    }))
}

fn validate_manifest_artifact_files(
    root: &Path,
    manifest_value: &serde_json::Value,
) -> Result<(), String> {
    let artifacts = manifest_value
        .get("artifacts")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "manifest artifact file replay has no artifact array".to_string())?;
    for artifact in artifacts {
        let name = artifact
            .get("name")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "manifest artifact has no name".to_string())?;
        let relative = artifact
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("manifest artifact `{name}` has no path"))?;
        let expected_sha = artifact
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("manifest artifact `{name}` has no digest"))?;
        let expected_size = artifact
            .get("size")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("manifest artifact `{name}` has no size"))?;
        let path = root.join(relative);
        require_file(&path, &format!("manifest artifact `{name}` replay"))?;
        let actual_size = fs::metadata(&path)
            .map_err(|error| format!("read manifest artifact `{name}` metadata: {error}"))?
            .len();
        if actual_size != expected_size {
            return Err(format!(
                "manifest artifact `{name}` file size is {actual_size}, bound size is {expected_size}"
            ));
        }
        let actual_sha = sha256sum(&path)?;
        if actual_sha != expected_sha {
            return Err(format!(
                "manifest artifact `{name}` file digest is {actual_sha}, bound digest is {expected_sha}"
            ));
        }
    }
    Ok(())
}

fn sign_manifest(
    openssl: &Path,
    private_key: &Path,
    manifest_value: &mut serde_json::Value,
    output: &Path,
) -> Result<(), String> {
    fs::create_dir(output).map_err(|error| {
        format!(
            "create manifest signing directory {}: {error}",
            output.display()
        )
    })?;
    let payload = manifest::canonical_payload(manifest_value)?;
    let payload_path = output.join("payload.json");
    fs::write(&payload_path, payload)
        .map_err(|error| format!("write manifest signing payload: {error}"))?;
    let payload_sha = sha256sum(&payload_path)?;
    *manifest_value
        .pointer_mut("/signing/payload_sha256")
        .ok_or_else(|| "manifest has no signing payload digest field".to_string())? =
        serde_json::json!(payload_sha);

    let signature_path = output.join("signature.bin");
    run_checked(
        Command::new(openssl)
            .args(["pkeyutl", "-sign", "-rawin", "-inkey"])
            .arg(private_key)
            .args(["-keyform", "DER"])
            .arg("-in")
            .arg(&payload_path)
            .arg("-out")
            .arg(&signature_path),
        "sign canonical M0 manifest payload",
    )?;
    let signature = fs::read(&signature_path)
        .map_err(|error| format!("read Ed25519 manifest signature: {error}"))?;
    if signature.len() != 64 {
        return Err(format!(
            "Ed25519 manifest signature is {} bytes, expected 64",
            signature.len()
        ));
    }
    *manifest_value
        .pointer_mut("/signing/signature")
        .ok_or_else(|| "manifest has no signature field".to_string())? =
        serde_json::json!(hex_encode(&signature));
    fs::write(
        output.join("manifest.json"),
        manifest::canonical_manifest(manifest_value)?,
    )
    .map_err(|error| format!("write signed release manifest: {error}"))?;
    Ok(())
}

fn verify_manifest_signature(
    openssl: &Path,
    public_key: &Path,
    expected_public_sha: &str,
    manifest_value: &serde_json::Value,
    output: &Path,
    label: &str,
) -> Result<(), String> {
    let public_sha = sha256sum(public_key)?;
    let bound_public_sha = json_string(
        manifest_value,
        "/signing/public_key_sha256",
        "manifest public key digest",
    )?;
    if public_sha != expected_public_sha || public_sha != bound_public_sha {
        return Err(format!(
            "manifest public key digest is {public_sha}, expected/bound {expected_public_sha}/{bound_public_sha}"
        ));
    }
    let payload = manifest::canonical_payload(manifest_value)?;
    let payload_path = output.join("verify-payload.json");
    fs::write(&payload_path, payload)
        .map_err(|error| format!("write manifest verification payload: {error}"))?;
    let payload_sha = sha256sum(&payload_path)?;
    let bound_payload_sha = json_string(
        manifest_value,
        "/signing/payload_sha256",
        "manifest payload digest",
    )?;
    if payload_sha != bound_payload_sha {
        return Err(format!(
            "manifest signing payload digest is {payload_sha}, bound digest is {bound_payload_sha}"
        ));
    }
    let signature = hex_decode(json_string(
        manifest_value,
        "/signing/signature",
        "manifest signature",
    )?)?;
    let signature_path = output.join("verify-signature.bin");
    fs::write(&signature_path, signature)
        .map_err(|error| format!("write decoded manifest signature: {error}"))?;
    let result = run_checked(
        Command::new(openssl)
            .args(["pkeyutl", "-verify", "-pubin", "-inkey"])
            .arg(public_key)
            .args(["-rawin", "-in"])
            .arg(&payload_path)
            .arg("-sigfile")
            .arg(&signature_path),
        label,
    )?;
    write_combined_output(&output.join("verification.txt"), &result, label)
}

fn verify_manifest_signature_expect_failure(
    openssl: &Path,
    public_key: &Path,
    manifest_value: &serde_json::Value,
    output: &Path,
    label: &str,
) -> Result<String, String> {
    if !output.exists() {
        fs::create_dir(output)
            .map_err(|error| format!("create signature rejection directory: {error}"))?;
    }
    let payload_path = output.join("verify-payload.json");
    fs::write(&payload_path, manifest::canonical_payload(manifest_value)?)
        .map_err(|error| format!("write rejected manifest payload: {error}"))?;
    let signature = hex_decode(json_string(
        manifest_value,
        "/signing/signature",
        "manifest signature",
    )?)?;
    let signature_path = output.join("verify-signature.bin");
    fs::write(&signature_path, signature)
        .map_err(|error| format!("write rejected manifest signature: {error}"))?;
    let result = run_expect_failure(
        Command::new(openssl)
            .args(["pkeyutl", "-verify", "-pubin", "-inkey"])
            .arg(public_key)
            .args(["-rawin", "-in"])
            .arg(&payload_path)
            .arg("-sigfile")
            .arg(&signature_path),
        label,
    )?;
    write_combined_output(&output.join("rejection.txt"), &result, label)?;
    let mut diagnostic = Vec::new();
    diagnostic.extend_from_slice(&result.stdout);
    diagnostic.extend_from_slice(&result.stderr);
    let diagnostic = String::from_utf8_lossy(&diagnostic).trim().to_string();
    if diagnostic.is_empty() {
        return Err(format!("{label} failed without a diagnostic"));
    }
    Ok(diagnostic.replace('\n', " | "))
}

fn record_manifest_rejection(
    schema: &serde_json::Value,
    manifest_value: &serde_json::Value,
    label: &str,
    expected: &str,
    results: &mut String,
) -> Result<(), String> {
    let diagnostic = manifest::validate(schema, manifest_value)
        .err()
        .ok_or_else(|| format!("manifest mutation `{label}` unexpectedly passed validation"))?;
    if !diagnostic.contains(expected) {
        return Err(format!(
            "manifest mutation `{label}` diagnostic `{diagnostic}` does not contain `{expected}`"
        ));
    }
    results.push_str(&format!("{label}: {diagnostic}\n"));
    Ok(())
}

fn git_output(root: &Path, args: &[&str], label: &str) -> Result<String, String> {
    let output = run_checked(Command::new("git").current_dir(root).args(args), label)?;
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|error| format!("{label} output is not UTF-8: {error}"))
}

fn hex_encode(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    result
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("hex value has odd length".to_string());
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_nibble(pair[0])?;
            let low = hex_nibble(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(format!("invalid lowercase hex byte 0x{byte:02x}")),
    }
}

fn m0_forge_probe() -> Result<(), String> {
    let root = workspace_root()?;
    let forge = forge_binary()?;
    check_forge_skill(&forge)?;

    let work = root.join("build/m0");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;

    let thermite = root.join("thermite/core/probe.th");
    let check = run_checked(
        Command::new(&forge)
            .current_dir(&root)
            .arg("check")
            .arg(&thermite)
            .args(["--level", "l3", "--json"]),
        "Forge standalone L3 source check",
    )?;
    require_output_fragments(
        &check.stdout,
        "Forge standalone L3 source check",
        &[
            "\"item\": \"transition_probe\"",
            "\"level\": \"L3\"",
            "\"mutants_killed\": \"4/4\"",
            "\"kind\": \"end_to_end\"",
        ],
    )?;
    fs::write(work.join("source-check.json"), &check.stdout)
        .map_err(|error| format!("write standalone source-check evidence: {error}"))?;

    let audit = run_checked(
        Command::new(&forge)
            .current_dir(&root)
            .arg("audit")
            .arg(&thermite)
            .args(["--json", "--meaning", "--metrics"]),
        "Forge standalone audit",
    )?;
    require_output_fragments(
        &audit.stdout,
        "Forge standalone audit",
        &["\"project_assurance\"", "\"level\": \"L3\""],
    )?;
    fs::write(work.join("source-audit.txt"), &audit.stdout)
        .map_err(|error| format!("write standalone audit evidence: {error}"))?;

    let battery = run_checked(
        Command::new(&forge)
            .current_dir(&root)
            .arg("battery")
            .arg(&thermite),
        "Forge standalone mutation battery",
    )?;
    require_output_fragments(
        &battery.stdout,
        "Forge standalone mutation battery",
        &["battery — transition_probe", "mutants killed: 4/4"],
    )?;
    fs::write(work.join("source-battery.txt"), &battery.stdout)
        .map_err(|error| format!("write standalone battery evidence: {error}"))?;

    let bundle = work.join("probe.verified");
    run_checked(
        Command::new(&forge)
            .current_dir(&root)
            .arg("build")
            .arg(&thermite)
            .args(["--level", "l3", "--export", "transition_probe"])
            .args(["--crate-name", "tmk_probe", "--target", "kernel"])
            .arg("--out")
            .arg(&bundle),
        "Forge exact-source L3 kernel build",
    )?;
    run_checked(
        Command::new(&forge)
            .current_dir(&root)
            .arg("verify-build")
            .arg(&bundle),
        "Forge bundle validation",
    )?;
    run_checked(
        Command::new(&forge)
            .current_dir(&root)
            .arg("verify-build")
            .arg(&bundle)
            .arg("--replay"),
        "Forge bundle replay",
    )?;

    let receipt_path = bundle.join("receipt.json");
    let receipt = fs::read_to_string(&receipt_path)
        .map_err(|error| format!("read {}: {error}", receipt_path.display()))?;
    let receipt_json: serde_json::Value = serde_json::from_str(&receipt)
        .map_err(|error| format!("parse {}: {error}", receipt_path.display()))?;
    for required in [
        "\"assurance\": \"L3\"",
        "\"scope\": \"end_to_end\"",
        "\"target\": \"kernel\"",
        "\"thermite_name\": \"transition_probe\"",
    ] {
        if !receipt.contains(required) {
            return Err(format!(
                "verified-build receipt is missing required field fragment `{required}`"
            ));
        }
    }

    let artifact = bundle.join("artifact/libtmk_probe.rlib");
    let deps = bundle.join("artifact/deps");
    require_file(&artifact, "Forge L3 rlib")?;
    if !deps.is_dir() {
        return Err(format!(
            "missing Forge dependency directory {}",
            deps.display()
        ));
    }

    if env::var_os("TMK_UNBOUND_CODEGEN_RUSTC").is_some() {
        return Err(
            "TMK_UNBOUND_CODEGEN_RUSTC is obsolete; the consumer compiler must come from the bound receipt"
                .to_string(),
        );
    }
    let toolchain_path = bundle.join("evidence/toolchain.json");
    require_file(&toolchain_path, "Forge toolchain evidence")?;
    let toolchain_text = fs::read_to_string(&toolchain_path)
        .map_err(|error| format!("read {}: {error}", toolchain_path.display()))?;
    let toolchain_json: serde_json::Value = serde_json::from_str(&toolchain_text)
        .map_err(|error| format!("parse {}: {error}", toolchain_path.display()))?;
    let toolchain_sha = sha256sum(&toolchain_path)?;
    let bound_toolchain_sha = json_string(
        &receipt_json,
        "/binding/toolchain_sha256",
        "receipt toolchain digest",
    )?;
    if toolchain_sha != bound_toolchain_sha {
        return Err(format!(
            "toolchain evidence digest is {toolchain_sha}, receipt binds {bound_toolchain_sha}"
        ));
    }
    for (pointer, expected, label) in [
        (
            "/artifact_codegen/selection",
            "verus --version Toolchain",
            "codegen compiler selection",
        ),
        (
            "/artifact_codegen/rustup_toolchain",
            "1.95.0-x86_64-unknown-linux-gnu",
            "codegen rustup toolchain",
        ),
        (
            "/artifact_codegen/rustc_release",
            "1.95.0",
            "codegen rustc release",
        ),
        (
            "/artifact_codegen/rustc_sha256",
            "bff349e72704ff70bc08a234a3847338e797065bbedde5e556808bc87b7bf7c6",
            "codegen rustc digest",
        ),
        (
            "/artifact_codegen/target_triple",
            "x86_64-unknown-linux-gnu",
            "codegen target triple",
        ),
        (
            "/artifact_codegen/target_pointer_width",
            "64",
            "codegen target pointer width",
        ),
        (
            "/artifact_codegen/target_endian",
            "little",
            "codegen target endianness",
        ),
        (
            "/host_rustc/rustc_sha256",
            "ba4b837efb6612dfa8d941c5a72b8a50d1d03a0f36216743b173949aa8d9eb75",
            "ambient host rustc digest",
        ),
    ] {
        let actual = json_string(&toolchain_json, pointer, label)?;
        if actual != expected {
            return Err(format!("{label} is `{actual}`, expected `{expected}`"));
        }
    }

    let consumer_rustc = PathBuf::from(json_string(
        &toolchain_json,
        "/artifact_codegen/rustc_path",
        "codegen rustc path",
    )?);
    require_file(&consumer_rustc, "receipt-selected codegen rustc")?;
    let recorded_consumer_sha = json_string(
        &toolchain_json,
        "/artifact_codegen/rustc_sha256",
        "codegen rustc digest",
    )?;
    let consumer_rustc_sha = sha256sum(&consumer_rustc)?;
    if consumer_rustc_sha != recorded_consumer_sha {
        return Err(format!(
            "receipt-selected rustc digest is {consumer_rustc_sha}, expected {recorded_consumer_sha}"
        ));
    }
    let incompatible_rustc = PathBuf::from(json_string(
        &toolchain_json,
        "/host_rustc/rustc_path",
        "ambient host rustc path",
    )?);
    require_file(
        &incompatible_rustc,
        "receipt-recorded incompatible host rustc",
    )?;
    let recorded_incompatible_sha = json_string(
        &toolchain_json,
        "/host_rustc/rustc_sha256",
        "ambient host rustc digest",
    )?;
    let incompatible_rustc_sha = sha256sum(&incompatible_rustc)?;
    if incompatible_rustc_sha != recorded_incompatible_sha {
        return Err(format!(
            "receipt-recorded host rustc digest is {incompatible_rustc_sha}, expected {recorded_incompatible_sha}"
        ));
    }

    let host_consumer = work.join("host-probe-consumer");
    compile_consumer(
        &consumer_rustc,
        &root,
        &root.join("tests/m0/host_probe_consumer.rs"),
        &artifact,
        &deps,
        &host_consumer,
        false,
    )?;
    let output = run_checked(
        Command::new(&host_consumer).current_dir(&root),
        "execute linked Forge L3 function",
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim() != EXPECTED_RUNTIME_MARKER {
        return Err(format!(
            "runtime probe emitted `{}`, expected `{EXPECTED_RUNTIME_MARKER}`",
            stdout.trim()
        ));
    }

    let kernel_consumer = work.join("kernel-probe-consumer");
    compile_consumer(
        &consumer_rustc,
        &root,
        &root.join("tests/m0/kernel_probe_consumer.rs"),
        &artifact,
        &deps,
        &kernel_consumer,
        true,
    )?;
    require_file(&kernel_consumer, "linked no_std consumer")?;

    let incompatible_output_path = work.join("incompatible-host-probe-consumer");
    let incompatible = run_expect_failure(
        &mut consumer_command(
            &incompatible_rustc,
            &root,
            &root.join("tests/m0/host_probe_consumer.rs"),
            &artifact,
            &deps,
            &incompatible_output_path,
            false,
        ),
        "reject incompatible receipt-recorded host rustc consumer",
    )?;
    let mut incompatible_diagnostic = Vec::new();
    incompatible_diagnostic.extend_from_slice(&incompatible.stdout);
    incompatible_diagnostic.extend_from_slice(&incompatible.stderr);
    require_output_fragments(
        &incompatible_diagnostic,
        "incompatible host rustc rejection",
        &["incompatible version of rustc", "compiled by rustc 1.95.0"],
    )?;
    write_combined_output(
        &work.join("incompatible-rustc-result.txt"),
        &incompatible,
        "incompatible receipt-recorded host rustc",
    )?;

    let receipt_sha = sha256sum(&receipt_path)?;
    let artifact_sha = sha256sum(&artifact)?;
    let consumer_sha = sha256sum(&kernel_consumer)?;
    let report = format!(
        "M0_FORGE_PROBE_OK\nrelease_eligible=true\nmutants_killed=4/4\nconsumer_rustc={}\nconsumer_rustc_sha256={consumer_rustc_sha}\ntoolchain_evidence_sha256={toolchain_sha}\nreceipt_sha256={receipt_sha}\nartifact_sha256={artifact_sha}\nno_std_consumer_sha256={consumer_sha}\nincompatible_host_rustc_rejected=true\nruntime_marker={EXPECTED_RUNTIME_MARKER}\n",
        consumer_rustc.display()
    );
    let report_path = work.join("forge-probe-report.txt");
    fs::write(&report_path, &report)
        .map_err(|error| format!("write {}: {error}", report_path.display()))?;
    print!("{report}");
    println!("bundle={}", bundle.display());
    println!("report={}", report_path.display());
    Ok(())
}

fn m0_composition_source_check() -> Result<(), String> {
    let root = workspace_root()?;
    let forge = forge_binary()?;
    check_forge_skill(&forge)?;

    let source = root.join("thermite/core/composition_probe.th");
    require_file(&source, "rich-state composition probe")?;
    let work = root.join("build/m0-composition-source");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;

    let check = run_checked(
        Command::new(&forge)
            .current_dir(&root)
            .arg("check")
            .arg(&source)
            .args(["--level", "l3", "--json"]),
        "Forge rich-state L3 source check",
    )?;
    require_output_fragments(
        &check.stdout,
        "Forge rich-state L3 source check",
        &[
            "\"item\": \"composition_step\"",
            "\"level\": \"L3\"",
            "\"mutants_killed\": \"11/11\"",
            "\"kind\": \"end_to_end\"",
        ],
    )?;
    fs::write(work.join("check.json"), &check.stdout)
        .map_err(|error| format!("write rich-state check evidence: {error}"))?;

    let audit = run_checked(
        Command::new(&forge)
            .current_dir(&root)
            .arg("audit")
            .arg(&source)
            .args(["--json", "--meaning", "--metrics"]),
        "Forge rich-state audit",
    )?;
    require_output_fragments(
        &audit.stdout,
        "Forge rich-state audit",
        &["\"project_assurance\"", "\"level\": \"L3\""],
    )?;
    fs::write(work.join("audit.txt"), &audit.stdout)
        .map_err(|error| format!("write rich-state audit evidence: {error}"))?;

    let battery = run_checked(
        Command::new(&forge)
            .current_dir(&root)
            .arg("battery")
            .arg(&source),
        "Forge rich-state mutation battery",
    )?;
    require_output_fragments(
        &battery.stdout,
        "Forge rich-state mutation battery",
        &["battery — composition_step", "mutants killed: 11/11"],
    )?;
    fs::write(work.join("battery.txt"), &battery.stdout)
        .map_err(|error| format!("write rich-state battery evidence: {error}"))?;

    let unavailable_bundle = work.join("composition-unavailable.verified");
    let rejected = run_expect_failure(
        Command::new(&forge)
            .current_dir(&root)
            .arg("build")
            .arg(&source)
            .args(["--level", "l3", "--export", "composition_step"])
            .args([
                "--crate-name",
                "tmk_composition_probe",
                "--target",
                "kernel",
            ])
            .arg("--out")
            .arg(&unavailable_bundle),
        "Forge rich-state standalone-export refusal",
    )?;
    require_output_fragments(
        &rejected.stderr,
        "Forge rich-state standalone-export refusal",
        &["outside the v1 verified public ABI"],
    )?;

    let report = "M0_COMPOSITION_SOURCE_OK\nrelease_eligible=false\ncomposition_build=available-via-m0-composition\nmutants_killed=11/11\n";
    fs::write(work.join("report.txt"), report)
        .map_err(|error| format!("write rich-state composition report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn m0_forge_tamper() -> Result<(), String> {
    let root = workspace_root()?;
    let forge = forge_binary()?;
    check_forge_skill(&forge)?;

    let work = root.join("build/m0-tamper");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    let results = work.join("results");
    fs::create_dir_all(&results)
        .map_err(|error| format!("create {}: {error}", results.display()))?;

    let source = root.join("thermite/core/probe.th");
    let base = work.join("base.verified");
    run_checked(
        Command::new(&forge)
            .current_dir(&root)
            .arg("build")
            .arg(&source)
            .args(["--level", "l3", "--export", "transition_probe"])
            .args(["--crate-name", "tmk_probe", "--target", "kernel"])
            .arg("--out")
            .arg(&base),
        "Forge tamper-test baseline build",
    )?;
    run_checked(
        Command::new(&forge)
            .current_dir(&root)
            .arg("verify-build")
            .arg(&base),
        "Forge tamper-test baseline validation",
    )?;

    let append_cases = [
        ("raw-source", "evidence/input.th"),
        ("generated-source", "evidence/source.verus.rs"),
        ("certificate", "evidence/certificates.json"),
        (
            "translation-validation",
            "evidence/translation-validation.json",
        ),
        ("toolchain", "evidence/toolchain.json"),
        ("artifact", "artifact/libtmk_probe.rlib"),
    ];
    let mut passed = Vec::new();
    for (name, relative) in append_cases {
        let case = work.join(format!("case-{name}.verified"));
        copy_tree(&base, &case)?;
        append_tamper_byte(&case.join(relative))?;
        validate_tampered_bundle(
            &forge,
            &root,
            &case,
            name,
            "failed its length/digest check",
            &results,
        )?;
        fs::remove_dir_all(&case)
            .map_err(|error| format!("remove tamper case {}: {error}", case.display()))?;
        passed.push(name);
    }

    let receipt_case = work.join("case-receipt.verified");
    copy_tree(&base, &receipt_case)?;
    flip_first_byte(&receipt_case.join("receipt.json"))?;
    validate_tampered_bundle(
        &forge,
        &root,
        &receipt_case,
        "receipt",
        "invalid verified-build receipt",
        &results,
    )?;
    fs::remove_dir_all(&receipt_case)
        .map_err(|error| format!("remove tamper case {}: {error}", receipt_case.display()))?;
    passed.push("receipt");

    let missing_case = work.join("case-missing-file.verified");
    copy_tree(&base, &missing_case)?;
    fs::remove_file(missing_case.join("evidence/verus-result.json"))
        .map_err(|error| format!("remove tamper-test inventory member: {error}"))?;
    validate_tampered_bundle(
        &forge,
        &root,
        &missing_case,
        "missing-file",
        "bundle file inventory has missing, duplicate, or extra paths",
        &results,
    )?;
    fs::remove_dir_all(&missing_case)
        .map_err(|error| format!("remove tamper case {}: {error}", missing_case.display()))?;
    passed.push("missing-file");

    let extra_case = work.join("case-extra-file.verified");
    copy_tree(&base, &extra_case)?;
    fs::write(
        extra_case.join("unreceipted-object.o"),
        b"not allowlisted\n",
    )
    .map_err(|error| format!("create tamper-test extra inventory member: {error}"))?;
    validate_tampered_bundle(
        &forge,
        &root,
        &extra_case,
        "extra-file",
        "bundle file inventory has missing, duplicate, or extra paths",
        &results,
    )?;
    fs::remove_dir_all(&extra_case)
        .map_err(|error| format!("remove tamper case {}: {error}", extra_case.display()))?;
    passed.push("extra-file");

    fs::remove_dir_all(&base)
        .map_err(|error| format!("remove tamper-test baseline {}: {error}", base.display()))?;

    let report = format!(
        "M0_FORGE_TAMPER_OK\nrelease_eligible=false\nrejected_cases={}\ncases={}\n",
        passed.len(),
        passed.join(",")
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write Forge tamper report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn m0_verus_allocator() -> Result<(), String> {
    let root = workspace_root()?;
    let verus = PathBuf::from("/opt/verus/0.2026.05.24.ecee80a/verus");
    require_file(&verus, "Verus binary")?;
    let expected_verus_sha = "c5911ee43c7a92c49a48d2c8646c604d252a38c71c87bda88ad4d33eb9e7e0fc";
    let actual_verus_sha = sha256sum(&verus)?;
    if actual_verus_sha != expected_verus_sha {
        return Err(format!(
            "Verus digest is {actual_verus_sha}, expected {expected_verus_sha}"
        ));
    }

    let source = root.join("verus/platform/bounded_allocator.rs");
    require_file(&source, "bounded allocator Verus source")?;
    let source_sha = sha256sum(&source)?;
    let work = root.join("build/m0-allocator");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;
    let staged_source = work.join("tmk_allocator.rs");
    fs::copy(&source, &staged_source).map_err(|error| {
        format!(
            "stage {} as {}: {error}",
            source.display(),
            staged_source.display()
        )
    })?;
    if sha256sum(&staged_source)? != source_sha {
        return Err("staged allocator source digest differs from canonical source".to_string());
    }

    let verification = run_checked(
        &mut direct_verus_command(&verus, &work, "tmk_allocator.rs", true),
        "Verus bounded allocator proof and codegen",
    )?;
    require_output_fragments(
        &verification.stdout,
        "Verus bounded allocator proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 2",
            "\"errors\": 0",
            "\"version\": \"0.2026.05.24.ecee80a\"",
            "\"toolchain\": \"1.95.0-x86_64-unknown-linux-gnu\"",
        ],
    )?;
    fs::write(
        work.join("verus-result.json"),
        canonical_json(&verification.stdout, "allocator Verus result")?,
    )
    .map_err(|error| format!("write allocator Verus result: {error}"))?;
    if sha256sum(&staged_source)? != source_sha {
        return Err("allocator source changed during Verus proof/codegen".to_string());
    }

    let artifact = work.join("libtmk_allocator.rlib");
    require_file(&artifact, "compiled bounded allocator rlib")?;
    let artifact_sha = sha256sum(&artifact)?;

    for name in ["repro-a", "repro-b"] {
        let repro = work.join(name);
        fs::create_dir(&repro)
            .map_err(|error| format!("create allocator reproducibility path: {error}"))?;
        fs::copy(&source, repro.join("tmk_allocator.rs"))
            .map_err(|error| format!("stage allocator reproducibility source: {error}"))?;
        run_checked(
            &mut direct_verus_command(&verus, &repro, "tmk_allocator.rs", true),
            &format!("Verus allocator clean build in {name}"),
        )?;
        let repro_artifact = repro.join("libtmk_allocator.rlib");
        require_file(&repro_artifact, "reproducibility allocator rlib")?;
        let repro_sha = sha256sum(&repro_artifact)?;
        if repro_sha != artifact_sha {
            return Err(format!(
                "allocator build in {name} produced {repro_sha}, expected {artifact_sha}"
            ));
        }
        fs::remove_dir_all(&repro)
            .map_err(|error| format!("remove allocator reproducibility path: {error}"))?;
    }

    let undefined = run_checked(
        Command::new("nm").arg("-u").arg(&artifact),
        "bounded allocator undefined-symbol audit",
    )?;
    let undefined_text = String::from_utf8_lossy(&undefined.stdout);
    if undefined_text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("U ") || trimmed.contains(" U ")
    }) {
        return Err(format!(
            "bounded allocator rlib has undefined symbols:\n{undefined_text}"
        ));
    }
    fs::write(work.join("undefined-symbols.txt"), &undefined.stdout)
        .map_err(|error| format!("write allocator symbol audit: {error}"))?;

    let consumer = work.join("allocator-consumer");
    let rustc =
        PathBuf::from("/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc");
    run_checked(
        Command::new(&rustc)
            .current_dir(&root)
            .args(["--edition=2021"])
            .arg(root.join("tests/m0/allocator_consumer.rs"))
            .arg("--extern")
            .arg(format!("tmk_allocator={}", artifact.display()))
            .arg("-L")
            .arg("dependency=/opt/verus/0.2026.05.24.ecee80a")
            .args(["-C", "panic=abort"])
            .arg("-o")
            .arg(&consumer),
        "link bounded allocator host consumer",
    )?;
    let runtime = run_checked(
        Command::new(&consumer).current_dir(&root),
        "execute bounded allocator success/exhaustion cases",
    )?;
    require_output_fragments(
        &runtime.stdout,
        "bounded allocator runtime",
        &["M0_ALLOCATOR_OK:8:11:16"],
    )?;

    let canonical = fs::read_to_string(&source)
        .map_err(|error| format!("read {}: {error}", source.display()))?;
    let bad_update = canonical.replacen(
        "BumpState { next: state.next + units, end: state.end }",
        "BumpState { next: state.next, end: state.end }",
        1,
    );
    if bad_update == canonical {
        return Err("allocator bad-update mutation target was not found".to_string());
    }
    fs::write(work.join("bad-update.rs"), bad_update)
        .map_err(|error| format!("write bad allocator update mutation: {error}"))?;
    let bad_update_result = run_expect_failure(
        &mut direct_verus_command(&verus, &work, "bad-update.rs", false),
        "Verus rejects allocator state-update mutation",
    )?;
    let mut bad_update_diagnostic = Vec::new();
    bad_update_diagnostic.extend_from_slice(&bad_update_result.stdout);
    bad_update_diagnostic.extend_from_slice(&bad_update_result.stderr);
    require_output_fragments(
        &bad_update_diagnostic,
        "Verus allocator state-update rejection",
        &["postcondition not satisfied"],
    )?;
    write_combined_output(
        &work.join("bad-update-result.txt"),
        &bad_update_result,
        "bad allocator update mutation",
    )?;

    let bad_assume = canonical.replacen(
        "    if state.next <= state.end {",
        "    assume(false);\n    if state.next <= state.end {",
        1,
    );
    if bad_assume == canonical {
        return Err("allocator assume mutation target was not found".to_string());
    }
    fs::write(work.join("bad-assume.rs"), bad_assume)
        .map_err(|error| format!("write allocator assume mutation: {error}"))?;
    let bad_assume_result = run_expect_failure(
        &mut direct_verus_command(&verus, &work, "bad-assume.rs", false),
        "Verus no-cheating rejects allocator assume",
    )?;
    let mut assume_diagnostic = Vec::new();
    assume_diagnostic.extend_from_slice(&bad_assume_result.stdout);
    assume_diagnostic.extend_from_slice(&bad_assume_result.stderr);
    require_output_fragments(
        &assume_diagnostic,
        "Verus allocator assume rejection",
        &["assume"],
    )?;
    write_combined_output(
        &work.join("bad-assume-result.txt"),
        &bad_assume_result,
        "allocator assume mutation",
    )?;

    let verification_sha = sha256sum(&work.join("verus-result.json"))?;
    let consumer_sha = sha256sum(&consumer)?;
    let report = format!(
        "M0_VERUS_ALLOCATOR_OK\ncomponent_verified=true\nrelease_eligible=false\nsource_sha256={source_sha}\nartifact_sha256={artifact_sha}\nreproducibility_builds=3\nverus_result_sha256={verification_sha}\nconsumer_sha256={consumer_sha}\nruntime_marker=M0_ALLOCATOR_OK:8:11:16\nnegative_cases=bad-update,bad-assume\n"
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write allocator report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn m0_verus_byte_allocator() -> Result<(), String> {
    let root = workspace_root()?;
    let verus = PathBuf::from("/opt/verus/0.2026.05.24.ecee80a/verus");
    let rustc =
        PathBuf::from("/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc");
    let nm = PathBuf::from("/usr/sbin/nm");
    for (path, expected, label) in [
        (
            verus.as_path(),
            "c5911ee43c7a92c49a48d2c8646c604d252a38c71c87bda88ad4d33eb9e7e0fc",
            "Verus",
        ),
        (
            rustc.as_path(),
            "bff349e72704ff70bc08a234a3847338e797065bbedde5e556808bc87b7bf7c6",
            "Verus codegen rustc",
        ),
        (
            nm.as_path(),
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

    let source = root.join("verus/platform/byte_allocator.rs");
    require_file(&source, "byte/layout allocator Verus source")?;
    let canonical = fs::read_to_string(&source)
        .map_err(|error| format!("read {}: {error}", source.display()))?;
    for forbidden in [
        "assume(",
        "admit(",
        "axiom fn",
        "external_body",
        "unsafe",
        "asm!",
    ] {
        if canonical.contains(forbidden) {
            return Err(format!(
                "byte/layout allocator contains forbidden `{forbidden}`"
            ));
        }
    }
    let source_sha = sha256sum(&source)?;
    let work = root.join("build/m0-byte-allocator");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;
    let staged = work.join("tmk_byte_allocator.rs");
    fs::copy(&source, &staged)
        .map_err(|error| format!("stage byte/layout allocator source: {error}"))?;
    if sha256sum(&staged)? != source_sha {
        return Err(
            "staged byte/layout allocator source differs from canonical source".to_string(),
        );
    }

    let verification = run_checked(
        &mut direct_verus_command(&verus, &work, "tmk_byte_allocator.rs", true),
        "Verus byte/layout allocator proof and codegen",
    )?;
    require_output_fragments(
        &verification.stdout,
        "Verus byte/layout allocator proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 18",
            "\"errors\": 0",
            "\"version\": \"0.2026.05.24.ecee80a\"",
            "\"toolchain\": \"1.95.0-x86_64-unknown-linux-gnu\"",
        ],
    )?;
    fs::write(
        work.join("verus-result.json"),
        canonical_json(&verification.stdout, "byte/layout allocator Verus result")?,
    )
    .map_err(|error| format!("write byte/layout allocator Verus result: {error}"))?;
    if sha256sum(&staged)? != source_sha {
        return Err("byte/layout allocator source changed during proof/codegen".to_string());
    }

    let artifact = work.join("libtmk_byte_allocator.rlib");
    require_file(&artifact, "compiled byte/layout allocator rlib")?;
    let artifact_sha = sha256sum(&artifact)?;
    for name in ["repro-a", "repro-b"] {
        let repro = work.join(name);
        fs::create_dir(&repro)
            .map_err(|error| format!("create byte-allocator reproducibility path: {error}"))?;
        fs::copy(&source, repro.join("tmk_byte_allocator.rs"))
            .map_err(|error| format!("stage byte-allocator reproducibility source: {error}"))?;
        run_checked(
            &mut direct_verus_command(&verus, &repro, "tmk_byte_allocator.rs", true),
            &format!("Verus byte/layout allocator clean build in {name}"),
        )?;
        let repro_sha = sha256sum(&repro.join("libtmk_byte_allocator.rlib"))?;
        if repro_sha != artifact_sha {
            return Err(format!(
                "byte/layout allocator build in {name} produced {repro_sha}, expected {artifact_sha}"
            ));
        }
        fs::remove_dir_all(&repro)
            .map_err(|error| format!("remove byte-allocator reproducibility path: {error}"))?;
    }

    let undefined = run_checked(
        Command::new(&nm).arg("-u").arg(&artifact),
        "byte/layout allocator undefined-symbol audit",
    )?;
    let undefined_text = String::from_utf8_lossy(&undefined.stdout);
    if undefined_text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("U ") || trimmed.contains(" U ")
    }) {
        return Err(format!(
            "byte/layout allocator rlib has undefined symbols:\n{undefined_text}"
        ));
    }
    fs::write(work.join("undefined-symbols.txt"), &undefined.stdout)
        .map_err(|error| format!("write byte/layout allocator symbol audit: {error}"))?;

    let consumer = work.join("byte-allocator-consumer");
    run_checked(
        Command::new(&rustc)
            .current_dir(&root)
            .args(["--edition=2021"])
            .arg(root.join("tests/m0/byte_allocator_consumer.rs"))
            .arg("--extern")
            .arg(format!("tmk_byte_allocator={}", artifact.display()))
            .arg("-L")
            .arg("dependency=/opt/verus/0.2026.05.24.ecee80a")
            .args(["-C", "panic=abort"])
            .arg("-o")
            .arg(&consumer),
        "link byte/layout allocator host consumer",
    )?;
    let runtime = run_checked(
        Command::new(&consumer).current_dir(&root),
        "execute byte/layout allocator cases",
    )?;
    require_output_fragments(
        &runtime.stdout,
        "byte/layout allocator runtime",
        &["M0_BYTE_ALLOCATOR_OK:200008:200020:200030"],
    )?;

    let bad_alignment = canonical.replacen(
        "let address = cursor + padding;",
        "let address = cursor;",
        1,
    );
    if bad_alignment == canonical {
        return Err("byte allocator alignment mutation target was not found".to_string());
    }
    fs::write(work.join("bad-alignment.rs"), bad_alignment)
        .map_err(|error| format!("write byte allocator alignment mutation: {error}"))?;
    let bad_alignment_result = run_expect_failure(
        &mut direct_verus_command(&verus, &work, "bad-alignment.rs", false),
        "Verus rejects byte allocator alignment mutation",
    )?;
    let mut bad_alignment_diagnostic = Vec::new();
    bad_alignment_diagnostic.extend_from_slice(&bad_alignment_result.stdout);
    bad_alignment_diagnostic.extend_from_slice(&bad_alignment_result.stderr);
    require_output_fragments(
        &bad_alignment_diagnostic,
        "byte allocator alignment rejection",
        &["postcondition not satisfied"],
    )?;
    write_combined_output(
        &work.join("bad-alignment-result.txt"),
        &bad_alignment_result,
        "byte allocator alignment mutation",
    )?;

    let bad_exhaustion =
        canonical.replacen("if size > after_padding {", "if size >= after_padding {", 1);
    if bad_exhaustion == canonical {
        return Err("byte allocator exhaustion mutation target was not found".to_string());
    }
    fs::write(work.join("bad-exhaustion.rs"), bad_exhaustion)
        .map_err(|error| format!("write byte allocator exhaustion mutation: {error}"))?;
    let bad_exhaustion_result = run_expect_failure(
        &mut direct_verus_command(&verus, &work, "bad-exhaustion.rs", false),
        "Verus rejects byte allocator exhaustion mutation",
    )?;
    let mut bad_exhaustion_diagnostic = Vec::new();
    bad_exhaustion_diagnostic.extend_from_slice(&bad_exhaustion_result.stdout);
    bad_exhaustion_diagnostic.extend_from_slice(&bad_exhaustion_result.stderr);
    require_output_fragments(
        &bad_exhaustion_diagnostic,
        "byte allocator exhaustion rejection",
        &["postcondition not satisfied"],
    )?;
    write_combined_output(
        &work.join("bad-exhaustion-result.txt"),
        &bad_exhaustion_result,
        "byte allocator exhaustion mutation",
    )?;

    let bad_assume = canonical.replacen(
        "    if !(0 < state.base",
        "    assume(false);\n    if !(0 < state.base",
        1,
    );
    if bad_assume == canonical {
        return Err("byte allocator assume mutation target was not found".to_string());
    }
    fs::write(work.join("bad-assume.rs"), bad_assume)
        .map_err(|error| format!("write byte allocator assume mutation: {error}"))?;
    let bad_assume_result = run_expect_failure(
        &mut direct_verus_command(&verus, &work, "bad-assume.rs", false),
        "Verus no-cheating rejects byte allocator assume",
    )?;
    let mut bad_assume_diagnostic = Vec::new();
    bad_assume_diagnostic.extend_from_slice(&bad_assume_result.stdout);
    bad_assume_diagnostic.extend_from_slice(&bad_assume_result.stderr);
    require_output_fragments(
        &bad_assume_diagnostic,
        "byte allocator assume rejection",
        &["assume"],
    )?;
    write_combined_output(
        &work.join("bad-assume-result.txt"),
        &bad_assume_result,
        "byte allocator assume mutation",
    )?;

    let verification_sha = sha256sum(&work.join("verus-result.json"))?;
    let consumer_sha = sha256sum(&consumer)?;
    let report = format!(
        "M0_VERUS_BYTE_ALLOCATOR_OK\ncomponent_verified=true\nrelease_eligible=false\nsource_sha256={source_sha}\nartifact_sha256={artifact_sha}\nreproducibility_builds=3\nverus_result_sha256={verification_sha}\nconsumer_sha256={consumer_sha}\nruntime_marker=M0_BYTE_ALLOCATOR_OK:200008:200020:200030\nnegative_cases=bad-alignment,bad-exhaustion,bad-assume\n"
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write byte/layout allocator report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn m0_verus_capsule() -> Result<(), String> {
    let root = workspace_root()?;
    let verus = PathBuf::from("/opt/verus/0.2026.05.24.ecee80a/verus");
    let rustc =
        PathBuf::from("/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc");
    let ld = PathBuf::from("/usr/sbin/ld");
    let objcopy = PathBuf::from("/usr/sbin/objcopy");
    let objdump = PathBuf::from("/usr/sbin/objdump");
    let readelf = PathBuf::from("/usr/sbin/readelf");
    let nm = PathBuf::from("/usr/sbin/nm");
    for (path, expected, label) in [
        (
            verus.as_path(),
            "c5911ee43c7a92c49a48d2c8646c604d252a38c71c87bda88ad4d33eb9e7e0fc",
            "Verus",
        ),
        (
            ld.as_path(),
            "6cf122245638eb45fd981c75bf3a116675b3f9c7510ae2e3b386aa6738e46505",
            "GNU ld",
        ),
        (
            objcopy.as_path(),
            "8f09a5b2d8e2b34aebf269fffef2308492a022dcfedf87b49489592e838129b4",
            "GNU objcopy",
        ),
        (
            objdump.as_path(),
            "c7c3f8c5c0ed23b2330e148e58624f8d798f1673f4c9fb126ee81096f05e3653",
            "GNU objdump",
        ),
        (
            readelf.as_path(),
            "59d345f2a2b47f5617e8f53c72f6db5169c723c11d3e809a9e6e3c5673f2420c",
            "GNU readelf",
        ),
        (
            nm.as_path(),
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

    let source = root.join("verus/machine-model/hlt_register_capsule.rs");
    let linker_script = root.join("kernel-host/link/m0_capsule.ld");
    require_file(&source, "HLT/register capsule Verus source")?;
    require_file(&linker_script, "M0 capsule linker script")?;
    let source_sha = sha256sum(&source)?;
    let linker_sha = sha256sum(&linker_script)?;

    let work = root.join("build/m0-capsule");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;
    let staged = work.join("tmk_capsule.rs");
    fs::copy(&source, &staged).map_err(|error| format!("stage capsule Verus source: {error}"))?;
    if sha256sum(&staged)? != source_sha {
        return Err("staged capsule source differs from canonical source".to_string());
    }

    let verification = run_checked(
        &mut direct_verus_command(&verus, &work, "tmk_capsule.rs", true),
        "Verus HLT/register capsule proof and codegen",
    )?;
    require_output_fragments(
        &verification.stdout,
        "Verus HLT/register capsule proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 7",
            "\"errors\": 0",
            "\"version\": \"0.2026.05.24.ecee80a\"",
            "\"toolchain\": \"1.95.0-x86_64-unknown-linux-gnu\"",
        ],
    )?;
    fs::write(
        work.join("verus-result.json"),
        canonical_json(&verification.stdout, "capsule Verus result")?,
    )
    .map_err(|error| format!("write capsule Verus result: {error}"))?;
    if sha256sum(&staged)? != source_sha {
        return Err("capsule source changed during proof/codegen".to_string());
    }

    let artifact = work.join("libtmk_capsule.rlib");
    require_file(&artifact, "compiled capsule model rlib")?;
    let artifact_sha = sha256sum(&artifact)?;
    for name in ["repro-a", "repro-b"] {
        let repro = work.join(name);
        fs::create_dir(&repro)
            .map_err(|error| format!("create capsule reproducibility path: {error}"))?;
        fs::copy(&source, repro.join("tmk_capsule.rs"))
            .map_err(|error| format!("stage capsule reproducibility source: {error}"))?;
        run_checked(
            &mut direct_verus_command(&verus, &repro, "tmk_capsule.rs", true),
            &format!("Verus capsule clean build in {name}"),
        )?;
        let repro_sha = sha256sum(&repro.join("libtmk_capsule.rlib"))?;
        if repro_sha != artifact_sha {
            return Err(format!(
                "capsule model build in {name} produced {repro_sha}, expected {artifact_sha}"
            ));
        }
        fs::remove_dir_all(&repro)
            .map_err(|error| format!("remove capsule reproducibility path: {error}"))?;
    }

    let undefined = run_checked(
        Command::new(&nm).arg("-u").arg(&artifact),
        "capsule model undefined-symbol audit",
    )?;
    let undefined_text = String::from_utf8_lossy(&undefined.stdout);
    if undefined_text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("U ") || trimmed.contains(" U ")
    }) {
        return Err(format!(
            "capsule model rlib has undefined symbols:\n{undefined_text}"
        ));
    }

    let consumer = work.join("capsule-consumer");
    run_checked(
        Command::new(&rustc)
            .current_dir(&root)
            .args(["--edition=2021"])
            .arg(root.join("tests/m0/capsule_consumer.rs"))
            .arg("--extern")
            .arg(format!("tmk_capsule={}", artifact.display()))
            .arg("-L")
            .arg("dependency=/opt/verus/0.2026.05.24.ecee80a")
            .args(["-C", "panic=abort"])
            .arg("-o")
            .arg(&consumer),
        "link capsule model host consumer",
    )?;
    let capsule_bin = work.join("capsule.bin");
    let runtime = run_checked(
        Command::new(&consumer).current_dir(&root).arg(&capsule_bin),
        "execute capsule model and emit proved bytes",
    )?;
    require_output_fragments(
        &runtime.stdout,
        "capsule model runtime",
        &["M0_CAPSULE_OK:4889f8f4:5aa512348765cdef:1004"],
    )?;
    require_exact_bytes(&capsule_bin, &[0x48, 0x89, 0xf8, 0xf4], "emitted capsule")?;

    run_checked(
        Command::new(&ld).current_dir(&work).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "capsule-raw.o",
            "capsule.bin",
        ]),
        "wrap capsule bytes in relocatable object",
    )?;
    run_checked(
        Command::new(&objcopy).current_dir(&work).args([
            "--rename-section",
            ".data=.text.tmk_capsule,alloc,contents,load,readonly,code",
            "--redefine-sym",
            "_binary_capsule_bin_start=tmk_hlt_register_capsule",
            "--redefine-sym",
            "_binary_capsule_bin_end=tmk_hlt_register_capsule_end",
            "--redefine-sym",
            "_binary_capsule_bin_size=tmk_hlt_register_capsule_size",
            "capsule-raw.o",
            "capsule.o",
        ]),
        "name and classify capsule object section",
    )?;
    run_checked(
        Command::new(&ld)
            .current_dir(&work)
            .args([
                "-m",
                "elf_x86_64",
                "--build-id=none",
                "-nostdlib",
                "-static",
            ])
            .arg("-T")
            .arg(&linker_script)
            .args(["-o", "capsule.elf", "capsule.o"]),
        "link registered capsule ELF",
    )?;
    run_checked(
        Command::new(&objcopy).current_dir(&work).args([
            "--dump-section",
            ".text.tmk_capsule=linked-capsule.bin",
            "capsule.elf",
        ]),
        "extract linked capsule bytes",
    )?;
    let linked_bin = work.join("linked-capsule.bin");
    require_exact_bytes(&linked_bin, &[0x48, 0x89, 0xf8, 0xf4], "linked capsule")?;

    let relocations = run_checked(
        Command::new(&readelf)
            .current_dir(&work)
            .args(["-rW", "capsule.elf"]),
        "capsule relocation audit",
    )?;
    require_output_fragments(
        &relocations.stdout,
        "capsule relocation audit",
        &["There are no relocations in this file"],
    )?;
    fs::write(work.join("relocations.txt"), &relocations.stdout)
        .map_err(|error| format!("write capsule relocation evidence: {error}"))?;

    let sections = run_checked(
        Command::new(&readelf)
            .current_dir(&work)
            .args(["-SW", "capsule.elf"]),
        "capsule executable-section audit",
    )?;
    audit_executable_sections(&sections.stdout)?;
    fs::write(work.join("sections.txt"), &sections.stdout)
        .map_err(|error| format!("write capsule section evidence: {error}"))?;

    let symbols = run_checked(
        Command::new(&readelf)
            .current_dir(&work)
            .args(["-sW", "capsule.elf"]),
        "capsule symbol audit",
    )?;
    require_output_fragments(
        &symbols.stdout,
        "capsule symbol audit",
        &[
            "tmk_hlt_register_capsule_link_start",
            "tmk_hlt_register_capsule_link_end",
            "tmk_hlt_register_capsule",
        ],
    )?;
    fs::write(work.join("symbols.txt"), &symbols.stdout)
        .map_err(|error| format!("write capsule symbol evidence: {error}"))?;

    let disassembly = run_checked(
        Command::new(&objdump)
            .current_dir(&work)
            .args(["-d", "-Mintel", "capsule.elf"]),
        "capsule disassembly",
    )?;
    require_output_fragments(
        &disassembly.stdout,
        "capsule disassembly",
        &["mov    rax,rdi", "hlt"],
    )?;
    fs::write(work.join("disassembly.txt"), &disassembly.stdout)
        .map_err(|error| format!("write capsule disassembly: {error}"))?;

    let mut mutated_bytes = vec![0x48, 0x89, 0xf8, 0xf4];
    mutated_bytes[0] ^= 1;
    fs::write(work.join("mutated.bin"), mutated_bytes)
        .map_err(|error| format!("write mutated capsule bytes: {error}"))?;
    run_checked(
        Command::new(&ld).current_dir(&work).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "mutated-raw.o",
            "mutated.bin",
        ]),
        "wrap mutated capsule bytes",
    )?;
    run_checked(
        Command::new(&objcopy).current_dir(&work).args([
            "--rename-section",
            ".data=.text.tmk_capsule,alloc,contents,load,readonly,code",
            "--redefine-sym",
            "_binary_mutated_bin_start=tmk_hlt_register_capsule",
            "--redefine-sym",
            "_binary_mutated_bin_end=tmk_hlt_register_capsule_end",
            "--redefine-sym",
            "_binary_mutated_bin_size=tmk_hlt_register_capsule_size",
            "mutated-raw.o",
            "mutated.o",
        ]),
        "name mutated capsule object",
    )?;
    run_checked(
        Command::new(&ld)
            .current_dir(&work)
            .args([
                "-m",
                "elf_x86_64",
                "--build-id=none",
                "-nostdlib",
                "-static",
            ])
            .arg("-T")
            .arg(&linker_script)
            .args(["-o", "mutated.elf", "mutated.o"]),
        "link mutated capsule ELF",
    )?;
    run_checked(
        Command::new(&objcopy).current_dir(&work).args([
            "--dump-section",
            ".text.tmk_capsule=mutated-linked.bin",
            "mutated.elf",
        ]),
        "extract mutated linked capsule",
    )?;
    let byte_mutation_diagnostic = match require_exact_bytes(
        &work.join("mutated-linked.bin"),
        &[0x48, 0x89, 0xf8, 0xf4],
        "mutated linked capsule",
    ) {
        Ok(()) => return Err("capsule byte mutation passed the post-link audit".to_string()),
        Err(diagnostic) => diagnostic,
    };
    fs::write(
        work.join("byte-mutation-result.txt"),
        format!("{byte_mutation_diagnostic}\n"),
    )
    .map_err(|error| format!("write capsule byte-mutation evidence: {error}"))?;

    fs::write(work.join("extra.bin"), [0xf4])
        .map_err(|error| format!("write unregistered executable byte: {error}"))?;
    run_checked(
        Command::new(&ld).current_dir(&work).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "extra-raw.o",
            "extra.bin",
        ]),
        "wrap unregistered executable byte",
    )?;
    run_checked(
        Command::new(&objcopy).current_dir(&work).args([
            "--rename-section",
            ".data=.text.unregistered,alloc,contents,load,readonly,code",
            "extra-raw.o",
            "extra.o",
        ]),
        "classify unregistered executable section",
    )?;
    run_checked(
        Command::new(&ld)
            .current_dir(&work)
            .args([
                "-m",
                "elf_x86_64",
                "--build-id=none",
                "-nostdlib",
                "-static",
            ])
            .arg("-T")
            .arg(&linker_script)
            .args(["-o", "extra.elf", "capsule.o", "extra.o"]),
        "link ELF with unregistered executable section",
    )?;
    let extra_sections = run_checked(
        Command::new(&readelf)
            .current_dir(&work)
            .args(["-SW", "extra.elf"]),
        "inspect unregistered executable section",
    )?;
    let extra_diagnostic = match audit_executable_sections(&extra_sections.stdout) {
        Ok(()) => {
            return Err("unregistered executable section passed the post-link audit".to_string());
        }
        Err(diagnostic) => diagnostic,
    };
    fs::write(
        work.join("unregistered-section-result.txt"),
        format!("{extra_diagnostic}\n"),
    )
    .map_err(|error| format!("write unregistered-section evidence: {error}"))?;

    let canonical = fs::read_to_string(&source)
        .map_err(|error| format!("read {}: {error}", source.display()))?;
    let bad_semantics = canonical.replacen("rax: state.rdi,", "rax: state.rax,", 1);
    if bad_semantics == canonical {
        return Err("capsule semantic mutation target was not found".to_string());
    }
    fs::write(work.join("bad-semantics.rs"), bad_semantics)
        .map_err(|error| format!("write capsule semantic mutation: {error}"))?;
    let bad_semantics_result = run_expect_failure(
        &mut direct_verus_command(&verus, &work, "bad-semantics.rs", false),
        "Verus rejects capsule semantic mutation",
    )?;
    let mut bad_semantics_diagnostic = Vec::new();
    bad_semantics_diagnostic.extend_from_slice(&bad_semantics_result.stdout);
    bad_semantics_diagnostic.extend_from_slice(&bad_semantics_result.stderr);
    require_output_fragments(
        &bad_semantics_diagnostic,
        "Verus capsule semantic rejection",
        &["postcondition not satisfied"],
    )?;
    write_combined_output(
        &work.join("bad-semantics-result.txt"),
        &bad_semantics_result,
        "capsule semantic mutation",
    )?;

    let bad_assume = canonical.replacen(
        "    if word == 0xf4f88948u32",
        "    assume(false);\n    if word == 0xf4f88948u32",
        1,
    );
    if bad_assume == canonical {
        return Err("capsule assume mutation target was not found".to_string());
    }
    fs::write(work.join("bad-assume.rs"), bad_assume)
        .map_err(|error| format!("write capsule assume mutation: {error}"))?;
    let bad_assume_result = run_expect_failure(
        &mut direct_verus_command(&verus, &work, "bad-assume.rs", false),
        "Verus no-cheating rejects capsule assume",
    )?;
    let mut bad_assume_diagnostic = Vec::new();
    bad_assume_diagnostic.extend_from_slice(&bad_assume_result.stdout);
    bad_assume_diagnostic.extend_from_slice(&bad_assume_result.stderr);
    require_output_fragments(
        &bad_assume_diagnostic,
        "Verus capsule assume rejection",
        &["assume/admit not allowed with --no-cheating"],
    )?;
    write_combined_output(
        &work.join("bad-assume-result.txt"),
        &bad_assume_result,
        "capsule assume mutation",
    )?;

    let linked_sha = sha256sum(&linked_bin)?;
    let elf_sha = sha256sum(&work.join("capsule.elf"))?;
    let verification_sha = sha256sum(&work.join("verus-result.json"))?;
    let consumer_sha = sha256sum(&consumer)?;
    let report = format!(
        "M0_VERUS_CAPSULE_OK\ncomponent_verified=true\nrelease_eligible=false\nsource_sha256={source_sha}\nlinker_script_sha256={linker_sha}\nmodel_artifact_sha256={artifact_sha}\nreproducibility_builds=3\nverus_result_sha256={verification_sha}\nconsumer_sha256={consumer_sha}\nlinked_capsule_sha256={linked_sha}\nlinked_elf_sha256={elf_sha}\nruntime_marker=M0_CAPSULE_OK:4889f8f4:5aa512348765cdef:1004\nnegative_cases=byte-mutation,unregistered-executable,bad-semantics,bad-assume\n"
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write capsule report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn m0_host_link() -> Result<(), String> {
    let root = workspace_root()?;
    let verus = PathBuf::from("/opt/verus/0.2026.05.24.ecee80a/verus");
    let ld = PathBuf::from("/usr/sbin/ld");
    let objcopy = PathBuf::from("/usr/sbin/objcopy");
    let objdump = PathBuf::from("/usr/sbin/objdump");
    let readelf = PathBuf::from("/usr/sbin/readelf");
    let nm = PathBuf::from("/usr/sbin/nm");
    for (path, expected, label) in [
        (
            verus.as_path(),
            "c5911ee43c7a92c49a48d2c8646c604d252a38c71c87bda88ad4d33eb9e7e0fc",
            "Verus",
        ),
        (
            ld.as_path(),
            "6cf122245638eb45fd981c75bf3a116675b3f9c7510ae2e3b386aa6738e46505",
            "GNU ld",
        ),
        (
            objcopy.as_path(),
            "8f09a5b2d8e2b34aebf269fffef2308492a022dcfedf87b49489592e838129b4",
            "GNU objcopy",
        ),
        (
            objdump.as_path(),
            "c7c3f8c5c0ed23b2330e148e58624f8d798f1673f4c9fb126ee81096f05e3653",
            "GNU objdump",
        ),
        (
            readelf.as_path(),
            "59d345f2a2b47f5617e8f53c72f6db5169c723c11d3e809a9e6e3c5673f2420c",
            "GNU readelf",
        ),
        (
            nm.as_path(),
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

    let allocator = root.join("build/m0-allocator/libtmk_allocator.rlib");
    let allocator_report = root.join("build/m0-allocator/report.txt");
    let byte_allocator = root.join("build/m0-byte-allocator/libtmk_byte_allocator.rlib");
    let byte_allocator_report = root.join("build/m0-byte-allocator/report.txt");
    let capsule_object = root.join("build/m0-capsule/capsule.o");
    let capsule_report = root.join("build/m0-capsule/report.txt");
    for (path, label) in [
        (&allocator, "verified allocator rlib"),
        (&allocator_report, "allocator report"),
        (&byte_allocator, "verified byte/layout allocator rlib"),
        (&byte_allocator_report, "byte/layout allocator report"),
        (&capsule_object, "registered capsule object"),
        (&capsule_report, "capsule report"),
    ] {
        require_file(path, label)?;
    }
    let allocator_sha = sha256sum(&allocator)?;
    let allocator_report_text = fs::read_to_string(&allocator_report)
        .map_err(|error| format!("read {}: {error}", allocator_report.display()))?;
    let allocator_binding = format!("artifact_sha256={allocator_sha}");
    for required in ["component_verified=true", allocator_binding.as_str()] {
        if !allocator_report_text.contains(required) {
            return Err(format!("allocator report is missing `{required}`"));
        }
    }
    let byte_allocator_sha = sha256sum(&byte_allocator)?;
    let byte_allocator_report_text = fs::read_to_string(&byte_allocator_report)
        .map_err(|error| format!("read {}: {error}", byte_allocator_report.display()))?;
    let byte_allocator_binding = format!("artifact_sha256={byte_allocator_sha}");
    for required in ["component_verified=true", byte_allocator_binding.as_str()] {
        if !byte_allocator_report_text.contains(required) {
            return Err(format!(
                "byte/layout allocator report is missing `{required}`"
            ));
        }
    }
    let capsule_report_text = fs::read_to_string(&capsule_report)
        .map_err(|error| format!("read {}: {error}", capsule_report.display()))?;
    for required in [
        "component_verified=true",
        "linked_capsule_sha256=86f039964fb227ba98078e671367c11641ed25204ea080f1b5b30bd13c5deda8",
    ] {
        if !capsule_report_text.contains(required) {
            return Err(format!("capsule report is missing `{required}`"));
        }
    }

    let source = root.join("verus/platform/panic_host.rs");
    let linker_script = root.join("kernel-host/link/m0_host.ld");
    require_file(&source, "direct-Verus panic host source")?;
    require_file(&linker_script, "M0 host linker script")?;
    let source_sha = sha256sum(&source)?;
    let linker_sha = sha256sum(&linker_script)?;
    let canonical = fs::read_to_string(&source)
        .map_err(|error| format!("read {}: {error}", source.display()))?;
    audit_panic_host_source(&canonical)?;

    let work = root.join("build/m0-host");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;
    fs::copy(&source, work.join("tmk_panic_host.rs"))
        .map_err(|error| format!("stage panic host source: {error}"))?;
    fs::copy(&allocator, work.join("libtmk_allocator.rlib"))
        .map_err(|error| format!("stage allocator rlib: {error}"))?;
    fs::copy(&byte_allocator, work.join("libtmk_byte_allocator.rlib"))
        .map_err(|error| format!("stage byte/layout allocator rlib: {error}"))?;
    fs::copy(&capsule_object, work.join("capsule.o"))
        .map_err(|error| format!("stage capsule object: {error}"))?;

    let verification = run_checked(
        &mut direct_verus_command(&verus, &work, "tmk_panic_host.rs", true),
        "Verus panic lang-item proof and codegen",
    )?;
    require_output_fragments(
        &verification.stdout,
        "Verus panic lang-item proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 2",
            "\"errors\": 0",
            "\"version\": \"0.2026.05.24.ecee80a\"",
            "\"toolchain\": \"1.95.0-x86_64-unknown-linux-gnu\"",
        ],
    )?;
    fs::write(
        work.join("verus-result.json"),
        canonical_json(&verification.stdout, "panic host Verus result")?,
    )
    .map_err(|error| format!("write panic host Verus result: {error}"))?;
    if sha256sum(&work.join("tmk_panic_host.rs"))? != source_sha {
        return Err("panic host source changed during proof/codegen".to_string());
    }

    let panic_rlib = work.join("libtmk_panic_host.rlib");
    require_file(&panic_rlib, "compiled panic host rlib")?;
    let panic_sha = sha256sum(&panic_rlib)?;
    for name in ["proof-repro-a", "proof-repro-b"] {
        let repro = work.join(name);
        fs::create_dir(&repro)
            .map_err(|error| format!("create panic-host proof reproducibility path: {error}"))?;
        fs::copy(&source, repro.join("tmk_panic_host.rs"))
            .map_err(|error| format!("stage panic-host reproducibility source: {error}"))?;
        run_checked(
            &mut direct_verus_command(&verus, &repro, "tmk_panic_host.rs", true),
            &format!("Verus panic-host clean build in {name}"),
        )?;
        let repro_sha = sha256sum(&repro.join("libtmk_panic_host.rlib"))?;
        if repro_sha != panic_sha {
            return Err(format!(
                "panic host build in {name} produced {repro_sha}, expected {panic_sha}"
            ));
        }
        fs::remove_dir_all(&repro)
            .map_err(|error| format!("remove panic-host reproducibility path: {error}"))?;
    }

    let panic_symbols = run_checked(
        Command::new(&nm)
            .args(["-g", "--defined-only"])
            .arg(&panic_rlib),
        "find verified panic lang-item symbol",
    )?;
    let symbol_text = String::from_utf8_lossy(&panic_symbols.stdout);
    let entry_symbols: Vec<_> = symbol_text
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .filter(|symbol| symbol.contains("rust_begin_unwind"))
        .collect();
    if entry_symbols.len() != 1 {
        return Err(format!(
            "expected one panic lang-item symbol, found {entry_symbols:?}"
        ));
    }
    let entry = entry_symbols[0];

    let undefined_panic = run_checked(
        Command::new(&nm).arg("-u").arg(&panic_rlib),
        "panic host undefined-symbol audit",
    )?;
    if String::from_utf8_lossy(&undefined_panic.stdout)
        .lines()
        .any(|line| line.trim_start().starts_with("U "))
    {
        return Err("panic host rlib contains undefined symbols".to_string());
    }

    let host_elf = work.join("host.elf");
    run_checked(
        &mut m0_host_link_command(&ld, &work, &linker_script, entry, "host.elf"),
        "link verified M0 host ELF",
    )?;
    require_file(&host_elf, "linked M0 host ELF")?;

    let header = run_checked(
        Command::new(&readelf)
            .current_dir(&work)
            .args(["-hW", "host.elf"]),
        "M0 host ELF header audit",
    )?;
    require_output_fragments(
        &header.stdout,
        "M0 host ELF header audit",
        &[
            "Type:                              EXEC",
            "Machine:                           Advanced Micro Devices X86-64",
        ],
    )?;

    let sections = run_checked(
        Command::new(&readelf)
            .current_dir(&work)
            .args(["-SW", "host.elf"]),
        "M0 host section audit",
    )?;
    audit_host_sections(&sections.stdout)?;
    fs::write(work.join("sections.txt"), &sections.stdout)
        .map_err(|error| format!("write M0 host section evidence: {error}"))?;

    let segments = run_checked(
        Command::new(&readelf)
            .current_dir(&work)
            .args(["-lW", "host.elf"]),
        "M0 host segment audit",
    )?;
    audit_host_segments(&segments.stdout)?;
    fs::write(work.join("segments.txt"), &segments.stdout)
        .map_err(|error| format!("write M0 host segment evidence: {error}"))?;

    let relocations = run_checked(
        Command::new(&readelf)
            .current_dir(&work)
            .args(["-rW", "host.elf"]),
        "M0 host relocation audit",
    )?;
    require_output_fragments(
        &relocations.stdout,
        "M0 host relocation audit",
        &["There are no relocations in this file"],
    )?;
    let dynamic = run_checked(
        Command::new(&readelf)
            .current_dir(&work)
            .args(["-dW", "host.elf"]),
        "M0 host dynamic-dependency audit",
    )?;
    require_output_fragments(
        &dynamic.stdout,
        "M0 host dynamic-dependency audit",
        &["There is no dynamic section in this file"],
    )?;
    let undefined_host = run_checked(
        Command::new(&nm).arg("-u").arg(&host_elf),
        "M0 host unresolved-symbol audit",
    )?;
    if !undefined_host.stdout.is_empty() {
        return Err(format!(
            "M0 host has unresolved symbols:\n{}",
            String::from_utf8_lossy(&undefined_host.stdout)
        ));
    }

    run_checked(
        Command::new(&objcopy).current_dir(&work).args([
            "--dump-section",
            ".text.tmk_capsule=host-linked-capsule.bin",
            "host.elf",
        ]),
        "extract M0 host linked capsule",
    )?;
    require_exact_bytes(
        &work.join("host-linked-capsule.bin"),
        &[0x48, 0x89, 0xf8, 0xf4],
        "M0 host linked capsule",
    )?;

    let disassembly = run_checked(
        Command::new(&objdump)
            .current_dir(&work)
            .args(["-d", "-Mintel", "host.elf"]),
        "M0 host disassembly audit",
    )?;
    require_output_fragments(
        &disassembly.stdout,
        "M0 host disassembly audit",
        &[
            "<tmk_hlt_register_capsule>",
            "mov    rax,rdi",
            "hlt",
            "allocate_pair",
            "allocate_two_layouts",
            "rust_begin_unwind",
        ],
    )?;
    fs::write(work.join("disassembly.txt"), &disassembly.stdout)
        .map_err(|error| format!("write M0 host disassembly: {error}"))?;

    let execution = run_expect_failure(
        Command::new("timeout")
            .current_dir(&work)
            .args(["0.1s", "./host.elf"]),
        "execute fail-stop panic entry",
    )?;
    if execution.status.code() != Some(124) {
        return Err(format!(
            "fail-stop host exited with {}, expected timeout status 124",
            execution.status
        ));
    }

    let host_sha = sha256sum(&host_elf)?;
    for name in ["link-repro-a", "link-repro-b"] {
        let repro = work.join(name);
        fs::create_dir(&repro)
            .map_err(|error| format!("create host link reproducibility path: {error}"))?;
        fs::copy(&allocator, repro.join("libtmk_allocator.rlib"))
            .map_err(|error| format!("stage allocator in host repro path: {error}"))?;
        fs::copy(&byte_allocator, repro.join("libtmk_byte_allocator.rlib"))
            .map_err(|error| format!("stage byte/layout allocator in host repro path: {error}"))?;
        fs::copy(&panic_rlib, repro.join("libtmk_panic_host.rlib"))
            .map_err(|error| format!("stage panic host in repro path: {error}"))?;
        fs::copy(&capsule_object, repro.join("capsule.o"))
            .map_err(|error| format!("stage capsule in host repro path: {error}"))?;
        run_checked(
            &mut m0_host_link_command(&ld, &repro, &linker_script, entry, "host.elf"),
            &format!("link M0 host in {name}"),
        )?;
        let repro_sha = sha256sum(&repro.join("host.elf"))?;
        if repro_sha != host_sha {
            return Err(format!(
                "host link in {name} produced {repro_sha}, expected {host_sha}"
            ));
        }
        fs::remove_dir_all(&repro)
            .map_err(|error| format!("remove host link reproducibility path: {error}"))?;
    }

    let missing_divergence =
        canonical.replace("#[verifier::exec_allows_no_decreases_clause]\n", "");
    if missing_divergence == canonical {
        return Err("panic-host divergence mutation target was not found".to_string());
    }
    fs::write(work.join("missing-divergence.rs"), missing_divergence)
        .map_err(|error| format!("write panic-host divergence mutation: {error}"))?;
    let missing_divergence_result = run_expect_failure(
        &mut direct_verus_command(&verus, &work, "missing-divergence.rs", false),
        "Verus rejects unclassified panic divergence",
    )?;
    let mut missing_divergence_diagnostic = Vec::new();
    missing_divergence_diagnostic.extend_from_slice(&missing_divergence_result.stdout);
    missing_divergence_diagnostic.extend_from_slice(&missing_divergence_result.stderr);
    require_output_fragments(
        &missing_divergence_diagnostic,
        "panic divergence rejection",
        &["loop must have a decreases clause"],
    )?;
    write_combined_output(
        &work.join("missing-divergence-result.txt"),
        &missing_divergence_result,
        "missing panic divergence classification",
    )?;

    let external_panic = canonical.replacen(
        "#[panic_handler]\n",
        "#[panic_handler]\n#[verifier::external_body]\n",
        1,
    );
    if external_panic == canonical {
        return Err("panic-host external-body mutation target was not found".to_string());
    }
    fs::write(work.join("external-panic.rs"), external_panic)
        .map_err(|error| format!("write external panic mutation: {error}"))?;
    let external_panic_result = run_expect_failure(
        &mut direct_verus_command(&verus, &work, "external-panic.rs", false),
        "Verus no-cheating rejects external panic body",
    )?;
    let mut external_panic_diagnostic = Vec::new();
    external_panic_diagnostic.extend_from_slice(&external_panic_result.stdout);
    external_panic_diagnostic.extend_from_slice(&external_panic_result.stderr);
    require_output_fragments(
        &external_panic_diagnostic,
        "external panic body rejection",
        &["external_body"],
    )?;
    write_combined_output(
        &work.join("external-panic-result.txt"),
        &external_panic_result,
        "external panic body",
    )?;

    fs::write(work.join("writable.bin"), [1u8])
        .map_err(|error| format!("write forbidden writable input: {error}"))?;
    run_checked(
        Command::new(&ld).current_dir(&work).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "writable-raw.o",
            "writable.bin",
        ]),
        "wrap forbidden writable input",
    )?;
    run_checked(
        Command::new(&objcopy).current_dir(&work).args([
            "--rename-section",
            ".data=.data,alloc,contents,load,data",
            "writable-raw.o",
            "writable.o",
        ]),
        "classify forbidden writable input",
    )?;
    let writable_link = run_expect_failure(
        &mut m0_host_link_command_with_extra(
            &ld,
            &work,
            &linker_script,
            entry,
            "writable-host.elf",
            "writable.o",
        ),
        "M0 host linker rejects writable data",
    )?;
    let mut writable_diagnostic = Vec::new();
    writable_diagnostic.extend_from_slice(&writable_link.stdout);
    writable_diagnostic.extend_from_slice(&writable_link.stderr);
    require_output_fragments(
        &writable_diagnostic,
        "M0 host writable-data rejection",
        &["M0 host must not contain initialized writable data"],
    )?;
    write_combined_output(
        &work.join("writable-data-result.txt"),
        &writable_link,
        "writable host data",
    )?;

    let panic_result_sha = sha256sum(&work.join("verus-result.json"))?;
    let capsule_object_sha = sha256sum(&work.join("capsule.o"))?;
    let report = format!(
        "M0_HOST_LINK_OK\ncomponent_verified=true\nrelease_eligible=false\npanic_source_sha256={source_sha}\npanic_artifact_sha256={panic_sha}\npanic_verus_result_sha256={panic_result_sha}\nallocator_artifact_sha256={allocator_sha}\nbyte_allocator_artifact_sha256={byte_allocator_sha}\ncapsule_object_sha256={capsule_object_sha}\nlinker_script_sha256={linker_sha}\nhost_elf_sha256={host_sha}\nproof_reproducibility_builds=3\nlink_reproducibility_builds=3\nruntime_observation=fail-stop-timeout-124\nnegative_cases=missing-divergence,external-panic-body,writable-data\n"
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M0 host report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn audit_panic_host_source(source: &str) -> Result<(), String> {
    let opaque_type = "#[verifier::external_type_specification]\n#[verifier::external_body]\npub struct ExPanicInfo";
    if source
        .matches("#[verifier::external_type_specification]")
        .count()
        != 1
        || source.matches("#[verifier::external_body]").count() != 1
        || source.matches("pub struct ExPanicInfo").count() != 1
        || !source.contains(opaque_type)
    {
        return Err(
            "panic host permits exactly one opaque PanicInfo type specification and paired external_body"
                .to_string(),
        );
    }
    if source
        .matches("#[verifier::exec_allows_no_decreases_clause]")
        .count()
        != 1
    {
        return Err("panic host requires exactly one explicit divergence allowance".to_string());
    }
    for forbidden in [
        "assume(",
        "admit(",
        "axiom fn",
        "unsafe",
        "asm!",
        "impl ExPanicInfo",
        "impl<'",
    ] {
        if source.contains(forbidden) {
            return Err(format!(
                "panic host source contains forbidden `{forbidden}`"
            ));
        }
    }
    Ok(())
}

fn m0_host_link_command(
    ld: &Path,
    work: &Path,
    linker_script: &Path,
    entry: &str,
    output: &str,
) -> Command {
    let mut command = Command::new(ld);
    command
        .current_dir(work)
        .args([
            "-m",
            "elf_x86_64",
            "--build-id=none",
            "-nostdlib",
            "-static",
        ])
        .arg("-T")
        .arg(linker_script)
        .arg("-e")
        .arg(entry)
        .arg("-o")
        .arg(output)
        .arg("--whole-archive")
        .args([
            "libtmk_allocator.rlib",
            "libtmk_byte_allocator.rlib",
            "libtmk_panic_host.rlib",
        ])
        .arg("--no-whole-archive")
        .arg("capsule.o");
    command
}

fn m0_host_link_command_with_extra(
    ld: &Path,
    work: &Path,
    linker_script: &Path,
    entry: &str,
    output: &str,
    extra: &str,
) -> Command {
    let mut command = m0_host_link_command(ld, work, linker_script, entry, output);
    command.arg(extra);
    command
}

fn audit_host_sections(readelf_output: &[u8]) -> Result<(), String> {
    let output = String::from_utf8_lossy(readelf_output);
    let executable: Vec<_> = output
        .lines()
        .filter(|line| line.contains(" AX "))
        .collect();
    if executable.len() != 2
        || !executable
            .iter()
            .any(|line| line.contains(".text.tmk_capsule"))
        || !executable.iter().any(|line| line.contains(".text.host"))
    {
        return Err(format!(
            "M0 host executable-section mismatch: {executable:?}"
        ));
    }
    for forbidden in [
        ".eh_frame",
        ".gcc_except_table",
        ".got",
        ".got.plt",
        ".data",
        ".bss",
        ".rodata",
    ] {
        if output.lines().any(|line| line.contains(forbidden)) {
            return Err(format!("M0 host contains forbidden section `{forbidden}`"));
        }
    }
    Ok(())
}

fn audit_host_segments(readelf_output: &[u8]) -> Result<(), String> {
    let output = String::from_utf8_lossy(readelf_output);
    let loads: Vec<_> = output
        .lines()
        .filter(|line| line.trim_start().starts_with("LOAD"))
        .collect();
    if loads.len() != 1 || !loads[0].contains(" R E ") || loads[0].contains(" W ") {
        return Err(format!("M0 host segment policy mismatch: {loads:?}"));
    }
    Ok(())
}

fn require_exact_bytes(path: &Path, expected: &[u8], label: &str) -> Result<(), String> {
    let actual =
        fs::read(path).map_err(|error| format!("read {label} {}: {error}", path.display()))?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{label} byte mismatch: actual={actual:02x?} expected={expected:02x?}"
        ))
    }
}

fn audit_executable_sections(readelf_output: &[u8]) -> Result<(), String> {
    let output = String::from_utf8_lossy(readelf_output);
    let executable: Vec<_> = output
        .lines()
        .filter(|line| line.contains(" AX "))
        .collect();
    if executable.len() == 1 && executable[0].contains(".text.tmk_capsule") {
        Ok(())
    } else {
        Err(format!(
            "executable section allowlist mismatch: expected only .text.tmk_capsule, found {executable:?}"
        ))
    }
}

fn canonical_json(bytes: &[u8], label: &str) -> Result<Vec<u8>, String> {
    fn sort(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(object) => {
                let mut entries: Vec<_> = object.into_iter().collect();
                entries.sort_by(|left, right| left.0.cmp(&right.0));
                let mut sorted = serde_json::Map::new();
                for (key, value) in entries {
                    sorted.insert(key, sort(value));
                }
                serde_json::Value::Object(sorted)
            }
            serde_json::Value::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(sort).collect())
            }
            other => other,
        }
    }

    let parsed: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| format!("parse {label} JSON: {error}"))?;
    let mut canonical = serde_json::to_vec_pretty(&sort(parsed))
        .map_err(|error| format!("serialize canonical {label}: {error}"))?;
    canonical.push(b'\n');
    Ok(canonical)
}

fn direct_verus_command(verus: &Path, work: &Path, source_name: &str, compile: bool) -> Command {
    direct_verus_command_with_rlimit(verus, work, source_name, compile, "20")
}

fn direct_verus_command_with_rlimit(
    verus: &Path,
    work: &Path,
    source_name: &str,
    compile: bool,
    rlimit: &str,
) -> Command {
    let mut command = Command::new(verus);
    command
        .current_dir(work)
        .env("SOURCE_DATE_EPOCH", "0")
        .args(["--output-json", "--no-vstd", "--no-cheating"]);
    if compile {
        command.arg("--compile");
    }
    command
        .args(["--rlimit", rlimit])
        .args(["--smt-option", "smt.random_seed=1"])
        .args(["-C", "panic=abort"])
        .args(["-C", "overflow-checks=off"])
        .args(["-C", "relocation-model=static"])
        .args(["-C", "no-redzone=yes"])
        .arg(format!("--remap-path-prefix={}=.", work.display()))
        .arg(source_name);
    command
}

fn compile_consumer(
    rustc: &Path,
    root: &Path,
    source: &Path,
    artifact: &Path,
    deps: &Path,
    output: &Path,
    no_std: bool,
) -> Result<(), String> {
    let mut command = consumer_command(rustc, root, source, artifact, deps, output, no_std);
    run_checked(
        &mut command,
        if no_std {
            "link separate no_std Forge consumer"
        } else {
            "link executable Forge consumer"
        },
    )?;
    Ok(())
}

fn consumer_command(
    rustc: &Path,
    root: &Path,
    source: &Path,
    artifact: &Path,
    deps: &Path,
    output: &Path,
    no_std: bool,
) -> Command {
    let mut command = Command::new(rustc);
    command
        .current_dir(root)
        .args(["--edition=2021"])
        .arg(source)
        .arg("--extern")
        .arg(format!("tmk_probe={}", artifact.display()))
        .arg("-L")
        .arg(format!("dependency={}", deps.display()))
        .args(["-C", "panic=abort"]);
    if no_std {
        command.args(["-C", "link-arg=-nostartfiles"]);
    }
    command.arg("-o").arg(output);
    command
}

fn report_field<'a>(report: &'a str, key: &str) -> Result<&'a str, String> {
    let prefix = format!("{key}=");
    let mut values = report.lines().filter_map(|line| line.strip_prefix(&prefix));
    let value = values
        .next()
        .ok_or_else(|| format!("report is missing field `{key}`"))?;
    if values.next().is_some() {
        return Err(format!("report repeats field `{key}`"));
    }
    Ok(value)
}

fn json_string<'a>(
    value: &'a serde_json::Value,
    pointer: &str,
    label: &str,
) -> Result<&'a str, String> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{label} is missing or is not a string at `{pointer}`"))
}

fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask manifest has no workspace parent".to_string())
}

fn forge_binary() -> Result<PathBuf, String> {
    let forge = env::var_os("FORGE_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/doll/Thermite/target/debug/forge"));
    require_file(&forge, "Forge binary")?;
    Ok(forge)
}

fn check_forge_skill(forge: &Path) -> Result<(), String> {
    let skill = env::var_os("THERMITE_SKILL_REFERENCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from("/home/doll/.codex/skills/thermite/references/language.md")
        });
    require_file(&skill, "Thermite skill reference")?;
    run_checked(
        Command::new(forge).args([
            OsStr::new("skill"),
            OsStr::new("--check"),
            skill.as_os_str(),
        ]),
        "Forge skill freshness check",
    )?;
    Ok(())
}

fn require_file(path: &Path, label: &str) -> Result<(), String> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("{label} does not exist at {}", path.display()))
    }
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir(destination)
        .map_err(|error| format!("create copy destination {}: {error}", destination.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("read copy source {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("read directory entry: {error}"))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| format!("read file type for {}: {error}", source_path.display()))?;
        if file_type.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path).map_err(|error| {
                format!(
                    "copy {} to {}: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        } else {
            return Err(format!(
                "refuse non-file bundle member {}",
                source_path.display()
            ));
        }
    }
    Ok(())
}

fn append_tamper_byte(path: &Path) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|error| format!("open tamper target {}: {error}", path.display()))?;
    file.write_all(&[0xa5])
        .map_err(|error| format!("append tamper byte to {}: {error}", path.display()))
}

fn flip_first_byte(path: &Path) -> Result<(), String> {
    let mut bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let first = bytes
        .first_mut()
        .ok_or_else(|| format!("cannot mutate empty file {}", path.display()))?;
    *first ^= 0xff;
    fs::write(path, bytes).map_err(|error| format!("write {}: {error}", path.display()))
}

fn validate_tampered_bundle(
    forge: &Path,
    root: &Path,
    bundle: &Path,
    name: &str,
    expected_diagnostic: &str,
    results: &Path,
) -> Result<(), String> {
    let output = run_expect_failure(
        Command::new(forge)
            .current_dir(root)
            .arg("verify-build")
            .arg(bundle),
        &format!("Forge rejects {name} tamper"),
    )?;
    let mut evidence = Vec::new();
    evidence.extend_from_slice(&output.stdout);
    evidence.extend_from_slice(&output.stderr);
    if evidence.is_empty() {
        return Err(format!("Forge rejected {name} tamper without a diagnostic"));
    }
    require_output_fragments(
        &evidence,
        &format!("Forge {name} tamper diagnostic"),
        &[expected_diagnostic],
    )?;
    fs::write(results.join(format!("{name}.txt")), evidence)
        .map_err(|error| format!("write {name} tamper evidence: {error}"))?;
    Ok(())
}

fn write_combined_output(path: &Path, output: &Output, label: &str) -> Result<(), String> {
    let mut evidence = Vec::new();
    evidence.extend_from_slice(&output.stdout);
    evidence.extend_from_slice(&output.stderr);
    fs::write(path, evidence).map_err(|error| format!("write {label} evidence: {error}"))
}

fn run_checked(command: &mut Command, label: &str) -> Result<Output, String> {
    eprintln!("[{label}] {command:?}");
    let output = command
        .output()
        .map_err(|error| format!("spawn {label}: {error}"))?;
    if output.status.success() {
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(output)
    } else {
        Err(format!(
            "{label} failed with {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn run_expect_failure(command: &mut Command, label: &str) -> Result<Output, String> {
    eprintln!("[{label}] {command:?}");
    let output = command
        .output()
        .map_err(|error| format!("spawn {label}: {error}"))?;
    if output.status.success() {
        Err(format!("{label} unexpectedly succeeded"))
    } else {
        Ok(output)
    }
}

fn require_output_fragments(bytes: &[u8], label: &str, fragments: &[&str]) -> Result<(), String> {
    let output = String::from_utf8_lossy(bytes);
    for fragment in fragments {
        if !output.contains(fragment) {
            return Err(format!("{label} output is missing `{fragment}`"));
        }
    }
    Ok(())
}

fn sha256sum(path: &Path) -> Result<String, String> {
    let output = run_checked(
        Command::new("sha256sum").arg(path),
        &format!("hash {}", path.display()),
    )?;
    String::from_utf8(output.stdout)
        .map_err(|error| format!("sha256sum output is not UTF-8: {error}"))?
        .split_whitespace()
        .next()
        .map(str::to_string)
        .ok_or_else(|| format!("sha256sum returned no digest for {}", path.display()))
}
