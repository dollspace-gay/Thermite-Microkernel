# M0 HLT/register capsule evidence

The M0 capsule has the exact bytes `48 89 f8 f4`, decoded as:

```text
mov rax,rdi
hlt
```

`verus/machine-model/hlt_register_capsule.rs` proves that this registered word:

- decodes to those four bytes;
- copies pre-state RDI to post-state RAX;
- preserves RDI;
- advances RIP by exactly four without overflow;
- enters the halted state; and
- rejects a different word or an overflowing RIP without changing machine state.

The following command was run twice consecutively:

```text
cargo run -p xtask -- m0-verus-capsule
```

Each run:

1. checks exact Verus and GNU binutils digests;
2. verifies and compiles the `no_std` model with `--no-cheating`;
3. reproduces the rlib in two additional absolute paths;
4. links and executes a consumer that covers the accepted, malformed, and RIP
   overflow cases and emits the proved little-endian bytes;
5. converts the bytes into a named executable capsule object;
6. links it using `kernel-host/link/m0_capsule.ld`;
7. re-extracts and compares the linked bytes;
8. rejects relocations or any executable section outside the capsule allowlist;
9. checks entry/end symbols and archives a disassembly; and
10. rejects a changed capsule byte, an extra executable HLT section, a semantic
    body mutation, and an injected `assume(false)`.

Stable positive identities:

```text
source_sha256=538f1cc62fe6bdda666a798df395aa27ca67bc9f1b1aa499861448f423580347
linker_script_sha256=6b1700fed37eb5ceaf7fa4fb3c172712df20266a36b9406cef19913ba3e80e77
model_artifact_sha256=5f122bdcedeffd4d469f53a954041333c4fb535570b0104a58b99ffa0a110df4
verus_result_sha256=cb18c31a0a289672cf48f0e6d5f260a36ea9a8ffa31d013c84daebcf3f55e00b
consumer_sha256=586796e95f83507627e9176a9167c9c0e66ea64d0b71f4cfed0ab7906c3d54b2
linked_capsule_sha256=86f039964fb227ba98078e671367c11641ed25204ea080f1b5b30bd13c5deda8
linked_elf_sha256=76410ec81f67b150da038812bb45364293941f2813cbc23a97e826c2596ac576
```

Post-link review outputs:

```text
disassembly.txt  cae1d2e26fe0d8da8b5481238d0b0201bb6b1a8a061aa3e47e407818e4e38392
relocations.txt  15864317fea2c9ccafcbdd8216912d3497e933fce1ffe4a0dbf37d14cbb033d3
sections.txt     1d225f69b36d41747c55cdb5fe083b418f0b2109f39d89acbbc3314b611de14b
symbols.txt      a5ebd9a8a3e12475fcdaa853eff2c261f19e3387f4ee8cabc2607d425e182ebf
```

The deterministic byte-mutation and unregistered-section rejection diagnostics
have digests `6131a7485cb239f3105640cfd0e34cddd71b54199877af00a410e91dc5242b4b`
and `afa6279a31f6320fd0b60993ce0b5bc0bcd064706e7f02ce49eef51cec82f2e1`.
The report digest was
`04620a8879a67a4e3d41a44ced9734d34f64f9d0c5a8507ab69633e08b7645c3`.

This proves and post-link checks the M0 capsule instance. The exact bytes also
survive the freestanding component-host link recorded in `panic-host.md`. The
empty UEFI image and clean receipt/manifest binding now pass. Later privileged-
operation capsules are M1 work, so this component report correctly states
`release_eligible=false`.
