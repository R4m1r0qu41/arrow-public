use anyhow::{Context, Result};
use std::path::Path;

use arrow_identity::{
    crypto::ed25519::generate_test_keypair,
    resolver::{
        did_key::{did_key_from_public, DidKeyResolver},
        DidResolver,
    },
};

/// Resolve a DID and print the DID Document.
pub fn resolve(did: &str) -> Result<()> {
    let resolver = DidKeyResolver;
    let doc = resolver.resolve(did).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{}", serde_json::to_string_pretty(&doc)?);
    Ok(())
}

/// Generate a test Ed25519 keypair and print the DID + key file.
/// **Dev/test only — not for production key management.**
pub fn generate(output: &Path) -> Result<()> {
    let (sk, pk) = generate_test_keypair();
    let did = did_key_from_public(&pk);

    // Build key_ref
    let mut prefixed = vec![0xed, 0x01];
    prefixed.extend_from_slice(&pk);
    let method_specific = format!("z{}", bs58::encode(&prefixed).into_string());
    let key_ref = format!("{did}#{method_specific}");

    let key_file = serde_json::json!({
        "did": did,
        "key_ref": key_ref,
        "algorithm": "Ed25519",
        "private_key_b64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD_NO_PAD, sk),
        "public_key_b64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD_NO_PAD, pk),
        "warning": "DEV/TEST ONLY — do not use in production"
    });

    std::fs::write(output, serde_json::to_string_pretty(&key_file)?)?;
    println!("DID:     {did}");
    println!("Key ref: {key_ref}");
    println!("Written: {}", output.display());
    Ok(())
}

/// Sign an envelope JSON file using a dev key file. Writes a new signed file.
/// Delegates signing to [`super::signing::sign_envelope_in_place`].
pub fn sign(key_file: &Path, envelope_path: &Path, output: &Path) -> Result<()> {
    let mut envelope: arrow_transport::envelope::TransportEnvelope =
        serde_json::from_str(&std::fs::read_to_string(envelope_path).context("reading envelope")?)
            .context("parsing envelope")?;

    super::signing::sign_envelope_in_place(&mut envelope, key_file)?;

    std::fs::write(output, serde_json::to_string_pretty(&envelope)?)?;
    println!("Signed envelope written: {}", output.display());
    Ok(())
}
