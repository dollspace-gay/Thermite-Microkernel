# M1 descriptor-install capsule

Status: **accepted M1 subcomponent** with direct Verus
`0.2026.05.24.ecee80a`. This checkpoint proves the machine-state refinement and
exact linked bytes of the initial descriptor-install sequence. The privileged
bytes have not yet executed in the boot VM.

## Accepted implementation

`verus/machine-model/descriptor_install_capsule.rs` registers this exact
38-byte sequence at `0xffffffff80001010`:

```text
lgdt [rdi]
mov eax,0x10
mov ds,eax
mov es,eax
mov ss,eax
push 0x8
lea rax,[rip+0x3]
push rax
retfq
mov eax,0x28
ltr eax
lidt [rsi]
ret
```

The `RETFQ` target is offset 26 inside the registered capsule. The two far-
return pushes and far return have zero net stack movement; the final near return
advances RSP by eight bytes. RAX is the one modeled general register clobber.

The executable model requires:

- CPL 0 with maskable interrupts disabled;
- an explicit interval with asynchronous NMI/SMI/machine-check delivery absent;
- readable canonical ten-byte GDTR and IDTR operands;
- canonical, bounded registered GDT and IDT memory ranges with exact limits 55
  and 4095;
- an available writable TSS descriptor, because `LTR` sets its busy bit;
- sixteen writable bytes below RSP for the same-privilege far return, an
  eight-byte-aligned non-wrapping stack, and a readable final return slot; and
- a canonical final return address.

On success the model proves the operand bases and limits are installed, CS is
`0x08`, DS/ES/SS are `0x10`, TR is `0x28`, the TSS descriptor is busy, RDI/RSI
and RFLAGS are preserved, RAX is `0x28`, RSP advances by eight, and RIP receives
the caller return address. Rejected abstract executions commit no modeled state.

## Acceptance command

```text
cargo run -p xtask -- m1-descriptor-install
```

The command pins Verus, Rust 1.95, and all binutils tools; rejects proof escape
hatches; proves and compiles three byte-identical model rlibs; separately builds
and executes three byte-identical consumers; compares all emitted byte images;
creates three byte-identical high-half ELFs; and audits exact bytes, symbols,
sections, relocations, and every expected disassembled instruction.

The consumer executes the registered decoder, checks every final register and
selector observation, and rejects a malformed byte image, ring-3 execution,
enabled interrupts, a non-quiescent interval, unregistered GDT content, wrong
IDT limit, an already-busy or read-only TSS descriptor, undersized stack space,
and a noncanonical return address.

Eight negative gates must fail:

- changed linked instruction bytes;
- an unregistered executable section;
- the wrong final CS;
- failure to mark the TSS descriptor busy;
- the wrong final RSP;
- failure to install the IDTR limit;
- failure to expose the documented RAX clobber; and
- an inserted Verus `assume`.

## Stable result

```text
M1_DESCRIPTOR_INSTALL_OK
component_verified=true
release_eligible=false
hardware_executed=false
source_sha256=d1e193fe7e6a195dc921852c8822c3b5fbcc81b107edd084349f8979ab5a1538
descriptor_source_sha256=3bc64781e9c90a2bbb7af49a942cf931bb6b277ca50c0939bf6fc1c9fe1ea065
consumer_source_sha256=58a7ea1dad67323b2e35a1aac703453dcd9e76656f5419c16e774517a0829f49
linker_script_sha256=ca906ef85e410544b5cbbac081937f8b2d0279abd0d3dc44da5c59bdcf4feaa8
model_artifact_sha256=532e4b544b373c8b97f5b5e381a499d4d700380927b83119e854abfc87da0956
consumer_sha256=48cd01bc8cd563c7fefdec74dd7c7e9394eb4b6c7e16b9f23c89410a2c0b2b67
emitted_capsule_sha256=110202476bb2e489fad3aebc3fd5588582c33e24ad31fd3287e72fd46b8fc77e
linked_capsule_sha256=110202476bb2e489fad3aebc3fd5588582c33e24ad31fd3287e72fd46b8fc77e
linked_elf_sha256=0ff44ebe8974b8d902005819117c7ac1e7589b98b9605c3e5de3a639be27356a
model_reproducibility_builds=3
consumer_reproducibility_builds=3
post_link_reproducibility_builds=3
verus_verified=19
model_undefined_symbols=core-panic,memcpy
capsule_bytes=38
linked_virtual=ffffffff80001010
caller_requirements=cpl0,interrupts-disabled,asynchronous-quiescence,registered-readable-tables,writable-available-tss,readable-writable-stack,canonical-operands-and-return
runtime_marker=M1_DESCRIPTOR_INSTALL_OK bytes=38 cs=08 ss=10 tr=28 rsp=ffffe00000001008 busy=true
negative_cases=byte-mutation,unregistered-executable,cs-semantics,tss-busy,rsp-semantics,idtr-semantics,rax-semantics,bad-assume
```

Generated proof, runtime, and post-link evidence remains under ignored
`build/m1-descriptor-install/`.

## Remaining boundary

The x86 decoder and hardware remain explicit environmental assumptions. The
model abstracts readable registered table memory and stack accesses behind
caller obligations; the eventual BSP call site must connect those obligations
to the concrete descriptor and page-table ownership proofs.

FS and GS are intentionally untouched. Their selectors/bases and the GS-based
per-CPU block belong to the next per-CPU setup capsule. The BSP must also enforce
the asynchronous-quiescence interval, include this exact object in the final
kernel, call it under OVMF/QEMU, read back installed state, and then exercise
real exception entry through separately verified stubs.
