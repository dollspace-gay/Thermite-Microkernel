# M1 per-CPU scalar-core wrapper and fixed-address join

Status: **accepted M1 subcomponent** against public Thermite `main` commit
`b8dc3947f504454775aa70977d8bda5da677d2af`. This checkpoint closes the
registered scalar-core seam with exact GS setup, per-CPU lookup and frame-copy
bytes, the receipt-bound compiled adapter, explicit return/fail-stop routing,
and a reproducible fixed-address ELF. It does not claim QEMU or hardware
execution, and its schedule route is deliberately fail-closed until a verified
scheduler backend exists.

## Accepted boundary

The direct-Verus machine model owns four exact images:

| Image | Address | Bytes | Role |
|---|---:|---:|---|
| GS setup | `0xffffffff80001040` | 35 | write `IA32_GS_BASE` and `IA32_KERNEL_GS_BASE` |
| scalar wrapper | `0xffffffff80011300` | 314 | validate GS, copy frame/transport, call adapter, route control |
| fail-stop | `0xffffffff80011500` | 4 | `cli; hlt; jmp` nonreturning loop |
| schedule unavailable | `0xffffffff80011600` | 5 | branch to registered fail-stop |

GS setup requires CPL0, interrupts masked, registered current and kernel GS
headers, registered MSR access, and a readable return slot. The wrapper requires
CPL0, interrupts and DF clear, a registered four-word GS header, the exact
common return address, a registered adapter stack, and all linked control
targets. Header offsets are self 0, core block 8, active frame 16, and flags 24;
the block address must be 16-byte aligned and flags must equal `0x1ff`.

The wrapper copies saved-frame fields from the registered RDI memory and copies
the transported values independently from R10, RSI, RDX, RCX, R8, and R9. It
reads the RSP/SS tail only for user CS `0x23`. Both paths enter separate slots
of an exclusive 640-byte `ScalarCoreBlock` with 80 named `u64` fields. The real
verified adapter checks layout and frame/register agreement before invoking the
accepted Thermite policy/action core. The executed adapter scenarios cover a
user page fault, a CR2 mismatch that fail-stops with reason 101, and an invalid
snapshot that fail-stops with reason 100.

The final ELF places the receipt-bound adapter at `0xffffffff80012000`, followed
by its compiler runtime and the accepted nine-byte M0 `memcpy`. Link audit
requires eight registered executable sections, exact addresses and sizes, no
relocations, no undefined symbols, exact source-image bytes, adapter provenance,
and exact post-link `memcpy` identity. `rust_eh_personality` is bound to the
registered fail-stop target; the already verified M0 panic artifact is the
freestanding root's panic implementation.

## Acceptance command

```text
cargo run -p xtask -- m1-exception-scalar-core-wrapper
```

The command first reruns and replays the complete scalar prerequisite. It then
builds the 30-obligation wrapper model three times with `--no-vstd` and
`--no-cheating`, executes three model consumers, executes three consumers of the
actual L3 adapter, and links/audits three fixed-address ELFs. All proof and
binary products reproduce. The gate rejects frame-binding, kernel-tail,
fail-stop-route, and proof-escape mutations, a changed wrapper byte, an added
wrapper byte, and a moved adapter address.

## Stable result

```text
M1_EXCEPTION_SCALAR_CORE_WRAPPER_OK
component_verified=true
release_eligible=false
hardware_executed=false
qemu_executed=false
candidate_pin_verified=true
scalar_prerequisite_replayed=true
per_cpu_gs_setup_present=true
per_cpu_lookup_wrapper_present=true
scalar_core_fixed_address_linked=true
scalar_adapter_receipt_bound=true
scalar_adapter_executed=true
frame_register_cross_check=true
fail_stop_present=true
schedule_backend_present=false
schedule_route=registered-fail-stop-stub
model_source_sha256=457f6b5dc9ffff7391e11b12976e61caa061cc6a76d5e4d66c790c6fc13173d2
model_consumer_source_sha256=6d431386537b46c882b4dc8685cc00bfe1bdcfea4f1b4999080379cfdf03d7d7
adapter_consumer_source_sha256=8f632d62164f5c5792324067c2d63272fe066e0875d4a32e82e2a220b813971c
freestanding_source_sha256=c4a4a0ba22dc1ea957844bb655800fa70ee737ddd9721f06a91259c1dc0f8e91
linker_script_sha256=7c055d82cf117d7c9723be875e1084b59f5fa4152847f50940be158147764d90
scalar_shell_sha256=7dbe3b3b111c6da707b4413e73f4b24dd159cce0eb8ec11c5a1c51fa607e26dd
scalar_binding_sha256=744f6b59b5971805aa12711d51bdbbf66a33cb28092c6b88c726cd9e1ac7c57b
scalar_artifact_sha256=f7ab3f2f4909db60916a701a3f9d46808630f24f2ee1be89c70ff9aad9fef8bf
model_artifact_sha256=9c916206127a4455c6144e791e915364774b5d52a87cd85285f838ff8ca2a3f8
model_consumer_sha256=e1fd969ee23c859f3790c24fedfa6eb2ebd1e2eddafe8c9315ce4d4a9025451f
adapter_consumer_sha256=3998c86e1d880dc73edff89a41901500ffda7663c45b0505b24a420ca704317b
linked_elf_sha256=ddb0ef524fe313854c1372f5a214299719abb4e309219183d670bf23c2a721b7
linked_adapter_sha256=e8a9610d2ba7b383da6658a4889d37e4fed560aef8941ca9483e1d123cc7075f
linked_runtime_sha256=3bcc30caa6b4835786c346cbd695d4122269370fe23714aac966408f168ccc49
linked_memcpy_sha256=00d0174466d21d8a224c588f4bf2e324a605806fac0cb0d4fa2ba1a9667a49a9
panic_artifact_sha256=48bdebcba3090800b1bbb64524706660e5611a1a588e8d3b7ed5cec7b28967d6
platform_primitive_object_sha256=a3884a20bfb8193e6cfdbf921eae60bac038406aebfe9184ad5039e0629ec50f
forge_source_identity=b8dc3947f504454775aa70977d8bda5da677d2af
forge_sha256=b073fa34a955dc4ce723aac3cdba36ed031e7daa1ca5db6ead866d41ef36fbf9
verus_verified=30
model_reproducibility_builds=3
model_consumer_reproducibility_builds=3
adapter_consumer_executions=3
post_link_reproducibility_builds=3
gs_setup_virtual=ffffffff80001040
gs_setup_bytes=35
scalar_entry_virtual=ffffffff80011200
scalar_entry_bytes=11
scalar_wrapper_virtual=ffffffff80011300
scalar_wrapper_bytes=314
fail_stop_virtual=ffffffff80011500
fail_stop_bytes=4
schedule_stub_virtual=ffffffff80011600
schedule_stub_bytes=5
scalar_adapter_virtual=ffffffff80012000
scalar_adapter_bytes=1885
scalar_core_block_bytes=640
scalar_core_block_layout=80-u64-slots
scalar_core_block_offsets=frame-cr2:112,word-count:184,args:192,policy:384,outcome:600
gs_header_offsets=self:0,core-block:8,active-frame:16,flags:24
gs_header_flags=00000000000001ff
wrapper_runtime_marker=M1_EXCEPTION_SCALAR_CORE_WRAPPER_OK images=35,314,4,5 scenarios=10 rejected=4 routes=return,schedule-fail-closed,fail-stop cross-check=frame-vs-register
adapter_runtime_marker=M1_EXCEPTION_SCALAR_ADAPTER_OK layout=640 offsets=0,112,184,192,384,600,632 scenarios=page-fault,mismatch,bad-snapshot
negative_cases=frame-binding,kernel-tail,fail-stop-route,bad-assume,wrapper-byte,wrapper-extra-byte,adapter-address
```

Generated evidence remains under ignored
`build/m1-exception-scalar-core-wrapper/`.

## Remaining boundary

`schedule_backend_present=false`, `qemu_executed=false`,
`hardware_executed=false`, and `release_eligible=false` are deliberate. The next
entry-path work must provide real scheduler and IRQ/LAPIC/TLB/crash adapters,
join the already accepted stub/common/front images to this scalar-core ELF, and
exercise CPL0/CPL3 exceptions and interrupts through the live IDT under QEMU.
