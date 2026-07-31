use std::env;
use std::ffi::OsStr;
use std::fs;
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
        Some("m0-forge-probe") if args.next().is_none() => m0_forge_probe(),
        Some("toolchain-check") if args.next().is_none() => toolchain_check(),
        _ => Err("usage: cargo run -p xtask -- <toolchain-check|m0-forge-probe>".to_string()),
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
    let forge = env::var_os("FORGE_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/doll/Thermite/target/debug/forge"));
    require_file(&forge, "Forge binary")?;

    let skill = env::var_os("THERMITE_SKILL_REFERENCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from("/home/doll/.codex/skills/thermite/references/language.md")
        });
    require_file(&skill, "Thermite skill reference")?;
    run_checked(
        Command::new(&forge).args([
            OsStr::new("skill"),
            OsStr::new("--check"),
            skill.as_os_str(),
        ]),
        "Forge skill freshness check",
    )?;

    let work = root.join("build/m0");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;

    let thermite = root.join("thermite/core/probe.th");
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
        "M0_FORGE_PROBE_OK\nrelease_eligible={release_eligible}\nconsumer_rustc={}\nreceipt_sha256={receipt_sha}\nartifact_sha256={artifact_sha}\nno_std_consumer_sha256={consumer_sha}\nruntime_marker={EXPECTED_RUNTIME_MARKER}\n",
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

fn require_file(path: &Path, label: &str) -> Result<(), String> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("{label} does not exist at {}", path.display()))
    }
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
