# POSIX personality

## 1. Compatibility model

POSIX is implemented by TMK libc plus user-space servers. Kernel system calls are
not POSIX calls.

Compatibility means applications can be recompiled from source against TMK
headers and libc and observe the documented POSIX.1-2024 semantics. It does not
mean Linux binaries run unchanged.

The libc baseline is a pinned musl port with a TMK-native syscall/service backend.
Musl is user-space code and is not added to the kernel isolation TCB.

## 2. Profiles

### P0: bring-up

- static ELF64 loading;
- `_exit`, basic process identity, `write` to console;
- anonymous memory allocation;
- monotonic time and sleep;
- minimal directory/file read from initramfs.

### P1: useful release

- `fork`, `execve`, `posix_spawn`, `waitpid`, process groups, sessions;
- credentials and basic owner/group/mode checks;
- signals required by the shell and ordinary utilities;
- file descriptor/open-file-description semantics;
- regular files, directories, symlinks, pipes, terminal devices;
- `openat`, `close`, `read`, `write`, `pread`, `pwrite`, `lseek`;
- `fstatat`, `mkdirat`, `unlinkat`, `renameat`, `linkat`, `symlinkat`;
- `dup`, `dup2`, `fcntl`, `pipe`, `poll`;
- `fsync`, `fdatasync`, `sync`;
- `mmap`, `munmap`, `mprotect`, `brk`;
- clocks, timers, and sleep;
- IPv4 ICMP/UDP/TCP sockets; and
- a shell and core utilities sufficient for the acceptance scenario.

### P2: extended POSIX

- pthreads and process-shared synchronization;
- complete multi-threaded signal semantics;
- dynamic linking;
- UNIX-domain sockets and descriptor passing;
- asynchronous I/O;
- POSIX shared-memory and semaphore objects;
- IPv6;
- locale expansion; and
- additional POSIX option groups selected by a conformance profile.

No document or header may advertise P2 support while only P1 is implemented.

## 3. Service mapping

| POSIX concept | Owning component |
|---|---|
| PID, parent/child, credentials, session, signals | `procd` |
| address spaces, COW, ELF, `mmap` | `pagerd` |
| file descriptor table and open file descriptions | `vfsd` |
| path namespace, permissions, pipes | `vfsd` |
| persistent inode/data operations | `tmfsd` |
| sockets and protocol state | `netd` |
| terminal sessions and line discipline | `termd` |
| monotonic primitive | kernel timer |
| realtime clock policy | `timed` |
| libc buffering, TLS, cancellation | libc |

Kernel capability badges authenticate callers to servers. Servers do not trust a
PID supplied in an RPC payload.

## 4. Processes

`procd` owns a `Process` record:

- PID generation and parent;
- credentials;
- process group/session;
- child and wait status;
- signal dispositions and pending sets;
- references to kernel quota domain, VSpace, CSpace, threads, descriptor table,
  and current directory;
- resource limits; and
- lifecycle state.

PIDs are POSIX names, not authorization handles. Internal references include a
generation so PID reuse cannot confuse late messages.

### 4.1 `fork`

P1 `fork` supports a single-threaded caller. It:

1. quiesces the process at a libc safe point;
2. asks `procd` to reserve child identity and resources;
3. asks `pagerd` to construct parent/child COW mappings;
4. clones descriptor-table references so open file descriptions remain shared;
5. clones signal dispositions and credentials according to POSIX rules;
6. creates a child thread with copied register/TLS state and zero return value;
7. commits both service transactions; and
8. rolls back all reservations on pre-commit failure.

P2 implements POSIX multi-threaded `fork`, where only the calling thread appears
in the child, plus `pthread_atfork` behavior.

### 4.2 `execve`

`execve` is transactional:

- `pagerd` validates and stages the ELF image, stack, TLS, and arguments;
- `procd` checks credentials and serializes process state;
- `vfsd` applies close-on-exec;
- signal dispositions reset as specified; and
- the old VSpace is replaced only after the new image is ready.

Failure leaves the old process image running. Success does not return.

## 5. File descriptors

Each process has a descriptor table mapping small integers to references and
descriptor flags. An open file description is a separate server object holding:

- target object;
- current offset;
- status flags;
- access mode;
- reference count; and
- advisory/record-lock state where supported.

`dup` and `fork` share the open file description, including its offset. Separate
`open` calls receive distinct descriptions. Descriptor and description locking
must preserve atomic offset update for `read`/`write`.

## 6. Filesystem semantics

P1 provides:

- one rooted namespace with mount points;
- `.` and `..`;
- hard links, symlinks, link counts;
- owner/group/mode access checks;
- atomic rename within one filesystem;
- unlink-while-open;
- per-open offsets;
- durable `fsync`/`fdatasync`;
- stable inode identity during one mount; and
- documented timestamp resolution.

Path traversal is entirely in `vfsd` using directory capabilities returned by
filesystem servers. RPCs pass `(directory_handle, component)` rather than an
unchecked full path, preventing confused-deputy traversal.

The first release does not claim ACLs, extended attributes, quotas exposed through
POSIX, or cross-filesystem atomic rename.

## 7. Signals

`procd` owns signal policy; the kernel supplies thread suspension, fault delivery,
notifications, and register update.

P1 supports the signal set needed by the shell and standard process control,
including `SIGCHLD`, `SIGINT`, `SIGTERM`, `SIGKILL`, `SIGSTOP`, `SIGCONT`,
`SIGPIPE`, synchronous fault signals, and timer signals.

Delivery:

1. `procd` selects an eligible thread and disposition;
2. `pagerd`/libc validate a writable signal frame and trampoline;
3. the kernel installs validated user registers through a fault/control
   capability;
4. `sigreturn` validates the entire restored frame; and
5. unmaskable signals cannot be caught or blocked.

A malformed signal frame terminates the process. It cannot install kernel
selectors, noncanonical addresses, or privileged flags.

## 8. Threads and synchronization

P2 maps each pthread to a kernel thread. Uncontended mutexes and atomics remain
user-local. Contended waits use a versioned `syncd` protocol:

- client atomically records a wait sequence;
- `syncd` queues by `(process_generation, user_key, sequence)`;
- unlock/wake is idempotent;
- sequence comparison prevents lost wakeups; and
- priority-sensitive waits use an IPC endpoint so kernel donation applies.

Robust mutex owner death is reported when `procd` observes thread termination.

## 9. Clocks

The kernel provides monotonic deadlines and nanosecond conversion. `timed`
maintains realtime offset and adjustment policy.

- `CLOCK_MONOTONIC` never moves backward.
- `CLOCK_REALTIME` may be set by authorized policy.
- relative sleep uses monotonic deadlines.
- timeout RPCs include absolute monotonic deadlines to avoid restart extension.

## 10. Sockets

`netd` owns POSIX socket objects. P1 supports:

- IPv4 datagram and stream sockets;
- `bind`, `connect`, `listen`, `accept`;
- `send`, `sendto`, `recv`, `recvfrom`;
- `shutdown`;
- nonblocking mode;
- `poll`;
- basic socket options; and
- ICMP echo used by `ping`.

Data uses shared frame rings for large transfers and IPC fast words for control.
Network names and DNS are an optional user-space resolver; acceptance can use a
literal host address.

## 11. Terminal and shell

`termd` provides canonical/raw modes, foreground process groups, echo, and control
character to signal translation. The shell uses ordinary POSIX process, pipe,
descriptor, and signal interfaces; it does not receive special kernel privileges.

## 12. Conformance policy

The repository maintains a generated matrix for every advertised interface:

- POSIX reference section;
- profile stage;
- implementation owner;
- supported flags/options;
- required error cases;
- semantic tests;
- known deviations; and
- assurance level.

The image exposes the same matrix through `sysconf`/documentation. A full
POSIX.1-2024 conformance claim requires completion of the selected option groups
and an external conformance campaign; P1 alone is called “the TMK P1 POSIX
profile,” not “fully POSIX compliant.”
