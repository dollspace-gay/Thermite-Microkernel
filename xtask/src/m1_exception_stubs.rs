use super::{
    canonical_json, require_exact_bytes, require_file, require_output_fragments, run_checked,
    run_expect_failure, sha256sum, workspace_root, write_combined_output,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SOURCE: &str = "verus/machine-model/exception_stub_table.rs";
const DESCRIPTOR_SOURCE: &str = "verus/platform/descriptor_tables.rs";
const CONSUMER: &str = "tests/m1/exception_stub_table_consumer.rs";
const LINKER: &str = "kernel-host/link/m1_exception_stub_table.ld";
const CRATE_NAME: &str = "tmk_exception_stub_table";
const RLIB: &str = "libtmk_exception_stub_table.rlib";
const RUNTIME_MARKER: &str = "M1_EXCEPTION_STUBS_OK observation=255 vectors=256 error=10 synthetic=246 bytes=4096 target=ffffffff80011000";

struct Tools {
    verus: PathBuf,
    rustc: PathBuf,
    ar: PathBuf,
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
            ar: PathBuf::from("/usr/sbin/ar"),
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
                tools.ar.as_path(),
                "a21151402078c113fd801d16e0a0d2659ee44cee1b9828474f937bbf097b0df6",
                "GNU ar",
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

struct LinkedTable {
    elf: PathBuf,
    bytes: PathBuf,
}

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let tools = Tools::pinned()?;
    let source = root.join(SOURCE);
    let descriptors = root.join(DESCRIPTOR_SOURCE);
    let consumer = root.join(CONSUMER);
    let linker = root.join(LINKER);
    for (path, label) in [
        (&source, "exception-stub Verus source"),
        (&descriptors, "descriptor-table source"),
        (&consumer, "exception-stub consumer"),
        (&linker, "exception-stub linker"),
    ] {
        require_file(path, label)?;
    }
    let source_text = read(&source)?;
    let descriptor_text = read(&descriptors)?;
    let consumer_text = read(&consumer)?;
    let linker_text = read(&linker)?;
    audit_sources(&source_text, &descriptor_text, &consumer_text, &linker_text)?;

    let work = root.join("build/m1-exception-stubs");
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
    let model_sha = same_digest(&artifacts, "exception-stub model")?;
    audit_model(&tools, &artifacts[0], &work)?;

    let mut consumers = Vec::new();
    let mut emitted = Vec::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let executable = work.join(format!("consumer-{}", index + 1));
        let bytes = work.join(format!("stubs-{}.bin", index + 1));
        compile_consumer(&tools, &root, &consumer, artifact, &executable)?;
        let runtime = run_checked(
            Command::new(&executable).current_dir(&root).arg(&bytes),
            "execute exception-stub consumer",
        )?;
        require_output_fragments(&runtime.stdout, "exception-stub runtime", &[RUNTIME_MARKER])?;
        require_file(&bytes, "emitted exception-stub table")?;
        if fs::metadata(&bytes)
            .map_err(|error| format!("inspect {}: {error}", bytes.display()))?
            .len()
            != 4096
        {
            return Err("emitted exception-stub table is not 4096 bytes".to_string());
        }
        write_combined_output(
            &work.join(format!("runtime-{}.txt", index + 1)),
            &runtime,
            "exception-stub runtime evidence",
        )?;
        consumers.push(executable);
        emitted.push(bytes);
    }
    let consumer_sha = same_digest(&consumers, "exception-stub consumer")?;
    let emitted_sha = same_digest(&emitted, "emitted exception-stub table")?;

    let link_dirs = [
        work.join("link-primary"),
        work.join("link-repro-a"),
        work.join("link-repro-b"),
    ];
    let mut linked = Vec::new();
    for (directory, bytes) in link_dirs.iter().zip(emitted.iter()) {
        linked.push(link_table(&tools, &linker, bytes, directory)?);
    }
    let linked_sha = same_digest(
        &linked
            .iter()
            .map(|table| table.bytes.clone())
            .collect::<Vec<_>>(),
        "linked exception-stub table",
    )?;
    let elf_sha = same_digest(
        &linked
            .iter()
            .map(|table| table.elf.clone())
            .collect::<Vec<_>>(),
        "linked exception-stub ELF",
    )?;
    audit_linked(&tools, &linked[0], &work)?;
    run_link_negatives(&tools, &linker, &emitted[0], &work)?;
    run_proof_negatives(&tools, &source_text, &work)?;
    run_no_vstd_boundary(&tools, &source, &work)?;

    let report = format!(
        "M1_EXCEPTION_STUBS_OK\ncomponent_verified=true\nrelease_eligible=false\nhardware_executed=false\ncommon_entry_body_present=false\nsource_sha256={}\ndescriptor_source_sha256={}\nconsumer_source_sha256={}\nlinker_script_sha256={}\nmodel_artifact_sha256={model_sha}\nconsumer_sha256={consumer_sha}\nemitted_table_sha256={emitted_sha}\nlinked_table_sha256={linked_sha}\nlinked_elf_sha256={elf_sha}\nmodel_reproducibility_builds=3\nconsumer_reproducibility_builds=3\npost_link_reproducibility_builds=3\nverus_verified=20\nmodel_undefined_symbols=core-panic,core-panic-bounds-check,memcpy\nvectors=256\ncpu_error_vectors=10\nsynthetic_error_vectors=246\ntable_bytes=4096\nstub_bytes=16\ntable_virtual=ffffffff80010000\ncommon_entry_virtual=ffffffff80011000\nproof_library=vstd-array-spec-only\nruntime_marker={RUNTIME_MARKER}\nnegative_cases=byte-mutation,unregistered-executable,error-classification,displacement,synthetic-opcode,cpu-error-opcode,table-completeness,observation,bad-assume,vstd-proof-dependency\n",
        sha256sum(&source)?,
        sha256sum(&descriptors)?,
        sha256sum(&consumer)?,
        sha256sum(&linker)?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write exception-stub report: {error}"))?;
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
                "exception-stub source contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "#![no_std]",
        "use vstd::array::ArrayAdditionalSpecFns;",
        "pub const VECTOR_COUNT: usize = 256;",
        "pub const STUB_TABLE_VIRTUAL: u64 = 0xffff_ffff_8001_0000;",
        "pub const COMMON_ENTRY_VIRTUAL: u64 = 0xffff_ffff_8001_1000;",
        "#[repr(C, align(4096))]",
        "pub open spec fn cpu_pushes_error_code",
        "vector == 8",
        "vector == 14",
        "vector == 30",
        "pub open spec fn registered_stub",
        "pub fn registered_stub_image",
        "pub fn registered_stub_table",
        "while slot < VECTOR_COUNT",
        "forall|index: int| 0 <= index < slot",
        "0xe900_0000_0068_006au64",
        "0x0000_e900_0000_0068u64",
        "ensures result == 255",
    ] {
        if !source.contains(required) {
            return Err(format!("exception-stub source is missing `{required}`"));
        }
    }
    for required in [
        "pub const HANDLER_BASE: u64 = 0xffff_ffff_8001_0000;",
        "pub const HANDLER_STRIDE: u64 = 16;",
        "pub const IDT_ENTRIES: usize = 256;",
        "pub fn registered_idt()",
    ] {
        if !descriptors.contains(required) {
            return Err(format!(
                "exception-stub descriptor input is missing `{required}`"
            ));
        }
    }
    for forbidden in ["unsafe ", "asm!", "global_asm!", "external_body", "assume("] {
        if consumer.contains(forbidden) {
            return Err(format!(
                "exception-stub consumer contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "std::mem::size_of::<StubTable>()",
        "std::mem::align_of::<StubTable>()",
        "for (vector, stub) in table.entries.iter().enumerate()",
        "assert_eq!(cpu_error, 10)",
        "assert_eq!(synthetic, 246)",
        "assert_eq!(bytes.len(), 4096)",
        "assert_eq!(stub_jump_target(stub, vector as u16), COMMON_ENTRY_VIRTUAL)",
        "M1_EXCEPTION_STUBS_OK observation={observation}",
    ] {
        if !consumer.contains(required) {
            return Err(format!("exception-stub consumer is missing `{required}`"));
        }
    }
    for required in [
        "ENTRY(tmk_exception_stub_table)",
        ". = 0xffffffff80010000;",
        ".text.tmk_exception_stubs",
        "tmk_exception_common_entry = 0xffffffff80011000;",
        "SIZEOF(.text.tmk_exception_stubs) == 4096",
        "tmk_exception_stub_table_link_end == tmk_exception_common_entry",
    ] {
        if !linker.contains(required) {
            return Err(format!("exception-stub linker is missing `{required}`"));
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
    fs::copy(source, &staged).map_err(|error| format!("stage exception-stub model: {error}"))?;
    let output = run_checked(
        &mut verus_command(tools, directory, &format!("{CRATE_NAME}.rs"), true, false),
        "Verus exception-stub proof and codegen",
    )?;
    require_output_fragments(
        &output.stdout,
        "Verus exception-stub proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 20",
            "\"errors\": 0",
            "\"version\": \"0.2026.05.24.ecee80a\"",
        ],
    )?;
    if retain_result {
        fs::write(
            directory.join("verus-result.json"),
            canonical_json(&output.stdout, "exception-stub Verus result")?,
        )
        .map_err(|error| format!("write exception-stub Verus result: {error}"))?;
    }
    let artifact = directory.join(RLIB);
    require_file(&artifact, "compiled exception-stub model")?;
    Ok(artifact)
}

fn audit_model(tools: &Tools, artifact: &Path, work: &Path) -> Result<(), String> {
    let members = run_checked(
        Command::new(&tools.ar).arg("t").arg(artifact),
        "list exception-stub model members",
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
            "exception-stub rlib members changed:\n{member_text}"
        ));
    }
    write_combined_output(&work.join("model-members.txt"), &members, "member evidence")?;
    let undefined = run_checked(
        Command::new(&tools.nm).arg("-u").arg(artifact),
        "audit exception-stub undefined symbols",
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
            "exception-stub undefined symbols changed: {names:?}"
        ));
    }
    write_combined_output(
        &work.join("model-undefined-symbols.txt"),
        &undefined,
        "undefined-symbol evidence",
    )?;
    let defined = run_checked(
        Command::new(&tools.nm).arg("--defined-only").arg(artifact),
        "audit exception-stub defined symbols",
    )?;
    require_output_fragments(
        &defined.stdout,
        "exception-stub defined symbols",
        &[
            "registered_stub_image",
            "registered_stub_table",
            "stub_jump_target",
            "exception_stub_observation",
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
        "compile exception-stub consumer",
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

fn link_table(
    tools: &Tools,
    linker: &Path,
    bytes: &Path,
    directory: &Path,
) -> Result<LinkedTable, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create link path {}: {error}", directory.display()))?;
    fs::copy(bytes, directory.join("stubs.bin"))
        .map_err(|error| format!("stage exception-stub bytes: {error}"))?;
    wrap_table(tools, directory)?;
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
            .args(["-o", "stubs.elf", "stubs.o"]),
        "link exception-stub ELF",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(directory).args([
            "--dump-section",
            ".text.tmk_exception_stubs=linked-stubs.bin",
            "stubs.elf",
        ]),
        "extract linked exception-stub bytes",
    )?;
    let linked = LinkedTable {
        elf: directory.join("stubs.elf"),
        bytes: directory.join("linked-stubs.bin"),
    };
    require_exact_bytes(
        &linked.bytes,
        &fs::read(bytes).map_err(|e| e.to_string())?,
        "linked exception-stub table",
    )?;
    Ok(linked)
}

fn wrap_table(tools: &Tools, directory: &Path) -> Result<(), String> {
    run_checked(
        Command::new(&tools.ld).current_dir(directory).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "stubs-raw.o",
            "stubs.bin",
        ]),
        "wrap exception-stub bytes",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(directory).args([
            "--rename-section",
            ".data=.text.tmk_exception_stubs,alloc,contents,load,readonly,code",
            "--redefine-sym",
            "_binary_stubs_bin_start=tmk_exception_stub_table",
            "--redefine-sym",
            "_binary_stubs_bin_end=tmk_exception_stub_table_end",
            "--redefine-sym",
            "_binary_stubs_bin_size=tmk_exception_stub_table_size",
            "stubs-raw.o",
            "stubs.o",
        ]),
        "name exception-stub object",
    )?;
    Ok(())
}

fn audit_linked(tools: &Tools, linked: &LinkedTable, work: &Path) -> Result<(), String> {
    let relocations = run_checked(
        Command::new(&tools.readelf).args(["-rW"]).arg(&linked.elf),
        "audit exception-stub relocations",
    )?;
    require_output_fragments(
        &relocations.stdout,
        "exception-stub relocations",
        &["There are no relocations in this file"],
    )?;
    write_combined_output(
        &work.join("linked-relocations.txt"),
        &relocations,
        "relocation evidence",
    )?;
    let sections = run_checked(
        Command::new(&tools.readelf).args(["-SW"]).arg(&linked.elf),
        "audit exception-stub sections",
    )?;
    audit_sections(&sections)?;
    write_combined_output(
        &work.join("linked-sections.txt"),
        &sections,
        "section evidence",
    )?;
    let symbols = run_checked(
        Command::new(&tools.readelf).args(["-sW"]).arg(&linked.elf),
        "audit exception-stub symbols",
    )?;
    require_output_fragments(
        &symbols.stdout,
        "exception-stub symbols",
        &[
            "tmk_exception_stub_table_link_start",
            "tmk_exception_stub_table_link_end",
            "tmk_exception_stub_table",
            "tmk_exception_common_entry",
            "ffffffff80010000",
            "ffffffff80011000",
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
        "disassemble exception-stub table",
    )?;
    let disassembly_text = String::from_utf8_lossy(&disassembly.stdout);
    let jump_count = disassembly_text
        .lines()
        .filter(|line| line.contains("jmp") && line.contains("ffffffff80011000"))
        .count();
    if jump_count != 256 {
        return Err(format!(
            "exception-stub disassembly has {jump_count} common-entry jumps, expected 256"
        ));
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
    if executable.len() == 1 && executable[0].contains(".text.tmk_exception_stubs") {
        Ok(())
    } else {
        Err(format!(
            "exception-stub executable section allowlist mismatch: {executable:?}"
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
    let mut mutated =
        fs::read(valid_bytes).map_err(|error| format!("read valid bytes: {error}"))?;
    mutated[0] ^= 1;
    fs::write(byte_dir.join("stubs.bin"), &mutated)
        .map_err(|error| format!("write byte mutation: {error}"))?;
    wrap_table(tools, &byte_dir)?;
    run_checked(
        Command::new(&tools.ld)
            .current_dir(&byte_dir)
            .args([
                "-m",
                "elf_x86_64",
                "--build-id=none",
                "-nostdlib",
                "-static",
            ])
            .arg("-T")
            .arg(linker)
            .args(["-o", "stubs.elf", "stubs.o"]),
        "link mutated exception-stub table",
    )?;
    let diagnostic = require_exact_bytes(
        &byte_dir.join("stubs.bin"),
        &fs::read(valid_bytes).map_err(|error| error.to_string())?,
        "mutated exception-stub table",
    )
    .expect_err("exception-stub byte mutation must fail");
    fs::write(
        work.join("negative-byte-mutation.txt"),
        format!("{diagnostic}\n"),
    )
    .map_err(|error| format!("write byte negative evidence: {error}"))?;

    let extra_dir = work.join("negative-unregistered-executable");
    fs::create_dir(&extra_dir).map_err(|error| format!("create section negative: {error}"))?;
    fs::copy(valid_bytes, extra_dir.join("stubs.bin"))
        .map_err(|error| format!("stage valid stubs: {error}"))?;
    fs::write(extra_dir.join("extra.bin"), [0x90])
        .map_err(|error| format!("write extra byte: {error}"))?;
    wrap_table(tools, &extra_dir)?;
    run_checked(
        Command::new(&tools.ld).current_dir(&extra_dir).args([
            "-r",
            "-b",
            "binary",
            "-o",
            "extra-raw.o",
            "extra.bin",
        ]),
        "wrap extra executable byte",
    )?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(&extra_dir).args([
            "--rename-section",
            ".data=.text.unregistered,alloc,contents,load,readonly,code",
            "extra-raw.o",
            "extra.o",
        ]),
        "classify extra executable byte",
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
            .args(["-o", "extra.elf", "stubs.o", "extra.o"]),
        "link extra exception-stub section",
    )?;
    let sections = run_checked(
        Command::new(&tools.readelf)
            .current_dir(&extra_dir)
            .args(["-SW", "extra.elf"]),
        "inspect extra exception-stub section",
    )?;
    let diagnostic = audit_sections(&sections).expect_err("extra executable section must fail");
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
            "error-classification",
            "        || vector == 13\n        || vector == 14\n        || vector == 17\n        || vector == 21\n        || vector == 29\n        || vector == 30\n}\n\npub open spec fn expected_stub_address",
            "        || vector == 13\n        || vector == 17\n        || vector == 21\n        || vector == 29\n        || vector == 30\n}\n\npub open spec fn expected_stub_address",
        ),
        (
            "displacement",
            "    4096u32 - (vector as u32) * 16u32 - used",
            "    4095u32 - (vector as u32) * 16u32 - used",
        ),
        (
            "synthetic-opcode",
            "        let qword0 = 0xe900_0000_0068_006au64 | ((vector as u64) << 24);",
            "        let qword0 = 0xe900_0000_0069_006au64 | ((vector as u64) << 24);",
        ),
        (
            "cpu-error-opcode",
            "        let qword0 = 0x0000_e900_0000_0068u64",
            "        let qword0 = 0x0000_e900_0000_0069u64",
        ),
        (
            "table-completeness",
            "    while slot < VECTOR_COUNT",
            "    while slot < VECTOR_COUNT - 1",
        ),
        (
            "observation",
            "    ensures result == 255,",
            "    ensures result == 254,",
        ),
        (
            "bad-assume",
            "    let table = registered_stub_table();",
            "    assume(false);\n    let table = registered_stub_table();",
        ),
    ];
    for (name, needle, replacement) in cases {
        if source.matches(needle).count() != 1 {
            return Err(format!(
                "exception-stub negative `{name}` target is not unique"
            ));
        }
        let directory = work.join(format!("negative-{name}"));
        fs::create_dir(&directory).map_err(|error| format!("create negative {name}: {error}"))?;
        let staged = directory.join(format!("{CRATE_NAME}.rs"));
        fs::write(&staged, source.replacen(needle, replacement, 1))
            .map_err(|error| format!("write negative {name}: {error}"))?;
        let output = run_expect_failure(
            &mut verus_command(tools, &directory, &format!("{CRATE_NAME}.rs"), false, false),
            &format!("reject exception-stub {name} mutation"),
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
            return Err(format!("exception-stub {name} did not fail atomically"));
        }
        write_combined_output(
            &work.join(format!("negative-{name}.txt")),
            &output,
            &format!("exception-stub {name} negative"),
        )?;
    }
    Ok(())
}

fn run_no_vstd_boundary(tools: &Tools, source: &Path, work: &Path) -> Result<(), String> {
    let directory = work.join("negative-no-vstd");
    fs::create_dir(&directory).map_err(|error| format!("create no-vstd path: {error}"))?;
    let staged = directory.join(format!("{CRATE_NAME}.rs"));
    fs::copy(source, &staged).map_err(|error| format!("stage no-vstd source: {error}"))?;
    let output = run_expect_failure(
        &mut verus_command(tools, &directory, &format!("{CRATE_NAME}.rs"), false, true),
        "confirm exception-stub vstd proof dependency",
    )?;
    let mut combined = Vec::new();
    combined.extend_from_slice(&output.stdout);
    combined.extend_from_slice(&output.stderr);
    require_output_fragments(
        &combined,
        "exception-stub no-vstd boundary",
        &["cannot find module or crate `vstd`"],
    )?;
    write_combined_output(
        &work.join("negative-vstd-proof-dependency.txt"),
        &output,
        "vstd proof dependency evidence",
    )
}
