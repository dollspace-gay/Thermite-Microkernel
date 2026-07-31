# Thermite Microkernel

This repository contains the design for a capability-oriented, POSIX-compatible
microkernel written primarily in Thermite and verified through Forge and Verus.
M0 toolchain and proof-artifact closure is complete. A freestanding verified
composition final-links at the higher-half address, and a reproducible M0 UEFI
probe image boots under OVMF with both TCG and KVM. M1 kernel/BSP implementation
has not started.

The first platform is x86_64 QEMU/KVM on the `q35` machine, booted as a UEFI
application through OVMF. The first useful release is single-core but is designed
for SMP from the beginning. The next release enables four-core execution after
memory isolation and IPC are stable.

The system is not a Linux clone and does not expose the Linux syscall ABI.
Applications obtain POSIX source compatibility through a libc port and user-space
servers over a small capability-native kernel ABI.

## Non-negotiable release rule

A release kernel may contain:

- Thermite functions certified L3 or L4 with end-to-end assurance.
- Verus-compatible Rust whose executable bodies verify.
- Small x86 instruction capsules whose exact linked bytes refine a Verus machine
  model.

A release kernel may not contain `#[slag]`, unresolved proof holes, Forge L0/L1/L2
downgrades, Verus `assume`, axioms, executable `external_body`, or unverified
hand-written assembly. The only permitted `external_body` syntax is paired with
`external_type_specification` to name an opaque foreign type whose representation
and operations are never used; it cannot suppress verification of executable
code. Firmware, the hypervisor, the proof tools, the compiler backend, and
hardware remain explicit environmental or trusted-tool assumptions; they are
never silently described as verified.

## Design map

1. [Goals and scope](docs/00-goals-and-scope.md)
2. [System architecture](docs/01-system-architecture.md)
3. [Assurance and trust](docs/02-assurance-and-trust.md)
4. [x86_64 platform](docs/03-x86-platform.md)
5. [Kernel objects, capabilities, and IPC](docs/04-kernel-core.md)
6. [Memory management](docs/05-memory-management.md)
7. [Scheduling and SMP](docs/06-scheduling-and-smp.md)
8. [Native ABI](docs/07-native-abi.md)
9. [POSIX personality](docs/08-posix-personality.md)
10. [User-space services and drivers](docs/09-userspace-services.md)
11. [Security and recovery](docs/10-security-and-recovery.md)
12. [Build, verification, and release](docs/11-build-and-release.md)
13. [Milestones and acceptance gates](docs/12-milestones-and-acceptance.md)
14. [Decisions, gaps, and risks](docs/13-decisions-gaps-risks.md)
15. [Normative references](docs/14-references.md)
16. [Forge L3 integration and verified composition](docs/15-forge-integration.md)

## Normative language

`MUST`, `MUST NOT`, `REQUIRED`, `SHALL`, `SHALL NOT`, `SHOULD`, `SHOULD NOT`, and
`MAY` are normative. A requirement is not implemented merely because it appears
in these documents. Implementation status is tracked separately from design
status.

The working project name in these documents is **TMK**. It is not a commitment to
a final product name.

## Implementation status

M0 toolchain closure is complete. The public repository, pinned Cargo workspace,
replayable standalone and rich-state Forge paths, verified platform layer,
receipted higher-half link, reproducible UEFI probe, and signed development
manifest have all passed their acceptance gates. M1 verified UEFI/BSP bring-up is
next; the kernel proper has not started.

The standalone probe's toolchain-binding gate is locally closed by pinned
Thermite `v0.0.2` commit `845d684f00e829491ee4c537818fba2689bcaefc`. Forge records both
ambient Rust 1.96 and the authoritative Verus-selected Rust 1.95 codegen closure;
TMK selects the consumer compiler from the bound evidence, links and executes it,
and confirms that the incompatible host compiler is rejected. Upstream issue
[#103](https://github.com/dollspace-gay/Thermite/issues/103) is closed by PR #105,
and the coordinated pin retains that repair.

The rich-state acceptance transition and its direct-Verus shell are implemented
and accepted with the deterministic exact-source composition repair from
Thermite [#104](https://github.com/dollspace-gay/Thermite/issues/104). Three
independent composition builds reproduce the combined source, receipt, and rlib;
validation and replay pass. A hosted consumer executes authorized and rejected
transitions, private rich exports remain inaccessible, incompatible rustc is
rejected, and reproducible low/higher-half links retain only the selected proved
`memcpy` bytes. A fresh second absolute source root reproduces the receipt, rlib,
both final images, and linked primitive byte-for-byte. Eleven independent
composition mutations fail. A canonical final-link receipt binds every input,
tool, selected/discarded symbol, linked output, runtime result, and two-root
reproducibility result. The clean signed development manifest independently
replays and binds the component and receipt.

The direct-Verus allocation layer now closes the raw-pointer ABI as well as the
fixed-unit and byte/layout policies. A 39-obligation Verus machine model owns the
111-byte bump allocator, 12-byte seal operation, and exact `memcpy`/`memset`
encodings, with exact-image decoders connecting the registered bytes and shim
relocations to their semantics. A pinned minimal Rust `GlobalAlloc` ABI adapter
is admitted only when its complete function skeletons, relocation targets, arena
size/alignment, and undefined-symbol set match the registered model. Three model
rlibs, adapter rlibs, and static links reproduce byte-for-byte. A hosted consumer
actually runs `Box`, bounded-capacity `Vec`, rejected alignment, post-seal
failure, copy, and set operations; fully static low-address and
`0xffffffff80000000` higher-half consumers final-link reproducibly with no
unresolved symbol. The boot allocator deliberately returns null for `realloc`
and `alloc_zeroed` and is sealed before AP startup.

The native ABI now has a strict single-source IDL generator. It emits C11 and
`repr(C)` Rust definitions with compile-time layout assertions, reproduces all
outputs in three independent directories, compiles in hosted C/Rust and
freestanding `no_std` Rust contexts, and runs cross-language tag, capability-path,
and UTCB layout checks. Five malformed-schema/generated-output mutations are
rejected. This closes the M0 generator deliverable, not the later decoder-proof,
fuzzing, or failure-atomicity gates.

The M0 release-manifest schema is also executable. A clean run binds the actual
Forge receipt, direct-Verus results, capsule bytes, generated ABI, component ELF,
tool identities, assumptions, and test reports; replays each artifact digest;
then produces and verifies three byte-identical Ed25519-signed development
manifests. The manifest now reparses and binds the verified UEFI entry model,
PE loader, raw FAT image, pinned firmware/hypervisor tools, TCG/KVM observations,
platform model, `GlobalAlloc` adapter, primitive object, higher-half image, and
exact emitted/post-link primitive bytes, rich-state composition receipt,
receipted higher-half final link, and selected `memcpy` bytes. The manifest
reruns composition replay and final-ELF audits, reproduces three signatures, and
rejects seventeen mutations. The public M0 test key is policy-locked to
non-release development manifests and cannot authorize production.

The M0 x86 capsule is also live: Verus proves the exact encoding and machine-state
transition for `mov rax,rdi; hlt`; the emitted bytes survive object conversion and
static linking unchanged, with relocation, executable-section, symbol, and
disassembly audits plus four negative tests.

The reproducible empty-image gate now boots real media rather than a host-only
fixture. Verus proves a 56-byte EFI entry capsule that preserves the incoming
non-result registers, emits `TMK_M0_UEFI_OK!` through the QEMU debug port, returns
`EFI_SUCCESS`, and rejects every other registered encoding. Three rlibs, three
1 KiB PE32+ applications, and three 32 MiB FAT16 images reproduce byte-for-byte.
An independent PE/FAT parser checks the fallback path and exact executable bytes;
OVMF observes the marker under TCG and KVM, while a corrupt PE produces none.
Eight negative cases pass. This image remains a development probe, not an M1
loader or a release-eligible composed kernel.

Useful implementation commands:

```text
cargo run -p xtask -- toolchain-check
cargo run -p xtask -- m0-idl
cargo run -p xtask -- m0-manifest
cargo run -p xtask -- m0-uefi
cargo run -p xtask -- m0-forge-probe
cargo run -p xtask -- m0-forge-tamper
cargo run -p xtask -- m0-composition-source-check
cargo run -p xtask -- m0-composition
cargo run -p xtask -- m0-verus-allocator
cargo run -p xtask -- m0-verus-byte-allocator
cargo run -p xtask -- m0-verus-capsule
cargo run -p xtask -- m0-platform-primitives
cargo run -p xtask -- m0-host-link
```

The `m0-forge-probe` command is the strict standalone release gate. It accepts no
compiler override: the Rust consumer compiler is selected from the receipt-bound
Forge toolchain evidence and a passing report states `release_eligible=true`.

Cargo build directories are cleaned after evidence is captured at each milestone
boundary. Proof bundles and runtime reports live under ignored `build/` paths;
reviewed summaries live under `evidence/`.
