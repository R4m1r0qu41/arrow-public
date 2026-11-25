use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::Path;
#[cfg(any(feature = "replay-sqlite", test))]
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use chrono::Duration;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use arrow_core::registry::loader::Registries;
use arrow_node::{config::validate_node_transport, config::NodeConfig, handlers, state::AppState};
#[cfg(feature = "replay-sqlite")]
use arrow_transport::replay::SqliteReplayStore;
use arrow_transport::replay::{InMemoryReplayStore, ReplayStore};

#[derive(Parser)]
#[command(name = "arrow-node", about = "Arrow Protocol HTTP REST node")]
struct Cli {
    #[arg(long, default_value = "node.example.toml")]
    config: String,
}

fn parse_iso_to_duration(s: &str) -> Duration {
    if let Ok(d) = s.parse::<iso8601_duration::Duration>() {
        let secs = (d.hour as i64 * 3600) + (d.minute as i64 * 60) + (d.second as i64);
        if secs > 0 {
            return Duration::seconds(secs);
        }
    }
    Duration::hours(24)
}

#[cfg(any(feature = "replay-sqlite", test))]
fn resolve_sqlite_replay_path(config_path: &Path, configured_path: Option<&str>) -> PathBuf {
    let replay_path = Path::new(configured_path.unwrap_or("replay.db"));
    if replay_path.is_absolute() {
        return replay_path.to_path_buf();
    }

    let config_abs =
        std::fs::canonicalize(config_path).unwrap_or_else(|_| config_path.to_path_buf());
    let config_dir = config_abs.parent().unwrap_or_else(|| Path::new("."));
    config_dir.join(replay_path)
}

fn spawn_replay_sweep(replay: Arc<dyn ReplayStore>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5 * 60));
        loop {
            interval.tick().await;
            match replay.sweep() {
                Ok(deleted) if deleted > 0 => {
                    tracing::debug!(deleted, "swept expired replay nonces");
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::warn!(error = %err, "replay sweep failed");
                }
            }
        }
    });
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .init();

    let config_str = std::fs::read_to_string(&cli.config).unwrap_or_else(|e| {
        eprintln!("Cannot read config {}: {e}", cli.config);
        std::process::exit(2);
    });
    let config: NodeConfig = toml::from_str(&config_str).unwrap_or_else(|e| {
        eprintln!("Config parse error: {e}");
        std::process::exit(2);
    });

    let registries = if config.registries == "embedded" {
        Registries::load_embedded().unwrap_or_else(|e| {
            eprintln!("Failed to load embedded registries: {e}");
            std::process::exit(2);
        })
    } else {
        Registries::load_from_dir(Path::new(&config.registries)).unwrap_or_else(|e| {
            eprintln!("Failed: {e}");
            std::process::exit(2);
        })
    };

    let transport = validate_node_transport(&config, &registries).unwrap_or_else(|e| {
        eprintln!("Config validation error: {e}");
        std::process::exit(2);
    });
    let binding = transport.binding;
    let tls = transport.tls;
    let identity = config.identity.clone().unwrap_or_default();

    tracing::info!(
        node_id = %config.node_id,
        listen = %config.listen,
        channel_binding = %binding.code,
        "starting arrow-node"
    );
    tracing::info!("signature verification required");

    let retention = parse_iso_to_duration(&binding.nonce_retention_min);

    let replay: Arc<dyn ReplayStore> = match config.replay.backend.as_str() {
        "memory" => {
            tracing::info!("using in-memory replay store (state lost on restart)");
            Arc::new(InMemoryReplayStore::new(retention))
        }
        "sqlite" => {
            #[cfg(not(feature = "replay-sqlite"))]
            {
                eprintln!(
                    "SQLite replay backend requested, but arrow-node was built without the `replay-sqlite` feature"
                );
                std::process::exit(2);
            }
            #[cfg(feature = "replay-sqlite")]
            {
                let path = resolve_sqlite_replay_path(
                    Path::new(&cli.config),
                    config.replay.path.as_deref(),
                );
                tracing::info!(path = %path.display(), "opening SQLite replay store");
                Arc::new(
                    SqliteReplayStore::open(&path, retention).unwrap_or_else(|e| {
                        eprintln!(
                            "Failed to open SQLite replay store at `{}`: {e}",
                            path.display()
                        );
                        std::process::exit(2);
                    }),
                )
            }
        }
        other => {
            eprintln!("Invalid replay backend `{other}`; expected `memory` or `sqlite`");
            std::process::exit(2);
        }
    };
    spawn_replay_sweep(replay.clone());

    let known_senders: HashSet<String> = config.known_senders.allow.into_iter().collect();

    let state = Arc::new(AppState {
        node_id: config.node_id,
        registries: Arc::new(registries),
        replay,
        known_senders,
        binding,
        identity,
        tls,
    });

    let app = Router::new()
        .route("/arrow/health", get(handlers::health))
        .route("/arrow/events", post(handlers::receive_event))
        .route("/arrow/ack", post(handlers::receive_ack))
        .with_state(state);

    let addr: SocketAddr = config.listen.parse().unwrap_or_else(|e| {
        eprintln!("Invalid listen address: {e}");
        std::process::exit(2);
    });

    tracing::info!(%addr, "listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Cannot bind {addr}: {e}");
            std::process::exit(1);
        });

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap_or_else(|e| {
        eprintln!("Server error: {e}");
        std::process::exit(1);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_replay_path_resolves_relative_to_config_dir() {
        let config_path = std::env::current_dir()
            .unwrap()
            .join("crates/arrow-node/node.example.toml");
        let resolved = resolve_sqlite_replay_path(&config_path, Some("state/replay.db"));

        assert_eq!(
            resolved,
            config_path.parent().unwrap().join("state/replay.db")
        );
    }

    #[test]
    fn sqlite_replay_path_keeps_absolute_path() {
        let absolute = std::env::temp_dir().join("arrow-replay.db");
        let resolved =
            resolve_sqlite_replay_path(Path::new("node.example.toml"), absolute.to_str());

        assert_eq!(resolved, absolute);
    }
}
