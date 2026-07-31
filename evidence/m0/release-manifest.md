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
`3ae19b7f53ec3a29c68f721a0bc8d68f4b56373c`:

```text
cargo run -p xtask -- m0-manifest
```

The command rebuilt a development manifest from the real M0 standalone Forge
receipt, proof results, generated ABI, capsule, component ELF, and acceptance
reports. It rechecked the Forge receipt's source and artifact digests, replayed
every bound artifact's size and SHA-256 from disk, and required internal
receipt/proof/artifact references to agree.

The canonical signing payload retains the algorithm, key ID, and public-key
digest while omitting only the recursively dependent payload-digest and
signature fields. Pinned OpenSSL 3.2.6 signed that payload with Ed25519 in three
independent directories. All three manifests and 64-byte signatures were
identical and successfully verified.

The committed M0 key encoding is deliberately public test material. Validator
policy permits key ID `m0-development-test-key` only when `development=true` and
`release_eligible=false`; it cannot authorize a release image. Production
private keys remain external inputs.

Eleven negative cases were exercised. Unknown properties, capsule byte drift,
an unknown source binding, a direct-Verus/artifact digest mismatch, an on-disk
artifact digest mismatch, noncanonical ordering, a development-key release
claim, a release without composition, schema loosening, signature mutation, and
signed-payload mutation all failed at their named gates.

Stable clean-run identities:

```text
schema_sha256=31ed546a08737cb2a2c0347c58aa58b1a6932fa64d5d53396137f09eb253c598
validator_sha256=730f9b3419654123eeca9a65304a81a4e53148845bee4f557167639f0b0653c4
manifest_sha256=f1f9d647e792e9f4628349a99d629a1ae681538d858416f03843f9dce6d46e13
payload_sha256=b84b96177cc5e208a237aaee7ebaa95818bb4be413f89a55c5ea3000241b6e99
signature_sha256=2af436bc2b2fc2ae54741b9e47cd71f022b1db7650c8683cf2b93681bf1fcabf
public_key_sha256=c40b867d852bc86bb825aceb2600ffe03ea18cfb1a046108e23b2cfd1c47ea7b
negative_results_sha256=bb2d60b337009e2686d679d80ec3cf1b2311b19229dc67d0958190457e2afd81
report_sha256=a166eb43c8883201145cc632d85dd21ccc2de1a58ad6c725e6abf8971cf6b9aa
```

The generated manifest accurately states `release_eligible=false`: it has no
rich-state composition receipt or boot image, and the raw-pointer allocator ABI
remains open. The schema deliverable is complete, but a signed development
manifest is not a release artifact.
