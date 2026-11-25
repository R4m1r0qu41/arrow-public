use std::collections::HashSet;
use std::sync::Arc;

use crate::config::{IdentityConfig, TlsRuntimeConfig};
use arrow_core::{registry::loader::Registries, types::registry::ChannelBindingEntry};
use arrow_transport::replay::ReplayStore;

pub struct AppState {
    pub node_id: String,
    pub registries: Arc<Registries>,
    /// Replay protection store. May be in-memory or SQLite-backed.
    pub replay: Arc<dyn ReplayStore>,
    pub known_senders: HashSet<String>,
    pub binding: ChannelBindingEntry,
    pub identity: IdentityConfig,
    pub tls: TlsRuntimeConfig,
}
