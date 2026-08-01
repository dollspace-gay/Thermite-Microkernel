# M1 saved exception-frame decoder and policy bridge

Status: **accepted M1 subcomponent** against public Thermite `main` commit
`b8dc3947f504454775aa70977d8bda5da677d2af`. This checkpoint proves safe
normalization of the exact common-entry stack layout and invokes the accepted
exception policy in the same verified crate. It does not convert the raw RDI
pointer into a slice and executes no returned machine action.

## Accepted implementation

`tests/m1/exception_frame_shell.rs` supplies the direct-Verus half of a Forge
composition over `thermite/platform/exception_policy.th`. It accepts a bounded
`&[u64]` in the exact address order created by the accepted common-entry bytes:

| Word | Byte | Meaning |
|---:|---:|---|
| 0 | 0 | R15 |
| 1..7 | 8..56 | R14, R13, R12, R11, R10, R9, R8 |
| 8..13 | 64..104 | RBP, RDI, RSI, RDX, RCX, RBX |
| 14 | 112 | captured CR2 |
| 15 | 120 | original RAX |
| 16 | 128 | vector |
| 17 | 136 | error code |
| 18 | 144 | resume RIP |
| 19 | 152 | resume CS |
| 20 | 160 | resume RFLAGS |
| 21 | 168 | user RSP, privilege transition only |
| 22 | 176 | user SS, privilege transition only |

Same-ring kernel entry must be exactly 21 words/168 bytes, use CS `0x08`, and
resume in the higher canonical half. User entry must be exactly 23 words/184
bytes, use CS `0x23`, resume RIP/RSP in the lower canonical half, and use SS
`0x1b`. Both paths require vector at most 255 and the common-entry restricted
RFLAGS mask with architectural bit 1 set.

The executable decoder checks length before every slice index. It derives CR2,
vector, error, and privilege origin only from the listed offsets and combines
them with a validated `DispatchContext`. The resulting `ExceptionEvent` is sent
directly to `exception_policy_step`. Any invalid layout has
`frame_valid=false`; the bridge proves that it returns a latched panic with
rescheduling clear and exact reason 2, except that an already latched state
retains precedence as reason 1.

## Acceptance command

```text
cargo run -p xtask -- m1-exception-frame
```

The command validates the exact Forge candidate and generated skill, requires
the non-vacuous 64/64 policy battery, and produces three byte-identical combined
sources, receipts, rlibs, and kernel-vstd dependencies. Receipt validation,
replay, and both reproduction validations pass.

The separately compiled consumer executes 12 scenarios: valid user page-fault
delivery, kernel timer, endpoint-less user termination, kernel page fault,
truncated user tail, wrong user SS, noncanonical user RSP, low kernel RIP,
invalid RFLAGS, selector/length mismatch, vector 256, and a short prefix. Its
marker is:

```text
M1_EXCEPTION_FRAME_OK words=21/23 scenarios=12 observation=4095
```

A `no_std` rlib and higher-half ELF link. The ELF has no undefined symbols and
its compiler-emitted copies resolve to the accepted M0 primitive object; the
linked nine bytes are re-extracted and match the verified `memcpy` capsule.
Four direct-Verus changes—prefix length, user selector, short-vector result, and
fail-stop reason—fail atomically. Receipt, kernel-vstd source, and kernel-vstd
rlib tampering are rejected.

## Stable result

```text
M1_EXCEPTION_FRAME_OK
component_verified=true
release_eligible=false
candidate_pin_verified=true
raw_pointer_bridge_present=false
dispatcher_machine_actions_executed=false
receipt_validated=true
receipt_replayed=true
source_sha256=f5599ebc5fd6a5e39445028ba4d55f8980847b3456fb36b6336636670880a64e
shell_sha256=454c525f28d9609e6d34986fe68797856beb8eb723eb20635c84e2df20d12210
consumer_source_sha256=af5ee2272f196b6f37b90b61153432a862a45591b263dd5ee675fa6f12f16bed
freestanding_source_sha256=fc28e999c211ef9817ddf152d1a88870d80fcfa5b97d479939abe54e82246eea
combined_source_sha256=fda31943706efcaf44928c5a0c60740b67e5a18ecde581993e714225287809fb
receipt_sha256=0d35ff07ddff15565306c404746d7cc034a6b6871a9fc60742f4d097daeac302
binding_sha256=ceea24c765dd449fee67dc65575b5389bb5ed7def634e5ed440a0386ea543034
artifact_sha256=895d1dc87b5329a43fc485c55069857b46614b0e40ffbbaf191fb9bc5a27e7fb
consumer_sha256=167e17320b0423045f1f7301c86f70d5b729a772f26ca89af474ec85a4a95eab
freestanding_rlib_sha256=fc018ce3249f40527ba096d87f82b6f000ba5ca5423016e8a35fa7ef1c8081c2
freestanding_elf_sha256=74cc44419b63cb7f8f7296be692b5d819ba39d2dc24ce18439d18a0e1c407737
platform_primitive_object_sha256=a3884a20bfb8193e6cfdbf921eae60bac038406aebfe9184ad5039e0629ec50f
linked_memcpy_sha256=00d0174466d21d8a224c588f4bf2e324a605806fac0cb0d4fa2ba1a9667a49a9
forge_source_identity=b8dc3947f504454775aa70977d8bda5da677d2af
forge_sha256=b073fa34a955dc4ce723aac3cdba36ed031e7daa1ca5db6ead866d41ef36fbf9
reproducibility_builds=3
mutation_battery=64/64
frame_layout_words=21,23
frame_layout_bytes=168,184
frame_offsets=r15:0,cr2:112,rax:120,vector:128,error:136,rip:144,cs:152,rflags:160,rsp:168,ss:176
runtime_marker=M1_EXCEPTION_FRAME_OK words=21/23 scenarios=12 observation=4095
runtime_scenarios=12
freestanding_links=rlib,elf64-x86-64
freestanding_dependencies=verified-m0-memcpy
negative_cases=prefix-length,user-selector,short-vector,panic-reason,receipt-tamper,vstd-source-tamper,vstd-rlib-tamper
```

Generated proof, replay, runtime, link, and negative evidence remains under
ignored `build/m1-exception-frame/`.

## Remaining boundary

The accepted decoder begins with a valid safe slice. It intentionally contains
no raw pointer or unsafe code. The concrete dispatcher must prove the RDI frame
pointer is aligned and owns the exact readable 168- or 184-byte range before
constructing that slice. It must also snapshot the current CPU/thread/endpoint/
IRQ context under the kernel lock, provide valid generations, execute each
returned action through proved platform operations, and distinguish returning
from fail-stop paths.

The accepted dispatcher front now discharges the conditional raw reads, and the
accepted scalar bridge checks those transported values against the safe frame
before invoking policy and its action model; see
[M1 scalar exception bridge](exception-scalar-bridge.md). A concrete per-CPU
lookup wrapper, fixed-address scalar core, real scheduler/LAPIC backends, full
stub-to-scalar link, and CPL0/CPL3 execution under QEMU remain open. This older
component report therefore keeps `raw_pointer_bridge_present=false`,
`dispatcher_machine_actions_executed=false`, and `release_eligible=false` as
historical component-scoped claims.
