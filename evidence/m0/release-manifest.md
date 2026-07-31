# M0 release manifest schema evidence

`release/manifest.schema.json` defines the strict
`tmk.release-manifest.v1` envelope. Its executable validator accepts a bounded
JSON Schema draft 2020-12 subset and rejects unsupported schema keywords,
unresolved references, optional object fields, and objects that permit unknown
properties. The manifest records release/platform identity, source revisions,
trusted tools, per-function assurance and scope, Forge receipts, direct-Verus
results, exact-byte capsules, artifacts, test results, TCB/environmental
assumptions, format versions, limitations, and signing metadata.

The following command was run successfully from clean public commit
`7d47d395570fd1d8311fe07eef8b7a40dbc0a462`:

```text
cargo run -p xtask -- m0-manifest
```

The command rebuilt a development manifest from the real M0 standalone Forge
receipt, composition receipt, proof results, generated ABI, capsules, component
ELFs, platform model, `GlobalAlloc` adapter, primitive object, final-link receipt,
higher-half composition image, UEFI entry-model rlib, PE loader, raw FAT image,
and acceptance reports. It rechecked every field in the platform and composition
reports against current sources and artifacts, reran composition receipt replay,
validated the canonical final-link receipt and its full dependency/tool/input/
output allowlist, reran the ELF undefined-symbol, selected/discarded-symbol, and
entry-point audits, and independently re-extracted the linked `memcpy` bytes.
It also reparsed the PE and FAT media, required exact TCG and KVM marker logs,
replayed every bound artifact's size and SHA-256 from disk, and required internal
receipt/proof/artifact references to agree. The tool inventory distinguishes
pinned firmware/hypervisor environment identities from trusted build tools.

The canonical signing payload retains the algorithm, key ID, and public-key
digest while omitting only the recursively dependent payload-digest and
signature fields. Pinned OpenSSL 3.2.6 signed that payload with Ed25519 in three
independent directories. All three manifests and 64-byte signatures were
identical and successfully verified.

The committed M0 key encoding is deliberately public test material. Validator
policy permits key ID `m0-development-test-key` only when `development=true` and
`release_eligible=false`; it cannot authorize a release image. Production
private keys remain external inputs.

Seventeen negative cases were exercised. Unknown properties, capsule byte drift,
an unknown source binding, a direct-Verus/artifact digest mismatch, general,
platform-primitive-specific, composition-artifact-specific, final-link-receipt-
specific, and boot-image-specific on-disk digest mismatches, noncanonical
ordering, a development-key release claim, and release claims lacking
composition, a unique boot image, or a unique final-link receipt, schema
loosening, signature mutation, and signed-payload mutation all failed at their
named gates.

Stable clean-run identities:

```text
composition_binding_sha256=70e3aa411eb9c2944eb50d67613480852775475782043802c80f1ae8b2a1a9dd
final_link_receipt_sha256=3b5492f9129a403525d420580395ff06e248af43d0f7a30f210a79a9a5b200ea
schema_sha256=76524330998018c40cf89c69b90c10b7cd60b5b316db50b09b8ff56a5dc63000
validator_sha256=9534135218ba3559eacb68d3c1ce1db086ab49841148a115d492faa0f68f3d61
orchestrator_sha256=18d276a1177d3521ccda9ae44babc4773be687c51876deaddfa8dcc98d81c6d0
manifest_sha256=1148748333e8fa67edeea572539b1191c1d27e09b50cf7b4b79300986e20b749
payload_sha256=8ed16b68d6f9182846f2d76ae54c6e0db03b12957f116c0bf6075ab3154774da
signature_sha256=bcac17ef6f422927725c94ccade98268dec6a66d9d4d1b6689c27c0de1b14940
public_key_sha256=c40b867d852bc86bb825aceb2600ffe03ea18cfb1a046108e23b2cfd1c47ea7b
negative_results_sha256=e44885673548badeca5b694629e7c2233d5f123a5c33b46f47c310fbc672a51d
report_sha256=3d082573652c583533ecb454c400d90ef20ed3e1c6193d47701c27bbada8435e
```

The generated manifest accurately states `release_eligible=false`: it is a
development artifact signed by the public M0 test key, not a production release.
It now contains the replayed rich-state composition receipt, receipted final-link
allowlist, higher-half kernel ELF, platform primitives, boot image, and all proof/
test bindings required for M0 closure. A production manifest still requires an
external key and the later release input set.
