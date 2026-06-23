-- Story 2B.7 / BIT-179: generic idempotency-key ledger for replay-safe writes.
--
-- Stores the canonical response body for a tenant-scoped `(method, path, key)`
-- tuple so middleware can replay the original response for 24 hours instead of
-- re-running a duplicate mutation.

CREATE TABLE IF NOT EXISTS idempotency_keys (
    tenant_scope TEXT NOT NULL,
    request_method TEXT NOT NULL,
    request_path TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    response_status INTEGER NOT NULL,
    response_content_type TEXT,
    response_body BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '24 hours'),
    PRIMARY KEY (tenant_scope, request_method, request_path, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_idempotency_keys_expires_at
    ON idempotency_keys (expires_at);

COMMENT ON TABLE idempotency_keys IS
    'Tenant-scoped cached responses for Idempotency-Key request replay.';
COMMENT ON COLUMN idempotency_keys.tenant_scope IS
    'Resolved tenant scope (host tenant, X-Tenant-ID, or global sentinel).';
COMMENT ON COLUMN idempotency_keys.request_hash IS
    'SHA-256 of method + path + request body for mismatch detection.';
