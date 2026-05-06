CREATE TABLE worktree (
  name              TEXT PRIMARY KEY,
  branch            TEXT NOT NULL,
  backend_mode      TEXT NOT NULL,
  state             TEXT NOT NULL,
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
  state             TEXT NOT NULL,
  target            TEXT,
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
