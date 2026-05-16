-- Migration: 00149_mfa_enrollment.sql
-- Phase 6 (B6): MFA enrollment + recovery codes for platform principals.
--
-- The existing `user_2fa` table stores the TOTP secret and `enabled_at` for
-- all users. Platform principals use the same secret storage so enrollment
-- triggers the same 15-minute recency check already in place.
--
-- What is NEW here is a separate `mfa_recovery_codes` table that gives each
-- code its own row — this supports:
--   * single-use tracking per code (used_at NOT NULL = consumed)
--   * audit-friendly: who used which code and when
--   * independent expiry / rotation without replacing the whole JSONB blob
--
-- The existing `user_2fa.backup_codes` JSONB field continues to serve
-- non-platform users. Platform principals use `mfa_recovery_codes` instead.

-- Per-code recovery code table for platform principals.
CREATE TABLE IF NOT EXISTS mfa_recovery_codes (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Argon2id hash of the plaintext code (plaintext is returned ONCE at enroll time).
    code_hash   TEXT        NOT NULL,
    -- NULL = unused. SET to NOW() when the code is consumed.
    used_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mfa_recovery_codes_user
    ON mfa_recovery_codes(user_id)
    WHERE used_at IS NULL;

-- RLS: platform principals only (same pattern as capability_grants).
ALTER TABLE mfa_recovery_codes ENABLE ROW LEVEL SECURITY;

CREATE POLICY mfa_recovery_codes_super_admin ON mfa_recovery_codes
    FOR ALL
    USING (is_super_admin());

-- Self-read: a platform principal can read their own unused codes (e.g. for
-- "how many codes remain?" UI). They cannot read code_hash — that is enforced
-- at the application layer.
CREATE POLICY mfa_recovery_codes_select_own ON mfa_recovery_codes
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::uuid);

-- Insert policy: application layer inserts on enrollment.
CREATE POLICY mfa_recovery_codes_insert ON mfa_recovery_codes
    FOR INSERT
    WITH CHECK (true);

-- Update policy: mark used_at when consuming a code.
CREATE POLICY mfa_recovery_codes_update_own ON mfa_recovery_codes
    FOR UPDATE
    USING (user_id = current_setting('app.current_user_id', true)::uuid);

COMMENT ON TABLE mfa_recovery_codes IS
    'Phase 6 (B6): per-code recovery codes for platform principals. Each row is a single-use code; used_at IS NULL means available.';
COMMENT ON COLUMN mfa_recovery_codes.code_hash IS
    'Argon2id hash of the plaintext recovery code. Plaintext is returned exactly once at enrollment.';
COMMENT ON COLUMN mfa_recovery_codes.used_at IS
    'When this code was consumed. NULL = still available. Once set, the code is permanently exhausted.';
