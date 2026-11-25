use crate::{
    crypto::ed25519::verify_ed25519,
    error::IdentityError,
    resolver::{did_from_url, DidResolver},
    types::{SignatureAlgorithm, VerificationInput, VerificationOutcome},
};

/// Verify a signature over arbitrary payload bytes using a DID-resolved public key.
///
/// This function is generic over the resolver — the caller provides the appropriate
/// `DidResolver` implementation. `arrow-identity` never imports transport-specific types.
///
/// Steps:
/// 1. Extract the DID from the key_ref DID URL.
/// 2. Resolve the DID to a DID Document.
/// 3. Find the verification method matching key_ref.
/// 4. Confirm the method is listed under `authentication` or `assertionMethod`.
/// 5. Decode the public key from the verification method.
/// 6. Verify the cryptographic signature.
pub fn verify_signature<R: DidResolver>(
    resolver: &R,
    input: VerificationInput<'_>,
) -> Result<VerificationOutcome, IdentityError> {
    // 1. Extract DID from key_ref URL
    let did = did_from_url(input.key_ref);

    // 2. Resolve DID → DID Document
    let doc = resolver.resolve(did)?;

    // 3. Find verification method matching key_ref
    let vm = doc
        .verification_method
        .iter()
        .find(|vm| vm.id == input.key_ref)
        .ok_or_else(|| {
            IdentityError::VerificationMethodNotFound(format!(
                "no verification method with id `{}` in DID Document for `{}`",
                input.key_ref, did
            ))
        })?;

    // 4. Confirm the method is under an allowed verification relationship
    let is_auth = doc.authentication.iter().any(|r| r.id() == input.key_ref);
    let is_assert = doc.assertion_method.iter().any(|r| r.id() == input.key_ref);
    if !is_auth && !is_assert {
        return Err(IdentityError::VerificationRelationshipNotAllowed(format!(
            "key `{}` is not listed under authentication or assertionMethod",
            input.key_ref
        )));
    }

    // 5. Decode public key from the verification method
    let pub_key_bytes = decode_public_key(vm)?;

    // 6. Verify the signature
    match input.algorithm {
        SignatureAlgorithm::Ed25519 => {
            verify_ed25519(&pub_key_bytes, input.payload, input.signature_bytes)?;
        }
    }

    Ok(VerificationOutcome::Valid {
        did: did.to_string(),
        key_ref: input.key_ref.to_string(),
    })
}

/// Decode the public key bytes from a `VerificationMethod`.
/// Supports `publicKeyMultibase` (base58btc with multicodec prefix).
fn decode_public_key(vm: &crate::types::VerificationMethod) -> Result<Vec<u8>, IdentityError> {
    if let Some(multibase) = &vm.public_key_multibase {
        // Must start with 'z' (base58btc multibase prefix)
        let b58_part = multibase.strip_prefix('z').ok_or_else(|| {
            IdentityError::MalformedPublicKey(format!(
                "publicKeyMultibase must start with 'z', got `{}`",
                &multibase[..1.min(multibase.len())]
            ))
        })?;

        let decoded = bs58::decode(b58_part).into_vec().map_err(|e| {
            IdentityError::MalformedPublicKey(format!("base58btc decode failed: {e}"))
        })?;

        // Strip 2-byte multicodec prefix (0xed 0x01 for Ed25519)
        if decoded.len() < 2 {
            return Err(IdentityError::MalformedPublicKey(
                "decoded key too short for multicodec prefix".into(),
            ));
        }

        // For Ed25519VerificationKey2020, expect Ed25519 multicodec
        if vm.key_type == "Ed25519VerificationKey2020" && decoded[0..2] != [0xed, 0x01] {
            return Err(IdentityError::UnsupportedKeyType(format!(
                "key_type is Ed25519VerificationKey2020 but multicodec is [{:#04x}, {:#04x}]",
                decoded[0], decoded[1]
            )));
        }

        Ok(decoded[2..].to_vec())
    } else {
        Err(IdentityError::MalformedPublicKey(
            "verification method has no publicKeyMultibase".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        crypto::ed25519::{generate_test_keypair, sign_ed25519},
        resolver::did_key::{did_key_from_public, DidKeyResolver},
    };

    fn make_input<'a>(payload: &'a [u8], key_ref: &'a str, sig: &'a [u8]) -> VerificationInput<'a> {
        VerificationInput {
            payload,
            key_ref,
            algorithm: SignatureAlgorithm::Ed25519,
            signature_bytes: sig,
        }
    }

    #[test]
    fn valid_signature_accepted() {
        let (sk, pk) = generate_test_keypair();
        let did = did_key_from_public(&pk);
        let key_ref = format!(
            "{did}#z{}",
            bs58::encode({
                let mut v = vec![0xed, 0x01];
                v.extend_from_slice(&pk);
                v
            })
            .into_string()
        );

        let payload = b"canonical envelope bytes";
        let sig = sign_ed25519(&sk, payload);

        let resolver = DidKeyResolver;
        let input = make_input(payload, &key_ref, &sig);
        let result = verify_signature(&resolver, input).unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn tampered_payload_rejected() {
        let (sk, pk) = generate_test_keypair();
        let did = did_key_from_public(&pk);
        let key_ref = format!(
            "{did}#z{}",
            bs58::encode({
                let mut v = vec![0xed, 0x01];
                v.extend_from_slice(&pk);
                v
            })
            .into_string()
        );

        let sig = sign_ed25519(&sk, b"original");

        let resolver = DidKeyResolver;
        let input = make_input(b"tampered", &key_ref, &sig);
        let result = verify_signature(&resolver, input);
        assert!(matches!(result, Err(IdentityError::SignatureMismatch)));
    }

    #[test]
    fn wrong_key_ref_rejected() {
        let (sk, pk) = generate_test_keypair();
        let _did = did_key_from_public(&pk);

        // Use a valid key_ref but for a different key
        let (_, other_pk) = generate_test_keypair();
        let other_did = did_key_from_public(&other_pk);
        let wrong_key_ref = format!(
            "{other_did}#z{}",
            bs58::encode({
                let mut v = vec![0xed, 0x01];
                v.extend_from_slice(&other_pk);
                v
            })
            .into_string()
        );

        let payload = b"test";
        let sig = sign_ed25519(&sk, payload);

        let resolver = DidKeyResolver;
        let input = make_input(payload, &wrong_key_ref, &sig);
        let result = verify_signature(&resolver, input);
        // Signature was made with sk but verified against other_pk → mismatch
        assert!(matches!(result, Err(IdentityError::SignatureMismatch)));
    }

    #[test]
    fn unsupported_method_rejected() {
        let resolver = DidKeyResolver;
        let input = make_input(b"x", "did:web:example.com#key-1", &[0u8; 64]);
        let result = verify_signature(&resolver, input);
        assert!(matches!(result, Err(IdentityError::MalformedDid(_))));
    }
}
