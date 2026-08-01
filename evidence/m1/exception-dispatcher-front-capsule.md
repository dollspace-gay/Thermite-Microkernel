# M1 exception dispatcher-front capsule

Status: **accepted M1 subcomponent**. This checkpoint supplies the first exact
instruction image at the common-entry dispatcher address. It consumes the raw
frame pointer in RDI under explicit machine-memory ownership obligations and
passes six bounded scalar values to a registered verified seam. It does not yet
supply that scalar function, join the safe saved-frame decoder, or execute a
returned policy action.

## Accepted implementation

`verus/machine-model/exception_dispatcher_front_capsule.rs` registers the exact
93 bytes linked at `0xffffffff80011100` and models their x86_64 effect. The
capsule leaves the frame in R10 and loads:

| SysV argument | Saved-frame byte offset | Value |
|---|---:|---|
| RDI | 112 | captured CR2 |
| RSI | 136 | error code |
| RDX | 144 | resume RIP |
| RCX | 160 | resume RFLAGS |
| R8 | 168 when `CS & 3 == 3` | user RSP, otherwise zero |
| R9 | 128, 152, and conditionally 176 | vector, CS, and user SS packed as `vector | CS << 32 | SS << 48` |

The kernel path reads six frame words. The user path reads eight. The CS load
selects the path before either tail read, so same-ring frames never access RSP
or SS beyond their 168-byte extent. The only interprocedural transfer is a tail
jump to the registered scalar seam at `0xffffffff80011200`. The scalar inherits the
dispatcher-entry SysV RSP alignment and returns directly through the address
already placed by the common-entry call. The capsule writes neither frame nor
stack memory and touches no callee-saved register. Its
verified returning-caller contract requires CPL0, interrupt-gate IF clearing,
DF clear, RDI equal to a high-canonical registered frame base, readable prefix
memory, a readable user tail only for user origin, a registered readable return
address below and disjoint from the frame that equals the exact common
continuation `0xffffffff80011038`, dispatcher-entry RSP congruent to 8
modulo 16, and a registered returning scalar that preserves RBX and the frame.
The scalar consumes that return address, so the modeled continuation RSP is the
dispatcher-entry RSP plus eight.

The current model accepts only the exact kernel CS `0x08` or user CS `0x23`, a
vector at most 255, and a user SS value that fits the packed 16-bit field. RIP,
RFLAGS, user RSP, and exact user SS validation intentionally remain in the safe
decoder called by the next scalar bridge; malformed values are transported
without being blessed.

## Acceptance command

```text
cargo run -p xtask -- m1-exception-dispatcher-front
```

The command runs three whole-crate Verus proof/codegen builds with 22 proved
obligations and zero errors, compiles and executes three deterministic
consumers, and emits the registered instruction image from the proved
constants. Each consumer executes valid user and kernel paths plus rejection of
an unreadable prefix, unreadable user tail, absent scalar registration, RDI/base
mismatch, set DF, vector 256, a missing readable return address, a return target
other than the exact common continuation at `0xffffffff80011038`, a user SS too
large for the packed 16-bit field, misaligned dispatcher RSP, and a return
address that overlaps the frame. Its marker is:

```text
M1_EXCEPTION_DISPATCHER_FRONT_OK bytes=93 user_words=8 kernel_words=6 metadata=001b00230000000e scalar_entry_mod16=8 tail=1
```

Three fixed-address links reproduce byte-for-byte. Post-link extraction must
match the independent 93-byte acceptance constant; `readelf` must find no
relocations and exactly one executable section. The disassembly audit requires
all eight conditional loads, the metadata shifts/ORs, kernel R8 zeroing, and
exactly one tail jump to `0xffffffff80011200`, while rejecting push, pop, call,
return, and every callee-saved register name. Byte mutation and an extra
executable section are rejected. Eleven changed proofs—CR2, error, user RSP,
metadata, scalar address, frame preservation, stack alignment, tail transfer,
return target, read count, and an inserted `assume(false)`—fail atomically.

## Stable result

```text
M1_EXCEPTION_DISPATCHER_FRONT_OK
component_verified=true
release_eligible=false
hardware_executed=false
scalar_body_present=false
safe_decoder_joined=false
source_sha256=937e5017873e0cbed35554a9d121e4f739365a267e00767eda5a205b0f5e90bb
common_source_sha256=4c46a4107a9ae752e6ffbba4af33de1d2e422d3141189ef7012fd7211dc69da3
frame_source_sha256=454c525f28d9609e6d34986fe68797856beb8eb723eb20635c84e2df20d12210
consumer_source_sha256=31f445551d6fedb6d86310a0971225d453221255afaa9cd774b2feff0d774a20
linker_script_sha256=7873b9af5f7d2ddceaae46eec1ca0ee07f451e32c4b1bb17860229e6e9e253d5
model_artifact_sha256=27fa3fe3b3fee006c7bfdbee8ecad205a6a168262ea83b5975cbe4bd694f790c
consumer_sha256=329bdeaa3524e701badcbee1bd64e4ef6287adcb633d527309bea2733a475d7e
emitted_capsule_sha256=90d3b2fe61a8633f5e6f0cee32c20a22d7e01ea4cff7d6e3f2bc2123b86386bf
linked_capsule_sha256=90d3b2fe61a8633f5e6f0cee32c20a22d7e01ea4cff7d6e3f2bc2123b86386bf
linked_elf_sha256=b0c01b97aa140e4792447696614c70b44a9df9024199586fc6c0c03f830a2c8f
model_reproducibility_builds=3
consumer_reproducibility_builds=3
post_link_reproducibility_builds=3
verus_verified=22
model_undefined_symbols=core-panic,memcpy
capsule_bytes=93
dispatcher_virtual=ffffffff80011100
scalar_seam_virtual=ffffffff80011200
scalar_return_virtual=ffffffff80011038
scalar_transfer=tail-jump
scalar_abi=cr2,error,rip,rflags,user-rsp-or-zero,vector-cs-ss-metadata
frame_offsets=cr2:112,vector:128,error:136,rip:144,cs:152,rflags:160,rsp:168,ss:176
frame_words_read=user:8,kernel:6
scalar_stack_alignment=dispatcher-entry:8,scalar-entry:8
caller_requirements=cpl0,if-clear,df-clear,registered-readable-prefix,conditional-readable-user-tail,registered-readable-nonoverlapping-exact-common-continuation-return-address,registered-returning-frame-and-rbx-preserving-scalar
runtime_marker=M1_EXCEPTION_DISPATCHER_FRONT_OK bytes=93 user_words=8 kernel_words=6 metadata=001b00230000000e scalar_entry_mod16=8 tail=1
negative_cases=byte-mutation,unregistered-executable,cr2-argument,error-argument,user-rsp,metadata,scalar-target,frame-unchanged,stack-alignment,tail-transfer,return-target,word-count,bad-assume
```

Generated proof, runtime, link, disassembly, and negative evidence remains under
ignored `build/m1-exception-dispatcher-front/`.

## Remaining boundary

This checkpoint proves what the exact dispatcher-front bytes do when their
machine-memory caller obligations hold. The accepted common-entry model has not
yet been composed with this model to establish those obligations for its
concrete RDI value and stack range. The scalar function at
`0xffffffff80011200` is still only registered; it must decode the packed values,
join them to the accepted safe frame/policy bridge and per-CPU context, and
preserve the common-entry frame/RBX convention. Returning and fatal
non-returning behavior, action execution, one joined stub/common/dispatcher
image, and live CPL0/CPL3 delivery under QEMU remain open. Consequently
`scalar_body_present=false`, `safe_decoder_joined=false`,
`hardware_executed=false`, and `release_eligible=false` remain explicit.
