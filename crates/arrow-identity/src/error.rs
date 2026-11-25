use thiserror::Error;

/// Precise error taxonomy for identity resolution and signature verification.
/// Each variant maps to a distinct failure mode — never collapse into a generic error.
#[derive(Debug, Clone, Error)]
pub enum IdentityError {
    #[error("unsupported DID method: {0}")]
    UnsupportedDidMethod(String),

    #[error("malformed DID: {0}")]
    MalformedDid(String),

    #[error("DID resolution failed: {0}")]
    DidResolutionFailed(String),

    #[error("did:web HTTP fetch failed: {0}")]
    DidWebHttpFailed(String),

    #[error("DID Document invalid: {0}")]
    DidDocumentInvalid(String),

    #[error("verification method not found: {0}")]
    VerificationMethodNotFound(String),

    #[error("verification relationship not allowed: {0}")]
    VerificationRelationshipNotAllowed(String),

    #[error("unsupported key type: {0}")]
    UnsupportedKeyType(String),

    #[error("unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),

    #[error("malformed public key: {0}")]
    MalformedPublicKey(String),

    #[error("malformed signature: {0}")]
    MalformedSignature(String),

    #[error("signature does not match payload")]
    SignatureMismatch,

    #[error("policy denied: {0}")]
    PolicyDenied(String),

    #[error("signing metadata missing or incomplete: {0}")]
    SigningMetadataMissing(String),
}
