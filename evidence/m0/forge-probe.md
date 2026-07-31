# M0 Forge probe evidence

Date: 2026-07-31

Status: **standalone release gate accepted against pinned Thermite commit
`4fa63cb1a6d707e501d99a1da57b5a53f8346efa`.** This coordinated pin retains the
toolchain-binding fix from Thermite issue
[#103](https://github.com/dollspace-gay/Thermite/issues/103) and adds the
deterministic rich-state composition repair from issue #104.

The probe at `thermite/core/probe.th` was processed through:

1. generated-skill freshness checking;
2. a Forge L3 source check, audit, and mutation battery (4/4 killed);
3. `forge build --level l3 --target kernel`;
4. `forge verify-build`;
5. `forge verify-build --replay`;
6. independent validation that the receipt binds `evidence/toolchain.json`;
7. exact checks of the Verus-selected Rust 1.95 compiler and its target identity;
8. a host consumer selected from that evidence, linked, and executed;
9. a separate `#![no_std]`, `#![no_main]` final link; and
10. a negative link with the separately recorded ambient Rust 1.96 compiler,
    which failed with `E0514` and identified the rlib as Rust 1.95 output.

The strict command needs no compiler override:

```text
cargo run -p xtask -- m0-forge-probe
```

The executed function emitted:

```text
M0_FORGE_PROBE_OK:5aa512cb9889ff00
```

Observed evidence digests:

```text
consumer_rustc_sha256=bff349e72704ff70bc08a234a3847338e797065bbedde5e556808bc87b7bf7c6
toolchain_evidence_sha256=82cea2d3e619085812f3c786eb00d1c93cab6a5e8e9f2737e0bd7fff5011b868
receipt_sha256=aeaa9f7dbd64465e8121d1bc23080b5fb86602f5529433aa4662f25350dd846d
artifact_sha256=278c6835e311a2c3fa3bd84a3f5e7d3165e1b034b080449db5ece01b68f80cd3
no_std_consumer_sha256=0590a78a7f70eba210c3bbd504c1d4d0a9222942ed69cf9e0a11122924beaafd
incompatible_rustc_result_sha256=79f8245393e41fe0171c821af6e9e98a426fe420cb656e1c86bbe508a575d08d
report_sha256=aef10840ac1763834b1ff14ae7617b5fccf0116567696af08f3846b895578b3f
```

The report states `release_eligible=true`. This accepts the standalone primitive
Forge artifact path. Rich-state composition is accepted by its separate
component gate; signed-manifest binding remains independent.
