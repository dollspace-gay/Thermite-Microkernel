# M0 bounded allocator evidence

`verus/platform/bounded_allocator.rs` is a direct-Verus, `no_std` allocation
policy over a bounded arena of fixed-size units. It is total at the public
boundary: zero-size, exhausted, and internally invalid states return an explicit
failed allocation without arithmetic underflow or state mutation. Successful
allocations advance monotonically, remain in bounds, and sequential successful
allocations are proved non-overlapping.

The following command was run with Verus
`0.2026.05.24.ecee80a` and its Rust 1.95 toolchain:

```text
cargo run -p xtask -- m0-verus-allocator
```

The command:

- hashes the pinned Verus executable and canonical source;
- stages the exact source and checks it is unchanged after proof/codegen;
- invokes Verus with `--no-cheating --compile`, a pinned solver seed and resource
  limit, `panic=abort`, disabled redundant overflow checks, static relocation,
  disabled red-zone use, and a normalized source path;
- requires two verified functions and zero errors;
- reproduces the rlib in two additional absolute paths and requires all three
  artifacts to be byte-identical;
- rejects any undefined rlib symbol;
- links and runs a separate Rust consumer covering two adjacent successful
  allocations, exhaustion, zero-size rejection, and corrupt-state rejection;
- proves a wrong successful-state update fails its postcondition; and
- proves an injected `assume(false)` is rejected by `--no-cheating`.

Observed positive evidence:

```text
source_sha256=f3360c4f6ca68269b958f4115b99265a2572e5be8b966590cdc283d5d3136ea8
artifact_sha256=360717170e53f505ade8e81a120277cd4322d63cb1c08f9e6a158724fc9ab77d
verus_result_sha256=e2ad3f24aca4b1180a89464a0d22f2d00ac364d7ab94bbf7c8bdf09c50eb0285
consumer_sha256=83a335062e6669a2f07248695fe7bccf952598dd2c05016704f3d0a424611140
runtime_marker=M0_ALLOCATOR_OK:8:11:16
```

The negative results are retained as `bad-update-result.txt` and
`bad-assume-result.txt`; their raw diagnostic JSON object order is not used as a
reproducibility identity. The canonical positive Verus summary and report are
stable across reruns. The report digest was
`da0f457d8ec6c1898979a72c4fd8898be3226ce51137d688203557a00516b1e6`.
This policy is also present in the verified freestanding host recorded in
`panic-host.md`. It is not complete allocator integration: the byte/layout
adapter, memory permissions, `GlobalAlloc`, composition receipt, and final-image
binding remain outstanding, so the report states `release_eligible=false`.
