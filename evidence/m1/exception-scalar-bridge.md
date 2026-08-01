# M1 scalar exception bridge and entry capsule

Status: **accepted M1 subcomponent** against public Thermite `main` commit
`b8dc3947f504454775aa70977d8bda5da677d2af`. This scoped checkpoint implements
the verified policy/action core behind the six-scalar dispatcher ABI and the
exact CR2-retaining fixed-address entry capsule. Its own report intentionally
ends at the core seam; the separately accepted
[per-CPU scalar-core wrapper](exception-scalar-core-wrapper.md) consumes it and
links the compiled adapter.

## Accepted implementation

`tests/m1/exception_scalar_shell.rs` is a strict same-crate direct-Verus shell
over `thermite/platform/exception_policy.th`. It validates three boundaries
before an action can commit:

1. The per-CPU snapshot has a unique state token, the kernel lock, masked
   interrupts, the expected current thread, and available scheduler/crash
   records.
2. The six transported scalar values exactly match the independently supplied
   21-word kernel or 23-word user frame. The frame remains the authority for
   CR2; the scalar copy is a cross-check.
3. The selected action's backend and counters can complete without partial
   state or arithmetic overflow.

The formal `MachineState` gives executable semantics to the complete accepted
action algebra. Fault delivery marks the current thread faulted, stores the
generation-tagged fault payload, and schedules. Endpoint-less faults terminate
and schedule. Timer/reschedule actions schedule. IRQ delivery masks, notifies,
optionally acknowledges, and either returns or schedules. TLB actions advance
or preserve the epoch and acknowledge. Quarantine masks and acknowledges;
spurious entry returns; panic fail-stops.

Action preflight is transactional. If a fault slot, IRQ/TLB backend, current
thread, or acknowledgement counter is invalid, the bridge latches fail-stop
reason 103 while preserving every pre-policy accounting field. Snapshot failure
uses reason 100 without invoking policy; scalar/frame disagreement uses reason
101 without invoking policy. Five direct proof mutations include the rollback
contract, so an uncommitted policy increment cannot escape into accepted state.

The adapter ABI is a named-field `ScalarCoreBlock` containing exactly 80 `u64`
slots. This avoids an unchecked slice or array ABI at the compiled boundary.
The verified adapter consumes one thin reference and writes its policy, machine,
and control outcome back into the same fixed block.

## Exact scalar entry

`verus/machine-model/exception_scalar_entry_capsule.rs` registers these eleven
bytes at `0xffffffff80011200`:

```text
49 89 fa                    mov r10,rdi
48 89 df                    mov rdi,rbx
e9 f5 00 00 00             jmp 0xffffffff80011300
```

The accepted dispatcher front arrives with RBX holding its registered frame
pointer and RDI containing transported CR2. The capsule first retains CR2 in
R10, then puts the frame pointer in RDI while preserving error, RIP, RFLAGS,
user RSP, and packed metadata in RSI, RDX, RCX, R8, and R9. It changes no stack
memory and tail-jumps to the scalar-core seam. This retained value lets the
downstream wrapper populate the register-transport slot independently of its
frame-memory read.

The direct Verus model proves 12 whole-crate obligations. Three model builds,
three consumers, and three links reproduce. The post-link audit requires one
eleven-byte executable section, no relocations, entry address
`0xffffffff80011200`, registered core symbol `0xffffffff80011300`, and exact
`mov`/tail-`jmp` disassembly. Four proof mutations, a byte mutation, an extra
byte, and an unregistered executable section are rejected.

## Acceptance command

```text
cargo run -p xtask -- m1-exception-scalar
```

The core path requires the L3/end-to-end/no-slag audit and non-vacuous 64/64
policy battery. Three strict Forge builds reproduce the combined source,
receipt, kernel rlib, and kernel-vstd dependency; validation and replay pass.
Three independently linked consumers execute 11 scenarios, including IRQ
backend failure and TLB acknowledgement overflow with policy rollback.

Receipt, kernel-vstd source, and kernel-vstd rlib tampering are rejected. In
total the gate rejects 15 proof, receipt, dependency, byte, size, and executable
section changes.

## Stable result

```text
M1_EXCEPTION_SCALAR_OK
component_verified=true
release_eligible=false
candidate_pin_verified=true
hardware_executed=false
policy_action_model_executed=true
fixed_address_scalar_entry_present=true
cr2_retained_in_r10=true
per_cpu_lookup_wrapper_present=false
scalar_core_fixed_address_linked=false
receipt_validated=true
receipt_replayed=true
source_sha256=f5599ebc5fd6a5e39445028ba4d55f8980847b3456fb36b6336636670880a64e
shell_sha256=7dbe3b3b111c6da707b4413e73f4b24dd159cce0eb8ec11c5a1c51fa607e26dd
consumer_source_sha256=cb4fcfc24abb7fc29193dd113175e6754979894443f3ded3114bbccf6c950a86
combined_source_sha256=2b31629311bbefa01e30e56c8ebd13993d2b6740b621fae0a448bbb705272b95
receipt_sha256=f02536e78a2c8c25fa738062825ee5d9dd0ec4e5e3bf932186103d9a74cdb49b
binding_sha256=744f6b59b5971805aa12711d51bdbbf66a33cb28092c6b88c726cd9e1ac7c57b
artifact_sha256=f7ab3f2f4909db60916a701a3f9d46808630f24f2ee1be89c70ff9aad9fef8bf
core_consumer_sha256=50d63e634a8511fe97c4112623653f74ea0212394776c5b586381d112e8e1020
entry_source_sha256=a0362bd89c332ad20836181a7e56b271ef68866af730aa0c8ed01a96660e1471
entry_consumer_source_sha256=eb69707434c42b128efa3360837c931bb89267b9b15659f7546bc7ab06ce1d18
entry_linker_sha256=95ed84f0a2269fed17ca96705c6cf836386cfec3818460416b085bcef478e75c
entry_model_sha256=459c20d71efa98c47fa79d0f6f8cca794ab1409e31e2716a64ca855ec8cb9546
entry_consumer_sha256=6492188c1453ea80226a061111c69e0b8de43f3ca8a5e6d22735926c0ac99184
emitted_entry_sha256=758d3e089fab8f88c03d0429113b6090c27c608112f0ebf9f47218445edda82e
linked_entry_sha256=758d3e089fab8f88c03d0429113b6090c27c608112f0ebf9f47218445edda82e
linked_entry_elf_sha256=31f7f294994adc21376556f7e0d916ae60e3a01bbe82318a6a60696cac49f9a0
forge_source_identity=b8dc3947f504454775aa70977d8bda5da677d2af
forge_sha256=b073fa34a955dc4ce723aac3cdba36ed031e7daa1ca5db6ead866d41ef36fbf9
core_reproducibility_builds=3
entry_model_reproducibility_builds=3
entry_consumer_reproducibility_builds=3
post_link_reproducibility_builds=3
mutation_battery=64/64
verus_entry_verified=12
scalar_entry_virtual=ffffffff80011200
scalar_core_seam_virtual=ffffffff80011300
common_continuation_virtual=ffffffff80011038
scalar_entry_bytes=11
scalar_entry_instruction=mov-rdi-r10;mov-rbx-rdi;tail-jump
control_values=return:0,schedule:1,fail-stop:2
core_runtime_marker=M1_EXCEPTION_SCALAR_OK scenarios=11 observation=2047 controls=return,schedule,fail-stop actions=fault,terminate,timer,irq,tlb,quarantine,panic
entry_runtime_marker=M1_EXCEPTION_SCALAR_ENTRY_OK bytes=11 controls=return,schedule,fail-stop rejected=4 core=ffffffff80011300
core_runtime_scenarios=11
entry_runtime_controls=3
entry_runtime_rejections=4
negative_cases=argument-binding,policy-rollback,control-map,snapshot-reason,core-bad-assume,receipt-tamper,vstd-source-tamper,vstd-rlib-tamper,entry-frame-argument,entry-tail-transfer,entry-return-target,entry-bad-assume,byte-mutation,extra-byte,unregistered-executable
```

Generated evidence remains under ignored `build/m1-exception-scalar/`.

## Scoped boundary

The false wrapper/link fields above describe this prerequisite command, not the
repository's later aggregate state. The follow-on wrapper gate validates GS and
block ownership, independently copies frame and register values, invokes this
exact receipt-bound adapter, and links it at a fixed address. Real scheduler,
IRQ/LAPIC/TLB/crash backends, full exception-image linkage, and live QEMU/IDT
delivery remain outside both checkpoints.
