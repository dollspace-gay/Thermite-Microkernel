# Build, verification, and release

## 1. Future source layout

Implementation should converge on:

```text
thermite/
  core/                 verified kernel state transitions
  protocols/            verified decoders and state machines
verus/
  platform/             page tables, entry dispatcher, APIC, VT-d
  machine-model/        x86 state and instruction semantics
  capsules/             proved instruction sequences and emitter
kernel-host/
  boot/                 UEFI application and BootInfo handoff
  link/                 no_std host, allocator, panic path, linker script
services/
  init procd pagerd vfsd tmfsd devmgr drivers netd termd logd libc sh
abi/
  kernel.idl services/
tools/
  release-orchestrator postlink-audit image-builder
tests/
  model qemu fuzz fault-injection conformance
docs/
```

Generated files live under the build directory and are never hand edited.

## 2. Design-time toolchain baseline

The current design baseline is:

- Thermite commit `902f29242c068190320c1e1e1f702fb933e0dda6`;
- canonical Thermite skill SHA-256
  `cd37b3e309696a1512f6eef167911a498876cc0a49c138d1357c84f07efa3e29`;
- Verus `0.2026.05.24.ecee80a`;
- host rustc `1.96.0`;
- Verus artifact-codegen rustc `1.95.0`, selected from Verus's authoritative
  `Toolchain:` identity and bound independently from the host compiler;
- GNU binutils `2.44-12.fc42` for capsule object/link/post-link inspection;
- QEMU `9.2.4`; and
- locally available OVMF x86_64 firmware.

These are evidence of feasibility, not permanent dependency selections. The
implementation lock file pins exact binaries and digests, and upgrades require a
full proof/reproducibility run.

## 3. Thermite gate

For every release `.th` unit or composed closure:

1. verify that the installed/generated skill matches the Forge toolchain;
2. run `forge audit --json --meaning --metrics` and `forge battery` as intent-
   review and contract-strength evidence;
3. run the atomic strict build:

   ```text
   forge build <unit.th> --level l3 --target kernel \
     --export <root> --out <unit.verified>
   ```

4. require the build to publish a `VerifiedBuildReceiptV1` with headline L3,
   `scope=end_to_end`, the exact mandatory gate set, and no reachable downgrade;
5. run `forge verify-build <unit.verified>` immediately after publication;
6. run `forge verify-build <unit.verified> --replay` for release and clean-build
   reproducibility jobs;
7. independently inspect the receipt's closure, exports, ABI fingerprints,
   certificate aggregate, TV inventory, toolchain, and file inventory; and
8. record the receipt binding digest rather than reconstructing a trust claim from
   separate command outputs.

The L3 build internally repeats parse/spec/effect validation, per-item proof,
strict contract/expression/body/loop/export-guard TV, whole-crate no-cheating
verification, exact-source code generation, and staged-bundle validation. A
separate successful `forge check` never upgrades an L1 artifact.

If Thermite cannot express or translation-validate a required construct, that
function is written directly in verified Verus. It is never pushed through
`#[slag]` or a contract-only boundary to preserve a label.

## 4. Direct Verus gate

Each platform/machine module runs the pinned Verus binary with:

- zero verification errors;
- pinned solver limits;
- no timeouts;
- no `assume`, `admit`, axiom, or executable `external_body`;
- no reachable ignored executable body;
- dependency closure recorded; and
- proof result bound to the exact source digest.

Intentional environmental operations are represented as explicit input state or
machine-capsule semantics, not trusted executable stubs.

An exact-source audit MAY allow one `external_body` annotation paired with
`external_type_specification` on an opaque foreign type declaration. This does
not exempt executable code: the declaration may expose no fields or methods, the
receiving functions verify normally, and a negative build MUST show that moving
the annotation to an executable function is rejected by `--no-cheating`.

## 5. Forge kernel build and composition gate

Forge's default/`--level l1` build still compiles the independent runtime-checked
`lower_l1` output. TMK tooling rejects that manifest and consumes only explicit
`--level l3` bundles.

The shipped standalone L3 path:

1. freezes `ArtifactPlanV1` from source, exports, closure, ABI, target, and expected
   Verus source;
2. requires every reachable certificate and TV obligation to pass the strict L3
   policy;
3. compiles the exact verified Verus body once with
   `verus --no-cheating --compile`;
4. emits a `no_std` rlib with `panic=abort` for `--target kernel`;
5. binds source, plan, certificates, TV evidence, toolchain, exports, dependencies,
   and artifact in `VerifiedBuildReceiptV1`; and
6. publishes the bundle atomically only after its own validator accepts it.

The L3 path deliberately contains no independently lowered L1 runtime checks.
Internal contracts are proved. A nontrivial executable precondition on a
standalone scalar export is enforced by a verified total wrapper returning
`Result`, not by an unproved panic path.

For the real kernel, the release orchestrator additionally requires the rich-state
same-crate composition in [Forge L3 integration](15-forge-integration.md). That
composition combines the unchanged Thermite lowering and direct-Verus platform
modules into one canonical source, verifies and compiles it once, and emits a
second bound receipt. An ordinary rustc consumer link proves ABI usability but not
the direct-Verus caller's preconditions or representation relation.

The `no_std` host supplies a verified bounded allocator and fail-stop panic
handler. The M0 component-link gate already combines the allocation policy, panic
lang item, and registered HLT capsule into a static x86_64 ELF, requires one RX
load segment, no relocations, no dynamic section, no undefined symbols, no
runtime data, exact linked capsule bytes, three proof builds, three links, and an
observed fail-stop execution. This is not yet the final kernel link: a verified
byte/layout adapter and `GlobalAlloc`, receipt allowlist, UEFI image, and manifest
binding remain required. The final linker accepts only objects named by accepted
Forge, composition, direct-Verus, and capsule receipts.

## 6. Capsule gate

For every capsule:

- verify its specification;
- verify byte decoder and semantic execution;
- prove all control-flow paths;
- emit fixed bytes and explicit relocations;
- archive a disassembly for review;
- link;
- re-extract the symbol and resolved bytes;
- repeat semantic proof for the relocated instance when addresses affect meaning;
- scan for privileged opcodes outside capsule sections; and
- bind linked bytes to the release manifest.

An unknown instruction, unexpected relocation, linker relaxation, byte mismatch,
or executable padding outside the policy blocks release.

## 7. Link and image gate

The linker script asserts:

- expected virtual and physical ranges;
- non-overlap and page alignment;
- RX/R/RW section permissions with W^X;
- capsule section containment;
- no writable relocation remains;
- bounded kernel image size;
- required exported symbols exactly once;
- no undefined symbol;
- no unwinding/runtime personality code;
- no dynamic loader dependency; and
- guard gaps around stacks.

The post-link auditor independently parses ELF program/section headers and checks
the same properties. It records final symbol/section maps and image digest.

The image builder creates:

- EFI system partition containing the loader and manifest;
- kernel ELF;
- initial service bundle/initramfs;
- TMFS partition;
- optional development symbols outside the boot image; and
- a copied writable OVMF variable store for tests.

## 8. Test layers

### 8.1 Pure/model

- Thermite/Verus unit proofs;
- state-machine property tests;
- capability and scheduler model checking;
- parser differential tests;
- TMFS crash model;
- sequential-vs-SMP transition comparison.

### 8.2 Host executable

- ABI layout tests;
- IDL encoder/decoder round trips;
- service protocol tests;
- filesystem/network unit tests;
- fuzz harnesses with sanitizers.

### 8.3 QEMU deterministic

Pinned TCG runs:

- boot milestones;
- syscall/IPC/memory/fault tests;
- storage/network acceptance;
- snapshot-based fault injection;
- deterministic interrupt schedules where possible; and
- TMFS power-cut matrix.

### 8.4 KVM acceptance

KVM runs:

- complete P1 acceptance;
- timing-sensitive preemption;
- VirtIO/MSI-X/VT-d paths;
- driver restart;
- load and stress tests; and
- four-core SMP acceptance.

### 8.5 Negative proof/tooling

The pipeline intentionally injects:

- wrong capability rights;
- missing TLB invalidation;
- swapped PTE bits;
- stale reply reuse;
- unbounded queue update;
- capsule byte mutation;
- `assume`/executable `external_body`;
- certificate downgrade and each TV `Skipped`/`Unverifiable` class;
- post-plan canonical-source mutation;
- receipt, ABI, toolchain, and artifact mismatch;
- substitution of a different Forge archive during composition/final link; and
- unauthorized executable section.

Each mutation must fail the expected gate.

## 9. Reproducibility

A release requires two clean builds in separate paths with:

- identical pinned inputs;
- no network after dependency fetch;
- `SOURCE_DATE_EPOCH` pinned;
- normalized archive/link metadata;
- identical generated sources;
- identical proof receipts except explicitly normalized timing fields; and
- identical final image bytes.

Any non-reproducible byte is explained and eliminated before release; “close
enough” is not accepted for a proof-bound artifact.

## 10. Release manifest

The signed manifest contains:

- release/version/configuration;
- platform and CPU feature profile;
- source repository revisions;
- proof and tool identities;
- every Forge and combined-composition binding digest;
- per-function assurance and scope;
- direct-Verus results;
- capsule results;
- TCB and environmental assumptions;
- ABI/service/filesystem format versions;
- every artifact digest;
- test-suite result digests;
- known limitations;
- development/production mode; and
- final image digest and signature.

## 11. CI failure policy

No flaky proof, timeout, skipped reachable TV, or intermittent QEMU test is
rerun-until-green. It is classified, reproduced, and fixed or the release remains
blocked. Increasing a proof budget requires recorded evidence that the obligation
is unchanged and not vacuous.
