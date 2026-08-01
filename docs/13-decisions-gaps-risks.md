# Decisions, gaps, and risks

## 1. Closed architecture decisions

| ID | Decision | Rationale |
|---|---|---|
| D-001 | x86_64 QEMU `q35` with KVM and OVMF | requested target; practical repeatable platform |
| D-002 | UEFI loader, no runtime services after handoff | bounded boot interface and simpler kernel |
| D-003 | capability-native kernel ABI | least authority and small kernel surface |
| D-004 | POSIX in user-space servers and libc | preserves microkernel structure |
| D-005 | POSIX source, not Linux binary, compatibility | avoids importing Linux kernel ABI |
| D-006 | Thermite functional core plus verified Verus shell | keeps hardware effects out of the functional core |
| D-007 | no release slag/boundary/downgrade | user-required formal platform assurance |
| D-008 | verified instruction capsules, no hand assembly | exact treatment of privileged instructions |
| D-009 | user-space VirtIO block/network drivers | failure containment |
| D-010 | VT-d required | user drivers are not isolated without DMA remapping |
| D-011 | modern VirtIO PCI only | one auditable device transport |
| D-012 | single core first, structures SMP-ready | requested staging |
| D-013 | four-core SMP with initial global ticket lock | concurrency proof tractability |
| D-014 | eager FPU save/restore | avoids lazy-state leakage and complexity |
| D-015 | `IRETQ` common return initially | one checked return path |
| D-016 | TMFS journaled user-space filesystem | real persistence and POSIX semantics |
| D-017 | musl-based libc port | mature source-compatibility surface outside TCB |
| D-018 | static ELF first; dynamic linking later | reduces first compatibility and loader surface |
| D-019 | Forge Option A: compile the exact canonical Verus body | removes the independent L1 executable lowering from the L3 artifact claim |
| D-020 | rich Thermite state crosses into the platform shell through same-crate verified composition | avoids an unproved FFI/layout boundary |
| D-021 | permit an exact opaque `PanicInfo` type specification, never an executable `external_body` | Verus must name the foreign lang-item parameter type while still verifying the panic implementation |
| D-022 | one strict JSON IDL generates transient C11 and `repr(C)` Rust ABI definitions | prevents numeric/layout drift while keeping generated files out of the authored source set |
| D-023 | strict canonical JSON manifests use Ed25519 and a fail-closed release-policy validator | binds proof, tool, artifact, test, assumption, and image identities without allowing a development key to authorize release |
| D-024 | M0 signs the composed higher-half ELF and independently booted UEFI probe as separate artifacts; M1 implements their verified handoff | preserves the M0/M1 loader boundary and avoids temporary unverified PE-to-kernel glue |

No architecture-significant user choice remains open for implementation kickoff.
Numeric limits may be tuned only within the invariants and ABI rules documented
here.

## 2. Toolchain closure ledger

| ID | Status | Gap or boundary | Required closure |
|---|---|---|---|
| G-001 | closed upstream | Forge kernel rlib functions were private | explicit selected primitive/unit exports and ABI fingerprints in L3 receipts |
| G-002 | closed upstream | `forge build` was L1-only | atomic `--level l3` strict build mode |
| G-003 | closed upstream | build manifest lacked proof/source/artifact binding | `VerifiedBuildReceiptV1` plus validation and replay |
| G-004 | designed partition | Thermite has no raw machine/atomic/concurrency surface | direct verified Verus shell; do not add these effects to Thermite merely to reduce module count |
| G-005 | closed locally | exact-image Verus decoders now own the boot-arena allocator, sealing, `memcpy`, `memset`, and the complete `GlobalAlloc` shim shape/relocation plan; real hosted `Box`/`Vec` plus reproducible low and higher-half static links pass | preserve the byte/model/source pins and bind the accepted component into the final receipted link |
| G-006 | closed for M0 probe | no existing exact-byte x86 capsule system | M0 model/emitter/post-link auditor passes; extend the opcode model for each M1 operation |
| G-007 | closed upstream for L3 builds | general Forge TV commands can report unsupported bodies | strict L3 artifact mode rejects every reachable non-`Faithful` verdict |
| G-008 | closed upstream and locally | pinned Forge `v0.0.2` commit `845d684f` deterministically emits the rich multi-field-enum composition bundle; three local builds and a second absolute source root reproduce receipts, sources, rlibs, and final images, and replay succeeds | preserve the exact tool/source pins and replay regression |
| G-009 | accepted residual TCB | final rustc/LLVM correctness is trusted | record exact TCB; later add codegen validation if feasible |
| G-010 | closed upstream | L3 verification and kernel codegen used different lowerings | compile the same canonical Verus body with `--no-cheating --compile` |
| G-011 | closed locally | combined exact-source verification/codegen, runtime consumers, exact selected primitive, canonical final-link allowlist, independent manifest replay/audit, and signed binding pass | preserve the receipts, final-object allowlist, and seventeen-case manifest regression |
| G-012 | closed locally and upstream; #103 closed by merged PR #105 | L3 receipt previously recorded ambient rustc 1.96 although Verus emits an rlib with rustc 1.95 metadata | receipt binds rustc/sysroot/LLVM closure; selected consumer links and incompatible host rustc is rejected |
| G-013 | closed upstream and locally | the exact-source rich-state composition build reproduces and replays; the local build/validate/replay/runtime/link/11-negative matrix passes | preserve as a pinned shakedown regression |
| G-014 | closed locally on exact public candidate `1fb0a799`; Thermite #108 / draft PR #109 remain open | Forge now imports and binds the verified slice model plus an erased `no_std` link crate; the real BootInfo success theorem, three reproducible builds, replay, executable 1-positive/12-negative decoder, two freestanding links, three proof negatives, and three receipt/dependency tamper negatives pass | merge PR #109 and perform the coordinated project-wide main-branch repin before release; preserve the candidate regression meanwhile |
| G-015 | closed for the M1 initial-root capsule | direct Rust cannot express the privileged CR3 write | direct Verus machine model plus exact `0f22dfc3` byte registration, one-section relocation-free post-link audit, and explicit verified-caller obligations; retain hardware execution as a separate gate |
| G-016 | closed for descriptor memory images | x86 descriptor encodings and complete IDT initialization need executable/spec correspondence before privileged loads | direct `no_std` Verus constructors and decoders prove the seven-entry GDT, 104-byte TSS, packed pointers, and all 256 gates; retain register loads, entry stubs, and hardware delivery as separate exact-byte gates |
| G-017 | closed for the descriptor-install capsule | direct Rust cannot express descriptor-register loads, segment reload, or `LTR` | direct Verus machine model plus exact 38-byte registration, relocation-free single-section post-link audit, TSS busy-bit transition, and explicit quiescent-caller obligations; retain boot-time execution and exception delivery as separate gates |
| G-018 | closed for the exception-stub table | 256 IDT targets must normalize CPU-error and no-error frames without generated-assembly drift | direct Verus construction/decoding of a complete 4096-byte table, exhaustive runtime execution, exact post-link identity, and 256 decoded common-entry branches; retain the body at that bound address and live exception delivery as separate gates |
| G-019 | closed for the returning common-exception capsule | exact assembly must preserve every interrupted register, capture CR2 before dispatcher work, establish kernel GS/DF/stack calling state, and safely refine back to `IRETQ` | direct Verus model plus exact 105-byte registration, two user-path/zero kernel-path `SWAPGS` transitions, validated selectors/RFLAGS/canonical resume state, relocation-free post-link audit, and explicit caller/dispatcher obligations; retain dispatcher implementation, joined link, concrete stack ownership, and live delivery as separate gates |
| G-020 | closed for pure exception-dispatch policy | the common entry needs a total, non-vacuous classification of user/kernel faults, IPIs, device/reserved/spurious vectors, overflow, and fail-stop state before machine effects are connected | exact Thermite state/action contracts, same-crate direct-Verus composition, 64/64 mutation battery, 18-scenario runtime, seven negative gates, and a freestanding no-undefined-symbol link to the post-link-matched verified M0 `memcpy`; retain concrete frame/state bridge, lock/current-thread ownership, action execution, joined link, and live hardware delivery as separate gates |
| G-021 | closed for safe saved-frame decoding | the exact common-entry stack order must become a normalized policy event without unchecked offsets, ambiguous same-ring tails, or invalid return state | direct-Verus exact 21/23-word slice decoder, proved CR2/vector/error/CS/RFLAGS/RSP/SS offsets and validity, same-crate policy call, 12-scenario runtime, seven negative gates, reproducible freestanding link; retain raw RDI pointer ownership/slice construction, context snapshot, action execution, and joined hardware delivery as separate gates |
| G-022 | closed for the exact dispatcher-front capsule | the raw common-entry RDI value must be conditionally dereferenced without reading an absent same-ring tail and transported to a verified scalar seam without an unchecked Rust slice construction | direct Verus model plus exact 93-byte registration, conditional six-word kernel/eight-word user reads, six-scalar SysV packing, correctly aligned stack-neutral scalar tail transfer to the exact common continuation, frame/RBX preservation, three runtime/link reproductions, relocation-free single-section post-link audit, and thirteen negative gates; retain joined common-entry ownership, scalar decoder/context/action body, fail-stop split, and live delivery as separate gates |
| G-023 | closed for common-entry/dispatcher composition | the abstract common-entry stack contract must establish every concrete dispatcher-front memory and ABI obligation without assuming its RDI, tail readability, aligned call word, or continuation | direct Verus joined-stack theorem for both eight-byte entry alignments, exact lower/upper stack coverage, DF and RDI refinement, conditional user tail, non-overlap and exact continuation, three runtime/proof reproductions, two-section post-link identity for both accepted images, and eleven artifact/proof negatives; retain scalar decoder/context/action implementation, full stub image, and live delivery as separate gates |

No toolchain-closure ledger item blocks the completed M0 evidence. G-014 no
longer blocks the content-preserving raw-byte `BootInfo` composition: it is
locally closed against an exact public commit without an adapter. The open draft
PR and coordinated main-branch repin remain release-process work.
G-005 is a verified input to the accepted and manifest-bound G-011 final-link
receipt. G-006's M0 acceptance instance is closed, including the exact-byte UEFI
entry/return probe and real OVMF TCG/KVM boot. G-015 closes the initial CR3
capsule model and post-link identity but not its boot-time call site or hardware
execution. G-019 closes the exact returning common-entry sequence. G-020 closes
the pure dispatcher decision policy, but not the concrete body/ABI bridge,
machine-action execution, joined image, or hardware delivery; additional
privileged operations remain M1 proof work. G-021 closes the safe slice decoder
and policy invocation, but deliberately does not manufacture that slice from a
raw assembly pointer. G-022 closes the exact conditional raw-pointer loads and
scalar ABI under explicit machine-memory obligations. G-023 discharges those
caller obligations and post-link-composes both exact images; the scalar body,
context/action execution, full stub image, and hardware delivery remain open.
Closed-upstream rows remain pinned TMK regression tests; an
upstream capability is not treated as locally demonstrated until its local replay
and negative-test matrix pass.

G-005 was closed without an executable escape hatch. A direct Verus model
registers and decodes the exact allocation, seal, `memcpy`, and `memset`
instruction bytes, the arena state machine, and the complete Rust adapter
body/relocation plan. The build extracts the real rustc object, rejects any byte
or relocation drift, and
then runs hosted and freestanding consumers through the actual `GlobalAlloc`
ABI. The remaining trust is the already-declared rustc/LLVM codegen boundary,
not an assumed allocator contract.

## 3. Major risks

### R-001 — exact-byte x86 proof is larger than expected

Impact: M0/M1 blocked.

Mitigation:

- minimize opcode set and capsule count;
- use one common entry/return frame;
- prefer simple instructions and `IRETQ`;
- keep AP trampoline fixed and separately proved;
- reject compiler-generated privileged instructions; and
- grow the Verus machine model only with negative tests.

Fallback: none that preserves the requested assurance. The project stops for
review rather than trusting assembly by contract.

### R-002 — Thermite executable subset is too narrow for kernel state

Impact: excessive direct Verus code or poor generated runtime layout.

Mitigation:

- keep transitions small and value-oriented;
- use fixed-capacity indexes/arenas;
- add narrowly justified Thermite language/tooling features upstream;
- require translation validation for additions; and
- measure generated allocation and code size early.

Fallback: implement the affected module directly in verified Verus and preserve
the same abstract transition spec.

### R-003 — proof-to-binary gap

Impact: proof applies to source but wrong bytes ship.

Mitigation:

- shipped Forge exact-source L3 bundles and replayable receipts;
- M0 same-crate receipt for the Thermite/direct-Verus composition;
- deterministic code generation;
- final-object allowlist;
- post-link structural audit;
- exact capsule-byte checks; and
- clean-build reproducibility.

Residual: the standalone and rich-state paths are closed at their rlib,
receipted final-link, and signed-manifest boundaries. rustc/LLVM/linker remain
trusted until stronger translation validation is added.

### R-004 — POSIX cross-service atomicity

Impact: fork/exec/signals/descriptors diverge after service crash.

Mitigation:

- explicit prepare/commit protocols;
- generation-tagged reservations;
- idempotent request IDs;
- deterministic coordinator rules;
- fault injection at every phase; and
- no transparent success after uncertain side effects.

### R-005 — persistent filesystem complexity

Impact: data loss or M5 delay.

Mitigation:

- deliberately small format;
- metadata redo journal;
- two superblocks;
- flush-defined transactions;
- exhaustive crash-point model;
- read-only recovery mode; and
- no advanced features before core invariants.

### R-006 — user-space driver DMA escape

Impact: total isolation failure.

Mitigation:

- mandatory VT-d and interrupt remapping;
- `iommu_platform` VirtIO negotiation;
- one domain per driver;
- no identity mappings;
- bus mastering disabled during reset/revoke; and
- adversarial DMA tests.

### R-007 — global kernel lock performance

Impact: poor SMP syscall scaling.

Mitigation:

- bounded short transitions;
- fast-register IPC;
- shared-memory bulk transfer;
- measure lock hold and contention;
- four-core target only initially; and
- fine-grained refinement after correctness evidence.

The lock is not removed merely because a microbenchmark is slow.

### R-008 — service restart creates semantic ambiguity

Impact: duplicated writes or false success.

Mitigation:

- request IDs;
- bounded completion cache;
- journaled storage;
- service generations;
- explicit cancellation/error; and
- retry only operations proven idempotent.

### R-009 — POSIX scope expands without bound

Impact: no useful release.

Mitigation:

- P0/P1/P2 profiles;
- generated interface matrix;
- static linking first;
- acceptance-driven P1;
- explicit unsupported results; and
- no full-conformance claim before evidence.

### R-010 — solver scalability or vacuous proof

Impact: unverifiable modules or misleading assurance.

Mitigation:

- small transition functions;
- proof budgets and named timeout verdicts;
- mutation batteries;
- independent translation validation;
- proof-cheat scanner;
- no skipped reachable TV; and
- no contract weakening to obtain green output.

## 4. Requirements trace

| User requirement | Design locations |
|---|---|
| x86_64 QEMU/KVM | 00 §2, 03 |
| UEFI boot | 01 §4, 03 §3–4 |
| POSIX compatibility | 08, 09 |
| Rust/assembly platform formally verified in Verus | 02 §5–6, 11 §4–6 |
| useful shell/storage/network acceptance | 00 §3, 12 M4–M6 |
| SMP-ready, one core first | 06 §3, 12 M7 |
| four-core scheduling later | 06 §4–10, 12 M7 |
| design before implementation | implementation status in README; M0 gates in 12; Forge integration in 15 |

## 5. Design completion criteria

The design set is complete when:

- every component has an owner and boundary;
- the native ABI and POSIX placement are defined;
- platform topology and machine operations are fixed;
- safety claims and TCB are explicit;
- proof-to-binary composition is specified;
- single-core and four-core acceptance are testable;
- current tool gaps carry explicit open/closed status and acceptance evidence;
- failures and recovery outcomes are defined; and
- no requirement depends on an unstated assurance downgrade.

Implementation discoveries may amend the design through reviewed decision records.
They do not silently rewrite these constraints.
