# M1 CR3 installation capsule

Status: **accepted M1 subcomponent** with direct Verus
`0.2026.05.24.ecee80a`. This checkpoint proves the machine-state refinement and
exact linked bytes of the initial CR3 write/return capsule. The privileged bytes
have not yet executed in the boot VM, so this is not a live root-installation
claim.

## Accepted implementation

`verus/machine-model/cr3_install_capsule.rs` registers the four-byte sequence
`0f 22 df c3`, decoded as `mov cr3,rdi; ret`. The executable model accepts an
architecturally valid call state with:

- CPL 0;
- PCID disabled;
- a page-aligned CR3 value within the 52-bit physical-address field;
- a readable return stack whose eight-byte pop cannot overflow; and
- a canonical return address.

On success it proves that CR3 receives RDI, RSP advances by eight, RIP receives
the popped return address, non-global TLB translations are invalidated, and RAX,
RDI, RFLAGS, CPL, PCID state, and interrupt-enable state are preserved. Rejected
abstract executions commit none of those changes. The specialized integration
function requires RDI to equal the reference page-table root at `0x0040_0000`.
The capsule does not inspect or sanitize its caller; those are proof obligations
of the eventual verified call site.

The linker places the capsule alone in executable section
`.text.tmk_cr3_capsule` at `0xffffffff80001000`, asserts a four-byte section, and
binds the entry to its first byte. The post-link audit requires the same four
registered bytes, no relocations, exactly one executable section, and the
expected `mov cr3,rdi; ret` disassembly.

## Acceptance command

```text
cargo run -p xtask -- m1-cr3
```

The command pins Verus, Rust 1.95, and every binutils tool; rejects proof escape
hatches; proves and compiles three byte-identical model rlibs; separately builds
and executes three byte-identical consumers; compares the three emitted capsule
images; creates three byte-identical high-half ELF links; and audits the linked
bytes, symbols, sections, relocations, and disassembly.

The consumer executes the decoder for the registered root and checks CR3, RET,
preservation, and invalidation observations. It also executes the generic model
with another valid root and confirms rejection of a malformed opcode, ring-3
call, misaligned root, PCID-enabled state, unreadable stack, noncanonical return,
and overflowing RSP.

Six negative gates must fail:

- changed linked opcode bytes;
- an unregistered executable section;
- a model that leaves CR3 unchanged;
- a model that fails to invalidate non-global translations;
- a specialized call contract bound to the wrong root; and
- an inserted Verus `assume`.

## Stable result

```text
M1_CR3_CAPSULE_OK
component_verified=true
release_eligible=false
hardware_executed=false
source_sha256=3504954f9aab1db8334a2935d0765212cf65b2215d6e6acc5426f1517ac77c3b
page_table_source_sha256=802a5df7aba6d1cf527dd5b2fdf88d81e15b272d952a0a837fa2e1edbd024c18
consumer_source_sha256=1091fbd4cb7ad5e39783130b6256ee863e9244b3c49458d66e607f73709849bd
linker_script_sha256=9f157418778abedef232c5539237c218140465636b66778f5d3281d8f8f607ac
model_artifact_sha256=e1dd44299308c4fdd60eee6debe4f67943d3b0c198b3235ca0b14efdb2ac7808
consumer_sha256=096755b7822c507b53b220ac883340c656d721de3ac2194e575d182303175be4
emitted_capsule_sha256=fe1401c9a0334f051d9f1bb440d8ce9840c023fb5ab66cc2d9c32fa730feebc3
linked_capsule_sha256=fe1401c9a0334f051d9f1bb440d8ce9840c023fb5ab66cc2d9c32fa730feebc3
linked_elf_sha256=10295b53baeef792ac6de8761bd8a881cc0971e2f3fd54cecd794bba734d4cdc
model_reproducibility_builds=3
consumer_reproducibility_builds=3
post_link_reproducibility_builds=3
verus_verified=15
model_undefined_symbols=core-panic,memcpy
root_physical=0000000000400000
linked_virtual=ffffffff80001000
caller_requirements=cpl0,pcid-disabled,aligned-52-bit-root,readable-return-stack,canonical-return
runtime_marker=M1_CR3_CAPSULE_OK bytes=0f22dfc3 cr3=0000000000400000 rsp=2028 invalidated=true
negative_cases=byte-mutation,unregistered-executable,cr3-semantics,tlb-semantics,root-binding,bad-assume
```

Generated proof, runtime, and post-link evidence remains under ignored
`build/m1-cr3/`.

## Remaining boundary

The model rlib uses `memcpy` and a core panic path when run as hosted evidence;
neither dependency exists in the four-byte capsule itself. The x86 decoder and
hardware remain an explicit environmental assumption, as with every exact-byte
capsule.

The loader still has to populate and own the root frames, prove the call-state
requirements at the actual call site, include the exact object in the final
kernel link, execute it under OVMF/QEMU, and probe post-install translations.
PCID stays disabled for this initial capsule; PCID allocation, `INVPCID`, local
invalidation, and SMP shootdown protocols are later memory/SMP gates.
