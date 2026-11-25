use crate::error::ReplayStoreError;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Mutex;

#[cfg(feature = "replay-sqlite")]
use {
    rusqlite::{params, Connection, ErrorCode},
    std::path::{Path, PathBuf},
};

#[cfg(feature = "replay-sqlite")]
fn format_sqlite_ts(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

/// Trait for replay protection state. Implementations are Send + Sync.
pub trait ReplayStore: Send + Sync {
    /// Returns `true` if (sender_id, nonce) was already seen within the retention window.
    /// If not seen, records it and returns `false`.
    fn check_and_record(&self, sender_id: &str, nonce: &str) -> Result<bool, ReplayStoreError>;

    /// Prune expired replay entries. Returns the number of deleted entries when known.
    fn sweep(&self) -> Result<u64, ReplayStoreError>;
}

// ── InMemoryReplayStore ───────────────────────────────────────────────────────

/// In-memory replay store. Keyed by sender_id -> (nonce -> first_seen_at).
/// Prunes expired entries lazily on each call.
/// Suitable for development and testing. State is lost on process restart.
pub struct InMemoryReplayStore {
    retention: Duration,
    seen: Mutex<HashMap<String, HashMap<String, DateTime<Utc>>>>,
}

impl InMemoryReplayStore {
    pub fn new(retention: Duration) -> Self {
        Self {
            retention,
            seen: Mutex::new(HashMap::new()),
        }
    }

    /// Default 24-hour retention for the http-rest binding (PT24H).
    pub fn with_24h_retention() -> Self {
        Self::new(Duration::hours(24))
    }
}

impl ReplayStore for InMemoryReplayStore {
    fn check_and_record(&self, sender_id: &str, nonce: &str) -> Result<bool, ReplayStoreError> {
        let mut guard = self
            .seen
            .lock()
            .map_err(|_| ReplayStoreError::LockPoisoned)?;
        let now = Utc::now();
        let cutoff = now - self.retention;

        let sender_map = guard.entry(sender_id.to_string()).or_default();

        // Prune expired nonces for this sender lazily
        sender_map.retain(|_, ts| *ts > cutoff);

        if sender_map.contains_key(nonce) {
            return Ok(true); // replay
        }
        sender_map.insert(nonce.to_string(), now);
        Ok(false)
    }

    fn sweep(&self) -> Result<u64, ReplayStoreError> {
        let mut guard = self
            .seen
            .lock()
            .map_err(|_| ReplayStoreError::LockPoisoned)?;
        let cutoff = Utc::now() - self.retention;
        let before = guard.values().map(HashMap::len).sum::<usize>();
        for sender_map in guard.values_mut() {
            sender_map.retain(|_, ts| *ts > cutoff);
        }
        guard.retain(|_, sender_map| !sender_map.is_empty());
        let after = guard.values().map(HashMap::len).sum::<usize>();
        Ok((before - after) as u64)
    }
}

// ── SqliteReplayStore ─────────────────────────────────────────────────────────

/// Durable replay store backed by SQLite.
///
/// Nonces are persisted across process restarts. Expired entries are pruned
/// lazily on each `check_and_record` call.
///
/// Schema:
/// ```sql
/// CREATE TABLE replay_nonces (
///     sender_id    TEXT NOT NULL,
///     nonce        TEXT NOT NULL,
///     first_seen_at TEXT NOT NULL,
///     expires_at    TEXT NOT NULL,
///     PRIMARY KEY (sender_id, nonce)
/// )
/// ```
#[cfg(feature = "replay-sqlite")]
pub struct SqliteReplayStore {
    conn: Mutex<Connection>,
    retention: Duration,
}

#[cfg(feature = "replay-sqlite")]
impl SqliteReplayStore {
    const SCHEMA: &'static str = "
        CREATE TABLE IF NOT EXISTS replay_nonces (
            sender_id     TEXT    NOT NULL,
            nonce         TEXT    NOT NULL,
            first_seen_at TEXT    NOT NULL,
            expires_at    TEXT    NOT NULL,
            PRIMARY KEY (sender_id, nonce)
        );
        CREATE INDEX IF NOT EXISTS idx_replay_expires_at ON replay_nonces (expires_at);
    ";

    /// Open (or create) a SQLite database at `path` with the given retention.
    pub fn open(path: &Path, retention: Duration) -> Result<Self, ReplayStoreError> {
        let conn = Connection::open(path).map_err(|source| ReplayStoreError::Open {
            path: path.display().to_string(),
            message: source.to_string(),
        })?;
        Self::initialize(conn, path.to_path_buf(), retention)
    }

    /// Open (or create) a SQLite database with the default http-rest PT24H retention.
    pub fn open_with_24h_retention(path: &Path) -> Result<Self, ReplayStoreError> {
        Self::open(path, Duration::hours(24))
    }

    /// Open an in-memory SQLite database. Data is lost when the store is dropped.
    /// Used for tests and transient node runs where persistence is not required.
    pub fn open_in_memory(retention: Duration) -> Result<Self, ReplayStoreError> {
        let conn = Connection::open_in_memory().map_err(|source| ReplayStoreError::Open {
            path: ":memory:".to_string(),
            message: source.to_string(),
        })?;
        Self::initialize(conn, PathBuf::from(":memory:"), retention)
    }

    fn initialize(
        conn: Connection,
        path: PathBuf,
        retention: Duration,
    ) -> Result<Self, ReplayStoreError> {
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|source| ReplayStoreError::Initialize {
                path: path.display().to_string(),
                message: source.to_string(),
            })?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|source| ReplayStoreError::Initialize {
                path: path.display().to_string(),
                message: source.to_string(),
            })?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|source| ReplayStoreError::Initialize {
                path: path.display().to_string(),
                message: source.to_string(),
            })?;
        conn.execute_batch(Self::SCHEMA)
            .map_err(|source| ReplayStoreError::Initialize {
                path: path.display().to_string(),
                message: source.to_string(),
            })?;
        Ok(Self {
            conn: Mutex::new(conn),
            retention,
        })
    }

    fn sweep_at(&self, now: DateTime<Utc>) -> Result<u64, ReplayStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ReplayStoreError::LockPoisoned)?;
        let deleted = conn
            .execute(
                "DELETE FROM replay_nonces WHERE expires_at <= ?1",
                params![format_sqlite_ts(now)],
            )
            .map_err(|source| ReplayStoreError::Io {
                source: source.to_string(),
            })?;
        Ok(deleted as u64)
    }
}

#[cfg(feature = "replay-sqlite")]
impl ReplayStore for SqliteReplayStore {
    fn check_and_record(&self, sender_id: &str, nonce: &str) -> Result<bool, ReplayStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ReplayStoreError::LockPoisoned)?;
        let now = Utc::now();
        let expires_at = now + self.retention;

        conn.execute(
            "DELETE FROM replay_nonces WHERE expires_at <= ?1",
            params![format_sqlite_ts(now)],
        )
        .map_err(|source| ReplayStoreError::Io {
            source: source.to_string(),
        })?;

        match conn.execute(
            "INSERT INTO replay_nonces (sender_id, nonce, first_seen_at, expires_at) \
             VALUES (?1, ?2, ?3, ?4)",
            params![
                sender_id,
                nonce,
                format_sqlite_ts(now),
                format_sqlite_ts(expires_at)
            ],
        ) {
            Ok(_) => Ok(false),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == ErrorCode::ConstraintViolation =>
            {
                Ok(true)
            }
            Err(source) => Err(ReplayStoreError::Io {
                source: source.to_string(),
            }),
        }
    }

    fn sweep(&self) -> Result<u64, ReplayStoreError> {
        self.sweep_at(Utc::now())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── InMemoryReplayStore ───────────────────────────────────────────────────

    #[test]
    fn first_nonce_returns_false() {
        let store = InMemoryReplayStore::with_24h_retention();
        assert!(!store.check_and_record("sender-a", "nonce-1").unwrap());
    }

    #[test]
    fn repeated_nonce_returns_true() {
        let store = InMemoryReplayStore::with_24h_retention();
        store.check_and_record("sender-a", "nonce-1").unwrap();
        assert!(store.check_and_record("sender-a", "nonce-1").unwrap());
    }

    #[test]
    fn same_nonce_different_sender_is_ok() {
        let store = InMemoryReplayStore::with_24h_retention();
        store.check_and_record("sender-a", "nonce-1").unwrap();
        assert!(!store.check_and_record("sender-b", "nonce-1").unwrap());
    }

    #[test]
    fn expired_nonce_is_accepted_again() {
        // Use a 1 nanosecond retention so it expires immediately
        let store = InMemoryReplayStore::new(Duration::nanoseconds(1));
        store.check_and_record("sender-a", "nonce-1").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        assert!(
            !store.check_and_record("sender-a", "nonce-1").unwrap(),
            "expired nonce should be accepted again"
        );
    }

    #[test]
    fn in_memory_lock_poison_returns_error() {
        let store = std::sync::Arc::new(InMemoryReplayStore::with_24h_retention());
        let poison_store = store.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poison_store.seen.lock().unwrap();
            panic!("poison replay mutex");
        })
        .join();

        let err = store
            .check_and_record("sender-a", "nonce-1")
            .expect_err("poisoned replay mutex should return an error");
        assert!(matches!(err, ReplayStoreError::LockPoisoned));
    }

    // ── SqliteReplayStore ─────────────────────────────────────────────────────

    #[cfg(feature = "replay-sqlite")]
    fn remove_sqlite_files(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_first_nonce_returns_false() {
        let store = SqliteReplayStore::open_in_memory(Duration::hours(24)).unwrap();
        assert!(!store.check_and_record("sender-a", "nonce-1").unwrap());
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_repeated_nonce_returns_true() {
        let store = SqliteReplayStore::open_in_memory(Duration::hours(24)).unwrap();
        store.check_and_record("sender-a", "nonce-1").unwrap();
        assert!(store.check_and_record("sender-a", "nonce-1").unwrap());
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_same_nonce_different_sender_is_ok() {
        let store = SqliteReplayStore::open_in_memory(Duration::hours(24)).unwrap();
        store.check_and_record("sender-a", "nonce-1").unwrap();
        assert!(!store.check_and_record("sender-b", "nonce-1").unwrap());
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_expired_nonce_is_accepted_again() {
        let store = SqliteReplayStore::open_in_memory(Duration::zero()).unwrap();
        store.check_and_record("sender-a", "nonce-1").unwrap();
        {
            let conn = store.conn.lock().unwrap();
            conn.execute(
                "UPDATE replay_nonces SET expires_at = ?1 WHERE sender_id = ?2 AND nonce = ?3",
                params![
                    (Utc::now() - Duration::seconds(1)).to_rfc3339(),
                    "sender-a",
                    "nonce-1"
                ],
            )
            .unwrap();
        }
        assert!(
            !store.check_and_record("sender-a", "nonce-1").unwrap(),
            "expired sqlite nonce should be accepted again"
        );
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_retention_boundary_prunes_exact_expiry_only() {
        let retention = Duration::hours(24);
        let now = Utc::now();
        let exact_first_seen = now - retention;
        let kept_first_seen = exact_first_seen + Duration::microseconds(1);
        let store = SqliteReplayStore::open_in_memory(retention).unwrap();

        {
            let conn = store.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO replay_nonces (sender_id, nonce, first_seen_at, expires_at) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    "sender-a",
                    "exact-expiry",
                    format_sqlite_ts(exact_first_seen),
                    format_sqlite_ts(exact_first_seen + retention)
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO replay_nonces (sender_id, nonce, first_seen_at, expires_at) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    "sender-a",
                    "kept-expiry",
                    format_sqlite_ts(kept_first_seen),
                    format_sqlite_ts(kept_first_seen + retention)
                ],
            )
            .unwrap();
        }

        assert_eq!(store.sweep_at(now).unwrap(), 1);
        let remaining: i64 = {
            let conn = store.conn.lock().unwrap();
            conn.query_row(
                "SELECT COUNT(*) FROM replay_nonces WHERE sender_id = ?1 AND nonce = ?2",
                params!["sender-a", "kept-expiry"],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(remaining, 1);
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_write_failure_returns_error_not_panic() {
        let store = SqliteReplayStore::open_in_memory(Duration::hours(24)).unwrap();
        {
            let conn = store.conn.lock().unwrap();
            conn.pragma_update(None, "query_only", "ON").unwrap();
        }

        let err = store
            .check_and_record("sender-a", "nonce-1")
            .expect_err("readonly sqlite connection should return an error");
        assert!(matches!(err, ReplayStoreError::Io { .. }));
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_full_returns_error_not_panic() {
        let path =
            std::env::temp_dir().join(format!("arrow-replay-full-{}.db", uuid::Uuid::new_v4()));
        let store = SqliteReplayStore::open(&path, Duration::hours(24)).unwrap();
        {
            let conn = store.conn.lock().unwrap();
            let page_count: i64 = conn
                .query_row("PRAGMA page_count", [], |row| row.get(0))
                .unwrap();
            conn.pragma_update(None, "max_page_count", page_count)
                .unwrap();
        }

        let huge_nonce = "n".repeat(256 * 1024);
        let err = store
            .check_and_record("sender-a", &huge_nonce)
            .expect_err("page-limited sqlite database should return an error");

        assert!(matches!(err, ReplayStoreError::Io { .. }));
        assert!(
            err.to_string().contains("full"),
            "expected SQLITE_FULL-like error, got {err}"
        );
        remove_sqlite_files(&path);
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_lock_poison_returns_error() {
        let store =
            std::sync::Arc::new(SqliteReplayStore::open_in_memory(Duration::hours(24)).unwrap());
        let poison_store = store.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poison_store.conn.lock().unwrap();
            panic!("poison sqlite replay mutex");
        })
        .join();

        let err = store
            .check_and_record("sender-a", "nonce-1")
            .expect_err("poisoned sqlite mutex should return an error");
        assert!(matches!(err, ReplayStoreError::LockPoisoned));
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_multiple_senders_isolated() {
        let store = SqliteReplayStore::open_in_memory(Duration::hours(24)).unwrap();
        store.check_and_record("alice", "abc").unwrap();
        store.check_and_record("bob", "xyz").unwrap();
        // Same nonce, different sender — not a replay
        assert!(!store.check_and_record("alice", "xyz").unwrap());
        assert!(!store.check_and_record("bob", "abc").unwrap());
        // Repeated (sender, nonce) pairs — replays
        assert!(store.check_and_record("alice", "abc").unwrap());
        assert!(store.check_and_record("bob", "xyz").unwrap());
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_persists_nonce_across_reopen() {
        let path = std::env::temp_dir().join(format!("arrow-replay-{}.db", uuid::Uuid::new_v4()));
        {
            let store = SqliteReplayStore::open(&path, Duration::hours(24)).unwrap();
            assert!(!store.check_and_record("sender-a", "nonce-1").unwrap());
        }
        {
            let store = SqliteReplayStore::open(&path, Duration::hours(24)).unwrap();
            assert!(store.check_and_record("sender-a", "nonce-1").unwrap());
        }
        remove_sqlite_files(&path);
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_concurrent_unique_writers_do_not_replay() {
        let store =
            std::sync::Arc::new(SqliteReplayStore::open_in_memory(Duration::hours(24)).unwrap());
        let mut threads = Vec::new();
        for thread_id in 0..32 {
            let store = store.clone();
            threads.push(std::thread::spawn(move || {
                for nonce_id in 0..1000 {
                    let nonce = format!("nonce-{thread_id}-{nonce_id}");
                    assert!(!store.check_and_record("sender-a", &nonce).unwrap());
                }
            }));
        }
        for thread in threads {
            thread.join().unwrap();
        }
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_concurrent_same_nonce_has_single_winner() {
        let store =
            std::sync::Arc::new(SqliteReplayStore::open_in_memory(Duration::hours(24)).unwrap());
        let mut threads = Vec::new();
        for _ in 0..2 {
            let store = store.clone();
            threads.push(std::thread::spawn(move || {
                store.check_and_record("sender-a", "race-nonce").unwrap()
            }));
        }
        let results: Vec<bool> = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|seen| !**seen).count(), 1);
        assert_eq!(results.iter().filter(|seen| **seen).count(), 1);
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_busy_writer_waits_instead_of_erroring() {
        let path =
            std::env::temp_dir().join(format!("arrow-replay-busy-{}.db", uuid::Uuid::new_v4()));
        let store_a = SqliteReplayStore::open(&path, Duration::hours(24)).unwrap();
        let store_b = SqliteReplayStore::open(&path, Duration::hours(24)).unwrap();

        let conn_a = store_a.conn.lock().unwrap();
        conn_a.execute_batch("BEGIN IMMEDIATE").unwrap();

        let writer = std::thread::spawn(move || {
            store_b
                .check_and_record("sender-a", "busy-nonce")
                .expect("busy writer should wait for the lock")
        });
        std::thread::sleep(std::time::Duration::from_millis(100));
        conn_a.execute_batch("COMMIT").unwrap();

        assert!(!writer.join().unwrap());
        remove_sqlite_files(&path);
    }

    #[cfg(feature = "replay-sqlite")]
    #[test]
    fn sqlite_corrupted_db_errors_cleanly() {
        let path =
            std::env::temp_dir().join(format!("arrow-replay-corrupt-{}.db", uuid::Uuid::new_v4()));
        std::fs::write(&path, b"not a sqlite database").unwrap();
        let err = match SqliteReplayStore::open(&path, Duration::hours(24)) {
            Ok(_) => panic!("corrupt sqlite replay database should fail to open"),
            Err(err) => err,
        };
        assert!(err
            .to_string()
            .contains("failed to initialize replay store"));
        remove_sqlite_files(&path);
    }
}
