# M0 verified panic host and component-link evidence

`verus/platform/panic_host.rs` supplies the freestanding Rust panic lang item.
Its executable body is verified and deliberately diverges in a fail-stop loop.
Verus requires `core::panic::PanicInfo` to be introduced as an opaque foreign
type; the source audit permits exactly one `external_body` occurrence paired with
`external_type_specification` on that type declaration. It exposes no fields or
methods and does not exempt the panic function from verification.

The following command was run twice after regenerating the allocator and capsule
inputs:

```text
cargo run -p xtask -- m0-host-link
```

Each run:

1. checks exact Verus and GNU binutils digests;
2. requires current passing allocator and capsule reports and stages their exact
   artifacts;
3. audits, verifies, and compiles the panic source with `--no-cheating`, static
   relocation, `panic=abort`, and no red zone;
4. reproduces its rlib in two additional absolute paths;
5. confirms that the rlib has a single panic lang-item symbol and no undefined
   symbols;
6. links the allocator policy, panic lang item, and registered capsule into a
   static x86_64 ELF;
7. requires exactly two executable sections in one read/execute load segment,
   with no writable/read-only runtime data, relocations, dynamic section, or
   unresolved symbols;
8. extracts the linked capsule and requires the exact proved bytes
   `48 89 f8 f4`;
9. checks disassembly for the capsule, allocator, and panic entry;
10. executes the ELF and observes timeout status 124, showing that its entry
    remains in the fail-stop loop rather than returning or faulting;
11. reproduces the ELF in two additional link paths; and
12. rejects a panic loop without an explicit divergence allowance, an
    `external_body` annotation on the executable panic function, and injected
    writable data.

Stable positive identities:

```text
panic_source_sha256=744713ca5e4bcf0a671b1f1a452e6058ba8aeb4b02b616216fa2a29514503911
panic_artifact_sha256=48bdebcba3090800b1bbb64524706660e5611a1a588e8d3b7ed5cec7b28967d6
panic_verus_result_sha256=f51bd9e0242c3c6199a864e0514c3f207baa913970f79fe22b2bd9615311cac0
allocator_artifact_sha256=360717170e53f505ade8e81a120277cd4322d63cb1c08f9e6a158724fc9ab77d
capsule_object_sha256=1ef41489c02ddcbddc9b12fdb480422c7e039921a75fc147a6ba7e6822bf4cfb
linker_script_sha256=c085e150e9f7aae1ce25915e341c0251b1de07eb2112ba98a54ec1fcb6cfffc2
host_elf_sha256=3a613c5e5c9911255871e4b2e4207eb23f93fa1ef5285994a56bdb339b5e015c
linked_capsule_sha256=86f039964fb227ba98078e671367c11641ed25204ea080f1b5b30bd13c5deda8
runtime_observation=fail-stop-timeout-124
```

Post-link evidence digests:

```text
sections.txt     4b320f263429afdc3013ef12bcdbb31b05340a3219c19dff630e4c948ba71b06
segments.txt     44987769786f99fb875c8ceb297ecdc6bf5d1a8c7a6a87f13742273b403e498f
disassembly.txt  c71e74f37d8da9ff89fab848361a3262ab41e53d23d386ebfefa0a253c74c50c
report.txt       ab45f434f1fc6ea977412c5b4042285c53ea1206dffbf2ecb86ee349b516d986
```

The linked ELF is a component gate, not a boot image: the panic loop does not yet
invoke the HLT capsule, the allocation policy is not yet a byte/layout-aware
`GlobalAlloc`, and Forge/composition receipts, the release manifest, PE/COFF UEFI
loader, and QEMU boot observation remain outstanding. Accordingly the report
states `release_eligible=false`.
