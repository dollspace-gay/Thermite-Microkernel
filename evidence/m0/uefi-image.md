# M0 reproducible UEFI image evidence

The M0 empty-image gate is executable through:

```text
cargo run -p xtask -- m0-uefi
```

This command does not merely compile a UEFI test. It builds a real FAT16 disk
image, boots that image through pinned OVMF in QEMU, and requires an exact
firmware-visible observation under both software and hardware acceleration.

## Verified entry capsule

`verus/machine-model/uefi_debug_exit_capsule.rs` defines the registered 56-byte
x86_64 entry encoding and its machine-state transition. The accepted encoding:

1. selects QEMU debug port `0xe9`;
2. emits the 16 bytes `TMK_M0_UEFI_OK!\n`;
3. sets `RAX` to `EFI_SUCCESS` (`0`);
4. preserves `RBX`, `RCX`, and `RSP`; and
5. returns to firmware.

Every other encoding is rejected by the model without changing the supplied
abstract state. Verus runs with `--no-vstd --no-cheating --compile`, verifies all
three functions with zero errors, and produces the rlib consumed by the byte
emitter. The command scans the canonical source for `assume`, `admit`, axioms,
executable `external_body`, `unsafe`, and inline assembly before proof.

The emitter reconstructs all 56 bytes from the proved seven-word value, checks
each instruction boundary and output byte, executes the accepted and rejected
model paths, and writes `entry.bin`. The runtime marker is:

```text
M0_UEFI_CAPSULE_OK:56:00e9:0000000000000000
```

## PE and FAT closure

Pinned GNU `objcopy` and `ld` place only the registered bytes in a minimal PE32+
EFI application. `xtask/src/uefi.rs` independently parses the resulting bytes and
requires:

- x86_64 PE32+, EFI application subsystem, fixed image base, NX compatibility,
  and no dynamic-base flag;
- a zero timestamp, no symbols, relocations, imports, or other data directories;
- exactly one read/execute `.text` section;
- 56 exact entry bytes followed only by zero padding; and
- a 1 KiB total PE file.

Pinned `mkfs.fat --invariant`, `touch`, and `mcopy` create a 32 MiB FAT16 image.
The independent parser validates the fixed BPB/media geometry, mirrored FATs,
reserved entries, canonical short-name directory sets and dates, bounded
non-looping cluster chain, and exact `EFI/BOOT/BOOTX64.EFI` payload. It does not
trust `mtools` output as proof of the filesystem it produced.

Three independent model builds, PE links, and FAT image builds are required to be
byte-identical. The positive artifact identities are:

```text
source_sha256=663fc067cf75945e287c83777c35b48438cbb2b311e2cde795a1f78435290125
model_artifact_sha256=f4742488e41c034c7b2888aa9ae9728852a07f26e77426eaa1da7301778515e8
entry_sha256=982514cbcb73e37d2b0fa23c889fc98b9117909c511396a13f81154a1a04dbd5
pe_sha256=e5e1a277ecde916f2ea5f043c153839652fc7e505256cb5032d4762d2e29921b
image_sha256=20c49bf740d51384f4aec2d047fc2e69bc0f03b8fbf30a880d351f9f94d4a99d
```

## Real firmware observations and negative cases

The command boots the raw FAT image using OVMF on QEMU `q35` twice:

```text
TCG:  TMK_M0_UEFI_OK!\n
KVM:  TMK_M0_UEFI_OK!\n
```

Each debug log is exactly 16 bytes. A third TCG run corrupts the embedded PE's
`MZ` signature in the disk image; OVMF emits none of the marker (the debug log is
empty). This demonstrates firmware selection and execution of the audited media,
not merely execution of a host-side consumer.

Eight negative cases must fail at their named gate:

- wrong accepted-state semantics fails the Verus postcondition;
- injected `assume(false)` fails `--no-cheating`;
- changed PE executable byte;
- nonzero PE timestamp;
- non-EFI PE subsystem;
- changed executable byte inside FAT media;
- missing fallback `EFI` path; and
- malformed PE rejected by real OVMF.

The report correctly states `release_eligible=false`. This artifact is the M0
empty-image and proof-to-firmware gate. It is not the M1 UEFI loader: it does not
yet produce `BootInfo`, normalize the firmware memory map, call
`ExitBootServices`, load the kernel/service bundle, or carry the still-pending
Thermite/direct-Verus rich-state composition receipt and final-link allowlist.
