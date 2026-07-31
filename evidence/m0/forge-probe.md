# M0 Forge probe evidence

Date: 2026-07-31

Status: **development feature demonstrated; release gate blocked by Thermite
issue [#103](https://github.com/dollspace-gay/Thermite/issues/103).**

The probe at `thermite/core/probe.th` was processed through:

1. generated-skill freshness checking;
2. `forge build --level l3 --target kernel`;
3. `forge verify-build`;
4. `forge verify-build --replay`;
5. a Rust host consumer linked to the emitted kernel rlib and executed; and
6. a separate `#![no_std]`, `#![no_main]` final link.

The executed function emitted:

```text
M0_FORGE_PROBE_OK:5aa512cb9889ff00
```

The successful development run used Verus's actual Rust 1.95 codegen toolchain:

```text
TMK_UNBOUND_CODEGEN_RUSTC=/home/doll/.rustup/toolchains/1.95.0-x86_64-unknown-linux-gnu/bin/rustc \
  cargo run -p xtask -- m0-forge-probe
```

Observed evidence digests:

```text
receipt_sha256=872053e80293bc091300bb7c8d04c2af60b3ae0270b5982ad0cbd667b6112af4
artifact_sha256=278c6835e311a2c3fa3bd84a3f5e7d3165e1b034b080449db5ece01b68f80cd3
no_std_consumer_sha256=0590a78a7f70eba210c3bbd504c1d4d0a9222942ed69cf9e0a11122924beaafd
```

This is not release evidence because the override is deliberately unbound. The
same consumer compiled with the receipt-recorded host Rust 1.96 fails with
`E0514`: the rlib contains Rust 1.95 metadata. Forge #103 must make the receipt
bind and select the actual Verus codegen compiler. After that fix, the strict
command without `TMK_UNBOUND_CODEGEN_RUSTC` must pass and produce
`release_eligible=true` before the M0 standalone probe is accepted.

