# Kernel objects, capabilities, and IPC

## 1. Object model

Kernel objects are held in a boot-sized metadata arena. The default reference
configuration supports 65,536 live objects and one million capability slots.
Limits are configurable before boot and immutable afterward.

| Object | Purpose |
|---|---|
| `Untyped` | authority over an aligned physical-memory interval |
| `Frame4K` / `Frame2M` | mappable memory frame |
| `PageTable` | one level of an address-space tree |
| `VSpace` | address-space root and mapping epoch |
| `CNode` | capability slot array |
| `Domain` | quota/accounting container for one protection domain |
| `Thread` | schedulable context |
| `Endpoint` | synchronous IPC rendezvous |
| `Reply` | kernel-only one-shot reply authority bound to a server thread |
| `Notification` | asynchronous bit notification |
| `Irq` | interrupt source and mask/ack state |
| `IoRegion` | bounded port-I/O or MMIO authority |
| `IommuDomain` | DMA translation domain |
| `Device` | PCI requester identity and reset authority |
| `Timer` | notification deadline |
| `DebugPort` | development-only diagnostic authority |

Every object has an internal `(index: u32, generation: u32)` identity. Reusing an
arena slot increments the generation; stale references fail rather than naming a
new object.

## 2. Capabilities

A capability is an unforgeable kernel record containing:

- object identity and generation;
- object type;
- immutable rights;
- optional badge;
- parent derivation reference;
- revocation epoch; and
- object-type-specific guard data.

Users name capabilities with a 64-bit `CapPtr` path through their root CNode.
`CapPtr` is not an object pointer and reveals no kernel address.

Common rights are `Read`, `Write`, `Grant`, `GrantReply`, `Control`, `Map`,
`Execute`, and `Debug`. Each object defines its meaningful subset. Minting can
remove rights or add a badge; it cannot add rights. Moving preserves rights and
invalidates the source slot atomically.

### 2.1 Capability invariants

1. Every occupied slot names one live object generation.
2. A child’s rights are a subset of its parent’s rights.
3. Only the kernel creates root authority.
4. A badge cannot increase authority.
5. A capability operation either completes entirely or leaves all slots unchanged.
6. Revocation removes every live descendant existing at the selected epoch.
7. Object destruction requires zero capabilities, zero mappings, no queued IPC,
   and no in-flight machine action referencing the object.

The derivation structure uses a bounded intrusive metadata tree. Revocation is
restartable: each syscall performs bounded work and returns a continuation token
when more descendants remain. This prevents a hostile capability tree from
monopolizing the kernel.

## 3. Untyped memory and object creation

The root task receives `Untyped` capabilities for usable physical ranges not
retained by the kernel. `Untyped.Retype`:

1. validates alignment, size, object count, and non-overlap;
2. reserves object metadata;
3. zeroes any memory that could become user visible;
4. creates typed objects; and
5. installs capabilities into empty destination slots atomically.

Retyping never converts MMIO or firmware-reserved memory into ordinary frames.
An untyped interval cannot be simultaneously retyped into overlapping objects.
Reclaim requires destruction of every descendant object.

## 4. Endpoint IPC

Endpoints provide synchronous rendezvous with FIFO ordering within effective
priority. Supported operations are:

- `Send`: enqueue or transfer a one-way message;
- `Recv`: receive or block;
- `Call`: send and block with a one-shot reply object;
- `Reply`: reply to the server thread’s current caller;
- `ReplyRecv`: reply and atomically wait for the next request.

The fast path carries:

- a 32-bit protocol/message tag;
- four 64-bit words in registers;
- a sender badge; and
- up to two capability transfers described in the UTCB.

The per-thread user communication buffer (UTCB) carries up to 64 additional words,
transfer-slot paths, fault records, and protocol metadata. Large payloads use
shared frame grants.

### 4.1 Transfer rules

- The sender needs `Grant` to transfer a derived capability.
- The receiver chooses empty destination slots before blocking.
- Rights are intersected with the sender-specified mask.
- A failed transfer delivers neither message nor partial capabilities.
- Reply objects are kernel-created, never installed in a CNode, non-transferable,
  single-use, and destroyed on reply, caller cancellation, or server death.
- A server thread has at most one active reply. Receiving another `Call` before
  consuming it is rejected; `ReplyRecv` is the normal server loop.
- Sender identity is represented by the receiver-minted endpoint badge; global
  process IDs are not trusted for authorization.

## 5. Notifications

A notification contains a 64-bit pending bitmap and a wait queue. Signaling ORs
bits; waiting atomically consumes the current bits or blocks. IRQ and timer
objects signal notifications.

Notifications do not carry capabilities or arbitrary messages. This separation
keeps interrupt delivery asynchronous while RPC remains typed and synchronous.

## 6. Priority donation

`Call` donates the caller’s effective priority to the receiving server while the
reply object is live. Donation:

- is bounded to eight nested call edges;
- tracks a visited-thread bitmap to reject cycles;
- is removed on reply, timeout, cancellation, or server death; and
- never changes the thread’s base priority.

If the bound would be exceeded, `Call` fails with `E_DEPTH` before mutating queues.
The rule prevents common priority inversion without requiring unbounded graph
reasoning in the kernel.

## 7. Thread state

A thread is in exactly one state:

```text
Inactive
Ready(cpu)
Running(cpu)
BlockedSend(endpoint)
BlockedRecv(endpoint)
BlockedReply(reply)
BlockedNotification(notification)
Faulted(fault_id)
Suspended
Dead
```

A thread appears in at most one queue, and `Running(cpu)` is bijective with the
CPU’s current-thread field. Transition helpers remove the old queue membership
before installing the new state.

Each thread owns:

- saved integer and FPU contexts;
- one quota/accounting domain;
- a VSpace and root CNode capability reference;
- UTCB address and validated mapping generation;
- base/effective priority and affinity;
- fault endpoint capability.

## 8. Fault protocol

User exceptions become messages to the thread’s fault endpoint:

- fault kind and architecture error code;
- RIP, RSP, RFLAGS, and fault address when relevant;
- access type;
- VSpace mapping epoch; and
- a non-transferable fault-reply token.

The pager may map memory and resume with validated registers, terminate the
thread, or forward policy to `procd`. A stale fault token cannot resume a later
fault.

Kernel-mode page faults, invalid internal object references, and failed invariant
checks are fail-stop kernel faults.

### 8.1 Implemented exception-policy checkpoint

The first executable fault-policy transition is accepted in Thermite. It proves
that a valid live user thread with a fault endpoint receives an exact
generation-tagged fault action carrying vector/error, decoded page access, CR2,
and VSpace epoch; absence of the endpoint terminates the thread without creating
a reply generation. Fatal architectural vectors, kernel exceptions, corrupt
page-fault metadata, missing thread state, invalid frames/vectors, overflow, and
stop requests produce an exact reason and latch fail-stop state with rescheduling
cleared. The same transition classifies timer, reschedule, TLB, device,
quarantine, and spurious events without conflating those actions with fault
delivery.

The accepted claim ends at the returned state/action pair. Current-thread and
endpoint lookup, fault-token object allocation, scheduler queue mutation,
platform masking/acknowledgement, TLB invalidation, and the concrete frame-to-
event/action-to-machine bridge are still later verified transitions. See
[M1 exception-dispatch policy](../evidence/m1/exception-dispatch-policy.md).

## 9. Transition atomicity

Every syscall follows:

1. copy and normalize untrusted inputs;
2. resolve capabilities without mutation;
3. validate all preconditions and reserve bounded resources;
4. compute the Thermite transition;
5. execute proved machine actions;
6. commit the abstract state; and
7. copy bounded outputs.

No fallible user copy occurs after authority commit unless the result semantics
explicitly allow “operation completed, output unavailable.” Where POSIX requires
partial progress, that behavior belongs to the user-space server protocol, not an
accidental kernel-copy fault.

The action vector has a fixed maximum of 16 actions per transition. Longer work
uses explicit continuation objects.

Actions used in a committing transition are proved total on the supported
platform when their preconditions hold. A recoverable hardware observation is
collected before commit or becomes a new normalized event for a second
transition. If hardware violates an action assumption after a concrete mutation,
the kernel fail-stops before user re-entry; it does not roll forward an unproved
partially committed abstract state.

## 10. Core proof invariants

The core proof includes:

- object-generation safety;
- capability derivation and rights monotonicity;
- queue membership uniqueness;
- reply linearity;
- endpoint transfer atomicity;
- absence of unauthorized frame/device/IRQ access;
- bounded resource use per transition;
- preservation of VSpace and IOMMU correspondence;
- scheduler state well-formedness; and
- failure-state noninterference for authority-bearing fields.
