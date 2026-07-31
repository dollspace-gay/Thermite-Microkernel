# System architecture

## 1. Overview

TMK uses a functional-core/verified-shell architecture.

```text
 POSIX applications and shell
          |
      TMK libc
          |
  +-------+---------+----------+----------+
  | process server | VFS/TMFS | net stack | device manager
  +-------+---------+----------+----------+
          | capability-authenticated IPC
  +-------v--------------------------------------------------+
  | TMK microkernel                                          |
  | capabilities | IPC | threads | VSpaces | scheduler | IRQ |
  +-------+--------------------------------------------------+
          | declarative MachineAction values
  +-------v--------------------------------------------------+
  | verified Verus platform shell                            |
  | page tables | entry/exit | APIC | VT-d | context switch |
  +-------+--------------------------------------------------+
          | verified x86 instruction capsules
  +-------v--------------------------------------------------+
  | x86_64 CPU and QEMU/KVM q35 platform                     |
  +----------------------------------------------------------+
```

The Thermite core does not perform foreign calls. It consumes validated events
and current abstract state, then returns a new state plus a bounded list of
declarative actions. The Verus shell proves that executing those actions refines
their abstract meaning.

Examples:

- `handle_page_map(state, request)` returns updated ownership/mapping state and a
  `MapPte` action.
- `handle_irq(state, vector)` updates notification state and returns `MaskIrq`,
  `WakeThread`, and `Schedule` actions.
- `handle_ipc_call(state, caller, endpoint, msg)` transfers authority and returns
  a context-switch decision.

No Thermite kernel function carries ambient `read`, `write`, `net`, `time`,
`rand`, or `term` effects. Although the general Forge kernel profile can emit L1
code carrying `panic` or `diverge`, strict L3 artifacts reject both. Release
Thermite closures therefore carry only `pure` and, where a proved bounded host
allocator is available, `alloc`. Exhaustion is returned explicitly.

## 2. Kernel responsibilities

The kernel owns only:

- physical CPU and per-CPU execution state;
- threads and saved user contexts;
- address-space objects and page-table authority;
- frame ownership and mapping metadata;
- capability spaces and derivation metadata;
- endpoints, reply objects, and notifications;
- interrupt sources and delivery bindings;
- scheduling queues, timer deadlines, and CPU affinity;
- VT-d domains and DMA mappings; and
- boot-created authority handed to the root task.

The kernel does not parse paths, implement file descriptors, speak network
protocols, load device-specific VirtIO queues, or choose restart policy.

## 3. User-space responsibilities

The initial root task starts a least-authority service graph:

| Service | Responsibility |
|---|---|
| `init` | supervision, service graph, capability distribution |
| `procd` | POSIX processes, credentials, sessions, signals, wait state |
| `pagerd` | address-space policy, ELF loading, COW, `mmap` |
| `vfsd` | namespaces, path resolution, descriptors, pipes |
| `tmfsd` | journaled persistent filesystem |
| `devmgr` | PCI discovery and device-capability assignment |
| `virtioblkd` | modern VirtIO block queue and request handling |
| `virtionetd` | modern VirtIO network queue and packet handling |
| `virtiorngd` / `rngd` | isolated entropy input and cryptographic DRBG |
| `netd` | Ethernet, ARP, IPv4, ICMP, UDP, TCP, socket objects |
| `termd` | console sessions, line discipline, shell terminal |
| `timed` | wall-clock policy derived from kernel monotonic time |
| `logd` | structured logs and crash records |
| `sh` | interactive POSIX-oriented shell |

Drivers and servers communicate with framed RPC and shared-memory grant
capabilities. Bulk data does not traverse kernel-owned buffers.

## 4. Boot sequence

1. OVMF loads a signed or development UEFI PE/COFF loader.
2. The verified loader reads the kernel, initial service bundle, configuration,
   and expected digests from the EFI system partition.
3. It obtains and validates the UEFI memory map and ACPI root pointer.
4. It allocates boot stacks, page tables, the kernel image, and boot modules.
5. It retries `GetMemoryMap`/`ExitBootServices` according to UEFI rules.
6. A verified transition capsule installs the kernel page table and stack.
7. The BSP initializes GDT/TSS/IDT, exception stacks, APIC, VT-d, and the timer.
8. The kernel constructs initial frame, untyped-memory, device, IRQ, and root
   capability objects.
9. `init` starts in ring 3 with only the boot authority explicitly listed in
   `BootInfo`.
10. `init` delegates subsets of authority to services and starts the POSIX
    environment.

UEFI runtime services are not used after `ExitBootServices`. This avoids retaining
firmware mappings and firmware-call complexity in the running kernel.

## 5. Architectural boundaries

### 5.1 Thermite core boundary

Inputs to Thermite transitions are value types with no raw pointers:

- normalized syscall requests;
- bounded IPC messages;
- validated object identifiers and generations;
- normalized faults and interrupts;
- monotonic tick values; and
- bounded configuration records.

Outputs are state plus `MachineAction` values. The shell MUST validate that each
action’s precondition still holds before changing hardware state.

### 5.2 Platform boundary

The platform shell owns:

- raw physical and virtual addresses;
- control registers and model-specific registers;
- page-table memory writes;
- port I/O and MMIO;
- interrupt entry/return frames;
- FXSAVE/FXRSTOR state;
- AP startup; and
- UEFI calls before handoff.

Every operation has a Verus precondition/postcondition over the abstract machine
state. There are no contract-only external bodies in a release.

### 5.3 User/kernel boundary

Ring-3 code enters through `SYSCALL` or CPU exceptions. The entry capsule:

- switches to a per-CPU kernel stack;
- saves a complete user context;
- clears direction and unsafe flag bits;
- validates user canonical addresses;
- identifies the current thread and CPU; and
- creates a normalized event for the verified shell.

Return uses `IRETQ` for one uniform checked path. `SYSRETQ` is deferred until its
canonical-address and flag edge cases receive a separate proof.

## 6. Failure containment

| Failure | Required containment |
|---|---|
| User page fault | notify pager or terminate only the faulting process |
| Malformed syscall/RPC | return error; do not mutate authority |
| User server crash | supervisor restarts service; clients receive generation change |
| Driver crash | revoke BAR/IRQ/DMA caps, reset device, restart driver |
| IOMMU fault | quarantine device and notify `devmgr`/`init` |
| Kernel invariant failure | stop all CPUs, emit crash record, never continue |
| Proof/artifact gate failure | no release image |

Kernel panic is fail-stop. Recovery from kernel corruption is not claimed.

## 7. Versioning

Three independent versions exist:

- kernel native ABI version;
- service protocol versions; and
- persistent TMFS format version.

Each message begins with protocol, major version, operation, flags, and payload
length. Major mismatches fail explicitly. Minor extensions use length-delimited
fields and feature negotiation.
