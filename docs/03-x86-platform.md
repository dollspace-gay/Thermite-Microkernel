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

### 5.1 Implemented address-plan policy checkpoint

The scalar kernel address-plan transition is implemented and accepted as an
L3/end-to-end Thermite/direct-Verus composition. It fixes the windows and direct
physical offset above, requires the low guard and absence of a recursive mapping,
checks page alignment and global virtual non-overlap, rejects W+X, and requires
guards around every modeled per-CPU stack mapping. Direct-RAM, heap/MMIO, and
stack mappings may not alias any physical page in the kernel image. Image mappings must appear as
contiguous RX text, R/NX rodata, and RW/NX data and must cover the physical image
exactly.

`cargo run -p xtask -- m1-address` performs three reproducible builds, receipt
validation/replay, separate runtime execution, a 64/64 mutation battery, and
fourteen malformed-plan/proof/receipt rejection cases. See
[M1 address-space policy](../evidence/m1/address-space-policy.md).

This checkpoint consumes scalar mapping observations.

### 5.2 Implemented reference page-table checkpoint

The next accepted checkpoint lowers the six-region fixture into thirteen real,
contiguous, 4-KiB-aligned table pages. A `no_std` executable Verus constructor
populates all four levels for the direct, heap, guarded-stack, and kernel-image
paths and proves every other entry zero. Its executable walker follows the
encoded physical links, rejects absent and huge intermediate entries, combines
permission bits across levels, and proves ten registered translation and
non-translation observations.

`cargo run -p xtask -- m1-page-tables` performs three reproducible proof/codegen
runs and three reproducible runtime links, executes every consumer, audits the
artifact dependency surface, and rejects wrong-text-physical, executable-data,
unexpected-present-entry, and false-observation mutations. See
[M1 reference boot page tables](../evidence/m1/boot-page-tables.md).

The fixed image is an encoding/correspondence fixture, not the full boot map.
Real page-table frame ownership and physical writes, a bounded builder for all
accepted firmware ranges, cache-attribute connection, CR3 installation,
invalidation capsules, and live QEMU translation probes remain open M1 gates.

### 5.3 Implemented CR3 capsule checkpoint

The initial root-install capsule is accepted as the exact four bytes
`0f 22 df c3` (`mov cr3,rdi; ret`). Its direct Verus machine model requires CPL0,
PCID disabled, a page-aligned 52-bit root, a readable non-overflowing return
stack, and a canonical return address. It proves the CR3 write, RET stack/RIP
effects, non-global TLB invalidation, and preservation of unrelated state. The
specialized call contract binds RDI to the reference root at `0x0040_0000`.

`cargo run -p xtask -- m1-cr3` performs three reproducible model builds, three
runtime decoder executions, and three high-half post-links; requires one
four-byte executable section with no relocations; and rejects six byte, section,
semantic, binding, and proof-escape mutations. See
[M1 CR3 installation capsule](../evidence/m1/cr3-install-capsule.md).

This closes the capsule refinement and post-link identity only. The caller does
the validation; the instruction has not yet run in the boot VM. Root-frame
ownership/population, verified call-site state, final-kernel inclusion, and live
post-install translation probes remain open. PCID remains disabled until its
allocator, `INVPCID`, local invalidation, and SMP shootdown protocols are proved.

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

### 6.1 Implemented descriptor-table image checkpoint

The initial per-CPU descriptor images are implemented in executable `no_std`
Verus. The GDT has exactly seven entries: null, kernel code/data, user data/code,
and the two-slot available 64-bit TSS descriptor. Its selector order supports
the later `SYSCALL`/`SYSRET` STAR relationship while the initial return path
remains `IRETQ`. The packed 104-byte TSS installs RSP0, separate IST1/IST2/IST3
tops for double fault/NMI/machine check, and disables the I/O bitmap at byte 104.

The 4-KiB IDT contains 256 initialized 16-byte interrupt gates. Every gate uses
the kernel-code selector and a registered 16-byte handler slot; vector 3 alone
has DPL 3; vectors 8, 2, and 18 select IST1, IST2, and IST3 respectively; and all
reserved bits are zero. The executable decoders are proved to recover the
registered offsets, selectors, attributes, IST indices, and TSS base.

`cargo run -p xtask -- m1-descriptors` performs three reproducible proof/codegen
runs and three separately linked runtime executions, exhaustively scans all 256
gates, checks ABI sizes/alignments and descriptor pointers, audits the rlib
dependency surface, and rejects seven semantic/completeness/proof-escape
mutations plus the explicit no-`vstd` proof boundary. See
[M1 descriptor-table images](../evidence/m1/descriptor-tables.md).

This checkpoint proves memory construction only. The registered handler
addresses are fixture slots; the 256 entry stubs are not yet built or linked.
The selected stack tops still require placement in guarded mapped pages. No
`LGDT`, `LIDT`, `LTR`, segment reload, exception delivery, or privilege-return
instruction has executed on hardware. Those effects are the next capsule and
BSP-entry checkpoints.

### 6.2 Implemented descriptor-install capsule checkpoint

The exact 38-byte descriptor-install sequence is registered at
`0xffffffff80001010`. It executes `LGDT [RDI]`, loads the kernel-data selector
into DS/ES/SS, uses a same-privilege `RETFQ` to reload CS, executes `LTR` with the
TSS selector, executes `LIDT [RSI]`, and returns. The PC-relative far target is
inside the registered byte range, and the linked capsule has no relocations.

The direct Verus machine model requires CPL 0, disabled maskable interrupts, an
explicit asynchronous-event-free interval, canonical readable GDTR/IDTR
operands and table ranges, registered GDT/IDT content, a writable available TSS
descriptor, writable far-return stack space, and a readable final return slot.
It proves exact GDTR/IDTR limits and bases, CS/DS/ES/SS/TR values, the net
eight-byte call/return stack effect, RDI/RSI/RFLAGS preservation, RAX clobber to
`0x28`, and the architectural available-to-busy TSS-descriptor transition.

`cargo run -p xtask -- m1-descriptor-install` performs three reproducible model
builds, three runtime decoder/emitter executions, and three high-half post-links;
requires one 38-byte executable section with no relocations and the complete
expected disassembly; and rejects eight byte, extra-section, semantic, and
proof-escape mutations. See
[M1 descriptor-install capsule](../evidence/m1/descriptor-install-capsule.md).

This closes exact-byte refinement and link identity, not the live call site.
FS/GS and their bases are deferred to the per-CPU setup capsule. No exception
stub exists yet, and this capsule has not run under OVMF/QEMU. The BSP must still
prove concrete operand/table/stack ownership, enforce the asynchronous
quiescence requirement, call these exact linked bytes, and read back the
installed register state before the hardware-execution claim closes.

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
