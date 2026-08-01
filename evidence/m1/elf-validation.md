# M1 static-kernel ELF validation

Status: **accepted M1 subcomponent** against Thermite `v0.0.2` commit
`845d684f00e829491ee4c537818fba2689bcaefc`. This evidence does not close M1 or
claim a complete UEFI loader.

## Accepted implementation

`thermite/boot/elf_policy.th` defines a stateful ELF validation transition. The
accepted profile is intentionally narrower than general-purpose ELF:

- ELF64, little-endian, System V, x86-64 `ET_EXEC`;
- a fixed 64-byte ELF header and 56-byte program headers;
- 2–32 headers wholly contained in a file no larger than 1 GiB;
- an entry point in the linked TMK high-half image window;
- a separately approved image digest;
- readable, page-congruent, file-contained `PT_LOAD` segments;
- ascending non-overlapping virtual load ranges;
- no writable-executable segment;
- at least two load segments and executable coverage of the entry point;
- a non-executable zero-sized GNU stack declaration; and
- bounded read-only GNU RELRO metadata.

Dynamic, interpreter, executable-stack, unknown-metadata, overflowing, overlapping,
out-of-file, and entry-uncovered images are rejected. Physical-address fields are
required to equal linked virtual addresses in this first profile; the UEFI loader's
chosen physical placement and virtual-to-physical page plan are separate M1 proof
obligations.

`tests/m1/elf_policy_shell.rs` calls the Thermite transition from direct Verus in
the same exact source. `tests/m1/elf_policy_consumer.rs` calls the compiled public
observation from a separate crate and checks its result.

## Acceptance command

```text
cargo run -p xtask -- m1-elf
```

The command:

1. checks the installed Thermite skill against Forge;
2. runs `forge audit --json --meaning --metrics` and `forge battery`;
3. requires L3, end-to-end scope, no slag/boundary, and 64/64 killed mutants;
4. creates three independent kernel-target exact-source composition bundles;
5. requires byte-identical receipts, combined sources, and rlibs;
6. audits the canonical strict gates and local source/artifact digests;
7. validates the primary and reproduced receipts and replays the primary;
8. compiles a separate consumer with the receipt-selected rustc;
9. executes it and observes `M1_ELF_POLICY_OK observation=127`;
10. proves modeled rejection of bad digest, W+X, dynamic segments, executable
    stack, missing entry coverage, overlap, and file overrun;
11. rejects direct-Verus shells that lie about the observation or dynamic-segment
    result; and
12. rejects a changed receipt binding digest.

Failed composition builds publish no partial bundle.

## Stable result

```text
M1_ELF_OK
component_verified=true
release_eligible=false
receipt_validated=true
receipt_replayed=true
source_sha256=273ee3e637c0057cdb956a396f65de28117e5400e9df22c6bb3fc05b8964ec4a
shell_sha256=71729b9f4767cd59513db877f016b0942e014f9c961803b2f5db521b60f2b665
combined_source_sha256=1d62f7d25cb1b5a6c05d8f2c99554d2459cd8383fbdfc87ec913969bf3d5bdf0
receipt_sha256=478171bd2cd025d22a23c1cc6d199b90669481c4763ef142e9b774fd6f02ebf6
binding_sha256=e25961b163c287604fd3c0f47d558bca5f60548da7e9e7f8974b23018fca427d
artifact_sha256=d2837f7308461de778500b1f7d3c7d38430251338a5858a2773ef261368d1ac8
consumer_sha256=701ca3b5c110d2140dc6ce2b28f8d17786aa889f00ff886f5a88cf147ffe91e5
reproducibility_builds=3
runtime_marker=M1_ELF_POLICY_OK observation=127
```

Generated proof bundles and logs remain under ignored `build/m1-elf/`; this file
is the reviewed summary.

## Remaining boundary

This checkpoint validates scalar ELF header/program-header events. It does not
claim that untrusted file bytes have already been decoded into those events. The
same-source content-preserving ELF byte decoder has not yet been authored. The
needed no-`vstd` kernel slice model is now locally demonstrated by the separate
[BootInfo decoder](bootinfo-decoder.md) against merged Thermite `main` commit
`b8dc3947`; it no longer requires an unverified adapter. The loader, page
installation, service bundle, `ExitBootServices`, and QEMU/OVMF gates remain open.
