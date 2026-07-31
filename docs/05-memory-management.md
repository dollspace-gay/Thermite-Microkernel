# Memory management

## 1. Physical memory

The loader and kernel normalize the UEFI memory map into sorted, non-overlapping
half-open intervals. Overlap, wraparound, invalid alignment, or contradictory
types stop boot.

Physical memory is classified as:

- kernel image and metadata;
- boot structures;
- ACPI/VT-d reserved;
- MMIO;
- reclaimable loader memory;
- ordinary usable RAM; or
- unavailable.

Only ordinary usable and explicitly reclaimed loader RAM can back `Untyped`
objects. All memory is zeroed before crossing from one protection domain to
another.

## 2. Kernel allocation

The kernel uses three bounded allocators:

1. a bootstrap bump allocator before object initialization;
2. fixed-size metadata slabs for kernel objects, capabilities, queue nodes, and
   proof-visible records; and
3. a buddy allocator for kernel-owned physical pages.

There is no general unbounded kernel heap after boot. Forge-generated `alloc`
operations are routed through a verified bounded allocator whose failure is an
ordinary result, never hidden panic. The final design must ensure generated
collection capacity cannot grow beyond a pre-proved bound.

The M0 byte/layout policy accepts alignments 1, 2, 4, 8, 16, 32, 64, and 4096;
all other alignments, zero-sized requests, corrupt arena state, exhaustion, and
arithmetic-edge failures leave the cursor unchanged and return failure. The
alignment set covers the selected kernel object profile and page-aligned boot
objects; adding another alignment requires a new bit-vector proof and runtime
case. Runtime alignment uses a mask only after proving it equivalent to modulo,
so the freestanding artifact has no division-by-zero panic dependency.

`core::alloc::GlobalAlloc` is only an ABI adapter. Its implementation MUST refine
the verified byte/layout result and create a pointer carrying the arena's
provenance and writable permission. If pinned Verus cannot verify `Layout`
inspection and pointer construction, the adapter MUST be an exact-byte machine
capsule with those semantics or wait for verifier support. An `external_body`,
assumed standard-library specification, or unchecked `unsafe` wrapper is not an
acceptable bridge.

The implemented M0 bridge takes the exact-byte route. Its boot arena is a
4096-aligned 64 KiB zero-initialized region followed by an eight-byte cursor and
an eight-byte sealed flag. A no-cheating Verus model proves the registered
allocation bytes refine the byte/layout policy, preserve failure state, return an
arena-derived aligned address on success, and refuse every allocation after the
seal transition. Exact-image decoders accept only the registered allocator,
seal, memory-operation, shim, and relocation encodings before exposing those
semantics. The pointwise `memcpy` and `memset` models require valid ranges,
non-overlap for `memcpy`, and the kernel's DF-clear calling invariant. Pinned
Rust contributes only the `GlobalAlloc` calling-convention adapter; every
emitted function body and relocation is compared against the model-owned
skeleton and target plan before linking. Thus the unsafe Rust syntax is not
trusted by inspection or contract:
only the audited machine instance is accepted. Both a runnable low-address ELF
and a kernel-code-model ELF based at `0xffffffff80000000` must link without
undefined symbols or relocation truncation.

This allocator is boot-only. `dealloc` is a no-op, while `realloc` and
`alloc_zeroed` return null. Kernel collections must request their proved capacity
up front and never grow. The BSP seals the arena before any AP is started; after
that happens all cores can only observe allocation failure, and no cursor write
is possible. General physical/object allocation uses the verified allocators in
§1–2 rather than this Rust ABI bridge.

## 3. VSpaces

A `VSpace` owns:

- a PML4 root;
- page-table object set;
- mapping metadata;
- address-space identifier/PCID;
- active-CPU mask; and
- monotonically increasing mapping epoch.

Page-table pages are kernel-owned and supervisor-only. User tasks never receive
write access to them.

Supported mappings are 4 KiB and 2 MiB. User mappings require:

- canonical user range;
- page alignment;
- a live frame capability with `Map`;
- permissions no greater than the capability;
- W^X;
- no overlap unless replacing the exact mapping under an explicit operation; and
- no alias with a frame reserved for kernel/page-table/VT-d metadata.

`Map`, `Unmap`, and `Protect` are transactional with the abstract mapping tree.
The concrete PTE update occurs through a proved `MachineAction`.

## 4. Mapping invariant

For every present user PTE:

- exactly one live mapping record names the VSpace, virtual range, frame
  generation, offset, size, and permissions;
- the physical address lies within the named frame;
- `U/S=1`, kernel global mappings are unreachable, and reserved bits are valid;
- executable implies not writable;
- the mapping’s frame authority permits its access;
- every active CPU has observed at least the mapping’s required invalidation
  epoch; and
- a device DMA alias, if any, is separately authorized by its IOMMU domain.

The inverse also holds: committed abstract mappings are reflected by concrete page
tables before returning to user mode.

## 5. TLB invalidation

Single-core:

- local changes use `INVLPG` or CR3/INVPCID according to range size;
- the invalidation completes before the mapping transition commits to user mode.

SMP:

1. increment the VSpace mapping epoch under the global lock;
2. update PTEs;
3. snapshot the active-CPU mask;
4. send shootdown IPIs to other active CPUs;
5. each target invalidates, records the epoch with release ordering, and
   acknowledges;
6. wait with a bounded watchdog while still preventing user re-entry into that
   VSpace; and
7. complete only after every target acknowledges or is fail-stopped.

Frames are not reassigned until shootdown completion. This prevents stale TLB
access to retyped memory.

## 6. User copy

Kernel code does not dereference arbitrary user pointers. Copy operations:

- validate start plus length without overflow;
- limit one call to 64 KiB;
- walk current mapping records;
- pin referenced frames for the copy duration;
- enforce requested read/write rights;
- use SMAP-bracketed capsule primitives;
- return copied length and a defined fault code; and
- unpin on all exits.

IPC fast words avoid user copies. Bulk I/O uses shared-memory capabilities.

## 7. Pager and POSIX virtual memory

`pagerd` owns address-space policy:

- ELF segment placement;
- anonymous memory;
- shared memory;
- file-backed `mmap`;
- copy-on-write after `fork`;
- stack growth within fixed guards;
- `mprotect`;
- and process teardown.

The kernel supplies mechanism:

- create and map frame/page-table objects;
- deliver page faults;
- suspend/resume faulting threads;
- clone capability authority under policy; and
- atomically replace mappings.

### 7.1 Copy-on-write

On `fork`, writable private frames are remapped read-only into parent and child
with a COW record in `pagerd`. A write fault causes:

1. allocation of a new zeroed frame;
2. copy from the pinned source frame;
3. mapping replacement in the faulting VSpace;
4. required TLB invalidation; and
5. resume using the matching fault token.

The kernel does not interpret “COW”; it enforces frame/mapping authority.

## 8. DMA memory

DMA buffers are frame objects with explicit pin counts. Mapping a DMA buffer:

- requires frame and IOMMU-domain capabilities;
- validates device address width and alignment;
- installs VT-d translations with least permissions;
- invalidates IOTLB state;
- records domain/frame correspondence; and
- returns an I/O virtual address, not a physical address.

A frame cannot be destroyed, retyped, or reassigned while pinned or DMA mapped.
On driver failure, device bus mastering is disabled before unmapping and
reassigning buffers.

## 9. Resource accounting

Each protection domain has quotas for:

- frame bytes;
- page-table pages;
- mappings;
- pinned/DMA bytes;
- kernel objects;
- capability slots; and
- outstanding faults.

Kernel creation operations charge before commit and refund on destruction.
Quota exhaustion is local and cannot consume global emergency reserves used for
fault reporting and service recovery.

## 10. Verification obligations

Memory proofs cover:

- interval normalization and non-overlap;
- allocator conservation;
- zero-before-reuse;
- page-table encoding and decoding;
- abstract/concrete mapping correspondence;
- W^X and supervisor/user separation;
- checked canonical-address arithmetic;
- no premature frame reuse;
- TLB epoch safety;
- COW protocol safety across pager crashes; and
- IOMMU mapping subset and invalidation correctness.
