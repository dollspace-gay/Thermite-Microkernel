# M0 rich-state composition source evidence

The pre-composition Thermite source fixture is
`thermite/core/composition_probe.th`, SHA-256
`d53d61ecb2cc92b6a8bbe94cd35ccba628f663014d31f7e548f7e7d5a0494370`.
It defines:

- `ProbeState` with owner, generation, and a state field bounded to four slots;
- a typed `ProbeEvent`;
- a `ProbeAction` carrying the authorized store parameters or an explicit
  rejection; and
- `composition_step`, whose postcondition pins both successful authorization and
  complete rejection behavior.

The following command was run against pinned Forge commit
`ae79a0f59ce5c08b20db47d23047f1f0665d122f`:

```text
cargo run -p xtask -- m0-composition-source-check
```

Observed results:

- all three ADTs and `composition_step` certified at L3;
- `composition_step` had end-to-end scope and discharged its proof obligation;
- the mutation battery killed 11 of 11 mutants;
- the audit reported project assurance L3; and
- a standalone L3 export attempt failed because the rich return type is outside
  the primitive v1 link ABI.

The complete ignored outputs had these SHA-256 digests:

```text
check.json   52bf624761e36b944ad4ce2e0fc3e3033ec8fb827c894f663674f7994e5917aa
audit.txt    241d563c3e5739cfc235991aca6f11c07058049bd7c90c84e86d24960532b776
battery.txt  34b2527cb1a203579a1268a3648e59d4e3d430b36c596ccc03a0d595a7ab32c0
report.txt   c68759da700582f6d2abaa248bde6c3318a1e83eb31774adc9bd64bca6fe9fc2
```

This is not the M0 composition acceptance result. Forge still lacks the required
same-canonical-source direct-Verus shell inclusion, combined rlib, versioned
composition receipt, validator, and replay path. That fail-closed missing
capability is tracked as Thermite
[#104](https://github.com/dollspace-gay/Thermite/issues/104), and the generated
report states `release_eligible=false`.
