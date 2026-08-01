# Forge L3 integration and verified composition

## 1. Purpose and status

This document defines how TMK consumes Thermite code without losing assurance
between proof, compilation, and final kernel composition.

The baseline is Thermite `v0.0.2` commit
`845d684f00e829491ee4c537818fba2689bcaefc`, which ships:

- `forge build --level l3 --target kernel --export ...`;
- exact-source Verus proof and code generation;
- strict reachable-closure and translation-validation gates;
- explicit primitive/unit Rust exports with ABI fingerprints;
- separate, bound identities for the ambient host compiler and the complete
  Verus-selected artifact-codegen rustc/sysroot/LLVM closure;
- `VerifiedBuildReceiptV1`; and
- `forge verify-build [--replay]`.

The standalone L3 rlib path and same-crate rich-state composition path are
implemented and independently exercised. The pinned public Thermite commit
deterministically generates the multi-field-enum composition bundle; three TMK
builds reproduce its combined source, receipt, and rlib, a clean build in a
second absolute source root reproduces those products and both final images, and
receipt replay succeeds. Nothing in this document relabels an ordinary Forge L1
build.

One M1 component uses a separately recorded candidate extension: Thermite
commit `1fb0a799071d35493815ba99b9ca26af9a22eb1c` from draft PR #109. That
extension explicitly imports and receipts the verified slice model for a
`--no-vstd` kernel composition and generates an erased `no_std` metadata/link
rlib. It is pinned under `[thermite_issue_108]` in `toolchain/lock.toml`; it does
not silently replace the M0 `v0.0.2` baseline. The raw BootInfo gate binds the
model/source/link identities and replays the complete receipt. Release requires
the PR to merge and a coordinated project-wide baseline repin.

## 2. Two different interfaces

TMK distinguishes two interfaces that must not be conflated.

| Interface | Purpose | Type surface | Binary boundary |
|---|---|---|---|
| L3 link export | independently link a proved Forge rlib | `u32`, `u64`, `usize`, `bool`, `()` in v1 | pinned Rust ABI |
| L3 composition export | let the verified platform shell call Thermite transitions | rich Thermite/Verus ADTs, references, bounded collections | none; same canonical Verus crate |

The shipped link-export path is appropriate for probes, small arithmetic/policy
functions, and standalone proof artifacts. It is not sufficient for a transition
such as:

```text
step(KernelState, KernelEvent) -> (KernelState, Vec<MachineAction>)
```

That transition must retain its structured types in one verifier-visible crate.
Serializing it through integers, raw pointers, byte slices, or an unproved layout
adapter is forbidden.

## 3. Shipped standalone L3 pipeline

For a primitive export, the release orchestrator runs:

```text
Thermite source bytes
        |
        v
parse + spec validation + effect checking
        |
        v
ArtifactPlanV1
  exports + complete closure + ABI + expected source digest
        |
        v
strict certificates and contract/exec/body/loop/wrapper TV
        |
        v
canonical Verus library source
        |
        v
verus --no-cheating --compile
        |
        v
validated atomic bundle + VerifiedBuildReceiptV1
```

The exact source hashed in the plan is the source read by the final Verus process.
Forge re-hashes it after the process returns. The L3 path MUST NOT call
`lower_l1`, insert `thermite_check!`, or compile a second reconstruction.

Every reachable gate is fail-closed. In particular, these outcomes publish no L3
bundle:

- parse, specification, or effect failure;
- unresolved, ambiguous, indirect, or cross-file call closure;
- open body or proof hole;
- reachable `#[slag]`, `#[boundary]`, `fx diverge`, or `fx panic`;
- certificate below L3, downgrade, timeout, counterexample, or reject;
- TV verdict other than `Faithful`, including `Skipped` and `Unverifiable`;
- forbidden Verus escape hatch;
- source mutation before or during proof/codegen;
- code-generation failure; or
- receipt, inventory, or staged-bundle validation failure.

## 4. Bundle consumption

A TMK build copies a Forge bundle into content-addressed build storage and treats
it as immutable. At minimum, the bundle contains:

- original Thermite input;
- canonical `ArtifactPlanV1`;
- exact generated Verus source;
- per-item certificates;
- complete translation-validation evidence;
- whole-crate Verus result;
- toolchain and dependency identities;
- compiled rlib and its link dependencies; and
- `VerifiedBuildReceiptV1`.

Development builds run `forge verify-build`. Release and reproducibility builds
also run `forge verify-build --replay`. The orchestrator records the receipt
binding SHA-256, every file digest, and the replay result before accepting any
export.

The receipt is a cryptographic binding, not an identity signature. TMK's signed
release manifest provides provenance and authenticity over the accepted binding
digest.

## 5. Export rules

### 5.1 Standalone link exports

Only explicitly selected functions are public. Transitive helpers remain private.
Every accepted export MUST match the receipt's:

- source name and semantic address;
- public Rust path;
- canonical signature;
- ownership mode;
- target triple, pointer width, and endianness;
- crate and toolchain identities; and
- ABI fingerprint.

An export with `req true` may expose the proved implementation. A nontrivial
executable precondition receives a proved total wrapper that returns
`Result<Return, ThermiteContractError>`. An unverified caller cannot invoke a
partial Verus function and pretend the erased precondition was checked.

TMK v1 does not claim a compiler-independent Rust ABI. A link consumer must use
the receipt-pinned toolchain and target. Stable `extern "C"` wrappers, if added,
must be generated inside the canonical Verus source and verified there.

### 5.2 Composition exports

A composition export is visible only inside the final canonical crate. It:

- uses `pub(crate)` or equivalent verifier-scoped visibility;
- may carry declared Thermite ADTs, references, tuples, and bounded collections;
- does not create an ELF-visible symbol unless separately selected as a link
  export;
- preserves requires/ensures and type invariants for direct Verus calls; and
- is listed with its full dependency and type closure in the composition plan.

The user-facing native kernel ABI in [Native ABI](07-native-abi.md) is unrelated
to this verifier-internal interface. Native ABI structs remain explicit
`repr(C)` word layouts in the platform shell and are decoded into verified
internal types before invoking the Thermite core.

## 6. Selected full-kernel composition design

TMK will use one exact-source Verus compilation for the Thermite core and direct
platform shell together. The required Forge composition API has this semantic
shape; its final CLI spelling may be chosen during implementation:

```text
Thermite units + selected composition exports
                |
                v
strict Forge plan/cert/TV gates
                |
                +---------------------------+
                | unchanged lowered module  |
                v                           |
direct Verus platform modules --------------+
                |
                v
CombinedArtifactPlanV1 + canonical combined source
                |
                v
one verus --no-cheating --compile invocation
                |
                v
VerifiedCompositionReceiptV1 + kernel-core rlib
```

The composition implementation MUST be owned by Forge or a small upstream crate
that uses Forge's typed AST/lowering API. A TMK text-splicing script is not an
acceptable compiler stage.

The combined invocation proves:

1. every Thermite transition retains its strict Forge certificates and TV
   coverage;
2. the exact lowered transition body appears unchanged in the combined source;
3. every direct-Verus caller establishes the transition precondition;
4. the shell consumes only actions permitted by the transition postcondition;
5. representation conversion preserves the abstract/concrete relation;
6. all reachable direct-Verus executable bodies verify without escape hatches;
   and
7. the same combined source is passed to code generation.

If Verus later supports independently verified dependency metadata with the same
end-to-end property, Forge may implement composition by consuming its exact rlib
and verifier metadata. That implementation must demonstrate downstream call
verification and exact dependency selection; ordinary rustc `--extern` linking is
not enough.

## 7. Combined plan and receipt

`CombinedArtifactPlanV1` or its equivalent MUST bind:

- each input Thermite source and normalized program digest;
- each source Forge receipt binding digest;
- selected composition and link exports;
- full executable, specification, type, helper, and wrapper closure;
- canonical lowered Thermite module digest;
- every direct-Verus source and dependency digest;
- combined-source digest and strict Verus arguments;
- target, crate type, panic strategy, features, and linker-relevant flags;
- Forge, Verus, Z3, rustc/LLVM, vstd, and lockfile identities;
- per-member assurance and minimum aggregate;
- compiled kernel-core artifact digest; and
- a canonical file inventory.

The aggregate headline is at most L3. A member with L4 clause evidence does not
upgrade executable correspondence above L3. Any reachable member below L3 blocks
publication.

The receipt validator reconstructs the closure and combined source independently,
checks all digests and policy fields, and optionally replays proof/codegen. The
bundle is published by atomic rename only after local validation succeeds.

## 8. Functional-core boundary

Thermite owns deterministic authority decisions and state evolution. Direct Verus
owns representation-bearing and machine-facing operations.

The preferred call shape is:

```text
normalize concrete event
        |
        v
Thermite transition over abstract KernelState
        |
        v
bounded MachineAction sequence
        |
        v
verified shell simulation lemmas
        |
        v
page tables / scheduler context / APIC / VT-d / capsules
```

The composition boundary MUST NOT expose raw physical pointers, mutable aliases,
or hardware registers to Thermite. The shell maps concrete objects to stable
indices and generation-tagged handles, proves the representation invariant, and
applies returned actions only while holding the required ownership token.

All action collections are bounded. Allocation failure and capacity exhaustion
are ordinary proved error results; they are not panic paths.

## 9. Final link rule

The standalone Forge rlib, combined kernel-core rlib, direct machine-model
objects, and proved capsule objects enter an allowlisted final-link plan. Each
input is named by digest and receipt. The post-link auditor rejects:

- an unreceipted object or archive member;
- a different archive with the same filename;
- duplicate or unexpected public symbols;
- unresolved symbols;
- unwinding support or dynamic dependencies;
- executable bytes outside declared sections; or
- a capsule whose linked bytes differ from its proved instance.

The final image manifest relates the Forge and composition binding digests to the
linked ELF/PE and boot-image digests. It does not claim that the linker or
rustc/LLVM were proved correct.

## 10. M0 acceptance program

M0 contains two probes.

### 10.1 Standalone probe

A primitive Thermite transition is built with the shipped L3 kernel target,
replay-validated, linked into a separate `no_std` consumer, and called. This pins
the already shipped Forge behavior in the TMK toolchain lock.

### 10.2 Rich-state composition probe

A Thermite `ProbeState` contains at least an owner, generation, and bounded state
field. A transition accepts a typed event and returns an updated state plus an
action. A direct-Verus shell:

- constructs a related concrete representation;
- calls the composition export using rich types;
- proves the returned action is authorized;
- applies it to a mock platform state; and
- is verified and compiled in the same exact-source invocation.

The resulting rlib is linked into the receipted higher-half M0 kernel ELF. The
signed manifest binds that ELF and its final-link receipt alongside the
independently verified UEFI debug/return image and its TCG/KVM observations. M1
provides the first loader that transfers control to the composed kernel; M0 does
not add an unverified PE-to-kernel handoff merely to collapse those two artifacts.

Negative tests independently inject:

- a bad Thermite operator, branch, return, and loop update;
- a missing or downgraded certificate;
- every TV non-pass verdict;
- post-plan Thermite-source and generated-source mutation;
- a direct-Verus `assume`, axiom, or executable `external_body`;
- an unproved rich-state adapter;
- a different Forge rlib at final link;
- receipt/file/ABI/toolchain tampering; and
- a post-Verus failure before publication.

Every case MUST fail at its named gate and leave no apparently successful bundle.
Two clean builds in different absolute paths MUST reproduce the receipt-normalized
evidence and final image bytes.

## 11. Thermite shakedown workflow

TMK is also a production-scale Thermite shakedown. A blocked kernel requirement is
classified before any workaround:

| Class | Response |
|---|---|
| Thermite expressiveness gap | add the smallest general language feature and its parser/spec/lowering/TV tests upstream |
| Forge assurance gap | open an upstream soundness issue with a minimal counterexample and fail-closed acceptance test |
| Forge usability gap | improve diagnostics or workflow without weakening contracts or gates |
| TMK design defect | revise this design and preserve the original security claim |
| Platform-model gap | extend the Verus model and capsule tests before using the operation |

No gap is hidden with `#[slag]`, `#[boundary]`, executable `external_body`,
`assume`, a raw FFI shim, or a manually edited generated file. An opaque foreign
type declaration may use the exact type-specification exception in
[Assurance and trust](02-assurance-and-trust.md), but it cannot carry executable
behavior. The upstream regression becomes part of Thermite's conformance suite,
and the TMK case remains as an integration test.

## 12. Readiness gates

| Gate | Current status | Required evidence |
|---|---|---|
| Standalone exact-source L3 artifact | accepted locally | pinned build, validation, replay, runtime/link, compiler-mismatch, and tamper tests pass |
| Primitive explicit exports and ABI fingerprint | accepted locally | independent consumer and nine bundle-tamper tests pass |
| Strict rejection of non-L3/TV non-pass cases | accepted locally | standalone and composition certificate/TV/source/visibility fault matrices pass |
| Actual codegen-rustc receipt binding | accepted locally and upstream | receipt-selected consumers link; mismatched host compiler is rejected |
| Rich-state same-crate composition | accepted and manifest-bound | three builds plus a second absolute source root reproduce receipt/source/rlib and final images; validation, replay, runtime, links, and eleven negatives pass |
| Verified bounded allocator and panic host | accepted and manifest-bound | real `Box`/`Vec`, exact-image primitives, low/higher-half links, and composition-selected `memcpy` pass |
| Exact-byte instruction capsules | accepted for M0 | component, UEFI, and composition-selected capsule bytes survive link exactly; extend per M1 operation |
| Native ABI IDL generator | accepted and manifest-bound | three-path generation/compilation/runtime and five negative cases pass; M2 adds verified decoders/fuzzing |
| Release manifest schema | accepted for M0 development | composition replay, final-link audit, PE/FAT/firmware evidence, three signatures, and seventeen negatives pass |
| Final receipted link/image | accepted and manifest-bound | higher-half allowlist receipt and separate reproducible OVMF-booted PE/FAT image are signed together |

Every M0 row is demonstrated locally. M1 may begin; the pinned tests remain
mandatory regressions even when the upstream component carries conformance tests.
