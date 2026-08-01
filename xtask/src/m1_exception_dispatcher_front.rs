use super::{
    canonical_json, require_exact_bytes, require_file, require_output_fragments, run_checked,
    run_expect_failure, sha256sum, workspace_root, write_combined_output,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SOURCE: &str = "verus/machine-model/exception_dispatcher_front_capsule.rs";
const COMMON_SOURCE: &str = "verus/machine-model/exception_common_capsule.rs";
const FRAME_SOURCE: &str = "tests/m1/exception_frame_shell.rs";
const CONSUMER: &str = "tests/m1/exception_dispatcher_front_consumer.rs";
const LINKER: &str = "kernel-host/link/m1_exception_dispatcher_front_capsule.ld";
const CRATE_NAME: &str = "tmk_exception_dispatcher_front";
const RLIB: &str = "libtmk_exception_dispatcher_front.rlib";
const RUNTIME_MARKER: &str = "M1_EXCEPTION_DISPATCHER_FRONT_OK bytes=93 user_words=8 kernel_words=6 metadata=001b00230000000e scalar_entry_mod16=8 tail=1";
const EXPECTED_BYTES: [u8; 93] = [
    0x49, 0x89, 0xfa, 0x49, 0x8b, 0x7a, 0x70, 0x49, 0x8b, 0xb2, 0x88, 0x00, 0x00, 0x00, 0x49, 0x8b,
    0x92, 0x90, 0x00, 0x00, 0x00, 0x49, 0x8b, 0x8a, 0xa0, 0x00, 0x00, 0x00, 0x4d, 0x8b, 0x8a, 0x80,
    0x00, 0x00, 0x00, 0x4d, 0x8b, 0x9a, 0x98, 0x00, 0x00, 0x00, 0x41, 0xf6, 0xc3, 0x03, 0x74, 0x1e,
    0x4d, 0x8b, 0x82, 0xa8, 0x00, 0x00, 0x00, 0x49, 0x8b, 0x82, 0xb0, 0x00, 0x00, 0x00, 0x48, 0xc1,
    0xe0, 0x30, 0x49, 0xc1, 0xe3, 0x20, 0x4d, 0x09, 0xd9, 0x49, 0x09, 0xc1, 0xeb, 0x0a, 0x45, 0x31,
    0xc0, 0x49, 0xc1, 0xe3, 0x20, 0x4d, 0x09, 0xd9, 0xe9, 0xa3, 0x00, 0x00, 0x00,
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
    let tools = Tools::pinned()?;
    let source = root.join(SOURCE);
    let common_source = root.join(COMMON_SOURCE);
    let frame_source = root.join(FRAME_SOURCE);
    let consumer = root.join(CONSUMER);
    let linker = root.join(LINKER);
    for (path, label) in [
        (&source, "dispatcher-front Verus source"),
        (&common_source, "common-entry Verus source"),
        (&frame_source, "safe frame-decoder source"),
        (&consumer, "dispatcher-front consumer"),
        (&linker, "dispatcher-front linker"),
    ] {
        require_file(path, label)?;
    }
    let source_text = read(&source)?;
    let common_text = read(&common_source)?;
    let frame_text = read(&frame_source)?;
    let consumer_text = read(&consumer)?;
    let linker_text = read(&linker)?;
    audit_sources(
        &source_text,
        &common_text,
        &frame_text,
        &consumer_text,
        &linker_text,
    )?;

    let work = root.join("build/m1-exception-dispatcher-front");
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
    let model_sha = same_digest(&artifacts, "dispatcher-front model")?;
    audit_model(&tools, &artifacts[0], &work)?;

    let mut consumers = Vec::new();
    let mut emitted = Vec::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let executable = work.join(format!("consumer-{}", index + 1));
        let bytes = work.join(format!("dispatcher-{}.bin", index + 1));
        compile_consumer(&tools, &root, &consumer, artifact, &executable)?;
        let runtime = run_checked(
            Command::new(&executable).current_dir(&root).arg(&bytes),
            "execute dispatcher-front model and emit bytes",
        )?;
        require_output_fragments(
            &runtime.stdout,
            "dispatcher-front runtime",
            &[RUNTIME_MARKER],
        )?;
        require_exact_bytes(&bytes, &EXPECTED_BYTES, "emitted dispatcher-front capsule")?;
        write_combined_output(
            &work.join(format!("runtime-{}.txt", index + 1)),
            &runtime,
            "dispatcher-front runtime evidence",
        )?;
        consumers.push(executable);
        emitted.push(bytes);
    }
    let consumer_sha = same_digest(&consumers, "dispatcher-front consumer")?;
    let emitted_sha = same_digest(&emitted, "emitted dispatcher-front capsule")?;

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
        "linked dispatcher-front capsule",
    )?;
    let elf_sha = same_digest(
        &linked
            .iter()
            .map(|item| item.elf.clone())
            .collect::<Vec<_>>(),
        "linked dispatcher-front ELF",
    )?;
    audit_linked(&tools, &linked[0], &work)?;
    run_link_negatives(&tools, &linker, &emitted[0], &work)?;
    run_proof_negatives(&tools, &source_text, &work)?;

    let report = format!(
        "M1_EXCEPTION_DISPATCHER_FRONT_OK\ncomponent_verified=true\nrelease_eligible=false\nhardware_executed=false\nscalar_body_present=false\nsafe_decoder_joined=false\nsource_sha256={}\ncommon_source_sha256={}\nframe_source_sha256={}\nconsumer_source_sha256={}\nlinker_script_sha256={}\nmodel_artifact_sha256={model_sha}\nconsumer_sha256={consumer_sha}\nemitted_capsule_sha256={emitted_sha}\nlinked_capsule_sha256={linked_sha}\nlinked_elf_sha256={elf_sha}\nmodel_reproducibility_builds=3\nconsumer_reproducibility_builds=3\npost_link_reproducibility_builds=3\nverus_verified=22\nmodel_undefined_symbols=core-panic,memcpy\ncapsule_bytes=93\ndispatcher_virtual=ffffffff80011100\nscalar_seam_virtual=ffffffff80011200\nscalar_return_virtual=ffffffff80011038\nscalar_transfer=tail-jump\nscalar_abi=cr2,error,rip,rflags,user-rsp-or-zero,vector-cs-ss-metadata\nframe_offsets=cr2:112,vector:128,error:136,rip:144,cs:152,rflags:160,rsp:168,ss:176\nframe_words_read=user:8,kernel:6\nscalar_stack_alignment=dispatcher-entry:8,scalar-entry:8\ncaller_requirements=cpl0,if-clear,df-clear,registered-readable-prefix,conditional-readable-user-tail,registered-readable-nonoverlapping-exact-common-continuation-return-address,registered-returning-frame-and-rbx-preserving-scalar\nruntime_marker={RUNTIME_MARKER}\nnegative_cases=byte-mutation,unregistered-executable,cr2-argument,error-argument,user-rsp,metadata,scalar-target,frame-unchanged,stack-alignment,tail-transfer,return-target,word-count,bad-assume\n",
        sha256sum(&source)?,
        sha256sum(&common_source)?,
        sha256sum(&frame_source)?,
        sha256sum(&consumer)?,
        sha256sum(&linker)?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write dispatcher-front report: {error}"))?;
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
    frame: &str,
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
                "dispatcher-front source contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "#![no_std]",
        "pub const DISPATCHER_VIRTUAL: u64 = 0xffff_ffff_8001_1100;",
        "pub const SCALAR_SEAM_VIRTUAL: u64 = 0xffff_ffff_8001_1200;",
        "pub const QWORD0: u64 = 0x4970_7a8b_49fa_8949;",
        "pub const QWORD10: u64 = 0xd909_4d20_e3c1_49c0;",
        "pub const TAIL: u64 = 0x0000_0000_00a3_e9;",
        "pub open spec fn dispatcher_front_precondition",
        "pub open spec fn packed_metadata",
        "pub fn decode_execute",
        "state.rdi == state.frame.base",
        "state.prefix_readable",
        "from_user(&state.frame) ==> state.user_tail_readable",
        "state.scalar_preserves_rbx",
        "state.scalar_preserves_frame",
        "state.rsp & 15 == 8",
        "state.rsp <= state.frame.base - 8",
        "state.return_address == 0xffff_ffff_8001_1038",
        "result.frame_memory_unchanged",
        "result.frame_words_read == if from_user(&state.frame) { 8u8 } else { 6u8 }",
        "result.scalar_entry_rsp & 15 == 8",
        "result.post_rsp == state.rsp + 8",
        "result.post_rip == state.return_address",
        "result.scalar_tail_transfer",
        "result.return_address_consumed",
        "ensures result == 1023",
    ] {
        if !source.contains(required) {
            return Err(format!("dispatcher-front source is missing `{required}`"));
        }
    }
    for required in [
        "pub const DISPATCHER_VIRTUAL: u64 = 0xffff_ffff_8001_1100;",
        "dispatcher_frame: state.rsp - 128",
        "state.dispatcher_preserves_rbx",
        "state.dispatcher_preserves_frame",
    ] {
        if !common.contains(required) {
            return Err(format!(
                "common-entry binding source is missing `{required}`"
            ));
        }
    }
    for required in [
        "words.len() >= 15 ==> result.cr2 == words@[14]",
        "words.len() >= 17 ==> result.vector == if words@[16] <= 255",
        "words.len() >= 18 ==> result.error == words@[17]",
        "words.len() >= 20 && words@[19] == USER_CODE_SELECTOR",
        "spec_kernel_address(words@[18])",
        "spec_return_flags(words@[20])",
        "spec_user_address(words@[21])",
        "words@[22] == USER_DATA_SELECTOR",
        "pub fn normalize_exception_event",
    ] {
        if !frame.contains(required) {
            return Err(format!(
                "safe frame-decoder binding is missing `{required}`"
            ));
        }
    }
    for forbidden in ["unsafe ", "asm!", "global_asm!", "external_body", "assume("] {
        if consumer.contains(forbidden) {
            return Err(format!(
                "dispatcher-front consumer contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "assert_eq!(bytes.len(), 93)",
        "decode_execute(registered_image(), state(true))",
        "assert_eq!(user.frame_words_read, 8)",
        "assert_eq!(kernel.frame_words_read, 6)",
        "prefix_readable: false",
        "user_tail_readable: false",
        "scalar_registered: false",
        "scalar_return_address_readable: false",
        "return_address: 0xffff_ffff_8001_1039",
        "direction_flag: true",
        "vector: 256",
        "user_ss: 0x1_0000",
        "rsp: 0xffff_e000_0000_2e70",
        "rsp: 0xffff_e000_0000_2e88",
        "M1_EXCEPTION_DISPATCHER_FRONT_OK bytes=93",
    ] {
        if !consumer.contains(required) {
            return Err(format!("dispatcher-front consumer is missing `{required}`"));
        }
    }
    for required in [
        "ENTRY(tmk_exception_dispatcher)",
        ". = 0xffffffff80011100;",
        ".text.tmk_exception_dispatcher",
        "tmk_exception_scalar = 0xffffffff80011200;",
        "SIZEOF(.text.tmk_exception_dispatcher) == 93",
    ] {
        if !linker.contains(required) {
            return Err(format!("dispatcher-front linker is missing `{required}`"));
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
    fs::copy(source, &staged).map_err(|error| format!("stage dispatcher-front model: {error}"))?;
    let output = run_checked(
        &mut verus_command(tools, directory, &format!("{CRATE_NAME}.rs"), true),
        "Verus dispatcher-front proof and codegen",
    )?;
    require_output_fragments(
        &output.stdout,
        "Verus dispatcher-front proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 22",
            "\"errors\": 0",
            "\"is-verifying-entire-crate\": true",
            "\"version\": \"0.2026.05.24.ecee80a\"",
        ],
    )?;
    if retain_result {
        fs::write(
            directory.join("verus-result.json"),
            canonical_json(&output.stdout, "dispatcher-front Verus result")?,
        )
        .map_err(|error| format!("write dispatcher-front Verus result: {error}"))?;
    }
    let artifact = directory.join(RLIB);
    require_file(&artifact, "compiled dispatcher-front model")?;
    Ok(artifact)
}

fn audit_model(tools: &Tools, artifact: &Path, work: &Path) -> Result<(), String> {
    let undefined = run_checked(
        Command::new(&tools.nm).arg("-u").arg(artifact),
        "audit dispatcher-front undefined symbols",
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
            "dispatcher-front undefined symbols changed: {names:?}"
        ));
    }
    write_combined_output(
        &work.join("model-undefined-symbols.txt"),
        &undefined,
        "undefined-symbol evidence",
    )?;
    let defined = run_checked(
        Command::new(&tools.nm).arg("--defined-only").arg(artifact),
        "audit dispatcher-front defined symbols",
    )?;
    require_output_fragments(
        &defined.stdout,
        "dispatcher-front defined symbols",
        &[
            "registered_image",
            "decode_execute",
            "dispatcher_front_observation",
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
        "compile dispatcher-front consumer",
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
    fs::copy(bytes, directory.join("dispatcher.bin"))
        .map_err(|error| format!("stage dispatcher-front bytes: {error}"))?;
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
            .args(["-o", "dispatcher.elf", "dispatcher.o"]),
        "link dispatcher-front ELF",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(directory).args([
            "--dump-section",
            ".text.tmk_exception_dispatcher=linked-dispatcher.bin",
            "dispatcher.elf",
        ]),
        "extract linked dispatcher-front bytes",
    )?;
    let linked = LinkedCapsule {
        elf: directory.join("dispatcher.elf"),
        bytes: directory.join("linked-dispatcher.bin"),
    };
    require_exact_bytes(
        &linked.bytes,
        &EXPECTED_BYTES,
        "linked dispatcher-front capsule",
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
            "dispatcher-raw.o",
            "dispatcher.bin",
        ]),
        "wrap dispatcher-front bytes",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(directory).args([
            "--rename-section",
            ".data=.text.tmk_exception_dispatcher,alloc,contents,load,readonly,code",
            "--redefine-sym",
            "_binary_dispatcher_bin_start=tmk_exception_dispatcher",
            "--redefine-sym",
            "_binary_dispatcher_bin_end=tmk_exception_dispatcher_end",
            "--redefine-sym",
            "_binary_dispatcher_bin_size=tmk_exception_dispatcher_size",
            "dispatcher-raw.o",
            "dispatcher.o",
        ]),
        "name dispatcher-front object",
    )?;
    Ok(())
}

fn audit_linked(tools: &Tools, linked: &LinkedCapsule, work: &Path) -> Result<(), String> {
    let relocations = run_checked(
        Command::new(&tools.readelf).args(["-rW"]).arg(&linked.elf),
        "audit dispatcher-front relocations",
    )?;
    require_output_fragments(
        &relocations.stdout,
        "dispatcher-front relocations",
        &["There are no relocations in this file"],
    )?;
    write_combined_output(
        &work.join("linked-relocations.txt"),
        &relocations,
        "relocation evidence",
    )?;

    let sections = run_checked(
        Command::new(&tools.readelf).args(["-SW"]).arg(&linked.elf),
        "audit dispatcher-front sections",
    )?;
    audit_sections(&sections)?;
    write_combined_output(
        &work.join("linked-sections.txt"),
        &sections,
        "section evidence",
    )?;

    let symbols = run_checked(
        Command::new(&tools.readelf).args(["-sW"]).arg(&linked.elf),
        "audit dispatcher-front symbols",
    )?;
    require_output_fragments(
        &symbols.stdout,
        "dispatcher-front symbols",
        &[
            "tmk_exception_dispatcher",
            "tmk_exception_dispatcher_end",
            "tmk_exception_scalar",
            "ffffffff80011100",
            "ffffffff80011200",
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
        "disassemble dispatcher-front capsule",
    )?;
    require_output_fragments(
        &disassembly.stdout,
        "dispatcher-front disassembly",
        &[
            "mov    r10,rdi",
            "mov    rdi,QWORD PTR [r10+0x70]",
            "mov    rsi,QWORD PTR [r10+0x88]",
            "mov    rdx,QWORD PTR [r10+0x90]",
            "mov    rcx,QWORD PTR [r10+0xa0]",
            "mov    r9,QWORD PTR [r10+0x80]",
            "mov    r11,QWORD PTR [r10+0x98]",
            "test   r11b,0x3",
            "mov    r8,QWORD PTR [r10+0xa8]",
            "mov    rax,QWORD PTR [r10+0xb0]",
            "shl    rax,0x30",
            "shl    r11,0x20",
            "or     r9,r11",
            "or     r9,rax",
            "xor    r8d,r8d",
            "jmp    ffffffff80011200",
        ],
    )?;
    let text = String::from_utf8_lossy(&disassembly.stdout);
    if text.matches("jmp    ffffffff80011200").count() != 1 {
        return Err("dispatcher-front does not have exactly one scalar tail jump".to_string());
    }
    for forbidden in ["push", "pop", "call", "ret"] {
        if text.contains(forbidden) {
            return Err(format!(
                "dispatcher-front unexpectedly contains `{forbidden}`"
            ));
        }
    }
    for forbidden in ["rbx", "rbp", "r12", "r13", "r14", "r15"] {
        if text.contains(forbidden) {
            return Err(format!(
                "dispatcher-front unexpectedly touches callee-saved `{forbidden}`"
            ));
        }
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
    if executable.len() == 1 && executable[0].contains(".text.tmk_exception_dispatcher") {
        Ok(())
    } else {
        Err(format!(
            "dispatcher-front executable section mismatch: {executable:?}"
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
    let mut mutated = fs::read(valid_bytes)
        .map_err(|error| format!("read valid dispatcher-front bytes: {error}"))?;
    mutated[6] ^= 1;
    fs::write(byte_dir.join("dispatcher.bin"), &mutated)
        .map_err(|error| format!("write byte mutation: {error}"))?;
    let diagnostic = require_exact_bytes(
        &byte_dir.join("dispatcher.bin"),
        &EXPECTED_BYTES,
        "mutated dispatcher-front capsule",
    )
    .expect_err("dispatcher-front byte mutation must fail");
    fs::write(
        work.join("negative-byte-mutation.txt"),
        format!("{diagnostic}\n"),
    )
    .map_err(|error| format!("write byte negative: {error}"))?;

    let extra_dir = work.join("negative-unregistered-executable");
    fs::create_dir(&extra_dir).map_err(|error| format!("create section negative: {error}"))?;
    fs::copy(valid_bytes, extra_dir.join("dispatcher.bin"))
        .map_err(|error| format!("stage dispatcher-front bytes: {error}"))?;
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
        "wrap extra dispatcher-front byte",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(&extra_dir).args([
            "--rename-section",
            ".data=.text.unregistered,alloc,contents,load,readonly,code",
            "extra-raw.o",
            "extra.o",
        ]),
        "classify extra dispatcher-front byte",
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
            .args(["-o", "extra.elf", "dispatcher.o", "extra.o"]),
        "link extra dispatcher-front section",
    )?;
    let sections = run_checked(
        Command::new(&tools.readelf)
            .current_dir(&extra_dir)
            .args(["-SW", "extra.elf"]),
        "inspect extra dispatcher-front section",
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
            "cr2-argument",
            "                cr2: state.frame.cr2,",
            "                cr2: 0,",
        ),
        (
            "error-argument",
            "                error: state.frame.error,",
            "                error: 0,",
        ),
        (
            "user-rsp",
            "                user_rsp: if user { state.frame.user_rsp } else { 0 },",
            "                user_rsp: 0,",
        ),
        (
            "metadata",
            "                metadata,",
            "                metadata: 0,",
        ),
        (
            "scalar-target",
            "            scalar_address: SCALAR_SEAM_VIRTUAL,",
            "            scalar_address: 0,",
        ),
        (
            "frame-unchanged",
            "            frame_memory_unchanged: true,",
            "            frame_memory_unchanged: false,",
        ),
        (
            "stack-alignment",
            "            scalar_stack_aligned: true,",
            "            scalar_stack_aligned: false,",
        ),
        (
            "tail-transfer",
            "            scalar_tail_transfer: true,",
            "            scalar_tail_transfer: false,",
        ),
        (
            "return-target",
            "            post_rip: state.return_address,",
            "            post_rip: 0,",
        ),
        (
            "word-count",
            "            frame_words_read: if user { 8 } else { 6 },",
            "            frame_words_read: if user { 7 } else { 5 },",
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
                "dispatcher-front negative `{name}` target is not unique"
            ));
        }
        let directory = work.join(format!("negative-{name}"));
        fs::create_dir(&directory).map_err(|error| format!("create negative {name}: {error}"))?;
        let staged = directory.join(format!("{CRATE_NAME}.rs"));
        fs::write(&staged, source.replacen(needle, replacement, 1))
            .map_err(|error| format!("write negative {name}: {error}"))?;
        let output = run_expect_failure(
            &mut verus_command(tools, &directory, &format!("{CRATE_NAME}.rs"), false),
            &format!("reject dispatcher-front {name} mutation"),
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
            return Err(format!("dispatcher-front {name} did not fail atomically"));
        }
        write_combined_output(
            &work.join(format!("negative-{name}.txt")),
            &output,
            &format!("dispatcher-front {name} negative"),
        )?;
    }
    Ok(())
}
