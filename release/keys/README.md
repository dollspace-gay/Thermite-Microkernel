# Development manifest key

`m0-development-private.der.hex` is a deliberately public test-key encoding. It
exists only to exercise deterministic Ed25519 manifest signing and verification
during M0.
It MUST NOT authorize a production or release-eligible image. The manifest
validator enforces that key ID `m0-development-test-key` is usable only when
`development=true` and `release_eligible=false`.

Production signing keys are external release inputs. They are never committed to
this repository or copied into a build image.
