-- Create the (previously unmigrated) `voice_assistant_devices` table and add an
-- indexed, keyed-HMAC lookup column for voice-webhook authentication (#2662).
--
-- Background
-- ----------
-- `create_voice_device` / `update_voice_device_tokens` / `find_voice_device` and
-- the voice-webhook auth path (`authenticate_voice_user`,
-- api-server routes/voice_webhooks.rs) have always read and written a
-- `voice_assistant_devices` table, but **no migration ever created it** (grep
-- for `voice_assistant_devices` across backend/**/*.sql returned zero hits
-- before this file — the same missing-DDL class fixed for
-- `portal_webhook_events` in 00219). The table was relying on an ambient /
-- hand-created schema, which is why the `#[sqlx::test]`-backed regression for
-- the multi-device selection path could not be written (see the note in
-- tests/suites/voice_oauth_exchange_auth_tests.rs). This lands the DDL so that
-- selection path is testable and the auth query below is index-backed.
--
-- The #2662 improvement
-- ---------------------
-- `authenticate_voice_user` used to `fetch_all` every active, token-bearing
-- device for the platform (cross-org, system-wide) and AES-GCM-decrypt +
-- constant-time-compare each in a linear scan — O(N) decryptions per webhook
-- request and an amplification/DoS vector. `access_token_hash` stores a keyed
-- HMAC-SHA256 of the access token (keyed with INTEGRATION_ENCRYPTION_KEY, so it
-- is not an offline-guessable plain digest) alongside the encrypted token, so
-- the candidate device is selected by the partial index below in SQL and only
-- that single row is decrypted + constant-time-verified (defence in depth).
--
-- Backfill
-- --------
-- Existing rows keep a NULL `access_token_hash`: the HMAC needs the plaintext
-- token, and only the AES-GCM ciphertext is stored — it cannot be recomputed in
-- SQL. Such rows are re-hashed the next time their token is written (device
-- linking / token refresh); until then `authenticate_voice_user` falls back to
-- the linear scan over NULL-hash rows only, so no device is locked out.
--
-- RLS
-- ---
-- Intentionally NOT `ENABLE/FORCE ROW LEVEL SECURITY`. Voice devices are matched
-- on the *unauthenticated* voice-webhook path (`authenticate_voice_user` runs a
-- cross-org token lookup; `oauth_token_refresh` uses a context-cleared public
-- connection), which carries no `app.current_organization_id` GUC. An org-scoped
-- policy would deny-all those reads. `organization_id` / `user_id` are logical
-- FKs (bound from the verified PM access token on the write paths) but are left
-- unconstrained here to keep the unauthenticated match path working.

CREATE TABLE IF NOT EXISTS voice_assistant_devices (
    id                       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id          UUID NOT NULL,
    user_id                  UUID NOT NULL,
    unit_id                  UUID,
    platform                 VARCHAR(50) NOT NULL,   -- alexa, google_assistant
    device_id                VARCHAR(255) NOT NULL,
    device_name              VARCHAR(255),
    linked_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at             TIMESTAMPTZ,
    access_token_encrypted   TEXT,                   -- base64(nonce+AES-GCM ciphertext)
    refresh_token_encrypted  TEXT,
    token_expires_at         TIMESTAMPTZ,
    is_active                BOOLEAN NOT NULL DEFAULT TRUE,
    capabilities             JSONB NOT NULL DEFAULT '[]'::jsonb,
    access_token_hash        BYTEA,                  -- keyed HMAC-SHA256 lookup key (#2662)
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- For environments where the table already existed (hand-created schema) without
-- the lookup column, add it idempotently.
ALTER TABLE voice_assistant_devices
    ADD COLUMN IF NOT EXISTS access_token_hash BYTEA;

-- Fast-path selection index for `authenticate_voice_user`:
--   WHERE platform = $1 AND is_active = TRUE AND access_token_hash = $2 LIMIT 1
-- Partial (hash IS NOT NULL) so un-backfilled rows don't bloat it.
CREATE INDEX IF NOT EXISTS idx_voice_devices_platform_token_hash
    ON voice_assistant_devices (platform, access_token_hash)
    WHERE access_token_hash IS NOT NULL;

-- Supporting indexes for the other lookup paths (by owner, by external device).
CREATE INDEX IF NOT EXISTS idx_voice_devices_user
    ON voice_assistant_devices (user_id);

CREATE INDEX IF NOT EXISTS idx_voice_devices_platform_device
    ON voice_assistant_devices (platform, device_id);
