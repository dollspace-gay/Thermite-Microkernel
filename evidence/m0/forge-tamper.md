# M0 Forge bundle tamper evidence

The following command rebuilt and validated a clean standalone L3 probe bundle,
then made and destroyed one isolated copied bundle per mutation:

```text
cargo run -p xtask -- m0-forge-tamper
```

Pinned Forge rejected all nine cases at the expected gate:

| Case | Expected diagnostic class | Evidence SHA-256 |
|---|---|---|
| raw Thermite source | bound-file length/digest | `ecea5247cf841b8a981ed4eb21f89eb62ad34f1298ee870511db6e4a88e8a4d7` |
| generated Verus source | bound-file length/digest | `b3b8c730a03bbb0ccb8ce20c7c10783a2598df4ba2dcdc4ae68c40b4522014f0` |
| certificate set | bound-file length/digest | `db722d396ce8be4ba5e95dc647e028338785d1575d16bae05fb41d0457895c71` |
| translation-validation evidence | bound-file length/digest | `3e78758b20581e3f8e6ac2878198029c539cac5e42716b3dac4893ee8749425b` |
| toolchain evidence | bound-file length/digest | `2ae6c8a9400f7aaa9d3a74036cd279997b40ded3e10ca91b33364d17a993e52d` |
| compiled rlib | bound-file length/digest | `ec96b12e18c2b86b4ca07056554bcbccb24bbd673743db24731655aeae29e6e9` |
| receipt syntax | invalid verified-build receipt | `4848cf230b3423818a5d9de5d7e7a502ff8ea8f3107a1016ec0d6582ca187542` |
| missing inventory member | non-canonical inventory | `7057098e639b28c6c478a651c269bc6f3d56895c0c9021598f52006d8914a34a` |
| unreceipted extra object | non-canonical inventory | `7057098e639b28c6c478a651c269bc6f3d56895c0c9021598f52006d8914a34a` |

The normalized report digest was
`b76d3ed678e7074b7d97537205841d980bdb6f1ab9662f3c3efcd10ab9772ee6`
and reported `rejected_cases=9`. Case directories were removed after each check,
so the matrix used bounded temporary disk space.

This covers bundle binding and inventory tampering. It does not yet complete the
separate pre-publication certificate downgrade, each TV non-pass verdict,
post-plan mutation, private-symbol, or wrong-valid-archive injections required by
the full M0 negative matrix.
