-- Phase 2: Identity Unification — user_memberships table.
--
-- The new authorization spine: a unified per-(user, organization, role) join
-- with grant/revoke/expiry semantics, enabling the per-request authz model
-- demanded by leak #10/#11 (token never carries authority — it always re-derives
-- from `user_memberships ∩ ResolvedTenant.organization_id`).
--
-- This runs SIDE-BY-SIDE with the legacy `organization_members` for one phase.
-- The legacy table is NOT dropped here. Callers that need the new contract
-- migrate first; the legacy table is retired in a later phase.

CREATE TABLE IF NOT EXISTS user_memberships (
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    role            VARCHAR(64) NOT NULL,
    granted_by      UUID REFERENCES users(id) ON DELETE SET NULL,
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NULL,
    revoked_at      TIMESTAMPTZ NULL,
    PRIMARY KEY (user_id, organization_id, role)
);

-- Active-membership lookups (the hot path for `RequestPrincipal`):
-- one row per (user, org) means an active grant exists. Filtered partial index
-- keeps it small and fast.
CREATE INDEX IF NOT EXISTS idx_user_memberships_active
    ON user_memberships (user_id, organization_id)
    WHERE revoked_at IS NULL
      AND (expires_at IS NULL OR expires_at > NOW());

CREATE INDEX IF NOT EXISTS idx_user_memberships_org
    ON user_memberships (organization_id);

-- RLS — the standard tenant policy. Super-admin sees all; everyone else only
-- sees their own org's grants. Insert/Update/Delete go through the
-- capability-gated admin endpoints (see Phase 5); the policy guards readback.
ALTER TABLE user_memberships ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_memberships FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS user_memberships_tenant_isolation ON user_memberships;
CREATE POLICY user_memberships_tenant_isolation ON user_memberships
    FOR ALL
    USING      (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- Backfill from `organization_members`. Map legacy role_type → unified role
-- string; collapse multi-row legacy rows to a single (user, org, role) row.
-- This is idempotent — the PRIMARY KEY suppresses repeated runs.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'public' AND table_name = 'organization_members'
    ) THEN
        EXECUTE $sql$
            INSERT INTO user_memberships (
                user_id, organization_id, role, granted_by, granted_at,
                expires_at, revoked_at
            )
            SELECT om.user_id,
                   om.organization_id,
                   COALESCE(NULLIF(om.role_type, ''), 'member')           AS role,
                   om.invited_by                                          AS granted_by,
                   COALESCE(om.joined_at, om.created_at, NOW())           AS granted_at,
                   NULL::TIMESTAMPTZ                                      AS expires_at,
                   CASE WHEN om.status = 'removed' THEN COALESCE(om.updated_at, NOW())
                        ELSE NULL END                                     AS revoked_at
              FROM organization_members om
             WHERE om.user_id IS NOT NULL
               AND om.organization_id IS NOT NULL
            ON CONFLICT (user_id, organization_id, role) DO NOTHING
        $sql$;
    END IF;
END $$;

COMMENT ON TABLE user_memberships IS
    'Phase 2 authz spine: per-request authority is derived from active rows here intersected with the host-resolved organization. Defends leaks #9, #10, #11.';
