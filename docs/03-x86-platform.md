# x86_64 platform

## 1. Reference virtual machine

The reference launch profile is structurally equivalent to:

```sh
qemu-system-x86_64 \
  -machine q35,accel=kvm \
  -cpu host \
  -smp 4 \
  -m 2048 \
  -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=OVMF_VARS.work.fd \
  -device intel-iommu,intremap=on,caching-mode=on \
  -device virtio-blk-pci,disable-legacy=on,iommu_platform=on,drive=osdisk \
  -drive if=none,format=raw,id=osdisk,file=tmk.img \
  -device virtio-net-pci,disable-legacy=on,iommu_platform=on,netdev=net0 \
  -netdev user,id=net0 \
  -device virtio-rng-pci,disable-legacy=on,iommu_platform=on \
  -serial stdio \
  -display none
```

The build system pins exact QEMU and OVMF artifacts. The command above describes
required topology, not a promise that every future QEMU release uses identical
option spelling. CI also runs under TCG with a pinned CPU model. Acceptance is
repeated under KVM.

Modern VirtIO only is supported. Legacy port-I/O VirtIO negotiation is rejected.
All VirtIO devices MUST negotiate `VIRTIO_F_VERSION_1` and
`VIRTIO_F_IOMMU_PLATFORM`.

The `-netdev user` backend is convenient for development. Network acceptance uses
a test-owned isolated peer connected through a pinned QEMU socket or TAP backend;
it does not depend on Internet reachability or host firewall behavior.

## 2. CPU baseline

The BSP verifies CPUID before enabling the kernel. Required features are:

- long mode and four-level paging;
- NX;
- CMPXCHG16B;
- SSE2 and FXSAVE/FXRSTOR;
- local APIC and x2APIC;
- TSC-deadline timer and invariant TSC;
- SMEP and SMAP;
- SYSCALL/SYSRET instruction support, though return initially uses `IRETQ`; and
- PCID/INVPCID for the SMP release.

VT-d is a platform requirement, not a CPUID feature. If a feature is absent, boot
fails before user code. Future profiles MAY define verified fallbacks, but the
reference profile does not silently weaken isolation.

User SSE state is saved eagerly at every context switch. Lazy FPU switching is
forbidden because it complicates information-flow and fault reasoning.

## 3. UEFI loader

The loader is a freestanding Verus-compatible Rust UEFI application. UEFI
firmware calls are modeled as hostile environment interactions:

- every returned pointer/length pair is bounds checked;
- memory-map entry sizes and counts are checked before iteration;
- ACPI configuration table pointers are copied into validated `BootInfo`;
- kernel and service-bundle digests are checked before loading;
- ELF program headers are checked for overflow, overlap, alignment, W^X, and file
  bounds;
- the final memory map is acquired immediately before `ExitBootServices`;
- an invalid map key causes a bounded retry from `GetMemoryMap`; and
- no firmware pointer survives as an unchecked Rust reference.

The firmware itself is not verified. Loader proofs establish safe behavior for all
firmware responses satisfying the UEFI call-level assumptions; failures terminate
boot.

The indirect UEFI service invocation is emitted as a registered firmware-gateway
capsule. Its proof covers the calling convention, stack/register discipline, and
subsequent validation; the firmware’s internal state transition remains the named
OVMF environmental assumption. No Verus `external_body` is used to imply that the
firmware implementation was proved.

### 3.1 Implemented ELF policy checkpoint

The M1 static-kernel ELF/load-plan policy is implemented and accepted as a
same-crate Thermite/direct-Verus kernel composition. It deliberately accepts a
narrow ELF64 little-endian x86-64 `ET_EXEC` profile: a bounded program-header
table, fixed high-half entry range, digest approval, file-contained segments,
sorted non-overlapping `PT_LOAD` virtual ranges, page congruence, readable loads,
W^X, executable entry coverage, non-executable GNU stack metadata, and bounded
GNU RELRO metadata. Dynamic/interpreter segments and unknown metadata are
rejected.

`cargo run -p xtask -- m1-elf` performs three reproducible L3 builds, receipt
validation and replay, a separate runtime consumer, and positive/negative proof
and receipt tests. See [M1 ELF validation](../evidence/m1/elf-validation.md).
This closes the policy transition, not byte decoding, UEFI loading, relocation,
page installation, or the M1 exit gate.

### 3.2 Implemented memory-map and exit policy checkpoint

The M1 firmware-response policy is implemented as two L3/end-to-end Thermite
transitions composed with one direct-Verus shell:

- `memory_map_step` validates descriptor geometry, bounded counts and physical
  ranges, sorted non-overlap, known attributes, cache/runtime consistency, and
  deterministic conversion of UEFI types into nine kernel range classes. A map
  with no conventional usable pages is rejected.
- `firmware_exit_step` bounds map acquisition to eight attempts, grows a
  too-small buffer by a fixed 512-byte/two-maximum-descriptor margin, binds the
  accepted map key and descriptor count, and permits at most four stale-key
  reacquisitions before failing boot.

`cargo run -p xtask -- m1-firmware` performs three reproducible composition
builds, receipt validation/replay, separate runtime execution, and fourteen
malformed-state/proof/receipt rejection cases. The positive runtime trace
observes a stale key, reacquires key 78, exits boot services, and reaches the
terminal state. See [M1 firmware policy](../evidence/m1/firmware-policy.md).

This checkpoint proves policy over normalized call observations. The indirect
UEFI gateway capsule, raw `EFI_STATUS` conversion, raw descriptor reads, physical
copying, and real OVMF `ExitBootServices` execution remain separate gates.

## 4. Boot information

`BootInfoV1` is a versioned, length-delimited, read-only structure containing:

- magic, ABI major/minor, total length, and checksum;
- normalized physical-memory ranges with UEFI type and attributes;
- kernel image physical range and linked virtual range;
- initial service-bundle and configuration ranges with SHA-256 digests;
- ACPI RSDP physical address;
- framebuffer metadata when present, though graphics is unused initially;
- command-line bytes;
- random seed only when provided by an explicitly trusted boot source;
- BSP APIC ID; and
- reserved ranges for loader page tables, stacks, and the structure itself.

The kernel reparses and validates `BootInfo`; it does not trust the loader’s Rust
types across the binary boundary.

The authored `BootInfoV1` policy is L3 with all generated mutants killed. Its
same-source raw-byte shell is currently an intentional failing integration test:
Forge kernel compositions use `--no-vstd`, but have no verified slice element
view with which to connect an executable byte read to its postcondition. Thermite
[#108](https://github.com/dollspace-gay/Thermite/issues/108) records the minimized
failure and required no-stdlib acceptance gates. No unverified decoder adapter is
introduced while that issue is open.

## 5. Virtual-address layout

The initial LA48 layout is:

| Range | Use |
|---|---|
| `0x0000_0000_0000_0000..0x0000_0000_0001_0000` | unmapped low guard |
| `0x0000_0000_0001_0000..0x0000_8000_0000_0000` | user VSpaces |
| `0xffff_8000_0000_0000..0xffff_c000_0000_0000` | physical direct map |
| `0xffff_c000_0000_0000..0xffff_e000_0000_0000` | kernel heap and MMIO windows |
| `0xffff_e000_0000_0000..0xffff_f000_0000_0000` | per-CPU and fixmap windows |
| `0xffff_ffff_8000_0000..0xffff_ffff_c000_0000` | kernel image |

All other high-half ranges are initially unmapped. The direct map covers only
validated RAM and required platform pages; MMIO is mapped separately with correct
cache attributes. User page tables contain the minimum supervisor-only kernel
entry region needed for entry and switch. KPTI is a future hardening item and is
not claimed initially.

Kernel text is RX, rodata is R/NX, data is RW/NX, stacks are RW/NX with guard
pages, and no page is both writable and executable after relocation.

## 6. Descriptor and entry state

Each CPU has:

- a GDT with kernel and user code/data descriptors;
- a TSS with a ring-0 stack pointer;
- IST stacks for double fault, NMI, and machine check;
- an IDT with all 256 entries explicitly initialized;
- a kernel-entry stack with guard pages;
- saved user FPU state storage; and
- a GS-based per-CPU block.

Vector allocation:

| Vector range | Use |
|---|---|
| `0x00..0x1f` | architectural exceptions |
| `0x20..0x2f` | legacy/compatibility IRQ quarantine |
| `0x30..0xdf` | MSI-X and I/O APIC device IRQs |
| `0xe0` | local timer |
| `0xe1` | reschedule IPI |
| `0xe2` | TLB shootdown IPI |
| `0xe3` | stop/panic IPI |
| `0xf0..0xfe` | reserved |
| `0xff` | spurious interrupt |

Unexpected vectors are acknowledged when required, counted, and quarantined.
They never index an unchecked handler table.

## 7. System-call and exception entry

`IA32_LSTAR` points to a proved entry capsule. The capsule:

1. performs `SWAPGS`;
2. stores the user stack pointer in the per-CPU block;
3. loads the per-CPU entry stack;
4. materializes a common trap frame;
5. saves all user-visible general registers;
6. executes `CLD`;
7. validates canonical RIP/RSP and permitted RFLAGS;
8. transitions to the verified Rust dispatcher; and
9. eventually returns through a common `IRETQ` capsule.

Exception stubs normalize error-code and no-error-code exceptions into the same
frame. Page faults capture CR2 before code that could fault again.

SMAP is enabled. User memory is accessed only by proved copy primitives that
bound the range, validate the VSpace mapping epoch, bracket access with
`STAC`/`CLAC`, and return a partial-copy result on fault.

## 8. Time and interrupts

The kernel exposes monotonic time. The platform calibrates or obtains TSC
frequency from validated platform data, checks monotonicity assumptions across
vCPUs, and programs one-shot TSC-deadline interrupts.

Wall-clock time is a user-space policy provided by `timed`; it is not a direct
kernel ambient effect.

Device interrupts use mask-until-ack semantics:

1. the platform masks or consumes the interrupt source;
2. the kernel signals the bound notification;
3. the user driver services the device;
4. the driver invokes `Irq.Ack`; and
5. the platform unmasks the source if the binding generation still matches.

This prevents a crashed driver from causing an unbounded interrupt storm.

## 9. ACPI and PCI

The kernel’s verified parsers consume only:

- RSDP and XSDT;
- MADT for CPU/APIC topology;
- MCFG for PCIe ECAM;
- DMAR for VT-d remapping units and reserved regions; and
- HPET only as diagnostic fallback metadata.

Every ACPI table must pass signature, length, alignment, containment, and checksum
validation. Unknown subtables are skipped by checked length.

PCI enumeration policy is in `devmgr`. The kernel grants `devmgr` a bounded ECAM
capability. BAR regions, MSI-X tables, IRQs, and IOMMU device identities become
typed capabilities; arbitrary physical-memory access is never granted.

## 10. VT-d

The platform initializes DMA and interrupt remapping before starting untrusted
drivers. Each driver receives one IOMMU-domain capability. The kernel controls:

- PCI requester-ID attachment;
- second-level translation tables;
- DMA read/write permissions;
- IOTLB invalidation;
- queued-invalidation completion;
- interrupt-remapping entries; and
- fault reporting.

An assigned device has no identity mapping. DMA buffers are explicit frame grants
mapped into both the driver VSpace and its IOMMU domain. Revocation completes only
after device quiescence, translation removal, IOTLB invalidation, and outstanding
request cancellation.

## 11. AP startup

The SMP release uses MADT topology and a proved INIT-SIPI-SIPI state machine. A
generated low-memory capsule transitions each AP through real mode, protected
mode, and long mode using a dedicated temporary page table and stack.

An AP cannot become `Online` until:

- its APIC ID maps to a unique logical CPU ID;
- its per-CPU block, stacks, GDT, TSS, and IDT are initialized;
- it has joined the global-lock and TLB-epoch protocols; and
- the BSP has observed its startup acknowledgement.

Timeout leaves the AP offline. CPU-count dependent masks use checked widths and
never assume APIC IDs are dense.
