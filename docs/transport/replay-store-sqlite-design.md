# Arrow Replay Store — SQLite Design Note v0.1

**Status:** Design draft — not implemented  
**Replaces:** `InMemoryReplayStore` for persistent/multi-restart deployments  
**Date:** 2026-05-23

---

## Motivation

The pilot replay store (`InMemoryReplayStore`) does not survive node restarts. A restarted node briefly accepts replays of nonces seen before the restart. For any deployment where uptime cannot be guaranteed — including scheduled maintenance restarts — a persistent store is required.

This document specifies the minimal SQLite-backed implementation that satisfies Transport Model Part 3.3 with restart-safe semantics.

---

## Schema

```sql
CREATE TABLE IF NOT EXISTS replay_nonces (
    sender_id    TEXT NOT NULL,
    nonce        TEXT NOT NULL,
    first_seen_at TEXT NOT NULL,   -- ISO 8601 UTC, e.g. "2026-05-23T12:00:00Z"
    expires_at    TEXT NOT NULL,   -- ISO 8601 UTC; first_seen_at + nonce_retention_min
    PRIMARY KEY (sender_id, nonce)
);

CREATE INDEX IF NOT EXISTS idx_replay_expires_at ON replay_nonces (expires_at);
```

`PRIMARY KEY (sender_id, nonce)` enforces uniqueness at the database level. A duplicate INSERT is the detection event.

---

## Retention and Pruning

`expires_at` is written as `first_seen_at + nonce_retention_min` (e.g. +24 hours for the `http-rest` binding). Expired rows are pruned lazily on each `check_and_record` call using:

```sql
DELETE FROM replay_nonces WHERE expires_at < ?;   -- ? = Utc::now()
```

A background pruning task (every 15 minutes) is recommended for nodes with high throughput to prevent unbounded table growth.

---

## `check_and_record` Logic

```sql
-- 1. Prune expired entries
DELETE FROM replay_nonces WHERE expires_at < :now;

-- 2. Attempt insert (fails on duplicate PRIMARY KEY)
INSERT INTO replay_nonces (sender_id, nonce, first_seen_at, expires_at)
VALUES (:sender_id, :nonce, :now, :expires_at);
```

If the INSERT succeeds → nonce is new, return `false`.  
If the INSERT fails with `UNIQUE constraint failed` → replay detected, return `true`.

This is a single round-trip with no SELECT required; the constraint violation is the check.

---

## Rust Implementation Notes

- Use `sqlx` with `sqlx::SqlitePool` and WAL mode (`PRAGMA journal_mode=WAL`) for concurrent read performance.
- The database file path is provided via node config (`replay_store_path`). If absent, fall back to `InMemoryReplayStore`.
- `SqliteReplayStore` implements the existing `ReplayStore` trait — zero changes to handlers.
- Use `PRAGMA synchronous=NORMAL` for a balance of durability and write throughput; `FULL` is unnecessarily slow for a nonce log.

---

## Config Addition

```toml
[replay]
backend = "sqlite"
path    = "/var/lib/arrow-node/replay.db"
```

If `backend = "memory"` (default) or the `[replay]` section is absent, `InMemoryReplayStore` is used.

---

## Known Limitations of This Design

- SQLite is not suitable for horizontally scaled deployments (multiple nodes sharing a single replay store). A Redis or PostgreSQL backend is required for that case.
- WAL mode limits writes to one writer at a time; this is acceptable for a single-node pilot.
- Clock skew between the node and the SQLite system clock must be under 1 second for `expires_at` comparisons to be reliable.

---

*Arrow Protocol · SQLite Replay Store Design · v0.1 · 2026-05-23*
