use crate::error::IdentityError;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// Verify an Ed25519 signature over `payload` using a 32-byte public key.
pub fn verify_ed25519(
    public_key_bytes: &[u8],
    payload: &[u8],
    signature_bytes: &[u8],
) -> Result<(), IdentityError> {
    if public_key_bytes.len() != 32 {
        return Err(IdentityError::MalformedPublicKey(format!(
            "Ed25519 public key must be 32 bytes, got {}",
            public_key_bytes.len()
        )));
    }

    let key_array: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| IdentityError::MalformedPublicKey("cannot convert to [u8; 32]".into()))?;

    let verifying_key = VerifyingKey::from_bytes(&key_array)
        .map_err(|e| IdentityError::MalformedPublicKey(format!("invalid Ed25519 key: {e}")))?;

    if signature_bytes.len() != 64 {
        return Err(IdentityError::MalformedSignature(format!(
            "Ed25519 signature must be 64 bytes, got {}",
            signature_bytes.len()
        )));
    }

    let sig_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| IdentityError::MalformedSignature("cannot convert to [u8; 64]".into()))?;

    let signature = Signature::from_bytes(&sig_array);

    verifying_key
        .verify(payload, &signature)
        .map_err(|_| IdentityError::SignatureMismatch)
}

/// Generate an Ed25519 keypair for testing. Returns (private_key_bytes, public_key_bytes).
/// **Test/dev only — not for production key management.**
pub fn generate_test_keypair() -> ([u8; 32], [u8; 32]) {
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;

    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    (signing_key.to_bytes(), verifying_key.to_bytes())
}

/// Generate an Ed25519 keypair from a fixed 32-byte seed.
///
/// DEV/TEST ONLY: this is for reproducible fixtures and demos. Do not use it
/// for production key management.
#[doc(hidden)]
pub fn generate_keypair_from_seed(seed: [u8; 32]) -> ([u8; 32], [u8; 32]) {
    use ed25519_dalek::SigningKey;

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();
    (signing_key.to_bytes(), verifying_key.to_bytes())
}

/// Sign `payload` with an Ed25519 private key. Returns the 64-byte signature.
/// **Test/dev only.**
pub fn sign_ed25519(private_key_bytes: &[u8; 32], payload: &[u8]) -> [u8; 64] {
    use ed25519_dalek::{Signer, SigningKey};

    let signing_key = SigningKey::from_bytes(private_key_bytes);
    let signature = signing_key.sign(payload);
    signature.to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_roundtrip() {
        let (sk, pk) = generate_test_keypair();
        let payload = b"Arrow Protocol canonical bytes";
        let sig = sign_ed25519(&sk, payload);
        assert!(verify_ed25519(&pk, payload, &sig).is_ok());
    }

    #[test]
    fn seeded_keypair_is_reproducible() {
        let seed = [42u8; 32];
        let first = generate_keypair_from_seed(seed);
        let second = generate_keypair_from_seed(seed);
        assert_eq!(first, second);
    }

    #[test]
    fn wrong_payload_fails() {
        let (sk, pk) = generate_test_keypair();
        let sig = sign_ed25519(&sk, b"original payload");
        assert!(matches!(
            verify_ed25519(&pk, b"tampered payload", &sig),
            Err(IdentityError::SignatureMismatch)
        ));
    }

    #[test]
    fn wrong_key_fails() {
        let (sk, _pk) = generate_test_keypair();
        let (_, wrong_pk) = generate_test_keypair();
        let payload = b"test";
        let sig = sign_ed25519(&sk, payload);
        assert!(matches!(
            verify_ed25519(&wrong_pk, payload, &sig),
            Err(IdentityError::SignatureMismatch)
        ));
    }

    #[test]
    fn bad_key_length_rejected() {
        let short_key = [0u8; 16];
        assert!(matches!(
            verify_ed25519(&short_key, b"x", &[0u8; 64]),
            Err(IdentityError::MalformedPublicKey(_))
        ));
    }

    #[test]
    fn bad_sig_length_rejected() {
        let (_, pk) = generate_test_keypair();
        assert!(matches!(
            verify_ed25519(&pk, b"x", &[0u8; 32]),
            Err(IdentityError::MalformedSignature(_))
        ));
    }
}
