use anyhow::{Context, Result};
use std::path::Path;

use arrow_identity::crypto::ed25519::sign_ed25519;
use arrow_transport::{
    canonical::envelope_canonical_bytes,
    envelope::{EnvelopeSigningMetadata, TransportEnvelope},
};

/// Sign an existing envelope in place using a dev key file.
///
/// The DID embedded in the key file always wins as `sender_id` — the signature
/// is over the canonical bytes that include `sender_id`, so they must agree.
/// If the caller passed a `--from` value it is silently overridden; callers
/// should warn the user before calling this function.
///
/// **Dev / test only — key files carry unprotected private keys.**
pub fn sign_envelope_in_place(envelope: &mut TransportEnvelope, key_file: &Path) -> Result<()> {
    let key_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(key_file).context("reading key file")?)
            .context("parsing key file")?;

    let sk_b64 = key_json["private_key_b64"]
        .as_str()
        .context("missing private_key_b64 in key file")?;
    let key_ref = key_json["key_ref"]
        .as_str()
        .context("missing key_ref in key file")?;
    let did = key_json["did"]
        .as_str()
        .context("missing did in key file")?;

    let sk_bytes: Vec<u8> =
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, sk_b64)
            .context("decoding private key (expected STANDARD_NO_PAD base64)")?;
    let sk: [u8; 32] = sk_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("private key must be exactly 32 bytes"))?;

    // DID from key file wins — sender_id must equal the signing key DID (handlers.rs:237).
    envelope.sender_id = did.to_string();
    // signing must be absent during canonicalisation (canonical bytes exclude it).
    envelope.signing = None;

    let canonical = envelope_canonical_bytes(envelope)
        .map_err(|e| anyhow::anyhow!("canonicalisation failed: {e}"))?;
    let sig = sign_ed25519(&sk, &canonical);
    let sig_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD_NO_PAD, sig);

    envelope.signing = Some(EnvelopeSigningMetadata {
        alg: "Ed25519".into(),
        key_ref: key_ref.into(),
        sig_b64,
        created: Some(chrono::Utc::now().to_rfc3339()),
        canonicalization: "arrow-canonical-json-v0".into(),
        signed_fields: "envelope-minus-signature-and-signing".into(),
    });

    Ok(())
}
