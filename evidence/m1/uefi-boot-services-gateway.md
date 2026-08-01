# M1 UEFI boot-services gateway

Status: **accepted M1 subcomponent** against public Thermite `main` commit
`b8dc3947f504454775aa70977d8bda5da677d2af`. This checkpoint implements and
executes the first real x86_64 EFIAPI service call. It validates the system and
boot-services table prefixes, calls `GetMemoryMap` with a null descriptor buffer,
accepts only `EFI_BUFFER_TOO_SMALL` with a bounded nonzero required size, and
returns an EFI status. It does not decode descriptors or call
`ExitBootServices`.

## Accepted boundary

The direct-Verus model registers one exact 308-byte PE entry image. At entry it
requires the UEFI x64 environment: long mode, identity-readable registered table
prefixes, DF clear, a readable return word, and an entry `RSP` congruent to 8
modulo 16. The image then performs these concrete operations:

1. Reject null or unaligned `RDX`, then require the system-table signature and a
   header size of at least 104 bytes.
2. Load `BootServices` from system-table offset 96; reject a null or unaligned
   value, then require the boot-services signature and a header size of at least
   64 bytes.
3. Load the `GetMemoryMap` target from boot-services offset 56 and reject zero.
4. Subtract 104 bytes from `RSP`, preserving 32 bytes of EFIAPI shadow space.
   Place the fifth argument at `[RSP+32]`, with local outputs at offsets 40, 48,
   56, and 64. The indirect call receives `&MemoryMapSize`, null `MemoryMap`,
   `&MapKey`, `&DescriptorSize`, and `&DescriptorVersion` through
   `RCX/RDX/R8/R9/stack`.
5. Accept only raw status `0x8000000000000005` and a returned required size in
   `1..=1 MiB`. Restore the exact entry stack and emit
   `TMK_M1_UEFI_GATE_OK\n` only on this path. Every checked failure restores the
   stack if allocated, emits nothing, and returns
   `EFI_LOAD_ERROR` (`0x8000000000000001`).

The image itself uses only volatile EFIAPI registers. Preservation across the
indirect call is explicitly conditional on the firmware honoring the ABI; OVMF
is an environmental component, not verified code. Registered-readable table,
stack, call-target, return, and firmware-return obligations are explicit model
inputs rather than unchecked Rust references.

The offsets, signatures, status value, five-argument layout, shadow space, and
stack alignment are derived from the official
[UEFI 2.11 system-table](https://uefi.org/specs/UEFI/2.11/04_EFI_System_Table.html),
[boot-services](https://uefi.org/specs/UEFI/2.11/07_Services_Boot_Services.html),
and [x64 calling-convention](https://uefi.org/specs/UEFI/2.11/02_Overview.html)
chapters.

## Acceptance command

```text
cargo run -p xtask -- m1-uefi-gateway
```

The command pins and checks the repaired public Forge/skill pair, rejects proof
escapes, verifies 16 direct-Verus obligations three times, executes 15 model
scenarios three times, and requires byte-identical model, verification, consumer,
and 308-byte image artifacts. It then builds and audits three identical 1-KiB
PE32+ images and three identical fixed FAT16 images. The PE has one RX `.text`
section, no relocations or nonempty data directories, and an entry exactly equal
to the registered bytes. Independent disassembly checks both table offsets, the
104-byte frame, fifth stack argument, indirect `call *%r10`, raw status/size
checks, stack restoration, and error return.

The FAT carrier deliberately reuses the already accepted fixed M0 transport
geometry and volume identity. This gate owns and distinguishes the M1 PE entry
bytes and their live firmware behavior; it does not claim the final M1 loader
media or manifest.

The positive FAT image boots with pinned OVMF on QEMU `q35` under both TCG and
KVM. Each run emits exactly `TMK_M1_UEFI_GATE_OK\n`, which is reachable only
after OVMF returns `EFI_BUFFER_TOO_SMALL` and a bounded nonzero map size. A
malformed PE boots under OVMF but emits no success marker. Proof, model, PE, FAT,
call-opcode, extra-byte, and firmware negatives all reject.

## Stable result

```text
M1_UEFI_GATEWAY_OK
component_verified=true
release_eligible=false
candidate_pin_verified=true
forge_skill_current=true
hardware_executed=true
qemu_executed=true
tcg=true
kvm=true
uefi_spec_version=2.11
efiapi=x86_64
system_table_boot_services_offset=96
boot_services_get_memory_map_offset=56
system_table_required_bytes=104
boot_services_required_bytes=64
call_frame_bytes=104
shadow_space_bytes=32
call_site_stack_aligned=true
nonvolatile_registers_preserved=true
return_address_preserved=true
dereference_footprint_conditional=true
get_memory_map_called=true
get_memory_map_arguments=5
descriptor_buffer_null=true
required_size_observed=true
required_size_limit=1048576
exit_boot_services_called=false
raw_descriptors_decoded=false
environmental_assumption=OVMF-implements-UEFI-2.x-boot-services
model_source_sha256=f7f98520e536bffffd82b1b06143f368167a8649e9e689dc3c6ceb2301bfd02c
consumer_source_sha256=c4ab61aa58b6747c89f3d06c9b75e56e4aab37b0c9ab79882b62ebbc89a1288e
linker_script_sha256=3718c161ed1e1bf5ca24f0920d641c08aa90a4774043f9f2596e3caf6a3e2dc9
model_artifact_sha256=0873b8a9d6c8f914e1a19fddbb87db9c885d2ab7e695a0f5defe38c8ab4b6210
verification_result_sha256=c25d644c8500cc44d42f749cc7e6ebc42d676a1b1603b0654aadb6c5770f2331
consumer_sha256=c0f3d8ea8e2a6d5a3990eda58476c6ac693d42fc33541ce9abeb5b2b3860951b
entry_sha256=31ba989f27b7ca424ffbc214db0a98186781f429327186a17019ee3d08f7353b
pe_sha256=e3a513f9a92c10bbec5b896229f2084e5add1de1f39f7bd1d48880d302f761b0
image_sha256=6b384245115ded3ca3deadff4b35264e74c350eed3b6a711d280779f16d76ad2
ovmf_code_sha256=4e87e4be6bb9cdced848ec0b43adab3c7f15623e36055525d0691d137eb74af9
ovmf_vars_sha256=6ed987af3a3c155be71665f510eae3e007eda9b8b94afd59d45e91c4a11565cc
forge_source_identity=b8dc3947f504454775aa70977d8bda5da677d2af
forge_sha256=b073fa34a955dc4ce723aac3cdba36ed031e7daa1ca5db6ead866d41ef36fbf9
verus_verified=16
model_reproducibility_builds=3
model_consumer_executions=3
entry_reproducibility_emissions=3
pe_reproducibility_builds=3
image_reproducibility_builds=3
```

Generated evidence remains under ignored `build/m1-uefi-gateway/`.

## Remaining boundary

This gate proves and executes only the initial size probe. The following
[raw UEFI memory-map checkpoint](firmware-raw-memory-map.md) now allocates and
owns a bounded buffer, issues the second `GetMemoryMap`, validates every raw
descriptor, composes a content-preserving decoder with the accepted policy,
observes the map key, and frees the buffer on every post-allocation path. Final
map retention, bounded real `ExitBootServices` calls, physical copying, page
installation, kernel handoff, and final signed `M1_OK` remain open.
