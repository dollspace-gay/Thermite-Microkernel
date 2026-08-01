# M1 exception entry/dispatcher join

Status: **accepted M1 subcomponent**. This checkpoint composes the accepted
105-byte common exception entry with the accepted 93-byte dispatcher front. It
proves that the common entry establishes the dispatcher's concrete frame,
memory, stack-alignment, and return-address obligations. It introduces no new
machine instructions and does not yet supply the scalar dispatcher body.

## Accepted composition

`verus/machine-model/exception_entry_dispatcher_join.rs` models the exact stack
relationship across the two registered images. Starting from a normalized
entry RSP, the common capsule saves 128 bytes, rounds the call stack down to a
16-byte boundary, and writes the call return address. The theorem covers both
possible eight-byte-aligned entry cases:

| Entry RSP low bits | Saved-frame base | Dispatcher RSP |
|---:|---:|---:|
| 0 | `entry_rsp - 128` | `entry_rsp - 136` |
| 8 | `entry_rsp - 128` | `entry_rsp - 144` |

Both dispatcher RSP results are congruent to 8 modulo 16. RDI and RBX contain
the saved-frame base, DF is clear after the common entry's `CLD`, and the return
word is readable, below the saved frame, and exactly
`0xffffffff80011038`. The scalar tail transfer therefore inherits a valid SysV
stack and returns directly to the common restoration sequence.

The concrete stack-region contract covers from `entry_rsp - 144` through the
normalized frame end: `entry_rsp + 40` for a same-ring frame or
`entry_rsp + 56` for a user frame. This establishes readable CR2, vector,
error, RIP, CS, and RFLAGS words in both cases and the RSP/SS tail only for user
origin. The joined result preserves the frame and RBX, transports the exact six
scalar arguments, and refines the returning path through the common restore and
`IRETQ` state.

## Acceptance command

```text
cargo run -p xtask -- m1-exception-entry-dispatcher-join
```

The command first reruns the complete accepted common-entry and
dispatcher-front gates. It then performs three whole-crate Verus proof/codegen
builds with 27 verified obligations and zero errors, compiles and executes
three byte-identical consumers, and exercises valid user and kernel paths for
the two entry-alignment cases. Thirteen runtime rejections cover insufficient
lower or upper stack bounds, non-eight-byte entry alignment, unreadable or
unwritable stack memory, an unregistered normalized frame, mismatched GS mode,
an invalid selector or vector, absent/non-returning scalar registration, and
mutated common or dispatcher registrations. The runtime marker is:

```text
M1_EXCEPTION_ENTRY_DISPATCHER_JOIN_OK common=105 dispatcher=93 user_rsp=ffffe00000002e78 kernel_rsp=ffffe00000003e78 alignment=8 continuation=ffffffff80011038 rejects=13
```

Three fixed-address links reproduce byte-for-byte. Each ELF contains exactly
the two registered executable sections at `0xffffffff80011000` and
`0xffffffff80011100`, exposes the scalar seam at `0xffffffff80011200`, has no
relocations, and post-link-matches both accepted component images. Disassembly
auditing requires one direct common-to-dispatcher call, its exact continuation,
one dispatcher-to-scalar tail jump, and one final `IRETQ`. Common-byte,
dispatcher-byte, and extra-executable mutations fail. Eight proof mutations of
the frame base, dispatcher RSP, return address, metadata, DF state, tail
transfer, final RIP, and no-cheating policy fail atomically.

## Stable result

```text
M1_EXCEPTION_ENTRY_DISPATCHER_JOIN_OK
component_verified=true
release_eligible=false
hardware_executed=false
scalar_body_present=false
common_obligations_discharged=true
source_sha256=38f0a08c0c9cb3cfd28a82a74b62d301d4a05720dd2b939186e86569d85ccdee
common_source_sha256=4c46a4107a9ae752e6ffbba4af33de1d2e422d3141189ef7012fd7211dc69da3
dispatcher_source_sha256=937e5017873e0cbed35554a9d121e4f739365a267e00767eda5a205b0f5e90bb
consumer_source_sha256=7294ce1a0625d7562f5b291d55653af5815b0c98d1e703ba6c839d4e3977ad93
linker_script_sha256=74282cd276c30686999c3fe71044a2d2c0b9f1f1af2d37ae9a18eab57008f331
model_artifact_sha256=0e0091e8215469f4901ef4956b90feb844e32a28b8c5f926662e53e943bf19d4
consumer_sha256=d1940df46a22abd4344d06813c1a9564cf1765e7958ecddb5aadc2c8ecf88d9a
joined_common_sha256=e1581161930fa06ac2e35a71be20b1955fab8ba061787de5392ee676d3900433
joined_dispatcher_sha256=90d3b2fe61a8633f5e6f0cee32c20a22d7e01ea4cff7d6e3f2bc2123b86386bf
joined_elf_sha256=64d800b58dcf78198bf7e914d789bf544a279cb72e61f46317876fe90f20f04c
model_reproducibility_builds=3
consumer_reproducibility_builds=3
post_link_reproducibility_builds=3
verus_verified=27
model_undefined_symbols=core-panic,memcpy
common_bytes=105
dispatcher_bytes=93
common_virtual=ffffffff80011000
dispatcher_virtual=ffffffff80011100
scalar_virtual=ffffffff80011200
continuation_virtual=ffffffff80011038
entry_alignment_cases=0,8
dispatcher_alignment=8
frame_bounds=low:entry-rsp-144,high:entry-rsp+40-or-56
joined_properties=rdi-frame-identity,conditional-tail-readability,df-clear,exact-return-address,nonoverlap,scalar-alignment,frame-rbx-preservation,iret-resume
runtime_marker=M1_EXCEPTION_ENTRY_DISPATCHER_JOIN_OK common=105 dispatcher=93 user_rsp=ffffe00000002e78 kernel_rsp=ffffe00000003e78 alignment=8 continuation=ffffffff80011038 rejects=13
negative_cases=common-byte,dispatcher-byte,extra-executable,frame-base,dispatcher-rsp,return-address,metadata,df-clear,tail-transfer,final-rip,bad-assume
```

Generated proof, execution, component-replay, link, disassembly, and negative
evidence remains under ignored
`build/m1-exception-entry-dispatcher-join/`.

## Remaining boundary

The common-entry caller obligations are no longer assumed at the dispatcher
front. The function at `0xffffffff80011200` is still only a registered
returning seam. It must reconstruct a bounded safe frame view, join the accepted
decoder and exception policy with locked per-CPU/current-thread context, and
execute the selected machine action or enter a proved non-returning fail-stop
path. The full stub/common/dispatcher/scalar image and live IDT delivery under
QEMU remain separate acceptance gates.
