# Scheduling and SMP

## 1. Scheduling policy

The kernel implements a fixed-priority, preemptive scheduler with 64 priorities:

- `0` is idle;
- `1..47` are normal service and application priorities;
- `48..62` are real-time-capable priorities; and
- `63` is reserved for bounded kernel recovery work.

Each priority has a FIFO ready queue. Normal threads use a 4 ms round-robin
quantum. `SCHED_FIFO` threads run until block, yield, or preemption by a higher
priority. `SCHED_RR` threads use a configured quantum. POSIX policy and permissions
are enforced by `procd`; the kernel enforces only capability-authorized base
priority, affinity, and queue semantics.

The scheduler is tickless. Each CPU programs its next deadline from quantum
expiry, timer objects, and maintenance deadlines.

## 2. Scheduling invariants

1. Each online CPU has exactly one current thread, including its idle thread.
2. A non-idle thread is running on at most one CPU.
3. A ready thread is present in exactly one ready queue.
4. A blocked, suspended, faulted, dead, or running thread is in no ready queue.
5. Queue placement equals effective priority.
6. CPU affinity contains the selected CPU.
7. Effective priority is the maximum of base priority and valid donations.
8. A lower-priority thread is selected only if no eligible higher-priority thread
   is ready for that CPU.

The proof is over selection safety and queue correctness. Starvation freedom is
claimed only within one priority under finite higher-priority demand.

## 3. Single-core milestone

All structures include CPU IDs, affinity masks, active-VSpace masks, and per-CPU
state from the first implementation. The one-core milestone:

- starts only the BSP;
- uses the same scheduler API and per-CPU layout as SMP;
- treats remote masks as singleton;
- proves sequential state transitions; and
- exercises preemption, donation, fault blocking, and timer wakeup.

No one-core shortcut may make an object or ABI incompatible with SMP.

## 4. Four-core locking model

The first SMP implementation uses a verified global ticket lock for all mutable
kernel object state. The rules are:

- every syscall, exception-state transition, and scheduler mutation acquires the
  lock;
- no thread blocks while holding the lock;
- machine actions that can wait are split into prepare/execute/commit phases;
- maskable interrupts do not recursively acquire the lock on the same CPU;
- the lock holder owns a unique ghost token for global state;
- acquisition is FIFO with acquire ordering;
- release has release ordering; and
- NMI/machine-check paths use per-CPU fail-stop records, not normal kernel state.

This serializes kernel transitions but not user execution. It is adequate for four
cores and dramatically reduces the first concurrent proof surface.

## 5. Per-CPU state

Each CPU owns:

- current and idle thread IDs;
- kernel and IST stacks;
- scheduler deadline;
- local ready-queue cache metadata;
- active VSpace and observed TLB epoch;
- pending reschedule/TLB/stop flags;
- interrupt nesting state;
- APIC ID and logical CPU ID; and
- local crash record.

Fields are either CPU-local while interrupts are masked or protected by the global
lock. The ownership rule is encoded in Verus ghost state.

## 6. Load placement

Ready threads have an affinity mask and preferred CPU. Placement is deterministic:

1. retain the last CPU when allowed and its load is within one runnable thread of
   the minimum;
2. otherwise choose the allowed online CPU with the smallest weighted ready
   count;
3. break ties by logical CPU ID.

Migration occurs only while a thread is not running. The source removes queue and
VSpace-active membership before the destination installs them. A reschedule IPI
prompts an idle or lower-priority target.

## 7. Preemption and interrupt interaction

Kernel execution is non-preemptible under the global lock. Timer interrupts:

- record expiry in per-CPU state;
- request reschedule;
- and schedule at the verified exit boundary.

All kernel transitions are bounded. Restartable revocation, bounded IPC transfer,
and continuation objects prevent attacker-controlled unbounded lock hold time.

Higher-priority IRQ notifications wake a server and request rescheduling but do
not context-switch inside an inconsistent transition.

## 8. Priority donation

IPC donation composes with SMP placement:

- a server inherits the maximum valid donor priority;
- a remote running server receives a reschedule IPI when its effective priority
  changes;
- donation-chain changes occur under the global lock;
- cycles and depth greater than eight are rejected before queue changes; and
- removing the final donation requeues a ready server at its base/effective
  priority.

The proof prevents a thread from being queued at a stale priority.

## 9. TLB shootdown and CPU lifecycle

Shootdown uses VSpace epochs as specified in
[memory management](05-memory-management.md). CPU online/offline transitions are
serialized under the global lock.

An AP is:

```text
Absent -> Starting -> Quiescent -> Online -> Stopping -> Offline
```

Only `Online` CPUs run user threads or appear in affinity/load masks. A failed AP
startup returns to `Offline`; it does not reduce single-core correctness.

Panic transitions every online CPU to a stop request. Each CPU enters a capsule
halt loop after saving its crash record.

## 10. SMP acceptance

The four-core milestone must pass:

1. four CPU-bound processes making forward progress concurrently;
2. one million cross-core IPC calls with reply matching and no lost wakeups;
3. concurrent map/unmap plus forced migrations with TLB epoch checks;
4. a priority-inversion test demonstrating bounded donation;
5. repeated driver IRQ delivery while applications run on other cores;
6. AP startup failure injection leaving the system usable on remaining cores;
7. kernel-lock contention instrumentation showing bounded transition duration;
8. randomized state-machine schedules checked against a sequential model; and
9. the complete single-core storage/network/restart acceptance scenario.

## 11. Future fine-grained locking

Per-object locks and lock-free notification paths are not part of the first SMP
release. Any such change requires:

- a lock-order graph;
- linearization points for every affected syscall;
- replacement concurrency proofs;
- new race/fault tests; and
- performance evidence that justifies the larger proof surface.
