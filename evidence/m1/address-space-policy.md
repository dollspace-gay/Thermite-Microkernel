# M1 kernel address-space policy

Status: **accepted M1 subcomponent** against Thermite `v0.0.2` commit
`845d684f00e829491ee4c537818fba2689bcaefc`. This proves the scalar mapping-plan
policy and its executable observation; it does not yet claim page-table memory,
CR3 installation, or an M1 boot.

## Accepted implementation

`thermite/platform/address_space_policy.th` defines an L3/end-to-end transition
for a bounded, globally ordered kernel mapping plan. The accepted profile:

- requires 4–64 regions and a page-aligned kernel physical image of at most
  1 GiB below the 52-bit physical limit;
- fixes the linked image at `0xffffffff80000000` and requires an exactly matching
  physical/virtual image span;
- requires the low guard to be unmapped and forbids a recursive page-table map;
- separates the direct-RAM, heap/MMIO, per-CPU/fixmap, and kernel-image windows
  and proves the direct-RAM virtual address is the fixed base plus its physical
  address;
- requires mapped ranges to be page aligned, nonempty, bounded, and globally
  non-overlapping by virtual address;
- rejects every writable-executable mapping;
- excludes all direct, heap/MMIO, and stack aliases of kernel-image physical
  pages, so image W^X is physical-alias aware;
- requires guarded RW/NX per-CPU stack mappings; and
- requires contiguous RX text, R/NX rodata, and RW/NX data mappings that cover
  the kernel physical image exactly and in order before completion.

The direct-Verus shell accepts and completes a six-region plan containing one
region of every kind. A separately compiled Rust consumer calls the emitted
observation and checks the result.

## Acceptance command

```text
cargo run -p xtask -- m1-address
```

The command checks the installed Thermite skill; audits and batteries the
transition; requires L3/end-to-end scope, no slag or boundary, and 64/64 killed
mutants; builds three exact-source kernel composition bundles; compares receipts,
combined sources, and rlibs byte-for-byte; validates all three and replays the
primary; executes a consumer compiled with the receipt-selected Rust compiler;
and checks these rejection cases:

- a writable direct-map alias of kernel text;
- an incorrect direct-map virtual/physical offset;
- writable-executable image flags;
- an unguarded per-CPU stack;
- rodata before text;
- a non-page-aligned length;
- a mapped low guard;
- a physical gap between image segments;
- a virtual gap between image segments;
- virtual overlap;
- a plan that reaches its declared count without data or full image coverage;
- a false observation postcondition;
- a false alias-acceptance claim; and
- a changed receipt binding digest.

False proof shells publish no partial bundle.

## Stable result

```text
M1_ADDRESS_SPACE_OK
component_verified=true
release_eligible=false
receipt_validated=true
receipt_replayed=true
source_sha256=6170190c07717e843dcfebd14fa2872bccdd758c48b2c67c0989646b26c8bb5c
shell_sha256=c8b7914208cdbb2c71e59cefe03de56af27999f7a14f048d5698c5d8e89aee7f
combined_source_sha256=5a1a1cf41b71e8a3ddd13e35031bf50eacbf545de88b8cb7698d6b38f4f0b6e5
receipt_sha256=1c11ecf86d155fa5a6dd400a6474320d0a509f6cfcf8a3ecd14baa8ad56a3e53
binding_sha256=8a719f9243138ecdf3c1f8046c14130863d04da75316d146709eb381ce4ec723
artifact_sha256=976585d2a8c8add516a57df4a8ae7cc3bbc7f93f7d2fe196af2940fa4af660f6
consumer_sha256=f6e7de5cfa3967737b414032ab3793c7c32e6722e3bdee3b148afa632717c1ae
reproducibility_builds=3
runtime_marker=M1_ADDRESS_SPACE_POLICY_OK observation=511
```

Generated proof bundles and logs remain under ignored `build/m1-address/`.

## Remaining boundary

The verified shell supplies scalar mapping observations to the policy. It does
not allocate, zero, populate, or walk real x86 page-table pages; prove page-table
memory ownership; emit the CR3/INVLPG/INVPCID capsules; install a root; or inspect
the resulting translations from a running VM. Those are separate verified
machine-state and post-link gates. Physical map selection and cache-attribute
derivation must also be connected to the accepted firmware policy. This
checkpoint therefore closes the address-plan policy, not the M1 page-installation
deliverable.
