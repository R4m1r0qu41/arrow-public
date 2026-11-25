use std::net::{IpAddr, SocketAddr};

use arrow_core::{registry::loader::Registries, types::registry::ChannelBindingEntry};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NodeConfig {
    pub listen: String,
    pub node_id: String,
    /// "embedded" or a filesystem directory path to registries.
    pub registries: String,
    pub known_senders: KnownSenders,
    /// Transport channel binding and advertised URL.
    #[serde(default)]
    pub channel: ChannelConfig,
    /// TLS deployment boundary. Native TLS is not implemented in the v0.2 MVP.
    #[serde(default)]
    pub tls: TlsConfig,
    /// Replay protection backend. Defaults to in-memory replay protection.
    #[serde(default)]
    pub replay: ReplayConfig,
    /// Identity / signature policy. Required by validated node startup config.
    #[serde(default)]
    pub identity: Option<IdentityConfig>,
}

#[derive(Debug, Deserialize)]
pub struct KnownSenders {
    pub allow: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelConfig {
    #[serde(default = "default_channel_binding")]
    pub binding: String,
    #[serde(default)]
    pub public_base_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TlsConfig {
    /// "local-disabled" or "terminated-upstream".
    #[serde(default = "default_tls_mode")]
    pub mode: String,
    /// When true, HTTPS deployments require forwarded proto metadata from a trusted proxy.
    #[serde(default)]
    pub require_forwarded_proto_https: bool,
    /// Trusted reverse proxy IP address for terminated-upstream mode.
    #[serde(default)]
    pub trusted_proxy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsMode {
    LocalDisabled,
    TerminatedUpstream,
}

#[derive(Debug, Clone)]
pub struct TlsRuntimeConfig {
    pub mode: TlsMode,
    pub require_forwarded_proto_https: bool,
    pub trusted_proxy: Option<IpAddr>,
}

#[derive(Debug, Clone)]
pub struct ValidatedNodeTransport {
    pub binding: ChannelBindingEntry,
    pub tls: TlsRuntimeConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReplayConfig {
    /// "memory" or "sqlite".
    #[serde(default = "default_replay_backend")]
    pub backend: String,
    /// SQLite database path. Ignored for the in-memory backend.
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdentityConfig {
    /// If true, envelopes without a `signing` block are rejected with TPT-SIGNATURE-MISSING.
    #[serde(default)]
    pub require_signature: bool,
    /// Accepted DID methods (e.g. ["key", "did:key"]). Empty = accept all.
    #[serde(default)]
    pub allowed_did_methods: Vec<String>,
    /// Accepted algorithms. Default: ["Ed25519"].
    #[serde(default = "default_algorithms")]
    pub allowed_algorithms: Vec<String>,
}

fn default_algorithms() -> Vec<String> {
    vec!["Ed25519".to_string()]
}

fn default_channel_binding() -> String {
    "http-rest-local".to_string()
}

fn default_replay_backend() -> String {
    "memory".to_string()
}

fn default_tls_mode() -> String {
    "local-disabled".to_string()
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            backend: default_replay_backend(),
            path: None,
        }
    }
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            binding: default_channel_binding(),
            public_base_url: None,
        }
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            mode: default_tls_mode(),
            require_forwarded_proto_https: false,
            trusted_proxy: None,
        }
    }
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            require_signature: false,
            allowed_did_methods: Vec::new(),
            allowed_algorithms: default_algorithms(),
        }
    }
}

impl TlsRuntimeConfig {
    pub fn local_disabled() -> Self {
        Self {
            mode: TlsMode::LocalDisabled,
            require_forwarded_proto_https: false,
            trusted_proxy: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    message: String,
}

impl ConfigError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

pub fn validate_node_transport(
    config: &NodeConfig,
    registries: &Registries,
) -> Result<ValidatedNodeTransport, ConfigError> {
    let binding = registries
        .channel_bindings
        .get(config.channel.binding.as_str())
        .ok_or_else(|| {
            ConfigError::new(format!(
                "channel.binding `{}` is not registered in channel_bindings",
                config.channel.binding
            ))
        })?
        .clone();

    if binding.status != "active" {
        return Err(ConfigError::new(format!(
            "channel.binding `{}` is not active (status: {})",
            binding.code, binding.status
        )));
    }

    let tls = parse_tls_runtime(&config.tls)?;
    validate_identity(config.identity.as_ref())?;
    validate_binding_profile(config, &binding, &tls)?;

    Ok(ValidatedNodeTransport { binding, tls })
}

fn parse_tls_runtime(config: &TlsConfig) -> Result<TlsRuntimeConfig, ConfigError> {
    let mode = match config.mode.as_str() {
        "local-disabled" => TlsMode::LocalDisabled,
        "terminated-upstream" => TlsMode::TerminatedUpstream,
        other => {
            return Err(ConfigError::new(format!(
                "tls.mode `{other}` is not supported; expected `local-disabled` or `terminated-upstream`"
            )))
        }
    };

    let trusted_proxy = match config.trusted_proxy.as_deref() {
        Some(proxy) => Some(proxy.parse::<IpAddr>().map_err(|e| {
            ConfigError::new(format!(
                "tls.trusted_proxy `{proxy}` is not a valid IP address: {e}"
            ))
        })?),
        None => None,
    };

    Ok(TlsRuntimeConfig {
        mode,
        require_forwarded_proto_https: config.require_forwarded_proto_https,
        trusted_proxy,
    })
}

fn validate_identity(identity: Option<&IdentityConfig>) -> Result<(), ConfigError> {
    let identity = identity.ok_or_else(|| {
        ConfigError::new("identity.require_signature=true is required for arrow-node v0.2")
    })?;

    if !identity.require_signature {
        return Err(ConfigError::new(
            "identity.require_signature must be true for arrow-node v0.2",
        ));
    }

    if !identity
        .allowed_algorithms
        .iter()
        .any(|algorithm| algorithm == "Ed25519")
    {
        return Err(ConfigError::new(
            "identity.allowed_algorithms must include Ed25519",
        ));
    }

    Ok(())
}

fn validate_binding_profile(
    config: &NodeConfig,
    binding: &ChannelBindingEntry,
    tls: &TlsRuntimeConfig,
) -> Result<(), ConfigError> {
    match binding.code.as_str() {
        "http-rest-local" => validate_http_rest_local(config, tls),
        "https-rest" => validate_https_rest(config, tls),
        other => Err(ConfigError::new(format!(
            "channel.binding `{other}` is registered but not supported by arrow-node v0.2"
        ))),
    }
}

fn validate_http_rest_local(
    config: &NodeConfig,
    tls: &TlsRuntimeConfig,
) -> Result<(), ConfigError> {
    if tls.mode != TlsMode::LocalDisabled {
        return Err(ConfigError::new(
            "http-rest-local requires tls.mode=`local-disabled`",
        ));
    }
    if tls.require_forwarded_proto_https || tls.trusted_proxy.is_some() {
        return Err(ConfigError::new(
            "http-rest-local must not configure forwarded HTTPS proxy settings",
        ));
    }
    if !listen_is_loopback(&config.listen)? {
        return Err(ConfigError::new(
            "http-rest-local may only listen on a loopback address",
        ));
    }
    if let Some(url) = config.channel.public_base_url.as_deref() {
        if !is_local_http_url(url) {
            return Err(ConfigError::new(
                "http-rest-local public_base_url must be an http:// localhost, 127.0.0.1, or [::1] URL",
            ));
        }
    }
    Ok(())
}

fn validate_https_rest(config: &NodeConfig, tls: &TlsRuntimeConfig) -> Result<(), ConfigError> {
    if tls.mode != TlsMode::TerminatedUpstream {
        return Err(ConfigError::new(
            "https-rest requires tls.mode=`terminated-upstream`",
        ));
    }
    if !tls.require_forwarded_proto_https {
        return Err(ConfigError::new(
            "https-rest requires tls.require_forwarded_proto_https=true",
        ));
    }
    if tls.trusted_proxy.is_none() {
        return Err(ConfigError::new(
            "https-rest requires tls.trusted_proxy to identify the reverse proxy boundary",
        ));
    }
    if !listen_is_loopback_or_private(&config.listen)? {
        return Err(ConfigError::new(
            "https-rest arrow-node listener must be loopback or private behind the TLS terminator",
        ));
    }
    match config.channel.public_base_url.as_deref() {
        Some(url) if url.starts_with("https://") => Ok(()),
        _ => Err(ConfigError::new(
            "https-rest requires channel.public_base_url to start with https://",
        )),
    }
}

fn listen_addr(listen: &str) -> Result<SocketAddr, ConfigError> {
    listen.parse::<SocketAddr>().map_err(|e| {
        ConfigError::new(format!(
            "listen `{listen}` is not a valid socket address: {e}"
        ))
    })
}

fn listen_is_loopback(listen: &str) -> Result<bool, ConfigError> {
    Ok(listen_addr(listen)?.ip().is_loopback())
}

fn listen_is_loopback_or_private(listen: &str) -> Result<bool, ConfigError> {
    let ip = listen_addr(listen)?.ip();
    Ok(match ip {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private(),
        IpAddr::V6(ip) => ip.is_loopback() || ((ip.segments()[0] & 0xfe00) == 0xfc00),
    })
}

fn is_local_http_url(url: &str) -> bool {
    url.starts_with("http://127.0.0.1")
        || url.starts_with("http://localhost")
        || url.starts_with("http://[::1]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_core::registry::loader::Registries;

    fn parse_config(extra: &str) -> NodeConfig {
        toml::from_str(&format!(
            r#"
listen = "127.0.0.1:0"
node_id = "did:arrow:pilot:node-a"
registries = "embedded"

[known_senders]
allow = []

[identity]
require_signature = true
allowed_did_methods = ["did:key"]
allowed_algorithms = ["Ed25519"]

{extra}
"#
        ))
        .unwrap()
    }

    #[test]
    fn replay_defaults_to_memory_when_section_is_absent() {
        let config: NodeConfig = toml::from_str(
            r#"
listen = "127.0.0.1:0"
node_id = "did:arrow:pilot:node-a"
registries = "embedded"

[known_senders]
allow = []
"#,
        )
        .unwrap();

        assert_eq!(config.replay.backend, "memory");
        assert_eq!(config.replay.path, None);
        assert_eq!(config.channel.binding, "http-rest-local");
        assert_eq!(config.tls.mode, "local-disabled");
    }

    #[test]
    fn replay_sqlite_section_parses_path() {
        let config: NodeConfig = toml::from_str(
            r#"
listen = "127.0.0.1:0"
node_id = "did:arrow:pilot:node-a"
registries = "embedded"

[replay]
backend = "sqlite"
path = "/tmp/arrow-replay.db"

[known_senders]
allow = []
"#,
        )
        .unwrap();

        assert_eq!(config.replay.backend, "sqlite");
        assert_eq!(config.replay.path.as_deref(), Some("/tmp/arrow-replay.db"));
    }

    #[test]
    fn local_http_profile_accepts_loopback_config() {
        let config = parse_config(
            r#"
[channel]
binding = "http-rest-local"
public_base_url = "http://127.0.0.1:3000"
"#,
        );

        let transport = validate_node_transport(&config, &Registries::load_embedded().unwrap())
            .expect("local profile should validate");

        assert_eq!(transport.binding.code, "http-rest-local");
        assert_eq!(transport.tls.mode, TlsMode::LocalDisabled);
    }

    #[test]
    fn local_http_profile_rejects_public_listener() {
        let mut config = parse_config(
            r#"
[channel]
binding = "http-rest-local"
public_base_url = "http://127.0.0.1:3000"
"#,
        );
        config.listen = "0.0.0.0:3000".to_string();

        let err = validate_node_transport(&config, &Registries::load_embedded().unwrap())
            .expect_err("public local HTTP listener must fail");

        assert!(err.to_string().contains("loopback"));
    }

    #[test]
    fn https_profile_accepts_terminated_proxy_config() {
        let config = parse_config(
            r#"
[channel]
binding = "https-rest"
public_base_url = "https://node.example.test"

[tls]
mode = "terminated-upstream"
require_forwarded_proto_https = true
trusted_proxy = "127.0.0.1"
"#,
        );

        let transport = validate_node_transport(&config, &Registries::load_embedded().unwrap())
            .expect("https profile should validate");

        assert_eq!(transport.binding.code, "https-rest");
        assert_eq!(transport.tls.mode, TlsMode::TerminatedUpstream);
    }

    #[test]
    fn https_profile_rejects_missing_proxy_boundary() {
        let config = parse_config(
            r#"
[channel]
binding = "https-rest"
public_base_url = "https://node.example.test"

[tls]
mode = "terminated-upstream"
require_forwarded_proto_https = true
"#,
        );

        let err = validate_node_transport(&config, &Registries::load_embedded().unwrap())
            .expect_err("missing trusted proxy must fail");

        assert!(err.to_string().contains("trusted_proxy"));
    }

    #[test]
    fn https_profile_rejects_http_public_url() {
        let config = parse_config(
            r#"
[channel]
binding = "https-rest"
public_base_url = "http://node.example.test"

[tls]
mode = "terminated-upstream"
require_forwarded_proto_https = true
trusted_proxy = "127.0.0.1"
"#,
        );

        let err = validate_node_transport(&config, &Registries::load_embedded().unwrap())
            .expect_err("http public URL must fail for https-rest");

        assert!(err.to_string().contains("https://"));
    }

    #[test]
    fn unregistered_binding_is_rejected() {
        let config = parse_config(
            r#"
[channel]
binding = "not-registered"
"#,
        );

        let err = validate_node_transport(&config, &Registries::load_embedded().unwrap())
            .expect_err("unregistered binding must fail");

        assert!(err.to_string().contains("not registered"));
    }

    #[test]
    fn inactive_binding_is_rejected() {
        let config = parse_config(
            r#"
[channel]
binding = "http-rest"
"#,
        );

        let err = validate_node_transport(&config, &Registries::load_embedded().unwrap())
            .expect_err("inactive legacy binding must fail");

        assert!(err.to_string().contains("not active"));
    }

    #[test]
    fn startup_config_requires_ed25519_signatures() {
        let config: NodeConfig = toml::from_str(
            r#"
listen = "127.0.0.1:0"
node_id = "did:arrow:pilot:node-a"
registries = "embedded"

[known_senders]
allow = []

[channel]
binding = "http-rest-local"
"#,
        )
        .unwrap();

        let err = validate_node_transport(&config, &Registries::load_embedded().unwrap())
            .expect_err("missing identity must fail");

        assert!(err.to_string().contains("require_signature"));
    }
}
