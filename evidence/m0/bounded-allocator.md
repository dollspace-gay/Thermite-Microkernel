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
verus_result_sha256=bb01e646bbfb9b7516789f5825088fceb608d49e8e89e3bda9b784ec3128612d
consumer_sha256=8670118e82e8963ad34ce1e7e375405dc310065b5d780504bf825b090386eeb0
runtime_marker=M0_ALLOCATOR_OK:8:11:16
```

Negative diagnostic digests:

```text
bad-update-result.txt  40d526f22358b2900a47ddeca6aebeb4ab36c8eeee04b4d3ee6d38d176202489
bad-assume-result.txt  604bec44c696568a7cd361b895de30745f5f24515d34a490def81ff2f64b2eff
```

The report digest was
`d6be243d09d5bd50a2a2e40722c35415b2613cce52501fd483bf0ae6cdbef09c`.
This is a verified component, not complete allocator integration: the byte/layout
adapter, memory permissions, `GlobalAlloc`, fail-stop panic host, composition
receipt, and final-image binding remain outstanding, so the report states
`release_eligible=false`.
