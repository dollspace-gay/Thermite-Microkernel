# Normative and informative references

## 1. Normative platform specifications

- [UEFI Specification 2.11](https://uefi.org/specs/UEFI/2.11/) — UEFI image,
  boot-services, memory-map, and handoff behavior.
- [ACPI Specification 6.6](https://uefi.org/specs/ACPI/6.6/) — RSDP/XSDT, MADT,
  MCFG, and platform-table formats.
- [Intel 64 and IA-32 Architectures Software Developer’s Manual](https://www.intel.com/content/www/us/en/content-details/843820/intel-64-and-ia-32-architectures-software-developer-s-manual-combined-volumes-1-2a-2b-2c-2d-3a-3b-3c-3d-and-4.html)
  — paging, privilege, exceptions, interrupts, APIC, MSRs, and instructions.
- [Intel Virtualization Technology for Directed I/O Architecture Specification](https://www.intel.com/content/www/us/en/content-details/868911/intel-virtualization-technology-for-directed-i-o-architecture-specification.html)
  — DMA and interrupt remapping.
- [Virtual I/O Device (VirtIO) Version 1.3](https://docs.oasis-open.org/virtio/virtio/v1.3/virtio-v1.3.html)
  — modern PCI transport, queues, block, network, and RNG devices.
- [System V AMD64 ABI](https://gitlab.com/x86-psABIs/x86-64-ABI) — x86_64 user
  process and C calling conventions.

The implementation lock records exact downloaded document revisions/digests.
When a newer specification appears, an upgrade is reviewed; “latest” is never an
unpinned build input.

## 2. POSIX

- [The Open Group Base Specifications Issue 8 / POSIX.1-2024](https://pubs.opengroup.org/onlinepubs/9799919799/)
  — base definitions, system interfaces, shell, utilities, and rationale.
- [POSIX.1-2024 scope](https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap01.html)
  — source-level operating-system interface and environment.

TMK advertises only its staged compatibility profile until conformance evidence
supports a broader claim.

## 3. Verification

- [Verus repository](https://github.com/verus-lang/verus) — verified Rust
  implementation and releases.
- [Verus assumptions and trusted components](https://verus-lang.github.io/verus/guide/tcb.html)
  — the trust implications of assumptions, external bodies, and external items.
- [Verus attributes](https://verus-lang.github.io/verus/guide/reference-attributes.html)
  — especially `external` and `external_body`.
- [Verus: A Practical Foundation for Systems Verification](https://verus-lang.github.io/paper-sosp24-artifact/assets/paper-20240921-162720-b7db935.pdf)
  — Verus architecture and systems-verification approach.
- [Comprehensive Formal Verification of an OS Microkernel](https://sel4.systems/Research/pdfs/comprehensive-formal-verification-os-microkernel.pdf)
  — informative refinement/assurance precedent; not a proof of TMK.

## 4. Virtual platform and firmware

- [QEMU VirtIO device documentation](https://qemu.readthedocs.io/en/master/system/devices/virtio/index.html)
  — QEMU VirtIO integration.
- [QEMU security model](https://qemu.readthedocs.io/en/master/system/security.html)
  — host/guest virtualization context.
- [TianoCore OVMF FAQ](https://github.com/tianocore/tianocore.github.io/wiki/OVMF-FAQ)
  — OVMF’s role as UEFI firmware for virtual machines.

QEMU/OVMF documentation is informative. The release uses pinned binaries plus
device introspection and conformance tests.

## 5. Runtime and terminology

- [musl libc](https://musl.libc.org/) — libc port baseline.
- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) and
  [RFC 8174](https://www.rfc-editor.org/rfc/rfc8174) — normative requirement
  language.

## 6. Thermite toolchain baseline

The design was read against the local Thermite repository at commit:

```text
902f29242c068190320c1e1e1f702fb933e0dda6
```

The canonical generated language reference at design time had SHA-256:

```text
cd37b3e309696a1512f6eef167911a498876cc0a49c138d1357c84f07efa3e29
```

This baseline includes correspondence-backed L3 build bundles from Thermite
issue #101 / implementation PR #102. TMK's normative consumption and remaining
same-crate composition requirements are defined in
[Forge L3 integration](15-forge-integration.md).

Normative Thermite behavior for implementation comes from the pinned Forge
binary’s generated skill/reference, not from recollection or these architecture
documents. Release tooling checks reference freshness before proof.
