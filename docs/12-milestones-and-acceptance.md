# Milestones and acceptance gates

## 1. Sequencing rule

Each milestone is a vertical slice with proof and negative tests. Later work may
begin experimentally, but no milestone is declared complete until all prior exit
gates remain green.

## 2. M0 — toolchain closure

Pinned Forge already provides the following upstream capability, which M0 must
reproduce locally rather than redesign:

- explicit primitive/unit kernel-module exports with ABI fingerprints;
- atomic `forge build --level l3 --target kernel`;
- exact-source `verus --no-cheating --compile` correspondence;
- strict refusal below L3 or on any reachable TV non-pass result;
- cryptographically bound `VerifiedBuildReceiptV1` bundles; and
- validation, replay, tamper, mutation, visibility, and separate `no_std` link
  tests.

TMK delivers:

- repository/toolchain lock;
- a locally replayed standalone Forge L3 bundle and primitive export probe;
- Forge rich-state composition exports and one exact-source combined
  Thermite/direct-Verus build as specified in
  [Forge L3 integration](15-forge-integration.md);
- `VerifiedCompositionReceiptV1` or equivalent binding the combined source and
  kernel-core rlib;
- verified bounded allocator integration;
- direct-Verus `no_std` host;
- x86 machine model, capsule format, emitter, and post-link checker;
- IDL generator for kernel ABI;
- release manifest schema; and
- reproducible empty UEFI image build.

The kernel IDL generator deliverable is locally demonstrated: strict schema and
layout validation, byte-identical three-path C/Rust generation, hosted C/Rust and
freestanding Rust compilation, cross-language runtime agreement, and five
negative cases pass. See `evidence/m0/kernel-idl.md`. This does not close M0 while
the composition, final manifest/link, and raw-pointer allocator bridge remain
open.

The release manifest schema deliverable is also locally demonstrated. A clean
development run binds and replays current M0 artifacts, signs the canonical
payload reproducibly with Ed25519, verifies it, and rejects thirteen structural,
provenance, release-policy, schema, and cryptographic mutations. The committed
test key is forbidden for release eligibility. See
`evidence/m0/release-manifest.md`. The PE loader, FAT image, entry proof, pinned
firmware tools, and TCG/KVM observations are bound. Final manifest closure still
requires the composition receipt and receipted final-link allowlist.

The reproducible empty UEFI image is locally demonstrated by
`cargo run -p xtask -- m0-uefi`. A no-cheating Verus proof generates the exact
entry bytes; three model rlibs, PE32+ applications, and FAT16 disk images reproduce
byte-for-byte; independent parsers bind `EFI/BOOT/BOOTX64.EFI` to those bytes; and
OVMF emits the exact success marker under both TCG and KVM. Eight structural,
proof, and real-firmware negative cases pass. See `evidence/m0/uefi-image.md`.
The signed development manifest binds this checkpoint; rich-state composition
and final-link selection remain separate M0 gates.

Exit:

- a primitive Thermite probe is L3-built, receipt-validated, replayed, and linked
  from a separate `no_std` consumer using the pinned Forge bundle;
- a rich-state Thermite transition is called from direct Verus in one canonical
  exact-source verification/codegen invocation and bound to the kernel-core rlib;
- both Forge and composition binding digests are consumed by the final image
  manifest, with no post-verification wrapper or reconstructed body;
- a proved `HLT`/register capsule survives link byte-identically;
- injected private-symbol, source/cert mismatch, TV non-pass, post-plan mutation,
  rich-state adapter, executable `external_body`, wrong-archive, and capsule-byte
  changes all block the build; and
- two clean builds are byte-identical.

No kernel implementation begins until both the standalone artifact path and the
rich-state composition path demonstrate honest composition.

## 3. M1 — verified UEFI and BSP bring-up

Deliver:

- UEFI loader and `BootInfo`;
- ELF/service-bundle validation;
- memory-map normalization and `ExitBootServices`;
- kernel virtual layout;
- GDT/TSS/IDT and exception entry;
- serial crash/log path;
- local APIC and TSC-deadline timer; and
- fail-stop panic path.

Exit:

- boot through OVMF under TCG and KVM;
- reject malformed maps/ELF/ACPI;
- survive expected user-mode test exceptions once ring 3 is entered;
- prove and post-link-check all entry/exit capsules; and
- emit a manifest-bound serial `M1_OK`.

## 4. M2 — memory and capability foundation

Deliver:

- physical allocators and object arena;
- untyped retyping;
- CNodes and derivation/revocation;
- VSpaces and page-table actions;
- user copy;
- VT-d initialization and domains;
- frame/DMA mapping;
- one minimal root task.

Exit:

- W^X, kernel/user separation, zero-before-reuse, mapping correspondence, and DMA
  subset proofs;
- malicious root-child tests cannot map kernel/peer memory;
- stale generations and rights escalation fail;
- revocation is bounded/restartable;
- VT-d fault injection quarantines a test device; and
- missing IOMMU refuses untrusted-driver mode.

## 5. M3 — IPC, faults, scheduling, and isolation

Deliver:

- threads and user contexts;
- priority scheduler and timer preemption;
- endpoints, calls, replies, notifications, capability transfer;
- priority donation;
- pager fault protocol; and
- root supervisor skeleton.

Exit:

- four isolated test processes;
- one million call/reply operations with no mismatch;
- endpoint/capability-transfer atomicity proofs;
- peer fault containment;
- timeout/cancellation/reply-destruction tests;
- bounded donation and cycle rejection; and
- complete single-core scheduler invariant suite.

Memory isolation and IPC are considered stable only after M2 and M3 stress/fault
tests run continuously without invariant failure.

## 6. M4 — POSIX bring-up

Deliver:

- service directory and `init`;
- P0 libc;
- `procd`, `pagerd`, `vfsd`, `termd`, `logd`;
- static ELF execution;
- fork/COW/exec/wait;
- descriptors, pipes, signals, and shell job control;
- initramfs; and
- shell plus basic utilities.

Exit:

- shell launches pipelines and redirected processes;
- fork COW preserves isolation and file-description sharing;
- failed exec leaves old image intact;
- signal frames reject privilege changes;
- service-generation restart wakes clients explicitly; and
- advertised P0/P1 interface matrix matches implementation.

## 7. M5 — persistent storage

Deliver:

- PCI/`devmgr`;
- modern VirtIO block driver with VT-d and MSI-X;
- block broker;
- TMFS format, journal, VFS adapter, and recovery;
- `fsync`, atomic rename, unlink-while-open; and
- block-driver restart.

Exit:

- write/fsync/reboot/read acceptance;
- power cut after every transaction write preserves last committed state;
- malformed filesystem images fail safely;
- driver crash/restart does not crash kernel or unrelated processes;
- unresolved writes report explicit errors; and
- TMFS invariant and recovery proofs pass.

## 8. M6 — networking and single-core useful release

Deliver:

- modern VirtIO network driver with VT-d and MSI-X;
- Ethernet/ARP/IPv4/ICMP/UDP/TCP `netd`;
- P1 socket API and `poll`;
- network configuration and `ping`;
- VirtIO RNG/entropy service; and
- production/development boot modes.

Exit:

- every P1 interface advertised in the compatibility matrix passes its semantic
  and required-error tests;
- complete useful-release scenario in
  [goals and scope §3](00-goals-and-scope.md#3-useful-release-acceptance-scenario);
- TCP loopback and host-network tests;
- malformed packet corpus;
- network-driver crash/restart with explicit socket failures;
- entropy initialization and fail-closed tests;
- full proof/artifact/reproducibility gates; and
- signed single-core release manifest.

M6 is the first non-toy release.

## 9. M7 — four-core SMP release

Deliver:

- AP startup;
- verified global ticket lock;
- per-CPU scheduler state;
- deterministic placement/migration;
- reschedule and TLB IPIs;
- multi-CPU VSpace epochs;
- SMP priority donation; and
- four-vCPU release profile.

Exit:

- all tests in [SMP acceptance](06-scheduling-and-smp.md#10-smp-acceptance);
- complete M6 scenario with four enabled vCPUs;
- sequential-reference differential checking under randomized schedules;
- no data race in verified ownership model;
- bounded kernel transition/lock-hold measurements; and
- signed SMP release manifest.

## 10. P2 and later

After M7:

- pthreads and extended POSIX profile;
- dynamic linking;
- UNIX-domain sockets;
- IPv6;
- fine-grained kernel locking if justified;
- KPTI/KASLR;
- hardware platform profiles; and
- stronger liveness/information-flow proofs.

These do not weaken M6/M7 acceptance.

## 11. Stop conditions

Implementation stops and returns to design when:

- a required action cannot be expressed without an unverified body;
- a proof requires weakening a stated security invariant;
- exact capsule-byte refinement cannot be established;
- the IOMMU profile cannot isolate DMA;
- POSIX semantics conflict across services without an atomic protocol; or
- a toolchain gap prevents binding proof to artifact.

The correct response is a revised reviewed design, not an assurance downgrade.
