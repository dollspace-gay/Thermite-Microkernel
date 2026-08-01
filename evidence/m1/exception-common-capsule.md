# M1 common exception-entry capsule

Status: **accepted M1 subcomponent** with direct Verus
`0.2026.05.24.ecee80a`. This checkpoint proves the exact returning common-entry
instruction image and its abstract machine refinement. It does not supply the
dispatcher body and has not executed through a live IDT.

## Accepted implementation

`verus/machine-model/exception_common_capsule.rs` registers 105 exact bytes at
`0xffffffff80011000`. The image:

1. saves the interrupted RAX and immediately captures CR2;
2. saves RBX, RCX, RDX, RSI, RDI, RBP, and R8 through R15;
3. tests the normalized saved CS and executes `SWAPGS` for user origin only;
4. clears DF with `CLD`;
5. passes the frame in RDI, retains it in RBX, and aligns RSP to 16 bytes;
6. calls the registered seam at `0xffffffff80011100`;
7. restores the frame stack and conditionally restores user GS;
8. restores every saved register and discards captured CR2/vector/error; and
9. executes `IRETQ`.

The machine model covers CPL, IF/DF, GS mode, CR2, CS/SS, RIP/RSP/RFLAGS, all
general registers, the normalized frame, dispatcher state, and final resume
state. The precondition requires a CPL0 interrupt-gate context, a normalized
frame, 151 valid entry-stack bytes, canonical return addresses, exact selectors,
allowed RFLAGS, a registered returning dispatcher that preserves RBX/the frame,
and GS mode consistent with origin. User-origin execution proves two `SWAPGS`
transitions; kernel-origin execution proves zero. `CLD` governs dispatcher entry;
validated interrupted RFLAGS are restored by `IRETQ`.

## Acceptance command

```text
cargo run -p xtask -- m1-exception-common
```

The command proves and compiles three byte-identical model rlibs, builds and
executes three byte-identical consumers, emits three byte-identical capsule
images, and links three byte-identical high-half ELFs. Post-link auditing
requires one 105-byte executable section, no relocations, the exact registered
addresses, two conditional `SWAPGS` sites, a direct call to the dispatcher seam,
and final `IRETQ`.

The runtime consumer executes both a user-origin page-fault model with CR2
`0x12345000` and a same-ring kernel timer model. Eight negative gates reject a
changed byte, an unregistered executable section, stale CR2 capture, lost GPR
restoration, incorrect `SWAPGS`, uncleared dispatcher DF, altered resume RSP,
and a Verus `assume`.

## Stable result

```text
M1_EXCEPTION_COMMON_OK
component_verified=true
release_eligible=false
hardware_executed=false
dispatcher_body_present=false
source_sha256=4c46a4107a9ae752e6ffbba4af33de1d2e422d3141189ef7012fd7211dc69da3
stub_source_sha256=1ecc478052ce1aeab0e51eee79277ff8ad750d6e14fdd62b7b03ce93f65bb31f
consumer_source_sha256=7835394ff60afb533bdd7a51be60144c199e8d0bc5381a8f3772780b4905c66a
linker_script_sha256=4461697faeca3fc4df3094bd0a2295856f836924eb4f475399976eb173621f02
model_artifact_sha256=d650e89223c145ddd2e86a2c35109d3302b5c4fbe6afd1941c7a8e03fb401bee
consumer_sha256=1fbe0f1b2f21b1031cfb64988ea7a52c69575d589b4a6a2bccb3edd6ae613ab8
emitted_capsule_sha256=e1581161930fa06ac2e35a71be20b1955fab8ba061787de5392ee676d3900433
linked_capsule_sha256=e1581161930fa06ac2e35a71be20b1955fab8ba061787de5392ee676d3900433
linked_elf_sha256=aaccc4c8eb544d4b3b52052c297b37ee226abe217241c870621b9d978f5f6dc3
model_reproducibility_builds=3
consumer_reproducibility_builds=3
post_link_reproducibility_builds=3
verus_verified=27
model_undefined_symbols=core-panic,memcpy
capsule_bytes=105
common_entry_virtual=ffffffff80011000
dispatcher_virtual=ffffffff80011100
caller_requirements=cpl0,interrupt-gate-if-clear,normalized-frame,151-byte-valid-entry-stack,canonical-resume-state,valid-return-rflags,registered-returning-frame-preserving-dispatcher,gs-mode-match
runtime_marker=M1_EXCEPTION_COMMON_OK bytes=105 vector=14 cr2=0000000012345000 frame=ffffe00000002e80 swapgs=2 iret_cpl=3
negative_cases=byte-mutation,unregistered-executable,cr2-capture,gpr-restore,swapgs,df-clear,resume-rsp,bad-assume
```

Generated proof, execution, disassembly, and post-link evidence remains under
ignored `build/m1-exception-common/`.

## Remaining boundary

The direct model abstracts the valid stack/frame memory required by the exact
instruction sequence as caller obligations. BSP integration must establish
those obligations from concrete guarded mappings and must initialize the GS
bases before enabling delivery. The dispatcher address is registered, but its
body is absent; fatal/non-returning dispatch behavior is not modeled here.

The next joined entry gate must link the already accepted 4096-byte stub table,
this common body, and a proved dispatcher, then exercise representative
same-ring, privilege-transition, error/no-error, IST, and page-fault deliveries
under QEMU. Until then `hardware_executed=false` and
`dispatcher_body_present=false` remain release blockers.
