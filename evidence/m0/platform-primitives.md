# M0 verified platform primitives and GlobalAlloc bridge

`verus/machine-model/platform_primitives_capsule.rs` is a direct-Verus,
`no_std`, no-vstd, no-cheating model for the boot allocator ABI and the compiler
memory operations needed by the kernel. It verifies 39 obligations over:

- a 111-byte allocation capsule and a 12-byte seal capsule;
- 9-byte `memcpy` and 14-byte `memset` capsules;
- a 64 KiB page-aligned boot arena with explicit cursor and sealed state;
- exact alignment, bounds, null-on-failure, unchanged-state-on-failure, and
  post-seal refusal semantics;
- exact-image decoders that expose semantics only for the registered bytes; and
- the complete kernel-code-model `GlobalAlloc` adapter body shapes, forwarding
  arguments, and relocation offsets.

`kernel-host/platform/global_allocator.rs` is the pinned Rust ABI adapter. It
contains the real `GlobalAlloc` implementation and boot-arena storage, but it is
accepted only when the extracted rustc object matches every registered function
body, relocation, undefined symbol, and arena-layout constraint. `dealloc` is a
boot-lifetime no-op; `realloc` and `alloc_zeroed` deliberately return null; and
the arena rejects every allocation after `tmk_global_alloc_seal`.

The following command was run successfully:

```text
cargo run -p xtask -- m0-platform-primitives
```

It produced three byte-identical model rlibs, three byte-identical adapter
rlibs, the four exact executable primitive sections, and three byte-identical
freestanding low-address links and three byte-identical higher-half links. A
hosted `no_std` consumer used real `Box` and `Vec`, checked unsupported alignment
and post-seal rejection, and exercised the actual
`memcpy`/`memset` symbols. A static freestanding consumer used `Box`, `Vec`, and
sealing, linked with no unresolved symbols, and remained in its fail-stop loop
until the expected timeout. A second static image placed `_start` exactly at
`0xffffffff80000000`, proved that the audited signed relocations resolve in the
intended kernel code model, re-extracted all four primitives, and matched them
byte-for-byte to their emitted registered images. The memory-operation semantics
explicitly require valid ranges, non-overlap for `memcpy`, and DF clear. Mutated
allocation bytes, a false allocation-address theorem, injected `assume`,
arena-layout drift, and accidental use of the small instead of kernel code model
all failed their named gates.

Stable identities from the passing run:

```text
model_source_sha256=453be38acca7df6b3a040f74ac2ab8cfadd21a5d6099749ed73502cf5a14b38e
adapter_source_sha256=e027bdb1c387b92f7edbe264173bb9c74a665a5f9c39dc0b2c053feace9ddab0
auditor_sha256=8e179064e255dcd7c6d4ac71a04a087f8f6c0244c244dab92701d327dfa06ce
linker_script_sha256=24a79defc5a11fa1802301fc910667f9c13584b5206eb8aaf57d165fbe4dfba4
model_artifact_sha256=a3e502361cb16032919539adfc50d06c7955a474b1a15046467928ff9eacee0c
adapter_artifact_sha256=83ba4fb59da7d97db85cf706a9a529f1bce409926ed2e30ddd45a18d79f536d4
primitive_object_sha256=a3884a20bfb8193e6cfdbf921eae60bac038406aebfe9184ad5039e0629ec50f
hosted_consumer_sha256=5aecea604b4a30787027ceec86e9f73e7793167b3ba000f76a4a56eefa3d3867
freestanding_consumer_sha256=a8e7678970872365dbb6245856522b57d49d7bd61e3f3c269d61280303dcaf5e
high_half_consumer_sha256=8d0d4f96d1c40045df9ab8a19c2668759a3aaef02bdf07fe7ebda4122013ec62
alloc_capsule_sha256=527c8b7e5bf504dd1a7ea834265a7686fb5365fd0148689fa6ca25a50cca4492
seal_capsule_sha256=e26eb14130f2dc8b6e0141f88d6c70ce28ed75a840168c34cd022568e7a366fe
memcpy_capsule_sha256=00d0174466d21d8a224c588f4bf2e324a605806fac0cb0d4fa2ba1a9667a49a9
memset_capsule_sha256=0d0dc6ef40ca8da2d7833d483b172e4d74418b38446b7658af583e94e9c0e3dc
runtime_marker=M0_GLOBAL_ALLOC_OK:box:vec:reject:sealed
freestanding_runtime=fail-stop-timeout-124
high_half_link_base=ffffffff80000000
report_sha256=4de332a6cae6720e26af3e891f8104525c395951006715426ac24f7828273a70
```

The primitive object also resolves the compiler-emitted `memcpy` in the real
rich-state composition consumer. The development-manifest gate now rechecks and
binds the model, adapter, primitive object, higher-half image, emitted bytes,
post-link bytes, complete report identities, and runtime/negative-test report.
This checkpoint is not release-eligible by itself. Deterministic composition
receipt replay and receipted final linking pass their component gate, and the
clean signed M0 development manifest now binds that evidence and the selected
exact `memcpy` bytes.
