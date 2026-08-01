# M1 raw UEFI memory-map acquisition and decoding

Status: **accepted M1 subcomponent** against public Thermite `main` commit
`b8dc3947f504454775aa70977d8bda5da677d2af`. This checkpoint closes bounded
loader-owned map acquisition, content-preserving raw descriptor decoding,
map-key observation, and buffer release. It deliberately does not retain the
map for, or call, `ExitBootServices`.

## Accepted boundary

The boundary has two independently executable halves.

The same-crate Thermite/direct-Verus artifact
`tmk_firmware_raw_map` accepts a `&[u8]` containing the exact bytes returned by
firmware. It checks a nonempty map no larger than 1 MiB, a nonzero key,
descriptor version 1, a descriptor stride in `40..=256` divisible by eight, an
exact stride/count relation, and at most 4096 descriptors. Checked
multiplication derives every offset. For every descriptor it reads the UEFI
2.11 fields at offsets 0, 8, 16, 24, and 32, ignores but preserves any future
extension bytes in the stride, validates physical and virtual bounds and known
attributes, derives cache/runtime metadata, and invokes the accepted Thermite
`memory_map_step`. Success proves every descriptor and ordering relation, the
last physical end, and a nonzero bounded usable-page total. Type 15
`EfiUnacceptedMemoryType` is reserved rather than usable.

The exact 1016-byte x86_64 EFIAPI capsule validates the registered system and
boot-services table prefixes, including `GetMemoryMap`, `AllocatePool`, and
`FreePool` at offsets 56, 64, and 72. It performs the null-buffer size probe,
adds a 512-byte growth margin, allocates at most 1 MiB from `EfiLoaderData`,
performs a second real `GetMemoryMap`, validates every returned descriptor with
the same limits, requires conventional memory, and calls `FreePool` on every
post-allocation path. The 168-byte call frame supplies 32 bytes of EFIAPI shadow
space and is 16-byte aligned at each call. Only successful validation plus a
successful free emits `TMK_MAP_OK\n` and returns `EFI_SUCCESS`.

The direct-Verus machine model registers all 127 little-endian words of the
capsule. It makes table, stack, target, returned-buffer, map-byte, firmware
return, and nonvolatile-preservation obligations explicit and conditional on
the first operation that dereferences or calls them. It proves call sequencing,
allocation size/pool type, scan count, all-path free behavior, ownership at
return, exact marker bytes, stack restoration, return address, and all x86_64
EFIAPI nonvolatile registers. OVMF remains an environmental implementation, not
verified code.

The live shakedown exposed an earlier policy defect: OVMF returns a runtime
type-11 MMIO descriptor with `UC | RUNTIME`. The corrected Thermite policy now
requires it for types 5/6, permits it for types 11/12, and rejects it elsewhere.
Both 64/64 mutation batteries still pass, and the older firmware-policy receipt
was regenerated and replayed.

The offsets and descriptor layout follow the official
[UEFI 2.11 boot-services chapter](https://uefi.org/specs/UEFI/2.11/07_Services_Boot_Services.html),
[system-table chapter](https://uefi.org/specs/UEFI/2.11/04_EFI_System_Table.html),
and [x64 calling convention](https://uefi.org/specs/UEFI/2.11/02_Overview.html).

## Acceptance commands

```text
cargo run -p xtask -- m1-firmware-raw-map
cargo run -p xtask -- m1-uefi-raw-map
```

The first command audits and batteries the Thermite source; produces three
byte-identical strict L3 composition receipts, combined sources, kernel-vstd
dependencies, and rlibs; validates all three and replays the primary; executes
one valid raw map and 17 malformed maps; rejects two false whole-crate proofs;
and rejects a changed receipt binding.

The second command verifies 21 direct-Verus obligations three times, executes
33 model scenarios three times, and requires identical rlibs, consumers,
verification JSON, and 1016-byte emissions. It builds and audits three identical
1536-byte one-section PE32+ images and three identical fixed FAT16 images. The
registered image boots with pinned OVMF on QEMU `q35` under both TCG and KVM;
both runs emit exactly `TMK_MAP_OK\n`. Proof escape, semantic, exact-image,
four call-site byte, PE, FAT, extra-byte linker, and malformed-firmware-image
mutations reject.

## Stable composed-decoder result

```text
M1_FIRMWARE_RAW_MAP_OK
component_verified=true
release_eligible=false
candidate_pin_verified=true
receipt_validated=true
receipt_replayed=true
source_sha256=48bf69891a07fa9888108d04df16c3eb83bfbe69a07f6b8d9fa4180f14c2668f
shell_sha256=30121a3f712ecf43f320d8edd3d1a1dd8dcdb14f9863cfff564fdb7259c834a0
consumer_source_sha256=c291a29bfd9492ef13f0cc6614288902c1e9c44cc78f712622acc74fa62f7ca1
combined_source_sha256=f46e434205d7abd7d5fae9f7b11d11b976ffdcdc12c25ba166fad76e7c461a51
receipt_sha256=c9cf2f92febaf168eeeac37787389e54b1e53824d1558ff159e35cb7ceed2634
binding_sha256=094207f4ca7dad6f3ccbe15ebb65ab2074cca229d479df3b46e6429bc559a7b2
artifact_sha256=e9747d6ba720f48a39ee53a3cd56bc33dc5be6cb2feaa8c4990b268421f436cc
consumer_sha256=09c6fa626aa69cd39a356f8933af3321d025f6d78e8a7a44dea5819a9472c3e1
reproducibility_builds=3
uefi_unaccepted_memory_type=reserved
runtime_required_types=5,6
runtime_optional_types=11,12
future_descriptor_tail_preserved=true
checked_offset_multiplication=true
runtime_marker=M1_FIRMWARE_RAW_MAP_OK descriptors=6 size=48 key=77 usable=16 runtime-mmio=both unaccepted=reserved negatives=17
```

## Stable live-capsule result

```text
M1_UEFI_RAW_MAP_OK
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
call_frame_bytes=168
probe_get_memory_map_called=true
allocate_pool_called=true
allocation_pool_type=EfiLoaderData
second_get_memory_map_called=true
raw_descriptors_scanned=true
free_pool_all_post_allocation_paths=true
map_key_observed=true
map_key_retained=false
exit_boot_services_called=false
runtime_mmio_accepted=true
model_source_sha256=87555139451ae45e4eecf7e59bec192ff1f81926d54028690435a1479d073918
consumer_source_sha256=74015517f890ff7edc07b73a790330e26d6dca1053dd62a3bcef7be5c8243177
linker_script_sha256=a306abcf044cfe2fb8517215d040dc149c1721550e0afaaaf4b4793476c33995
auditor_sha256=16ee6477c21da0c57f67b7ce7890b41f83e6c66688b89164115374ab61a844ce
model_artifact_sha256=7c091368b793226c3e548429c918923e01e7560515d0101865982158eac5d9ff
verification_result_sha256=146ef1fbbc50bbdaf4570990881a53ed1447136f4fe9cd8ce1588a5d1e6dbd8a
consumer_sha256=3e5d9a310fa5031b69b48e1ecd26e3c1706a9159d27779a9afbdc33b80c94662
entry_sha256=2d6649e99a08d6c561eb26f3003d9e2f16fa9bf29190214646c1ece0e6ab9278
pe_sha256=40c2ad3f53dc8465f9015c318ea9f31e4b3a364fca5e2c60a9af4d8ee490b6fe
image_sha256=7e0a41f426630fc0db58d06bfd95c25445c71af7999a7bfda4e8b64f9487d580
forge_source_identity=b8dc3947f504454775aa70977d8bda5da677d2af
forge_sha256=b073fa34a955dc4ce723aac3cdba36ed031e7daa1ca5db6ead866d41ef36fbf9
verus_verified=21
model_reproducibility_builds=3
model_consumer_executions=3
entry_reproducibility_emissions=3
pe_reproducibility_builds=3
image_reproducibility_builds=3
tcg_marker=TMK_MAP_OK\n
kvm_marker=TMK_MAP_OK\n
```

Generated evidence remains under ignored `build/m1-firmware-raw-map/` and
`build/m1-uefi-raw-map/`.

## Remaining boundary

This capsule frees its buffer before return, so the observed map key becomes
stale by construction and is not suitable for `ExitBootServices`. The next
slice must reacquire and validate a final map into storage that remains owned,
call `ExitBootServices` without an intervening map mutation, handle
`EFI_INVALID_PARAMETER` with the already bounded Thermite retry policy, and
transfer the retained normalized map into `BootInfoV1`. Physical page
installation, kernel handoff, and final signed `M1_OK` remain separate gates.
