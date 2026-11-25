use std::str::FromStr;

use arrow_identity::{
    resolver::{did_from_url, extract_method, MultiResolver},
    types::{SignatureAlgorithm, VerificationInput},
    verify::verify_signature,
};
use base64::Engine;

use crate::{
    canonical::envelope_canonical_bytes, envelope::TransportEnvelope, error::TransportError,
};

pub const CANONICALIZATION: &str = "arrow-canonical-json-v0";
pub const SIGNED_FIELDS: &str = "envelope-minus-signature-and-signing";

#[derive(Debug, Clone)]
pub struct SignatureVerificationPolicy {
    pub require_signature: bool,
    pub allowed_did_methods: Vec<String>,
    pub allowed_algorithms: Vec<String>,
}

impl Default for SignatureVerificationPolicy {
    fn default() -> Self {
        Self {
            require_signature: false,
            allowed_did_methods: Vec::new(),
            allowed_algorithms: vec!["Ed25519".to_string()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedEnvelopeSignature {
    pub signer_did: String,
    pub key_ref: String,
}

pub fn verify_envelope_signature(
    envelope: &TransportEnvelope,
    policy: &SignatureVerificationPolicy,
) -> Result<Option<VerifiedEnvelopeSignature>, TransportError> {
    let signing = match &envelope.signing {
        Some(signing) => signing,
        None if policy.require_signature => {
            return Err(TransportError::SignatureMissing(
                "this endpoint requires a signing block".to_string(),
            ));
        }
        None => return Ok(None),
    };

    if signing.canonicalization != CANONICALIZATION {
        return Err(TransportError::SignatureInvalid(format!(
            "unsupported canonicalization `{}`",
            signing.canonicalization
        )));
    }

    if signing.signed_fields != SIGNED_FIELDS {
        return Err(TransportError::SignatureInvalid(format!(
            "unsupported signed_fields `{}`",
            signing.signed_fields
        )));
    }

    let signer_did = did_from_url(&signing.key_ref);
    if signer_did != envelope.sender_id {
        return Err(TransportError::SignatureInvalid(
            "signing key DID does not match envelope sender_id".to_string(),
        ));
    }

    let signer_method = extract_method(signer_did).map_err(|e| {
        TransportError::SignatureInvalid(format!("signing key DID method invalid: {e}"))
    })?;

    if !policy.allowed_did_methods.is_empty() {
        let allowed_methods = policy
            .allowed_did_methods
            .iter()
            .map(|method| configured_did_method(method))
            .collect::<Result<Vec<_>, _>>()?;

        if !allowed_methods
            .iter()
            .any(|method| method == &signer_method)
        {
            return Err(TransportError::SignatureInvalid(format!(
                "DID method `{signer_method}` is not in allowed_did_methods"
            )));
        }
    }

    let alg = SignatureAlgorithm::from_str(&signing.alg).map_err(|_| {
        TransportError::SignatureInvalid(format!("unsupported algorithm `{}`", signing.alg))
    })?;

    if !policy.allowed_algorithms.is_empty()
        && !policy
            .allowed_algorithms
            .iter()
            .any(|a| a.eq_ignore_ascii_case(alg.as_str()))
    {
        return Err(TransportError::SignatureInvalid(format!(
            "algorithm `{}` is not in allowed_algorithms",
            alg.as_str()
        )));
    }

    let sig_bytes = base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(&signing.sig_b64)
        .map_err(|e| TransportError::SignatureInvalid(format!("sig_b64 decode failed: {e}")))?;

    let canonical = envelope_canonical_bytes(envelope)
        .map_err(|e| TransportError::SignatureInvalid(format!("canonicalization failed: {e}")))?;

    let resolver = MultiResolver::default_config();
    let input = VerificationInput {
        payload: &canonical,
        key_ref: &signing.key_ref,
        algorithm: alg,
        signature_bytes: &sig_bytes,
    };

    match verify_signature(&resolver, input) {
        Ok(outcome) if outcome.is_valid() => Ok(Some(VerifiedEnvelopeSignature {
            signer_did: signer_did.to_string(),
            key_ref: signing.key_ref.clone(),
        })),
        Ok(_) => Err(TransportError::SignatureInvalid(
            "signature verification returned invalid".to_string(),
        )),
        Err(e) => Err(TransportError::SignatureInvalid(format!(
            "identity error: {e}"
        ))),
    }
}

pub fn configured_did_method(method: &str) -> Result<String, TransportError> {
    let method = method.trim();
    if method.is_empty() {
        return Err(TransportError::SignatureInvalid(
            "allowed_did_methods contains an empty entry".to_string(),
        ));
    }

    let method = if let Some(rest) = method.strip_prefix("did:") {
        if rest.is_empty() || rest.contains(':') {
            return Err(TransportError::SignatureInvalid(format!(
                "allowed_did_methods entry `{method}` must be a method name or did:<method>"
            )));
        }
        rest
    } else {
        if method.contains(':') {
            return Err(TransportError::SignatureInvalid(format!(
                "allowed_did_methods entry `{method}` must be a method name or did:<method>"
            )));
        }
        method
    };

    if !method
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err(TransportError::SignatureInvalid(format!(
            "allowed_did_methods entry `{method}` is not a valid DID method"
        )));
    }

    Ok(method.to_string())
}
