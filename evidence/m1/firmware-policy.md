# M1 firmware memory-map and exit policy

Status: **accepted M1 subcomponent** against public Thermite `main` commit
`b8dc3947f504454775aa70977d8bda5da677d2af`. This is a verified policy over
normalized firmware responses, not yet a complete UEFI loader or M1 exit gate.

## Accepted implementation

`thermite/boot/firmware_policy.th` contains two state machines:

- `memory_map_step` validates 40–256 byte, eight-byte-aligned version-1
  descriptors; bounds the raw map to 1 MiB and 4096 entries; requires aligned,
  nonempty, sorted, non-overlapping physical ranges below the 52-bit physical
  limit; validates normalized cache/runtime metadata; and maps UEFI types to
  reserved, loader-reclaimable, boot-reclaimable, usable, bad, ACPI-reclaimable,
  ACPI-NVS, MMIO, and persistent classes. UEFI 2.11 type 15 unaccepted memory
  remains reserved. Runtime code/data (types 5 and 6) require the runtime bit;
  types 11 and 12 may additionally carry it for runtime MMIO/MMIO-port ranges,
  while other types may not.
  Completion requires usable memory.
- `firmware_exit_step` allows at most eight map calls and four invalid-key exit
  retries. `BUFFER_TOO_SMALL` grows capacity to `required + 512`, enough for two
  maximum-size descriptors. A successful map binds its key and count; an invalid
  key clears them and forces reacquisition; only a successful exit can reach the
  terminal state.

The direct-Verus shell runs a good three-range normalization, including runtime
MMIO, and a real policy
trace of buffer resize, key 77 rejection, key 78 reacquisition, successful exit,
and finish. A separate Rust crate calls the emitted observation.

## Acceptance command

```text
cargo run -p xtask -- m1-firmware
```

The command checks the Thermite skill; audits and batteries both functions;
requires L3/end-to-end scope, no slag/boundary, and 64/64 killed mutants for each;
builds three exact-source kernel composition bundles; compares receipts, combined
sources, and rlibs byte-for-byte; validates all three and replays the primary;
executes the receipt-toolchain consumer; and runs these negative cases:

- misaligned descriptor size;
- overlapping ranges;
- runtime/type mismatch;
- unknown attributes;
- no usable range;
- oversized map request;
- firmware device error;
- exhausted map attempts;
- exhausted exit retries;
- wrong exit key;
- zero successful-map key;
- a false observation postcondition;
- a false stale-key action expectation; and
- a changed receipt binding digest.

False proof shells publish no partial bundle.

## Stable result

```text
M1_FIRMWARE_OK
component_verified=true
release_eligible=false
receipt_validated=true
receipt_replayed=true
source_sha256=48bf69891a07fa9888108d04df16c3eb83bfbe69a07f6b8d9fa4180f14c2668f
shell_sha256=61539b9aba1a2e0209a85f0c192168a90989a3e3456e29befb65600143d81de5
combined_source_sha256=19e5aa8f270d846c177775198e53acc6f52d56fe31e954afcc0a0d8c596899dc
receipt_sha256=21cda9cd0ed80057ccefaef153197a53416acee474c283bb9cace2cbbd0670a3
binding_sha256=716569411ef00beb835ee12104d218e8a620ad005eca674727824b6a8bef8d35
artifact_sha256=9915fde3e088f92e9d6c380498016d86aa7245f19644476c5b08c33894b67f2f
consumer_sha256=07daf9bdba76037b0d9117efb18fb4c38dc8598a7972e62672fc2c881196d388
reproducibility_builds=3
runtime_marker=M1_FIRMWARE_POLICY_OK observation=255
```

Generated bundles and logs remain under ignored `build/m1-firmware/`.

## Remaining boundary

The accepted [raw UEFI memory-map checkpoint](firmware-raw-memory-map.md) now
allocates a bounded loader-owned buffer, performs the second real
`GetMemoryMap`, validates every raw descriptor, observes the map key, composes a
content-preserving decoder with this policy, and frees the buffer on every
post-allocation path. Because `FreePool` changes the firmware map, this slice
does not retain its observed key or claim `ExitBootServices`. Bounded final-map
reacquisition without a subsequent allocation/free, real `ExitBootServices`,
page installation, and final `M1_OK` remain open.
