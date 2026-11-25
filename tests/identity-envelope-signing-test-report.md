# Arrow Identity Envelope Signing — Test Report

Status: Experimental Reference Pilot  
Date: 2026-05-24  
Branch: main  
Commit: 5a43605

## Purpose

This report records the test evidence for the Arrow Protocol DID-based envelope signing layer.

The identity layer introduces cryptographic sender authentication for Arrow transport envelopes using Decentralized Identifiers (DIDs), Ed25519 signatures, canonical envelope bytes, and node-level signature admission policy.

## Scope

Covered:

- `arrow-identity` crate integration.
- `did:key` identity resolution.
- Ed25519 signature verification.
- Envelope signing metadata.
- Canonicalization with signature fields removed.
- Node-level signature requirement policy.
- Sender binding between `signing.key_ref` and `envelope.sender_id`.
- DID method allowlist validation.
- Negative tests for tampering, wrong signer, malformed DID method prefix, wrong `signed_fields`, and wrong `canonicalization`.
- CLI identity utilities for development/testing.

Not covered:

- `did:ion`
- `did:ethr`
- secp256k1
- delegation
- key rotation
- revocation
- Verifiable Credentials
- trust registries
- production key management
- legal entity trust registries
- on-chain identity registry integration

## Architecture Under Test

Workspace crates:

- `arrow-core`
- `arrow-transport`
- `arrow-identity`
- `arrow-node`
- `arrow-cli`

Identity boundary:

- `arrow-identity` verifies generic canonical bytes plus signing metadata.
- `arrow-identity` must not parse full Arrow envelopes.
- `arrow-identity` must not import `arrow-core`, `arrow-transport`, `arrow-node`, or `arrow-cli`.
- `arrow-transport` owns the envelope and canonicalization rules.
- `arrow-node` owns signature admission policy and HTTP error mapping.
- `arrow-cli` exposes development/test identity commands.

## Canonicalization Rule

The signed payload is the canonical Arrow transport envelope with both legacy `signature` and modern `signing` fields removed before canonicalization.

Canonical signed-fields label:

    envelope-minus-signature-and-signing

Required canonicalization label:

    arrow-canonical-json-v0

## Security Invariants Tested

- `signing.key_ref` must identify the same Decentralized Identifier (DID) as `envelope.sender_id`.
- A valid signature from a different DID must not authenticate an allowlisted sender.
- DID method allowlisting must use exact DID method extraction, not raw string prefix matching.
- `did:keyevil:...` must not match `did:key`.
- Unsupported `canonicalization` values must be rejected.
- Unsupported `signed_fields` values must be rejected.
- Unsigned envelopes must be accepted only when `[identity].require_signature = false`.
- Unsigned envelopes must be rejected when `[identity].require_signature = true`.
- Signature verification must occur before replay nonce burn.

## Node Identity Configuration

    [identity]
    require_signature = true
    allowed_did_methods = ["did:key"]
    allowed_algorithms = ["Ed25519"]

## Commands Executed

    cargo fmt --all
    cargo check --all
    cargo test --all
    cargo build --release

Boundary check:

    grep -R "arrow_core\|arrow-transport\|arrow_transport" crates/arrow-identity/src crates/arrow-identity/Cargo.toml

Expected boundary check result:

    no output

## Test Results

Full workspace test suite:

    cargo test --all

Expected result:

    74 tests passed

Breakdown:

- `arrow_identity`: 16 passed
- `arrow_node` integration: 25 passed
- `arrow_transport`: 19 passed
- `arrow_cli` integration: 9 passed
- `arrow_core`: 5 passed

Release build:

    cargo build --release

Expected result:

    Finished release profile successfully

## Identity Test Checklist

| Test | Expected Result | Status |
|---|---|---|
| `cargo test --all` runs `arrow_identity` | `arrow_identity` appears in test run | Passed |
| Signed envelope accepted | HTTP 200 / accepted | Passed |
| Unsigned envelope rejected when signatures required | HTTP 401 / signature missing | Passed |
| Tampered envelope rejected | HTTP 401 / signature invalid | Passed |
| Valid signature by different DID rejected | HTTP 401 / signature invalid | Passed |
| `did:keyevil:...` does not satisfy `did:key` | Rejected | Passed |
| Wrong `signed_fields` rejected | Rejected | Passed |
| Wrong `canonicalization` rejected | Rejected | Passed |
| Replay protection still works | Replay rejected | Passed |
| Signature verification before nonce burn | Invalid signature does not consume nonce | Passed |

## CLI Identity Commands

Development/test commands:

    arrow identity generate --output dev-ed25519.json
    arrow identity resolve did:key:z...
    arrow identity sign --key-file dev-ed25519.json --envelope raw.json --output signed.json

Warning:

Generated CLI key files are development-only and may contain unencrypted private key material. They are not production key management.

## Known Limitations

This is an experimental reference pilot, not production identity infrastructure.

Known limitations:

- Development key generation writes local key material for tests.
- `did:web` is feature-gated and not the primary pilot path.
- No production key custody model.
- No key rotation.
- No revocation.
- No delegated signer model.
- No trust registry.
- No legal entity registry.
- No Verifiable Credential layer.
- No `did:ion` or `did:ethr` resolver.

## Conclusion

The Arrow Protocol main branch now includes a working experimental DID-based identity layer for transport-envelope authentication.

The current implementation validates the core authentication claim:

A declared Arrow sender must cryptographically action the DID referenced in the envelope signing metadata, and the signature must verify over canonical envelope bytes with signature-bearing fields removed.

The implementation is suitable as a reference pilot and testable foundation for future identity extensions, but it should not be described as production-ready identity, custody, delegation, or legal-entity trust infrastructure.
