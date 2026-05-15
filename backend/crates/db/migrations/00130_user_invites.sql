-- Phase 2: Identity Unification — user_invites table.
--
-- Defends leak #9 ("membership row injection — unguarded path creates a
-- membership → join any org"). Every membership grant goes through an invite
-- that is single-use, expiring, and email-bound. A token is stored only as a
-- SHA-256 hash; the plaintext is shown ONCE at creation time.

CREATE TABLE IF NOT EXISTS user_invites (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           VARCHAR(320) NOT NULL,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    role            VARCHAR(64) NOT NULL,
    invited_by      UUID REFERENCES users(id) ON DELETE SET NULL,
    token_hash      VARCHAR(128) NOT NULL UNIQUE,
    expires_at      TIMESTAMPTZ NOT NULL,
    accepted_at     TIMESTAMPTZ NULL,
    accepted_by     UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_invites_email ON user_invites (LOWER(email));
CREATE INDEX IF NOT EXISTS idx_user_invites_org ON user_invites (organization_id);
CREATE INDEX IF NOT EXISTS idx_user_invites_pending
    ON user_invites (expires_at)
    WHERE accepted_at IS NULL;

ALTER TABLE user_invites ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_invites FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS user_invites_tenant_isolation ON user_invites;
CREATE POLICY user_invites_tenant_isolation ON user_invites
    FOR ALL
    USING      (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

COMMENT ON TABLE user_invites IS
    'Phase 2: single-use, expiring, email-bound invites. The only sanctioned path to create a user_memberships row. Token stored as SHA-256 hash. Defends leak #9.';
