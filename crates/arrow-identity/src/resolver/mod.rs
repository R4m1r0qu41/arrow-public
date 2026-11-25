pub mod did_key;

#[cfg(feature = "did-web")]
pub mod did_web;

use crate::error::IdentityError;
use crate::types::DidDocument;

/// Trait for resolving a DID to a DID Document.
/// Implementations are method-specific (did:key, did:web, etc.).
pub trait DidResolver: Send + Sync {
    /// Resolve a DID string to a minimal DID Document.
    fn resolve(&self, did: &str) -> Result<DidDocument, IdentityError>;
}

/// A composite resolver that dispatches to method-specific resolvers based on the DID prefix.
pub struct MultiResolver {
    resolvers: Vec<(String, Box<dyn DidResolver>)>,
}

impl MultiResolver {
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    /// Register a resolver for a specific DID method (e.g. "key", "web").
    pub fn register(mut self, method: &str, resolver: Box<dyn DidResolver>) -> Self {
        self.resolvers.push((method.to_string(), resolver));
        self
    }

    /// Build a default resolver with did:key enabled (and did:web if the feature is active).
    pub fn default_config() -> Self {
        let r = Self::new().register("key", Box::new(did_key::DidKeyResolver));

        #[cfg(feature = "did-web")]
        let r = r.register("web", Box::new(did_web::DidWebResolver::default()));

        r
    }
}

impl DidResolver for MultiResolver {
    fn resolve(&self, did: &str) -> Result<DidDocument, IdentityError> {
        let method = extract_method(did)?;

        for (m, resolver) in &self.resolvers {
            if m == &method {
                return resolver.resolve(did);
            }
        }

        Err(IdentityError::UnsupportedDidMethod(format!(
            "no resolver registered for method `{method}`"
        )))
    }
}

impl Default for MultiResolver {
    fn default() -> Self {
        Self::default_config()
    }
}

/// Extract the method portion from a DID string (e.g. "key" from "did:key:z6Mk...").
pub fn extract_method(did: &str) -> Result<String, IdentityError> {
    if !did.starts_with("did:") {
        return Err(IdentityError::MalformedDid(format!(
            "DID must start with 'did:', got `{did}`"
        )));
    }
    let rest = &did[4..];
    let method = rest.split(':').next().unwrap_or("");
    if method.is_empty() {
        return Err(IdentityError::MalformedDid(format!(
            "DID method is empty in `{did}`"
        )));
    }
    Ok(method.to_string())
}

/// Extract the DID portion (without fragment) from a DID URL.
/// "did:key:z6Mk...#z6Mk..." → "did:key:z6Mk..."
pub fn did_from_url(did_url: &str) -> &str {
    did_url.split('#').next().unwrap_or(did_url)
}
