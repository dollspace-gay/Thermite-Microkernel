use super::{
    canonical_json, direct_verus_command, require_file, require_output_fragments, run_checked,
    run_expect_failure, sha256sum, uefi, workspace_root, write_combined_output,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MARKER: &[u8; 16] = b"TMK_M0_UEFI_OK!\n";

pub(super) struct Tools {
    pub(super) verus: PathBuf,
    pub(super) rustc: PathBuf,
    pub(super) ld: PathBuf,
    pub(super) objcopy: PathBuf,
    pub(super) objdump: PathBuf,
    pub(super) qemu: PathBuf,
    pub(super) mkfs_fat: PathBuf,
    pub(super) mcopy: PathBuf,
    pub(super) touch: PathBuf,
    pub(super) timeout: PathBuf,
    pub(super) ovmf_code: PathBuf,
    pub(super) ovmf_vars: PathBuf,
}

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let tools = Tools::pinned()?;
    let source = root.join("verus/machine-model/uefi_debug_exit_capsule.rs");
    let consumer_source = root.join("tests/m0/uefi_capsule_consumer.rs");
    let linker_script = root.join("kernel-host/link/m0_uefi.ld");
    for (path, label) in [
        (&source, "UEFI entry Verus source"),
        (&consumer_source, "UEFI entry consumer"),
        (&linker_script, "UEFI PE linker script"),
    ] {
        require_file(path, label)?;
    }
    let canonical_source = fs::read_to_string(&source)
        .map_err(|error| format!("read {}: {error}", source.display()))?;
    for forbidden in [
        "assume(",
        "admit(",
        "axiom fn",
        "external_body",
        "unsafe ",
        "asm!",
    ] {
        if canonical_source.contains(forbidden) {
            return Err(format!(
                "UEFI entry Verus source contains forbidden `{forbidden}`"
            ));
        }
    }
    let source_sha = sha256sum(&source)?;
    let linker_sha = sha256sum(&linker_script)?;
    let auditor_sha = sha256sum(&root.join("xtask/src/uefi.rs"))?;

    let work = root.join("build/m0-uefi");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;
    let staged = work.join("tmk_uefi_capsule.rs");
    fs::copy(&source, &staged).map_err(|error| format!("stage UEFI Verus source: {error}"))?;
    if sha256sum(&staged)? != source_sha {
        return Err("staged UEFI Verus source differs from canonical source".to_string());
    }

    let verification = run_checked(
        &mut direct_verus_command(&tools.verus, &work, "tmk_uefi_capsule.rs", true),
        "Verus exact-byte UEFI entry proof and codegen",
    )?;
    require_output_fragments(
        &verification.stdout,
        "Verus exact-byte UEFI entry proof and codegen",
        &[
            "\"success\": true",
            "\"verified\": 3",
            "\"errors\": 0",
            "\"version\": \"0.2026.05.24.ecee80a\"",
            "\"toolchain\": \"1.95.0-x86_64-unknown-linux-gnu\"",
        ],
    )?;
    fs::write(
        work.join("verus-result.json"),
        canonical_json(&verification.stdout, "UEFI Verus result")?,
    )
    .map_err(|error| format!("write UEFI Verus result: {error}"))?;
    if sha256sum(&staged)? != source_sha {
        return Err("UEFI Verus source changed during proof/codegen".to_string());
    }
    let model = work.join("libtmk_uefi_capsule.rlib");
    require_file(&model, "compiled UEFI entry model")?;
    let model_sha = sha256sum(&model)?;
    for name in ["repro-a", "repro-b"] {
        let repro = work.join(name);
        fs::create_dir(&repro)
            .map_err(|error| format!("create UEFI model reproducibility path: {error}"))?;
        fs::copy(&source, repro.join("tmk_uefi_capsule.rs"))
            .map_err(|error| format!("stage UEFI model reproducibility source: {error}"))?;
        run_checked(
            &mut direct_verus_command(&tools.verus, &repro, "tmk_uefi_capsule.rs", true),
            &format!("Verus UEFI model clean build in {name}"),
        )?;
        let actual = sha256sum(&repro.join("libtmk_uefi_capsule.rlib"))?;
        if actual != model_sha {
            return Err(format!(
                "UEFI model build in {name} produced {actual}, expected {model_sha}"
            ));
        }
        fs::remove_dir_all(&repro)
            .map_err(|error| format!("remove UEFI model reproducibility path: {error}"))?;
    }

    let consumer = work.join("uefi-capsule-consumer");
    run_checked(
        Command::new(&tools.rustc)
            .current_dir(&root)
            .args(["--edition=2021"])
            .arg(&consumer_source)
            .arg("--extern")
            .arg(format!("tmk_uefi_capsule={}", model.display()))
            .arg("-L")
            .arg("dependency=/opt/verus/0.2026.05.24.ecee80a")
            .args(["-C", "panic=abort"])
            .arg("-o")
            .arg(&consumer),
        "link exact-byte UEFI capsule consumer",
    )?;
    let entry_bin = work.join("entry.bin");
    let runtime = run_checked(
        Command::new(&consumer).current_dir(&root).arg(&entry_bin),
        "execute exact-byte UEFI capsule consumer",
    )?;
    require_output_fragments(
        &runtime.stdout,
        "UEFI capsule consumer runtime",
        &["M0_UEFI_CAPSULE_OK:56:00e9:0000000000000000"],
    )?;
    write_combined_output(
        &work.join("capsule-runtime.txt"),
        &runtime,
        "UEFI capsule runtime",
    )?;
    let entry = expected_entry_bytes();
    let emitted = fs::read(&entry_bin).map_err(|error| format!("read entry bytes: {error}"))?;
    if emitted != entry {
        return Err("emitted UEFI entry bytes differ from registered encoding".to_string());
    }

    run_verus_negative_cases(&tools, &work, &canonical_source)?;

    let pe_dir = work.join("pe-primary");
    let pe = build_pe(&tools, &linker_script, &entry_bin, &pe_dir)?;
    let pe_bytes = fs::read(&pe).map_err(|error| format!("read UEFI PE image: {error}"))?;
    let pe_audit = uefi::audit_pe(&pe_bytes, &entry)?;
    if pe_audit.text_size != entry.len() {
        return Err("PE auditor returned the wrong registered text size".to_string());
    }
    let pe_sha = sha256sum(&pe)?;
    for name in ["pe-repro-a", "pe-repro-b"] {
        let reproduced = build_pe(&tools, &linker_script, &entry_bin, &work.join(name))?;
        let bytes =
            fs::read(&reproduced).map_err(|error| format!("read reproduced PE image: {error}"))?;
        uefi::audit_pe(&bytes, &entry)?;
        let actual = sha256sum(&reproduced)?;
        if actual != pe_sha {
            return Err(format!(
                "PE build in {name} produced {actual}, expected {pe_sha}"
            ));
        }
        fs::remove_dir_all(work.join(name))
            .map_err(|error| format!("remove PE reproducibility path: {error}"))?;
    }
    let objdump = run_checked(
        Command::new(&tools.objdump)
            .args(["-f", "-p", "-h"])
            .arg(&pe),
        "independent tool view of UEFI PE image",
    )?;
    require_output_fragments(
        &objdump.stdout,
        "UEFI PE objdump",
        &[
            "file format pei-x86-64",
            "Subsystem",
            "EFI application",
            ".text",
        ],
    )?;
    fs::write(work.join("pe-objdump.txt"), &objdump.stdout)
        .map_err(|error| format!("write PE objdump evidence: {error}"))?;

    let image_dir = work.join("image-primary");
    let disk = build_fat_image(&tools, &pe, &image_dir)?;
    let disk_bytes = fs::read(&disk).map_err(|error| format!("read FAT image: {error}"))?;
    let boot = uefi::extract_bootx64(&disk_bytes)?;
    if boot.bytes != pe_bytes {
        return Err("FAT BOOTX64.EFI differs from the audited PE image".to_string());
    }
    let disk_sha = sha256sum(&disk)?;
    for name in ["image-repro-a", "image-repro-b"] {
        let reproduced = build_fat_image(&tools, &pe, &work.join(name))?;
        let bytes =
            fs::read(&reproduced).map_err(|error| format!("read reproduced FAT image: {error}"))?;
        let extracted = uefi::extract_bootx64(&bytes)?;
        if extracted.bytes != pe_bytes {
            return Err(format!("FAT build in {name} changed BOOTX64.EFI"));
        }
        let actual = sha256sum(&reproduced)?;
        if actual != disk_sha {
            return Err(format!(
                "FAT image build in {name} produced {actual}, expected {disk_sha}"
            ));
        }
        fs::remove_dir_all(work.join(name))
            .map_err(|error| format!("remove FAT reproducibility path: {error}"))?;
    }

    run_qemu(&tools, &work, &disk, "tcg", "tcg", MARKER, true)?;
    run_qemu(&tools, &work, &disk, "kvm", "kvm", MARKER, true)?;
    run_structural_negative_cases(&entry, &pe_bytes, pe_audit, &disk_bytes, &boot, &work)?;

    let malformed = work.join("malformed-pe.img");
    let mut malformed_bytes = disk_bytes.clone();
    malformed_bytes[boot.first_data_offset] ^= 0xff;
    fs::write(&malformed, malformed_bytes)
        .map_err(|error| format!("write malformed firmware-test image: {error}"))?;
    run_qemu(
        &tools,
        &work,
        &malformed,
        "malformed-pe",
        "tcg",
        MARKER,
        false,
    )?;

    let verification_sha = sha256sum(&work.join("verus-result.json"))?;
    let entry_sha = sha256sum(&entry_bin)?;
    let consumer_sha = sha256sum(&consumer)?;
    let report = format!(
        "M0_UEFI_OK\ncomponent_verified=true\nrelease_eligible=false\nsource_sha256={source_sha}\nlinker_script_sha256={linker_sha}\nauditor_sha256={auditor_sha}\nmodel_artifact_sha256={model_sha}\nverus_result_sha256={verification_sha}\nconsumer_sha256={consumer_sha}\nentry_sha256={entry_sha}\npe_sha256={pe_sha}\nimage_sha256={disk_sha}\novmf_code_sha256={}\novmf_vars_sha256={}\nreproducibility_model_builds=3\nreproducibility_pe_builds=3\nreproducibility_image_builds=3\ntcg_marker=TMK_M0_UEFI_OK!\\n\nkvm_marker=TMK_M0_UEFI_OK!\\n\nnegative_cases=bad-semantics,bad-assume,pe-byte,pe-timestamp,pe-subsystem,fat-byte,fat-path,firmware-malformed-pe\n",
        sha256sum(&tools.ovmf_code)?,
        sha256sum(&tools.ovmf_vars)?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write M0 UEFI report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

impl Tools {
    pub(super) fn pinned() -> Result<Self, String> {
        let tools = Self {
            verus: "/opt/verus/0.2026.05.24.ecee80a/verus".into(),
            rustc: "/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc".into(),
            ld: "/usr/sbin/ld".into(),
            objcopy: "/usr/sbin/objcopy".into(),
            objdump: "/usr/sbin/objdump".into(),
            qemu: "/usr/bin/qemu-system-x86_64".into(),
            mkfs_fat: "/usr/sbin/mkfs.fat".into(),
            mcopy: "/usr/sbin/mcopy".into(),
            touch: "/usr/bin/touch".into(),
            timeout: "/usr/bin/timeout".into(),
            ovmf_code: "/usr/share/edk2/ovmf/OVMF_CODE.fd".into(),
            ovmf_vars: "/usr/share/edk2/ovmf/OVMF_VARS.fd".into(),
        };
        for (path, expected, label) in [
            (
                &tools.verus,
                "c5911ee43c7a92c49a48d2c8646c604d252a38c71c87bda88ad4d33eb9e7e0fc",
                "Verus",
            ),
            (
                &tools.rustc,
                "bff349e72704ff70bc08a234a3847338e797065bbedde5e556808bc87b7bf7c6",
                "Forge codegen rustc",
            ),
            (
                &tools.ld,
                "6cf122245638eb45fd981c75bf3a116675b3f9c7510ae2e3b386aa6738e46505",
                "GNU ld",
            ),
            (
                &tools.objcopy,
                "8f09a5b2d8e2b34aebf269fffef2308492a022dcfedf87b49489592e838129b4",
                "GNU objcopy",
            ),
            (
                &tools.objdump,
                "c7c3f8c5c0ed23b2330e148e58624f8d798f1673f4c9fb126ee81096f05e3653",
                "GNU objdump",
            ),
            (
                &tools.qemu,
                "8294f7d61d86167076194e834c3e4c592923f1812709a46edf4bb8f76e55ec7e",
                "QEMU",
            ),
            (
                &tools.mkfs_fat,
                "7075f676c8dd292015f8f72d3574eb024c5ab5e545c3b031b8ef5355a5701093",
                "mkfs.fat",
            ),
            (
                &tools.mcopy,
                "92d837c9b2ad562e5597a1881b7cdd7828e9c0e8ccfbc874fb396eee22fcebf3",
                "mcopy",
            ),
            (
                &tools.touch,
                "22c0c7439c659dff1d88dbe7e096d5f4f6fc12d82673395304815626e240934f",
                "touch",
            ),
            (
                &tools.timeout,
                "350001cc47ad731c4e797532fe46a999477aba359692e2de3e93f316b4021dab",
                "timeout",
            ),
            (
                &tools.ovmf_code,
                "4e87e4be6bb9cdced848ec0b43adab3c7f15623e36055525d0691d137eb74af9",
                "OVMF code",
            ),
            (
                &tools.ovmf_vars,
                "6ed987af3a3c155be71665f510eae3e007eda9b8b94afd59d45e91c4a11565cc",
                "OVMF variables template",
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

fn expected_entry_bytes() -> Vec<u8> {
    let words = [
        0xb0ee_54b0_00e9_ba66u64,
        0xee5f_b0ee_4bb0_ee4du64,
        0x5fb0_ee30_b0ee_4db0u64,
        0xb0ee_45b0_ee55_b0eeu64,
        0xee5f_b0ee_49b0_ee46u64,
        0x21b0_ee4b_b0ee_4fb0u64,
        0xccc3_c031_ee0a_b0eeu64,
    ];
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn run_verus_negative_cases(tools: &Tools, work: &Path, canonical: &str) -> Result<(), String> {
    let bad_semantics = canonical.replacen("                rax: 0,", "                rax: 1,", 1);
    if bad_semantics == canonical {
        return Err("UEFI semantic mutation target was not found".to_string());
    }
    fs::write(work.join("bad-semantics.rs"), bad_semantics)
        .map_err(|error| format!("write UEFI semantic mutation: {error}"))?;
    let output = run_expect_failure(
        &mut direct_verus_command(&tools.verus, work, "bad-semantics.rs", false),
        "Verus rejects UEFI semantic mutation",
    )?;
    let diagnostics = combined(&output);
    require_output_fragments(
        &diagnostics,
        "Verus UEFI semantic rejection",
        &["postcondition not satisfied"],
    )?;
    fs::write(work.join("bad-semantics-result.txt"), diagnostics)
        .map_err(|error| format!("write UEFI semantic rejection: {error}"))?;

    let bad_assume = canonical.replacen(
        "    if code.word0 ==",
        "    assume(false);\n    if code.word0 ==",
        1,
    );
    if bad_assume == canonical {
        return Err("UEFI assume mutation target was not found".to_string());
    }
    fs::write(work.join("bad-assume.rs"), bad_assume)
        .map_err(|error| format!("write UEFI assume mutation: {error}"))?;
    let output = run_expect_failure(
        &mut direct_verus_command(&tools.verus, work, "bad-assume.rs", false),
        "Verus no-cheating rejects UEFI assume",
    )?;
    let diagnostics = combined(&output);
    require_output_fragments(
        &diagnostics,
        "Verus UEFI assume rejection",
        &["assume/admit not allowed with --no-cheating"],
    )?;
    fs::write(work.join("bad-assume-result.txt"), diagnostics)
        .map_err(|error| format!("write UEFI assume rejection: {error}"))?;
    Ok(())
}

pub(super) fn build_pe(
    tools: &Tools,
    linker_script: &Path,
    entry_bin: &Path,
    output: &Path,
) -> Result<PathBuf, String> {
    fs::create_dir(output)
        .map_err(|error| format!("create PE build directory {}: {error}", output.display()))?;
    fs::copy(entry_bin, output.join("entry.bin"))
        .map_err(|error| format!("stage PE entry bytes: {error}"))?;
    run_checked(
        Command::new(&tools.objcopy).current_dir(output).args([
            "-I",
            "binary",
            "-O",
            "pe-x86-64",
            "-B",
            "i386:x86-64",
            "--rename-section",
            ".data=.text,alloc,contents,load,readonly,code",
            "--redefine-sym",
            "_binary_entry_bin_start=efi_main",
            "--redefine-sym",
            "_binary_entry_bin_end=efi_main_end",
            "--redefine-sym",
            "_binary_entry_bin_size=efi_main_size",
            "entry.bin",
            "entry.o",
        ]),
        "wrap verified UEFI entry in PE/COFF object",
    )?;
    run_checked(
        Command::new(&tools.ld)
            .current_dir(output)
            .env("SOURCE_DATE_EPOCH", "0")
            .args([
                "-mi386pep",
                "--subsystem",
                "10",
                "--entry",
                "efi_main",
                "--image-base",
                "0x100000",
                "--disable-dynamicbase",
                "--file-alignment",
                "512",
                "--section-alignment",
                "4096",
                "--stack",
                "0,0",
                "--heap",
                "0,0",
                "--build-id=none",
                "--no-insert-timestamp",
                "--strip-all",
            ])
            .arg("-T")
            .arg(linker_script)
            .args(["-o", "BOOTX64.EFI", "entry.o"]),
        "link deterministic minimal UEFI PE image",
    )?;
    Ok(output.join("BOOTX64.EFI"))
}

pub(super) fn build_fat_image(tools: &Tools, pe: &Path, output: &Path) -> Result<PathBuf, String> {
    fs::create_dir(output).map_err(|error| {
        format!(
            "create FAT image build directory {}: {error}",
            output.display()
        )
    })?;
    let staging = output.join("staging");
    let boot_dir = staging.join("EFI/BOOT");
    fs::create_dir_all(&boot_dir)
        .map_err(|error| format!("create FAT staging directory: {error}"))?;
    fs::copy(pe, boot_dir.join("BOOTX64.EFI"))
        .map_err(|error| format!("stage BOOTX64.EFI: {error}"))?;
    for path in [
        boot_dir.join("BOOTX64.EFI"),
        boot_dir.clone(),
        staging.join("EFI"),
        staging.clone(),
    ] {
        run_checked(
            Command::new(&tools.touch)
                .args(["-t", "198001010000"])
                .arg(path),
            "normalize FAT staging timestamp",
        )?;
    }
    let disk = output.join("thermite-microkernel-m0.img");
    run_checked(
        Command::new(&tools.mkfs_fat)
            .args([
                "--invariant",
                "-C",
                "-F",
                "16",
                "-i",
                "544d4b30",
                "-n",
                "TMK_M0",
            ])
            .arg(&disk)
            .arg("32768"),
        "create deterministic FAT16 boot image",
    )?;
    run_checked(
        Command::new(&tools.mcopy)
            .current_dir(output)
            .args(["-m", "-s", "-i"])
            .arg(&disk)
            .arg("staging/EFI")
            .arg("::"),
        "copy UEFI fallback path into FAT16 image",
    )?;
    Ok(disk)
}

pub(super) fn run_qemu(
    tools: &Tools,
    work: &Path,
    disk: &Path,
    name: &str,
    accelerator: &str,
    marker: &[u8],
    expect_marker: bool,
) -> Result<(), String> {
    let vars = work.join(format!("ovmf-vars-{name}.fd"));
    let log = work.join(format!("qemu-{name}-debugcon.log"));
    fs::copy(&tools.ovmf_vars, &vars)
        .map_err(|error| format!("copy OVMF variable template for {name}: {error}"))?;
    let output = Command::new(&tools.timeout)
        .args(["--signal=TERM", "--kill-after=2s", "8s"])
        .arg(&tools.qemu)
        .args([
            "-machine",
            &format!("q35,accel={accelerator}"),
            "-m",
            "128M",
        ])
        .arg("-drive")
        .arg(format!(
            "if=pflash,format=raw,readonly=on,file={}",
            tools.ovmf_code.display()
        ))
        .arg("-drive")
        .arg(format!("if=pflash,format=raw,file={}", vars.display()))
        .arg("-drive")
        .arg(format!("if=ide,format=raw,file={}", disk.display()))
        .args([
            "-display",
            "none",
            "-serial",
            "none",
            "-monitor",
            "none",
            "-debugcon",
        ])
        .arg(format!("file:{}", log.display()))
        .args(["-global", "isa-debugcon.iobase=0xe9", "-no-reboot"])
        .output()
        .map_err(|error| format!("spawn QEMU {name}: {error}"))?;
    if output.status.code() != Some(124) {
        return Err(format!(
            "QEMU {name} exited with {}, expected timeout status 124\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let observed = fs::read(&log).map_err(|error| format!("read QEMU {name} log: {error}"))?;
    if expect_marker {
        if observed != marker {
            return Err(format!(
                "QEMU {name} debug marker is {:?}, expected {:?}",
                observed, marker
            ));
        }
    } else if observed
        .windows(marker.len())
        .any(|window| window == marker)
    {
        return Err(format!("QEMU {name} executed the malformed PE image"));
    }
    write_combined_output(
        &work.join(format!("qemu-{name}-process.txt")),
        &output,
        &format!("QEMU {name}"),
    )?;
    fs::remove_file(&vars).map_err(|error| format!("remove QEMU {name} vars: {error}"))?;
    Ok(())
}

pub(super) fn run_structural_negative_cases(
    entry: &[u8],
    pe: &[u8],
    pe_audit: uefi::PeAudit,
    disk: &[u8],
    boot: &uefi::BootFile,
    work: &Path,
) -> Result<(), String> {
    let mut results = String::new();
    let mut pe_byte = pe.to_vec();
    pe_byte[pe_audit.text_file_offset] ^= 1;
    record_rejection(
        "pe-byte",
        uefi::audit_pe(&pe_byte, entry),
        "PE .text bytes differ",
        &mut results,
    )?;
    let pe_header = u32::from_le_bytes([pe[0x3c], pe[0x3d], pe[0x3e], pe[0x3f]]) as usize;
    let mut pe_timestamp = pe.to_vec();
    pe_timestamp[pe_header + 8] = 1;
    record_rejection(
        "pe-timestamp",
        uefi::audit_pe(&pe_timestamp, entry),
        "timestamp is not zero",
        &mut results,
    )?;
    let mut pe_subsystem = pe.to_vec();
    pe_subsystem[pe_header + 24 + 68] = 2;
    record_rejection(
        "pe-subsystem",
        uefi::audit_pe(&pe_subsystem, entry),
        "subsystem is not EFI application",
        &mut results,
    )?;
    let mut fat_byte = disk.to_vec();
    fat_byte[boot.first_data_offset + pe_audit.text_file_offset] ^= 1;
    let extracted = uefi::extract_bootx64(&fat_byte)?;
    record_rejection(
        "fat-byte",
        uefi::audit_pe(&extracted.bytes, entry),
        "PE .text bytes differ",
        &mut results,
    )?;
    let mut fat_path = disk.to_vec();
    let name_offset = fat_path
        .windows(11)
        .position(|window| window == b"EFI        ")
        .ok_or_else(|| "FAT path mutation target was not found".to_string())?;
    fat_path[name_offset] = b'X';
    record_rejection(
        "fat-path",
        uefi::extract_bootx64(&fat_path).map(|_| ()),
        "EFI directory occurs 0 times",
        &mut results,
    )?;
    fs::write(work.join("negative-structural-results.txt"), results)
        .map_err(|error| format!("write UEFI structural negative evidence: {error}"))?;
    Ok(())
}

fn record_rejection<T>(
    name: &str,
    result: Result<T, String>,
    expected: &str,
    results: &mut String,
) -> Result<(), String> {
    let diagnostic = result
        .err()
        .ok_or_else(|| format!("{name} mutation unexpectedly passed"))?;
    if !diagnostic.contains(expected) {
        return Err(format!(
            "{name} mutation diagnostic `{diagnostic}` does not contain `{expected}`"
        ));
    }
    results.push_str(&format!("{name}: {diagnostic}\n"));
    Ok(())
}

fn combined(output: &Output) -> Vec<u8> {
    let mut bytes = output.stdout.clone();
    bytes.extend_from_slice(&output.stderr);
    bytes
}
