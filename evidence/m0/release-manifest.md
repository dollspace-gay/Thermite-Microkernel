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
`05ccfc9c5e60f43aae5740d0676242e579b5e28b`:

```text
cargo run -p xtask -- m0-manifest
```

The command rebuilt a development manifest from the real M0 standalone Forge
receipt, proof results, generated ABI, capsules, component ELF, platform model,
`GlobalAlloc` adapter, primitive object, higher-half image, UEFI entry-model rlib,
PE loader, raw FAT image, and acceptance reports. It rechecked every field in the
platform report against the current sources and artifacts, required emitted and
post-link primitive digests to agree, rechecked the Forge receipt's source and
artifact digests, independently reparsed the PE and FAT media, required exact TCG
and KVM marker logs, replayed every bound artifact's size and SHA-256 from disk,
and required internal receipt/proof/artifact references to agree. The tool
inventory distinguishes pinned firmware/hypervisor environment identities from
trusted build tools.

The canonical signing payload retains the algorithm, key ID, and public-key
digest while omitting only the recursively dependent payload-digest and
signature fields. Pinned OpenSSL 3.2.6 signed that payload with Ed25519 in three
independent directories. All three manifests and 64-byte signatures were
identical and successfully verified.

The committed M0 key encoding is deliberately public test material. Validator
policy permits key ID `m0-development-test-key` only when `development=true` and
`release_eligible=false`; it cannot authorize a release image. Production
private keys remain external inputs.

Fourteen negative cases were exercised. Unknown properties, capsule byte drift,
an unknown source binding, a direct-Verus/artifact digest mismatch, general,
platform-primitive-specific, and boot-image-specific on-disk digest mismatches,
noncanonical ordering, a development-key release claim, releases without
composition or without exactly one boot image, schema loosening, signature
mutation, and signed-payload mutation all failed at their named gates.

Stable clean-run identities:

```text
schema_sha256=d1758fedaf4a257d5a2aa0799491c456c722a56764dd744ac20c9fec63722d84
validator_sha256=730f9b3419654123eeca9a65304a81a4e53148845bee4f557167639f0b0653c4
manifest_sha256=21a00a767a2352b5390210f4afb969b13801341561ebb51ba2ea602442cfc9a7
payload_sha256=587ee06c1bd79cb9875225f486870cedae8d9d8cfbedb41fba7bd4bbcdf07af7
signature_sha256=966ee0d3b73dc5f0cae618edf86ea8755d891b43906295567d6aaa51cda6b4f6
public_key_sha256=c40b867d852bc86bb825aceb2600ffe03ea18cfb1a046108e23b2cfd1c47ea7b
negative_results_sha256=1bd89085f90b8a7c2ed2f67b958a092170397b9c055564ff68f2c939f62ecab5
report_sha256=e596b245b7159bcb58460252ec6e2703937a8b4c6b7e41896123ea16326f1209
```

The generated manifest accurately states `release_eligible=false`: it has a boot
image, the raw-pointer allocator ABI and platform-primitive bindings, and their
proof/test evidence, but no rich-state composition receipt or receipted
final-link allowlist. The schema deliverable is complete, but a signed
development manifest is not a release artifact.
