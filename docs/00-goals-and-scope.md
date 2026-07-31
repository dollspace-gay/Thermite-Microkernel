# Goals and scope

## 1. Mission

TMK is a useful, security-oriented microkernel operating system for x86_64 virtual
machines. It combines:

- a small verified kernel mechanism layer;
- capability-based authority;
- isolated user-space drivers and operating-system services;
- POSIX source compatibility;
- persistent storage and TCP/IP networking; and
- an evidence-producing build in which assurance claims are tied to exact source
  and binary artifacts.

“Microkernel” is structural, not branding. Filesystems, the network stack, process
policy, POSIX semantics, device drivers, and the shell live outside the kernel.
The kernel implements only execution, isolation, capabilities, IPC, scheduling,
interrupt delivery, time primitives, and memory-translation mechanisms.

## 2. Fixed platform profile

The first supported platform is:

| Property | Decision |
|---|---|
| ISA | x86_64, four-level IA-32e paging |
| Machine | QEMU `q35` |
| Accelerator | KVM for acceptance; TCG is permitted for deterministic CI |
| Firmware | OVMF, UEFI 2.11 interface |
| CPU count | one enabled CPU first; four enabled CPUs in the SMP milestone |
| Memory | 2 GiB reference configuration; 512 MiB minimum |
| Interrupts | local APIC, I/O APIC, MSI-X, interrupt remapping |
| DMA isolation | emulated Intel VT-d IOMMU, required for untrusted drivers |
| Block device | modern VirtIO block over PCI |
| Network device | modern VirtIO network over PCI |
| Entropy device | modern VirtIO RNG over PCI |
| Early console | emulated 16550A serial port |
| Persistent filesystem | TMFS, a user-space journaled filesystem |

The implementation MUST probe required CPU and platform features and fail closed
with a diagnostic when the profile is not met. It MUST NOT silently run an
untrusted DMA driver without IOMMU isolation.

## 3. Useful-release acceptance scenario

The single-core useful release is accepted only when one reproducible image can:

1. boot through OVMF and `ExitBootServices()` without firmware services afterward;
2. initialize paging, interrupts, a timer, and VT-d;
3. launch at least four mutually isolated user processes;
4. exchange capability-authenticated synchronous IPC;
5. terminate a process that faults without corrupting the kernel or peers;
6. launch a supervisor, process server, pager, VFS, filesystem, block driver,
   network driver, network stack, terminal service, and shell;
7. run `ls`, `cat`, `echo`, `ps`, `mkdir`, `rm`, and service-status commands;
8. write a file through the VirtIO block stack, call `fsync`, reboot, and read the
   same bytes back;
9. configure the VirtIO network interface and receive a valid ICMP echo reply
   from a test-owned peer attached to the QEMU network backend; and
10. crash and restart either VirtIO driver while the kernel and unrelated
    processes remain alive, with affected clients receiving a defined I/O error.

The four-core release repeats that scenario with four enabled vCPUs and adds the
SMP tests in [scheduling and SMP](06-scheduling-and-smp.md).

## 4. POSIX objective

TMK targets POSIX.1-2024 source compatibility for a documented profile. Programs
are recompiled against the TMK libc and headers. Initial releases do not promise:

- Linux binary compatibility;
- the Linux syscall ABI;
- every POSIX option group;
- unmodified dynamically linked Linux executables; or
- POSIX certification.

The compatibility profile grows in explicit stages. Unsupported interfaces MUST
return a documented error or fail at link time; they MUST NOT appear to work with
silently different semantics.

## 5. Verification objective

The isolation kernel and platform layer are proof-bearing deliverables.

- Kernel policy and state transitions SHOULD be expressed in Thermite wherever
  the current language can represent them.
- Forge MUST certify release Thermite at L3 or L4 and report end-to-end scope.
- Low-level executable Rust MUST be written in the Verus-supported subset and
  verified directly.
- Privileged instruction sequences MUST use verified machine capsules, not
  unverified inline or external assembly.
- A release manifest MUST bind proof results, generated source, linked sections,
  instruction-capsule bytes, tool versions, and the final image digest.

User-space servers are outside the kernel isolation TCB. Their memory-safe
implementation is required, and selected state machines receive Verus proofs, but
the first useful release does not require a whole-system functional-correctness
proof of the POSIX environment.

### 5.1 Thermite shakedown objective

TMK is intentionally a production-scale Thermite shakedown. Kernel requirements
that expose a language, lowering, proof, artifact, or workflow defect are reduced
to upstream conformance cases and fixed in Thermite/Forge. The kernel then consumes
the strengthened toolchain.

This secondary objective never weakens the primary kernel claim. TMK does not keep
moving by inserting slag, contract-only boundaries, unverified adapters, or manual
generated-code edits. The classification and upstream-fix workflow is normative in
[Forge L3 integration §11](15-forge-integration.md#11-thermite-shakedown-workflow).

## 6. Explicit non-goals for the first two releases

- Physical-hardware support beyond the specified QEMU platform.
- Graphics, USB, audio, suspend/resume, hotplug, or power management.
- A Linux compatibility layer.
- Nested virtualization.
- Kernel modules or runtime kernel code loading.
- Distributed capability transport.
- Hard real-time certification.
- Protection against malicious firmware, QEMU/KVM, the host kernel, physical
  attacks, or all speculative-execution side channels.
- Transparent overcommit or swapping.
- Live migration.

These exclusions limit claims, not architecture. Interfaces are versioned so a
later design can add implementations without weakening existing invariants.

## 7. Engineering quality bar

“Not a toy” means the design requires:

- bounds-checked parsing of every untrusted binary format;
- explicit resource accounting and denial-of-service limits;
- crash-consistent persistent storage;
- DMA isolation;
- process and driver restart;
- stable, versioned ABIs;
- negative and fault-injection tests;
- no release proof downgrade;
- reproducible artifacts; and
- an auditable list of every assumption and unverified component.

Performance matters after correctness. The design provides fast-register IPC,
shared-memory bulk transfer, MSI-X, per-core run queues, and a path away from the
initial global kernel lock, but no performance optimization may bypass a proof
invariant or artifact-binding gate.
