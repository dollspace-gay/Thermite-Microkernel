use super::{
    canonical_json, check_forge_skill, copy_tree, forge_binary, json_string, report_field,
    require_output_fragments, run_checked, run_expect_failure, sha256sum, workspace_root,
};
use serde_json::Value;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SOURCE: &str = "thermite/core/composition_probe.th";
const SHELL: &str = "tests/m0/composition_shell.rs";
const CRATE_NAME: &str = "tmk_composition_probe";
const EXPORT: &str = "composition_step";
const ARTIFACT: &str = "artifact/libtmk_composition_probe.rlib";
const RECEIPT_SCHEMA: &str = "thermite.verified-composition-receipt.v1";
const RUNTIME_MARKER: &str = "M0_COMPOSITION_OK:store:reject:1";

pub fn run() -> Result<(), String> {
    let root = workspace_root()?;
    let forge = forge_binary()?;
    let pins = validate_pins(&root, &forge)?;

    for (path, label) in [
        (root.join(SOURCE), "Thermite composition source"),
        (root.join(SHELL), "direct-Verus composition shell"),
        (
            root.join("tests/m0/composition_consumer.rs"),
            "hosted composition consumer",
        ),
        (
            root.join("tests/m0/composition_kernel_consumer.rs"),
            "freestanding composition consumer",
        ),
        (
            root.join("tests/m0/composition_private_consumer.rs"),
            "composition privacy consumer",
        ),
    ] {
        require_file(&path, label)?;
    }

    let work = root.join("build/m0-composition");
    if work.exists() {
        fs::remove_dir_all(&work)
            .map_err(|error| format!("remove stale {}: {error}", work.display()))?;
    }
    fs::create_dir_all(&work).map_err(|error| format!("create {}: {error}", work.display()))?;

    run_source_assurance(&root, &forge, &work)?;

    let bundles = [
        work.join("primary.verified"),
        work.join("repro-a.verified"),
        work.join("repro-b.verified"),
    ];
    for (index, bundle) in bundles.iter().enumerate() {
        let output = run_checked(
            &mut composition_build_command(&forge, &root, SHELL, bundle),
            &format!("Forge rich-state composition build {}", index + 1),
        )?;
        write_output(
            &work.join(format!("build-{}.txt", index + 1)),
            &output,
            "composition build evidence",
        )?;
        require_file(&bundle.join("receipt.json"), "composition receipt")?;
        require_file(&bundle.join(ARTIFACT), "composition rlib")?;
    }

    let primary = &bundles[0];
    let receipt = validate_receipt(primary, &root)?;
    let binding_sha =
        json_string(&receipt, "/binding_sha256", "composition receipt binding")?.to_string();
    let artifact_sha = json_string(
        &receipt,
        "/binding/artifact/sha256",
        "composition receipt artifact digest",
    )?
    .to_string();
    let combined_source_sha = json_string(
        &receipt,
        "/binding/composition/combined_source_sha256",
        "composition combined-source digest",
    )?
    .to_string();

    let validation = verify_bundle(&forge, &root, primary, false)?;
    fs::write(
        work.join("verify.json"),
        serde_json::to_vec_pretty(&validation)
            .map_err(|error| format!("serialize composition validation: {error}"))?,
    )
    .map_err(|error| format!("write composition validation evidence: {error}"))?;
    let replay = verify_bundle(&forge, &root, primary, true)?;
    fs::write(
        work.join("replay.json"),
        serde_json::to_vec_pretty(&replay)
            .map_err(|error| format!("serialize composition replay: {error}"))?,
    )
    .map_err(|error| format!("write composition replay evidence: {error}"))?;
    for (report, expected_replayed, label) in [
        (&validation, false, "validation"),
        (&replay, true, "replay"),
    ] {
        if report.get("replayed").and_then(Value::as_bool) != Some(expected_replayed)
            || json_string(report, "/binding_sha256", label)? != binding_sha
            || json_string(report, "/artifact_sha256", label)? != artifact_sha
        {
            return Err(format!(
                "composition {label} report does not match the accepted receipt"
            ));
        }
    }

    let primary_receipt = fs::read(primary.join("receipt.json"))
        .map_err(|error| format!("read primary composition receipt: {error}"))?;
    let primary_artifact = fs::read(primary.join(ARTIFACT))
        .map_err(|error| format!("read primary composition artifact: {error}"))?;
    let primary_source = fs::read(primary.join("evidence/source.verus.rs"))
        .map_err(|error| format!("read primary combined source: {error}"))?;
    for bundle in bundles.iter().skip(1) {
        for (relative, expected, label) in [
            ("receipt.json", primary_receipt.as_slice(), "receipt"),
            (ARTIFACT, primary_artifact.as_slice(), "artifact"),
            (
                "evidence/source.verus.rs",
                primary_source.as_slice(),
                "combined source",
            ),
        ] {
            let actual = fs::read(bundle.join(relative)).map_err(|error| {
                format!("read reproduced composition {label} `{relative}`: {error}")
            })?;
            if actual != expected {
                return Err(format!(
                    "composition {label} differs across independent builds"
                ));
            }
        }
        verify_bundle(&forge, &root, bundle, false)?;
    }

    audit_combined_evidence(primary, &root, &receipt, &primary_artifact)?;
    let toolchain = read_json(
        &primary.join("evidence/toolchain.json"),
        "toolchain evidence",
    )?;
    let codegen_rustc = receipt_codegen_rustc(&toolchain)?;
    let hosted_sha = run_hosted_consumer(&root, &work, primary, &codegen_rustc)?;
    run_private_consumer_negative(&root, &work, primary, &codegen_rustc)?;
    run_incompatible_rustc_negative(&root, &work, primary, &toolchain)?;

    let (low_sha, high_sha) = run_freestanding_links(&root, &work, primary, &codegen_rustc)?;
    run_cross_absolute_path_reproduction(
        &forge,
        &root,
        &work,
        primary,
        &codegen_rustc,
        &low_sha,
        &high_sha,
    )?;
    run_bundle_tamper_negatives(&forge, &root, &work, primary)?;
    run_build_negatives(&forge, &root, &work)?;

    let receipt_sha = sha256sum(&primary.join("receipt.json"))?;
    let final_link_receipt_sha = write_final_link_receipt(
        &root, &work, primary, &receipt, &toolchain, &low_sha, &high_sha,
    )?;
    let platform_object_sha =
        sha256sum(&root.join("build/m0-platform-primitives/objects/platform-primitives.o"))?;
    let report = format!(
        "M0_COMPOSITION_OK\ncomponent_verified=true\nrelease_eligible=false\nreceipt_validated=true\nreceipt_replayed=true\nfinal_link_receipted=true\nlinked_primitives_verified=true\nselected_primitives=memcpy\npositive_gates=13\nforge_revision={}\nforge_sha256={}\nskill_sha256={}\nsource_sha256={}\nshell_sha256={}\ncombined_source_sha256={combined_source_sha}\nreceipt_sha256={receipt_sha}\nbinding_sha256={binding_sha}\nartifact_sha256={artifact_sha}\nplatform_primitive_object_sha256={platform_object_sha}\nfinal_link_receipt_sha256={final_link_receipt_sha}\nhosted_consumer_sha256={hosted_sha}\nlow_static_consumer_sha256={low_sha}\nhigh_half_consumer_sha256={high_sha}\ncomposition_reproducibility_builds=3\nlow_static_reproducibility_links=3\nhigh_half_reproducibility_links=3\nabsolute_path_reproducibility_roots=2\nhosted_runtime_marker={RUNTIME_MARKER}\nfreestanding_runtime=fail-stop-timeout-124\nhigh_half_link_base=ffffffff80000000\nnegative_cases=artifact-tamper,binding-tamper,certificate-l2,external-body,extra-file,host-rustc,post-plan-shell,private-export,rich-standalone-export,shell-tamper,tv-nonpass\n",
        pins.revision,
        pins.forge_sha,
        pins.skill_sha,
        sha256sum(&root.join(SOURCE))?,
        sha256sum(&root.join(SHELL))?,
    );
    fs::write(work.join("report.txt"), &report)
        .map_err(|error| format!("write composition report: {error}"))?;
    print!("{report}");
    println!("evidence={}", work.display());
    Ok(())
}

fn write_final_link_receipt(
    root: &Path,
    work: &Path,
    bundle: &Path,
    composition_receipt: &Value,
    toolchain: &Value,
    low_sha: &str,
    high_sha: &str,
) -> Result<String, String> {
    let lock_path = root.join("toolchain/lock.toml");
    let lock = fs::read_to_string(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let tools = [
        pinned_tool(&lock, "c", "cc", "cc_path", "cc_sha256")?,
        pinned_tool(&lock, "binutils", "ld", "ld_path", "ld_sha256")?,
        pinned_tool(&lock, "binutils", "nm", "nm_path", "nm_sha256")?,
        pinned_tool(
            &lock,
            "binutils",
            "objcopy",
            "objcopy_path",
            "objcopy_sha256",
        )?,
        pinned_tool(
            &lock,
            "binutils",
            "readelf",
            "readelf_path",
            "readelf_sha256",
        )?,
        serde_json::json!({
            "name": "rustc-codegen",
            "path": json_string(toolchain, "/artifact_codegen/rustc_path", "link rustc path")?,
            "sha256": json_string(toolchain, "/artifact_codegen/rustc_sha256", "link rustc digest")?,
        }),
        pinned_tool(
            &lock,
            "process",
            "timeout",
            "timeout_path",
            "timeout_sha256",
        )?,
    ];
    let dependency_files: Vec<Value> = composition_receipt
        .pointer("/binding/files")
        .and_then(Value::as_array)
        .ok_or_else(|| "composition receipt has no final-link file inventory".to_string())?
        .iter()
        .filter(|row| {
            row.get("path")
                .and_then(Value::as_str)
                .is_some_and(|path| path.starts_with("artifact/deps/"))
        })
        .cloned()
        .collect();
    if dependency_files.is_empty() {
        return Err("composition receipt has no final-link dependencies".to_string());
    }
    let composition_artifact = bundle.join(ARTIFACT);
    let platform_object = root.join("build/m0-platform-primitives/objects/platform-primitives.o");
    let platform_report = root.join("build/m0-platform-primitives/report.txt");
    let consumer = root.join("tests/m0/composition_kernel_consumer.rs");
    let linker = root.join("tests/m0/global_allocator_kernel.ld");
    let low = work.join("composition-kernel-low");
    let high = work.join("composition-kernel-high-half");
    let linked_memcpy = work.join("linked-primitives/memcpy.bin");
    for (path, label) in [
        (&composition_artifact, "composition final-link artifact"),
        (&platform_object, "platform final-link object"),
        (&platform_report, "platform final-link report"),
        (&consumer, "final-link consumer source"),
        (&linker, "final-link linker script"),
        (&low, "low final-link image"),
        (&high, "higher-half final-link image"),
        (&linked_memcpy, "linked memcpy selection"),
    ] {
        require_file(path, label)?;
    }
    if sha256sum(&low)? != low_sha || sha256sum(&high)? != high_sha {
        return Err("final-link outputs changed before receipt generation".to_string());
    }
    let value = serde_json::json!({
        "schema": "tmk.final-link-receipt.v1",
        "composition": {
            "binding_sha256": json_string(composition_receipt, "/binding_sha256", "final-link composition binding")?,
            "receipt_sha256": sha256sum(&bundle.join("receipt.json"))?,
            "artifact": file_record(root, &composition_artifact)?,
            "dependencies": dependency_files,
            "replay_passed": true,
        },
        "platform": {
            "object": file_record(root, &platform_object)?,
            "acceptance_report": file_record(root, &platform_report)?,
            "linked_primitive": file_record(root, &linked_memcpy)?,
        },
        "link_plan": {
            "orchestrator_source_sha256": sha256sum(&root.join("xtask/src/composition.rs"))?,
            "consumer": file_record(root, &consumer)?,
            "linker_script": file_record(root, &linker)?,
            "selected_symbols": [
                "memcpy",
                "tmk_composition_probe::composition_shell::boot_observation",
                "tmk_composition_probe::composition_step"
            ],
            "discarded_platform_symbols": ["memset", "tmk_alloc_capsule", "tmk_seal_capsule"],
            "undefined_symbols": 0,
            "tools": tools,
        },
        "outputs": {
            "low_static": file_record(root, &low)?,
            "higher_half": file_record(root, &high)?,
            "higher_half_entry": "ffffffff80000000",
            "low_static_reproducibility_links": 3,
            "higher_half_reproducibility_links": 3,
            "absolute_path_reproducibility_roots": 2,
            "freestanding_runtime": "fail-stop-timeout-124",
        },
    });
    let raw = serde_json::to_vec(&value)
        .map_err(|error| format!("serialize final-link receipt input: {error}"))?;
    let canonical = canonical_json(&raw, "final-link receipt")?;
    let path = work.join("final-link-receipt.json");
    fs::write(&path, canonical).map_err(|error| format!("write final-link receipt: {error}"))?;
    sha256sum(&path)
}

fn pinned_tool(
    lock: &str,
    section: &str,
    name: &str,
    path_key: &str,
    digest_key: &str,
) -> Result<Value, String> {
    let path = PathBuf::from(lock_value(lock, section, path_key)?);
    let expected = lock_value(lock, section, digest_key)?;
    require_file(&path, &format!("pinned final-link tool `{name}`"))?;
    let actual = sha256sum(&path)?;
    if actual != expected {
        return Err(format!(
            "final-link tool `{name}` digest is {actual}, lock requires {expected}"
        ));
    }
    Ok(serde_json::json!({
        "name": name,
        "path": path,
        "sha256": expected,
    }))
}

fn file_record(root: &Path, path: &Path) -> Result<Value, String> {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let length = fs::metadata(path)
        .map_err(|error| format!("stat final-link file {}: {error}", path.display()))?
        .len();
    Ok(serde_json::json!({
        "path": relative,
        "length": length,
        "sha256": sha256sum(path)?,
    }))
}

struct Pins {
    revision: String,
    forge_sha: String,
    skill_sha: String,
}

fn validate_pins(root: &Path, forge: &Path) -> Result<Pins, String> {
    let lock_path = root.join("toolchain/lock.toml");
    let lock = fs::read_to_string(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let revision = lock_value(&lock, "thermite", "commit")?;
    let forge_sha = lock_value(&lock, "thermite", "forge_sha256")?;
    let skill_sha = lock_value(&lock, "thermite", "skill_sha256")?;
    let locked_forge = PathBuf::from(lock_value(&lock, "thermite", "forge_path")?);
    if forge != locked_forge {
        return Err(format!(
            "composition gate Forge path is {}, lock requires {}",
            forge.display(),
            locked_forge.display()
        ));
    }
    let actual_forge_sha = sha256sum(forge)?;
    if actual_forge_sha != forge_sha {
        return Err(format!(
            "composition gate Forge digest is {actual_forge_sha}, lock requires {forge_sha}"
        ));
    }

    let skill = env::var_os("THERMITE_SKILL_REFERENCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from("/home/doll/.codex/skills/thermite/references/language.md")
        });
    require_file(&skill, "Thermite skill reference")?;
    let actual_skill_sha = sha256sum(&skill)?;
    if actual_skill_sha != skill_sha {
        return Err(format!(
            "Thermite skill digest is {actual_skill_sha}, lock requires {skill_sha}"
        ));
    }
    check_forge_skill(forge)?;

    let thermite_root = env::var_os("THERMITE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/doll/Thermite"));
    let head = run_checked(
        Command::new("git")
            .arg("-C")
            .arg(&thermite_root)
            .args(["rev-parse", "HEAD"]),
        "composition Thermite revision",
    )?;
    let actual_revision = String::from_utf8_lossy(&head.stdout).trim().to_string();
    if actual_revision != revision {
        return Err(format!(
            "composition Thermite revision is {actual_revision}, lock requires {revision}"
        ));
    }
    let status = run_checked(
        Command::new("git").arg("-C").arg(&thermite_root).args([
            "status",
            "--porcelain",
            "--untracked-files=all",
        ]),
        "composition Thermite source cleanliness",
    )?;
    if !status.stdout.is_empty() {
        return Err("composition Thermite source tree is dirty".to_string());
    }
    Ok(Pins {
        revision,
        forge_sha,
        skill_sha,
    })
}

fn lock_value(lock: &str, section: &str, key: &str) -> Result<String, String> {
    let wanted_section = format!("[{section}]");
    let mut active = false;
    for raw in lock.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            active = line == wanted_section;
            continue;
        }
        if !active {
            continue;
        }
        let Some((candidate, value)) = line.split_once('=') else {
            continue;
        };
        if candidate.trim() == key {
            let value = value.trim();
            return value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .map(str::to_string)
                .ok_or_else(|| format!("toolchain lock `{section}.{key}` is not quoted"));
        }
    }
    Err(format!("toolchain lock is missing `{section}.{key}`"))
}

fn run_source_assurance(root: &Path, forge: &Path, work: &Path) -> Result<(), String> {
    let check = run_checked(
        Command::new(forge)
            .current_dir(root)
            .args(["check", SOURCE, "--level", "l3", "--json"]),
        "Forge composition L3 source check",
    )?;
    require_output_fragments(
        &check.stdout,
        "Forge composition L3 source check",
        &[
            "\"item\": \"composition_step\"",
            "\"level\": \"L3\"",
            "\"mutants_killed\": \"11/11\"",
            "\"kind\": \"end_to_end\"",
        ],
    )?;
    fs::write(work.join("check.json"), &check.stdout)
        .map_err(|error| format!("write composition check evidence: {error}"))?;

    let audit = run_checked(
        Command::new(forge).current_dir(root).args([
            "audit",
            SOURCE,
            "--json",
            "--meaning",
            "--metrics",
        ]),
        "Forge composition assurance audit",
    )?;
    require_output_fragments(
        &audit.stdout,
        "Forge composition assurance audit",
        &["\"project_assurance\"", "\"level\": \"L3\""],
    )?;
    fs::write(work.join("audit.json"), &audit.stdout)
        .map_err(|error| format!("write composition audit evidence: {error}"))?;

    let battery = run_checked(
        Command::new(forge)
            .current_dir(root)
            .args(["battery", SOURCE, EXPORT]),
        "Forge composition mutation battery",
    )?;
    require_output_fragments(
        &battery.stdout,
        "Forge composition mutation battery",
        &["battery — composition_step", "mutants killed: 11/11"],
    )?;
    fs::write(work.join("battery.txt"), &battery.stdout)
        .map_err(|error| format!("write composition battery evidence: {error}"))?;
    Ok(())
}

fn composition_build_command(forge: &Path, root: &Path, shell: &str, out: &Path) -> Command {
    let mut command = Command::new(forge);
    command
        .current_dir(root)
        .args([
            "build",
            SOURCE,
            "--level",
            "l3",
            "--compose-export",
            EXPORT,
            "--compose-shell",
            shell,
            "--crate-name",
            CRATE_NAME,
            "--target",
            "kernel",
            "--out",
        ])
        .arg(out);
    command
}

fn validate_receipt(bundle: &Path, root: &Path) -> Result<Value, String> {
    let receipt = read_json(&bundle.join("receipt.json"), "composition receipt")?;
    for (pointer, expected, label) in [
        ("/schema", RECEIPT_SCHEMA, "receipt schema"),
        ("/binding/schema", RECEIPT_SCHEMA, "binding schema"),
        ("/binding/assurance", "L3", "binding assurance"),
        ("/binding/scope", "end_to_end", "binding scope"),
        ("/binding/crate_name", CRATE_NAME, "binding crate name"),
        ("/binding/target", "kernel", "binding target"),
        ("/binding/artifact/path", ARTIFACT, "artifact path"),
        ("/binding/artifact/kind", "rlib", "artifact kind"),
    ] {
        if json_string(&receipt, pointer, label)? != expected {
            return Err(format!("composition {label} is not `{expected}`"));
        }
    }
    let source_sha = sha256sum(&root.join(SOURCE))?;
    if json_string(&receipt, "/binding/raw_source_sha256", "bound source")? != source_sha {
        return Err("composition receipt does not bind the canonical Thermite source".to_string());
    }
    require_receipt_file(&receipt, bundle, "evidence/input.th", &root.join(SOURCE))?;
    require_receipt_file(
        &receipt,
        bundle,
        "evidence/direct-verus/00-composition_shell.rs",
        &root.join(SHELL),
    )?;
    require_receipt_file(&receipt, bundle, ARTIFACT, &bundle.join(ARTIFACT))?;

    let artifact_length = fs::metadata(bundle.join(ARTIFACT))
        .map_err(|error| format!("stat composition artifact: {error}"))?
        .len();
    if receipt
        .pointer("/binding/artifact/length")
        .and_then(Value::as_u64)
        != Some(artifact_length)
        || json_string(&receipt, "/binding/artifact/sha256", "artifact digest")?
            != sha256sum(&bundle.join(ARTIFACT))?
    {
        return Err("composition receipt artifact record does not match disk".to_string());
    }
    let members = receipt
        .pointer("/binding/assurance_aggregate/members")
        .and_then(Value::as_array)
        .ok_or_else(|| "composition receipt has no assurance members".to_string())?;
    if !members.iter().any(|member| {
        member.get("name").and_then(Value::as_str) == Some(EXPORT)
            && member.get("kind").and_then(Value::as_str) == Some("executable")
            && member.get("achieved").and_then(Value::as_str) == Some("L3")
    }) {
        return Err(
            "composition transition is not recorded as an executable L3 member".to_string(),
        );
    }
    let gates = receipt
        .pointer("/binding/strict_gates")
        .and_then(Value::as_array)
        .ok_or_else(|| "composition receipt has no strict-gate list".to_string())?;
    for required in [
        "no-escape-hatches",
        "contract-tv-complete",
        "exec-tv-complete",
        "body-loop-tv-complete",
        "rich-composition-visibility",
        "direct-verus-source-policy",
        "combined-source-inventory",
        "whole-crate-no-cheating",
        "verus-codegen",
        "cryptographic-binding",
    ] {
        if !gates.iter().any(|gate| gate.as_str() == Some(required)) {
            return Err(format!(
                "composition receipt is missing strict gate `{required}`"
            ));
        }
    }
    Ok(receipt)
}

fn require_receipt_file(
    receipt: &Value,
    bundle: &Path,
    relative: &str,
    canonical: &Path,
) -> Result<(), String> {
    let files = receipt
        .pointer("/binding/files")
        .and_then(Value::as_array)
        .ok_or_else(|| "composition receipt has no file inventory".to_string())?;
    let row = files
        .iter()
        .find(|row| row.get("path").and_then(Value::as_str) == Some(relative))
        .ok_or_else(|| format!("composition receipt does not inventory `{relative}`"))?;
    let staged = bundle.join(relative);
    require_file(&staged, &format!("receipt file `{relative}`"))?;
    let staged_bytes =
        fs::read(&staged).map_err(|error| format!("read receipt file `{relative}`: {error}"))?;
    let canonical_bytes = fs::read(canonical)
        .map_err(|error| format!("read canonical file {}: {error}", canonical.display()))?;
    if staged_bytes != canonical_bytes
        || row.get("length").and_then(Value::as_u64) != Some(staged_bytes.len() as u64)
        || row.get("sha256").and_then(Value::as_str) != Some(sha256sum(&staged)?.as_str())
    {
        return Err(format!(
            "composition receipt file `{relative}` does not match its canonical input"
        ));
    }
    Ok(())
}

fn verify_bundle(forge: &Path, root: &Path, bundle: &Path, replay: bool) -> Result<Value, String> {
    let mut command = Command::new(forge);
    command
        .current_dir(root)
        .args([OsStr::new("verify-build"), bundle.as_os_str()]);
    if replay {
        command.arg("--replay");
    }
    command.arg("--json");
    let output = run_checked(
        &mut command,
        if replay {
            "replay composition receipt and artifact"
        } else {
            "validate composition receipt and artifact"
        },
    )?;
    serde_json::from_slice(&output.stdout).map_err(|error| {
        format!(
            "parse composition {} report: {error}",
            if replay { "replay" } else { "validation" }
        )
    })
}

fn audit_combined_evidence(
    bundle: &Path,
    root: &Path,
    receipt: &Value,
    artifact: &[u8],
) -> Result<(), String> {
    let source_path = bundle.join("evidence/source.verus.rs");
    let source = fs::read_to_string(&source_path)
        .map_err(|error| format!("read combined Verus source: {error}"))?;
    if source.matches("verus!").count() != 1
        || !source.contains("macro_rules! __thermite_deterministic_enum")
        || !source.contains("#[verus::internal(verus_macro)]")
        || !source.contains("pub(crate) fn composition_step")
        || source.contains("pub fn composition_step")
        || !source.contains("pub mod composition_shell")
        || !source.contains("pub fn boot_observation")
    {
        return Err("combined Verus source violates the selected composition shape".to_string());
    }
    for forbidden in ["external_body", "assume(", "admit(", "decreases *"] {
        if source.contains(forbidden) {
            return Err(format!(
                "combined Verus source contains forbidden `{forbidden}`"
            ));
        }
    }
    let source_sha = sha256sum(&source_path)?;
    if json_string(
        receipt,
        "/binding/composition/combined_source_sha256",
        "combined source digest",
    )? != source_sha
        || json_string(
            receipt,
            "/binding/verus_source_sha256",
            "Verus source digest",
        )? != source_sha
    {
        return Err("composition receipt does not bind the audited combined source".to_string());
    }
    for randomized_helper in [
        "arrow_owner",
        "arrow_generation",
        "arrow_slot",
        "arrow_value",
    ] {
        if artifact
            .windows(randomized_helper.len())
            .any(|window| window == randomized_helper.as_bytes())
        {
            return Err(format!(
                "composition artifact contains nondeterministic Verus helper `{randomized_helper}`"
            ));
        }
    }

    let tv = read_json(
        &bundle.join("evidence/translation-validation.json"),
        "composition translation validation",
    )?;
    let rows = tv
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| "composition translation validation has no rows".to_string())?;
    if rows.is_empty()
        || rows
            .iter()
            .any(|row| row.get("verdict").and_then(Value::as_str) != Some("faithful"))
    {
        return Err("composition translation validation contains a non-faithful row".to_string());
    }

    let plan = read_json(
        &bundle.join("evidence/artifact-plan.v1"),
        "composition artifact plan",
    )?;
    if json_string(&plan, "/schema", "composition plan schema")?
        != "thermite.combined-artifact-plan.v1"
        || json_string(
            &plan,
            "/composition/composition_exports/0/thermite_name",
            "composition plan export",
        )? != EXPORT
        || json_string(
            &plan,
            "/composition/composition_exports/0/visibility",
            "composition plan visibility",
        )? != "crate"
        || json_string(
            &plan,
            "/composition/composition_exports/0/return_type",
            "composition plan return type",
        )? != "(ProbeState,ProbeAction)"
    {
        return Err(
            "composition artifact plan does not select the expected private rich export"
                .to_string(),
        );
    }
    let closure = plan
        .pointer("/composition/composition_exports/0/type_closure")
        .and_then(Value::as_array)
        .ok_or_else(|| "composition plan has no rich type closure".to_string())?;
    for required in ["ProbeState", "ProbeEvent", "ProbeAction"] {
        if !closure
            .iter()
            .any(|item| item.as_str().is_some_and(|item| item.contains(required)))
        {
            return Err(format!("composition plan type closure omits `{required}`"));
        }
    }
    if fs::read(bundle.join("evidence/direct-verus/00-composition_shell.rs"))
        .map_err(|error| format!("read staged composition shell: {error}"))?
        != fs::read(root.join(SHELL))
            .map_err(|error| format!("read canonical composition shell: {error}"))?
    {
        return Err("composition shell changed between canonical source and bundle".to_string());
    }
    Ok(())
}

fn receipt_codegen_rustc(toolchain: &Value) -> Result<PathBuf, String> {
    let path = PathBuf::from(json_string(
        toolchain,
        "/artifact_codegen/rustc_path",
        "composition codegen rustc path",
    )?);
    require_file(&path, "composition codegen rustc")?;
    let expected = json_string(
        toolchain,
        "/artifact_codegen/rustc_sha256",
        "composition codegen rustc digest",
    )?;
    let actual = sha256sum(&path)?;
    if actual != expected {
        return Err(format!(
            "composition codegen rustc digest is {actual}, receipt records {expected}"
        ));
    }
    Ok(path)
}

fn rustc_consumer_command(
    rustc: &Path,
    root: &Path,
    source: &Path,
    bundle: &Path,
    output: &Path,
) -> Command {
    let mut command = Command::new(rustc);
    command
        .current_dir(root)
        .args(["--edition=2021"])
        .arg(source)
        .arg("--extern")
        .arg(format!("{CRATE_NAME}={}", bundle.join(ARTIFACT).display()))
        .arg("-L")
        .arg(format!(
            "dependency={}",
            bundle.join("artifact/deps").display()
        ))
        .args(["-C", "panic=abort"])
        .arg(format!("--remap-path-prefix={}=.", root.display()))
        .arg("-o")
        .arg(output);
    command
}

fn run_hosted_consumer(
    root: &Path,
    work: &Path,
    bundle: &Path,
    rustc: &Path,
) -> Result<String, String> {
    let output = work.join("composition-consumer");
    run_checked(
        &mut rustc_consumer_command(
            rustc,
            root,
            &root.join("tests/m0/composition_consumer.rs"),
            bundle,
            &output,
        ),
        "link hosted composition consumer",
    )?;
    let runtime = run_checked(
        Command::new(&output).current_dir(root),
        "execute hosted rich-state composition consumer",
    )?;
    require_output_fragments(
        &runtime.stdout,
        "hosted rich-state composition consumer",
        &[RUNTIME_MARKER],
    )?;
    write_output(
        &work.join("hosted-runtime.txt"),
        &runtime,
        "hosted composition runtime",
    )?;
    sha256sum(&output)
}

fn run_private_consumer_negative(
    root: &Path,
    work: &Path,
    bundle: &Path,
    rustc: &Path,
) -> Result<(), String> {
    let output = run_expect_failure(
        &mut rustc_consumer_command(
            rustc,
            root,
            &root.join("tests/m0/composition_private_consumer.rs"),
            bundle,
            &work.join("must-not-link-private"),
        ),
        "reject external access to private composition transition",
    )?;
    require_output_fragments(
        &output.stderr,
        "composition private-export rejection",
        &["private"],
    )?;
    write_output(
        &work.join("negative/private-export.txt"),
        &output,
        "private composition export rejection",
    )
}

fn run_incompatible_rustc_negative(
    root: &Path,
    work: &Path,
    bundle: &Path,
    toolchain: &Value,
) -> Result<(), String> {
    let host = PathBuf::from(json_string(
        toolchain,
        "/host_rustc/rustc_path",
        "receipt host rustc path",
    )?);
    require_file(&host, "receipt host rustc")?;
    let expected = json_string(
        toolchain,
        "/host_rustc/rustc_sha256",
        "receipt host rustc digest",
    )?;
    if sha256sum(&host)? != expected {
        return Err("receipt host rustc digest does not match disk".to_string());
    }
    let output = run_expect_failure(
        &mut rustc_consumer_command(
            &host,
            root,
            &root.join("tests/m0/composition_consumer.rs"),
            bundle,
            &work.join("must-not-link-host-rustc"),
        ),
        "reject incompatible host rustc for composition artifact",
    )?;
    require_output_fragments(
        &output.stderr,
        "incompatible composition rustc rejection",
        &[
            "compiled by an incompatible version of rustc",
            "compiled by rustc 1.95.0",
        ],
    )?;
    write_output(
        &work.join("negative/host-rustc.txt"),
        &output,
        "incompatible composition rustc rejection",
    )
}

fn run_freestanding_links(
    root: &Path,
    work: &Path,
    bundle: &Path,
    rustc: &Path,
) -> Result<(String, String), String> {
    let primitives = root.join("build/m0-platform-primitives/objects/platform-primitives.o");
    let platform_report = root.join("build/m0-platform-primitives/report.txt");
    let linker = root.join("tests/m0/global_allocator_kernel.ld");
    for (path, label) in [
        (&primitives, "verified platform primitive object"),
        (&platform_report, "platform primitive acceptance report"),
        (&linker, "higher-half linker script"),
    ] {
        require_file(path, label)?;
    }
    let platform = fs::read_to_string(&platform_report)
        .map_err(|error| format!("read platform primitive report: {error}"))?;
    if report_field(&platform, "component_verified")? != "true"
        || report_field(&platform, "linked_primitives_verified")? != "true"
        || report_field(&platform, "verus_verified")? != "39"
        || report_field(&platform, "primitive_object_sha256")? != sha256sum(&primitives)?
    {
        return Err(
            "composition link input is not the accepted platform primitive object".to_string(),
        );
    }

    let low = work.join("composition-kernel-low");
    compile_freestanding(root, rustc, bundle, &primitives, None, &low)?;
    audit_no_undefined(&low, "low static composition image")?;
    let execution = run_expect_failure(
        Command::new("/usr/bin/timeout")
            .current_dir(root)
            .args(["0.1s"])
            .arg(&low),
        "execute low static composition image",
    )?;
    if execution.status.code() != Some(124) {
        return Err(format!(
            "low static composition image exited with {}, expected timeout 124",
            execution.status
        ));
    }
    let low_sha = sha256sum(&low)?;
    for name in [
        "composition-kernel-low-repro-a",
        "composition-kernel-low-repro-b",
    ] {
        let reproduced = work.join(name);
        compile_freestanding(root, rustc, bundle, &primitives, None, &reproduced)?;
        if sha256sum(&reproduced)? != low_sha {
            return Err("low static composition link is not reproducible".to_string());
        }
    }

    let high = work.join("composition-kernel-high-half");
    compile_freestanding(root, rustc, bundle, &primitives, Some(&linker), &high)?;
    audit_no_undefined(&high, "higher-half composition image")?;
    let headers = run_checked(
        Command::new("/usr/sbin/readelf").args(["-hW"]).arg(&high),
        "higher-half composition ELF header audit",
    )?;
    require_output_fragments(
        &headers.stdout,
        "higher-half composition ELF header audit",
        &["Entry point address:               0xffffffff80000000"],
    )?;
    let symbols = run_checked(
        Command::new("/usr/sbin/nm").args(["-C"]).arg(&high),
        "higher-half composition symbol audit",
    )?;
    require_output_fragments(
        &symbols.stdout,
        "higher-half composition symbol audit",
        &[
            "tmk_composition_probe::composition_step",
            "tmk_composition_probe::composition_shell::boot_observation",
            " memcpy",
        ],
    )?;
    reject_unselected_primitive_symbols(&symbols.stdout)?;
    super::platform_primitives::audit_linked_composition_primitives(
        &high,
        &work.join("linked-primitives"),
    )?;
    let high_sha = sha256sum(&high)?;
    for name in [
        "composition-kernel-high-repro-a",
        "composition-kernel-high-repro-b",
    ] {
        let reproduced = work.join(name);
        compile_freestanding(root, rustc, bundle, &primitives, Some(&linker), &reproduced)?;
        if sha256sum(&reproduced)? != high_sha {
            return Err("higher-half composition link is not reproducible".to_string());
        }
    }
    Ok((low_sha, high_sha))
}

fn run_cross_absolute_path_reproduction(
    forge: &Path,
    root: &Path,
    work: &Path,
    primary: &Path,
    rustc: &Path,
    expected_low_sha: &str,
    expected_high_sha: &str,
) -> Result<(), String> {
    let secondary_root = env::temp_dir().join(format!(
        "tmk-m0-composition-absolute-{}",
        std::process::id()
    ));
    if secondary_root.exists() {
        fs::remove_dir_all(&secondary_root).map_err(|error| {
            format!(
                "remove stale absolute-path reproduction root {}: {error}",
                secondary_root.display()
            )
        })?;
    }

    let result = (|| {
        fs::create_dir_all(&secondary_root).map_err(|error| {
            format!(
                "create absolute-path reproduction root {}: {error}",
                secondary_root.display()
            )
        })?;
        let primary_absolute = fs::canonicalize(root)
            .map_err(|error| format!("canonicalize primary composition root: {error}"))?;
        let secondary_absolute = fs::canonicalize(&secondary_root)
            .map_err(|error| format!("canonicalize secondary composition root: {error}"))?;
        if primary_absolute == secondary_absolute {
            return Err("absolute-path reproduction roots unexpectedly coincide".to_string());
        }
        fs::write(
            work.join("absolute-roots.txt"),
            format!(
                "primary={}\nsecondary={}\n",
                primary_absolute.display(),
                secondary_absolute.display()
            ),
        )
        .map_err(|error| format!("write absolute-path root evidence: {error}"))?;

        for relative in [
            SOURCE,
            SHELL,
            "tests/m0/composition_kernel_consumer.rs",
            "tests/m0/global_allocator_kernel.ld",
            "build/m0-platform-primitives/objects/platform-primitives.o",
        ] {
            let source = root.join(relative);
            let destination = secondary_root.join(relative);
            let parent = destination.parent().ok_or_else(|| {
                format!("absolute-path reproduction input `{relative}` has no parent")
            })?;
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "create absolute-path reproduction input directory {}: {error}",
                    parent.display()
                )
            })?;
            fs::copy(&source, &destination).map_err(|error| {
                format!(
                    "copy absolute-path reproduction input {} to {}: {error}",
                    source.display(),
                    destination.display()
                )
            })?;
        }

        let bundle = secondary_root.join("build/absolute.verified");
        let build = run_checked(
            &mut composition_build_command(forge, &secondary_root, SHELL, &bundle),
            "Forge composition build from a second absolute root",
        )?;
        write_output(
            &work.join("absolute-build.txt"),
            &build,
            "absolute-path composition build evidence",
        )?;
        let receipt = validate_receipt(&bundle, &secondary_root)?;
        let artifact = fs::read(bundle.join(ARTIFACT))
            .map_err(|error| format!("read absolute-path composition artifact: {error}"))?;
        audit_combined_evidence(&bundle, &secondary_root, &receipt, &artifact)?;

        let binding_sha =
            json_string(&receipt, "/binding_sha256", "absolute-path binding")?.to_string();
        let artifact_sha = json_string(
            &receipt,
            "/binding/artifact/sha256",
            "absolute-path artifact digest",
        )?
        .to_string();
        for (replay, file_name) in [
            (false, "absolute-verify.json"),
            (true, "absolute-replay.json"),
        ] {
            let report = verify_bundle(forge, &secondary_root, &bundle, replay)?;
            if report.get("replayed").and_then(Value::as_bool) != Some(replay)
                || json_string(&report, "/binding_sha256", "absolute-path replay")? != binding_sha
                || json_string(&report, "/artifact_sha256", "absolute-path replay")? != artifact_sha
            {
                return Err(
                    "absolute-path composition verification does not match its receipt".to_string(),
                );
            }
            fs::write(
                work.join(file_name),
                serde_json::to_vec_pretty(&report).map_err(|error| {
                    format!("serialize absolute-path composition verification: {error}")
                })?,
            )
            .map_err(|error| format!("write absolute-path composition verification: {error}"))?;
        }

        for (relative, label) in [
            ("receipt.json", "receipt"),
            (ARTIFACT, "verified rlib"),
            ("evidence/source.verus.rs", "combined Verus source"),
        ] {
            let expected = fs::read(primary.join(relative)).map_err(|error| {
                format!("read primary composition {label} for absolute comparison: {error}")
            })?;
            let actual = fs::read(bundle.join(relative)).map_err(|error| {
                format!("read secondary composition {label} for absolute comparison: {error}")
            })?;
            if actual != expected {
                return Err(format!(
                    "composition {label} differs across absolute source roots"
                ));
            }
        }

        let primitives =
            secondary_root.join("build/m0-platform-primitives/objects/platform-primitives.o");
        let low = secondary_root.join("build/composition-kernel-low");
        compile_freestanding(&secondary_root, rustc, &bundle, &primitives, None, &low)?;
        audit_no_undefined(&low, "absolute-path low static composition image")?;
        let execution = run_expect_failure(
            Command::new("/usr/bin/timeout")
                .current_dir(&secondary_root)
                .args(["0.1s"])
                .arg(&low),
            "execute absolute-path low static composition image",
        )?;
        if execution.status.code() != Some(124) {
            return Err(format!(
                "absolute-path low image exited with {}, expected timeout 124",
                execution.status
            ));
        }
        if sha256sum(&low)? != expected_low_sha
            || fs::read(&low)
                .map_err(|error| format!("read absolute-path low composition image: {error}"))?
                != fs::read(work.join("composition-kernel-low"))
                    .map_err(|error| format!("read primary low composition image: {error}"))?
        {
            return Err(
                "low static composition image differs across absolute source roots".to_string(),
            );
        }

        let linker = secondary_root.join("tests/m0/global_allocator_kernel.ld");
        let high = secondary_root.join("build/composition-kernel-high-half");
        compile_freestanding(
            &secondary_root,
            rustc,
            &bundle,
            &primitives,
            Some(&linker),
            &high,
        )?;
        audit_no_undefined(&high, "absolute-path higher-half composition image")?;
        let header = run_checked(
            Command::new("/usr/sbin/readelf").args(["-hW"]).arg(&high),
            "absolute-path higher-half ELF-header audit",
        )?;
        require_output_fragments(
            &header.stdout,
            "absolute-path higher-half ELF header",
            &["Entry point address:               0xffffffff80000000"],
        )?;
        let symbols = run_checked(
            Command::new("/usr/sbin/nm").args(["-C"]).arg(&high),
            "absolute-path higher-half symbol audit",
        )?;
        require_output_fragments(
            &symbols.stdout,
            "absolute-path higher-half symbol audit",
            &[
                "tmk_composition_probe::composition_step",
                "tmk_composition_probe::composition_shell::boot_observation",
                " memcpy",
            ],
        )?;
        reject_unselected_primitive_symbols(&symbols.stdout)?;
        let linked = secondary_root.join("build/linked-primitives");
        super::platform_primitives::audit_linked_composition_primitives(&high, &linked)?;
        if fs::read(linked.join("memcpy.bin"))
            .map_err(|error| format!("read absolute-path linked memcpy: {error}"))?
            != fs::read(work.join("linked-primitives/memcpy.bin"))
                .map_err(|error| format!("read primary linked memcpy: {error}"))?
            || sha256sum(&high)? != expected_high_sha
            || fs::read(&high).map_err(|error| {
                format!("read absolute-path higher-half composition image: {error}")
            })? != fs::read(work.join("composition-kernel-high-half"))
                .map_err(|error| format!("read primary higher-half composition image: {error}"))?
        {
            return Err(
                "higher-half composition output differs across absolute source roots".to_string(),
            );
        }
        Ok(())
    })();

    let cleanup = if secondary_root.exists() {
        fs::remove_dir_all(&secondary_root).map_err(|error| {
            format!(
                "remove absolute-path reproduction root {}: {error}",
                secondary_root.display()
            )
        })
    } else {
        Ok(())
    };
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(cleanup_error)) => Err(format!("{error}; {cleanup_error}")),
    }
}

fn reject_unselected_primitive_symbols(symbols: &[u8]) -> Result<(), String> {
    let symbols = String::from_utf8_lossy(symbols);
    for forbidden in [
        "tmk_alloc_capsule",
        "tmk_alloc_capsule_end",
        "tmk_seal_capsule",
        "tmk_seal_capsule_end",
        "memset",
        "memset_end",
    ] {
        if symbols
            .lines()
            .any(|line| line.split_whitespace().last() == Some(forbidden))
        {
            return Err(format!(
                "composition final link retained unselected primitive `{forbidden}`"
            ));
        }
    }
    Ok(())
}

fn compile_freestanding(
    root: &Path,
    rustc: &Path,
    bundle: &Path,
    primitives: &Path,
    linker: Option<&Path>,
    output: &Path,
) -> Result<(), String> {
    let mut command = rustc_consumer_command(
        rustc,
        root,
        &root.join("tests/m0/composition_kernel_consumer.rs"),
        bundle,
        output,
    );
    command
        .args(["-C", "code-model=kernel"])
        .args(["-C", "link-arg=-nostartfiles"])
        .args(["-C", "link-arg=-no-pie"])
        .args(["-C", "link-arg=-static"])
        .arg("-C")
        .arg(format!("link-arg={}", primitives.display()))
        .args(["-C", "link-arg=-Wl,--build-id=none"]);
    if let Some(linker) = linker {
        command
            .arg("-C")
            .arg(format!("link-arg=-T{}", linker.display()));
    }
    run_checked(&mut command, "link freestanding composition image")?;
    require_file(output, "freestanding composition image")
}

fn audit_no_undefined(image: &Path, label: &str) -> Result<(), String> {
    let undefined = run_checked(
        Command::new("/usr/sbin/nm").arg("-u").arg(image),
        &format!("{label} undefined-symbol audit"),
    )?;
    if !undefined.stdout.is_empty() {
        return Err(format!(
            "{label} has undefined symbols:\n{}",
            String::from_utf8_lossy(&undefined.stdout)
        ));
    }
    Ok(())
}

fn run_bundle_tamper_negatives(
    forge: &Path,
    root: &Path,
    work: &Path,
    primary: &Path,
) -> Result<(), String> {
    let negative = work.join("negative");
    fs::create_dir_all(&negative)
        .map_err(|error| format!("create composition negative path: {error}"))?;

    let shell = negative.join("shell-tamper.verified");
    copy_tree(primary, &shell)?;
    let shell_path = shell.join("evidence/direct-verus/00-composition_shell.rs");
    let mut bytes =
        fs::read(&shell_path).map_err(|error| format!("read shell tamper target: {error}"))?;
    bytes.push(b' ');
    fs::write(&shell_path, bytes).map_err(|error| format!("write shell tamper: {error}"))?;
    reject_bundle(forge, root, &shell, &negative.join("shell-tamper.txt"))?;

    let artifact = negative.join("artifact-tamper.verified");
    copy_tree(primary, &artifact)?;
    let artifact_path = artifact.join(ARTIFACT);
    let mut bytes = fs::read(&artifact_path)
        .map_err(|error| format!("read artifact tamper target: {error}"))?;
    let index = bytes.len() / 2;
    bytes[index] ^= 1;
    fs::write(&artifact_path, bytes).map_err(|error| format!("write artifact tamper: {error}"))?;
    reject_bundle(
        forge,
        root,
        &artifact,
        &negative.join("artifact-tamper.txt"),
    )?;

    let binding = negative.join("binding-tamper.verified");
    copy_tree(primary, &binding)?;
    let receipt_path = binding.join("receipt.json");
    let mut receipt = read_json(&receipt_path, "binding-tamper receipt")?;
    *receipt
        .pointer_mut("/binding_sha256")
        .ok_or_else(|| "binding-tamper receipt has no binding digest".to_string())? =
        Value::String("0".repeat(64));
    fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("serialize binding tamper: {error}"))?,
    )
    .map_err(|error| format!("write binding tamper: {error}"))?;
    reject_bundle(forge, root, &binding, &negative.join("binding-tamper.txt"))?;

    let extra = negative.join("extra-file.verified");
    copy_tree(primary, &extra)?;
    fs::write(extra.join("evidence/unbound.txt"), b"unbound\n")
        .map_err(|error| format!("write extra bundle member: {error}"))?;
    reject_bundle(forge, root, &extra, &negative.join("extra-file.txt"))?;
    Ok(())
}

fn reject_bundle(forge: &Path, root: &Path, bundle: &Path, evidence: &Path) -> Result<(), String> {
    let output = run_expect_failure(
        Command::new(forge)
            .current_dir(root)
            .args([OsStr::new("verify-build"), bundle.as_os_str()]),
        "reject tampered composition bundle",
    )?;
    write_output(evidence, &output, "composition bundle rejection")
}

fn run_build_negatives(forge: &Path, root: &Path, work: &Path) -> Result<(), String> {
    let negative = work.join("negative");
    let standalone = negative.join("rich-standalone.verified");
    let output = run_expect_failure(
        Command::new(forge)
            .current_dir(root)
            .args([
                "build",
                SOURCE,
                "--level",
                "l3",
                "--export",
                EXPORT,
                "--crate-name",
                CRATE_NAME,
                "--target",
                "kernel",
                "--out",
            ])
            .arg(&standalone),
        "reject rich transition as standalone ABI export",
    )?;
    require_output_fragments(
        &output.stderr,
        "rich standalone-export rejection",
        &["outside the v1 verified public ABI"],
    )?;
    if standalone.exists() {
        return Err("rejected rich standalone export published a bundle".to_string());
    }
    write_output(
        &negative.join("rich-standalone-export.txt"),
        &output,
        "rich standalone export rejection",
    )?;

    let canonical_shell = fs::read_to_string(root.join(SHELL))
        .map_err(|error| format!("read composition shell for mutation: {error}"))?;
    let external = canonical_shell.replacen(
        "pub fn boot_observation()",
        "#[verifier::external_body]\npub fn boot_observation()",
        1,
    );
    if external == canonical_shell {
        return Err("external-body mutation target was not found".to_string());
    }
    let external_path = negative.join("external-body-shell.rs");
    fs::write(&external_path, external)
        .map_err(|error| format!("write external-body shell mutation: {error}"))?;
    let external_bundle = negative.join("external-body.verified");
    let output = run_expect_failure(
        &mut composition_build_command(
            forge,
            root,
            external_path
                .strip_prefix(root)
                .unwrap_or(&external_path)
                .to_string_lossy()
                .as_ref(),
            &external_bundle,
        ),
        "reject external_body composition shell",
    )?;
    if external_bundle.exists() {
        return Err("external_body composition shell published a bundle".to_string());
    }
    write_output(
        &negative.join("external-body.txt"),
        &output,
        "external-body shell rejection",
    )?;

    for (fault, evidence_name) in [
        ("composition-after-plan-shell-mutation", "post-plan-shell"),
        ("certificate-l2", "certificate-l2"),
        ("tv-contract-divergent", "tv-nonpass"),
    ] {
        let bundle = negative.join(format!("{evidence_name}.verified"));
        let mut command = composition_build_command(forge, root, SHELL, &bundle);
        command.env("THERMITE_L3_TEST_FAULT", fault);
        let output = run_expect_failure(
            &mut command,
            &format!("reject injected composition fault `{fault}`"),
        )?;
        if bundle.exists() {
            return Err(format!("composition fault `{fault}` published a bundle"));
        }
        write_output(
            &negative.join(format!("{evidence_name}.txt")),
            &output,
            &format!("composition fault `{fault}`"),
        )?;
    }
    Ok(())
}

fn read_json(path: &Path, label: &str) -> Result<Value, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("read {label} {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {label}: {error}"))
}

fn require_file(path: &Path, label: &str) -> Result<(), String> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("{label} does not exist at {}", path.display()))
    }
}

fn write_output(path: &Path, output: &Output, label: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create {label} directory: {error}"))?;
    }
    let mut bytes = output.stdout.clone();
    bytes.extend_from_slice(&output.stderr);
    fs::write(path, bytes).map_err(|error| format!("write {label}: {error}"))
}
