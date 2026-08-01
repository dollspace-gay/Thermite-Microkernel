# M1 raw BootInfo decoder

Status: **accepted M1 subcomponent** against exact public Thermite `main`
commit `b8dc3947f504454775aa70977d8bda5da677d2af`. The kernel-byte-slice work from
[#109](https://github.com/dollspace-gay/Thermite/pull/109) and the later Forge
receipt/composition repairs from PRs #112 and #113 are merged; issues #108,
#110, and #111 are closed. The candidate-bound receipts were regenerated and
replayed after the coordinated repin.

## Accepted implementation

`thermite/boot/boot_policy.th` is an L3/end-to-end state machine with a 64/64
mutation battery. `tests/m1/bootinfo_shell.rs` is a same-crate direct-Verus
decoder over the real `&[u8]` boundary. It uses exact little-endian readers and
proves that `code == 0` implies `bootinfo_accepted(bytes, result)`.

That success predicate covers:

- ABI magic/version, all fixed offsets/sizes, flags, reserved fields, and the
  XOR checksum over the complete 256-byte header;
- exact input length and map/command/seed containment before variable reads;
- aligned, bounded kernel/service/config ranges and digest presence;
- seed and framebuffer rules plus aligned nonzero RSDP;
- every normalized range's exact start/end/kind bytes;
- page alignment, physical ceiling, kind bound, sorted non-overlap, and all 12
  reserved bytes in every 32-byte record; and
- exact returned range count, final range end, and BSP APIC ID.

The Thermite contract exposes the accepted map count and exact accepted range
end so the direct decoder can carry these facts through its bounded loop. The
loop proves the full map is contained before the first range read and retains
quantified validity/order for every processed entry.

## Kernel slice model

The issue #108 repair preserves an explicit `--no-vstd` kernel profile while
importing the pinned verified `vstd.vir` model and a separately generated erased
`no_std` rlib used for Rust name resolution/linking. The receipt binds:

- `vstd.vir` digest;
- the 120-file/1,516,424-byte vstd source-tree digest;
- the generated `kernel-vstd-link.rs` source and build arguments;
- the generated `libvstd.rlib`; and
- the exact combined proof source and kernel artifact.

The erased link skeleton is not a new semantic authority: slice semantics come
from the imported, already-verified `vstd.vir`; exact digests make the split
auditable and replayable.

## Acceptance command

```text
cargo run -p xtask -- m1-bootinfo
```

The command performs three independent proof/codegen builds; validates and
replays the receipt; requires byte-identical receipts, combined sources, kernel
artifacts, link rlibs, and link sources; and executes a separately compiled
consumer. The consumer accepts a two-range 320-byte image and rejects 12
malformed cases covering truncation, magic, checksum, reserved header data,
missing digest, framebuffer state, kind, overlap, alignment, both halves of the
12-byte range reservation, and truncated variable map.

Both a freestanding rlib consumer and an ELF64/x86-64 `_start` image link against
the real verified decoder. Three source mutations fail whole-crate Verus: a
wrong byte model, an offset overrun, and omission of the upper eight reserved
bytes. Receipt, generated-vstd-source, and generated-vstd-rlib tampering all fail
verification.

## Stable result

```text
M1_BOOTINFO_OK
component_verified=true
release_eligible=false
candidate_pin_verified=true
receipt_validated=true
receipt_replayed=true
source_sha256=1569fe008a9d061a7138498ad809bbff2745f372face057522b11571517e0e98
shell_sha256=7f45d0d591b03f7f12574becadc77d090d1276548f8ffee23386c0d45bb03b45
consumer_source_sha256=c57d6e50de9431aa8b1c6500f521ae6fdb31e912c29673497ef14f7a831cdda3
freestanding_source_sha256=f44c72edf361a4edf5e2052fdb40d37edcd2f4579274efce9f3d834f6cb99e3e
combined_source_sha256=9021b2cabfd25a44546fb01e3b322d3aad22f78dc040b805ab3940a69d2c85bb
receipt_sha256=574ce4c08940dfb3531604254ccf99b4a2dac8661a31b2e95de77ed746c4f68d
binding_sha256=ba221c8abe1aea50332418844a69b8f3cddfa27a44ddb1fd037e8b15644a6bac
artifact_sha256=42f873f661eac450ec9de17fc7629dc98fec2eae0c23ee8c390507215751e7f3
consumer_sha256=8df7a68c39bfde9ea0ff592bc36608da62229f4cfe702595baaced110d082929
freestanding_rlib_sha256=1675925c841b4b9f332e7f6adbff8453b0902db465ed212ec63047c7a3bc938b
freestanding_elf_sha256=d1754aa932b92526d6f53c643cfb77bb57c9b3bdde1128f9ec0a2c9406d74bb7
kernel_vstd_vir_sha256=d6622a14a77948332f9601bfc9bd0fb71b7aca6f9ea7b10345c8a825fc5fff7e
kernel_vstd_source_sha256=75b9e1b3277f143f3a0c7424f8dccc73f44b00d4f7dcee1e69aa619d6cf35016
kernel_vstd_link_source_sha256=414a10d0b1f819004e300ab34197fda3d281f6b4dd8261cf1b9709ffa51fbda5
kernel_vstd_link_rlib_sha256=428b45dec5a144db16af5da29a06e9c5490d6dcf891c3fcc73a4ae2703bcd6dd
forge_source_identity=b8dc3947f504454775aa70977d8bda5da677d2af
reproducibility_builds=3
verified_success_contract=header,checksum,digests,framebuffer,map-bounds,range-content,range-order,reserved-zero,last-end,bsp-apic-id
freestanding_links=rlib,elf64-x86-64
runtime_marker=M1_BOOTINFO_OK ranges=2 last=0000000000a00000 bsp=7 negatives=12
runtime_negative_cases=12
negative_cases=wrong-byte-model,map-overrun,reserved-tail-omission,receipt-tamper,vstd-source-tamper,vstd-rlib-tamper
```

Generated build, proof, replay, consumer, freestanding-link, and negative evidence
remains under ignored `build/m1-bootinfo/`.

## Remaining boundary

This component validates a complete byte image; it does not allocate or populate
that image from UEFI descriptors. The loader-side encoder/copy, raw UEFI
descriptor decoder, firmware call gateway, and live handoff remain separate M1
gates. The upstream-merge/repin gate is closed; these implementation boundaries,
not Thermite branch state, now prevent a release claim.
