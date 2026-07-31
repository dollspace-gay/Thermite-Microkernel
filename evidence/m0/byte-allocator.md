# M0 byte/layout allocator evidence

`verus/platform/byte_allocator.rs` refines the fixed-unit allocation policy to
byte addresses and an explicit layout profile. It is direct Verus and `no_std`,
with no `unsafe`, `assume`, axiom, `external_body`, assembly, or vstd dependency.

The public operation is total. It accepts a well-formed non-null arena, nonzero
size, sufficient remaining capacity, and alignment 1, 2, 4, 8, 16, 32, 64, or
4096. Success is characterized exactly, returns a nonzero aligned address, moves
the cursor to the allocation end without overflow, stays within the arena, and
proves sequential successful allocations do not overlap. Every rejected request
leaves the cursor unchanged and returns a zeroed failure record.

Runtime alignment uses `cursor & (align - 1)` rather than division. Eight
bit-vector proofs establish that this is equal to `cursor % align` for every
allowed alignment. This preserves the mathematical contract while eliminating
the freestanding compiler's division-by-zero panic dependency.

The following command was run successfully:

```text
cargo run -p xtask -- m0-verus-byte-allocator
```

It performs source escape-hatch scanning, exact Verus/tool hashing, 18 proof
queries with `--no-cheating --compile`, three byte-identical builds in separate
paths, an undefined-symbol audit, and a separately linked runtime consumer. The
consumer executes aligned sequential allocations plus zero size, unsupported
alignment, exhaustion, corrupt state, and `usize::MAX` edge failures. Mutations
that drop padding, reject an exactly fitting allocation, or inject `assume(false)`
all fail their named gates.

Stable positive identities:

```text
source_sha256=2e0e0befb81bd5afc6727509dc0ed3d4f7ca3edbe8b200f02514173e8e79ae95
artifact_sha256=a177589091da931699844d6d7ede9b58b86582afd1e23d272c55a9baf435f04a
verus_result_sha256=a3c996fb5db7a10687ba90341870bffacb05ff8a20f4d017bcc02790a7eb07d8
consumer_sha256=3e311fb90616c7f607f297b77de004993f4ca6a2f3f5563e6f80e9c682d3a151
runtime_marker=M0_BYTE_ALLOCATOR_OK:200008:200020:200030
report_sha256=65294ee5cd4aa4874f96fbc19839f993b10ad2907f2f7185b6f950e4cd99b751
```

The artifact is also retained by the freestanding host link in `panic-host.md`.
It does not yet implement `core::alloc::GlobalAlloc`: pinned Verus accepts the
unsafe trait method bodies for verification but cannot inspect `Layout` or create
the arena raw pointer with provenance/permission evidence under the current
no-vstd/no-cheating gate. The remaining ABI bridge must verify directly or be an
exact-byte refined capsule. The report therefore states
`release_eligible=false`.
