# M1 scalar exception bridge and entry capsule

Status: **accepted M1 subcomponent** against public Thermite `main` commit
`b8dc3947f504454775aa70977d8bda5da677d2af`. This checkpoint implements the
verified policy/action core behind the six-scalar dispatcher ABI and supplies
the exact fixed-address entry capsule. It does not yet implement the per-CPU
lookup wrapper or link the compiled core body at its registered address.

## Accepted implementation

`tests/m1/exception_scalar_shell.rs` is a strict same-crate direct-Verus shell
over `thermite/platform/exception_policy.th`. It validates three boundaries
before an action can commit:

1. The per-CPU snapshot has a unique state token, the kernel lock, masked
   interrupts, the expected current thread, and available scheduler/crash
   records.
2. The six transported scalar values exactly match the already validated
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

## Exact scalar entry

`verus/machine-model/exception_scalar_entry_capsule.rs` registers these eight
bytes at `0xffffffff80011200`:

```text
48 89 df e9 f8 00 00 00    mov rdi,rbx; jmp 0xffffffff80011300
```

The accepted dispatcher front arrives with RBX holding its registered frame
pointer, while RDI contains a redundant CR2 copy. The capsule replaces only RDI
with the frame pointer, preserves error/RIP/RFLAGS/user-RSP/metadata in
RSI/RDX/RCX/R8/R9, changes no stack memory, and tail-jumps to the scalar-core
seam. Control 0 returns through the inherited common continuation at
`0xffffffff80011038`; controls 1 and 2 are respectively scheduling and
fail-stop nonreturning paths.

The direct Verus model proves 11 whole-crate obligations. Three model builds,
three consumers, and three links reproduce. The post-link audit requires one
eight-byte executable section, no relocations, entry address
`0xffffffff80011200`, registered core symbol `0xffffffff80011300`, and the exact
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
per_cpu_lookup_wrapper_present=false
scalar_core_fixed_address_linked=false
receipt_validated=true
receipt_replayed=true
source_sha256=f5599ebc5fd6a5e39445028ba4d55f8980847b3456fb36b6336636670880a64e
shell_sha256=e751eb3b0f36afd76eedf5a0b62a174f98fdfc0938051791e1ba8ac45fc7587f
consumer_source_sha256=cb4fcfc24abb7fc29193dd113175e6754979894443f3ded3114bbccf6c950a86
combined_source_sha256=ca93dc4944fa3f516a605093845266b8f5bedd4521602c387e580061555313d3
receipt_sha256=c23604fc199fabc12e7bc334a7984326a466787682f098c7ffae7fd58039af73
binding_sha256=efa03dc6655d85dfb1356f2598eda7e4221ffdd50e82185da6124c2efea0d40d
artifact_sha256=6ab42ef1c88c182520c632518ecf4048b449bdf9bb46b8a06929aa98f9c92832
core_consumer_sha256=a5426cb4892dc535337666efe90446a79167390ce5cc0ee43419ac122b37cac6
entry_source_sha256=4776ea97cc8e981460370360355e58ee0a5809246000f992057c5075fb1b3b04
entry_consumer_source_sha256=f3dcbb6b046a45bf1ff27fdc4161b43fec020be9d167e081cd68c0fb64e7a0eb
entry_linker_sha256=4d74b703242bf29104b34a5ec9e2aa4d1092f927611cdd4ad94150abc6d98b9f
entry_model_sha256=365532d6fd267b5671bfb1993b5a7479cd2279497fbf2db3b91440a0b28383d9
entry_consumer_sha256=155968b04fe5c6588bf2543c7cbb787eae5292b146d8f33a965fbe2ad2b775ce
emitted_entry_sha256=ab0c01dbc9d688c9c475dac85972c3c6ac1262ea3a568f1429a9041cdf3d88e4
linked_entry_sha256=ab0c01dbc9d688c9c475dac85972c3c6ac1262ea3a568f1429a9041cdf3d88e4
linked_entry_elf_sha256=3188d76db58115dad1cbe405c8d5f0fd4a34d17669fba53fb1297c284df18267
forge_source_identity=b8dc3947f504454775aa70977d8bda5da677d2af
forge_sha256=b073fa34a955dc4ce723aac3cdba36ed031e7daa1ca5db6ead866d41ef36fbf9
core_reproducibility_builds=3
entry_model_reproducibility_builds=3
entry_consumer_reproducibility_builds=3
post_link_reproducibility_builds=3
mutation_battery=64/64
verus_entry_verified=11
scalar_entry_virtual=ffffffff80011200
scalar_core_seam_virtual=ffffffff80011300
common_continuation_virtual=ffffffff80011038
scalar_entry_bytes=8
scalar_entry_instruction=mov-rbx-rdi;tail-jump
control_values=return:0,schedule:1,fail-stop:2
core_runtime_marker=M1_EXCEPTION_SCALAR_OK scenarios=11 observation=2047 controls=return,schedule,fail-stop actions=fault,terminate,timer,irq,tlb,quarantine,panic
entry_runtime_marker=M1_EXCEPTION_SCALAR_ENTRY_OK bytes=8 controls=return,schedule,fail-stop rejected=4 core=ffffffff80011300
core_runtime_scenarios=11
entry_runtime_controls=3
entry_runtime_rejections=4
negative_cases=argument-binding,policy-rollback,control-map,snapshot-reason,core-bad-assume,receipt-tamper,vstd-source-tamper,vstd-rlib-tamper,entry-frame-argument,entry-tail-transfer,entry-return-target,entry-bad-assume,byte-mutation,extra-byte,unregistered-executable
```

Generated evidence remains under ignored `build/m1-exception-scalar/`.

## Remaining boundary

The entry capsule targets a registered seam, not the compiled rich-state core.
The next implementation must provide a verified lower-TPL wrapper at
`0xffffffff80011300` that reads the per-CPU pointer from initialized kernel GS,
obtains the global-lock/unique-state token, turns the registered frame ownership
into the safe slice, snapshots context, and calls this core. Its schedule and
fail-stop controls must connect to nonreturning verified platform paths, while
return control must preserve RBX/frame state and reach the common continuation.

After that wrapper exists, the project can link the complete stub/common/front/
entry/core path, bind the real scheduler, IRQ, TLB, and crash backends, and run
CPL0/CPL3 exceptions, timer IPIs, and device IRQs under QEMU. Until then
`hardware_executed=false`, `per_cpu_lookup_wrapper_present=false`,
`scalar_core_fixed_address_linked=false`, and `release_eligible=false` are
deliberate.
