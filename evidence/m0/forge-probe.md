# M0 Forge probe evidence

Date: 2026-07-31

Status: **standalone release gate accepted against pinned Thermite commit
`845d684f00e829491ee4c537818fba2689bcaefc` (`v0.0.2`).** This coordinated pin retains the
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
toolchain_evidence_sha256=f265b36bc42c39ee9ababee8ae85686bc9e0cb17e45c8432f5af79469c1065e9
receipt_sha256=760e3128faea0583164c809c7630fffc9ad910c09de4a159431be03df878bc5d
artifact_sha256=278c6835e311a2c3fa3bd84a3f5e7d3165e1b034b080449db5ece01b68f80cd3
no_std_consumer_sha256=0590a78a7f70eba210c3bbd504c1d4d0a9222942ed69cf9e0a11122924beaafd
incompatible_rustc_result_sha256=79f8245393e41fe0171c821af6e9e98a426fe420cb656e1c86bbe508a575d08d
report_sha256=7928f7889d5b73f79fb229d58f29efdea5bffef968603cf3431bec21ff24af05
```

The report states `release_eligible=true`. This accepts the standalone primitive
Forge artifact path. Rich-state composition is accepted by its separate
component gate; signed-manifest binding remains independent.
