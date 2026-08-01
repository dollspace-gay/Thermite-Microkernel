use super::{
    canonical_json, require_exact_bytes, require_file, require_output_fragments, run_checked,
    run_expect_failure, sha256sum, workspace_root, write_combined_output,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SOURCE: &str = "verus/machine-model/exception_common_capsule.rs";
const STUB_SOURCE: &str = "verus/machine-model/exception_stub_table.rs";
const CONSUMER: &str = "tests/m1/exception_common_capsule_consumer.rs";
const LINKER: &str = "kernel-host/link/m1_exception_common_capsule.ld";
const CRATE_NAME: &str = "tmk_exception_common_capsule";
const RLIB: &str = "libtmk_exception_common_capsule.rlib";
const RUNTIME_MARKER: &str = "M1_EXCEPTION_COMMON_OK bytes=105 vector=14 cr2=0000000012345000 frame=ffffe00000002e80 swapgs=2 iret_cpl=3";

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
    let stub_source = root.join(STUB_SOURCE);
    let consumer = root.join(CONSUMER);
    let linker = root.join(LINKER);
    for (path, label) in [
        (&source, "common-entry Verus source"),
        (&stub_source, "exception-stub Verus source"),
        (&consumer, "common-entry consumer"),
        (&linker, "common-entry linker"),
    ] {
        require_file(path, label)?;
    }
    let source_text = read(&source)?;
    let stub_text = read(&stub_source)?;
    let consumer_text = read(&consumer)?;
    let linker_text = read(&linker)?;
    audit_sources(&source_text, &stub_text, &consumer_text, &linker_text)?;

    let work = root.join("build/m1-exception-common");
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
    let model_sha = same_digest(&artifacts, "common-entry model")?;
    audit_model(&tools, &artifacts[0], &work)?;

    let mut consumers = Vec::new();
    let mut emitted = Vec::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let executable = work.join(format!("consumer-{}", index + 1));
        let bytes = work.join(format!("common-{}.bin", index + 1));
        compile_consumer(&tools, &root, &consumer, artifact, &executable)?;
        let runtime = run_checked(
            Command::new(&executable).current_dir(&root).arg(&bytes),
            "execute common-entry model and emit bytes",
        )?;
        require_output_fragments(&runtime.stdout, "common-entry runtime", &[RUNTIME_MARKER])?;
        if fs::metadata(&bytes)
            .map_err(|error| format!("inspect {}: {error}", bytes.display()))?
            .len()
            != 105
        {
            return Err("emitted common-entry capsule is not 105 bytes".to_string());
        }
        write_combined_output(
            &work.join(format!("runtime-{}.txt", index + 1)),
            &runtime,
            "common-entry runtime evidence",
        )?;
        consumers.push(executable);
        emitted.push(bytes);
    }
    let consumer_sha = same_digest(&consumers, "common-entry consumer")?;
    let emitted_sha = same_digest(&emitted, "emitted common-entry capsule")?;

    let link_dirs = [
        work.join("link-primary"),
        work.join("link-repro-a"),
        work.join("link-repro-b"),
    ];
    let mut linked = Vec::new();
    for (directory, bytes) in link_dirs.iter().zip(emitted.iter()) {
        linked.push(link_capsule(&tools, &linker, bytes, directory)?);
    }
    let linked_sha = same_digest(
        &linked
            .iter()
            .map(|item| item.bytes.clone())
            .collect::<Vec<_>>(),
        "linked common-entry capsule",
    )?;
    let elf_sha = same_digest(
        &linked
            .iter()
            .map(|item| item.elf.clone())
            .collect::<Vec<_>>(),
        "linked common-entry ELF",
    )?;
    audit_linked(&tools, &linked[0], &work)?;
    run_link_negatives(&tools, &linker, &emitted[0], &work)?;
    run_proof_negatives(&tools, &source_text, &work)?;

    let report = format!(
        "M1_EXCEPTION_COMMON_OK\ncomponent_verified=true\nrelease_eligible=false\nhardware_executed=false\ndispatcher_body_present=false\nsource_sha256={}\nstub_source_sha256={}\nconsumer_source_sha256={}\nlinker_script_sha256={}\nmodel_artifact_sha256={model_sha}\nconsumer_sha256={consumer_sha}\nemitted_capsule_sha256={emitted_sha}\nlinked_capsule_sha256={linked_sha}\nlinked_elf_sha256={elf_sha}\nmodel_reproducibility_builds=3\nconsumer_reproducibility_builds=3\npost_link_reproducibility_builds=3\nverus_verified=27\nmodel_undefined_symbols=core-panic,memcpy\ncapsule_bytes=105\ncommon_entry_virtual=ffffffff80011000\ndispatcher_virtual=ffffffff80011100\ncaller_requirements=cpl0,interrupt-gate-if-clear,normalized-frame,151-byte-valid-entry-stack,canonical-resume-state,valid-return-rflags,registered-returning-frame-preserving-dispatcher,gs-mode-match\nruntime_marker={RUNTIME_MARKER}\nnegative_cases=byte-mutation,unregistered-executable,cr2-capture,gpr-restore,swapgs,df-clear,resume-rsp,bad-assume\n",
        sha256sum(&source)?,
        sha256sum(&stub_source)?,
        sha256sum(&consumer)?,
        sha256sum(&linker)?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write common-entry report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
}

fn audit_sources(source: &str, stubs: &str, consumer: &str, linker: &str) -> Result<(), String> {
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
                "common-entry source contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "#![no_std]",
        "pub const COMMON_ENTRY_VIRTUAL: u64 = 0xffff_ffff_8001_1000;",
        "pub const DISPATCHER_VIRTUAL: u64 = 0xffff_ffff_8001_1100;",
        "pub const QWORD0: u64 = 0x5251_5350_d020_0f50;",
        "pub const QWORD12: u64 = 0x4810_c483_4858_08c4;",
        "pub const TAIL: u8 = 0xcf;",
        "pub open spec fn common_precondition",
        "pub open spec fn return_rflags_valid",
        "pub const RETURN_RFLAGS_ALLOWED: u64 = 0x0025_0fd7;",
        "pub fn decode_execute",
        "state.dispatcher_preserves_rbx",
        "state.dispatcher_preserves_frame",
        "captured_cr2: state.cr2",
        "dispatcher_frame: state.rsp - 128",
        "dispatcher_df_clear: true",
        "swapgs_count: if returning_user { 2 } else { 0 }",
        "rsp: state.resume_rsp",
        "rip: state.resume_rip",
        "ensures result == 255",
    ] {
        if !source.contains(required) {
            return Err(format!("common-entry source is missing `{required}`"));
        }
    }
    for required in [
        "pub const COMMON_ENTRY_VIRTUAL: u64 = 0xffff_ffff_8001_1000;",
        "spec_stub_jump_target(stub, vector) == COMMON_ENTRY_VIRTUAL as int",
        "pub fn registered_stub_table()",
    ] {
        if !stubs.contains(required) {
            return Err(format!("common-entry stub input is missing `{required}`"));
        }
    }
    for forbidden in ["unsafe ", "asm!", "global_asm!", "external_body", "assume("] {
        if consumer.contains(forbidden) {
            return Err(format!(
                "common-entry consumer contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "assert_eq!(encoded.len(), 105)",
        "decode_execute(registered_image(), state())",
        "assert_eq!(user.captured_cr2, 0x0000_1234_5000)",
        "assert_eq!(user.swapgs_count, 2)",
        "assert_eq!(kernel.swapgs_count, 0)",
        "dispatcher_registered: false",
        "dispatcher_preserves_rbx: false",
        "resume_rflags: 0x3002",
        "M1_EXCEPTION_COMMON_OK bytes=105",
    ] {
        if !consumer.contains(required) {
            return Err(format!("common-entry consumer is missing `{required}`"));
        }
    }
    for required in [
        "ENTRY(tmk_exception_common_entry)",
        ". = 0xffffffff80011000;",
        ".text.tmk_exception_common",
        "tmk_exception_dispatcher = 0xffffffff80011100;",
        "SIZEOF(.text.tmk_exception_common) == 105",
    ] {
        if !linker.contains(required) {
            return Err(format!("common-entry linker is missing `{required}`"));
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
        .args(["--rlimit", "120"])
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
    fs::copy(source, &staged).map_err(|error| format!("stage common-entry model: {error}"))?;
    let output = run_checked(
        &mut verus_command(tools, directory, &format!("{CRATE_NAME}.rs"), true),
        "Verus common-entry proof and codegen",
    )?;
    require_output_fragments(
        &output.stdout,
        "Verus common-entry proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 27",
            "\"errors\": 0",
            "\"version\": \"0.2026.05.24.ecee80a\"",
        ],
    )?;
    if retain_result {
        fs::write(
            directory.join("verus-result.json"),
            canonical_json(&output.stdout, "common-entry Verus result")?,
        )
        .map_err(|error| format!("write common-entry Verus result: {error}"))?;
    }
    let artifact = directory.join(RLIB);
    require_file(&artifact, "compiled common-entry model")?;
    Ok(artifact)
}

fn audit_model(tools: &Tools, artifact: &Path, work: &Path) -> Result<(), String> {
    let undefined = run_checked(
        Command::new(&tools.nm).arg("-u").arg(artifact),
        "audit common-entry undefined symbols",
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
        return Err(format!("common-entry undefined symbols changed: {names:?}"));
    }
    write_combined_output(
        &work.join("model-undefined-symbols.txt"),
        &undefined,
        "undefined-symbol evidence",
    )?;
    let defined = run_checked(
        Command::new(&tools.nm).arg("--defined-only").arg(artifact),
        "audit common-entry defined symbols",
    )?;
    require_output_fragments(
        &defined.stdout,
        "common-entry defined symbols",
        &[
            "registered_image",
            "decode_execute",
            "common_entry_observation",
        ],
    )?;
    write_combined_output(
        &work.join("model-defined-symbols.txt"),
        &defined,
        "defined-symbol evidence",
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
        "compile common-entry consumer",
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

fn link_capsule(
    tools: &Tools,
    linker: &Path,
    bytes: &Path,
    directory: &Path,
) -> Result<LinkedCapsule, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create link path {}: {error}", directory.display()))?;
    fs::copy(bytes, directory.join("common.bin"))
        .map_err(|error| format!("stage common-entry bytes: {error}"))?;
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
            .args(["-o", "common.elf", "common.o"]),
        "link common-entry ELF",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(directory).args([
            "--dump-section",
            ".text.tmk_exception_common=linked-common.bin",
            "common.elf",
        ]),
        "extract linked common-entry bytes",
    )?;
    let linked = LinkedCapsule {
        elf: directory.join("common.elf"),
        bytes: directory.join("linked-common.bin"),
    };
    require_exact_bytes(
        &linked.bytes,
        &fs::read(bytes).map_err(|error| error.to_string())?,
        "linked common-entry capsule",
    )?;
    Ok(linked)
}

fn wrap_capsule(tools: &Tools, directory: &Path) -> Result<(), String> {
    run_checked(
        Command::new(&tools.ld).current_dir(directory).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "common-raw.o",
            "common.bin",
        ]),
        "wrap common-entry bytes",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(directory).args([
            "--rename-section",
            ".data=.text.tmk_exception_common,alloc,contents,load,readonly,code",
            "--redefine-sym",
            "_binary_common_bin_start=tmk_exception_common_entry",
            "--redefine-sym",
            "_binary_common_bin_end=tmk_exception_common_entry_end",
            "--redefine-sym",
            "_binary_common_bin_size=tmk_exception_common_entry_size",
            "common-raw.o",
            "common.o",
        ]),
        "name common-entry object",
    )?;
    Ok(())
}

fn audit_linked(tools: &Tools, linked: &LinkedCapsule, work: &Path) -> Result<(), String> {
    let relocations = run_checked(
        Command::new(&tools.readelf).args(["-rW"]).arg(&linked.elf),
        "audit common-entry relocations",
    )?;
    require_output_fragments(
        &relocations.stdout,
        "common-entry relocations",
        &["There are no relocations in this file"],
    )?;
    write_combined_output(
        &work.join("linked-relocations.txt"),
        &relocations,
        "relocation evidence",
    )?;
    let sections = run_checked(
        Command::new(&tools.readelf).args(["-SW"]).arg(&linked.elf),
        "audit common-entry sections",
    )?;
    audit_sections(&sections)?;
    write_combined_output(
        &work.join("linked-sections.txt"),
        &sections,
        "section evidence",
    )?;
    let symbols = run_checked(
        Command::new(&tools.readelf).args(["-sW"]).arg(&linked.elf),
        "audit common-entry symbols",
    )?;
    require_output_fragments(
        &symbols.stdout,
        "common-entry symbols",
        &[
            "tmk_exception_common_entry",
            "tmk_exception_common_link_start",
            "tmk_exception_common_link_end",
            "tmk_exception_dispatcher",
            "ffffffff80011000",
            "ffffffff80011100",
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
        "disassemble common-entry capsule",
    )?;
    require_output_fragments(
        &disassembly.stdout,
        "common-entry disassembly",
        &[
            "mov    rax,cr2",
            "test   BYTE PTR [rsp+0x98],0x3",
            "swapgs",
            "cld",
            "mov    rdi,rsp",
            "mov    rbx,rsp",
            "and    rsp,0xfffffffffffffff0",
            "call   ffffffff80011100",
            "mov    rsp,rbx",
            "pop    r15",
            "pop    rax",
            "add    rsp,0x10",
            "iretq",
        ],
    )?;
    let text = String::from_utf8_lossy(&disassembly.stdout);
    if text.matches("swapgs").count() != 2 {
        return Err(
            "common-entry disassembly does not have exactly two SWAPGS instructions".to_string(),
        );
    }
    write_combined_output(
        &work.join("linked-disassembly.txt"),
        &disassembly,
        "disassembly evidence",
    )
}

fn audit_sections(output: &Output) -> Result<(), String> {
    let text = String::from_utf8_lossy(&output.stdout);
    let executable: Vec<_> = text.lines().filter(|line| line.contains(" AX ")).collect();
    if executable.len() == 1 && executable[0].contains(".text.tmk_exception_common") {
        Ok(())
    } else {
        Err(format!(
            "common-entry executable section mismatch: {executable:?}"
        ))
    }
}

fn run_link_negatives(
    tools: &Tools,
    linker: &Path,
    valid_bytes: &Path,
    work: &Path,
) -> Result<(), String> {
    let byte_dir = work.join("negative-byte-mutation");
    fs::create_dir(&byte_dir).map_err(|error| format!("create byte negative: {error}"))?;
    let expected = fs::read(valid_bytes).map_err(|error| format!("read valid bytes: {error}"))?;
    let mut mutated = expected.clone();
    mutated[1] ^= 1;
    fs::write(byte_dir.join("common.bin"), &mutated)
        .map_err(|error| format!("write byte mutation: {error}"))?;
    let diagnostic = require_exact_bytes(
        &byte_dir.join("common.bin"),
        &expected,
        "mutated common-entry capsule",
    )
    .expect_err("common-entry byte mutation must fail");
    fs::write(
        work.join("negative-byte-mutation.txt"),
        format!("{diagnostic}\n"),
    )
    .map_err(|error| format!("write byte negative: {error}"))?;

    let extra_dir = work.join("negative-unregistered-executable");
    fs::create_dir(&extra_dir).map_err(|error| format!("create section negative: {error}"))?;
    fs::copy(valid_bytes, extra_dir.join("common.bin"))
        .map_err(|error| format!("stage common bytes: {error}"))?;
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
        "wrap extra common-entry byte",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(&extra_dir).args([
            "--rename-section",
            ".data=.text.unregistered,alloc,contents,load,readonly,code",
            "extra-raw.o",
            "extra.o",
        ]),
        "classify extra common-entry byte",
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
            .args(["-o", "extra.elf", "common.o", "extra.o"]),
        "link extra common-entry section",
    )?;
    let sections = run_checked(
        Command::new(&tools.readelf)
            .current_dir(&extra_dir)
            .args(["-SW", "extra.elf"]),
        "inspect extra common-entry section",
    )?;
    let diagnostic = audit_sections(&sections).expect_err("extra executable section must fail");
    fs::write(
        work.join("negative-unregistered-executable.txt"),
        format!("{diagnostic}\n"),
    )
    .map_err(|error| format!("write section negative: {error}"))?;
    Ok(())
}

fn run_proof_negatives(tools: &Tools, source: &str, work: &Path) -> Result<(), String> {
    let cases = [
        (
            "cr2-capture",
            "captured_cr2: state.cr2,",
            "captured_cr2: 0,",
        ),
        ("gpr-restore", "r15: state.r15,", "r15: state.r14,"),
        (
            "swapgs",
            "swapgs_count: if returning_user { 2 } else { 0 },",
            "swapgs_count: 0,",
        ),
        (
            "df-clear",
            "dispatcher_df_clear: true,",
            "dispatcher_df_clear: false,",
        ),
        (
            "resume-rsp",
            "                rsp: state.resume_rsp,\n                rip: state.resume_rip,",
            "                rsp: state.rsp,\n                rip: state.resume_rip,",
        ),
        (
            "bad-assume",
            "    if image.qword0 == QWORD0",
            "    assume(false);\n    if image.qword0 == QWORD0",
        ),
    ];
    for (name, needle, replacement) in cases {
        if source.matches(needle).count() != 1 {
            return Err(format!(
                "common-entry negative `{name}` target is not unique"
            ));
        }
        let directory = work.join(format!("negative-{name}"));
        fs::create_dir(&directory).map_err(|error| format!("create negative {name}: {error}"))?;
        let staged = directory.join(format!("{CRATE_NAME}.rs"));
        fs::write(&staged, source.replacen(needle, replacement, 1))
            .map_err(|error| format!("write negative {name}: {error}"))?;
        let output = run_expect_failure(
            &mut verus_command(tools, &directory, &format!("{CRATE_NAME}.rs"), false),
            &format!("reject common-entry {name} mutation"),
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
            return Err(format!("common-entry {name} did not fail atomically"));
        }
        write_combined_output(
            &work.join(format!("negative-{name}.txt")),
            &output,
            &format!("common-entry {name} negative"),
        )?;
    }
    Ok(())
}
