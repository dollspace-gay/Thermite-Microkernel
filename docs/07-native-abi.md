# Native kernel ABI

## 1. Principles

The native ABI is capability-oriented, small, versioned, and independent of
POSIX. It exposes kernel mechanisms, not filesystem paths, process IDs, sockets,
or file descriptors.

The ABI is source and binary stable within one major version. Reserved fields
must be zero. Unknown operations return `K_E_VERSION` or `K_E_UNSUPPORTED`.

All integers are little-endian. Public structures use explicit-width integer
fields and explicit padding. Rust layout without `repr(C)` is never an ABI.

## 2. System-call register convention

`SYSCALL` inputs:

| Register | Meaning |
|---|---|
| `RAX` | syscall number |
| `RDI` | capability pointer or zero |
| `RSI` | message tag or invocation operation |
| `RDX` | fast word 0 |
| `R10` | fast word 1 |
| `R8` | fast word 2 |
| `R9` | fast word 3 |

Outputs:

| Register | Meaning |
|---|---|
| `RAX` | `KernelStatus` |
| `RDI` | returned message tag |
| `RSI` | sender badge or auxiliary result |
| `RDX`, `R10`, `R8`, `R9` | returned fast words |

`RCX` and `R11` have architectural SYSCALL meanings and are saved in the common
trap frame. All other user registers are preserved unless a specific operation
documents a return value.

## 3. System calls

| Number | Name | Meaning |
|---:|---|---|
| 0 | `Invoke` | invoke an operation on a typed capability |
| 1 | `Send` | synchronous or queued one-way endpoint send |
| 2 | `Recv` | receive from an endpoint |
| 3 | `Call` | send and block for one reply |
| 4 | `Reply` | consume the current thread’s one-shot reply |
| 5 | `ReplyRecv` | reply and atomically receive on an endpoint |
| 6 | `Yield` | yield the current quantum |
| 7 | `AbiQuery` | return ABI/features/limits without mutation |

All object management, notifications, timers, IRQ acknowledgements, VSpace
operations, and IOMMU operations are typed `Invoke` operations. Debug output is an
invocation on a `DebugPort` capability, not an ambient syscall.

`Reply` uses the kernel-held active reply bound to the current server thread;
`RDI` is zero. `ReplyRecv` uses `RDI` for the receive endpoint and consumes the
active reply atomically. Reply authority is never a user-forgeable `CapPtr`.

## 4. Capability pointers

`CapPtr` is a 64-bit path:

```text
bits 63..48  root guard
bits 47..32  level-1 slot
bits 31..16  level-2 slot
bits 15..0   reserved for future depth; zero in ABI v1
```

The root CNode configuration determines active guard/depth bits. Resolution has a
bounded maximum depth of two in ABI v1. A malformed path returns `K_E_CAP`.

Object generations and kernel addresses are not exposed in `CapPtr`.

## 5. User communication buffer

Each thread is configured with a page-aligned UTCB virtual address in its VSpace.
The kernel validates and pins that page while the thread exists. Threads sharing a
VSpace may use different UTCB addresses.

`UtcbV1` contains:

- ABI magic/version and total length;
- message length, flags, and protocol tag;
- 64 message words;
- two send-capability descriptors;
- two receive-slot paths;
- fault-message fields;
- timeout/deadline fields;
- returned sender badge and status detail; and
- reserved zeroed extension space.

Fast IPC with at most four words and no capability transfer need not touch the
UTCB. The kernel copies the UTCB into bounded kernel values before state mutation;
it never holds a user pointer across a block or context switch.

## 6. Message tags

A 64-bit tag is:

```text
bits 63..48  protocol major
bits 47..32  protocol identifier
bits 31..16  operation
bits 15..8   capability-transfer count
bits 7..0    word count beyond fast registers
```

Service protocols can add their own minor version in word 0. The kernel validates
only lengths and transfer counts; protocol meaning belongs to user space.

## 7. Kernel status

The native status namespace is not `errno`.

| Status | Meaning |
|---|---|
| `K_OK` | operation completed |
| `K_E_CAP` | invalid capability path, type, generation, or rights |
| `K_E_ARG` | malformed or out-of-range value |
| `K_E_STATE` | operation invalid in current object state |
| `K_E_EMPTY` | required destination slot or queue state unavailable |
| `K_E_FULL` | bounded queue, slot, or metadata limit reached |
| `K_E_QUOTA` | protection-domain resource limit reached |
| `K_E_TIMEOUT` | deadline expired without completion |
| `K_E_CANCELLED` | wait/reply cancelled by destruction or restart |
| `K_E_DEPTH` | derivation/donation/restart work exceeded one-call bound |
| `K_E_FAULT` | user copy or supplied mapping faulted |
| `K_E_VERSION` | unsupported ABI or protocol major |
| `K_E_UNSUPPORTED` | valid but unavailable feature |
| `K_E_HARDWARE` | proved platform operation reported a hardware failure |

TMK libc and services translate these statuses into operation-specific POSIX
`errno` values. A global one-to-one translation is forbidden because POSIX errors
depend on interface context.

## 8. Typed invocation operations

### 8.1 CNode

- `Copy(src, dst, rights_mask)`
- `Mint(src, dst, rights_mask, badge)`
- `Move(src, dst)`
- `Delete(slot)`
- `Revoke(slot, continuation)`

### 8.2 Untyped

- `Retype(kind, offset, count, destination_slots)`
- `Reset(continuation)`

### 8.3 VSpace and page tables

- `MapTable(table, virtual_address)`
- `MapFrame(frame, virtual_address, permissions, cache_policy)`
- `Unmap(virtual_address, size)`
- `Protect(virtual_address, size, permissions)`
- `Query(virtual_address)`

### 8.4 Thread

- `Configure(vspace, cspace, utcb, fault_endpoint)`
- `ReadRegisters`
- `WriteRegisters`
- `Start(entry, stack, argument)`
- `Suspend`
- `Resume`
- `SetPriority`
- `SetAffinity`
- `Destroy`

### 8.5 Domain

- `SetQuota(resource, limit)`
- `QueryUsage(resource)`
- `BindObject(object)`
- `Destroy`

Only an authorized supervisor can raise a quota or move an uncharged new object
into a domain. Moving a live charged object between domains is not supported in
ABI v1.

### 8.6 Notification and timer

- `Signal(bits)`
- `Wait(mask, deadline)`
- `Arm(deadline, period, notification, bits)`
- `Cancel`

### 8.7 IRQ

- `Bind(notification, bits)`
- `Mask`
- `Ack`
- `Unbind`

### 8.8 Device and IOMMU

- `ReadConfig(offset, width)`
- `WriteConfig(offset, width, value, mask)`
- `CreateBarRegion(bar_index, destination_slot)`
- `ConfigureMsiX(vector, notification)`
- `AttachDevice(device)`
- `DetachDevice(device)`
- `MapDma(frame, io_address, permissions)`
- `UnmapDma(io_address, size)`
- `ResetDevice`
- `QueryFault`

PCI writes are filtered by field and rights; a caller cannot disable remapping,
retarget another function, or enable bus mastering before domain attachment.

### 8.9 I/O region

- `Read(offset, width)`
- `Write(offset, width, value)`
- `MapMmio(vspace, virtual_address, permissions, cache_policy)`
- `UnmapMmio(vspace, virtual_address)`

Port I/O remains a kernel-mediated invocation. MMIO may be mapped into a driver
VSpace only within the exact BAR interval and with non-executable device-memory
attributes.

Operations are further constrained by capability type and rights. The full
generated IDL becomes the executable source of numeric operation codes.

## 9. User process entry

A new user process begins at an ELF entry point with:

- `RSP` pointing to a System-V-compatible initial stack;
- `RDI` holding a read-only startup-block pointer;
- `RSI` holding the startup-block length;
- FS base initialized for TLS;
- DF clear and interrupts enabled;
- only user-permitted RFLAGS bits;
- SSE control state initialized; and
- all unspecified registers zeroed to prevent cross-domain disclosure.

The startup block names the ABI version, UTCB address, initial capability slots,
argument/environment vectors, auxiliary values, and service-directory capability.

## 10. Fault ABI

Fault messages use a separate protocol identifier and include:

- fault generation;
- exception class and architecture vector;
- error code;
- access address and access type;
- complete resumable user register state;
- VSpace epoch; and
- one fault-reply token.

The responder can `Resume`, `SetRegistersAndResume`, or `Terminate`. Register
updates validate canonical user addresses, flags, segment state, and reserved
bits.

## 11. Cancellation and restart

Destroying an endpoint, reply object, server thread, or service generation wakes
affected callers with `K_E_CANCELLED`. Blocking kernel calls are not transparently
restarted.

TMK libc decides whether a POSIX interface returns `EINTR`, retries, or reports
service restart. Requests with external side effects carry service-level unique
request IDs so a client can determine whether retry is safe.

## 12. ABI verification

The ABI test suite:

- generates C and Rust layouts from one IDL;
- proves bounds and tag decoding in Thermite/Verus;
- checks structure size, alignment, and offsets at compile time;
- fuzzes every syscall decoder;
- checks failure atomicity for each operation;
- replays golden register/UTCB vectors; and
- runs old-minor clients against newer-minor kernels.
