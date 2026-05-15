-- Phase 2 — Defense N2: per-organization authentication policy.
--
-- Defends leak #13 from the Phase 0 brainstorming: "policy resolved under
-- wrong tenant." Without per-org policy, password / MFA / email-verification
-- rules are evaluated against a global default — meaning a strict-policy org
-- (e.g. "MFA required for managers") cannot enforce its rules at the moment
-- a privilege change happens, and a permissive override leaks across tenants.
--
-- Schema:
--   organization_id  → composite PK (one row per org).
--   policy           → JSONB blob deserialized into the Rust `AuthPolicy`
--                      struct (db::models::auth_policy::AuthPolicy). Partial
--                      shapes are valid; missing keys fall back to defaults.
--
-- The Rust loader (`AuthPolicy::for_org`) already handles the "no row →
-- default" case, so seeding is NOT required at migration time. Orgs that
-- want overrides INSERT lazily via the admin endpoint surface (Phase 5).
--
-- RLS: standard tenant-isolation pattern — an org can read/write only its
-- own row. Super-admin bypass for the loader, which reads as platform
-- authority (the policy is policy ABOUT the tenant, not OF the tenant).

CREATE TABLE IF NOT EXISTS org_auth_policies (
    organization_id UUID NOT NULL PRIMARY KEY
        REFERENCES organizations(id) ON DELETE CASCADE,
    policy          JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_by      UUID,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_org_auth_policies_updated_at
    ON org_auth_policies(updated_at DESC);

CREATE TRIGGER update_org_auth_policies_updated_at
    BEFORE UPDATE ON org_auth_policies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE org_auth_policies ENABLE ROW LEVEL SECURITY;
ALTER TABLE org_auth_policies FORCE ROW LEVEL SECURITY;

CREATE POLICY org_auth_policies_isolation ON org_auth_policies
    FOR ALL
    USING      (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

COMMENT ON TABLE org_auth_policies IS
    'Per-organization authentication policy (Defense N2 / Phase 2 leak #13). '
    'Effective policy = platform default merged with this row''s overrides. '
    'Loaded by db::models::auth_policy::AuthPolicy::for_org and enforced by '
    'AuthPolicyEnforcer at every privilege change.';

COMMENT ON COLUMN org_auth_policies.policy IS
    'JSONB shape mirrors db::models::auth_policy::AuthPolicy. Recognized keys: '
    'min_password_length, require_uppercase, require_digit, require_symbol, '
    'require_email_verification, mfa_required_for_roles, max_session_age_minutes. '
    'Missing keys fall back to the platform default.';
