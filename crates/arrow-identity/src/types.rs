use serde::{Deserialize, Serialize};

// ── Signing metadata (carried on the Arrow envelope) ──────────────────────────

/// Arrow-specific signing metadata block, embedded in transport envelopes.
///
/// ```json
/// {
///   "signing": {
///     "alg": "Ed25519",
///     "key_ref": "did:key:z6Mk...#z6Mk...",
///     "sig_b64": "...",
///     "created": "2026-05-23T00:00:00Z",
///     "canonicalization": "arrow-canonical-json-v0",
///     "signed_fields": "envelope-minus-signature-and-signing"
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningMetadata {
    /// Algorithm identifier (e.g. "Ed25519").
    pub alg: String,
    /// DID URL referencing the verification method, e.g. "did:key:z6Mk...#z6Mk...".
    pub key_ref: String,
    /// Base64-encoded signature bytes (standard, no padding).
    pub sig_b64: String,
    /// ISO 8601 UTC timestamp of when the signature was created.
    #[serde(default)]
    pub created: Option<String>,
    /// Canonicalization scheme used. Default: "arrow-canonical-json-v0".
    #[serde(default = "default_canonicalization")]
    pub canonicalization: String,
    /// What fields are signed. Default: "envelope-minus-signature-and-signing".
    #[serde(default = "default_signed_fields")]
    pub signed_fields: String,
}

fn default_canonicalization() -> String {
    "arrow-canonical-json-v0".to_string()
}
fn default_signed_fields() -> String {
    "envelope-minus-signature-and-signing".to_string()
}

// ── Supported algorithms ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    Ed25519,
}

impl SignatureAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ed25519 => "Ed25519",
        }
    }
}

impl std::str::FromStr for SignatureAlgorithm {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Ed25519" | "ed25519" | "ED25519" => Ok(Self::Ed25519),
            _ => Err(format!("unknown algorithm: {s}")),
        }
    }
}

// ── Minimal DID Document model ────────────────────────────────────────────────

/// Minimal DID Document — only the fields needed for Arrow signature verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    pub id: String,
    #[serde(default, rename = "verificationMethod")]
    pub verification_method: Vec<VerificationMethod>,
    #[serde(default)]
    pub authentication: Vec<VerificationRelationship>,
    #[serde(default, rename = "assertionMethod")]
    pub assertion_method: Vec<VerificationRelationship>,
}

/// A verification method entry in a DID Document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    pub controller: String,
    #[serde(rename = "type")]
    pub key_type: String,
    /// Multibase-encoded public key (e.g. "z6Mk..." for Ed25519).
    #[serde(default, rename = "publicKeyMultibase")]
    pub public_key_multibase: Option<String>,
    /// JWK representation (not used in v0.1 but parsed if present).
    #[serde(default, rename = "publicKeyJwk")]
    pub public_key_jwk: Option<serde_json::Value>,
}

/// A verification relationship — either an embedded method or a reference string.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VerificationRelationship {
    /// Reference to a verification method by its `id`.
    Reference(String),
    /// Embedded verification method.
    Embedded(VerificationMethod),
}

impl VerificationRelationship {
    /// Get the ID this relationship points to.
    pub fn id(&self) -> &str {
        match self {
            Self::Reference(s) => s.as_str(),
            Self::Embedded(vm) => vm.id.as_str(),
        }
    }
}

// ── Resolved key ──────────────────────────────────────────────────────────────

/// A resolved public key ready for signature verification.
#[derive(Debug, Clone)]
pub struct ResolvedKey {
    pub did: String,
    pub key_ref: String,
    pub algorithm: SignatureAlgorithm,
    pub public_key_bytes: Vec<u8>,
}

// ── Verification input (generic over bytes) ───────────────────────────────────

/// Input to the generic verify function. The caller (transport layer) constructs
/// this from envelope-specific types; `arrow-identity` never imports envelope types.
pub struct VerificationInput<'a> {
    /// Canonical payload bytes (envelope minus signature and signing fields, JCS-serialized).
    pub payload: &'a [u8],
    /// DID URL referencing the verification method.
    pub key_ref: &'a str,
    /// Declared algorithm.
    pub algorithm: SignatureAlgorithm,
    /// Raw signature bytes (decoded from base64 by the caller or this crate).
    pub signature_bytes: &'a [u8],
}

/// Outcome of signature verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationOutcome {
    /// Signature is valid; the signer actions the key at `key_ref`.
    Valid { did: String, key_ref: String },
    /// Signature verification failed.
    Invalid(String),
}

impl VerificationOutcome {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid { .. })
    }
}
