# Thermite Microkernel

This repository contains the design for a capability-oriented, POSIX-compatible
microkernel written primarily in Thermite and verified through Forge and Verus.
Implementation has begun with M0 toolchain and proof-artifact closure; a
freestanding verified component host now links and executes, but bootable
kernel/BSP implementation has not started.

The first platform is x86_64 QEMU/KVM on the `q35` machine, booted as a UEFI
application through OVMF. The first useful release is single-core but is designed
for SMP from the beginning. The next release enables four-core execution after
memory isolation and IPC are stable.

The system is not a Linux clone and does not expose the Linux syscall ABI.
Applications obtain POSIX source compatibility through a libc port and user-space
servers over a small capability-native kernel ABI.

## Non-negotiable release rule

A release kernel may contain:

- Thermite functions certified L3 or L4 with end-to-end assurance.
- Verus-compatible Rust whose executable bodies verify.
- Small x86 instruction capsules whose exact linked bytes refine a Verus machine
  model.

A release kernel may not contain `#[slag]`, unresolved proof holes, Forge L0/L1/L2
downgrades, Verus `assume`, axioms, executable `external_body`, or unverified
hand-written assembly. The only permitted `external_body` syntax is paired with
`external_type_specification` to name an opaque foreign type whose representation
and operations are never used; it cannot suppress verification of executable
code. Firmware, the hypervisor, the proof tools, the compiler backend, and
hardware remain explicit environmental or trusted-tool assumptions; they are
never silently described as verified.

## Design map

1. [Goals and scope](docs/00-goals-and-scope.md)
2. [System architecture](docs/01-system-architecture.md)
3. [Assurance and trust](docs/02-assurance-and-trust.md)
4. [x86_64 platform](docs/03-x86-platform.md)
5. [Kernel objects, capabilities, and IPC](docs/04-kernel-core.md)
6. [Memory management](docs/05-memory-management.md)
7. [Scheduling and SMP](docs/06-scheduling-and-smp.md)
8. [Native ABI](docs/07-native-abi.md)
9. [POSIX personality](docs/08-posix-personality.md)
10. [User-space services and drivers](docs/09-userspace-services.md)
11. [Security and recovery](docs/10-security-and-recovery.md)
12. [Build, verification, and release](docs/11-build-and-release.md)
13. [Milestones and acceptance gates](docs/12-milestones-and-acceptance.md)
14. [Decisions, gaps, and risks](docs/13-decisions-gaps-risks.md)
15. [Normative references](docs/14-references.md)
16. [Forge L3 integration and verified composition](docs/15-forge-integration.md)

## Normative language

`MUST`, `MUST NOT`, `REQUIRED`, `SHALL`, `SHALL NOT`, `SHOULD`, `SHOULD NOT`, and
`MAY` are normative. A requirement is not implemented merely because it appears
in these documents. Implementation status is tracked separately from design
status.

The working project name in these documents is **TMK**. It is not a commitment to
a final product name.

## Implementation status

Implementation is in M0 toolchain closure. The public repository and pinned Cargo
workspace exist. The first Thermite L3 kernel rlib has been verified, replayed,
executed through a host consumer, and linked through a separate `no_std` consumer.

That probe is development-only pending Thermite
[#103](https://github.com/dollspace-gay/Thermite/issues/103): Forge currently
records host Rust 1.96 in the receipt although Verus emits Rust 1.95 rlib metadata.
The mismatch is fail-closed and M0 is not marked complete.

The rich-state acceptance transition is also implemented and passes Forge L3,
audit, and mutation-battery checks. Its honest same-crate Thermite/direct-Verus
composition build remains blocked on Thermite
[#104](https://github.com/dollspace-gay/Thermite/issues/104).

The first direct-Verus platform component is a total bounded bump-allocation
policy over fixed-size units. Its proof/codegen, three-path reproducibility,
separate runtime consumer, symbol audit, and two negative proof tests pass. A
verified fail-stop panic lang item and the allocator policy are now linked with
the registered capsule into a deterministic freestanding x86_64 ELF. The host
actually runs its fail-stop entry, has one read/execute load segment, and passes
negative divergence, executable-`external_body`, and writable-data tests. The
byte/layout adapter and real `GlobalAlloc` implementation remain M0 work.

The M0 x86 capsule is also live: Verus proves the exact encoding and machine-state
transition for `mov rax,rdi; hlt`; the emitted bytes survive object conversion and
static linking unchanged, with relocation, executable-section, symbol, and
disassembly audits plus four negative tests.

Useful implementation commands:

```text
cargo run -p xtask -- toolchain-check
cargo run -p xtask -- m0-forge-probe
cargo run -p xtask -- m0-forge-tamper
cargo run -p xtask -- m0-composition-source-check
cargo run -p xtask -- m0-verus-allocator
cargo run -p xtask -- m0-verus-capsule
cargo run -p xtask -- m0-host-link
```

The second command is the strict release gate. A temporary
`TMK_UNBOUND_CODEGEN_RUSTC=<rustc-1.95>` override exists only to reproduce issue
#103 and exercise the implementation; reports produced with it state
`release_eligible=false`.

Cargo build directories are cleaned after evidence is captured at each milestone
boundary. Proof bundles and runtime reports live under ignored `build/` paths;
reviewed summaries live under `evidence/`.
