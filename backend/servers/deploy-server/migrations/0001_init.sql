-- CHECK constraints below pin enum-like text columns to the supported set of
-- values from the Rust domain types. They mirror the `#[serde(rename_all =
-- "lowercase")]` discriminants of `BackendMode`, `WorktreeState`, and
-- `ReleaseState`, plus the `TargetKind::as_str()` values for `release.target`.
-- Without these, a bug or manual edit could persist e.g. `state = 'pasued'`
-- and the row would deserialize-fail or silently be skipped by `WHERE state =
-- 'paused'` filters. SQLite enforces CHECK at INSERT/UPDATE time.

CREATE TABLE worktree (
  name              TEXT PRIMARY KEY,
  branch            TEXT NOT NULL,
  backend_mode      TEXT NOT NULL CHECK (backend_mode IN ('shared', 'dedicated')),
  state             TEXT NOT NULL CHECK (state IN ('running', 'paused', 'closing', 'closed')),
  urls              TEXT NOT NULL,                 -- JSON
  containers        TEXT NOT NULL,                 -- JSON array
  db_name           TEXT,
  dump_path         TEXT,
  ttl_seconds       INTEGER NOT NULL DEFAULT 172800,
  last_traffic_at   INTEGER,                       -- unix ts seconds
  closed_at         INTEGER,
  created_at        INTEGER NOT NULL,
  created_by        TEXT NOT NULL
);

CREATE INDEX idx_worktree_state ON worktree(state);

CREATE TABLE release (
  tag               TEXT PRIMARY KEY,
  images            TEXT NOT NULL,                 -- JSON
  state             TEXT NOT NULL CHECK (state IN ('candidate', 'staging', 'prod', 'previous', 'archived')),
  target            TEXT CHECK (target IS NULL OR target IN ('staging', 'prod')),
  promoted_at       INTEGER,
  notes             TEXT
);

CREATE TABLE audit (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  ts                INTEGER NOT NULL,
  caller_kind       TEXT NOT NULL,
  caller_id         TEXT NOT NULL,
  endpoint          TEXT NOT NULL,
  params            TEXT,                          -- JSON
  result            TEXT,
  duration_ms       INTEGER
);

CREATE INDEX idx_audit_ts ON audit(ts);
