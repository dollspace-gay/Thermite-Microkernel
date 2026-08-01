use super::m1_bootinfo::{validate_candidate_pin, FORGE_SHA256, THERMITE_COMMIT};
use super::m1_elf::{read_json, require_file, verify_bundle};
use super::{
    canonical_json, check_forge_skill, forge_binary, json_string, require_output_fragments,
    run_checked, run_expect_failure, sha256sum, workspace_root, write_combined_output,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MODEL_SOURCE: &str = "verus/machine-model/exception_scalar_core_wrapper.rs";
const MODEL_CONSUMER: &str = "tests/m1/exception_scalar_core_wrapper_consumer.rs";
const ADAPTER_CONSUMER: &str = "tests/m1/exception_scalar_adapter_consumer.rs";
const FREESTANDING: &str = "tests/m1/exception_scalar_core_wrapper_freestanding.rs";
const LINKER: &str = "kernel-host/link/m1_exception_scalar_core_wrapper.ld";
const SCALAR_SHELL: &str = "tests/m1/exception_scalar_shell.rs";
const SCALAR_ARTIFACT: &str = "artifact/libtmk_exception_scalar.rlib";
const PANIC_RLIB: &str = "build/m0-host/libtmk_panic_host.rlib";
const PANIC_REPORT: &str = "build/m0-host/report.txt";
const PRIMITIVES: &str = "build/m0-platform-primitives/objects/platform-primitives.o";
const PRIMITIVES_REPORT: &str = "build/m0-platform-primitives/report.txt";
const MODEL_CRATE: &str = "tmk_exception_scalar_core_wrapper";
const MODEL_RLIB: &str = "libtmk_exception_scalar_core_wrapper.rlib";
const MODEL_MARKER: &str = "M1_EXCEPTION_SCALAR_CORE_WRAPPER_OK images=35,314,4,5 scenarios=10 rejected=4 routes=return,schedule-fail-closed,fail-stop cross-check=frame-vs-register";
const ADAPTER_MARKER: &str = "M1_EXCEPTION_SCALAR_ADAPTER_OK layout=640 offsets=0,112,184,192,384,600,632 scenarios=page-fault,mismatch,bad-snapshot";
const GS_SHA256: &str = "278f11cd2e36f9f095c5bef8639f2759481ef0818c1cf922f8e3dc866869f8af";
const WRAPPER_SHA256: &str = "7438c9b75ccf80276c37f625355c7c0c226b3f1ed1c1cda06f330c0539a5035c";
const FAIL_STOP_SHA256: &str = "a7413110d0afeaa3ef808b851d8c1c7cdd074fbface71b286cf1fde0d19dd226";
const SCHEDULE_SHA256: &str = "9dddea87b38bb2b72eaa40f5738b6c57dbb1ef3fa62d3ca201da507a79fcaeec";
const MEMCPY_SHA256: &str = "00d0174466d21d8a224c588f4bf2e324a605806fac0cb0d4fa2ba1a9667a49a9";

struct Tools {
    forge: PathBuf,
    verus: PathBuf,
    rustc: PathBuf,
    cc: PathBuf,
    lld: PathBuf,
    objcopy: PathBuf,
    objdump: PathBuf,
    readelf: PathBuf,
    nm: PathBuf,
}

impl Tools {
    fn pinned() -> Result<Self, String> {
        let tools = Self {
            forge: forge_binary()?,
            verus: PathBuf::from("/opt/verus/0.2026.05.24.ecee80a/verus"),
            rustc: PathBuf::from(
                "/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc",
            ),
            cc: PathBuf::from("/usr/sbin/cc"),
            lld: PathBuf::from(
                "/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld/ld.lld",
            ),
            objcopy: PathBuf::from("/usr/sbin/objcopy"),
            objdump: PathBuf::from("/usr/sbin/objdump"),
            readelf: PathBuf::from("/usr/sbin/readelf"),
            nm: PathBuf::from("/usr/sbin/nm"),
        };
        for (path, expected, label) in [
            (
                tools.forge.as_path(),
                FORGE_SHA256,
                "candidate Forge executable",
            ),
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
                tools.cc.as_path(),
                "1ce580ecfabf35747bc550481621e2f2c04fd8fc23b8182779f33b82d07856d0",
                "GCC linker driver",
            ),
            (
                tools.lld.as_path(),
                "e7d44b7571a8250326e99aa238aa4be7ddaa1fa696cf3c0e2ca829da5846b325",
                "Rust LLD",
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

struct ModelBuild {
    rlib: PathBuf,
    consumer: PathBuf,
    images: PathBuf,
}

struct LinkedBuild {
    elf: PathBuf,
    map: PathBuf,
    directory: PathBuf,
}

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let tools = Tools::pinned()?;
    validate_candidate_pin(&tools.forge)?;
    check_forge_skill(&tools.forge)?;
    for (relative, label) in [
        (MODEL_SOURCE, "scalar-core wrapper Verus model"),
        (MODEL_CONSUMER, "scalar-core wrapper model consumer"),
        (ADAPTER_CONSUMER, "receipt-bound adapter consumer"),
        (FREESTANDING, "scalar-core freestanding link root"),
        (LINKER, "scalar-core fixed-address linker"),
        (SCALAR_SHELL, "scalar adapter direct-Verus shell"),
        (PANIC_RLIB, "verified panic lang-item rlib"),
        (PANIC_REPORT, "verified panic report"),
        (PRIMITIVES, "verified platform primitive object"),
        (PRIMITIVES_REPORT, "verified platform primitive report"),
    ] {
        require_file(&root.join(relative), label)?;
    }
    validate_prerequisites(&root)?;
    audit_sources(&root)?;

    super::m1_exception_scalar::run()?;
    let scalar_work = root.join("build/m1-exception-scalar");
    let bundles = [
        scalar_work.join("primary.verified"),
        scalar_work.join("repro-a.verified"),
        scalar_work.join("repro-b.verified"),
    ];
    let scalar_report = read(&scalar_work.join("report.txt"))?;
    for required in [
        "component_verified=true",
        "cr2_retained_in_r10=true",
        "scalar_entry_bytes=11",
        "scalar_entry_instruction=mov-rdi-r10;mov-rbx-rdi;tail-jump",
    ] {
        if !scalar_report.contains(required) {
            return Err(format!(
                "scalar prerequisite report is missing `{required}`"
            ));
        }
    }
    let receipt = read_json(&bundles[0].join("receipt.json"), "scalar adapter receipt")?;
    let scalar_binding = json_string(&receipt, "/binding_sha256", "scalar binding")?.to_string();
    let scalar_artifact_sha =
        json_string(&receipt, "/binding/artifact/sha256", "scalar artifact")?.to_string();
    for bundle in &bundles {
        let verified = verify_bundle(&tools.forge, &root, bundle, false)?;
        if json_string(&verified, "/binding_sha256", "scalar validation")? != scalar_binding {
            return Err("scalar wrapper prerequisite receipt binding drifted".to_string());
        }
    }

    let work = root.join("build/m1-exception-scalar-core-wrapper");
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
    let mut models = Vec::new();
    for (index, directory) in model_dirs.iter().enumerate() {
        models.push(build_model(&tools, &root, directory, index, index == 0)?);
    }
    let model_sha = same_digest(
        &models
            .iter()
            .map(|model| model.rlib.clone())
            .collect::<Vec<_>>(),
        "scalar-core wrapper model",
    )?;
    let model_consumer_sha = same_digest(
        &models
            .iter()
            .map(|model| model.consumer.clone())
            .collect::<Vec<_>>(),
        "scalar-core wrapper consumer",
    )?;
    validate_images(&models)?;

    let adapter_consumers = run_adapter_consumers(&tools, &root, &work, &bundles)?;
    let adapter_consumer_sha = same_digest(&adapter_consumers, "scalar adapter consumer")?;

    let link_dirs = [
        work.join("link-primary"),
        work.join("link-repro-a"),
        work.join("link-repro-b"),
    ];
    let mut linked = Vec::new();
    for index in 0..3 {
        linked.push(link_kernel(
            &tools,
            &root,
            &models[index].images,
            &scalar_work.join(format!("scalar-entry-{}.bin", index + 1)),
            &bundles[index].join(SCALAR_ARTIFACT),
            &link_dirs[index],
            &root.join(LINKER),
        )?);
    }
    let linked_elf_sha = same_digest(
        &linked
            .iter()
            .map(|item| item.elf.clone())
            .collect::<Vec<_>>(),
        "scalar-core fixed-address ELF",
    )?;
    let audit = audit_linked(&tools, &root, &linked[0], &work)?;
    for item in linked.iter().skip(1) {
        audit_linked(&tools, &root, item, &item.directory.join("audit"))?;
    }

    run_model_negatives(&tools, &root, &work)?;
    run_link_negatives(
        &tools,
        &root,
        &work,
        &models[0].images,
        &scalar_work.join("scalar-entry-1.bin"),
        &bundles[0].join(SCALAR_ARTIFACT),
    )?;

    let report = format!(
        "M1_EXCEPTION_SCALAR_CORE_WRAPPER_OK\ncomponent_verified=true\nrelease_eligible=false\nhardware_executed=false\nqemu_executed=false\ncandidate_pin_verified=true\nscalar_prerequisite_replayed=true\nper_cpu_gs_setup_present=true\nper_cpu_lookup_wrapper_present=true\nscalar_core_fixed_address_linked=true\nscalar_adapter_receipt_bound=true\nscalar_adapter_executed=true\nframe_register_cross_check=true\nfail_stop_present=true\nschedule_backend_present=false\nschedule_route=registered-fail-stop-stub\nmodel_source_sha256={}\nmodel_consumer_source_sha256={}\nadapter_consumer_source_sha256={}\nfreestanding_source_sha256={}\nlinker_script_sha256={}\nscalar_shell_sha256={}\nscalar_binding_sha256={scalar_binding}\nscalar_artifact_sha256={scalar_artifact_sha}\nmodel_artifact_sha256={model_sha}\nmodel_consumer_sha256={model_consumer_sha}\nadapter_consumer_sha256={adapter_consumer_sha}\nlinked_elf_sha256={linked_elf_sha}\nlinked_adapter_sha256={}\nlinked_runtime_sha256={}\nlinked_memcpy_sha256={}\npanic_artifact_sha256={}\nplatform_primitive_object_sha256={}\nforge_source_identity={THERMITE_COMMIT}\nforge_sha256={FORGE_SHA256}\nverus_verified=30\nmodel_reproducibility_builds=3\nmodel_consumer_reproducibility_builds=3\nadapter_consumer_executions=3\npost_link_reproducibility_builds=3\ngs_setup_virtual=ffffffff80001040\ngs_setup_bytes=35\nscalar_entry_virtual=ffffffff80011200\nscalar_entry_bytes=11\nscalar_wrapper_virtual=ffffffff80011300\nscalar_wrapper_bytes=314\nfail_stop_virtual=ffffffff80011500\nfail_stop_bytes=4\nschedule_stub_virtual=ffffffff80011600\nschedule_stub_bytes=5\nscalar_adapter_virtual=ffffffff80012000\nscalar_adapter_bytes=1885\nscalar_core_block_bytes=640\nscalar_core_block_layout=80-u64-slots\nscalar_core_block_offsets=frame-cr2:112,word-count:184,args:192,policy:384,outcome:600\ngs_header_offsets=self:0,core-block:8,active-frame:16,flags:24\ngs_header_flags=00000000000001ff\nwrapper_runtime_marker={MODEL_MARKER}\nadapter_runtime_marker={ADAPTER_MARKER}\nnegative_cases=frame-binding,kernel-tail,fail-stop-route,bad-assume,wrapper-byte,wrapper-extra-byte,adapter-address\n",
        sha256sum(&root.join(MODEL_SOURCE))?,
        sha256sum(&root.join(MODEL_CONSUMER))?,
        sha256sum(&root.join(ADAPTER_CONSUMER))?,
        sha256sum(&root.join(FREESTANDING))?,
        sha256sum(&root.join(LINKER))?,
        sha256sum(&root.join(SCALAR_SHELL))?,
        audit.adapter_sha,
        audit.runtime_sha,
        audit.memcpy_sha,
        sha256sum(&root.join(PANIC_RLIB))?,
        sha256sum(&root.join(PRIMITIVES))?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write scalar-core wrapper report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
}

fn validate_prerequisites(root: &Path) -> Result<(), String> {
    let panic_report = read(&root.join(PANIC_REPORT))?;
    let panic_sha = sha256sum(&root.join(PANIC_RLIB))?;
    for required in [
        "component_verified=true".to_string(),
        format!("panic_artifact_sha256={panic_sha}"),
        "proof_reproducibility_builds=3".to_string(),
    ] {
        if !panic_report.contains(&required) {
            return Err(format!("panic prerequisite report is missing `{required}`"));
        }
    }
    let primitive_report = read(&root.join(PRIMITIVES_REPORT))?;
    let primitive_sha = sha256sum(&root.join(PRIMITIVES))?;
    for required in [
        "component_verified=true".to_string(),
        "linked_primitives_verified=true".to_string(),
        format!("primitive_object_sha256={primitive_sha}"),
        format!("memcpy_capsule_sha256={MEMCPY_SHA256}"),
    ] {
        if !primitive_report.contains(&required) {
            return Err(format!(
                "platform primitive prerequisite report is missing `{required}`"
            ));
        }
    }
    Ok(())
}

fn audit_sources(root: &Path) -> Result<(), String> {
    let model = read(&root.join(MODEL_SOURCE))?;
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
        if model.contains(forbidden) {
            return Err(format!("scalar-core wrapper model contains `{forbidden}`"));
        }
    }
    for required in [
        "pub const GS_SETUP_VIRTUAL: u64 = 0xffff_ffff_8000_1040;",
        "pub const SCALAR_WRAPPER_VIRTUAL: u64 = 0xffff_ffff_8001_1300;",
        "pub const ADAPTER_VIRTUAL: u64 = 0xffff_ffff_8001_2000;",
        "pub open spec fn transport_matches_frame",
        "state.frame_cs == USER_CODE_SELECTOR ==> state.frame_user_tail_registered",
        "state.core_block_writable_bytes >= SCALAR_CORE_BLOCK_BYTES",
        "state.adapter_receipt_bound",
        "pub fn install_gs",
        "pub fn decode_execute_wrapper",
        "result.fail_stopped == !result.returned",
        "ensures result == 4095",
    ] {
        if !model.contains(required) {
            return Err(format!("scalar-core wrapper model is missing `{required}`"));
        }
    }
    let model_consumer = read(&root.join(MODEL_CONSUMER))?;
    for required in [
        MODEL_MARKER,
        "frame_user_tail_registered: false",
        "transport_cr2: 0x9999_9000",
        "adapter_registered: false",
        "core_block_exclusive: false",
    ] {
        if !model_consumer.contains(required) {
            return Err(format!("wrapper consumer is missing `{required}`"));
        }
    }
    let adapter_consumer = read(&root.join(ADAPTER_CONSUMER))?;
    for required in [
        ADAPTER_MARKER,
        "align_of::<ScalarCoreBlock>(), 8",
        "size_of::<ScalarCoreBlock>(), 640",
        "offset_of!(ScalarCoreBlock, slot_79), 632",
        "tmk_exception_scalar_adapter(&mut page)",
        "bad_snapshot.slot_74, 100",
    ] {
        if !adapter_consumer.contains(required) {
            return Err(format!("adapter consumer is missing `{required}`"));
        }
    }
    let scalar_shell = read(&root.join(SCALAR_SHELL))?;
    for required in [
        "pub struct ScalarCoreBlock",
        "pub fn tmk_exception_scalar_adapter",
    ] {
        if !scalar_shell.contains(required) {
            return Err(format!("scalar adapter shell is missing `{required}`"));
        }
    }
    let freestanding = read(&root.join(FREESTANDING))?;
    if freestanding.contains("panic_handler")
        || freestanding.contains("unsafe")
        || !freestanding.contains("extern crate tmk_panic_host;")
    {
        return Err("freestanding root must delegate only to the verified panic rlib".to_string());
    }
    let linker = read(&root.join(LINKER))?;
    for required in [
        ". = 0xffffffff80001040;",
        ". = 0xffffffff80011200;",
        ". = 0xffffffff80011300;",
        ". = 0xffffffff80011500;",
        ". = 0xffffffff80011600;",
        ". = 0xffffffff80012000;",
        "rust_eh_personality = tmk_exception_fail_stop;",
        "SIZEOF(.text.tmk_exception_scalar_adapter) == 0x75d",
        "SIZEOF(.text.tmk_memcpy_capsule) == 9",
    ] {
        if !linker.contains(required) {
            return Err(format!("scalar-core linker is missing `{required}`"));
        }
    }
    Ok(())
}

fn verus_command(tools: &Tools, directory: &Path, compile: bool) -> Command {
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
        .arg(format!("{MODEL_CRATE}.rs"));
    command
}

fn build_model(
    tools: &Tools,
    root: &Path,
    directory: &Path,
    index: usize,
    retain_result: bool,
) -> Result<ModelBuild, String> {
    fs::create_dir_all(directory).map_err(|error| format!("create wrapper model path: {error}"))?;
    fs::copy(
        root.join(MODEL_SOURCE),
        directory.join(format!("{MODEL_CRATE}.rs")),
    )
    .map_err(|error| format!("stage wrapper model: {error}"))?;
    let output = run_checked(
        &mut verus_command(tools, directory, true),
        "Verus scalar-core wrapper proof and codegen",
    )?;
    require_output_fragments(
        &output.stdout,
        "scalar-core wrapper Verus result",
        &[
            "\"success\": true",
            "\"verified\": 30",
            "\"errors\": 0",
            "\"is-verifying-entire-crate\": true",
            "\"version\": \"0.2026.05.24.ecee80a\"",
        ],
    )?;
    if retain_result {
        fs::write(
            directory.join("verus-result.json"),
            canonical_json(&output.stdout, "wrapper model Verus result")?,
        )
        .map_err(|error| format!("write wrapper model Verus result: {error}"))?;
    }
    let rlib = directory.join(MODEL_RLIB);
    require_file(&rlib, "compiled scalar-core wrapper model")?;
    let consumer = directory.join("consumer");
    run_checked(
        Command::new(&tools.rustc)
            .current_dir(root)
            .env("SOURCE_DATE_EPOCH", "0")
            .args(["--edition=2021"])
            .arg(MODEL_CONSUMER)
            .arg("--extern")
            .arg(format!("{MODEL_CRATE}={}", rlib.display()))
            .args(["-L", "dependency=/opt/verus/0.2026.05.24.ecee80a"])
            .args(["-C", "panic=abort"])
            .args(["-C", "codegen-units=1"])
            .arg(format!("--remap-path-prefix={}=.", root.display()))
            .arg("-o")
            .arg(&consumer),
        "compile scalar-core wrapper consumer",
    )?;
    let images = directory.join("images");
    let runtime = run_checked(
        Command::new(&consumer).current_dir(root).arg(&images),
        "execute scalar-core wrapper model",
    )?;
    require_output_fragments(&runtime.stdout, "wrapper model runtime", &[MODEL_MARKER])?;
    write_combined_output(
        &directory.join(format!("runtime-{}.txt", index + 1)),
        &runtime,
        "wrapper model runtime evidence",
    )?;
    Ok(ModelBuild {
        rlib,
        consumer,
        images,
    })
}

fn validate_images(models: &[ModelBuild]) -> Result<(), String> {
    for model in models {
        for (name, expected, size) in [
            ("gs-setup.bin", GS_SHA256, 35u64),
            ("scalar-wrapper.bin", WRAPPER_SHA256, 314),
            ("fail-stop.bin", FAIL_STOP_SHA256, 4),
            ("schedule-unavailable.bin", SCHEDULE_SHA256, 5),
        ] {
            let path = model.images.join(name);
            require_file(&path, "emitted wrapper capsule")?;
            let metadata =
                fs::metadata(&path).map_err(|error| format!("stat {}: {error}", path.display()))?;
            let actual = sha256sum(&path)?;
            if metadata.len() != size || actual != expected {
                return Err(format!(
                    "wrapper image {name} is {} bytes/{actual}, expected {size}/{expected}",
                    metadata.len()
                ));
            }
        }
    }
    Ok(())
}

fn run_adapter_consumers(
    tools: &Tools,
    root: &Path,
    work: &Path,
    bundles: &[PathBuf; 3],
) -> Result<Vec<PathBuf>, String> {
    let mut executables = Vec::new();
    for (index, bundle) in bundles.iter().enumerate() {
        let executable = work.join(format!("adapter-consumer-{}", index + 1));
        run_checked(
            Command::new(&tools.rustc)
                .current_dir(root)
                .env("SOURCE_DATE_EPOCH", "0")
                .args(["--edition=2021"])
                .arg(ADAPTER_CONSUMER)
                .arg("--extern")
                .arg(format!(
                    "tmk_exception_scalar={}",
                    bundle.join(SCALAR_ARTIFACT).display()
                ))
                .arg("-L")
                .arg(format!(
                    "dependency={}",
                    bundle.join("artifact/deps").display()
                ))
                .args(["-C", "panic=abort"])
                .args(["-C", "codegen-units=1"])
                .arg(format!("--remap-path-prefix={}=.", root.display()))
                .arg("-o")
                .arg(&executable),
            "compile scalar adapter consumer",
        )?;
        let runtime = run_checked(
            Command::new(&executable).current_dir(root),
            "execute receipt-bound scalar adapter",
        )?;
        require_output_fragments(&runtime.stdout, "adapter runtime", &[ADAPTER_MARKER])?;
        write_combined_output(
            &work.join(format!("adapter-runtime-{}.txt", index + 1)),
            &runtime,
            "adapter runtime evidence",
        )?;
        executables.push(executable);
    }
    Ok(executables)
}

fn wrap_image(
    tools: &Tools,
    directory: &Path,
    name: &str,
    section: &str,
) -> Result<PathBuf, String> {
    let object = directory.join(format!("{name}.o"));
    run_checked(
        Command::new(&tools.objcopy)
            .current_dir(directory)
            .args(["-I", "binary", "-O", "elf64-x86-64", "-B", "i386:x86-64"])
            .arg("--rename-section")
            .arg(format!(".data={section},alloc,load,readonly,code,contents"))
            .arg(format!("{name}.bin"))
            .arg(format!("{name}.o")),
        "wrap scalar-core capsule bytes",
    )?;
    Ok(object)
}

fn stage_objects(
    tools: &Tools,
    images: &Path,
    entry: &Path,
    directory: &Path,
) -> Result<Vec<PathBuf>, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("create scalar-core link path: {error}"))?;
    for (source, name) in [
        (images.join("gs-setup.bin"), "gs.bin"),
        (entry.to_path_buf(), "entry.bin"),
        (images.join("scalar-wrapper.bin"), "wrapper.bin"),
        (images.join("fail-stop.bin"), "fail.bin"),
        (images.join("schedule-unavailable.bin"), "schedule.bin"),
    ] {
        fs::copy(&source, directory.join(name))
            .map_err(|error| format!("stage {}: {error}", source.display()))?;
    }
    let mut objects = Vec::new();
    for (name, section) in [
        ("gs", ".text.tmk_per_cpu_gs_setup"),
        ("entry", ".text.tmk_exception_scalar_entry"),
        ("wrapper", ".text.tmk_exception_scalar_wrapper"),
        ("fail", ".text.tmk_exception_fail_stop"),
        ("schedule", ".text.tmk_exception_schedule_unavailable"),
    ] {
        objects.push(wrap_image(tools, directory, name, section)?);
    }
    Ok(objects)
}

fn link_command(
    tools: &Tools,
    root: &Path,
    directory: &Path,
    adapter: &Path,
    linker: &Path,
    objects: &[PathBuf],
    output: &Path,
    map: &Path,
) -> Command {
    let mut command = Command::new(&tools.rustc);
    command
        .current_dir(root)
        .env("SOURCE_DATE_EPOCH", "0")
        .args(["--edition=2021"])
        .arg(FREESTANDING)
        .arg("--extern")
        .arg(format!(
            "tmk_panic_host={}",
            root.join(PANIC_RLIB).display()
        ))
        .args(["-L", "dependency=/opt/verus/0.2026.05.24.ecee80a"])
        .args(["-C", "panic=abort"])
        .args(["-C", "code-model=kernel"])
        .args(["-C", "relocation-model=static"])
        .args(["-C", "no-redzone=yes"])
        .arg("-C")
        .arg(format!("linker={}", tools.cc.display()))
        .args(["-C", "link-arg=-nostartfiles"])
        .args(["-C", "link-arg=-no-pie"])
        .args(["-C", "link-arg=-static"])
        .args(["-C", "link-arg=-Wl,--build-id=none"])
        .args(["-C", "link-arg=-Wl,--gc-sections"])
        .args(["-C", "link-arg=-Wl,--whole-archive"])
        .arg("-C")
        .arg(format!("link-arg={}", adapter.display()))
        .args(["-C", "link-arg=-Wl,--no-whole-archive"])
        .arg("-C")
        .arg(format!("link-arg={}", root.join(PRIMITIVES).display()));
    for object in objects {
        command
            .arg("-C")
            .arg(format!("link-arg={}", object.display()));
    }
    command
        .arg("-C")
        .arg(format!("link-arg=-T{}", linker.display()))
        .arg("-C")
        .arg(format!("link-arg=-Wl,-Map={}", map.display()))
        .arg(format!("--remap-path-prefix={}=.", root.display()))
        .arg(format!("--remap-path-prefix={}=.", directory.display()))
        .arg("-o")
        .arg(output);
    command
}

fn link_kernel(
    tools: &Tools,
    root: &Path,
    images: &Path,
    entry: &Path,
    adapter: &Path,
    directory: &Path,
    linker: &Path,
) -> Result<LinkedBuild, String> {
    let objects = stage_objects(tools, images, entry, directory)?;
    let elf = directory.join("kernel.elf");
    let map = directory.join("kernel.map");
    run_checked(
        &mut link_command(
            tools, root, directory, adapter, linker, &objects, &elf, &map,
        ),
        "link scalar-core fixed-address ELF",
    )?;
    Ok(LinkedBuild {
        elf,
        map,
        directory: directory.to_path_buf(),
    })
}

struct LinkAudit {
    adapter_sha: String,
    runtime_sha: String,
    memcpy_sha: String,
}

fn audit_linked(
    tools: &Tools,
    root: &Path,
    linked: &LinkedBuild,
    evidence: &Path,
) -> Result<LinkAudit, String> {
    fs::create_dir_all(evidence)
        .map_err(|error| format!("create link audit path {}: {error}", evidence.display()))?;
    let header = run_checked(
        Command::new(&tools.readelf).args(["-hW"]).arg(&linked.elf),
        "inspect scalar-core ELF header",
    )?;
    require_output_fragments(
        &header.stdout,
        "scalar-core ELF header",
        &[
            "ELF64",
            "Advanced Micro Devices X86-64",
            "0xffffffff80011200",
        ],
    )?;
    let sections = run_checked(
        Command::new(&tools.readelf).args(["-SW"]).arg(&linked.elf),
        "inspect scalar-core ELF sections",
    )?;
    let section_text = String::from_utf8_lossy(&sections.stdout);
    let executable: Vec<_> = section_text
        .lines()
        .filter(|line| line.contains(" AX "))
        .collect();
    let expected = [
        (".text.tmk_per_cpu_gs_setup", "ffffffff80001040", "000023"),
        (
            ".text.tmk_exception_scalar_entry",
            "ffffffff80011200",
            "00000b",
        ),
        (
            ".text.tmk_exception_scalar_wrapper",
            "ffffffff80011300",
            "00013a",
        ),
        (
            ".text.tmk_exception_fail_stop",
            "ffffffff80011500",
            "000004",
        ),
        (
            ".text.tmk_exception_schedule_unavailable",
            "ffffffff80011600",
            "000005",
        ),
        (
            ".text.tmk_exception_scalar_adapter",
            "ffffffff80012000",
            "00075d",
        ),
        (".text.tmk_memcpy_capsule", "ffffffff80012760", "000009"),
        (
            ".text.tmk_exception_scalar_runtime",
            "ffffffff80012770",
            "0032e9",
        ),
    ];
    if executable.len() != expected.len()
        || expected.iter().any(|(name, address, size)| {
            !executable
                .iter()
                .any(|line| line.contains(name) && line.contains(address) && line.contains(size))
        })
    {
        return Err(format!(
            "scalar-core executable-section audit failed: {executable:?}"
        ));
    }
    let relocations = run_checked(
        Command::new(&tools.readelf).args(["-rW"]).arg(&linked.elf),
        "inspect scalar-core relocations",
    )?;
    require_output_fragments(
        &relocations.stdout,
        "scalar-core relocations",
        &["There are no relocations in this file."],
    )?;
    let undefined = run_checked(
        Command::new(&tools.nm).arg("-u").arg(&linked.elf),
        "inspect scalar-core undefined symbols",
    )?;
    if !undefined.stdout.is_empty() {
        return Err(format!(
            "scalar-core ELF has undefined symbols:\n{}",
            String::from_utf8_lossy(&undefined.stdout)
        ));
    }
    let symbols = run_checked(
        Command::new(&tools.nm).args(["-nSC"]).arg(&linked.elf),
        "inspect scalar-core symbols",
    )?;
    require_output_fragments(
        &symbols.stdout,
        "scalar-core symbols",
        &[
            "ffffffff80001040",
            "tmk_per_cpu_gs_setup",
            "ffffffff80011200",
            "tmk_exception_scalar_entry",
            "ffffffff80011300",
            "tmk_exception_scalar_wrapper",
            "ffffffff80011500",
            "tmk_exception_fail_stop",
            "rust_eh_personality",
            "ffffffff80011600",
            "tmk_exception_schedule",
            "ffffffff80012000",
            "tmk_exception_scalar_adapter",
            "ffffffff80012760",
            "memcpy",
        ],
    )?;
    let symbol_text = String::from_utf8_lossy(&symbols.stdout);
    if symbol_text.matches("tmk_exception_scalar_adapter").count() != 2
        || !symbol_text.contains("000000000000075d")
    {
        return Err("scalar adapter alias/mangled symbol size audit failed".to_string());
    }
    let disassembly = run_checked(
        Command::new(&tools.objdump).arg("-d").arg(&linked.elf),
        "disassemble scalar-core ELF",
    )?;
    require_output_fragments(
        &disassembly.stdout,
        "scalar-core disassembly",
        &[
            "wrmsr",
            "mov    %rdi,%r10",
            "mov    %rbx,%rdi",
            "mov    %gs:0x0,%rax",
            "cmp    %gs:0x10,%rdi",
            "mov    0x70(%rdi),%rax",
            "mov    %r10,0xc0(%r11)",
            "call   ffffffff80012000",
            "je     ffffffff80011600",
            "jmp    ffffffff80011500",
            "sub    $0x2e8,%rsp",
            "cli",
            "hlt",
        ],
    )?;
    let disassembly_text = String::from_utf8_lossy(&disassembly.stdout);
    if disassembly_text.matches("wrmsr").count() != 2
        || disassembly_text.matches("call   ffffffff80012000").count() != 1
        || disassembly_text.matches("jmp    ffffffff80011500").count() < 3
    {
        return Err("scalar-core control-transfer count audit failed".to_string());
    }
    let map = read(&linked.map)?;
    for required in [
        "libtmk_exception_scalar.rlib",
        "libtmk_panic_host.rlib",
        "platform-primitives.o",
        "gs.o",
        "entry.o",
        "wrapper.o",
        "fail.o",
        "schedule.o",
        "libcore-",
    ] {
        if !map.contains(required) {
            return Err(format!("scalar-core link map is missing `{required}`"));
        }
    }
    if map.contains(".text.unregistered") {
        return Err("scalar-core link map contains unregistered executable input".to_string());
    }

    let extracted = evidence.join("sections");
    fs::create_dir_all(&extracted)
        .map_err(|error| format!("create extracted-section path: {error}"))?;
    for (section, file) in [
        (".text.tmk_per_cpu_gs_setup", "gs.bin"),
        (".text.tmk_exception_scalar_entry", "entry.bin"),
        (".text.tmk_exception_scalar_wrapper", "wrapper.bin"),
        (".text.tmk_exception_fail_stop", "fail.bin"),
        (".text.tmk_exception_schedule_unavailable", "schedule.bin"),
        (".text.tmk_exception_scalar_adapter", "adapter.bin"),
        (".text.tmk_memcpy_capsule", "memcpy.bin"),
        (".text.tmk_exception_scalar_runtime", "runtime.bin"),
    ] {
        run_checked(
            Command::new(&tools.objcopy)
                .arg("--dump-section")
                .arg(format!("{section}={}", extracted.join(file).display()))
                .arg(&linked.elf),
            "extract scalar-core linked section",
        )?;
    }
    for (linked_name, staged_name) in [
        ("gs.bin", "gs.bin"),
        ("entry.bin", "entry.bin"),
        ("wrapper.bin", "wrapper.bin"),
        ("fail.bin", "fail.bin"),
        ("schedule.bin", "schedule.bin"),
    ] {
        let linked_bytes = fs::read(extracted.join(linked_name))
            .map_err(|error| format!("read extracted {linked_name}: {error}"))?;
        let staged = fs::read(linked.directory.join(staged_name))
            .map_err(|error| format!("read staged {staged_name}: {error}"))?;
        if linked_bytes != staged {
            return Err(format!(
                "linked {linked_name} does not match verified input"
            ));
        }
    }
    let memcpy_sha = sha256sum(&extracted.join("memcpy.bin"))?;
    if memcpy_sha != MEMCPY_SHA256
        || fs::read(extracted.join("memcpy.bin"))
            .map_err(|error| format!("read linked memcpy: {error}"))?
            != fs::read(root.join("build/m0-platform-primitives/emitted/memcpy.bin"))
                .map_err(|error| format!("read registered memcpy: {error}"))?
    {
        return Err("linked memcpy does not match the registered verified capsule".to_string());
    }
    for (name, output) in [
        ("header.txt", &header),
        ("sections.txt", &sections),
        ("relocations.txt", &relocations),
        ("symbols.txt", &symbols),
        ("disassembly.txt", &disassembly),
    ] {
        write_combined_output(&evidence.join(name), output, "scalar-core link evidence")?;
    }
    fs::copy(&linked.map, evidence.join("kernel.map"))
        .map_err(|error| format!("copy scalar-core map: {error}"))?;
    Ok(LinkAudit {
        adapter_sha: sha256sum(&extracted.join("adapter.bin"))?,
        runtime_sha: sha256sum(&extracted.join("runtime.bin"))?,
        memcpy_sha,
    })
}

fn run_model_negatives(tools: &Tools, root: &Path, work: &Path) -> Result<(), String> {
    let source = read(&root.join(MODEL_SOURCE))?;
    let cases = [
        (
            "frame-binding",
            source.replacen(
                "result.block_frame_cr2 == state.frame_cr2\n                && result.block_arg_cr2 == state.transport_cr2,",
                "result.block_frame_cr2 == state.transport_cr2\n                && result.block_arg_cr2 == state.transport_cr2,",
                1,
            ),
        ),
        (
            "kernel-tail",
            source.replacen(
                "block_frame_user_rsp: if user { state.frame_user_rsp } else { 0 },",
                "block_frame_user_rsp: state.frame_user_rsp,",
                1,
            ),
        ),
        (
            "fail-stop-route",
            source.replacen(
                "result.accepted ==> result.fail_stopped == !result.returned,",
                "result.accepted ==> result.fail_stopped == result.returned,",
                1,
            ),
        ),
        (
            "bad-assume",
            source.replacen(
                "pub fn wrapper_observation() -> (result: u64)\n    ensures result == 4095,\n{",
                "pub fn wrapper_observation() -> (result: u64)\n    ensures result == 4095,\n{\n    assume(false);",
                1,
            ),
        ),
    ];
    for (name, mutated) in cases {
        if mutated == source {
            return Err(format!("could not construct wrapper `{name}` negative"));
        }
        let directory = work.join(format!("proof-negative-{name}"));
        fs::create_dir_all(&directory)
            .map_err(|error| format!("create wrapper negative path: {error}"))?;
        fs::write(directory.join(format!("{MODEL_CRATE}.rs")), mutated)
            .map_err(|error| format!("write wrapper {name} negative: {error}"))?;
        let output = run_expect_failure(
            &mut verus_command(tools, &directory, false),
            &format!("scalar-core wrapper {name} proof negative"),
        )?;
        write_combined_output(
            &work.join(format!("negative-{name}.txt")),
            &output,
            "wrapper proof-negative evidence",
        )?;
    }
    Ok(())
}

fn run_link_negatives(
    tools: &Tools,
    root: &Path,
    work: &Path,
    images: &Path,
    entry: &Path,
    adapter: &Path,
) -> Result<(), String> {
    let mutation = work.join("negative-wrapper-byte.bin");
    let mut bytes = fs::read(images.join("scalar-wrapper.bin"))
        .map_err(|error| format!("read wrapper for mutation: {error}"))?;
    bytes[0] ^= 1;
    fs::write(&mutation, bytes).map_err(|error| format!("write wrapper byte mutation: {error}"))?;
    let mutated_sha = sha256sum(&mutation)?;
    if mutated_sha == WRAPPER_SHA256 {
        return Err("wrapper byte mutation retained the accepted digest".to_string());
    }
    fs::write(
        work.join("negative-wrapper-byte.txt"),
        format!("rejected_sha256={mutated_sha}\nexpected_sha256={WRAPPER_SHA256}\n"),
    )
    .map_err(|error| format!("write wrapper byte negative evidence: {error}"))?;

    let extra = work.join("link-negative-wrapper-extra-byte");
    let objects = stage_objects(tools, images, entry, &extra)?;
    let mut wrapper = fs::read(extra.join("wrapper.bin"))
        .map_err(|error| format!("read staged wrapper negative: {error}"))?;
    wrapper.push(0x90);
    fs::write(extra.join("wrapper.bin"), wrapper)
        .map_err(|error| format!("write extra-byte wrapper: {error}"))?;
    wrap_image(
        tools,
        &extra,
        "wrapper",
        ".text.tmk_exception_scalar_wrapper",
    )?;
    let extra_objects = objects
        .into_iter()
        .map(|path| {
            if path.file_name().and_then(|name| name.to_str()) == Some("wrapper.o") {
                extra.join("wrapper.o")
            } else {
                path
            }
        })
        .collect::<Vec<_>>();
    let output = run_expect_failure(
        &mut link_command(
            tools,
            root,
            &extra,
            adapter,
            &root.join(LINKER),
            &extra_objects,
            &extra.join("kernel.elf"),
            &extra.join("kernel.map"),
        ),
        "scalar-core wrapper extra-byte link negative",
    )?;
    write_combined_output(
        &work.join("negative-wrapper-extra-byte.txt"),
        &output,
        "wrapper extra-byte link evidence",
    )?;

    let address = work.join("link-negative-adapter-address");
    let objects = stage_objects(tools, images, entry, &address)?;
    let linker_text = read(&root.join(LINKER))?;
    let mutated = linker_text.replacen(". = 0xffffffff80012000;", ". = 0xffffffff80012100;", 1);
    if mutated == linker_text {
        return Err("could not construct adapter-address linker negative".to_string());
    }
    let linker = address.join("adapter-address.ld");
    fs::write(&linker, mutated)
        .map_err(|error| format!("write adapter-address linker negative: {error}"))?;
    let output = run_expect_failure(
        &mut link_command(
            tools,
            root,
            &address,
            adapter,
            &linker,
            &objects,
            &address.join("kernel.elf"),
            &address.join("kernel.map"),
        ),
        "scalar-core adapter-address link negative",
    )?;
    write_combined_output(
        &work.join("negative-adapter-address.txt"),
        &output,
        "adapter-address link evidence",
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
