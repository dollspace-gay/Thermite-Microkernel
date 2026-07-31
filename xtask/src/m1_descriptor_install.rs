use super::{
    canonical_json, require_exact_bytes, require_file, require_output_fragments, run_checked,
    run_expect_failure, sha256sum, workspace_root, write_combined_output,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SOURCE: &str = "verus/machine-model/descriptor_install_capsule.rs";
const DESCRIPTOR_SOURCE: &str = "verus/platform/descriptor_tables.rs";
const CONSUMER: &str = "tests/m1/descriptor_install_capsule_consumer.rs";
const LINKER: &str = "kernel-host/link/m1_descriptor_install_capsule.ld";
const CRATE_NAME: &str = "tmk_descriptor_install_capsule";
const RLIB: &str = "libtmk_descriptor_install_capsule.rlib";
const CAPSULE_BYTES: &[u8] = &[
    0x0f, 0x01, 0x17, 0xb8, 0x10, 0x00, 0x00, 0x00, 0x8e, 0xd8, 0x8e, 0xc0, 0x8e, 0xd0, 0x6a, 0x08,
    0x48, 0x8d, 0x05, 0x03, 0x00, 0x00, 0x00, 0x50, 0x48, 0xcb, 0xb8, 0x28, 0x00, 0x00, 0x00, 0x0f,
    0x00, 0xd8, 0x0f, 0x01, 0x1e, 0xc3,
];
const RUNTIME_MARKER: &str =
    "M1_DESCRIPTOR_INSTALL_OK bytes=38 cs=08 ss=10 tr=28 rsp=ffffe00000001008 busy=true";

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
    let descriptor_source = root.join(DESCRIPTOR_SOURCE);
    let consumer = root.join(CONSUMER);
    let linker = root.join(LINKER);
    for (path, label) in [
        (&source, "descriptor-install Verus source"),
        (&descriptor_source, "descriptor-table Verus source"),
        (&consumer, "descriptor-install runtime consumer"),
        (&linker, "descriptor-install linker script"),
    ] {
        require_file(path, label)?;
    }
    let source_text = read(&source)?;
    let descriptor_text = read(&descriptor_source)?;
    let consumer_text = read(&consumer)?;
    let linker_text = read(&linker)?;
    audit_sources(&source_text, &descriptor_text, &consumer_text, &linker_text)?;

    let work = root.join("build/m1-descriptor-install");
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
    let model_sha = require_same_digest(&artifacts, "descriptor-install model")?;
    audit_model_symbols(&tools, &artifacts[0], &work)?;

    let mut executables = Vec::new();
    let mut emitted = Vec::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let executable = work.join(format!("consumer-{}", index + 1));
        let bytes = work.join(format!("capsule-{}.bin", index + 1));
        compile_consumer(&tools, &root, &consumer, artifact, &executable)?;
        let runtime = run_checked(
            Command::new(&executable).current_dir(&root).arg(&bytes),
            "execute descriptor-install model and emit registered bytes",
        )?;
        require_output_fragments(
            &runtime.stdout,
            "descriptor-install runtime",
            &[RUNTIME_MARKER],
        )?;
        require_exact_bytes(&bytes, CAPSULE_BYTES, "emitted descriptor-install capsule")?;
        write_combined_output(
            &work.join(format!("runtime-{}.txt", index + 1)),
            &runtime,
            "descriptor-install runtime evidence",
        )?;
        executables.push(executable);
        emitted.push(bytes);
    }
    let consumer_sha = require_same_digest(&executables, "descriptor-install consumer")?;
    let emitted_sha = require_same_digest(&emitted, "emitted descriptor-install capsule")?;

    let link_dirs = [
        work.join("link-primary"),
        work.join("link-repro-a"),
        work.join("link-repro-b"),
    ];
    let mut linked = Vec::new();
    for (directory, bytes) in link_dirs.iter().zip(emitted.iter()) {
        linked.push(link_capsule(&tools, &linker, bytes, directory, true)?);
    }
    let linked_sha = require_same_digest(
        &linked
            .iter()
            .map(|capsule| capsule.bytes.clone())
            .collect::<Vec<_>>(),
        "linked descriptor-install capsule",
    )?;
    let elf_sha = require_same_digest(
        &linked
            .iter()
            .map(|capsule| capsule.elf.clone())
            .collect::<Vec<_>>(),
        "linked descriptor-install ELF",
    )?;
    audit_linked_capsule(&tools, &linked[0], &work)?;
    run_link_negatives(&tools, &linker, &work)?;
    run_proof_negatives(&tools, &source_text, &work)?;

    let report = format!(
        "M1_DESCRIPTOR_INSTALL_OK\ncomponent_verified=true\nrelease_eligible=false\nhardware_executed=false\nsource_sha256={}\ndescriptor_source_sha256={}\nconsumer_source_sha256={}\nlinker_script_sha256={}\nmodel_artifact_sha256={model_sha}\nconsumer_sha256={consumer_sha}\nemitted_capsule_sha256={emitted_sha}\nlinked_capsule_sha256={linked_sha}\nlinked_elf_sha256={elf_sha}\nmodel_reproducibility_builds=3\nconsumer_reproducibility_builds=3\npost_link_reproducibility_builds=3\nverus_verified=19\nmodel_undefined_symbols=core-panic,memcpy\ncapsule_bytes=38\nlinked_virtual=ffffffff80001010\ncaller_requirements=cpl0,interrupts-disabled,asynchronous-quiescence,registered-readable-tables,writable-available-tss,readable-writable-stack,canonical-operands-and-return\nruntime_marker={RUNTIME_MARKER}\nnegative_cases=byte-mutation,unregistered-executable,cs-semantics,tss-busy,rsp-semantics,idtr-semantics,rax-semantics,bad-assume\n",
        sha256sum(&source)?,
        sha256sum(&descriptor_source)?,
        sha256sum(&consumer)?,
        sha256sum(&linker)?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write descriptor-install report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
}

fn audit_sources(
    source: &str,
    descriptors: &str,
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
                "descriptor-install source contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "#![no_std]",
        "pub const CAPSULE_VIRTUAL: u64 = 0xffff_ffff_8000_1010;",
        "pub const FAR_TARGET_VIRTUAL: u64 = CAPSULE_VIRTUAL + 26;",
        "pub const REGISTERED_QWORD0: u64 = 0x0000_0010_b817_010f;",
        "pub const REGISTERED_WORD5: u16 = 0xc31e;",
        "pub open spec fn install_precondition",
        "pub fn decode_execute",
        "pub fn install_registered_tables",
        "!state.interrupts_enabled",
        "state.asynchronous_events_absent",
        "state.tss_descriptor_writable",
        "!state.tss_descriptor_busy",
        "tss_descriptor_busy: true",
        "cs: KERNEL_CODE_SELECTOR",
        "ss: KERNEL_DATA_SELECTOR",
        "tr: TSS_SELECTOR",
        "rsp: state.rsp + 8",
        "rip: state.return_address",
        "ensures result == 255",
    ] {
        if !source.contains(required) {
            return Err(format!("descriptor-install source is missing `{required}`"));
        }
    }
    for required in [
        "pub const KERNEL_CODE_SELECTOR: u16 = 0x08;",
        "pub const KERNEL_DATA_SELECTOR: u16 = 0x10;",
        "pub const TSS_SELECTOR: u16 = 0x28;",
        "pub const GDT_ENTRIES: usize = 7;",
        "pub const IDT_ENTRIES: usize = 256;",
        "pub fn registered_gdt",
        "pub fn registered_idt",
        "pub fn registered_tss",
        "result.iomap_base == 104",
    ] {
        if !descriptors.contains(required) {
            return Err(format!(
                "descriptor-install input source is missing `{required}`"
            ));
        }
    }
    for forbidden in ["unsafe ", "asm!", "global_asm!", "external_body", "assume("] {
        if consumer.contains(forbidden) {
            return Err(format!(
                "descriptor-install consumer contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "assert_eq!(bytes.len(), 38)",
        "install_registered_tables(state())",
        "assert_eq!(installed.state.cs, KERNEL_CODE_SELECTOR)",
        "assert_eq!(installed.state.ss, KERNEL_DATA_SELECTOR)",
        "assert_eq!(installed.state.tr, TSS_SELECTOR)",
        "assert!(installed.state.tss_descriptor_busy)",
        "MachineState { cpl: 3, ..state() }",
        "interrupts_enabled: true",
        "asynchronous_events_absent: false",
        "tss_descriptor_writable: false",
        "M1_DESCRIPTOR_INSTALL_OK bytes=38",
    ] {
        if !consumer.contains(required) {
            return Err(format!(
                "descriptor-install consumer is missing `{required}`"
            ));
        }
    }
    for required in [
        "ENTRY(tmk_descriptor_install_capsule)",
        ". = 0xffffffff80001010;",
        ".text.tmk_descriptor_install_capsule",
        "SIZEOF(.text.tmk_descriptor_install_capsule) == 38",
    ] {
        if !linker.contains(required) {
            return Err(format!("descriptor-install linker is missing `{required}`"));
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
        .args(["--rlimit", "100"])
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
    fs::copy(source, &staged).map_err(|error| format!("stage model: {error}"))?;
    let output = run_checked(
        &mut verus_command(tools, directory, &format!("{CRATE_NAME}.rs"), true),
        "Verus descriptor-install proof and codegen",
    )?;
    require_output_fragments(
        &output.stdout,
        "Verus descriptor-install proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 19",
            "\"errors\": 0",
            "\"version\": \"0.2026.05.24.ecee80a\"",
        ],
    )?;
    if retain_result {
        fs::write(
            directory.join("verus-result.json"),
            canonical_json(&output.stdout, "descriptor-install Verus result")?,
        )
        .map_err(|error| format!("write Verus result: {error}"))?;
    }
    let artifact = directory.join(RLIB);
    require_file(&artifact, "compiled descriptor-install model")?;
    Ok(artifact)
}

fn audit_model_symbols(tools: &Tools, artifact: &Path, work: &Path) -> Result<(), String> {
    let undefined = run_checked(
        Command::new(&tools.nm).arg("-u").arg(artifact),
        "audit descriptor-install undefined symbols",
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
            "descriptor-install model has unexpected undefined symbols: {names:?}"
        ));
    }
    write_combined_output(
        &work.join("model-undefined-symbols.txt"),
        &undefined,
        "model undefined-symbol evidence",
    )?;
    let defined = run_checked(
        Command::new(&tools.nm).arg("--defined-only").arg(artifact),
        "audit descriptor-install defined symbols",
    )?;
    require_output_fragments(
        &defined.stdout,
        "descriptor-install defined symbols",
        &[
            "registered_image",
            "decode_execute",
            "install_registered_tables",
            "registered_install_observation",
        ],
    )?;
    write_combined_output(
        &work.join("model-defined-symbols.txt"),
        &defined,
        "model defined-symbol evidence",
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
        "compile descriptor-install runtime consumer",
    )?;
    Ok(())
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

fn link_capsule(
    tools: &Tools,
    linker: &Path,
    bytes: &Path,
    directory: &Path,
    check_bytes: bool,
) -> Result<LinkedCapsule, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create link path {}: {error}", directory.display()))?;
    fs::copy(bytes, directory.join("capsule.bin"))
        .map_err(|error| format!("stage capsule bytes: {error}"))?;
    wrap_capsule(tools, directory)?;
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
        "link descriptor-install capsule ELF",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(directory).args([
            "--dump-section",
            ".text.tmk_descriptor_install_capsule=linked-capsule.bin",
            "capsule.elf",
        ]),
        "extract linked descriptor-install bytes",
    )?;
    let linked = LinkedCapsule {
        elf: directory.join("capsule.elf"),
        bytes: directory.join("linked-capsule.bin"),
    };
    if check_bytes {
        require_exact_bytes(
            &linked.bytes,
            CAPSULE_BYTES,
            "linked descriptor-install capsule",
        )?;
    }
    Ok(linked)
}

fn wrap_capsule(tools: &Tools, directory: &Path) -> Result<(), String> {
    run_checked(
        Command::new(&tools.ld).current_dir(directory).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "capsule-raw.o",
            "capsule.bin",
        ]),
        "wrap descriptor-install capsule bytes",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(directory).args([
            "--rename-section",
            ".data=.text.tmk_descriptor_install_capsule,alloc,contents,load,readonly,code",
            "--redefine-sym",
            "_binary_capsule_bin_start=tmk_descriptor_install_capsule",
            "--redefine-sym",
            "_binary_capsule_bin_end=tmk_descriptor_install_capsule_end",
            "--redefine-sym",
            "_binary_capsule_bin_size=tmk_descriptor_install_capsule_size",
            "capsule-raw.o",
            "capsule.o",
        ]),
        "name descriptor-install capsule object",
    )?;
    Ok(())
}

fn audit_linked_capsule(tools: &Tools, linked: &LinkedCapsule, work: &Path) -> Result<(), String> {
    let relocations = run_checked(
        Command::new(&tools.readelf).args(["-rW"]).arg(&linked.elf),
        "audit descriptor-install relocations",
    )?;
    require_output_fragments(
        &relocations.stdout,
        "descriptor-install relocations",
        &["There are no relocations in this file"],
    )?;
    write_combined_output(
        &work.join("linked-relocations.txt"),
        &relocations,
        "relocation evidence",
    )?;

    let sections = run_checked(
        Command::new(&tools.readelf).args(["-SW"]).arg(&linked.elf),
        "audit descriptor-install sections",
    )?;
    audit_executable_sections(&sections)?;
    write_combined_output(
        &work.join("linked-sections.txt"),
        &sections,
        "section evidence",
    )?;

    let symbols = run_checked(
        Command::new(&tools.readelf).args(["-sW"]).arg(&linked.elf),
        "audit descriptor-install symbols",
    )?;
    require_output_fragments(
        &symbols.stdout,
        "descriptor-install symbols",
        &[
            "tmk_descriptor_install_capsule_link_start",
            "tmk_descriptor_install_capsule_link_end",
            "tmk_descriptor_install_capsule",
            "ffffffff80001010",
        ],
    )?;
    write_combined_output(
        &work.join("linked-symbols.txt"),
        &symbols,
        "symbol evidence",
    )?;

    let disassembly = run_checked(
        Command::new(&tools.objdump)
            .args(["-d", "-Mintel"])
            .arg(&linked.elf),
        "disassemble descriptor-install capsule",
    )?;
    require_output_fragments(
        &disassembly.stdout,
        "descriptor-install disassembly",
        &[
            "lgdt   [rdi]",
            "mov    eax,0x10",
            "mov    ds,eax",
            "mov    es,eax",
            "mov    ss,eax",
            "push   0x8",
            "lea    rax,[rip+0x3]",
            "push   rax",
            "retfq",
            "mov    eax,0x28",
            "ltr    eax",
            "lidt   [rsi]",
            "ret",
        ],
    )?;
    write_combined_output(
        &work.join("linked-disassembly.txt"),
        &disassembly,
        "disassembly evidence",
    )
}

fn audit_executable_sections(output: &Output) -> Result<(), String> {
    let text = String::from_utf8_lossy(&output.stdout);
    let executable: Vec<_> = text.lines().filter(|line| line.contains(" AX ")).collect();
    if executable.len() == 1 && executable[0].contains(".text.tmk_descriptor_install_capsule") {
        Ok(())
    } else {
        Err(format!(
            "descriptor-install executable section allowlist mismatch: {executable:?}"
        ))
    }
}

fn run_link_negatives(tools: &Tools, linker: &Path, work: &Path) -> Result<(), String> {
    let mutated_dir = work.join("negative-byte-mutation");
    fs::create_dir(&mutated_dir).map_err(|error| format!("create byte negative path: {error}"))?;
    let mut mutated_bytes = CAPSULE_BYTES.to_vec();
    mutated_bytes[0] ^= 1;
    let mutated = mutated_dir.join("mutated.bin");
    fs::write(&mutated, mutated_bytes).map_err(|error| format!("write byte mutation: {error}"))?;
    let linked = link_capsule(tools, linker, &mutated, &mutated_dir, false)?;
    let diagnostic = require_exact_bytes(
        &linked.bytes,
        CAPSULE_BYTES,
        "mutated descriptor-install capsule",
    )
    .expect_err("descriptor-install byte mutation must fail exact-byte audit");
    fs::write(
        work.join("negative-byte-mutation.txt"),
        format!("{diagnostic}\n"),
    )
    .map_err(|error| format!("write byte negative evidence: {error}"))?;

    let extra_dir = work.join("negative-unregistered-executable");
    fs::create_dir(&extra_dir).map_err(|error| format!("create extra-section path: {error}"))?;
    fs::write(extra_dir.join("capsule.bin"), CAPSULE_BYTES)
        .map_err(|error| format!("write base bytes: {error}"))?;
    fs::write(extra_dir.join("extra.bin"), [0x90])
        .map_err(|error| format!("write extra byte: {error}"))?;
    wrap_capsule(tools, &extra_dir)?;
    run_checked(
        Command::new(&tools.ld).current_dir(&extra_dir).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "extra-raw.o",
            "extra.bin",
        ]),
        "wrap unregistered descriptor-install byte",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(&extra_dir).args([
            "--rename-section",
            ".data=.text.unregistered,alloc,contents,load,readonly,code",
            "extra-raw.o",
            "extra.o",
        ]),
        "classify unregistered descriptor-install section",
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
        "link ELF with unregistered executable section",
    )?;
    let sections = run_checked(
        Command::new(&tools.readelf)
            .current_dir(&extra_dir)
            .args(["-SW", "extra.elf"]),
        "inspect unregistered executable section",
    )?;
    let diagnostic = audit_executable_sections(&sections)
        .expect_err("unregistered executable section must fail audit");
    fs::write(
        work.join("negative-unregistered-executable.txt"),
        format!("{diagnostic}\n"),
    )
    .map_err(|error| format!("write section negative evidence: {error}"))?;
    Ok(())
}

fn run_proof_negatives(tools: &Tools, source: &str, work: &Path) -> Result<(), String> {
    let cases = [
        (
            "cs-semantics",
            "cs: KERNEL_CODE_SELECTOR,",
            "cs: KERNEL_DATA_SELECTOR,",
            "postcondition not satisfied",
        ),
        (
            "tss-busy",
            "tss_descriptor_busy: true,",
            "tss_descriptor_busy: false,",
            "postcondition not satisfied",
        ),
        (
            "rsp-semantics",
            "rsp: state.rsp + 8,",
            "rsp: state.rsp,",
            "postcondition not satisfied",
        ),
        (
            "idtr-semantics",
            "idtr_limit: state.idtr_operand_limit,",
            "idtr_limit: state.idtr_limit,",
            "postcondition not satisfied",
        ),
        (
            "rax-semantics",
            "rax: TSS_SELECTOR as u64,",
            "rax: state.rax,",
            "postcondition not satisfied",
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
        "    if image.qword0 == REGISTERED_QWORD0",
        "    assume(false);\n    if image.qword0 == REGISTERED_QWORD0",
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
        return Err(format!(
            "descriptor-install negative `{name}` target is not unique"
        ));
    }
    let directory = work.join(format!("negative-{name}"));
    fs::create_dir(&directory).map_err(|error| format!("create negative {name} path: {error}"))?;
    let staged = directory.join(format!("{CRATE_NAME}.rs"));
    fs::write(&staged, source.replacen(needle, replacement, 1))
        .map_err(|error| format!("write negative {name}: {error}"))?;
    let output = run_expect_failure(
        &mut verus_command(tools, &directory, &format!("{CRATE_NAME}.rs"), false),
        &format!("reject descriptor-install {name} mutation"),
    )?;
    let mut combined = Vec::new();
    combined.extend_from_slice(&output.stdout);
    combined.extend_from_slice(&output.stderr);
    require_output_fragments(
        &combined,
        &format!("descriptor-install {name} negative"),
        &[diagnostic],
    )?;
    if combined
        .windows(b"\"success\": true".len())
        .any(|window| window == b"\"success\": true")
        || directory.join(RLIB).exists()
    {
        return Err(format!(
            "descriptor-install {name} mutation did not fail atomically"
        ));
    }
    write_combined_output(
        &work.join(format!("negative-{name}.txt")),
        &output,
        &format!("descriptor-install {name} negative evidence"),
    )
}
