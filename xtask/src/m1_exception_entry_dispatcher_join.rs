use super::{
    canonical_json, require_file, require_output_fragments, run_checked, run_expect_failure,
    sha256sum, workspace_root, write_combined_output,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SOURCE: &str = "verus/machine-model/exception_entry_dispatcher_join.rs";
const COMMON_SOURCE: &str = "verus/machine-model/exception_common_capsule.rs";
const DISPATCHER_SOURCE: &str = "verus/machine-model/exception_dispatcher_front_capsule.rs";
const CONSUMER: &str = "tests/m1/exception_entry_dispatcher_join_consumer.rs";
const LINKER: &str = "kernel-host/link/m1_exception_entry_dispatcher_join.ld";
const CRATE_NAME: &str = "tmk_exception_entry_dispatcher_join";
const RLIB: &str = "libtmk_exception_entry_dispatcher_join.rlib";
const RUNTIME_MARKER: &str = "M1_EXCEPTION_ENTRY_DISPATCHER_JOIN_OK common=105 dispatcher=93 user_rsp=ffffe00000002e78 kernel_rsp=ffffe00000003e78 alignment=8 continuation=ffffffff80011038 rejects=13";

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

struct LinkedJoin {
    elf: PathBuf,
    common: PathBuf,
    dispatcher: PathBuf,
}

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let tools = Tools::pinned()?;
    let source = root.join(SOURCE);
    let common_source = root.join(COMMON_SOURCE);
    let dispatcher_source = root.join(DISPATCHER_SOURCE);
    let consumer = root.join(CONSUMER);
    let linker = root.join(LINKER);
    for (path, label) in [
        (&source, "entry/dispatcher join Verus source"),
        (&common_source, "accepted common-entry source"),
        (&dispatcher_source, "accepted dispatcher-front source"),
        (&consumer, "entry/dispatcher join consumer"),
        (&linker, "entry/dispatcher join linker"),
    ] {
        require_file(path, label)?;
    }
    let source_text = read(&source)?;
    let common_text = read(&common_source)?;
    let dispatcher_text = read(&dispatcher_source)?;
    let consumer_text = read(&consumer)?;
    let linker_text = read(&linker)?;
    audit_sources(
        &source_text,
        &common_text,
        &dispatcher_text,
        &consumer_text,
        &linker_text,
    )?;

    super::m1_exception_common::run()?;
    super::m1_exception_dispatcher_front::run()?;

    let work = root.join("build/m1-exception-entry-dispatcher-join");
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
    let model_sha = same_digest(&artifacts, "entry/dispatcher join model")?;
    audit_model(&tools, &artifacts[0], &work)?;

    let mut consumers = Vec::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let executable = work.join(format!("consumer-{}", index + 1));
        compile_consumer(&tools, &root, &consumer, artifact, &executable)?;
        let output = run_checked(
            Command::new(&executable).current_dir(&root),
            "execute entry/dispatcher join consumer",
        )?;
        require_output_fragments(
            &output.stdout,
            "entry/dispatcher join runtime",
            &[RUNTIME_MARKER],
        )?;
        write_combined_output(
            &work.join(format!("runtime-{}.txt", index + 1)),
            &output,
            "entry/dispatcher join runtime evidence",
        )?;
        consumers.push(executable);
    }
    let consumer_sha = same_digest(&consumers, "entry/dispatcher join consumer")?;

    let common_bins = [
        root.join("build/m1-exception-common/common-1.bin"),
        root.join("build/m1-exception-common/common-2.bin"),
        root.join("build/m1-exception-common/common-3.bin"),
    ];
    let dispatcher_bins = [
        root.join("build/m1-exception-dispatcher-front/dispatcher-1.bin"),
        root.join("build/m1-exception-dispatcher-front/dispatcher-2.bin"),
        root.join("build/m1-exception-dispatcher-front/dispatcher-3.bin"),
    ];
    for path in common_bins.iter().chain(dispatcher_bins.iter()) {
        require_file(path, "accepted component byte image")?;
    }

    let link_dirs = [
        work.join("link-primary"),
        work.join("link-repro-a"),
        work.join("link-repro-b"),
    ];
    let mut linked = Vec::new();
    for ((directory, common), dispatcher) in link_dirs
        .iter()
        .zip(common_bins.iter())
        .zip(dispatcher_bins.iter())
    {
        linked.push(link_join(&tools, &linker, common, dispatcher, directory)?);
    }
    let elf_sha = same_digest(
        &linked
            .iter()
            .map(|item| item.elf.clone())
            .collect::<Vec<_>>(),
        "joined entry/dispatcher ELF",
    )?;
    let joined_common_sha = same_digest(
        &linked
            .iter()
            .map(|item| item.common.clone())
            .collect::<Vec<_>>(),
        "joined common-entry bytes",
    )?;
    let joined_dispatcher_sha = same_digest(
        &linked
            .iter()
            .map(|item| item.dispatcher.clone())
            .collect::<Vec<_>>(),
        "joined dispatcher-front bytes",
    )?;
    compare_bytes(
        &common_bins[0],
        &linked[0].common,
        "joined common-entry identity",
    )?;
    compare_bytes(
        &dispatcher_bins[0],
        &linked[0].dispatcher,
        "joined dispatcher-front identity",
    )?;
    audit_linked(&tools, &linked[0], &work)?;
    run_artifact_negatives(&tools, &linker, &common_bins[0], &dispatcher_bins[0], &work)?;
    run_proof_negatives(&tools, &source_text, &work)?;

    let report = format!(
        "M1_EXCEPTION_ENTRY_DISPATCHER_JOIN_OK\ncomponent_verified=true\nrelease_eligible=false\nhardware_executed=false\nscalar_body_present=false\ncommon_obligations_discharged=true\nsource_sha256={}\ncommon_source_sha256={}\ndispatcher_source_sha256={}\nconsumer_source_sha256={}\nlinker_script_sha256={}\nmodel_artifact_sha256={model_sha}\nconsumer_sha256={consumer_sha}\njoined_common_sha256={joined_common_sha}\njoined_dispatcher_sha256={joined_dispatcher_sha}\njoined_elf_sha256={elf_sha}\nmodel_reproducibility_builds=3\nconsumer_reproducibility_builds=3\npost_link_reproducibility_builds=3\nverus_verified=27\nmodel_undefined_symbols=core-panic,memcpy\ncommon_bytes=105\ndispatcher_bytes=93\ncommon_virtual=ffffffff80011000\ndispatcher_virtual=ffffffff80011100\nscalar_virtual=ffffffff80011200\ncontinuation_virtual=ffffffff80011038\nentry_alignment_cases=0,8\ndispatcher_alignment=8\nframe_bounds=low:entry-rsp-144,high:entry-rsp+40-or-56\njoined_properties=rdi-frame-identity,conditional-tail-readability,df-clear,exact-return-address,nonoverlap,scalar-alignment,frame-rbx-preservation,iret-resume\nruntime_marker={RUNTIME_MARKER}\nnegative_cases=common-byte,dispatcher-byte,extra-executable,frame-base,dispatcher-rsp,return-address,metadata,df-clear,tail-transfer,final-rip,bad-assume\n",
        sha256sum(&source)?,
        sha256sum(&common_source)?,
        sha256sum(&dispatcher_source)?,
        sha256sum(&consumer)?,
        sha256sum(&linker)?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write entry/dispatcher join report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
}

fn audit_sources(
    source: &str,
    common: &str,
    dispatcher: &str,
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
                "entry/dispatcher join source contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "#![no_std]",
        "pub const COMMON_CONTINUATION: u64 = 0xffff_ffff_8001_1038;",
        "pub open spec fn entry_join_precondition",
        "state.stack_low <= state.rsp - 144",
        "state.stack_high >= state.rsp + normalized_bytes(state)",
        "pub open spec fn dispatcher_stack",
        "pub fn decode_execute_join",
        "result.dispatcher_rsp & 15 == 8",
        "result.dispatcher_rsp <= result.frame_base - 8",
        "result.return_address == COMMON_CONTINUATION",
        "result.dispatcher_precondition_established",
        "result.dispatcher_df_clear",
        "result.scalar_tail_transfer",
        "result.frame_unchanged",
        "result.rbx_preserved",
        "result.final_rip == state.resume_rip",
        "ensures result == 4095",
    ] {
        if !source.contains(required) {
            return Err(format!(
                "entry/dispatcher join source is missing `{required}`"
            ));
        }
    }
    for required in [
        "pub const COMMON_ENTRY_VIRTUAL: u64 = 0xffff_ffff_8001_1000;",
        "dispatcher_frame: state.rsp - 128",
        "dispatcher_df_clear: true",
        "state.dispatcher_preserves_frame",
    ] {
        if !common.contains(required) {
            return Err(format!(
                "common-entry binding source is missing `{required}`"
            ));
        }
    }
    for required in [
        "pub const DISPATCHER_VIRTUAL: u64 = 0xffff_ffff_8001_1100;",
        "state.rdi == state.frame.base",
        "state.rsp & 15 == 8",
        "state.return_address == 0xffff_ffff_8001_1038",
        "result.scalar_tail_transfer",
    ] {
        if !dispatcher.contains(required) {
            return Err(format!(
                "dispatcher-front binding source is missing `{required}`"
            ));
        }
    }
    for forbidden in ["unsafe ", "asm!", "global_asm!", "external_body", "assume("] {
        if consumer.contains(forbidden) {
            return Err(format!(
                "entry/dispatcher join consumer contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "assert_eq!(user.dispatcher_rsp, 0xffff_e000_0000_2e78)",
        "assert_eq!(kernel.dispatcher_rsp, 0xffff_e000_0000_3e78)",
        "assert!(user.dispatcher_df_clear)",
        "invalid.stack_low = invalid.rsp - 143",
        "invalid.stack_high = invalid.rsp + 55",
        "image.common_last ^= 1",
        "image.dispatcher_tail ^= 1",
        "M1_EXCEPTION_ENTRY_DISPATCHER_JOIN_OK common=105 dispatcher=93",
    ] {
        if !consumer.contains(required) {
            return Err(format!(
                "entry/dispatcher join consumer is missing `{required}`"
            ));
        }
    }
    for required in [
        "ENTRY(tmk_exception_common)",
        ". = 0xffffffff80011000;",
        ". = 0xffffffff80011100;",
        "tmk_exception_scalar = 0xffffffff80011200;",
        "SIZEOF(.text.tmk_exception_common) == 105",
        "SIZEOF(.text.tmk_exception_dispatcher) == 93",
    ] {
        if !linker.contains(required) {
            return Err(format!(
                "entry/dispatcher join linker is missing `{required}`"
            ));
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
    fs::copy(source, &staged).map_err(|error| format!("stage join model: {error}"))?;
    let output = run_checked(
        &mut verus_command(tools, directory, &format!("{CRATE_NAME}.rs"), true),
        "Verus entry/dispatcher join proof and codegen",
    )?;
    require_output_fragments(
        &output.stdout,
        "Verus entry/dispatcher join proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 27",
            "\"errors\": 0",
            "\"is-verifying-entire-crate\": true",
            "\"version\": \"0.2026.05.24.ecee80a\"",
        ],
    )?;
    if retain_result {
        fs::write(
            directory.join("verus-result.json"),
            canonical_json(&output.stdout, "entry/dispatcher join Verus result")?,
        )
        .map_err(|error| format!("write entry/dispatcher join Verus result: {error}"))?;
    }
    let artifact = directory.join(RLIB);
    require_file(&artifact, "compiled entry/dispatcher join model")?;
    Ok(artifact)
}

fn audit_model(tools: &Tools, artifact: &Path, work: &Path) -> Result<(), String> {
    let undefined = run_checked(
        Command::new(&tools.nm).arg("-u").arg(artifact),
        "audit entry/dispatcher join undefined symbols",
    )?;
    let text = String::from_utf8_lossy(&undefined.stdout);
    let names: BTreeSet<_> = text
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .filter(|name| !name.ends_with(':'))
        .collect();
    if names.len() != 2
        || !names.contains("memcpy")
        || !names.iter().any(|name| name.ends_with("5panic"))
    {
        return Err(format!(
            "entry/dispatcher join undefined symbols changed: {names:?}"
        ));
    }
    write_combined_output(
        &work.join("model-undefined-symbols.txt"),
        &undefined,
        "entry/dispatcher join undefined symbols",
    )?;
    let defined = run_checked(
        Command::new(&tools.nm).arg("--defined-only").arg(artifact),
        "audit entry/dispatcher join defined symbols",
    )?;
    require_output_fragments(
        &defined.stdout,
        "entry/dispatcher join defined symbols",
        &[
            "registered_image",
            "decode_execute_join",
            "entry_dispatcher_join_observation",
        ],
    )?;
    write_combined_output(
        &work.join("model-defined-symbols.txt"),
        &defined,
        "entry/dispatcher join defined symbols",
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
        "compile entry/dispatcher join consumer",
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

fn compare_bytes(expected: &Path, actual: &Path, label: &str) -> Result<(), String> {
    let expected_bytes =
        fs::read(expected).map_err(|error| format!("read {}: {error}", expected.display()))?;
    let actual_bytes =
        fs::read(actual).map_err(|error| format!("read {}: {error}", actual.display()))?;
    if expected_bytes == actual_bytes {
        Ok(())
    } else {
        Err(format!("{label} differs"))
    }
}

fn wrap_binary(
    tools: &Tools,
    directory: &Path,
    input: &str,
    stem: &str,
    section: &str,
    symbol: &str,
) -> Result<(), String> {
    run_checked(
        Command::new(&tools.ld)
            .current_dir(directory)
            .args(["-r", "-b", "binary", "-o"])
            .arg(format!("{stem}-raw.o"))
            .arg(input),
        "wrap joined exception bytes",
    )?;
    let binary_symbol = input.replace(['.', '-'], "_");
    run_checked(
        Command::new(&tools.objcopy)
            .current_dir(directory)
            .arg("--rename-section")
            .arg(format!(".data={section},alloc,contents,load,readonly,code"))
            .args(["--redefine-sym"])
            .arg(format!("_binary_{binary_symbol}_start={symbol}"))
            .args(["--redefine-sym"])
            .arg(format!("_binary_{binary_symbol}_end={symbol}_end"))
            .args(["--redefine-sym"])
            .arg(format!("_binary_{binary_symbol}_size={symbol}_size"))
            .arg(format!("{stem}-raw.o"))
            .arg(format!("{stem}.o")),
        "classify joined exception bytes",
    )?;
    Ok(())
}

fn link_join(
    tools: &Tools,
    linker: &Path,
    common: &Path,
    dispatcher: &Path,
    directory: &Path,
) -> Result<LinkedJoin, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create link path {}: {error}", directory.display()))?;
    fs::copy(common, directory.join("common.bin"))
        .map_err(|error| format!("stage common-entry bytes: {error}"))?;
    fs::copy(dispatcher, directory.join("dispatcher.bin"))
        .map_err(|error| format!("stage dispatcher-front bytes: {error}"))?;
    wrap_binary(
        tools,
        directory,
        "common.bin",
        "common",
        ".text.tmk_exception_common",
        "tmk_exception_common",
    )?;
    wrap_binary(
        tools,
        directory,
        "dispatcher.bin",
        "dispatcher",
        ".text.tmk_exception_dispatcher",
        "tmk_exception_dispatcher",
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
            .args(["-o", "joined.elf", "common.o", "dispatcher.o"]),
        "link joined exception entry/dispatcher ELF",
    )?;
    for (section, output) in [
        (".text.tmk_exception_common", "linked-common.bin"),
        (".text.tmk_exception_dispatcher", "linked-dispatcher.bin"),
    ] {
        run_checked(
            Command::new(&tools.objcopy)
                .current_dir(directory)
                .arg("--dump-section")
                .arg(format!("{section}={output}"))
                .arg("joined.elf"),
            "extract joined exception bytes",
        )?;
    }
    let linked = LinkedJoin {
        elf: directory.join("joined.elf"),
        common: directory.join("linked-common.bin"),
        dispatcher: directory.join("linked-dispatcher.bin"),
    };
    compare_bytes(common, &linked.common, "post-link common-entry image")?;
    compare_bytes(
        dispatcher,
        &linked.dispatcher,
        "post-link dispatcher-front image",
    )?;
    Ok(linked)
}

fn audit_linked(tools: &Tools, linked: &LinkedJoin, work: &Path) -> Result<(), String> {
    let relocations = run_checked(
        Command::new(&tools.readelf).args(["-rW"]).arg(&linked.elf),
        "audit joined exception relocations",
    )?;
    require_output_fragments(
        &relocations.stdout,
        "joined exception relocations",
        &["There are no relocations in this file"],
    )?;
    write_combined_output(
        &work.join("joined-relocations.txt"),
        &relocations,
        "joined exception relocation evidence",
    )?;
    let sections = run_checked(
        Command::new(&tools.readelf).args(["-SW"]).arg(&linked.elf),
        "audit joined exception sections",
    )?;
    audit_sections(&sections)?;
    write_combined_output(
        &work.join("joined-sections.txt"),
        &sections,
        "joined exception section evidence",
    )?;
    let symbols = run_checked(
        Command::new(&tools.readelf).args(["-sW"]).arg(&linked.elf),
        "audit joined exception symbols",
    )?;
    require_output_fragments(
        &symbols.stdout,
        "joined exception symbols",
        &[
            "tmk_exception_common",
            "tmk_exception_dispatcher",
            "tmk_exception_scalar",
            "ffffffff80011000",
            "ffffffff80011100",
            "ffffffff80011200",
        ],
    )?;
    write_combined_output(
        &work.join("joined-symbols.txt"),
        &symbols,
        "joined exception symbol evidence",
    )?;
    let disassembly = run_checked(
        Command::new(&tools.objdump)
            .args(["-d", "-Mintel"])
            .arg(&linked.elf),
        "disassemble joined exception entry/dispatcher",
    )?;
    require_output_fragments(
        &disassembly.stdout,
        "joined exception disassembly",
        &[
            "call   ffffffff80011100",
            "ffffffff80011038:",
            "mov    rsp,rbx",
            "mov    r10,rdi",
            "jmp    ffffffff80011200",
            "iretq",
        ],
    )?;
    let text = String::from_utf8_lossy(&disassembly.stdout);
    if text.matches("call   ffffffff80011100").count() != 1
        || text.matches("jmp    ffffffff80011200").count() != 1
        || text.matches("iretq").count() != 1
    {
        return Err("joined exception control-transfer cardinality changed".to_string());
    }
    write_combined_output(
        &work.join("joined-disassembly.txt"),
        &disassembly,
        "joined exception disassembly evidence",
    )
}

fn audit_sections(output: &Output) -> Result<(), String> {
    let text = String::from_utf8_lossy(&output.stdout);
    let executable: Vec<_> = text.lines().filter(|line| line.contains(" AX ")).collect();
    if executable.len() == 2
        && executable
            .iter()
            .any(|line| line.contains(".text.tmk_exception_common"))
        && executable
            .iter()
            .any(|line| line.contains(".text.tmk_exception_dispatcher"))
    {
        Ok(())
    } else {
        Err(format!(
            "joined exception executable sections changed: {executable:?}"
        ))
    }
}

fn run_artifact_negatives(
    tools: &Tools,
    linker: &Path,
    common: &Path,
    dispatcher: &Path,
    work: &Path,
) -> Result<(), String> {
    for (name, valid, index) in [
        ("common-byte", common, 8usize),
        ("dispatcher-byte", dispatcher, 6usize),
    ] {
        let directory = work.join(format!("negative-{name}"));
        fs::create_dir(&directory).map_err(|error| format!("create {name} negative: {error}"))?;
        let mut bytes = fs::read(valid).map_err(|error| format!("read {name} bytes: {error}"))?;
        bytes[index] ^= 1;
        let mutated = directory.join("mutated.bin");
        fs::write(&mutated, bytes).map_err(|error| format!("write {name} mutation: {error}"))?;
        let diagnostic = compare_bytes(valid, &mutated, name)
            .expect_err("joined exception byte mutation must fail");
        fs::write(
            work.join(format!("negative-{name}.txt")),
            format!("{diagnostic}\n"),
        )
        .map_err(|error| format!("write {name} negative: {error}"))?;
    }

    let extra = work.join("negative-extra-executable");
    let linked = link_join(tools, linker, common, dispatcher, &extra)?;
    fs::write(extra.join("extra.bin"), [0x90])
        .map_err(|error| format!("write extra executable byte: {error}"))?;
    run_checked(
        Command::new(&tools.ld).current_dir(&extra).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "extra-raw.o",
            "extra.bin",
        ]),
        "wrap extra joined executable byte",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(&extra).args([
            "--rename-section",
            ".data=.text.unregistered,alloc,contents,load,readonly,code",
            "extra-raw.o",
            "extra.o",
        ]),
        "classify extra joined executable byte",
    )?;
    run_checked(
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
            .args(["-o", "extra.elf", "common.o", "dispatcher.o", "extra.o"]),
        "link extra joined exception section",
    )?;
    let sections = run_checked(
        Command::new(&tools.readelf)
            .current_dir(&extra)
            .args(["-SW", "extra.elf"]),
        "inspect extra joined exception section",
    )?;
    let diagnostic = audit_sections(&sections).expect_err("extra joined executable must fail");
    fs::write(
        work.join("negative-extra-executable.txt"),
        format!("{diagnostic}\n"),
    )
    .map_err(|error| format!("write extra executable negative: {error}"))?;
    require_file(&linked.elf, "baseline joined ELF for section negative")?;
    Ok(())
}

fn run_proof_negatives(tools: &Tools, source: &str, work: &Path) -> Result<(), String> {
    let cases = [
        (
            "frame-base",
            "            frame_base: base,",
            "            frame_base: 0,",
        ),
        (
            "dispatcher-rsp",
            "            dispatcher_rsp: call_rsp,",
            "            dispatcher_rsp: 0,",
        ),
        (
            "return-address",
            "            return_address: COMMON_CONTINUATION,",
            "            return_address: 0,",
        ),
        (
            "metadata",
            "                metadata: state.vector | (state.resume_cs << 32)",
            "                metadata: 0 | (state.resume_cs << 32)",
        ),
        (
            "df-clear",
            "            dispatcher_df_clear: true,",
            "            dispatcher_df_clear: false,",
        ),
        (
            "tail-transfer",
            "            scalar_tail_transfer: true,",
            "            scalar_tail_transfer: false,",
        ),
        (
            "final-rip",
            "            final_rip: state.resume_rip,",
            "            final_rip: 0,",
        ),
        (
            "bad-assume",
            "    if image.common_first == COMMON_QWORD0",
            "    assume(false);\n    if image.common_first == COMMON_QWORD0",
        ),
    ];
    for (name, needle, replacement) in cases {
        if source.matches(needle).count() != 1 {
            return Err(format!(
                "entry/dispatcher negative `{name}` target is not unique"
            ));
        }
        let directory = work.join(format!("negative-{name}"));
        fs::create_dir(&directory)
            .map_err(|error| format!("create proof negative {name}: {error}"))?;
        let staged = directory.join(format!("{CRATE_NAME}.rs"));
        fs::write(&staged, source.replacen(needle, replacement, 1))
            .map_err(|error| format!("write proof negative {name}: {error}"))?;
        let output = run_expect_failure(
            &mut verus_command(tools, &directory, &format!("{CRATE_NAME}.rs"), false),
            &format!("reject entry/dispatcher {name} mutation"),
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
            return Err(format!("entry/dispatcher {name} did not fail atomically"));
        }
        write_combined_output(
            &work.join(format!("negative-{name}.txt")),
            &output,
            &format!("entry/dispatcher {name} negative"),
        )?;
    }
    Ok(())
}
