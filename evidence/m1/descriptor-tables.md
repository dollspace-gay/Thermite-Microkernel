# M1 descriptor-table images

Status: **accepted M1 subcomponent** with direct Verus
`0.2026.05.24.ecee80a`. This checkpoint proves concrete descriptor memory
construction and executable decoding. It does not claim that descriptor
registers were loaded or that an exception entered through these tables.

## Accepted implementation

`verus/platform/descriptor_tables.rs` constructs these exact per-CPU images:

- a naturally 8-byte-aligned, 56-byte GDT containing null, kernel code/data,
  user data/code, and a two-slot available 64-bit TSS descriptor;
- a packed 104-byte x86_64 TSS with RSP0, IST1 for double fault, IST2 for NMI,
  IST3 for machine check, zeroed reserved/unused fields, and `iomap_base=104`;
- packed 10-byte GDTR/IDTR operand values; and
- a 16-byte-aligned, 4096-byte IDT containing all 256 interrupt gates.

The GDT selectors are kernel code `0x08`, kernel data `0x10`, user data `0x1b`,
user code `0x23`, and TSS `0x28`. The user descriptor order preserves the later
STAR/SYSRET selector relationship, although initial privilege return remains an
`IRETQ` design requirement.

Every IDT gate names the kernel-code selector and a registered handler slot at
`0xffffffff80010000 + vector * 16`. Vector 3 alone is present at DPL 3. Double
fault, NMI, and machine check select IST1, IST2, and IST3; every other vector
selects the current RSP. All gates are present 64-bit interrupt gates, and their
reserved bits are zero. Executable decoders recover the gate offset, selector,
IST, attributes, reserved-bit state, and the split TSS descriptor base, with
postconditions tied to the construction specifications.

## Acceptance command

```text
cargo run -p xtask -- m1-descriptors
```

The command pins Verus, Rust 1.95, GNU `ar`, and GNU `nm`; rejects proof escape
hatches; proves and compiles three byte-identical model rlibs; audits the archive
members, defined entry points, and exact undefined-symbol class; and separately
builds and executes three byte-identical consumers.

Each consumer checks the Rust ABI sizes and alignments, creates a TSS at a stable
host address, verifies that the GDT descriptor decodes to that address, scans all
256 gates, counts exactly one user-callable vector and three IST vectors, builds
GDTR/IDTR operands with exact limits, and executes the verified observation.

Eight negative gates must fail:

- removing breakpoint DPL 3;
- removing the double-fault IST;
- substituting a user data descriptor for user code;
- enabling the TSS I/O bitmap by moving its offset;
- leaving the final IDT gate uninitialized;
- changing the verified observation;
- inserting a Verus `assume`; and
- building the proof without its explicit `vstd` array-spec dependency.

## Stable result

```text
M1_DESCRIPTOR_TABLES_OK
component_verified=true
release_eligible=false
hardware_loaded=false
source_sha256=3bc64781e9c90a2bbb7af49a942cf931bb6b277ca50c0939bf6fc1c9fe1ea065
consumer_source_sha256=29c11723a584e658440910ace2789d1449f1c494c308b2e401bb9841a5170342
model_artifact_sha256=cded79f1e00c589c2e4dfd6ca722fa004a13e01ef8bed19b2b4e46ae642b99ae
consumer_sha256=c506fcd04d59773878be575e3539210cbda338cf4ec8991ea18c9573761114ac
model_reproducibility_builds=3
consumer_reproducibility_builds=3
verus_verified=36
gdt_entries=7
idt_entries=256
tss_bytes=104
user_callable_vectors=1
ist_vectors=3
proof_library=vstd-array-spec-only
executable_undefined_symbols=core-panic,core-panic-bounds-check,memcpy
runtime_marker=M1_DESCRIPTOR_TABLES_OK observation=255 gdt=7 idt=256 ist=3 dpl3=1 tss=104
negative_cases=breakpoint-dpl,double-fault-ist,user-code-descriptor,iomap-base,idt-completeness,observation,bad-assume,vstd-proof-dependency
```

Generated proof and runtime evidence remains under ignored
`build/m1-descriptors/`.

## Remaining boundary

The registered handler addresses are bounded fixture slots. Their corresponding
entry stubs have not yet been emitted or linked. The RSP0 and IST values likewise
name designed per-CPU stack tops; the general page-table builder must still
allocate, guard, and map them before use.

The BSP must construct the images at stable kernel addresses, prove the actual
load-state preconditions, execute separately verified `LGDT`, `LIDT`, `LTR`, and
segment-reload capsules, include those exact bytes in the final kernel, and run
exception-delivery probes under OVMF/QEMU. The hosted consumer establishes real
construction and decoding behavior, not privileged hardware execution.
