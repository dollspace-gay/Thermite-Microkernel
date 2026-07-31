# M0 Forge probe evidence

Date: 2026-07-31

Status: **standalone release gate accepted against pinned Thermite commit
`902f29242c068190320c1e1e1f702fb933e0dda6`.** Thermite issue
[#103](https://github.com/dollspace-gay/Thermite/issues/103) remains open pending
merge, but TMK pins and validates the immutable fix commit.

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
toolchain_evidence_sha256=aa0df25072d6ef0f8bef575acaab51fc5c7a386c5e4e594b8e9285db20ad5cda
receipt_sha256=361d608e49cdc0d4028e17a158985d3cfa73bc8cd55d96e579226a37c53db38c
artifact_sha256=278c6835e311a2c3fa3bd84a3f5e7d3165e1b034b080449db5ece01b68f80cd3
no_std_consumer_sha256=0590a78a7f70eba210c3bbd504c1d4d0a9222942ed69cf9e0a11122924beaafd
incompatible_rustc_result_sha256=79f8245393e41fe0171c821af6e9e98a426fe420cb656e1c86bbe508a575d08d
report_sha256=3c1555a9a1c78b2ffdb7d4fb7d7cf9135bc69c8913885d83c18627a4c75ef723
```

The report states `release_eligible=true`. This accepts the standalone primitive
Forge artifact path; it does not accept rich-state composition or the final
kernel image, which retain their independent M0 gates.
