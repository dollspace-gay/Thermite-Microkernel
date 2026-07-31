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
2. requires current passing fixed-unit allocator, byte/layout allocator, and
   capsule reports and stages their exact artifacts;
3. audits, verifies, and compiles the panic source with `--no-cheating`, static
   relocation, `panic=abort`, and no red zone;
4. reproduces its rlib in two additional absolute paths;
5. confirms that the rlib has a single panic lang-item symbol and no undefined
   symbols;
6. links both allocator policies, the panic lang item, and registered capsule
   into a static x86_64 ELF;
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
byte_allocator_artifact_sha256=a177589091da931699844d6d7ede9b58b86582afd1e23d272c55a9baf435f04a
capsule_object_sha256=1ef41489c02ddcbddc9b12fdb480422c7e039921a75fc147a6ba7e6822bf4cfb
linker_script_sha256=c085e150e9f7aae1ce25915e341c0251b1de07eb2112ba98a54ec1fcb6cfffc2
host_elf_sha256=2103affbb1480ea9f0149d5d457184d02ccebd612d0b2bbad1d5f1cdc73390cb
linked_capsule_sha256=86f039964fb227ba98078e671367c11641ed25204ea080f1b5b30bd13c5deda8
runtime_observation=fail-stop-timeout-124
```

Post-link evidence digests:

```text
sections.txt     9c8ed1af2fedbc5d9ef8b87436579f8af442996a181b56c4fe1afd208555f8cc
segments.txt     200882cd6bf2a289cd43d5713c949247debebd458e72554cafee925ce47fd3eb
disassembly.txt  6745af9080e36d3ae7dc56c6d6a1537e436164b3bdfd9c6ce953f7b43480c685
report.txt       fa55072bd017d57a28e76b6d3a4fcdb3045317b5cfd10e4c834048ac15937b0b
```

The linked ELF is a component gate, not a boot image: the panic loop does not yet
invoke the HLT capsule, the verified byte/layout policy is not yet connected to
Rust's raw-pointer `GlobalAlloc` ABI, and Forge/composition receipts, the release
manifest, PE/COFF UEFI loader, and QEMU boot observation remain outstanding.
Accordingly the report states `release_eligible=false`.
