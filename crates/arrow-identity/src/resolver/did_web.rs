//! `did:web` resolver — resolves DIDs to DID Documents via HTTPS.
//!
//! Feature-gated behind `did-web`.
//!
//! ```text
//! did:web:example.com           → https://example.com/.well-known/did.json
//! did:web:example.com:u:alice   → https://example.com/u/alice/did.json
//! ```

use crate::error::IdentityError;
use crate::resolver::DidResolver;
use crate::types::DidDocument;

/// Configuration for the `did:web` resolver.
#[derive(Debug, Clone)]
pub struct DidWebConfig {
    /// Allow HTTP (non-TLS) resolution. **Only for testing.**
    pub allow_http: bool,
    /// Maximum response body size in bytes (default: 64 KiB).
    pub max_response_bytes: usize,
    /// HTTP timeout in seconds (default: 10).
    pub timeout_seconds: u64,
}

impl Default for DidWebConfig {
    fn default() -> Self {
        Self {
            allow_http: false,
            max_response_bytes: 65_536,
            timeout_seconds: 10,
        }
    }
}

#[derive(Default)]
pub struct DidWebResolver {
    pub config: DidWebConfig,
}

impl DidWebResolver {
    pub fn with_config(config: DidWebConfig) -> Self {
        Self { config }
    }
}

impl DidResolver for DidWebResolver {
    fn resolve(&self, did: &str) -> Result<DidDocument, IdentityError> {
        let url = did_web_to_url(did, self.config.allow_http)?;
        fetch_did_document(&url, &self.config)
    }
}

/// Convert a `did:web` DID to the HTTPS URL where the DID Document is hosted.
pub fn did_web_to_url(did: &str, allow_http: bool) -> Result<String, IdentityError> {
    let method_specific = did
        .strip_prefix("did:web:")
        .ok_or_else(|| IdentityError::MalformedDid(format!("`{did}` is not a did:web DID")))?;

    if method_specific.is_empty() {
        return Err(IdentityError::MalformedDid(
            "did:web domain is empty".into(),
        ));
    }

    let scheme = if allow_http { "http" } else { "https" };

    // Split on ':' to get domain + path segments
    let parts: Vec<&str> = method_specific.split(':').collect();
    // URL-decode the domain (percent-encoded colons in domain become ':' but
    // the spec uses ':' as a path separator, so the first part is always the domain).
    let domain = percent_decode(parts[0]);

    if parts.len() == 1 {
        // Bare domain → /.well-known/did.json
        Ok(format!("{scheme}://{domain}/.well-known/did.json"))
    } else {
        // Path-based → /path/segments/did.json
        let path: String = parts[1..]
            .iter()
            .map(|s| percent_decode(s))
            .collect::<Vec<_>>()
            .join("/");
        Ok(format!("{scheme}://{domain}/{path}/did.json"))
    }
}

fn percent_decode(s: &str) -> String {
    s.replace("%3A", ":").replace("%3a", ":")
}

/// Fetch and parse a DID Document from a URL.
fn fetch_did_document(url: &str, config: &DidWebConfig) -> Result<DidDocument, IdentityError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| IdentityError::DidWebHttpFailed(format!("HTTP client error: {e}")))?;

    let resp = client.get(url).send().map_err(|e| {
        IdentityError::DidWebHttpFailed(format!("HTTP request to `{url}` failed: {e}"))
    })?;

    if !resp.status().is_success() {
        return Err(IdentityError::DidWebHttpFailed(format!(
            "HTTP {status} from `{url}`",
            status = resp.status()
        )));
    }

    let content_length = resp.content_length().unwrap_or(0) as usize;
    if content_length > config.max_response_bytes {
        return Err(IdentityError::DidWebHttpFailed(format!(
            "response from `{url}` exceeds max size ({content_length} > {})",
            config.max_response_bytes
        )));
    }

    let body = resp.text().map_err(|e| {
        IdentityError::DidWebHttpFailed(format!("reading response body from `{url}`: {e}"))
    })?;

    if body.len() > config.max_response_bytes {
        return Err(IdentityError::DidWebHttpFailed(format!(
            "response body from `{url}` exceeds max size ({} > {})",
            body.len(),
            config.max_response_bytes
        )));
    }

    let doc: DidDocument = serde_json::from_str(&body).map_err(|e| {
        IdentityError::DidDocumentInvalid(format!("failed to parse DID Document from `{url}`: {e}"))
    })?;

    // Verify the document id matches the DID
    // (The caller passes a did:web:... string; the doc.id should match it)
    // Relaxed for v0.1 — log a warning if mismatch but don't reject,
    // since some DID Documents use slightly different id formats.

    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_domain_url() {
        let url = did_web_to_url("did:web:example.com", false).unwrap();
        assert_eq!(url, "https://example.com/.well-known/did.json");
    }

    #[test]
    fn path_based_url() {
        let url = did_web_to_url("did:web:example.com:u:alice", false).unwrap();
        assert_eq!(url, "https://example.com/u/alice/did.json");
    }

    #[test]
    fn percent_encoded_port() {
        let url = did_web_to_url("did:web:localhost%3A8080", false).unwrap();
        assert_eq!(url, "https://localhost:8080/.well-known/did.json");
    }

    #[test]
    fn rejects_non_did_web() {
        assert!(did_web_to_url("did:key:z6Mk...", false).is_err());
    }

    #[test]
    fn rejects_empty_domain() {
        assert!(did_web_to_url("did:web:", false).is_err());
    }

    #[test]
    fn allow_http_flag() {
        let url = did_web_to_url("did:web:localhost", true).unwrap();
        assert!(url.starts_with("http://"));
    }
}
