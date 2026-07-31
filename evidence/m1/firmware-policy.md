# M1 firmware memory-map and exit policy

Status: **accepted M1 subcomponent** against Thermite `v0.0.2` commit
`845d684f00e829491ee4c537818fba2689bcaefc`. This is a verified policy over
normalized firmware responses, not yet a complete UEFI loader or M1 exit gate.

## Accepted implementation

`thermite/boot/firmware_policy.th` contains two state machines:

- `memory_map_step` validates 40–256 byte, eight-byte-aligned version-1
  descriptors; bounds the raw map to 1 MiB and 4096 entries; requires aligned,
  nonempty, sorted, non-overlapping physical ranges below the 52-bit physical
  limit; validates normalized cache/runtime metadata; and maps UEFI types to
  reserved, loader-reclaimable, boot-reclaimable, usable, bad, ACPI-reclaimable,
  ACPI-NVS, MMIO, and persistent classes. Completion requires usable memory.
- `firmware_exit_step` allows at most eight map calls and four invalid-key exit
  retries. `BUFFER_TOO_SMALL` grows capacity to `required + 512`, enough for two
  maximum-size descriptors. A successful map binds its key and count; an invalid
  key clears them and forces reacquisition; only a successful exit can reach the
  terminal state.

The direct-Verus shell runs a good three-range normalization and a real policy
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
source_sha256=22188bc9c61f45255daf74e828bd0de41e2068d35ffaf8ca8c178279d14338c5
shell_sha256=df581c96cb83e0678559afa409a89748a0f9e7f99be1c02273778d4df7b2a720
combined_source_sha256=51cbb8baabedc18fc205232df3a3bb9e543efc917f75c2cc91a23915e276cbf6
receipt_sha256=b2c8981886ae8a3c578386d79ef5d442bb520cec948bf73bcdda0f5655b9c07c
binding_sha256=e904118bbbd580d37ca051463b2dee7972710818e72535fb1751c13d366e02b1
artifact_sha256=4b4a490c81634ee0e5502bd5d5b7fabd319a69d2227f6abbc13caf265efc0dba
consumer_sha256=409bb6840d3c5f8ab7b079775f101b5dd3a737f2eaad83682f451ed896e9bff6
reproducibility_builds=3
runtime_marker=M1_FIRMWARE_POLICY_OK observation=255
```

Generated bundles and logs remain under ignored `build/m1-firmware/`.

## Remaining boundary

The shell currently supplies scalar normalized firmware observations. The
registered UEFI calling-convention capsule must still issue the indirect calls,
validate raw pointers/status values, and connect raw descriptor bytes to these
events. Content-preserving byte reads remain tracked by Thermite
[#108](https://github.com/dollspace-gay/Thermite/issues/108). Actual OVMF map
growth/key invalidation, `ExitBootServices`, page installation, and final `M1_OK`
are not claimed by this checkpoint.
