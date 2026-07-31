# M1 reference boot page tables

Status: **accepted M1 subcomponent** with direct Verus
`0.2026.05.24.ecee80a`. This checkpoint proves and executes one concrete
four-level x86_64 page-table image for the accepted M1 mapping fixture. It is a
reference construction and walker, not yet the general loader page-table
builder or a CR3 installation claim.

## Accepted implementation

`verus/platform/boot_page_tables.rs` is a `no_std` executable Verus crate. It
constructs thirteen contiguous `#[repr(C, align(4096))]` pages, each containing
512 real `u64` entries:

- a PML4 rooted at the registered physical address `0x0040_0000`;
- separate PDPT, PD, and PT pages for direct-RAM, heap, stack, and image paths;
- an RW/NX supervisor direct mapping from `0xffff800000100000` to `0x00100000`;
- an RW/NX supervisor heap mapping from `0xffffc00000000000` to `0x00300000`;
- one RW/NX supervisor stack page at `0xffffe00000000000`, with absent pages on
  both sides;
- two RX text pages, two R/NX rodata pages, and two RW/NX data pages beginning
  at `0xffffffff80000000`; and
- no low mapping, recursive mapping, user mapping, or huge-page entry.

Every entry not named by the fixture is proved zero. The executable walker
performs the real four-level array lookup, follows physical-address links,
rejects absent or huge intermediate entries, combines permissions across all
levels, and adds the page offset. Its contract proves the registered direct,
heap, stack, guard, text, rodata, data, low-guard, and recursive-address
observations. The executable observation covers all ten cases and returns
`1023`.

The crate imports `vstd::array::ArrayAdditionalSpecFns` solely for erased array
specification and quantifier support. Its executable artifact has no `vstd`
symbols. The audited unresolved set is exactly `memcpy`, `memset`, and the two
core panic paths. The first two are supplied by the already accepted M0 verified
platform primitives; their final link, plus the fail-stop panic path, remains a
later integration gate.

## Acceptance command

```text
cargo run -p xtask -- m1-page-tables
```

The command pins Verus, Rust 1.95, `ar`, and `nm`; rejects proof escape hatches;
proves and compiles the exact source three times; compares all three rlibs
byte-for-byte; inventories members and defined/undefined symbols; separately
compiles and runs three consumers; compares the consumer executables
byte-for-byte; and confirms at runtime that the object is exactly thirteen
contiguous 4-KiB-aligned pages with 21 present entries.

Four source mutations must fail proof without publishing an rlib:

- text mapped to the wrong physical page;
- executable data caused by clearing NX;
- an unexpected present PML4 entry; and
- a false observation postcondition.

An additional negative run proves that the explicit `vstd` array specification
dependency is required when verification is invoked with `--no-vstd`.

## Stable result

```text
M1_BOOT_PAGE_TABLES_OK
component_verified=true
release_eligible=false
source_sha256=802a5df7aba6d1cf527dd5b2fdf88d81e15b272d952a0a837fa2e1edbd024c18
consumer_source_sha256=77b1a6e2109914f93f93f523f4e41d62d503f43979a8144f223e593e9cfb22b5
model_artifact_sha256=0eb5c3c31731756d7d6ee63f3313ec2f9ec9b5442706b231846876328e88c7f5
consumer_sha256=442bb4e74c60a6eee0501791914d26eb81aa314d15a1703527525672770b6802
model_reproducibility_builds=3
consumer_reproducibility_builds=3
verus_verified=75
page_table_pages=13
present_entries=21
root_physical=0000000000400000
proof_library=vstd-array-spec-only
executable_undefined_symbols=core-panic,core-panic-bounds-check,memcpy,memset
runtime_marker=M1_BOOT_PAGE_TABLES_OK observation=1023 pages=13 present=21 aligned=4096
negative_cases=text-physical,data-execute,unexpected-present-entry,observation,vstd-proof-dependency
```

Generated proof artifacts and runtime logs remain under ignored
`build/m1-page-tables/`.

## Remaining boundary

This image intentionally represents the bounded six-region acceptance fixture;
it does not pretend that one mapped direct-RAM page is a complete machine map.
The loader still needs a verified bounded builder that consumes the accepted
firmware and address-plan outputs, assigns owned physical frames, writes or
copies each page to the registered physical addresses, covers all selected RAM
and MMIO, and connects cache attributes. The CR3/invalidation instruction
capsules, final freestanding link, installed-translation probes, and live QEMU
execution are also separate M1 gates.
