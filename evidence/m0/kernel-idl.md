# M0 kernel IDL generator evidence

`abi/kernel.idl` is now the executable source for ABI v1 syscall numbers,
native status values, typed invocation operation numbers, message-tag and
capability-path bitfields, and the C-compatible `TmkSendCapV1` and
`TmkUtcbV1` layouts.

The generator in `xtask/src/idl.rs` accepts only schema
`tmk.kernel-idl.v1`. It rejects unknown keys, malformed identifiers, duplicate
or non-dense number spaces, bitfield gaps/overlaps, unsupported integer widths,
unknown or forward wire types, implicit padding, offset drift, and incorrect
size or alignment. Generated Rust uses `repr(C)` plus constant size, alignment,
and offset assertions. Generated C uses C11 `_Static_assert`, `_Alignof`, and
`offsetof`. Generated files are transient build products and are not hand
edited or treated as a second source of truth.

The following acceptance command was run twice successfully:

```text
cargo run -p xtask -- m0-idl
```

Each run generated Rust, C, and canonical JSON in three independent directories
and required byte-identical outputs. The generated Rust was compiled both into
an executable and into a `no_std` rlib with the pinned Verus codegen compiler.
The generated header was compiled with pinned GCC 15.2.1 under C11 with
`-Wall -Wextra -Werror`. The Rust and C executables then decoded the same tag and
capability path and observed the same 1024-byte UTCB layout at runtime.

The negative matrix changed a syscall number to a duplicate, moved the UTCB word
array one byte, overlapped message-tag fields, introduced an unknown `u128` wire
type, and changed a generated Rust file after generation. All five changes were
rejected by their named gates.

Stable positive identities:

```text
source_sha256=01f8b644c7e70042afe977400a04b44172ef010d4e2e181d71f6606f2f3b1414
generator_sha256=6d73a4ea90a812ec33973567377015cb7e137984a44cc85fa7f8043b1cf16c8a
rust_output_sha256=2a363f1b5888a9519a8cf2afacb95d865f1aaeef4283d435f0cc301d17d2bdbc
c_output_sha256=dc77a19ecb2f2ab6c9be9f6c278b35af4a4574647c15393a99cfc8799f22706b
canonical_output_sha256=94b23c368c24ac3033d236de45c5de60ecc94500bab9c49caf5cee598f9055bf
rust_consumer_sha256=0636657c7d958b43bb5b58a916d0d4384ba7c3385f61f8b05664a6a4c3f6b93d
nostd_consumer_sha256=5611929762e1a69b35b9d058f53725b8131a29bef7440da11ef267d81220cfba
c_consumer_sha256=33bc201c515bac04987cabb0acb6d96553bec9d349b5b34169618c72c052bd18
negative_results_sha256=091c137eaa9625d414a6ba120154d9a50b2096f1e2a4105306f5e71113bce2eb
rust_runtime_marker=M0_IDL_RUST_OK:1024:536:680:0001123400560204
c_runtime_marker=M0_IDL_C_OK:1024:536:680:0001123400560204
report_sha256=ccbb1772349d5e573c62c5ada5a29e1738e634f8a873250997561b538af4429b
```

The generator is a validated build tool, not a formally verified kernel
component, so the report states `release_eligible=false` while M0 remains open.
Thermite/Verus decoder proofs, fuzzing of every syscall decoder, failure
atomicity, and old-minor compatibility are later ABI implementation gates and
are not claimed by this checkpoint.
