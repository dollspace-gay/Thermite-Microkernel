use super::{
    canonical_json, direct_verus_command_with_rlimit, require_exact_bytes, require_file,
    require_output_fragments, run_checked, run_expect_failure, sha256sum, workspace_root,
    write_combined_output,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const EXPECTED_ALLOC: &[u8] = &[
    0x48, 0x85, 0xf6, 0x74, 0x67, 0x48, 0x85, 0xd2, 0x74, 0x62, 0x48, 0x8d, 0x4a, 0xff, 0x48, 0x85,
    0xca, 0x75, 0x59, 0x48, 0x83, 0xfa, 0x40, 0x76, 0x09, 0x48, 0x81, 0xfa, 0x00, 0x10, 0x00, 0x00,
    0x75, 0x4a, 0x48, 0x83, 0xbf, 0x08, 0x00, 0x01, 0x00, 0x00, 0x75, 0x40, 0x48, 0x8b, 0x87, 0x00,
    0x00, 0x01, 0x00, 0x48, 0x3d, 0x00, 0x00, 0x01, 0x00, 0x77, 0x31, 0x48, 0x01, 0xc1, 0x72, 0x2c,
    0x48, 0xf7, 0xda, 0x48, 0x21, 0xd1, 0x48, 0x81, 0xf9, 0x00, 0x00, 0x01, 0x00, 0x77, 0x1d, 0x41,
    0xb8, 0x00, 0x00, 0x01, 0x00, 0x49, 0x29, 0xc8, 0x4c, 0x39, 0xc6, 0x77, 0x0f, 0x48, 0x01, 0xce,
    0x48, 0x89, 0xb7, 0x00, 0x00, 0x01, 0x00, 0x48, 0x8d, 0x04, 0x0f, 0xc3, 0x31, 0xc0, 0xc3,
];
const EXPECTED_SEAL: &[u8] = &[
    0x48, 0xc7, 0x87, 0x08, 0x00, 0x01, 0x00, 0x01, 0, 0, 0, 0xc3,
];
const EXPECTED_MEMCPY: &[u8] = &[0x48, 0x89, 0xf8, 0x48, 0x89, 0xd1, 0xf3, 0xa4, 0xc3];
const EXPECTED_MEMSET: &[u8] = &[
    0x49, 0x89, 0xf8, 0x48, 0x89, 0xd1, 0x89, 0xf0, 0xf3, 0xaa, 0x4c, 0x89, 0xc0, 0xc3,
];

const RUST_ALLOC_SKELETON: &[u8] = &[0x48, 0x89, 0xfa, 0xe9, 0, 0, 0, 0];
const NULL_SKELETON: &[u8] = &[0x31, 0xc0, 0xc3];
const RETURN_SKELETON: &[u8] = &[0xc3];
const METHOD_ALLOC_SKELETON: &[u8] = &[
    0x48, 0x89, 0xf0, 0x48, 0xc7, 0xc7, 0, 0, 0, 0, 0x48, 0x89, 0xd6, 0x48, 0x89, 0xc2, 0xe9, 0, 0,
    0, 0,
];
const SEAL_SKELETON: &[u8] = &[0x48, 0xc7, 0xc7, 0, 0, 0, 0, 0xe9, 0, 0, 0, 0];

struct Tools {
    verus: PathBuf,
    rustc: PathBuf,
    ld: PathBuf,
    ar: PathBuf,
    objcopy: PathBuf,
    objdump: PathBuf,
    readelf: PathBuf,
    nm: PathBuf,
    timeout: PathBuf,
}

impl Tools {
    fn pinned() -> Result<Self, String> {
        let tools = Self {
            verus: PathBuf::from("/opt/verus/0.2026.05.24.ecee80a/verus"),
            rustc: PathBuf::from(
                "/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc",
            ),
            ld: PathBuf::from("/usr/sbin/ld"),
            ar: PathBuf::from("/usr/sbin/ar"),
            objcopy: PathBuf::from("/usr/sbin/objcopy"),
            objdump: PathBuf::from("/usr/sbin/objdump"),
            readelf: PathBuf::from("/usr/sbin/readelf"),
            nm: PathBuf::from("/usr/sbin/nm"),
            timeout: PathBuf::from("/usr/bin/timeout"),
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
                tools.ar.as_path(),
                "a21151402078c113fd801d16e0a0d2659ee44cee1b9828474f937bbf097b0df6",
                "GNU ar",
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
            (
                tools.timeout.as_path(),
                "350001cc47ad731c4e797532fe46a999477aba359692e2de3e93f316b4021dab",
                "timeout",
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
    let model_source = root.join("verus/machine-model/platform_primitives_capsule.rs");
    let model_consumer = root.join("tests/m0/platform_primitives_consumer.rs");
    let adapter_source = root.join("kernel-host/platform/global_allocator.rs");
    let runtime_source = root.join("tests/m0/global_allocator_consumer.rs");
    let kernel_source = root.join("tests/m0/global_allocator_kernel_consumer.rs");
    let kernel_linker = root.join("tests/m0/global_allocator_kernel.ld");
    for (path, label) in [
        (&model_source, "platform primitive Verus model"),
        (&model_consumer, "platform primitive model consumer"),
        (&adapter_source, "GlobalAlloc adapter"),
        (&runtime_source, "GlobalAlloc runtime consumer"),
        (&kernel_source, "GlobalAlloc freestanding consumer"),
        (&kernel_linker, "GlobalAlloc higher-half linker script"),
    ] {
        require_file(path, label)?;
    }
    let model_text = fs::read_to_string(&model_source)
        .map_err(|error| format!("read {}: {error}", model_source.display()))?;
    audit_model_source(&model_text)?;
    let adapter_text = fs::read_to_string(&adapter_source)
        .map_err(|error| format!("read {}: {error}", adapter_source.display()))?;
    audit_adapter_source(&adapter_text)?;

    let model_source_sha = sha256sum(&model_source)?;
    let adapter_source_sha = sha256sum(&adapter_source)?;
    let auditor_sha = sha256sum(&root.join("xtask/src/platform_primitives.rs"))?;
    let work = root.join("build/m0-platform-primitives");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;

    let model = build_model(&tools, &model_source, &work.join("model-primary"), true)?;
    let model_sha = sha256sum(&model)?;
    for name in ["model-repro-a", "model-repro-b"] {
        let reproduced = build_model(&tools, &model_source, &work.join(name), false)?;
        let actual = sha256sum(&reproduced)?;
        if actual != model_sha {
            return Err(format!(
                "platform model build in {name} produced {actual}, expected {model_sha}"
            ));
        }
    }

    let emitted = work.join("emitted");
    fs::create_dir(&emitted)
        .map_err(|error| format!("create primitive byte directory: {error}"))?;
    let consumer = work.join("model-consumer");
    run_checked(
        Command::new(&tools.rustc)
            .current_dir(&root)
            .args(["--edition=2021"])
            .arg(&model_consumer)
            .arg("--extern")
            .arg(format!("tmk_platform_primitives={}", model.display()))
            .arg("-L")
            .arg("dependency=/opt/verus/0.2026.05.24.ecee80a")
            .args(["-C", "panic=abort"])
            .arg("-o")
            .arg(&consumer),
        "link platform primitive model consumer",
    )?;
    let runtime = run_checked(
        Command::new(&consumer).current_dir(&root).arg(&emitted),
        "execute platform primitive model and emit registered bytes",
    )?;
    require_output_fragments(
        &runtime.stdout,
        "platform primitive model runtime",
        &["M0_PLATFORM_PRIMITIVES_OK:111:12:9:14:20008:21000:sealed"],
    )?;
    write_combined_output(&work.join("model-runtime.txt"), &runtime, "model runtime")?;
    require_registered_bytes(&emitted)?;

    let primitives = build_primitive_object(&tools, &emitted, &work)?;
    audit_primitive_object(&tools, &primitives, &work)?;

    let adapter = build_adapter(&tools, &adapter_source, &work.join("adapter-primary"))?;
    let adapter_sha = sha256sum(&adapter)?;
    for name in ["adapter-repro-a", "adapter-repro-b"] {
        let reproduced = build_adapter(&tools, &adapter_source, &work.join(name))?;
        let actual = sha256sum(&reproduced)?;
        if actual != adapter_sha {
            return Err(format!(
                "GlobalAlloc adapter build in {name} produced {actual}, expected {adapter_sha}"
            ));
        }
    }
    audit_adapter_object(&tools, &adapter, &work.join("adapter-audit"))?;

    let hosted = work.join("global-allocator-consumer");
    let hosted_output = compile_hosted_consumer(
        &tools,
        &root,
        &runtime_source,
        &adapter,
        &primitives,
        &hosted,
    )?;
    require_output_fragments(
        &hosted_output.stdout,
        "GlobalAlloc hosted execution",
        &["M0_GLOBAL_ALLOC_OK:box:vec:reject:sealed"],
    )?;
    write_combined_output(
        &work.join("hosted-runtime.txt"),
        &hosted_output,
        "GlobalAlloc hosted execution",
    )?;

    let kernel = work.join("global-allocator-kernel-consumer");
    compile_kernel_consumer(
        &tools,
        &root,
        &kernel_source,
        &adapter,
        &primitives,
        &kernel,
    )?;
    let undefined = run_checked(
        Command::new(&tools.nm).arg("-u").arg(&kernel),
        "freestanding GlobalAlloc undefined-symbol audit",
    )?;
    if !undefined.stdout.is_empty() {
        return Err(format!(
            "freestanding GlobalAlloc consumer has undefined symbols:\n{}",
            String::from_utf8_lossy(&undefined.stdout)
        ));
    }
    let execution = run_expect_failure(
        Command::new(&tools.timeout)
            .current_dir(&root)
            .args(["0.1s"])
            .arg(&kernel),
        "execute freestanding GlobalAlloc consumer",
    )?;
    if execution.status.code() != Some(124) {
        return Err(format!(
            "freestanding GlobalAlloc consumer exited with {}, expected timeout 124",
            execution.status
        ));
    }
    let kernel_sha = sha256sum(&kernel)?;
    for name in ["kernel-repro-a", "kernel-repro-b"] {
        let reproduced = work.join(name);
        compile_kernel_consumer(
            &tools,
            &root,
            &kernel_source,
            &adapter,
            &primitives,
            &reproduced,
        )?;
        let actual = sha256sum(&reproduced)?;
        if actual != kernel_sha {
            return Err(format!(
                "freestanding link in {name} produced {actual}, expected {kernel_sha}"
            ));
        }
    }

    let high_half = work.join("global-allocator-high-half");
    compile_high_half_kernel_consumer(
        &tools,
        &root,
        &kernel_source,
        &kernel_linker,
        &adapter,
        &primitives,
        &high_half,
    )?;
    let undefined = run_checked(
        Command::new(&tools.nm).arg("-u").arg(&high_half),
        "higher-half GlobalAlloc undefined-symbol audit",
    )?;
    if !undefined.stdout.is_empty() {
        return Err(format!(
            "higher-half GlobalAlloc consumer has undefined symbols:\n{}",
            String::from_utf8_lossy(&undefined.stdout)
        ));
    }
    audit_high_half_symbols(&tools, &high_half)?;
    let high_half_sha = sha256sum(&high_half)?;
    for name in ["high-half-repro-a", "high-half-repro-b"] {
        let reproduced = work.join(name);
        compile_high_half_kernel_consumer(
            &tools,
            &root,
            &kernel_source,
            &kernel_linker,
            &adapter,
            &primitives,
            &reproduced,
        )?;
        let actual = sha256sum(&reproduced)?;
        if actual != high_half_sha {
            return Err(format!(
                "higher-half link in {name} produced {actual}, expected {high_half_sha}"
            ));
        }
    }

    run_negative_cases(
        &tools,
        &root,
        &model_source,
        &model_text,
        &adapter_text,
        &work,
    )?;

    let primitives_sha = sha256sum(&primitives)?;
    let hosted_sha = sha256sum(&hosted)?;
    let report = format!(
        "M0_PLATFORM_PRIMITIVES_OK\ncomponent_verified=true\nrelease_eligible=false\nmodel_source_sha256={model_source_sha}\nadapter_source_sha256={adapter_source_sha}\nauditor_sha256={auditor_sha}\nlinker_script_sha256={}\nmodel_artifact_sha256={model_sha}\nadapter_artifact_sha256={adapter_sha}\nprimitive_object_sha256={primitives_sha}\nhosted_consumer_sha256={hosted_sha}\nfreestanding_consumer_sha256={kernel_sha}\nhigh_half_consumer_sha256={high_half_sha}\nmodel_reproducibility_builds=3\nadapter_reproducibility_builds=3\nfreestanding_reproducibility_links=3\nhigh_half_reproducibility_links=3\nverus_verified=39\nalloc_capsule_sha256={}\nseal_capsule_sha256={}\nmemcpy_capsule_sha256={}\nmemset_capsule_sha256={}\nruntime_marker=M0_GLOBAL_ALLOC_OK:box:vec:reject:sealed\nfreestanding_runtime=fail-stop-timeout-124\nhigh_half_link_base=ffffffff80000000\nnegative_cases=alloc-byte,alloc-semantics,assume,arena-layout,code-model\n",
        sha256sum(&kernel_linker)?,
        sha256sum(&emitted.join("alloc.bin"))?,
        sha256sum(&emitted.join("seal.bin"))?,
        sha256sum(&emitted.join("memcpy.bin"))?,
        sha256sum(&emitted.join("memset.bin"))?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write platform primitive report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn audit_model_source(source: &str) -> Result<(), String> {
    for forbidden in [
        "assume(",
        "admit(",
        "axiom fn",
        "external_body",
        "unsafe ",
        "asm!",
    ] {
        if source.contains(forbidden) {
            return Err(format!(
                "platform primitive model contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "pub fn execute_alloc_capsule",
        "pub fn decode_execute_alloc_capsule",
        "pub fn execute_global_alloc_adapter",
        "pub fn decode_execute_global_alloc_adapter",
        "pub fn execute_memcpy_observation",
        "pub fn decode_execute_memcpy_observation",
        "pub fn execute_memset_observation",
        "pub fn decode_execute_memset_observation",
        "direction_flag_clear: bool",
        "pub fn registered_global_alloc_relocations",
    ] {
        if !source.contains(required) {
            return Err(format!("platform primitive model is missing `{required}`"));
        }
    }
    Ok(())
}

fn audit_adapter_source(source: &str) -> Result<(), String> {
    for forbidden in [
        "asm!",
        "global_asm!",
        "transmute",
        "external_body",
        "assume(",
        "admit(",
        "Atomic",
    ] {
        if source.contains(forbidden) {
            return Err(format!(
                "GlobalAlloc adapter contains forbidden `{forbidden}`"
            ));
        }
    }
    for required in [
        "const BOOT_ARENA_BYTES: usize = 65_536;",
        "#[repr(C, align(4096))]",
        "unsafe impl GlobalAlloc for TmkBootAllocator",
        "tmk_alloc_capsule(",
        "tmk_seal_capsule(",
        "core::ptr::null_mut()",
        "pub extern \"C\" fn tmk_global_alloc_seal()",
    ] {
        if !source.contains(required) {
            return Err(format!("GlobalAlloc adapter is missing `{required}`"));
        }
    }
    if source.matches("unsafe impl GlobalAlloc").count() != 1
        || source.matches("tmk_alloc_capsule(").count() != 2
        || source.matches("tmk_seal_capsule(").count() != 2
        || source.matches("core::ptr::null_mut()").count() != 2
    {
        return Err("GlobalAlloc adapter executable shape changed".to_string());
    }
    Ok(())
}

fn build_model(
    tools: &Tools,
    source: &Path,
    directory: &Path,
    retain_result: bool,
) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create model build path {}: {error}", directory.display()))?;
    let staged = directory.join("tmk_platform_primitives.rs");
    fs::copy(source, &staged)
        .map_err(|error| format!("stage platform primitive model: {error}"))?;
    let output = run_checked(
        &mut direct_verus_command_with_rlimit(
            &tools.verus,
            directory,
            "tmk_platform_primitives.rs",
            true,
            "40",
        ),
        "Verus platform primitive proof and codegen",
    )?;
    require_output_fragments(
        &output.stdout,
        "Verus platform primitive proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 39",
            "\"errors\": 0",
            "\"version\": \"0.2026.05.24.ecee80a\"",
        ],
    )?;
    if retain_result {
        fs::write(
            directory.join("verus-result.json"),
            canonical_json(&output.stdout, "platform primitive Verus result")?,
        )
        .map_err(|error| format!("write platform primitive Verus result: {error}"))?;
    }
    let model = directory.join("libtmk_platform_primitives.rlib");
    require_file(&model, "compiled platform primitive model")?;
    Ok(model)
}

fn require_registered_bytes(directory: &Path) -> Result<(), String> {
    for (name, expected) in [
        ("alloc.bin", EXPECTED_ALLOC),
        ("seal.bin", EXPECTED_SEAL),
        ("memcpy.bin", EXPECTED_MEMCPY),
        ("memset.bin", EXPECTED_MEMSET),
    ] {
        require_exact_bytes(&directory.join(name), expected, name)?;
    }
    Ok(())
}

fn build_primitive_object(tools: &Tools, emitted: &Path, work: &Path) -> Result<PathBuf, String> {
    let specs = [
        ("alloc", ".text.tmk_alloc_capsule", "tmk_alloc_capsule"),
        ("seal", ".text.tmk_seal_capsule", "tmk_seal_capsule"),
        ("memcpy", ".text.tmk_memcpy_capsule", "memcpy"),
        ("memset", ".text.tmk_memset_capsule", "memset"),
    ];
    let object_dir = work.join("objects");
    fs::create_dir(&object_dir)
        .map_err(|error| format!("create primitive object path: {error}"))?;
    let mut objects = Vec::new();
    for (name, section, symbol) in specs {
        fs::copy(
            emitted.join(format!("{name}.bin")),
            object_dir.join(format!("{name}.bin")),
        )
        .map_err(|error| format!("stage {name} bytes for object wrapping: {error}"))?;
        run_checked(
            Command::new(&tools.ld).current_dir(&object_dir).args([
                "-r",
                "-b",
                "binary",
                "-o",
                &format!("{name}-raw.o"),
                &format!("{name}.bin"),
            ]),
            &format!("wrap {name} primitive bytes"),
        )?;
        let prefix = format!("_binary_{}_bin", name.replace('-', "_"));
        run_checked(
            Command::new(&tools.objcopy).current_dir(&object_dir).args([
                "--rename-section",
                &format!(".data={section},alloc,contents,load,readonly,code"),
                "--redefine-sym",
                &format!("{prefix}_start={symbol}"),
                "--redefine-sym",
                &format!("{prefix}_end={symbol}_end"),
                "--redefine-sym",
                &format!("{prefix}_size={symbol}_size"),
                &format!("{name}-raw.o"),
                &format!("{name}.o"),
            ]),
            &format!("name and classify {name} primitive object"),
        )?;
        objects.push(format!("{name}.o"));
    }
    let output = object_dir.join("platform-primitives.o");
    let mut command = Command::new(&tools.ld);
    command
        .current_dir(&object_dir)
        .args(["-r", "-o", "platform-primitives.o"]);
    command.args(&objects);
    run_checked(&mut command, "combine platform primitive objects")?;
    require_file(&output, "combined platform primitive object")?;
    Ok(output)
}

fn audit_primitive_object(tools: &Tools, object: &Path, work: &Path) -> Result<(), String> {
    let sections = run_checked(
        Command::new(&tools.readelf).args(["-SW"]).arg(object),
        "platform primitive section audit",
    )?;
    require_output_fragments(
        &sections.stdout,
        "platform primitive sections",
        &[
            ".text.tmk_alloc_capsule",
            ".text.tmk_seal_capsule",
            ".text.tmk_memcpy_capsule",
            ".text.tmk_memset_capsule",
        ],
    )?;
    let text = String::from_utf8_lossy(&sections.stdout);
    let executable: Vec<_> = text.lines().filter(|line| line.contains(" AX ")).collect();
    if executable.len() != 4 {
        return Err(format!(
            "primitive object has unexpected executable sections: {executable:?}"
        ));
    }
    let relocations = run_checked(
        Command::new(&tools.readelf).args(["-rW"]).arg(object),
        "platform primitive relocation audit",
    )?;
    require_output_fragments(
        &relocations.stdout,
        "platform primitive relocation audit",
        &["There are no relocations in this file"],
    )?;
    let disassembly = run_checked(
        Command::new(&tools.objdump)
            .args(["-d", "-Mintel"])
            .arg(object),
        "platform primitive disassembly",
    )?;
    require_output_fragments(
        &disassembly.stdout,
        "platform primitive disassembly",
        &[
            "<tmk_alloc_capsule>",
            "rep movs",
            "rep stos",
            "<tmk_seal_capsule>",
        ],
    )?;
    fs::write(work.join("primitive-sections.txt"), &sections.stdout)
        .map_err(|error| format!("write primitive section evidence: {error}"))?;
    fs::write(work.join("primitive-disassembly.txt"), &disassembly.stdout)
        .map_err(|error| format!("write primitive disassembly evidence: {error}"))?;
    Ok(())
}

fn adapter_command(
    tools: &Tools,
    root: &Path,
    source: &Path,
    output: &Path,
    code_model: &str,
) -> Command {
    let mut command = Command::new(&tools.rustc);
    command
        .current_dir(root)
        .args(["--crate-name", "tmk_global_allocator", "--edition=2024"])
        .args(["-C", "panic=abort"])
        .args(["-C", "opt-level=z"])
        .args(["-C", "codegen-units=1"])
        .args(["-C", "relocation-model=static"])
        .arg("-C")
        .arg(format!("code-model={code_model}"))
        .args(["-C", "no-redzone=yes"])
        .args(["-C", "overflow-checks=off"])
        .arg(format!("--remap-path-prefix={}=.", root.display()))
        .arg(source)
        .arg("-o")
        .arg(output);
    command
}

fn build_adapter(tools: &Tools, source: &Path, directory: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create adapter build path {}: {error}", directory.display()))?;
    let staged = directory.join("tmk_global_allocator.rs");
    fs::copy(source, &staged).map_err(|error| format!("stage GlobalAlloc adapter: {error}"))?;
    let output = directory.join("libtmk_global_allocator.rlib");
    run_checked(
        &mut adapter_command(tools, directory, &staged, &output, "kernel"),
        "compile pinned GlobalAlloc ABI adapter",
    )?;
    require_file(&output, "compiled GlobalAlloc adapter")?;
    Ok(output)
}

fn audit_adapter_object(tools: &Tools, rlib: &Path, directory: &Path) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|error| format!("create adapter audit path: {error}"))?;
    run_checked(
        Command::new(&tools.ar)
            .current_dir(directory)
            .arg("x")
            .arg(rlib),
        "extract GlobalAlloc adapter object",
    )?;
    let objects: Vec<_> = fs::read_dir(directory)
        .map_err(|error| format!("read adapter audit path: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "o"))
        .collect();
    if objects.len() != 1 {
        return Err(format!(
            "expected one GlobalAlloc codegen object, found {objects:?}"
        ));
    }
    let object = &objects[0];
    let headers = run_checked(
        Command::new(&tools.objdump).args(["-h"]).arg(object),
        "list GlobalAlloc adapter sections",
    )?;
    let header_text = String::from_utf8_lossy(&headers.stdout);
    let sections: Vec<&str> = header_text
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            (fields.len() >= 3 && fields[0].parse::<usize>().is_ok()).then(|| fields[1])
        })
        .collect();
    let executable_sections: Vec<_> = sections
        .iter()
        .copied()
        .filter(|section| section.starts_with(".text."))
        .collect();
    if executable_sections.len() != 9 {
        return Err(format!(
            "GlobalAlloc adapter has unexpected executable sections: {executable_sections:?}"
        ));
    }
    let arena_section = header_text
        .lines()
        .find(|line| line.contains(".bss.TMK_BOOT_ARENA"))
        .ok_or_else(|| "GlobalAlloc adapter arena section is missing".to_string())?;
    if !arena_section.contains("00011000") || !arena_section.ends_with("2**12") {
        return Err(format!(
            "GlobalAlloc adapter arena section has wrong size/alignment: {arena_section}"
        ));
    }
    let cases = [
        ("rust-alloc", "12___rust_alloc", RUST_ALLOC_SKELETON),
        ("rust-dealloc", "14___rust_dealloc", RETURN_SKELETON),
        ("rust-realloc", "14___rust_realloc", NULL_SKELETON),
        ("rust-alloc-zeroed", "19___rust_alloc_zeroed", NULL_SKELETON),
        (
            "method-alloc",
            "GlobalAlloc$GT$5alloc17",
            METHOD_ALLOC_SKELETON,
        ),
        (
            "method-dealloc",
            "GlobalAlloc$GT$7dealloc17",
            RETURN_SKELETON,
        ),
        ("method-realloc", "GlobalAlloc$GT$7realloc17", NULL_SKELETON),
        (
            "method-alloc-zeroed",
            "GlobalAlloc$GT$12alloc_zeroed17",
            NULL_SKELETON,
        ),
        ("seal", ".text.tmk_global_alloc_seal", SEAL_SKELETON),
    ];
    for (name, needle, expected) in cases {
        let matches: Vec<_> = sections
            .iter()
            .copied()
            .filter(|section| section.contains(needle))
            .collect();
        if matches.len() != 1 {
            return Err(format!(
                "adapter section `{needle}` resolved as {matches:?}"
            ));
        }
        let output = directory.join(format!("{name}.bin"));
        run_checked(
            Command::new(&tools.objcopy)
                .arg("--dump-section")
                .arg(format!("{}={}", matches[0], output.display()))
                .arg(object),
            &format!("extract {name} adapter bytes"),
        )?;
        require_exact_bytes(&output, expected, &format!("GlobalAlloc {name} skeleton"))?;
    }

    let relocations = run_checked(
        Command::new(&tools.readelf).args(["-rW"]).arg(object),
        "GlobalAlloc adapter relocation audit",
    )?;
    require_output_fragments(
        &relocations.stdout,
        "GlobalAlloc adapter relocations",
        &[
            "R_X86_64_32S           0000000000000000 TMK_BOOT_ARENA + 0",
            "R_X86_64_PLT32         0000000000000000 tmk_alloc_capsule - 4",
            "R_X86_64_PLT32         0000000000000000 tmk_seal_capsule - 4",
        ],
    )?;
    let relocation_text = String::from_utf8_lossy(&relocations.stdout);
    let relocation_lines: Vec<_> = relocation_text
        .lines()
        .filter(|line| line.contains("R_X86_64_"))
        .collect();
    if relocation_lines.len() != 14
        || relocation_lines
            .iter()
            .filter(|line| line.contains("R_X86_64_PLT32"))
            .count()
            != 3
        || relocation_lines
            .iter()
            .filter(|line| line.contains("R_X86_64_32S"))
            .count()
            != 2
        || relocation_lines
            .iter()
            .filter(|line| line.contains("R_X86_64_PC32"))
            .count()
            != 9
    {
        return Err(format!(
            "GlobalAlloc adapter relocation set changed: {relocation_lines:?}"
        ));
    }
    for (offset, kind, target) in [
        (
            "0000000000000004",
            "R_X86_64_PLT32",
            "GlobalAlloc$GT$5alloc17",
        ),
        ("0000000000000006", "R_X86_64_32S", "TMK_BOOT_ARENA + 0"),
        (
            "0000000000000011",
            "R_X86_64_PLT32",
            "tmk_alloc_capsule - 4",
        ),
        ("0000000000000003", "R_X86_64_32S", "TMK_BOOT_ARENA + 0"),
        ("0000000000000008", "R_X86_64_PLT32", "tmk_seal_capsule - 4"),
    ] {
        let matches = relocation_lines
            .iter()
            .filter(|line| line.contains(offset) && line.contains(kind) && line.contains(target))
            .count();
        if matches != 1 {
            return Err(format!(
                "GlobalAlloc adapter relocation {offset}/{kind}/{target} matched {matches} entries"
            ));
        }
    }
    if relocation_lines
        .iter()
        .filter(|line| line.contains("R_X86_64_PC32"))
        .any(|line| !line.contains(".text."))
    {
        return Err("GlobalAlloc adapter has a non-local unwind relocation".to_string());
    }
    let undefined = run_checked(
        Command::new(&tools.nm).arg("-u").arg(rlib),
        "GlobalAlloc adapter undefined-symbol audit",
    )?;
    let undefined_text = String::from_utf8_lossy(&undefined.stdout);
    let undefined_symbols: Vec<_> = undefined_text
        .lines()
        .filter(|line| line.trim_start().starts_with("U "))
        .filter_map(|line| line.split_whitespace().last())
        .collect();
    if undefined_symbols != ["tmk_alloc_capsule", "tmk_seal_capsule"] {
        return Err(format!(
            "GlobalAlloc adapter undefined-symbol set is {undefined_symbols:?}"
        ));
    }
    let symbols = run_checked(
        Command::new(&tools.nm)
            .args(["-S", "--size-sort"])
            .arg(object),
        "GlobalAlloc adapter symbol-size audit",
    )?;
    require_output_fragments(
        &symbols.stdout,
        "GlobalAlloc adapter symbol-size audit",
        &["0000000000011000 B TMK_BOOT_ARENA"],
    )?;
    fs::write(directory.join("relocations.txt"), &relocations.stdout)
        .map_err(|error| format!("write adapter relocation evidence: {error}"))?;
    fs::write(directory.join("symbols.txt"), &symbols.stdout)
        .map_err(|error| format!("write adapter symbol evidence: {error}"))?;
    Ok(())
}

fn compile_hosted_consumer(
    tools: &Tools,
    root: &Path,
    source: &Path,
    adapter: &Path,
    primitives: &Path,
    output: &Path,
) -> Result<Output, String> {
    run_checked(
        Command::new(&tools.rustc)
            .current_dir(root)
            .args(["--edition=2024"])
            .arg(source)
            .arg("--extern")
            .arg(format!("tmk_global_allocator={}", adapter.display()))
            .args(["-C", "panic=abort"])
            .args(["-C", "link-arg=-nostartfiles"])
            .args(["-C", "link-arg=-no-pie"])
            .arg("-C")
            .arg(format!("link-arg={}", primitives.display()))
            .args(["-l", "c"])
            .arg("-o")
            .arg(output),
        "link GlobalAlloc hosted acceptance consumer",
    )?;
    run_checked(
        Command::new(output).current_dir(root),
        "execute GlobalAlloc hosted acceptance consumer",
    )
}

fn compile_kernel_consumer(
    tools: &Tools,
    root: &Path,
    source: &Path,
    adapter: &Path,
    primitives: &Path,
    output: &Path,
) -> Result<(), String> {
    run_checked(
        Command::new(&tools.rustc)
            .current_dir(root)
            .args(["--edition=2024"])
            .arg(source)
            .arg("--extern")
            .arg(format!("tmk_global_allocator={}", adapter.display()))
            .args(["-C", "panic=abort"])
            .args(["-C", "link-arg=-nostartfiles"])
            .args(["-C", "link-arg=-no-pie"])
            .args(["-C", "link-arg=-static"])
            .args(["-C", "code-model=kernel"])
            .arg("-C")
            .arg(format!("link-arg={}", primitives.display()))
            .args(["-C", "link-arg=-Wl,--build-id=none"])
            .arg(format!("--remap-path-prefix={}=.", root.display()))
            .arg("-o")
            .arg(output),
        "link static freestanding GlobalAlloc consumer",
    )?;
    require_file(output, "static freestanding GlobalAlloc consumer")?;
    Ok(())
}

fn compile_high_half_kernel_consumer(
    tools: &Tools,
    root: &Path,
    source: &Path,
    linker_script: &Path,
    adapter: &Path,
    primitives: &Path,
    output: &Path,
) -> Result<(), String> {
    run_checked(
        Command::new(&tools.rustc)
            .current_dir(root)
            .args(["--edition=2024"])
            .arg(source)
            .arg("--extern")
            .arg(format!("tmk_global_allocator={}", adapter.display()))
            .args(["-C", "panic=abort"])
            .args(["-C", "code-model=kernel"])
            .args(["-C", "link-arg=-nostartfiles"])
            .args(["-C", "link-arg=-no-pie"])
            .args(["-C", "link-arg=-static"])
            .arg("-C")
            .arg(format!("link-arg={}", primitives.display()))
            .args(["-C", "link-arg=-Wl,--build-id=none"])
            .arg("-C")
            .arg(format!("link-arg=-T{}", linker_script.display()))
            .arg(format!("--remap-path-prefix={}=.", root.display()))
            .arg("-o")
            .arg(output),
        "link higher-half GlobalAlloc consumer",
    )?;
    require_file(output, "higher-half GlobalAlloc consumer")?;
    Ok(())
}

fn audit_high_half_symbols(tools: &Tools, image: &Path) -> Result<(), String> {
    let symbols = run_checked(
        Command::new(&tools.nm).args(["-n"]).arg(image),
        "higher-half GlobalAlloc symbol audit",
    )?;
    let text = String::from_utf8_lossy(&symbols.stdout);
    let symbol_address = |name: &str| -> Result<u64, String> {
        let line = text
            .lines()
            .find(|line| line.split_whitespace().last() == Some(name))
            .ok_or_else(|| format!("higher-half image is missing `{name}`"))?;
        let address = line
            .split_whitespace()
            .next()
            .ok_or_else(|| format!("higher-half symbol `{name}` has no address"))?;
        u64::from_str_radix(address, 16)
            .map_err(|error| format!("parse higher-half symbol `{name}`: {error}"))
    };
    let base = 0xffff_ffff_8000_0000_u64;
    if symbol_address("_start")? != base || symbol_address("TMK_BOOT_ARENA")? < base {
        return Err("GlobalAlloc higher-half symbols escaped the kernel image".to_string());
    }
    let headers = run_checked(
        Command::new(&tools.readelf).args(["-lW"]).arg(image),
        "higher-half GlobalAlloc program-header audit",
    )?;
    require_output_fragments(
        &headers.stdout,
        "higher-half GlobalAlloc program headers",
        &["Entry point 0xffffffff80000000", "0xffffffff80000000"],
    )?;
    Ok(())
}

fn run_negative_cases(
    tools: &Tools,
    root: &Path,
    model_source: &Path,
    model_text: &str,
    adapter_text: &str,
    work: &Path,
) -> Result<(), String> {
    let negatives = work.join("negative");
    fs::create_dir(&negatives).map_err(|error| format!("create negative path: {error}"))?;

    let mut mutated = EXPECTED_ALLOC.to_vec();
    mutated[0] ^= 1;
    let mutated_path = negatives.join("mutated-alloc.bin");
    fs::write(&mutated_path, mutated)
        .map_err(|error| format!("write mutated allocation capsule: {error}"))?;
    let diagnostic = require_exact_bytes(&mutated_path, EXPECTED_ALLOC, "mutated alloc capsule")
        .expect_err("mutated alloc capsule must fail exact-byte audit");
    fs::write(negatives.join("alloc-byte.txt"), format!("{diagnostic}\n"))
        .map_err(|error| format!("write alloc-byte rejection: {error}"))?;

    let bad_semantics = model_text.replacen(
        "address: state.base + offset,",
        "address: state.base + cursor,",
        1,
    );
    if bad_semantics == model_text {
        return Err("allocation semantic mutation target not found".to_string());
    }
    fs::write(negatives.join("bad-semantics.rs"), bad_semantics)
        .map_err(|error| format!("write allocation semantic mutation: {error}"))?;
    let result = run_expect_failure(
        &mut direct_verus_command_with_rlimit(
            &tools.verus,
            &negatives,
            "bad-semantics.rs",
            false,
            "40",
        ),
        "Verus rejects allocation semantic mutation",
    )?;
    require_output_fragments(
        &[result.stdout.as_slice(), result.stderr.as_slice()].concat(),
        "allocation semantic mutation",
        &["postcondition not satisfied"],
    )?;
    write_combined_output(
        &negatives.join("alloc-semantics.txt"),
        &result,
        "allocation semantic mutation",
    )?;

    let bad_assume = model_text.replacen(
        "    if state.sealed || size == 0",
        "    assume(false);\n    if state.sealed || size == 0",
        1,
    );
    if bad_assume == model_text {
        return Err("platform primitive assume mutation target not found".to_string());
    }
    fs::write(negatives.join("bad-assume.rs"), bad_assume)
        .map_err(|error| format!("write platform primitive assume mutation: {error}"))?;
    let result = run_expect_failure(
        &mut direct_verus_command_with_rlimit(
            &tools.verus,
            &negatives,
            "bad-assume.rs",
            false,
            "40",
        ),
        "Verus rejects platform primitive assume",
    )?;
    require_output_fragments(
        &[result.stdout.as_slice(), result.stderr.as_slice()].concat(),
        "platform primitive assume mutation",
        &["assume/admit not allowed with --no-cheating"],
    )?;
    write_combined_output(
        &negatives.join("assume.txt"),
        &result,
        "platform primitive assume mutation",
    )?;

    let bad_arena = adapter_text.replacen("65_536", "65_528", 1);
    if bad_arena == adapter_text {
        return Err("adapter arena mutation target not found".to_string());
    }
    let bad_arena_path = negatives.join("bad-arena.rs");
    fs::write(&bad_arena_path, &bad_arena)
        .map_err(|error| format!("write adapter arena mutation: {error}"))?;
    let diagnostic = audit_adapter_source(&bad_arena)
        .expect_err("mutated adapter arena must fail source-shape audit");
    fs::write(
        negatives.join("arena-layout.txt"),
        format!("{diagnostic}\n"),
    )
    .map_err(|error| format!("write arena-layout rejection: {error}"))?;

    let small_model = negatives.join("small-code-model");
    fs::create_dir(&small_model)
        .map_err(|error| format!("create small-code-model path: {error}"))?;
    let staged = small_model.join("tmk_global_allocator.rs");
    fs::write(&staged, adapter_text)
        .map_err(|error| format!("stage small-code-model adapter: {error}"))?;
    let rlib = small_model.join("libtmk_global_allocator.rlib");
    run_checked(
        &mut adapter_command(tools, &small_model, &staged, &rlib, "small"),
        "compile rejected small-code-model GlobalAlloc adapter",
    )?;
    let diagnostic = audit_adapter_object(tools, &rlib, &negatives.join("small-code-audit"))
        .expect_err("small-code-model adapter must fail the exact adapter audit");
    fs::write(negatives.join("code-model.txt"), format!("{diagnostic}\n"))
        .map_err(|error| format!("write code-model rejection: {error}"))?;

    if !model_source.starts_with(root) {
        return Err("platform primitive model escaped the workspace".to_string());
    }
    Ok(())
}
