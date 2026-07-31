# M0 rich-state composition evidence

Date: 2026-07-31

Status: **accepted as a replayable component gate against pinned Thermite commit
`4fa63cb1a6d707e501d99a1da57b5a53f8346efa`.** This exact public commit closes
the deterministic composition-replay defect reported as Thermite
[#104](https://github.com/dollspace-gay/Thermite/issues/104). It replaces
nondeterministic Verus-generated enum helpers with Forge-owned deterministic
item generation while retaining one exact combined source and one strict Verus
verification/code-generation invocation.

The Thermite fixture `thermite/core/composition_probe.th` defines a rich
`ProbeState`, typed `ProbeEvent`, multi-field `ProbeAction`, and
`composition_step`. The direct-Verus shell proves both an authorized `Store`
transition and a rejected transition with no state change. The rich transition
remains private to the canonical crate; it is not converted into an unchecked
FFI boundary.

The complete acceptance command is:

```text
cargo run -p xtask -- m0-composition
```

It performed:

- Forge source checking, audit, and an 11/11 mutation battery;
- three independent exact-source composition builds;
- independent receipt validation and `forge verify-build --replay`;
- byte-identical receipt, combined-source, and kernel rlib comparison;
- receipt, plan, source, translation-validation, visibility, dependency, and
  compiler-identity audits;
- a hosted execution of both `Store` and `Reject`, producing
  `M0_COMPOSITION_OK:store:reject:1`;
- rejection of an external consumer of the private rich transition;
- rejection of the incompatible ambient Rust 1.96 consumer;
- three reproducible low-address static links with no undefined symbols and the
  expected fail-stop timeout;
- three reproducible links at `0xffffffff80000000` with no undefined symbols;
- final-link garbage collection that retained only the selected proved
  `memcpy` primitive and discarded allocator, seal, and `memset`; and
- post-link extraction showing that the linked `memcpy` bytes exactly equal the
  registered Verus capsule.

The build also rejected eleven independent mutations:

```text
artifact-tamper
binding-tamper
certificate-l2
external-body
extra-file
host-rustc
post-plan-shell
private-export
rich-standalone-export
shell-tamper
tv-nonpass
```

Stable identities from the passing run:

```text
forge_revision=4fa63cb1a6d707e501d99a1da57b5a53f8346efa
forge_sha256=3fad9e2b328367ad0169b297ea03165664edc854f6a026fcb08bcfcb814f35d4
skill_sha256=92141afe423f30b495398e806589753fb4ad57c2d0d10f3ef0fcd417beb557dd
source_sha256=d53d61ecb2cc92b6a8bbe94cd35ccba628f663014d31f7e548f7e7d5a0494370
shell_sha256=eb5298f5d0aef48141bcf539873235adbfad4fb1279c3286a29682b7dd51d36d
combined_source_sha256=51c909f368c52b202ed74340d730b7e14d560a5a2d53db3a75135b04e48893f2
receipt_sha256=27ff22646b265e82e4eec170bc22ec1583e9a60604e69e9ebe971cfb380d858a
binding_sha256=c2ff30a35ffa69a60b3b5c73918d906f9614cc7c9d8fa4e88d6fa835ad174598
artifact_sha256=d5d5032af0a9e625a5663ca1ec5826cd181d00f7d8907ad26a6ed0f17c43d8f8
platform_primitive_object_sha256=a3884a20bfb8193e6cfdbf921eae60bac038406aebfe9184ad5039e0629ec50f
final_link_receipt_sha256=cf795dc2d092531bb2e20a7d87f48c4dd737fa025f566340f7f0a38b62ff3883
hosted_consumer_sha256=13d67123a98aa2889f614f82b507e771ab0eec96c31c1d57baf11bef5e5aaa2b
low_static_consumer_sha256=1c17a0a84a2c175b569dd7125c0df1fc7cbd77e7157581faefd245334b46c428
high_half_consumer_sha256=45203ad97f95930ee4d7638ee0604b2f70a81d84b194ea3f3911590f7536d5d6
report_sha256=47b1b3f2d3a8deb32c62a527f47ae72de76bd48769e2a6fb3285eae9a67a0699
```

The machine-readable `tmk.final-link-receipt.v1` additionally binds the exact
composition artifact and dependency archives, verified platform primitive
object and report, selected and discarded symbols, consumer and linker sources,
tool identities, runtime observations, and reproducible low/high output
digests. Its orchestrator-source digest is
`ef5056d2ad68d161a89de94b2b241f2d8e3795cf992e09ca56041c229561cd67`.

The smaller source-only regression remains available as
`m0-composition-source-check`; its current check and report digests are
`734e8a6118d8642c386e507a8c477e75412fa0df0242bfcbcd79c1ee1612109a`
and `b4baa0f01ba60ba531b6d86bed5328be41028b6a352b97085823b69c7415ba5e`.

This checkpoint intentionally reports `release_eligible=false`. The component
is accepted, but the signed M0 development manifest must still validate and bind
the composition receipt and final-link receipt before M0 closure.
