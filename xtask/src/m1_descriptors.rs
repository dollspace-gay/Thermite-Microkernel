use super::{
    canonical_json, require_file, require_output_fragments, run_checked, run_expect_failure,
    sha256sum, workspace_root, write_combined_output,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE: &str = "verus/platform/descriptor_tables.rs";
const CONSUMER: &str = "tests/m1/descriptor_tables_consumer.rs";
const CRATE_NAME: &str = "tmk_descriptor_tables";
const RLIB: &str = "libtmk_descriptor_tables.rlib";
const VERUS_VERSION: &str = "0.2026.05.24.ecee80a";
const RUNTIME_MARKER: &str =
    "M1_DESCRIPTOR_TABLES_OK observation=255 gdt=7 idt=256 ist=3 dpl3=1 tss=104";

struct Tools {
    verus: PathBuf,
    rustc: PathBuf,
    ar: PathBuf,
    nm: PathBuf,
}

impl Tools {
    fn pinned() -> Result<Self, String> {
        let tools = Self {
            verus: PathBuf::from("/opt/verus/0.2026.05.24.ecee80a/verus"),
            rustc: PathBuf::from(
                "/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc",
            ),
            ar: PathBuf::from("/usr/sbin/ar"),
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
                tools.ar.as_path(),
                "a21151402078c113fd801d16e0a0d2659ee44cee1b9828474f937bbf097b0df6",
                "GNU ar",
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

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let tools = Tools::pinned()?;
    let source = root.join(SOURCE);
    let consumer = root.join(CONSUMER);
    require_file(&source, "descriptor-table Verus source")?;
    require_file(&consumer, "descriptor-table runtime consumer")?;

    let source_text = fs::read_to_string(&source)
        .map_err(|error| format!("read {}: {error}", source.display()))?;
    let consumer_text = fs::read_to_string(&consumer)
        .map_err(|error| format!("read {}: {error}", consumer.display()))?;
    audit_sources(&source_text, &consumer_text)?;

    let work = root.join("build/m1-descriptors");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;

    let build_dirs = [
        work.join("model-primary"),
        work.join("model-repro-a"),
        work.join("model-repro-b"),
    ];
    let mut artifacts = Vec::new();
    for (index, directory) in build_dirs.iter().enumerate() {
        artifacts.push(build_model(&tools, &source, directory, index == 0)?);
    }
    let model_sha = require_same_digest(&artifacts, "descriptor-table model")?;
    audit_artifact(&tools, &artifacts[0], &work)?;
    let consumer_sha = run_consumers(&tools, &root, &consumer, &artifacts, &work)?;
    run_negative_proofs(&tools, &source_text, &work)?;
    run_no_vstd_boundary(&tools, &source, &work)?;

    let report = format!(
        "M1_DESCRIPTOR_TABLES_OK\ncomponent_verified=true\nrelease_eligible=false\nhardware_loaded=false\nsource_sha256={}\nconsumer_source_sha256={}\nmodel_artifact_sha256={model_sha}\nconsumer_sha256={consumer_sha}\nmodel_reproducibility_builds=3\nconsumer_reproducibility_builds=3\nverus_verified=36\ngdt_entries=7\nidt_entries=256\ntss_bytes=104\nuser_callable_vectors=1\nist_vectors=3\nproof_library=vstd-array-spec-only\nexecutable_undefined_symbols=core-panic,core-panic-bounds-check,memcpy\nruntime_marker={RUNTIME_MARKER}\nnegative_cases=breakpoint-dpl,double-fault-ist,user-code-descriptor,iomap-base,idt-completeness,observation,bad-assume,vstd-proof-dependency\n",
        sha256sum(&source)?,
        sha256sum(&consumer)?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M1 descriptor-table report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn audit_sources(source: &str, consumer: &str) -> Result<(), String> {
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
        if source.contains(forbidden) {
            return Err(format!(
                "descriptor-table Verus source contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "#![no_std]",
        "use vstd::array::ArrayAdditionalSpecFns;",
        "#[repr(C, align(16))]\npub struct Idt",
        "#[repr(C, align(8))]\npub struct Gdt",
        "#[repr(C, packed)]\npub struct Tss64",
        "#[repr(C, packed)]\npub struct DescriptorTablePointer",
        "pub const KERNEL_CODE_SELECTOR: u16 = 0x08;",
        "pub const USER_CODE_SELECTOR: u16 = 0x23;",
        "pub const TSS_SELECTOR: u16 = 0x28;",
        "pub open spec fn idt_well_formed",
        "pub open spec fn gdt_well_formed",
        "pub fn registered_idt()",
        "pub fn registered_gdt(tss_base: u64)",
        "pub fn registered_tss()",
        "while slot < IDT_ENTRIES",
        "forall|index: int| 0 <= index < slot",
        "if vector == 3 { 0xee } else { 0x8e }",
        "if vector == 8 { 1 }",
        "else if vector == 2 { 2 }",
        "else if vector == 18 { 3 }",
        "result.iomap_base == 104",
        "ensures result == 255",
    ] {
        if !source.contains(required) {
            return Err(format!("descriptor-table source is missing `{required}`"));
        }
    }
    if source
        .matches("pub entries: [IdtGate; IDT_ENTRIES]")
        .count()
        != 1
        || source.matches("pub entries: [u64; GDT_ENTRIES]").count() != 1
    {
        return Err("descriptor-table source layout declaration changed".to_string());
    }

    for forbidden in ["unsafe ", "asm!", "global_asm!", "external_body", "assume("] {
        if consumer.contains(forbidden) {
            return Err(format!(
                "descriptor-table consumer contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "std::mem::size_of::<Tss64>()",
        "std::mem::size_of::<DescriptorTablePointer>()",
        "for (vector, gate) in idt.entries.iter().enumerate()",
        "assert_eq!(present, 256)",
        "assert_eq!(user_callable, 1)",
        "assert_eq!(ist_entries, 3)",
        "assert_eq!(iomap_base, 104)",
        "assert_eq!(gdtr_limit, 55)",
        "assert_eq!(idtr_limit, 4095)",
        "M1_DESCRIPTOR_TABLES_OK observation={observation} gdt=7 idt={present} ist={ist_entries} dpl3={user_callable} tss=104",
    ] {
        if !consumer.contains(required) {
            return Err(format!("descriptor-table consumer is missing `{required}`"));
        }
    }
    Ok(())
}

fn verus_command(
    tools: &Tools,
    directory: &Path,
    source_name: &str,
    compile: bool,
    no_vstd: bool,
) -> Command {
    let mut command = Command::new(&tools.verus);
    command
        .current_dir(directory)
        .env("SOURCE_DATE_EPOCH", "0")
        .args(["--output-json", "--no-cheating", "--multiple-errors", "20"]);
    if compile {
        command.arg("--compile");
    }
    if no_vstd {
        command.arg("--no-vstd");
    }
    command
        .args(["--rlimit", "150"])
        .args(["--smt-option", "smt.random_seed=1"])
        .args(["-C", "panic=abort"])
        .args(["-C", "overflow-checks=off"])
        .args(["-C", "relocation-model=static"])
        .args(["-C", "no-redzone=yes"])
        .arg(format!("--remap-path-prefix={}=.", directory.display()))
        .arg(source_name);
    command
}

fn build_model(
    tools: &Tools,
    source: &Path,
    directory: &Path,
    retain_result: bool,
) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create model path {}: {error}", directory.display()))?;
    let staged = directory.join(format!("{CRATE_NAME}.rs"));
    fs::copy(source, &staged).map_err(|error| format!("stage descriptor-table model: {error}"))?;
    let output = run_checked(
        &mut verus_command(tools, directory, &format!("{CRATE_NAME}.rs"), true, false),
        "Verus descriptor-table proof and codegen",
    )?;
    require_output_fragments(
        &output.stdout,
        "Verus descriptor-table proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 36",
            "\"errors\": 0",
            &format!("\"version\": \"{VERUS_VERSION}\""),
        ],
    )?;
    if retain_result {
        fs::write(
            directory.join("verus-result.json"),
            canonical_json(&output.stdout, "descriptor-table Verus result")?,
        )
        .map_err(|error| format!("write descriptor-table Verus result: {error}"))?;
    }
    let artifact = directory.join(RLIB);
    require_file(&artifact, "compiled descriptor-table model")?;
    Ok(artifact)
}

fn require_same_digest(paths: &[PathBuf], label: &str) -> Result<String, String> {
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

fn audit_artifact(tools: &Tools, artifact: &Path, work: &Path) -> Result<(), String> {
    let members = run_checked(
        Command::new(&tools.ar).arg("t").arg(artifact),
        "list descriptor-table rlib members",
    )?;
    let member_text = String::from_utf8_lossy(&members.stdout);
    if member_text.lines().count() != 2
        || !member_text.lines().any(|line| line == "lib.rmeta")
        || member_text
            .lines()
            .filter(|line| line.ends_with(".rcgu.o"))
            .count()
            != 1
    {
        return Err(format!(
            "descriptor-table rlib member inventory changed:\n{member_text}"
        ));
    }
    write_combined_output(
        &work.join("artifact-members.txt"),
        &members,
        "rlib member audit",
    )?;

    let undefined = run_checked(
        Command::new(&tools.nm).arg("-u").arg(artifact),
        "audit descriptor-table undefined symbols",
    )?;
    let undefined_text = String::from_utf8_lossy(&undefined.stdout);
    let names: BTreeSet<_> = undefined_text
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .filter(|name| !name.ends_with(':'))
        .collect();
    if names.len() != 3
        || !names.contains("memcpy")
        || !names.iter().any(|name| name.ends_with("5panic"))
        || !names
            .iter()
            .any(|name| name.ends_with("18panic_bounds_check"))
    {
        return Err(format!(
            "descriptor-table executable undefined-symbol set changed: {names:?}"
        ));
    }
    write_combined_output(
        &work.join("artifact-undefined-symbols.txt"),
        &undefined,
        "undefined-symbol audit",
    )?;

    let defined = run_checked(
        Command::new(&tools.nm).arg("--defined-only").arg(artifact),
        "audit descriptor-table defined symbols",
    )?;
    require_output_fragments(
        &defined.stdout,
        "descriptor-table defined symbols",
        &[
            "registered_idt",
            "registered_gdt",
            "registered_tss",
            "descriptor_table_observation",
        ],
    )?;
    write_combined_output(
        &work.join("artifact-defined-symbols.txt"),
        &defined,
        "defined-symbol audit",
    )
}

fn run_consumers(
    tools: &Tools,
    root: &Path,
    consumer: &Path,
    artifacts: &[PathBuf],
    work: &Path,
) -> Result<String, String> {
    let mut executables = Vec::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let executable = work.join(format!("consumer-{}", index + 1));
        run_checked(
            Command::new(&tools.rustc)
                .current_dir(root)
                .env("SOURCE_DATE_EPOCH", "0")
                .args(["--edition=2021"])
                .arg(consumer)
                .arg("--extern")
                .arg(format!("{CRATE_NAME}={}", artifact.display()))
                .args(["-L", "dependency=/opt/verus/0.2026.05.24.ecee80a"])
                .args(["-C", "panic=abort"])
                .args(["-C", "relocation-model=static"])
                .args(["-C", "codegen-units=1"])
                .arg(format!("--remap-path-prefix={}=.", root.display()))
                .arg("-o")
                .arg(&executable),
            "compile separate descriptor-table runtime consumer",
        )?;
        let runtime = run_checked(
            Command::new(&executable).current_dir(root),
            "execute descriptor-table runtime consumer",
        )?;
        require_output_fragments(
            &runtime.stdout,
            "descriptor-table runtime consumer",
            &[RUNTIME_MARKER],
        )?;
        write_combined_output(
            &work.join(format!("runtime-{}.txt", index + 1)),
            &runtime,
            "descriptor-table runtime evidence",
        )?;
        executables.push(executable);
    }
    require_same_digest(&executables, "descriptor-table consumer")
}

fn run_negative_proofs(tools: &Tools, source: &str, work: &Path) -> Result<(), String> {
    let cases = [
        (
            "breakpoint-dpl",
            "let attributes: u64 = if vector == 3 { 0xee } else { 0x8e };",
            "let attributes: u64 = if vector == 3 { 0x8e } else { 0x8e };",
        ),
        (
            "double-fault-ist",
            "let ist: u64 = if vector == 8 { 1 }",
            "let ist: u64 = if vector == 8 { 0 }",
        ),
        (
            "user-code-descriptor",
            "            USER_DATA_DESCRIPTOR,\n            USER_CODE_DESCRIPTOR,\n            tss.low,",
            "            USER_DATA_DESCRIPTOR,\n            USER_DATA_DESCRIPTOR,\n            tss.low,",
        ),
        (
            "iomap-base",
            "        iomap_base: 104,",
            "        iomap_base: 0,",
        ),
        (
            "idt-completeness",
            "    while slot < IDT_ENTRIES",
            "    while slot < IDT_ENTRIES - 1",
        ),
        (
            "observation",
            "    ensures result == 255,",
            "    ensures result == 254,",
        ),
        (
            "bad-assume",
            "    let idt = registered_idt();",
            "    assume(false);\n    let idt = registered_idt();",
        ),
    ];
    for (name, needle, replacement) in cases {
        if source.matches(needle).count() != 1 {
            return Err(format!(
                "descriptor-table negative `{name}` target is not unique"
            ));
        }
        let directory = work.join(format!("negative-{name}"));
        fs::create_dir(&directory)
            .map_err(|error| format!("create negative {name} directory: {error}"))?;
        let staged = directory.join(format!("{CRATE_NAME}.rs"));
        fs::write(&staged, source.replacen(needle, replacement, 1))
            .map_err(|error| format!("write negative {name} source: {error}"))?;
        let output = run_expect_failure(
            &mut verus_command(tools, &directory, &format!("{CRATE_NAME}.rs"), false, false),
            &format!("reject descriptor-table {name} mutation"),
        )?;
        let mut combined = Vec::new();
        combined.extend_from_slice(&output.stdout);
        combined.extend_from_slice(&output.stderr);
        if combined.is_empty()
            || combined
                .windows(b"\"success\": true".len())
                .any(|window| window == b"\"success\": true")
            || directory.join(RLIB).exists()
        {
            return Err(format!(
                "descriptor-table {name} mutation did not fail atomically"
            ));
        }
        write_combined_output(
            &work.join(format!("negative-{name}.txt")),
            &output,
            &format!("descriptor-table {name} negative"),
        )?;
    }
    Ok(())
}

fn run_no_vstd_boundary(tools: &Tools, source: &Path, work: &Path) -> Result<(), String> {
    let directory = work.join("negative-no-vstd");
    fs::create_dir(&directory)
        .map_err(|error| format!("create no-vstd boundary directory: {error}"))?;
    let staged = directory.join(format!("{CRATE_NAME}.rs"));
    fs::copy(source, &staged).map_err(|error| format!("stage no-vstd source: {error}"))?;
    let output = run_expect_failure(
        &mut verus_command(tools, &directory, &format!("{CRATE_NAME}.rs"), false, true),
        "confirm explicit vstd array proof dependency",
    )?;
    let mut combined = Vec::new();
    combined.extend_from_slice(&output.stdout);
    combined.extend_from_slice(&output.stderr);
    require_output_fragments(
        &combined,
        "no-vstd descriptor-table proof boundary",
        &["cannot find module or crate `vstd`"],
    )?;
    write_combined_output(
        &work.join("negative-vstd-proof-dependency.txt"),
        &output,
        "vstd proof dependency evidence",
    )
}
