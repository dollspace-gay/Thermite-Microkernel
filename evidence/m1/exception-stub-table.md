# M1 exception-stub table

Status: **accepted M1 subcomponent** with direct Verus
`0.2026.05.24.ecee80a`. This checkpoint proves the concrete 256-slot IDT target
table, its normalization pushes, and all branches to one registered address. The
common-entry body is intentionally absent and no IDT delivery has run on
hardware.

## Accepted implementation

`verus/machine-model/exception_stub_table.rs` constructs one page-aligned,
4096-byte table at `0xffffffff80010000`. Every vector owns an exact 16-byte slot,
matching the already accepted IDT handler base and stride.

The ten architectural exception vectors that receive a CPU-pushed error code
are 8, 10, 11, 12, 13, 14, 17, 21, 29, and 30. Their slots push only the full
vector value before branching. Each of the other 246 slots first pushes a
synthetic zero error code, then the full vector. `PUSH imm32` is used for the
vector so values `0x80..0xff` are not incorrectly sign-extended from an
eight-bit immediate. Each slot has a proved displacement to
`0xffffffff80011000` and is padded with NOPs to the exact stride.

Executable decoders recover the normalization class, vector, displacement,
absolute branch target, opcode form, and padding from the two real instruction
qwords. The public branch decoder is total: malformed displacements beyond the
registered page return zero instead of overflowing.

## Acceptance command

```text
cargo run -p xtask -- m1-exception-stubs
```

The command pins Verus, Rust 1.95, and all binutils tools; rejects proof escape
hatches; proves and compiles three byte-identical model rlibs; separately builds
and executes three byte-identical consumers; emits three byte-identical 4096-byte
tables; and creates three byte-identical high-half ELFs.

Every consumer decodes all 256 slots, checks the full vector value, classifies
exactly ten CPU-error and 246 synthetic-error paths, confirms every branch
target, and emits the real instruction bytes. Post-link auditing requires no
relocations, exactly one executable section, exact byte identity, exact boundary
symbols, and 256 disassembled branches to the common-entry address.

Ten negative gates must fail:

- changed linked instruction bytes;
- an unregistered executable section;
- removing page fault from the CPU-error class;
- an off-by-one branch displacement;
- a changed synthetic-path push opcode;
- a changed CPU-error-path push opcode;
- leaving the final table slot uninitialized;
- changing the verified observation;
- inserting a Verus `assume`; and
- building without the explicit `vstd` array-spec proof dependency.

## Stable result

```text
M1_EXCEPTION_STUBS_OK
component_verified=true
release_eligible=false
hardware_executed=false
common_entry_body_present=false
source_sha256=1ecc478052ce1aeab0e51eee79277ff8ad750d6e14fdd62b7b03ce93f65bb31f
descriptor_source_sha256=3bc64781e9c90a2bbb7af49a942cf931bb6b277ca50c0939bf6fc1c9fe1ea065
consumer_source_sha256=943630a2fcc5c44fb902b00010c62407c0754768efc31c81367dbc2f84d9b865
linker_script_sha256=eda6d45b424b98d8c3883bfa158becc5ce6a06259d1a8d25daf8d12bcb00ef77
model_artifact_sha256=853761bc042907181b2f82f1519bb1c4ec0c8ae1c4f58fce56cd92c3f1c80269
consumer_sha256=21557949f239ce6554ab649d5108a40558825645deb42ede9e4efc92d205dc92
emitted_table_sha256=dec0e650bab2fd0ef44fb1668068b57b72b0a40b8f1b53d1319f4d34de15a071
linked_table_sha256=dec0e650bab2fd0ef44fb1668068b57b72b0a40b8f1b53d1319f4d34de15a071
linked_elf_sha256=748390aa378441d42689b2e5f7fb9c9b4152a5a3297f3377968b2628355711f4
model_reproducibility_builds=3
consumer_reproducibility_builds=3
post_link_reproducibility_builds=3
verus_verified=20
model_undefined_symbols=core-panic,core-panic-bounds-check,memcpy
vectors=256
cpu_error_vectors=10
synthetic_error_vectors=246
table_bytes=4096
stub_bytes=16
table_virtual=ffffffff80010000
common_entry_virtual=ffffffff80011000
proof_library=vstd-array-spec-only
runtime_marker=M1_EXCEPTION_STUBS_OK observation=255 vectors=256 error=10 synthetic=246 bytes=4096 target=ffffffff80011000
negative_cases=byte-mutation,unregistered-executable,error-classification,displacement,synthetic-opcode,cpu-error-opcode,table-completeness,observation,bad-assume,vstd-proof-dependency
```

Generated proof, runtime, and post-link evidence remains under ignored
`build/m1-exception-stubs/`.

## Remaining boundary

The linker binds `tmk_exception_common_entry` to the table end, but no bytes are
accepted for that symbol in this checkpoint. This is intentional: a branch to a
named address is not evidence that a safe destination exists.

The next exact-byte model must save all registers, capture CR2 before a possible
second fault on vector 14, materialize the normalized trap frame, establish the
verified dispatcher calling convention, and connect to a proved `IRETQ` return
path. Final linking must then replace the address-only seam with the exact proved
body and QEMU must exercise representative same-ring, ring-transition, error-
code, no-error-code, IST, and page-fault paths.
