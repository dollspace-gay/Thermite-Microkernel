# M0 rich-state composition evidence

Date: 2026-07-31

Status: **accepted as a replayable component gate against pinned Thermite commit
`845d684f00e829491ee4c537818fba2689bcaefc` (`v0.0.2`).** This merged public commit closes
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
- three independent exact-source composition builds plus a clean rebuild from a
  second absolute source root;
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
- byte-identical receipt, combined source, rlib, low image, higher-half image,
  and selected primitive across `/home/doll/...` and a fresh `/tmp/...` root;
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
forge_revision=845d684f00e829491ee4c537818fba2689bcaefc
forge_sha256=3fad9e2b328367ad0169b297ea03165664edc854f6a026fcb08bcfcb814f35d4
skill_sha256=92141afe423f30b495398e806589753fb4ad57c2d0d10f3ef0fcd417beb557dd
source_sha256=d53d61ecb2cc92b6a8bbe94cd35ccba628f663014d31f7e548f7e7d5a0494370
shell_sha256=eb5298f5d0aef48141bcf539873235adbfad4fb1279c3286a29682b7dd51d36d
combined_source_sha256=51c909f368c52b202ed74340d730b7e14d560a5a2d53db3a75135b04e48893f2
receipt_sha256=eed79d9db1437304172705ed9ce9f72857e9e691831e1b20c53cdf81e9e1f4da
binding_sha256=70e3aa411eb9c2944eb50d67613480852775475782043802c80f1ae8b2a1a9dd
artifact_sha256=d5d5032af0a9e625a5663ca1ec5826cd181d00f7d8907ad26a6ed0f17c43d8f8
platform_primitive_object_sha256=a3884a20bfb8193e6cfdbf921eae60bac038406aebfe9184ad5039e0629ec50f
final_link_receipt_sha256=3b5492f9129a403525d420580395ff06e248af43d0f7a30f210a79a9a5b200ea
hosted_consumer_sha256=13d67123a98aa2889f614f82b507e771ab0eec96c31c1d57baf11bef5e5aaa2b
low_static_consumer_sha256=1c17a0a84a2c175b569dd7125c0df1fc7cbd77e7157581faefd245334b46c428
high_half_consumer_sha256=45203ad97f95930ee4d7638ee0604b2f70a81d84b194ea3f3911590f7536d5d6
report_sha256=737c7e4ccddd76474e0e70f192300cfcbbc2a3c9225ca97095e347320ef0dbf6
```

The machine-readable `tmk.final-link-receipt.v1` additionally binds the exact
composition artifact and dependency archives, verified platform primitive
object and report, selected and discarded symbols, consumer and linker sources,
tool identities, runtime observations, and reproducible low/high output
digests and the two-root reproducibility result. Its orchestrator-source digest
is `9d256432c0ae3ccfe276899cfd18e17d22cf1ca5ac67c79ccd0a58938cf8a5f7`.

The smaller source-only regression remains available as
`m0-composition-source-check`; its current check and report digests are
`734e8a6118d8642c386e507a8c477e75412fa0df0242bfcbcd79c1ee1612109a`
and `b4baa0f01ba60ba531b6d86bed5328be41028b6a352b97085823b69c7415ba5e`.

This component report intentionally states `release_eligible=false`; release
authority belongs to the signed manifest. The clean M0 development manifest now
replays and binds this composition receipt, final-link receipt, higher-half ELF,
and exact selected primitive, closing the component's M0 integration gate.
