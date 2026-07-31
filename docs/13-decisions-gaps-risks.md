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
| G-005 | partially closed | fixed-unit and byte/layout policies plus fail-stop host link/run pass; pinned Verus cannot yet verify `Layout` inspection/raw-pointer construction in `GlobalAlloc` | verify the pointer ABI directly or as an exact-byte refined capsule, then exercise real `alloc` use |
| G-006 | closed for M0 probe | no existing exact-byte x86 capsule system | M0 model/emitter/post-link auditor passes; extend the opcode model for each M1 operation |
| G-007 | closed upstream for L3 builds | general Forge TV commands can report unsupported bodies | strict L3 artifact mode rejects every reachable non-`Faithful` verdict |
| G-008 | open | Forge link exports admit only primitive scalars/unit and do not expose rich verified kernel state | same-crate composition exports with rich types and complete closure |
| G-009 | accepted residual TCB | final rustc/LLVM correctness is trusted | record exact TCB; later add codegen validation if feasible |
| G-010 | closed upstream | L3 verification and kernel codegen used different lowerings | compile the same canonical Verus body with `--no-cheating --compile` |
| G-011 | open | standalone Forge receipt does not by itself prove a direct-Verus consumer's calls or final-image selection | combined exact-source verification/codegen receipt and receipted final-link allowlist |
| G-012 | closed locally at pinned commit `902f2924`; #103 pending merge | L3 receipt previously recorded ambient rustc 1.96 although Verus emits an rlib with rustc 1.95 metadata | receipt now binds rustc/sysroot/LLVM closure; selected consumer links and incompatible host rustc is rejected |
| G-013 | open upstream (#104) | Forge has no exact-source rich-state Thermite/direct-Verus composition build or receipt | implement and replay the M0 rich-state composition probe without a post-verification adapter |

G-005, G-008, G-011, and G-013 block M1. G-006's M0 acceptance instance
is closed; additional privileged operations are M1 proof work. Closed-upstream
rows remain pinned TMK regression tests; an upstream capability is not treated
as locally demonstrated until the M0 replay and negative-test matrix pass.

The G-005 boundary was reproduced with a minimal `GlobalAlloc` probe. Unsafe
trait methods themselves enter verification, but `core::alloc::Layout` is
unsupported without an external type/function specification and safe creation of
the arena raw pointer requires provenance/permission operations unavailable to
the trait signature under the current no-vstd/no-cheating gate. No executable
escape hatch was accepted.

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

Residual: the standalone Forge gap is closed at the rlib boundary; rich-state
composition and final-link selection remain M0 work. rustc/LLVM/linker remain
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
