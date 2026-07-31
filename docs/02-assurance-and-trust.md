# Assurance and trust

## 1. Claim

The release claim is:

> Given the named hardware, firmware/hypervisor, toolchain, and machine-model
> assumptions, every accepted kernel transition preserves capability integrity,
> address-space isolation, kernel-memory isolation, scheduling well-formedness,
> and DMA isolation; the linked instruction capsules refine their declared x86
> state transitions.

This is a safety and functional-refinement claim. It is not a proof that QEMU,
KVM, OVMF, rustc/LLVM, Verus, Z3, or the physical CPU are correct. It is not a
general liveness, availability, or side-channel proof.

## 2. Refinement chain

```text
 Abstract kernel transition system K0
          |
          | invariant-preservation and operation refinement
          v
 Thermite executable core K1
          |
          | Forge strict L3 gates + exact-source proof/codegen
          v
 Verus representation K2
          |
          | verified shell refinement
          v
 Rust-level platform state K3
          |
          | capsule proof + post-link byte identity
          v
 Linked x86_64 image K4
```

Each arrow produces a machine-readable record. A release manifest contains the
digest of the input and output at every arrow.

## 3. Assurance classes

| Class | Permitted release evidence |
|---|---|
| Thermite core | Forge L3 or L4, `Proved`, end-to-end scope |
| Direct platform code | Verus verifies every executable body |
| Machine capsules | Verus refinement proof over exact decoded bytes |
| User-space critical protocol state | Thermite L3/L4 or direct Verus |
| Ordinary user service code | memory-safe Rust plus tests; not in kernel TCB |

Forge L2, L1, L0, `#[slag]`, `#[boundary]`, holes, `Timeout`, `Stuck`,
`KernelBudget`, `Counterexample`, `RealWitness`, or `CovenantRefuted` block a
release kernel. A boundary contract may appear in host-side development tools,
but never in the release kernel closure.

Direct Verus code MUST be rejected if its transitive source contains:

- `assume`;
- `admit`;
- axioms;
- `#[verifier::external_body]`;
- executable `#[verifier::external]` dependencies;
- unreviewed `unsafe impl`; or
- inline/global assembly outside the capsule generator.

The scanner is necessary but not sufficient; Verus results and dependency closure
are also inspected.

## 4. Functional core obligations

Let `S` be the abstract kernel state and `step(S, E) = (S', A)` be the transition
for normalized event `E`, returning actions `A`.

Every transition proves:

1. `well_formed(S) -> well_formed(S')`;
2. failure results do not change authority-bearing state;
3. every emitted action is authorized by the pre-state;
4. action operands refer to live objects with matching generations;
5. no thread is `Running` on more than one CPU;
6. no writable user mapping aliases a frame owned exclusively by another
   protection domain;
7. kernel and page-table frames are never user writable;
8. capability rights can only decrease under copy/mint and are not forgeable;
9. deleting or revoking a capability cannot create authority;
10. an IPC transfer moves or derives only capabilities authorized by the sender;
11. reply authority is single-use and bound to one blocked caller; and
12. DMA mappings are a subset of frames granted to the owning IOMMU domain.

Arithmetic uses checked fixed-width operations. Resource counts and queues have
explicit maxima. Exhaustion returns an error rather than overflowing or allocating
without bound.

## 5. Verified shell obligations

For each `MachineAction`, the shell proves a simulation lemma. Examples:

- `MapPte`: the concrete PTE write plus required invalidation establishes the
  abstract mapping and preserves all unrelated translations.
- `Switch`: saved/restored registers correspond to the selected thread contexts,
  and the outgoing thread is no longer running.
- `ProgramIrq`: the APIC/IOAPIC/MSI-X state routes only the authorized vector to
  the bound notification.
- `MapDma`: VT-d tables expose exactly the authorized frame interval with the
  requested permissions.
- `EnterUser`: ring, selectors, flags, CR3, stack, and instruction pointer satisfy
  the target thread’s runnable invariant.

The shell is written directly as Verus-compatible executable Rust. It is not
ordinary Rust wrapped in trusted specifications, and no generator may claim a
plain-Rust function is verified merely by emitting a Verus signature.

## 6. Machine instruction capsules

Some operations cannot be expressed as ordinary verified Rust: `mov cr3`,
`wrmsr`, `invlpg`, `iretq`, `syscall` entry, interrupt stubs, `fxsave/fxrstor`,
port I/O, and the AP real-mode trampoline.

Release policy is:

1. Define a small, versioned x86 instruction semantics in Verus for the exact
   opcode subset used.
2. Represent a capsule as bytes, relocations, clobbers, precondition, and
   postcondition.
3. Decode the bytes with a verified decoder or reject them.
4. Prove that interpreting every path from the capsule entry establishes its
   postcondition or the declared fault outcome.
5. Emit those proved bytes; do not accept hand-written replacements.
6. After linking, extract each capsule by symbol and relocation map and require
   byte-for-byte equality with the proved instance.
7. Reject unexpected executable sections or privileged opcodes outside registered
   capsules.

This keeps the assembly semantic bridge explicit. A capsule proof is not replaced
by a Verus `external_body` contract.

## 7. Concurrency proof structure

The single-core kernel proves sequential transitions first. SMP adds:

- a verified ticket lock protecting the global kernel state;
- a ghost ownership token held by exactly one CPU while mutating that state;
- atomic acquire/release ordering for the lock;
- per-CPU state owned by one CPU or transferred under the lock;
- a TLB shootdown protocol with epoch and acknowledgement invariants; and
- an AP startup state machine in which an AP cannot enter user mode before its
  per-CPU structures and proof token exist.

The initial global lock deliberately reduces the concurrent proof surface. User
code still runs simultaneously on four cores. Fine-grained kernel locking is a
future refinement with a new proof, not an undocumented optimization.

## 8. Artifact binding

The release orchestrator MUST record:

- Thermite source digests;
- Forge version and generated skill/reference digest;
- every Forge certificate and audit manifest;
- the `VerifiedBuildReceiptV1` binding digest and complete bundle inventory;
- the canonical Verus source and `ArtifactPlanV1` digests;
- independent contract, expression, body, loop, and export-guard translation-
  validation results;
- the exact Forge-produced rlib digest, whether or not the final composition
  build consumes that rlib directly;
- direct-Verus module digests and results;
- capsule specification, proof, pre-relocation bytes, relocations, and linked
  bytes;
- rustc, LLVM, linker, OVMF, QEMU, and target configuration identities;
- generated Rust source digest;
- each rlib/object/archive digest;
- linker script and final ELF/PE section maps;
- initial service bundle and filesystem image digests; and
- final boot-image digest.

An L3 certificate for one source revision MUST NOT be paired with an rlib from
another revision. `forge verify-build` MUST accept every consumed bundle, and a
release build MUST additionally pass `forge verify-build --replay` with the
receipt-pinned Forge, Verus, rustc/LLVM, Z3, target, and dependency identities.

## 9. Trusted and assumed components

| Component | Classification | Reason |
|---|---|---|
| TMK Thermite and Verus sources | proved subject to tools |
| Capsule decoder/semantics | proved source; model adequacy assumed |
| Forge, Thermite semantics, translation validators | trusted proof pipeline |
| Verus, Z3, Lean kernel where used | trusted proof tools |
| rustc, LLVM, linker, archive tools | trusted code generation |
| x86 architectural specification/model correspondence | assumption |
| QEMU/KVM and host kernel | platform assumption |
| OVMF before `ExitBootServices` | boot-environment assumption |
| CPU, memory, IOMMU implementation | hardware assumption |
| `init` policy | trusted for authority distribution, not kernel isolation |
| user servers/drivers | untrusted for kernel isolation; trusted for their data |

The manifest distinguishes “proof source,” “trusted tool,” and “environmental
assumption.” Calling all three “verified” is forbidden.

## 10. Forge L3 artifact integration

Thermite commit `ae79a0f59ce5c08b20db47d23047f1f0665d122f` closes the
standalone proof-to-executable gap through an explicit strict build:

```text
forge build core.th --level l3 --target kernel \
  --export transition_probe --out build/core.verified
forge verify-build build/core.verified
forge verify-build build/core.verified --replay
```

The L3 mode:

1. freezes the reachable closure, selected exports, ABI fingerprints, strict
   gate inventory, and expected canonical Verus source in `ArtifactPlanV1`;
2. rejects holes, slag, boundaries, divergence, panic effects, unresolved calls,
   non-L3 certificates, and any reachable `Skipped` or `Unverifiable` TV result;
3. invokes Verus once with `--no-cheating --compile` over the exact canonical
   source that passed verification;
4. publishes only after validating a staged, cryptographically bound
   `VerifiedBuildReceiptV1`; and
5. retains the old independent `lower_l1` build as an unmistakably L1-only path
   that TMK release tooling MUST NOT consume.

This satisfies executable correspondence for the Forge-produced rlib. L1 runtime
checks are not spliced into the L3 output; contracts, invariants, and internal
preconditions are proof obligations. A nontrivial executable precondition on a
public scalar export is handled by a proved total `Result` wrapper.

### 10.1 Remaining kernel-composition requirement

Forge v1 link exports admit primitive scalars and unit. TMK's real transition
boundary carries structured abstract state, machine actions, and bounded
collections. Flattening that state into an unchecked FFI representation would
reintroduce the assurance gap under another name.

Before M1, TMK MUST demonstrate one exact-source Verus composition that lets the
direct-Verus shell call a Thermite transition while preserving its rich verified
types. The selected design is a same-crate composition mode described in
[Forge L3 integration](15-forge-integration.md): Forge emits composition-visible
functions and combines the unchanged Thermite lowering with the platform shell,
then one strict Verus invocation verifies and compiles the whole crate. Until that
mode exists, the primitive-export rlib is sufficient only for the M0 smoke test.

The final release orchestrator MUST consume the Forge binding digest, independently
bind the combined Verus source and final image, and prove that no post-Forge shim,
wrapper, or reconstructed Thermite body entered the executable closure.
