use crate::error::IdentityError;
use crate::resolver::DidResolver;
use crate::types::{DidDocument, VerificationMethod, VerificationRelationship};

/// Ed25519 multicodec prefix: 0xed01 (varint-encoded as [0xed, 0x01]).
const ED25519_MULTICODEC: [u8; 2] = [0xed, 0x01];

/// Deterministic, offline resolver for `did:key` (Ed25519 only).
///
/// Resolution is pure computation: the public key is encoded directly in the DID
/// string. No network access, no state, no blockchain.
pub struct DidKeyResolver;

impl DidResolver for DidKeyResolver {
    fn resolve(&self, did: &str) -> Result<DidDocument, IdentityError> {
        resolve_did_key(did)
    }
}

/// Resolve a `did:key` DID string to a minimal DID Document.
pub fn resolve_did_key(did: &str) -> Result<DidDocument, IdentityError> {
    // Validate prefix
    let method_specific = did
        .strip_prefix("did:key:")
        .ok_or_else(|| IdentityError::MalformedDid(format!("`{did}` is not a did:key DID")))?;

    if method_specific.is_empty() {
        return Err(IdentityError::MalformedDid(
            "did:key method-specific identifier is empty".into(),
        ));
    }

    // The method-specific identifier is a multibase-encoded value.
    // For Ed25519, it starts with 'z' (base58btc).
    if !method_specific.starts_with('z') {
        return Err(IdentityError::MalformedDid(format!(
            "did:key identifier must start with 'z' (base58btc), got `{}`",
            &method_specific[..1.min(method_specific.len())]
        )));
    }

    // Decode base58btc (strip the 'z' prefix which is the multibase code for base58btc)
    let decoded = bs58::decode(&method_specific[1..])
        .into_vec()
        .map_err(|e| IdentityError::MalformedDid(format!("base58btc decode failed: {e}")))?;

    // Check multicodec prefix (Ed25519 = 0xed 0x01)
    if decoded.len() < 2 {
        return Err(IdentityError::MalformedDid(
            "decoded key material is too short for multicodec prefix".into(),
        ));
    }

    if decoded[0..2] != ED25519_MULTICODEC {
        return Err(IdentityError::UnsupportedKeyType(format!(
            "multicodec prefix [{:#04x}, {:#04x}] is not Ed25519 (expected [0xed, 0x01])",
            decoded[0], decoded[1]
        )));
    }

    // Extract 32-byte public key
    let key_bytes = &decoded[2..];
    if key_bytes.len() != 32 {
        return Err(IdentityError::MalformedPublicKey(format!(
            "Ed25519 public key must be 32 bytes after multicodec prefix, got {}",
            key_bytes.len()
        )));
    }

    // Build the DID Document
    let key_id = format!("{did}#{method_specific}");

    let vm = VerificationMethod {
        id: key_id.clone(),
        controller: did.to_string(),
        key_type: "Ed25519VerificationKey2020".to_string(),
        public_key_multibase: Some(method_specific.to_string()),
        public_key_jwk: None,
    };

    Ok(DidDocument {
        id: did.to_string(),
        verification_method: vec![vm],
        authentication: vec![VerificationRelationship::Reference(key_id.clone())],
        assertion_method: vec![VerificationRelationship::Reference(key_id)],
    })
}

/// Extract the raw 32-byte Ed25519 public key from a `did:key` DID string.
pub fn extract_public_key(did: &str) -> Result<[u8; 32], IdentityError> {
    let method_specific = did
        .strip_prefix("did:key:")
        .ok_or_else(|| IdentityError::MalformedDid(format!("`{did}` is not a did:key DID")))?;

    if !method_specific.starts_with('z') {
        return Err(IdentityError::MalformedDid(
            "missing base58btc prefix 'z'".into(),
        ));
    }

    let decoded = bs58::decode(&method_specific[1..])
        .into_vec()
        .map_err(|e| IdentityError::MalformedDid(format!("base58btc decode failed: {e}")))?;

    if decoded.len() < 2 || decoded[0..2] != ED25519_MULTICODEC {
        return Err(IdentityError::UnsupportedKeyType(
            "not an Ed25519 multicodec key".into(),
        ));
    }

    let key_bytes = &decoded[2..];
    key_bytes.try_into().map_err(|_| {
        IdentityError::MalformedPublicKey(format!("expected 32 bytes, got {}", key_bytes.len()))
    })
}

/// Generate a `did:key` DID string from a 32-byte Ed25519 public key.
/// Useful for test key generation.
pub fn did_key_from_public(pub_key: &[u8; 32]) -> String {
    let mut prefixed = Vec::with_capacity(34);
    prefixed.extend_from_slice(&ED25519_MULTICODEC);
    prefixed.extend_from_slice(pub_key);
    let multibase = format!("z{}", bs58::encode(&prefixed).into_string());
    format!("did:key:{multibase}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::ed25519::generate_test_keypair;

    #[test]
    fn roundtrip_generate_and_resolve() {
        let (_sk, pk) = generate_test_keypair();
        let did = did_key_from_public(&pk);
        assert!(did.starts_with("did:key:z6Mk"));

        let doc = resolve_did_key(&did).unwrap();
        assert_eq!(doc.id, did);
        assert_eq!(doc.verification_method.len(), 1);
        assert_eq!(doc.authentication.len(), 1);
        assert_eq!(doc.assertion_method.len(), 1);

        let extracted = extract_public_key(&did).unwrap();
        assert_eq!(extracted, pk);
    }

    #[test]
    fn rejects_non_did_key() {
        assert!(matches!(
            resolve_did_key("did:web:example.com"),
            Err(IdentityError::MalformedDid(_))
        ));
    }

    #[test]
    fn rejects_malformed_did() {
        assert!(matches!(
            resolve_did_key("not-a-did"),
            Err(IdentityError::MalformedDid(_))
        ));
        assert!(matches!(
            resolve_did_key("did:key:"),
            Err(IdentityError::MalformedDid(_))
        ));
    }

    #[test]
    fn rejects_non_base58btc_prefix() {
        // 'M' is base64 multibase prefix
        assert!(matches!(
            resolve_did_key("did:key:Mabc123"),
            Err(IdentityError::MalformedDid(_))
        ));
    }

    #[test]
    fn rejects_unsupported_multicodec() {
        // Build a key with the P-256 multicodec prefix (0x80 0x24) instead of Ed25519
        let mut prefixed = vec![0x80, 0x24];
        prefixed.extend_from_slice(&[0u8; 33]); // P-256 is 33 bytes
        let multibase = format!("z{}", bs58::encode(&prefixed).into_string());
        let did = format!("did:key:{multibase}");
        assert!(matches!(
            resolve_did_key(&did),
            Err(IdentityError::UnsupportedKeyType(_))
        ));
    }

    #[test]
    fn rejects_wrong_key_length() {
        // Ed25519 prefix but only 16 bytes of key material
        let mut prefixed = vec![0xed, 0x01];
        prefixed.extend_from_slice(&[0u8; 16]);
        let multibase = format!("z{}", bs58::encode(&prefixed).into_string());
        let did = format!("did:key:{multibase}");
        assert!(matches!(
            resolve_did_key(&did),
            Err(IdentityError::MalformedPublicKey(_))
        ));
    }

    #[test]
    fn extract_key_matches_generated() {
        let (_sk, pk) = generate_test_keypair();
        let did = did_key_from_public(&pk);
        let extracted = extract_public_key(&did).unwrap();
        assert_eq!(extracted, pk, "extracted key must match generated key");
    }
}
