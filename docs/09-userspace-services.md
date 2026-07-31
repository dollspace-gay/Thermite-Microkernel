# User-space services and drivers

## 1. Service architecture

`init` starts services from a signed manifest. Each entry specifies:

- executable digest;
- protocol major/minor;
- initial capabilities;
- memory/object/CPU quotas;
- dependencies;
- restart policy;
- health-check deadline; and
- whether loss is fatal, degraded, or recoverable.

The service directory returns endpoint capabilities, not global numeric ports.
Each restart creates a new endpoint generation. Existing callers receive
`K_E_CANCELLED` and reconnect through the directory.

## 2. Initial authority

The root task initially receives:

- untyped RAM excluding kernel reserves;
- device and IOMMU-root authority;
- IRQ-root authority;
- boot-module frames;
- debug authority in development images; and
- control capabilities for its precreated domain, VSpace, CNode, and thread.

`init` immediately partitions authority. No ordinary service retains the root
capabilities from which unrelated device, memory, or IRQ authority could be
derived.

## 3. Service protocols

Protocols use the message tag and versioned payload rules from the native ABI.
Every request includes:

- client-generated 128-bit request ID for side-effecting operations;
- absolute monotonic deadline;
- protocol minor version;
- flags;
- lengths for each variable region; and
- explicit shared-frame grants.

Servers cache completed side-effecting request IDs within a bounded window so a
client can safely resolve “reply lost during restart” without duplicating writes.
When certainty is impossible, the result is explicit `EIO`, never an invented
success.

## 4. Device manager

`devmgr`:

- enumerates PCIe through its ECAM capability;
- validates BAR size/alignment and capabilities;
- enables only modern VirtIO functions in P1;
- asks the kernel to create one VT-d domain per driver;
- grants BAR, MSI-X, device-reset, and domain capabilities;
- starts drivers through `init`; and
- revokes/quiesces authority before restart.

PCI configuration writes are restricted by a typed kernel operation. A driver
cannot reprogram another function or disable the IOMMU.

## 5. VirtIO block driver

`virtioblkd` supports VirtIO 1.3 modern PCI transport with:

- feature negotiation and device-status state machine;
- one split virtqueue of default size 256 in P1;
- MSI-X;
- IOMMU-translated DMA buffers;
- read, write, flush, and device-ID requests;
- checked descriptor-chain construction;
- bounded outstanding requests;
- timeout and reset recovery; and
- deterministic request completion/cancellation.

Packed queues, discard, write-zeroes, and multiqueue are later negotiated
features. Unsupported device-offered features are masked; required missing
features fail device start.

Each descriptor chain proves:

- indices lie within the negotiated queue;
- no descriptor cycle;
- total length does not overflow;
- device-readable/device-writable permissions match request direction;
- every DMA interval is mapped in the driver’s IOMMU domain; and
- completion consumes exactly one outstanding request generation.

## 6. VirtIO network driver

`virtionetd` supports:

- one RX and one TX split queue in P1;
- MSI-X;
- fixed-size frame pools;
- receive replenishment;
- MAC discovery;
- checked VirtIO network headers;
- bounded TX completion; and
- reset/restart with all DMA mappings revoked.

Checksum, segmentation, mergeable-buffer, control-queue, and multiqueue offloads
are disabled in P1 to keep the first data path auditable. `netd` receives complete
Ethernet frames in shared buffers and returns buffer ownership explicitly.

## 7. Network stack

`netd` implements:

- Ethernet II;
- ARP with bounded cache and expiry;
- IPv4 parsing, fragmentation rejection in P1, and checksum validation;
- ICMP echo;
- UDP;
- TCP with bounded connection tables, retransmission, congestion control, and
  timeout state;
- socket queues, nonblocking semantics, and poll notifications; and
- static configuration plus optional DHCP client.

All packet parsers are length-first and treat packets as untrusted byte slices.
No parser creates references before validating header containment.

The acceptance profile uses a test-owned isolated peer with a literal address, so
DNS and external Internet reachability are not needed to pass.

## 8. Entropy driver and service

`virtiorngd` owns the modern VirtIO RNG function and transfers device bytes to
`rngd`. The driver has no authority over consumer address spaces. `rngd`:

- mixes independent boot and device inputs;
- maintains a standard cryptographic DRBG;
- credits only approved entropy sources;
- exposes capability-scoped random streams;
- reseeds after driver restart; and
- fails strong requests until initialization.

## 9. Block service

A block broker sits between filesystems and `virtioblkd`. It provides:

- sector-aligned read/write;
- flush barriers;
- partition capability slicing;
- request ordering;
- client quotas; and
- driver-restart recovery.

Filesystems receive capabilities to bounded partitions, not to the whole disk.

## 10. TMFS persistent filesystem

TMFS is a user-space, 4 KiB-block, journaled filesystem designed for the P1 POSIX
profile.

### 10.1 On-disk structures

- two checksummed superblock slots with monotonically increasing generations;
- allocation bitmap and free-space summary;
- fixed-size inodes with generation, mode, owner, times, size, links, and extent
  root;
- checksummed B+trees for directory entries and extents;
- an append-only metadata journal;
- transaction commit records with checksums and sequence numbers; and
- orphan records for unlink-while-open recovery.

Every on-disk pointer is range/alignment checked before use. Counts and sizes use
checked arithmetic. A corrupt structure returns a filesystem error and does not
become a raw pointer or allocation size.

### 10.2 Transaction protocol

For one metadata transaction:

1. reserve journal and destination blocks;
2. write new data blocks;
3. append metadata redo records;
4. issue a block flush;
5. append one checksummed commit record;
6. issue a second flush;
7. publish the in-memory committed generation;
8. checkpoint metadata lazily; and
9. free old blocks only after the checkpoint is durable.

Recovery selects the newest valid superblock, scans complete journal records, and
replays only transactions with valid commit records. Replay is idempotent.

`fsync(fd)` commits the file’s data and required metadata through the second
flush. `rename` is one metadata transaction and is atomic after recovery.

### 10.3 Filesystem proof targets

Selected TMFS state machines are verified in Thermite/Verus:

- allocator non-double-allocation;
- inode link-count/orphan invariants;
- directory uniqueness;
- transaction write ordering;
- recovery idempotence;
- committed-data reachability;
- no reuse before durable checkpoint; and
- `fsync`/rename crash semantics.

The VirtIO device and host storage are assumed to honor negotiated flush ordering.
That assumption appears in the release manifest.

## 11. Process server and pager

`procd` and `pagerd` use a prepare/commit protocol for operations spanning both:

- fork;
- exec;
- process exit;
- signal-frame installation;
- credential-changing executable load; and
- address-space teardown.

Reservations are generation-tagged and expire. A crash before commit is rolled
back by the surviving coordinator or `init`; a crash after commit is replayed
idempotently.

## 12. VFS

`vfsd` owns path traversal, descriptors, mounts, pipes, and open file descriptions.
Filesystem-server object handles are capabilities or unforgeable generation-tagged
tokens scoped to the authenticated VFS endpoint.

Pipes are bounded shared-memory rings with VFS-managed endpoints. Writer closure
and reader closure produce POSIX EOF/`SIGPIPE` behavior.

## 13. Terminal and shell

Early boot uses the serial `DebugPort`. Once `termd` starts, it owns the console
stream and implements terminal state. The P1 shell supports:

- foreground/background jobs;
- pipelines;
- redirections;
- environment variables;
- built-in `cd`, `exit`, `export`, and service status; and
- external utilities used by acceptance.

The shell receives no device, IRQ, or physical-memory capability.

## 14. Driver restart

Restart ordering is:

1. mark service generation unavailable;
2. cancel endpoint waits and outstanding client requests;
3. mask device interrupts;
4. disable bus mastering;
5. reset or quiesce the device;
6. remove IOMMU mappings and invalidate IOTLBs;
7. revoke BAR/IRQ/domain capabilities;
8. destroy the old driver VSpace;
9. start a fresh driver with a new endpoint generation;
10. renegotiate the device; and
11. allow clients to reconnect and retry only idempotent requests.

A storage-driver crash during a committed TMFS transaction is recovered through
the journal; it is not treated as successful merely because the client sent the
request.

## 15. Observability

`logd` receives structured records with monotonic timestamp, service generation,
severity, event ID, and bounded payload. Kernel logs use a fixed per-CPU ring and
are drained through a capability.

Debug images expose QEMU exit codes, serial logs, trace counters, and fault
injection controls. Production images omit or capability-gate them.
