# Security and recovery

## 1. Threat model

The initial security claim covers:

- malicious or compromised ring-3 applications;
- malicious inputs to system calls and service protocols;
- compromised user-space drivers;
- malformed disk, filesystem, ELF, ACPI, PCI, VirtIO, and network data;
- stale, replayed, or cross-generation service messages;
- resource-exhaustion attempts within documented limits; and
- ordinary process/server/driver crashes.

The initial claim excludes:

- malicious OVMF, QEMU, KVM, host kernel, proof tools, compiler, or linker;
- malicious or faulty CPU/IOMMU implementation;
- physical attacks;
- all timing, cache, branch-predictor, and power side channels;
- denial of service by the host or hardware;
- availability after kernel invariant failure; and
- rollback of the entire virtual disk by a malicious host.

Exclusions are manifest assumptions, not reasons to omit defensive validation.

## 2. Protected assets

- kernel code, data, stacks, page tables, and proof metadata;
- confidentiality and integrity of each protection domain’s frames;
- capability authority;
- saved thread contexts;
- IPC payloads and transferred capabilities;
- DMA isolation;
- persistent filesystem integrity subject to the storage assumptions;
- credentials and process-control state;
- cryptographic keys and entropy state; and
- release evidence and image identity.

## 3. Isolation properties

For any two domains `A` and `B`, absent an explicit shared-frame or IPC
capability:

- `A` cannot read or modify `B`’s frames through CPU mappings;
- `A`’s device cannot DMA into `B`’s frames;
- `A` cannot send to or receive from `B`’s endpoints;
- `A` cannot control `B`’s threads, mappings, IRQs, timers, or devices;
- a stale capability or service token cannot acquire authority after object
  reuse; and
- failure of `A` cannot mutate kernel state except through a validated operation.

This is an access-control and integrity property, not a timing noninterference
claim.

## 4. Boot integrity modes

Development mode:

- accepts locally built unsigned images;
- enables `DebugPort`, fault injection, and verbose serial logs;
- records `development=true` in `BootInfo` and the release manifest.

Production mode:

- OVMF Secure Boot is configured with pinned keys;
- the UEFI loader is signed;
- the loader verifies kernel, service bundle, policy manifest, and initial
  filesystem digests;
- debug authority is absent unless explicitly provisioned; and
- image and policy generations enforce a configured anti-rollback rule.

Secure Boot does not make firmware trusted code verified; it authenticates the
selected code chain.

## 5. Least authority

- Applications receive service endpoints and their own memory/thread
  capabilities only.
- Filesystem servers receive block-partition authority, not arbitrary devices.
- Drivers receive one device, its BARs/IRQs, an IOMMU domain, and bounded DMA
  frames.
- `netd` receives packet-buffer and network-driver endpoints, not PCI access.
- `procd` receives thread/process control authority but no device MMIO.
- `pagerd` receives controlled frame/VSpace authority but no process credentials.
- `init` retains root policy authority and is separately hardened and audited.

Authority grants are declared in the service manifest and checked against a
deny-by-default schema.

## 6. Memory hardening

- NX and W^X are mandatory.
- SMEP and SMAP are mandatory.
- Null and stack-guard pages remain unmapped.
- Kernel/user canonical ranges are checked before page-table operations.
- User contexts cannot set privileged RFLAGS or selectors.
- Freed cross-domain memory is zeroed.
- FPU and unspecified register state are cleared or restored, never leaked.
- Kernel stacks have guard pages and are not reused cross-CPU without clearing.
- Read-only boot data is remapped read-only before starting user code.

KASLR and KPTI are later defense-in-depth features. Their absence is documented;
they are not substitutes for capability and mapping proofs.

## 7. Input validation

Every parser follows length-first validation:

1. validate outer buffer bounds;
2. validate fixed header;
3. check every offset plus length for overflow and containment;
4. bound counts before allocation or loops;
5. validate alignment and enumerations;
6. reject overlapping regions where the format forbids them;
7. create typed views only after checks; and
8. preserve unknown-version data only when the protocol explicitly permits it.

Parsers for ACPI, ELF, VirtIO, TMFS, RPC, and packets have structured fuzz targets
and negative golden corpora.

## 8. Resource denial of service

Kernel transitions are bounded. Protection-domain quotas cover memory, objects,
caps, mappings, pinned DMA, threads, IPC queue nodes, timers, and outstanding
faults.

Servers enforce client quotas for:

- descriptors and processes;
- path depth and component length;
- filesystem transactions and dirty bytes;
- socket count and buffered data;
- connection tables;
- service requests and retry cache; and
- log rate.

Critical recovery paths use reserved memory, capabilities, and queue entries that
ordinary clients cannot consume.

CPU denial of service is mitigated with quanta and priority authorization.
Unbounded higher-priority workload can still starve lower priorities; that is an
explicit scheduling-policy limitation.

## 9. Driver compromise

The driver threat model assumes arbitrary code execution inside the driver
process. Protection relies on:

- ring-3 execution;
- minimal VSpace mappings;
- capability-limited MMIO/IRQ/device operations;
- VT-d DMA and interrupt remapping;
- checked PCI configuration operations;
- mask-until-ack interrupts;
- device-reset authority held by `devmgr`, not the driver;
- bounded shared DMA pools; and
- full revocation and IOTLB invalidation on restart.

Without active VT-d, untrusted drivers are not started. A diagnostic development
mode MAY run a driver with bounce buffers in a trusted I/O broker, but that mode
does not carry the normal isolation claim.

## 10. Cryptographic randomness

Production entropy comes from an isolated VirtIO RNG driver and entropy service,
optionally mixed with a validated UEFI RNG seed. The service:

- tracks whether it has reached the minimum credited entropy threshold;
- uses a standard cryptographic DRBG;
- never counts deterministic timing values as entropy;
- reseeds after driver restart; and
- blocks strong-random requests until initialized.

TCP sequence state, ASLR when added, key generation, and `/dev/random` use this
service. `/dev/urandom` behavior before initialization is documented and fails
closed in production.

## 11. Service failure and restart

Every service request and handle is generation tagged. Restart invalidates old
generations and wakes blocked callers.

Recovery policy:

| Component | Policy |
|---|---|
| ordinary application | terminate; report to parent |
| shell | restart on console |
| network stack | restart; sockets fail with reset/error |
| network driver | revoke/reset/restart |
| block driver | reset/restart; unresolved I/O is error/replayed only if idempotent |
| filesystem | remount/recover journal; enter read-only mode on corruption |
| VFS/process/pager | coordinated restart only from durable/checkpointed state; otherwise fail-stop user environment |
| `init` | fail-stop and reboot |
| kernel | stop all CPUs and emit crash record |

The kernel does not promise transparent recovery from stateful service failure.
It promises containment and explicit outcomes.

## 12. Storage integrity

TMFS protects against torn, reordered, and partially completed writes under the
assumption that VirtIO flush completion is honored by QEMU/host storage.

It does not protect against a malicious host replaying an older whole-disk image.
Production anti-rollback requires an external monotonic counter or remote
attestation service and is outside the first platform claim.

## 13. Security testing

Required campaigns include:

- syscall and RPC structure fuzzing;
- ACPI/ELF/PCI/VirtIO/TMFS/network parser fuzzing;
- capability forgery, stale generation, rights escalation, and revocation tests;
- page-table and IOMMU differential tests;
- driver process compromise with arbitrary MMIO/DMA attempts;
- interrupt storm and malformed completion injection;
- memory and quota exhaustion;
- power-loss injection at every TMFS transaction write;
- process/server/driver crash at every protocol phase;
- speculative-execution configuration audit; and
- proof-cheat scanner mutation tests.

Security bugs that reveal a missing invariant require a design/proof update, not
only a patch to the observed path.
