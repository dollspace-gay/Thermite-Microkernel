# M1 exception-dispatch policy

Status: **accepted M1 subcomponent** against public Thermite candidate commit
`1fb0a799071d35493815ba99b9ca26af9a22eb1c`. This checkpoint proves the pure
dispatcher state/action transition. It does not supply the concrete dispatcher
body called by the common-entry capsule and executes no machine actions.

## Accepted implementation

`thermite/platform/exception_policy.th` defines one total transition over a
normalized `ExceptionEvent` and bounded scalar `ExceptionState`. The state owns
fault generations, timer/IRQ/quarantine/spurious counters, the most recent TLB
epoch, reschedule state, and a fail-stop latch. The action algebra contains:

- generation-tagged fault delivery and endpoint-less thread termination;
- timer recording and reschedule;
- masked device notification;
- new and stale TLB shootdown classification;
- quarantine and spurious accounting; and
- reason-tagged fail-stop panic.

The success contract is exact for every action. A user page fault must carry the
user bit, only the supported present/write/user/instruction bits, CR2, decoded
read/write/execute access, and the observed VSpace epoch. User delivery advances
the fault generation once; termination does not. Kernel exceptions and vectors
2, 8, and 18 fail-stop. Invalid frame/vector/thread state, corrupt page-fault
metadata, operational-counter exhaustion, a zero shootdown epoch, and vector
`0xe3` each have a distinct panic reason. A panic preserves accounting fields,
clears reschedule, and latches fail-stop.

Timer and reschedule IPIs request rescheduling. TLB epoch advancement is
monotonic and stale nonzero requests preserve state. Bound vectors in
`0x30..0xdf` increment IRQ delivery and require `masked=true` in the notification
action; unbound device, legacy, and reserved vectors increment quarantine.
Vector `0xff` increments the spurious count. All incrementing paths fail-stop
before the operational cap, so no executable addition can wrap.

## Acceptance command

```text
cargo run -p xtask -- m1-exception-policy
```

The command validates the exact Forge candidate pin and generated skill, audits
the L3/end-to-end/no-slag source, and requires a non-vacuous 64/64 mutation
battery. It produces three byte-identical combined sources, receipts, verified
rlibs, and kernel-vstd dependencies. Receipt validation, replay, and both
reproduction validations pass.

The separately compiled hosted consumer executes the proved observation over 18
scenarios: user page fault, user termination, corrupt page fault, kernel page
fault, double fault, timer, reschedule, bound IRQ, unbound IRQ, new shootdown,
stale shootdown, stop IPI, spurious vector, invalid frame, invalid vector,
missing thread, counter exhaustion, and already-latched panic. Its marker is:

```text
M1_EXCEPTION_POLICY_OK observation=262143 scenarios=18
```

A `no_std` rlib and higher-half ELF also link. The ELF has no undefined symbols.
Forge-generated enum/state moves use the already accepted M0 platform primitive
object; the acceptance harness rechecks that object/report against the current
verified sources and linker script, then extracts the final linked bytes. The
selected `memcpy` is exactly the registered nine-byte capsule with SHA-256
`00d0174466d21d8a224c588f4bf2e324a605806fac0cb0d4fa2ba1a9667a49a9`.

Four changed-proof shells—wrong observation, page access, IRQ mask, and stop
reason—fail atomically. Receipt, kernel-vstd source, and kernel-vstd rlib tamper
cases are rejected.

## Stable result

```text
M1_EXCEPTION_POLICY_OK
component_verified=true
release_eligible=false
candidate_pin_verified=true
dispatcher_machine_actions_executed=false
receipt_validated=true
receipt_replayed=true
source_sha256=f5599ebc5fd6a5e39445028ba4d55f8980847b3456fb36b6336636670880a64e
shell_sha256=27ba08dd346d699f91498a39e57457138a1efa0126206ac51b2fcc8a15396817
consumer_source_sha256=02ee8d663c391f9d1f31f82a9d8a74591a076e4e71a2a54cefe59fccf205f706
freestanding_source_sha256=9ddb8029d3012faf7dc71df185c67e6697a734eb4c106f91c198d1f4c3c86e6d
combined_source_sha256=17957e445ee36580c130715de5b58abd0122aed605c67a23942f91153b418b7e
receipt_sha256=c4bc6afa9effb2fc556d132bc16ed71dd8a5a94daadd3b25850680df27cc279e
binding_sha256=fc3ade2e024c55b3bf075aefd8446af705e6d86821eeda7c8bfeacd20ebc1419
artifact_sha256=2c2e0999f82fca2977df7102d7cecb8dfc30f3ffab350d4d28de492a8f88b1fa
consumer_sha256=b9d4ac81a15c97d84a90b7b024b2c33c89c166c533a5c0d6cf8a9fb7200b471a
freestanding_rlib_sha256=2ca9054a3d91790046baff5da5691eb6fea77ff8537a8ecc4a644c52484e4d42
freestanding_elf_sha256=6b8b99a94f9b81a0187d5e74ec04245dcfd89c3554537e9b18ed26d00533ea0d
platform_primitive_object_sha256=a3884a20bfb8193e6cfdbf921eae60bac038406aebfe9184ad5039e0629ec50f
linked_memcpy_sha256=00d0174466d21d8a224c588f4bf2e324a605806fac0cb0d4fa2ba1a9667a49a9
forge_source_identity=1fb0a799071d35493815ba99b9ca26af9a22eb1c
forge_sha256=12240457546220ebefba7c7a5e3ab2d127acaf9b592543a8d0394bf0c8253b74
reproducibility_builds=3
mutation_battery=64/64
scenarios=user-page-fault,user-terminate,corrupt-page-fault,kernel-page-fault,double-fault,timer,reschedule,bound-irq,unbound-irq,new-shootdown,stale-shootdown,stop-ipi,spurious,bad-frame,bad-vector,missing-thread,counter-overflow,latched-panic
freestanding_links=rlib,elf64-x86-64
freestanding_dependencies=verified-m0-memcpy
runtime_marker=M1_EXCEPTION_POLICY_OK observation=262143 scenarios=18
negative_cases=wrong-observation,wrong-page-access,wrong-irq-mask,wrong-stop-reason,receipt-tamper,vstd-source-tamper,vstd-rlib-tamper
```

Generated proof, replay, runtime, link, and negative evidence remains under
ignored `build/m1-exception-policy/`.

## Remaining boundary

The accepted transition begins after a frame has been normalized and validated.
It does not decode the concrete stack frame, acquire the kernel lock, resolve the
current thread or fault endpoint, allocate a linear fault-reply token, mutate
scheduler queues, or execute platform actions. Its single TLB epoch is a policy
classifier, not the later per-VSpace/per-CPU shootdown proof. IRQ and timer
actions do not mask, acknowledge, EOI, or program LAPIC hardware.

The next joined entry gate must connect the accepted stub table and common-entry
capsule to a verified concrete dispatcher bridge, prove state/frame ownership
and returning versus non-returning behavior, execute each action through proved
platform capsules, and run representative CPL0/CPL3 exceptions, IPIs, and IRQs
under QEMU. Until then `dispatcher_machine_actions_executed=false` and
`release_eligible=false` remain explicit.
