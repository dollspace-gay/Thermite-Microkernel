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
`902f29242c068190320c1e1e1f702fb933e0dda6`:

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
check.json   73aac3efa79a1e3ced98d8bf508bdb473563319145abb183fa10092c96b4e8e4
audit.txt    241d563c3e5739cfc235991aca6f11c07058049bd7c90c84e86d24960532b776
battery.txt  34b2527cb1a203579a1268a3648e59d4e3d430b36c596ccc03a0d595a7ab32c0
report.txt   c68759da700582f6d2abaa248bde6c3318a1e83eb31774adc9bd64bca6fe9fc2
```

This is not the M0 composition acceptance result. Forge main now supplies the
same-canonical-source shell inclusion, combined rlib, receipt, validator, and
replay command. The M0 multi-field-enum fixture builds and validates, but replay
does not reproduce its bound artifact digest at commit `57848f3e`; Thermite
[#104](https://github.com/dollspace-gay/Thermite/issues/104) is therefore
reopened. The generated source-check report states `release_eligible=false`.

## Post-upstream shakedown

The checked-in direct-Verus shell now proves the accepted `Store` action's four
field values, the Thermite/platform representation relation, state advancement,
and rejection without state change. Against Forge main commit `57848f3e`, the
following richer build succeeds and ordinary validation accepts its receipt:

```text
forge build thermite/core/composition_probe.th --level l3 \
  --compose-export composition_step \
  --compose-shell tests/m0/composition_shell.rs \
  --crate-name tmk_composition_probe --target kernel \
  --out build/m0-composition/reported-fixed.verified
forge verify-build build/m0-composition/reported-fixed.verified
```

That run bound combined source
`41f790eaf177516d3352d08052f8ad2f52ca1115a8063efe0c3dfc57f2f6d6e7`,
artifact `291aec88346d4ff3fcd833d21ed58da04aac75d0781305c8efd5022c9f24c708`,
and binding
`74c3c1a2dfe294bb9eab9dc24e2dba0c70be822228b77116d74965a55deca50b`.
A hosted consumer executed `boot_observation() == 1`, a separate consumer was
correctly refused access to the private rich transition, and a static consumer
linked with no undefined symbols once the verified TMK `memcpy` primitive was
included. The static consumer remained in its expected fail-stop loop.

`forge verify-build ... --replay` still reports `replayed: false`; therefore the
artifact and binding above are diagnostic identities, not accepted release
evidence. No composition receipt enters the manifest until the same bundle
replays and independent clean builds reproduce byte-for-byte.
