use super::{
    canonical_json, require_exact_bytes, require_file, require_output_fragments, run_checked,
    run_expect_failure, sha256sum, workspace_root, write_combined_output,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SOURCE: &str = "verus/machine-model/cr3_install_capsule.rs";
const PAGE_TABLE_SOURCE: &str = "verus/platform/boot_page_tables.rs";
const CONSUMER: &str = "tests/m1/cr3_install_capsule_consumer.rs";
const LINKER: &str = "kernel-host/link/m1_cr3_capsule.ld";
const CRATE_NAME: &str = "tmk_cr3_install_capsule";
const RLIB: &str = "libtmk_cr3_install_capsule.rlib";
const CAPSULE_BYTES: &[u8] = &[0x0f, 0x22, 0xdf, 0xc3];
const RUNTIME_MARKER: &str =
    "M1_CR3_CAPSULE_OK bytes=0f22dfc3 cr3=0000000000400000 rsp=2028 invalidated=true";

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
    let tools = Tools::pinned()?;
    let source = root.join(SOURCE);
    let page_tables = root.join(PAGE_TABLE_SOURCE);
    let consumer = root.join(CONSUMER);
    let linker = root.join(LINKER);
    for (path, label) in [
        (&source, "CR3 capsule Verus source"),
        (&page_tables, "boot page-table Verus source"),
        (&consumer, "CR3 capsule consumer"),
        (&linker, "CR3 capsule linker script"),
    ] {
        require_file(path, label)?;
    }
    let source_text = fs::read_to_string(&source)
        .map_err(|error| format!("read {}: {error}", source.display()))?;
    let page_table_text = fs::read_to_string(&page_tables)
        .map_err(|error| format!("read {}: {error}", page_tables.display()))?;
    let consumer_text = fs::read_to_string(&consumer)
        .map_err(|error| format!("read {}: {error}", consumer.display()))?;
    let linker_text = fs::read_to_string(&linker)
        .map_err(|error| format!("read {}: {error}", linker.display()))?;
    audit_sources(&source_text, &page_table_text, &consumer_text, &linker_text)?;

    let work = root.join("build/m1-cr3");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;

    let model_dirs = [
        work.join("model-primary"),
        work.join("model-repro-a"),
        work.join("model-repro-b"),
    ];
    let mut artifacts = Vec::new();
    for (index, directory) in model_dirs.iter().enumerate() {
        artifacts.push(build_model(&tools, &source, directory, index == 0)?);
    }
    let model_sha = sha256sum(&artifacts[0])?;
    for artifact in artifacts.iter().skip(1) {
        let actual = sha256sum(artifact)?;
        if actual != model_sha {
            return Err(format!(
                "CR3 model {} is {actual}, expected {model_sha}",
                artifact.display()
            ));
        }
    }
    audit_model_symbols(&tools, &artifacts[0], &work)?;

    let mut executables = Vec::new();
    let mut emitted = Vec::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let executable = work.join(format!("consumer-{}", index + 1));
        let bytes = work.join(format!("capsule-{}.bin", index + 1));
        compile_consumer(&tools, &root, &consumer, artifact, &executable)?;
        let runtime = run_checked(
            Command::new(&executable).current_dir(&root).arg(&bytes),
            "execute CR3 model and emit registered bytes",
        )?;
        require_output_fragments(&runtime.stdout, "CR3 capsule runtime", &[RUNTIME_MARKER])?;
        require_exact_bytes(&bytes, CAPSULE_BYTES, "emitted CR3 capsule")?;
        write_combined_output(
            &work.join(format!("runtime-{}.txt", index + 1)),
            &runtime,
            "CR3 capsule runtime evidence",
        )?;
        executables.push(executable);
        emitted.push(bytes);
    }
    let consumer_sha = require_same_digest(&executables, "CR3 consumer")?;
    let emitted_sha = require_same_digest(&emitted, "emitted CR3 capsule")?;

    let link_dirs = [
        work.join("link-primary"),
        work.join("link-repro-a"),
        work.join("link-repro-b"),
    ];
    let mut linked = Vec::new();
    for (directory, bytes) in link_dirs.iter().zip(emitted.iter()) {
        linked.push(link_capsule(&tools, &linker, bytes, directory)?);
    }
    let linked_sha = require_same_digest(
        &linked
            .iter()
            .map(|capsule| capsule.bytes.clone())
            .collect::<Vec<_>>(),
        "linked CR3 capsule",
    )?;
    let elf_sha = require_same_digest(
        &linked
            .iter()
            .map(|capsule| capsule.elf.clone())
            .collect::<Vec<_>>(),
        "linked CR3 ELF",
    )?;
    audit_linked_capsule(&tools, &linked[0], &work)?;
    run_link_negatives(&tools, &linker, &work)?;
    run_proof_negatives(&tools, &source_text, &work)?;

    let report = format!(
        "M1_CR3_CAPSULE_OK\ncomponent_verified=true\nrelease_eligible=false\nhardware_executed=false\nsource_sha256={}\npage_table_source_sha256={}\nconsumer_source_sha256={}\nlinker_script_sha256={}\nmodel_artifact_sha256={model_sha}\nconsumer_sha256={consumer_sha}\nemitted_capsule_sha256={emitted_sha}\nlinked_capsule_sha256={linked_sha}\nlinked_elf_sha256={elf_sha}\nmodel_reproducibility_builds=3\nconsumer_reproducibility_builds=3\npost_link_reproducibility_builds=3\nverus_verified=15\nmodel_undefined_symbols=core-panic,memcpy\nroot_physical=0000000000400000\nlinked_virtual=ffffffff80001000\ncaller_requirements=cpl0,pcid-disabled,aligned-52-bit-root,readable-return-stack,canonical-return\nruntime_marker={RUNTIME_MARKER}\nnegative_cases=byte-mutation,unregistered-executable,cr3-semantics,tlb-semantics,root-binding,bad-assume\n",
        sha256sum(&source)?,
        sha256sum(&page_tables)?,
        sha256sum(&consumer)?,
        sha256sum(&linker)?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M1 CR3 report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn audit_sources(
    source: &str,
    page_tables: &str,
    consumer: &str,
    linker: &str,
) -> Result<(), String> {
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
                "CR3 capsule source contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "#![no_std]",
        "pub const ROOT_PHYSICAL: u64 = 0x0040_0000;",
        "pub const REGISTERED_WORD: u32 = 0xc3df_220f;",
        "pub open spec fn install_precondition",
        "pub fn decode_execute",
        "pub fn install_registered_root",
        "state.rdi == ROOT_PHYSICAL",
        "non_global_tlb_valid: false",
        "cr3: state.rdi",
        "rsp: state.rsp + 8",
        "rip: state.return_address",
    ] {
        if !source.contains(required) {
            return Err(format!("CR3 capsule source is missing `{required}`"));
        }
    }
    if page_tables
        .matches("pub const ROOT_PHYSICAL: u64 = 0x0040_0000;")
        .count()
        != 1
    {
        return Err("CR3 capsule root is not bound to the page-table root constant".to_string());
    }
    for forbidden in ["unsafe ", "asm!", "global_asm!"] {
        if consumer.contains(forbidden) {
            return Err(format!("CR3 consumer contains forbidden `{forbidden}`"));
        }
    }
    for required in [
        "[0x0f, 0x22, 0xdf, 0xc3]",
        "install_registered_root(state())",
        "assert!(other_root.accepted)",
        "assert!(!ring_three.accepted)",
        "assert!(!misaligned.accepted)",
        "assert!(!pcid.accepted)",
        "assert!(!noncanonical_return.accepted)",
    ] {
        if !consumer.contains(required) {
            return Err(format!("CR3 consumer is missing `{required}`"));
        }
    }
    for required in [
        "ENTRY(tmk_install_cr3_capsule)",
        ". = 0xffffffff80001000;",
        ".text.tmk_cr3_capsule",
        "SIZEOF(.text.tmk_cr3_capsule) == 4",
    ] {
        if !linker.contains(required) {
            return Err(format!("CR3 linker script is missing `{required}`"));
        }
    }
    Ok(())
}

fn verus_command(tools: &Tools, directory: &Path, source_name: &str, compile: bool) -> Command {
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
        .args(["--rlimit", "60"])
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
        .map_err(|error| format!("create CR3 model path {}: {error}", directory.display()))?;
    let staged = directory.join(format!("{CRATE_NAME}.rs"));
    fs::copy(source, &staged).map_err(|error| format!("stage CR3 model: {error}"))?;
    let output = run_checked(
        &mut verus_command(tools, directory, &format!("{CRATE_NAME}.rs"), true),
        "Verus CR3 capsule proof and codegen",
    )?;
    require_output_fragments(
        &output.stdout,
        "Verus CR3 capsule proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 15",
            "\"errors\": 0",
            "\"version\": \"0.2026.05.24.ecee80a\"",
        ],
    )?;
    if retain_result {
        fs::write(
            directory.join("verus-result.json"),
            canonical_json(&output.stdout, "CR3 capsule Verus result")?,
        )
        .map_err(|error| format!("write CR3 Verus result: {error}"))?;
    }
    let artifact = directory.join(RLIB);
    require_file(&artifact, "compiled CR3 model")?;
    Ok(artifact)
}

fn audit_model_symbols(tools: &Tools, artifact: &Path, work: &Path) -> Result<(), String> {
    let undefined = run_checked(
        Command::new(&tools.nm).arg("-u").arg(artifact),
        "audit CR3 model undefined symbols",
    )?;
    let undefined_text = String::from_utf8_lossy(&undefined.stdout);
    let names: BTreeSet<_> = undefined_text
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .filter(|name| !name.ends_with(':'))
        .collect();
    if names.len() != 2
        || !names.contains("memcpy")
        || !names.iter().any(|name| name.ends_with("5panic"))
    {
        return Err(format!(
            "CR3 model has unexpected undefined symbols: {names:?}"
        ));
    }
    write_combined_output(
        &work.join("model-undefined-symbols.txt"),
        &undefined,
        "CR3 model symbol evidence",
    )?;
    let defined = run_checked(
        Command::new(&tools.nm).arg("--defined-only").arg(artifact),
        "audit CR3 model defined symbols",
    )?;
    require_output_fragments(
        &defined.stdout,
        "CR3 model defined symbols",
        &[
            "decode_execute",
            "install_registered_root",
            "registered_word",
        ],
    )?;
    write_combined_output(
        &work.join("model-defined-symbols.txt"),
        &defined,
        "CR3 model defined-symbol evidence",
    )
}

fn compile_consumer(
    tools: &Tools,
    root: &Path,
    source: &Path,
    artifact: &Path,
    output: &Path,
) -> Result<(), String> {
    run_checked(
        Command::new(&tools.rustc)
            .current_dir(root)
            .env("SOURCE_DATE_EPOCH", "0")
            .args(["--edition=2021"])
            .arg(source)
            .arg("--extern")
            .arg(format!("{CRATE_NAME}={}", artifact.display()))
            .args(["-L", "dependency=/opt/verus/0.2026.05.24.ecee80a"])
            .args(["-C", "panic=abort"])
            .args(["-C", "relocation-model=static"])
            .args(["-C", "codegen-units=1"])
            .arg(format!("--remap-path-prefix={}=.", root.display()))
            .arg("-o")
            .arg(output),
        "compile separate CR3 capsule consumer",
    )?;
    Ok(())
}

fn require_same_digest(paths: &[PathBuf], label: &str) -> Result<String, String> {
    let first = paths
        .first()
        .ok_or_else(|| format!("no {label} artifacts were supplied"))?;
    let expected = sha256sum(first)?;
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

fn link_capsule(
    tools: &Tools,
    linker: &Path,
    bytes: &Path,
    directory: &Path,
) -> Result<LinkedCapsule, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create CR3 link path {}: {error}", directory.display()))?;
    fs::copy(bytes, directory.join("capsule.bin"))
        .map_err(|error| format!("stage CR3 bytes for link: {error}"))?;
    run_checked(
        Command::new(&tools.ld).current_dir(directory).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "capsule-raw.o",
            "capsule.bin",
        ]),
        "wrap CR3 capsule bytes",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(directory).args([
            "--rename-section",
            ".data=.text.tmk_cr3_capsule,alloc,contents,load,readonly,code",
            "--redefine-sym",
            "_binary_capsule_bin_start=tmk_install_cr3_capsule",
            "--redefine-sym",
            "_binary_capsule_bin_end=tmk_install_cr3_capsule_end",
            "--redefine-sym",
            "_binary_capsule_bin_size=tmk_install_cr3_capsule_size",
            "capsule-raw.o",
            "capsule.o",
        ]),
        "name CR3 capsule object",
    )?;
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
            .args(["-o", "capsule.elf", "capsule.o"]),
        "link registered CR3 capsule ELF",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(directory).args([
            "--dump-section",
            ".text.tmk_cr3_capsule=linked-capsule.bin",
            "capsule.elf",
        ]),
        "extract linked CR3 capsule bytes",
    )?;
    let linked = LinkedCapsule {
        elf: directory.join("capsule.elf"),
        bytes: directory.join("linked-capsule.bin"),
    };
    require_exact_bytes(&linked.bytes, CAPSULE_BYTES, "linked CR3 capsule")?;
    Ok(linked)
}

fn audit_linked_capsule(tools: &Tools, linked: &LinkedCapsule, work: &Path) -> Result<(), String> {
    let relocations = run_checked(
        Command::new(&tools.readelf).args(["-rW"]).arg(&linked.elf),
        "audit CR3 capsule relocations",
    )?;
    require_output_fragments(
        &relocations.stdout,
        "CR3 capsule relocations",
        &["There are no relocations in this file"],
    )?;
    write_combined_output(
        &work.join("linked-relocations.txt"),
        &relocations,
        "CR3 relocation evidence",
    )?;

    let sections = run_checked(
        Command::new(&tools.readelf).args(["-SW"]).arg(&linked.elf),
        "audit CR3 capsule sections",
    )?;
    audit_executable_sections(&sections)?;
    write_combined_output(
        &work.join("linked-sections.txt"),
        &sections,
        "CR3 section evidence",
    )?;

    let symbols = run_checked(
        Command::new(&tools.readelf).args(["-sW"]).arg(&linked.elf),
        "audit CR3 capsule symbols",
    )?;
    require_output_fragments(
        &symbols.stdout,
        "CR3 capsule symbols",
        &[
            "tmk_install_cr3_capsule_link_start",
            "tmk_install_cr3_capsule_link_end",
            "tmk_install_cr3_capsule",
            "ffffffff80001000",
        ],
    )?;
    write_combined_output(
        &work.join("linked-symbols.txt"),
        &symbols,
        "CR3 symbol evidence",
    )?;

    let disassembly = run_checked(
        Command::new(&tools.objdump)
            .args(["-d", "-Mintel"])
            .arg(&linked.elf),
        "disassemble CR3 capsule",
    )?;
    require_output_fragments(
        &disassembly.stdout,
        "CR3 capsule disassembly",
        &["mov    cr3,rdi", "ret"],
    )?;
    write_combined_output(
        &work.join("linked-disassembly.txt"),
        &disassembly,
        "CR3 disassembly evidence",
    )
}

fn audit_executable_sections(output: &Output) -> Result<(), String> {
    let text = String::from_utf8_lossy(&output.stdout);
    let executable: Vec<_> = text.lines().filter(|line| line.contains(" AX ")).collect();
    if executable.len() == 1 && executable[0].contains(".text.tmk_cr3_capsule") {
        Ok(())
    } else {
        Err(format!(
            "CR3 executable section allowlist mismatch: {executable:?}"
        ))
    }
}

fn run_link_negatives(tools: &Tools, linker: &Path, work: &Path) -> Result<(), String> {
    let mutated_dir = work.join("negative-byte-mutation");
    fs::create_dir(&mutated_dir)
        .map_err(|error| format!("create CR3 byte negative path: {error}"))?;
    let mutated = mutated_dir.join("mutated.bin");
    fs::write(&mutated, [0x0e, 0x22, 0xdf, 0xc3])
        .map_err(|error| format!("write CR3 byte mutation: {error}"))?;
    let linked = link_capsule_unchecked(tools, linker, &mutated, &mutated_dir)?;
    let diagnostic = require_exact_bytes(&linked.bytes, CAPSULE_BYTES, "mutated CR3 capsule")
        .expect_err("CR3 byte mutation must fail exact-byte audit");
    fs::write(
        work.join("negative-byte-mutation.txt"),
        format!("{diagnostic}\n"),
    )
    .map_err(|error| format!("write CR3 byte negative evidence: {error}"))?;

    let extra_dir = work.join("negative-unregistered-executable");
    fs::create_dir(&extra_dir)
        .map_err(|error| format!("create CR3 extra-section path: {error}"))?;
    fs::write(extra_dir.join("capsule.bin"), CAPSULE_BYTES)
        .map_err(|error| format!("write CR3 base bytes: {error}"))?;
    fs::write(extra_dir.join("extra.bin"), [0x90])
        .map_err(|error| format!("write unregistered CR3 executable byte: {error}"))?;
    wrap_named_capsule(tools, &extra_dir)?;
    run_checked(
        Command::new(&tools.ld).current_dir(&extra_dir).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "extra-raw.o",
            "extra.bin",
        ]),
        "wrap unregistered CR3 executable byte",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(&extra_dir).args([
            "--rename-section",
            ".data=.text.unregistered,alloc,contents,load,readonly,code",
            "extra-raw.o",
            "extra.o",
        ]),
        "classify unregistered CR3 executable section",
    )?;
    run_checked(
        Command::new(&tools.ld)
            .current_dir(&extra_dir)
            .args([
                "-m",
                "elf_x86_64",
                "--build-id=none",
                "-nostdlib",
                "-static",
            ])
            .arg("-T")
            .arg(linker)
            .args(["-o", "extra.elf", "capsule.o", "extra.o"]),
        "link CR3 ELF with unregistered executable section",
    )?;
    let sections = run_checked(
        Command::new(&tools.readelf)
            .current_dir(&extra_dir)
            .args(["-SW", "extra.elf"]),
        "inspect unregistered CR3 executable section",
    )?;
    let diagnostic = audit_executable_sections(&sections)
        .expect_err("unregistered CR3 executable section must fail audit");
    fs::write(
        work.join("negative-unregistered-executable.txt"),
        format!("{diagnostic}\n"),
    )
    .map_err(|error| format!("write CR3 executable-section negative: {error}"))?;
    Ok(())
}

fn link_capsule_unchecked(
    tools: &Tools,
    linker: &Path,
    bytes: &Path,
    directory: &Path,
) -> Result<LinkedCapsule, String> {
    fs::copy(bytes, directory.join("capsule.bin"))
        .map_err(|error| format!("stage unchecked CR3 capsule: {error}"))?;
    wrap_named_capsule(tools, directory)?;
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
            .args(["-o", "capsule.elf", "capsule.o"]),
        "link unchecked CR3 capsule ELF",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(directory).args([
            "--dump-section",
            ".text.tmk_cr3_capsule=linked-capsule.bin",
            "capsule.elf",
        ]),
        "extract unchecked CR3 capsule bytes",
    )?;
    Ok(LinkedCapsule {
        elf: directory.join("capsule.elf"),
        bytes: directory.join("linked-capsule.bin"),
    })
}

fn wrap_named_capsule(tools: &Tools, directory: &Path) -> Result<(), String> {
    run_checked(
        Command::new(&tools.ld).current_dir(directory).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "capsule-raw.o",
            "capsule.bin",
        ]),
        "wrap named CR3 capsule bytes",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(directory).args([
            "--rename-section",
            ".data=.text.tmk_cr3_capsule,alloc,contents,load,readonly,code",
            "--redefine-sym",
            "_binary_capsule_bin_start=tmk_install_cr3_capsule",
            "--redefine-sym",
            "_binary_capsule_bin_end=tmk_install_cr3_capsule_end",
            "--redefine-sym",
            "_binary_capsule_bin_size=tmk_install_cr3_capsule_size",
            "capsule-raw.o",
            "capsule.o",
        ]),
        "name CR3 capsule object",
    )?;
    Ok(())
}

fn run_proof_negatives(tools: &Tools, source: &str, work: &Path) -> Result<(), String> {
    let cases = [
        (
            "cr3-semantics",
            "cr3: state.rdi,",
            "cr3: state.cr3,",
            "postcondition not satisfied",
        ),
        (
            "tlb-semantics",
            "non_global_tlb_valid: false,",
            "non_global_tlb_valid: state.non_global_tlb_valid,",
            "postcondition not satisfied",
        ),
        (
            "root-binding",
            "state.rdi == ROOT_PHYSICAL,",
            "state.rdi == 0x0080_0000,",
            "precondition not satisfied",
        ),
    ];
    for (name, needle, replacement, diagnostic) in cases {
        reject_proof_mutation(tools, source, work, name, needle, replacement, diagnostic)?;
    }
    reject_proof_mutation(
        tools,
        source,
        work,
        "bad-assume",
        "    if word == REGISTERED_WORD",
        "    assume(false);\n    if word == REGISTERED_WORD",
        "assume/admit not allowed with --no-cheating",
    )
}

fn reject_proof_mutation(
    tools: &Tools,
    source: &str,
    work: &Path,
    name: &str,
    needle: &str,
    replacement: &str,
    diagnostic: &str,
) -> Result<(), String> {
    if source.matches(needle).count() != 1 {
        return Err(format!("CR3 proof negative `{name}` target is not unique"));
    }
    let directory = work.join(format!("negative-{name}"));
    fs::create_dir(&directory)
        .map_err(|error| format!("create CR3 {name} negative path: {error}"))?;
    let staged = directory.join(format!("{CRATE_NAME}.rs"));
    fs::write(&staged, source.replacen(needle, replacement, 1))
        .map_err(|error| format!("write CR3 {name} mutation: {error}"))?;
    let output = run_expect_failure(
        &mut verus_command(tools, &directory, &format!("{CRATE_NAME}.rs"), false),
        &format!("Verus rejects CR3 {name} mutation"),
    )?;
    let mut combined = Vec::new();
    combined.extend_from_slice(&output.stdout);
    combined.extend_from_slice(&output.stderr);
    require_output_fragments(&combined, &format!("CR3 {name} rejection"), &[diagnostic])?;
    if directory.join(RLIB).exists() {
        return Err(format!("CR3 {name} mutation published an rlib"));
    }
    write_combined_output(
        &work.join(format!("negative-{name}.txt")),
        &output,
        &format!("CR3 {name} negative evidence"),
    )
}
