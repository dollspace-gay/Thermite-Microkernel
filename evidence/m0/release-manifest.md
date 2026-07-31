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
`c11e2f45b8e339c7a0e0d7825e31dcd5975609c5`:

```text
cargo run -p xtask -- m0-manifest
```

The command rebuilt a development manifest from the real M0 standalone Forge
receipt, proof results, generated ABI, capsules, component ELF, UEFI entry-model
rlib, PE loader, raw FAT image, and acceptance reports. It rechecked the Forge
receipt's source and artifact digests, independently reparsed the PE and FAT
media, required exact TCG and KVM marker logs, replayed every bound artifact's
size and SHA-256 from disk, and required internal receipt/proof/artifact
references to agree. The tool inventory now also distinguishes pinned
firmware/hypervisor environment identities from trusted build tools.

The canonical signing payload retains the algorithm, key ID, and public-key
digest while omitting only the recursively dependent payload-digest and
signature fields. Pinned OpenSSL 3.2.6 signed that payload with Ed25519 in three
independent directories. All three manifests and 64-byte signatures were
identical and successfully verified.

The committed M0 key encoding is deliberately public test material. Validator
policy permits key ID `m0-development-test-key` only when `development=true` and
`release_eligible=false`; it cannot authorize a release image. Production
private keys remain external inputs.

Thirteen negative cases were exercised. Unknown properties, capsule byte drift,
an unknown source binding, a direct-Verus/artifact digest mismatch, general and
boot-image-specific on-disk digest mismatches, noncanonical ordering, a
development-key release claim, releases without composition or without exactly
one boot image, schema loosening, signature mutation, and signed-payload mutation
all failed at their named gates.

Stable clean-run identities:

```text
schema_sha256=cb150d54086c83a404c3bb133831a389a63191572e74fb22509b32b32bce713b
validator_sha256=730f9b3419654123eeca9a65304a81a4e53148845bee4f557167639f0b0653c4
manifest_sha256=d9a603a55e50876c65a5245f37f1a04ee0b6c2583cfc87e5d76a651467c92f57
payload_sha256=a09f8c6c65609407165422e8e8bd42c9f2205b970b13a83e6c08972f20ba14a5
signature_sha256=0154aef64342e1905fc3275c5c76ab9ac665c57a243054106aa73c730867fa72
public_key_sha256=c40b867d852bc86bb825aceb2600ffe03ea18cfb1a046108e23b2cfd1c47ea7b
negative_results_sha256=0f321d3d22b531cb4965d5096494aeeb1485254451a9077a13514576c5033285
report_sha256=71868369de80a909146a434f450cea85514c91002d866e2b01ceabe7781f7a57
```

The generated manifest accurately states `release_eligible=false`: it has a boot
image and its proof/test bindings, but no rich-state composition receipt or
receipted final-link allowlist, and the raw-pointer allocator ABI remains open.
The schema deliverable is complete, but a signed development manifest is not a
release artifact.
