-- Migration: 00145_capability_revoke_requires_mfa.sql
-- Phase 2 hardening (E3): defense-in-depth for the capability-revoke path.
--
-- Background
-- ----------
-- D2 wired `AuthPolicyEnforcer::check_capability_revoke` into the
-- `DELETE /admin/capabilities/{grant_id}` route, so the application layer
-- already verifies a recent MFA verification before issuing the soft-revoke
-- UPDATE. The schema, however, has no constraint enforcing this — a future
-- code path that bypasses the enforcer (or a SQL injection that hits the
-- table directly) could still revoke a grant without an MFA attestation.
--
-- This migration adds a row-level trigger that REJECTs any UPDATE which
-- transitions `revoked_at` from NULL to a non-NULL value unless the same
-- statement also writes `revoked_with_mfa_at` within the RECENT_MFA_WINDOW
-- (5 minutes). The application layer already enforces this via
-- `AuthPolicyEnforcer` (D2); this trigger is the defense-in-depth layer.
--
-- Acceptance:
--   * Direct UPDATE that sets `revoked_at` without `revoked_with_mfa_at`
--     => exception.
--   * UPDATE with stale `revoked_with_mfa_at` (older than 5 minutes)
--     => exception.
--   * UPDATE with `revoked_with_mfa_at` set to NOW() => succeeds.
--   * UPDATEs that do not change `revoked_at` are unaffected.

ALTER TABLE capability_grants
    ADD COLUMN IF NOT EXISTS revoked_with_mfa_at TIMESTAMPTZ;

COMMENT ON COLUMN capability_grants.revoked_with_mfa_at IS
    'Phase 2 (E3) hardening: timestamp of the recent MFA verification that '
    'authorized the revoke. Enforced by trigger `capability_grants_revoke_mfa_check` '
    'to be within 5 minutes of the actual revoke. The application layer '
    '(`AuthPolicyEnforcer::check_capability_revoke`) populates this with NOW() '
    'after the recent-MFA precondition has already been validated.';

CREATE OR REPLACE FUNCTION enforce_capability_revoke_mfa()
RETURNS TRIGGER AS $$
DECLARE
    mfa_window CONSTANT INTERVAL := '5 minutes';
BEGIN
    -- Only fire on the revoke transition (NULL -> NOT NULL on revoked_at).
    IF NEW.revoked_at IS NOT NULL AND OLD.revoked_at IS NULL THEN
        IF NEW.revoked_with_mfa_at IS NULL THEN
            RAISE EXCEPTION
                'capability_grants revoke requires an MFA-attested timestamp (revoked_with_mfa_at must be set)';
        END IF;
        IF NEW.revoked_with_mfa_at < NOW() - mfa_window THEN
            RAISE EXCEPTION
                'capability_grants revoke MFA timestamp (%) is older than the % window',
                NEW.revoked_with_mfa_at, mfa_window;
        END IF;
        IF NEW.revoked_with_mfa_at > NOW() + INTERVAL '1 minute' THEN
            RAISE EXCEPTION
                'capability_grants revoke MFA timestamp (%) is in the future', NEW.revoked_with_mfa_at;
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION enforce_capability_revoke_mfa() IS
    'Phase 2 (E3) hardening: rejects UPDATEs that revoke a capability grant '
    'unless the same statement also writes a fresh `revoked_with_mfa_at` '
    'timestamp (within 5 minutes of NOW()). Defense-in-depth alongside '
    'AuthPolicyEnforcer::check_capability_revoke (D2).';

DROP TRIGGER IF EXISTS capability_grants_revoke_mfa_check ON capability_grants;
CREATE TRIGGER capability_grants_revoke_mfa_check
BEFORE UPDATE ON capability_grants
FOR EACH ROW
WHEN (NEW.revoked_at IS DISTINCT FROM OLD.revoked_at)
EXECUTE FUNCTION enforce_capability_revoke_mfa();
