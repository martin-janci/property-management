-- Migration: 00138_create_capability_grants.sql
-- Phase 5: Super-admin Control Plane
--
-- Capability registry storage. Each row grants a single named capability to a
-- single user. The `RequireCapability` extractor in `admin-core` checks for an
-- active (non-revoked, non-expired) row before authorizing an admin action.
--
-- Defenses (from the brainstorming session):
--   * Leak #21 (super-admin = single catastrophic point): every grant is
--     recorded with `granted_by` and the application layer enforces that
--     `granted_by != user_id`, i.e. no platform principal may self-grant.
--   * Leak #12 (escalation to platform = escalation to god):
--     `mfa_required` defaults to TRUE; the extractor refuses without a recent
--     MFA verification.
--
-- Append-only-ish: revocation sets `revoked_at` rather than deleting the row.
-- The audit trail is preserved.

CREATE TABLE IF NOT EXISTS capability_grants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    capability VARCHAR(64) NOT NULL,
    granted_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Optional expiry. NULL means "until revoked".
    expires_at TIMESTAMPTZ,
    -- Soft revoke; preserves the audit trail.
    revoked_at TIMESTAMPTZ,
    revoked_by UUID REFERENCES users(id) ON DELETE SET NULL,
    -- Whether the extractor must require recent MFA before honoring this grant.
    -- Defaults to TRUE; only specific build-only/automation capabilities should
    -- ever flip this to FALSE.
    mfa_required BOOLEAN NOT NULL DEFAULT TRUE,
    -- Optional per-grant note (why was this granted, by what ticket, etc).
    note TEXT
);

-- A user may hold each capability at most once *while it is live* (revoked
-- rows are kept for audit, so the index excludes them).
CREATE UNIQUE INDEX IF NOT EXISTS idx_capability_grants_unique_live
    ON capability_grants(user_id, capability)
    WHERE revoked_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_capability_grants_user ON capability_grants(user_id);
CREATE INDEX IF NOT EXISTS idx_capability_grants_capability ON capability_grants(capability);
CREATE INDEX IF NOT EXISTS idx_capability_grants_granted_by ON capability_grants(granted_by);

-- RLS: platform principals only. There is no per-user "see your own grants"
-- escape hatch — UI surfaces this through the admin API, never directly.
ALTER TABLE capability_grants ENABLE ROW LEVEL SECURITY;

CREATE POLICY capability_grants_super_admin ON capability_grants
    FOR ALL
    USING (is_super_admin());

COMMENT ON TABLE capability_grants IS
    'Phase 5: capability grants for platform principals. Append-only: revocation sets revoked_at.';
COMMENT ON COLUMN capability_grants.mfa_required IS
    'When TRUE (default), the RequireCapability extractor refuses unless the user has a recent MFA verification.';
COMMENT ON COLUMN capability_grants.granted_by IS
    'Who granted this capability. Application layer enforces granted_by != user_id (no self-grant).';

-- =============================================================================
-- two_factor_auth_verifications: short-lived MFA verification record.
-- =============================================================================
--
-- Phase 5 introduces a recency check ("did this user MFA-verify within the
-- last 15 minutes?"). Tracking that requires a row per successful TOTP /
-- backup-code verification. The `user_2fa` table tracks setup state, not
-- per-attempt history.
--
-- Rows are inserted by the auth/MFA layer on a successful verify. They are
-- effectively transient — a periodic job MAY purge entries older than 24h —
-- but the table itself is RLS-protected and append-only at the application
-- layer.

CREATE TABLE IF NOT EXISTS two_factor_auth_verifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- 'totp' | 'backup_code' | 'webauthn' (future).
    method VARCHAR(32) NOT NULL,
    verified_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Optional context: ip + UA captured at verify time. Useful for audit
    -- (paired with capability checks) and for forensic review.
    ip_address VARCHAR(45),
    user_agent TEXT
);

CREATE INDEX IF NOT EXISTS idx_two_factor_auth_verifications_user_recent
    ON two_factor_auth_verifications(user_id, verified_at DESC);

ALTER TABLE two_factor_auth_verifications ENABLE ROW LEVEL SECURITY;

CREATE POLICY tfa_verifications_super_admin ON two_factor_auth_verifications
    FOR ALL
    USING (is_super_admin());

CREATE POLICY tfa_verifications_select_own ON two_factor_auth_verifications
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::uuid);

CREATE POLICY tfa_verifications_insert ON two_factor_auth_verifications
    FOR INSERT
    WITH CHECK (true);

COMMENT ON TABLE two_factor_auth_verifications IS
    'Phase 5: per-attempt MFA verification log. The 15-minute recency check reads from this table.';

-- =============================================================================
-- impersonation_tokens: short-lived (15 min) impersonation grants.
-- =============================================================================

CREATE TABLE IF NOT EXISTS impersonation_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Opaque random token (hashed, like password resets).
    token_hash VARCHAR(128) NOT NULL UNIQUE,
    actor_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- The capability that justified this impersonation (e.g. 'users.impersonate').
    justifying_capability VARCHAR(64) NOT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    -- Soft-stop: an operator may end the impersonation early.
    ended_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_impersonation_tokens_actor
    ON impersonation_tokens(actor_id);
CREATE INDEX IF NOT EXISTS idx_impersonation_tokens_target
    ON impersonation_tokens(target_user_id);
CREATE INDEX IF NOT EXISTS idx_impersonation_tokens_expires
    ON impersonation_tokens(expires_at);

ALTER TABLE impersonation_tokens ENABLE ROW LEVEL SECURITY;

CREATE POLICY impersonation_tokens_super_admin ON impersonation_tokens
    FOR ALL
    USING (is_super_admin());

COMMENT ON TABLE impersonation_tokens IS
    'Phase 5: short-lived (15 min default) impersonation grants. Frontend renders a banner whenever a token is active.';

-- =============================================================================
-- audit_logs: append-only enforcement.
-- =============================================================================
--
-- The existing `audit_logs` table (migration 00025) already documents itself
-- as append-only and has no UPDATE / DELETE RLS policies. That is necessary
-- but not sufficient: the table OWNER (and any role with ALL bypass) can
-- still mutate rows, and an UPDATE under `is_super_admin()` would silently
-- succeed because the super-admin policy is `FOR ALL`. Phase 5 adds a
-- belt-and-braces trigger that REJECTs UPDATE/DELETE unconditionally.

CREATE OR REPLACE FUNCTION audit_logs_reject_mutation()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION
        'audit_logs is append-only: % rejected on row %',
        TG_OP, COALESCE(OLD.id::text, 'unknown')
        USING ERRCODE = 'feature_not_supported';
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_audit_logs_no_update ON audit_logs;
CREATE TRIGGER trg_audit_logs_no_update
    BEFORE UPDATE ON audit_logs
    FOR EACH ROW
    EXECUTE FUNCTION audit_logs_reject_mutation();

DROP TRIGGER IF EXISTS trg_audit_logs_no_delete ON audit_logs;
CREATE TRIGGER trg_audit_logs_no_delete
    BEFORE DELETE ON audit_logs
    FOR EACH ROW
    EXECUTE FUNCTION audit_logs_reject_mutation();

COMMENT ON FUNCTION audit_logs_reject_mutation() IS
    'Phase 5 hardening: enforces append-only on audit_logs even for table-owner / super-admin paths.';
