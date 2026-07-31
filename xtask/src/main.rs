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
        Some("m0-composition-source-check") if args.next().is_none() => {
            m0_composition_source_check()
        }
        Some("m0-forge-probe") if args.next().is_none() => m0_forge_probe(),
        Some("m0-forge-tamper") if args.next().is_none() => m0_forge_tamper(),
        Some("m0-verus-allocator") if args.next().is_none() => m0_verus_allocator(),
        Some("m0-verus-capsule") if args.next().is_none() => m0_verus_capsule(),
        Some("toolchain-check") if args.next().is_none() => toolchain_check(),
        _ => Err(
            "usage: cargo run -p xtask -- <toolchain-check|m0-forge-probe|m0-forge-tamper|m0-composition-source-check|m0-verus-allocator|m0-verus-capsule>"
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
    let expected = "ae79a0f59ce5c08b20db47d23047f1f0665d122f";
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

    // Development-only escape for diagnosing Thermite issue #103. A release
    // probe must obtain this compiler from authoritative receipt evidence; the
    // current receipt incorrectly names ambient Rust 1.96 although Verus emits
    // Rust 1.95 metadata. Setting this variable never makes a bundle
    // release-eligible.
    let consumer_rustc = env::var_os("TMK_UNBOUND_CODEGEN_RUSTC")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("rustc"));
    let release_eligible = env::var_os("TMK_UNBOUND_CODEGEN_RUSTC").is_none();

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

    let receipt_sha = sha256sum(&receipt_path)?;
    let artifact_sha = sha256sum(&artifact)?;
    let consumer_sha = sha256sum(&kernel_consumer)?;
    let report = format!(
        "M0_FORGE_PROBE_OK\nrelease_eligible={release_eligible}\nmutants_killed=4/4\nconsumer_rustc={}\nreceipt_sha256={receipt_sha}\nartifact_sha256={artifact_sha}\nno_std_consumer_sha256={consumer_sha}\nruntime_marker={EXPECTED_RUNTIME_MARKER}\n",
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

    let report = "M0_COMPOSITION_SOURCE_OK\nrelease_eligible=false\ncomposition_build=blocked-by-thermite-issue-104\nmutants_killed=11/11\n";
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
    let mut command = Command::new(verus);
    command
        .current_dir(work)
        .args(["--output-json", "--no-vstd", "--no-cheating"]);
    if compile {
        command.arg("--compile");
    }
    command
        .args(["--rlimit", "20"])
        .args(["--smt-option", "smt.random_seed=1"])
        .args(["-C", "panic=abort"])
        .args(["-C", "overflow-checks=off"])
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
