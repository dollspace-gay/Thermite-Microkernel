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
  limit, `panic=abort`, disabled redundant overflow checks, and a normalized
  source path;
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
artifact_sha256=04f78bf2b8f5d9ec777a41cc6ed3f9fc77408ac8771155988aab592fd2473a03
verus_result_sha256=e2ad3f24aca4b1180a89464a0d22f2d00ac364d7ab94bbf7c8bdf09c50eb0285
consumer_sha256=8670118e82e8963ad34ce1e7e375405dc310065b5d780504bf825b090386eeb0
runtime_marker=M0_ALLOCATOR_OK:8:11:16
```

The negative results are retained as `bad-update-result.txt` and
`bad-assume-result.txt`; their raw diagnostic JSON object order is not used as a
reproducibility identity. The canonical positive Verus summary and report are
stable across reruns. The report digest was
`59b771b31a74dd18306ae55b5b1462829e248f1948b6418de18491f711694c36`.
This is a verified component, not complete allocator integration: the byte/layout
adapter, memory permissions, `GlobalAlloc`, fail-stop panic host, composition
receipt, and final-image binding remain outstanding, so the report states
`release_eligible=false`.
