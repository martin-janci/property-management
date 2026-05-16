-- Phase 1: Tenant Resolution Keystone
-- Maps an inbound Host header (subdomain OR full custom domain) to exactly one
-- organization (the RLS tenant). The host-resolution middleware queries this
-- table BEFORE any tenant context is set, on a system (non-RLS) connection.

CREATE TABLE agency_domains (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    host            VARCHAR(253) NOT NULL,
    kind            VARCHAR(20)  NOT NULL DEFAULT 'subdomain'
                        CHECK (kind IN ('subdomain', 'custom')),
    verification_state VARCHAR(20) NOT NULL DEFAULT 'pending'
                        CHECK (verification_state IN ('pending', 'verifying', 'verified', 'failed')),
    verification_token VARCHAR(64),
    verified_at        TIMESTAMPTZ,
    cert_enabled    BOOLEAN NOT NULL DEFAULT TRUE,
    is_primary      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX idx_agency_domains_host ON agency_domains (host);
CREATE INDEX idx_agency_domains_org ON agency_domains (organization_id);
CREATE UNIQUE INDEX idx_agency_domains_one_primary
    ON agency_domains (organization_id) WHERE is_primary;
CREATE TRIGGER update_agency_domains_updated_at
    BEFORE UPDATE ON agency_domains
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE agency_domains ENABLE ROW LEVEL SECURITY;
ALTER TABLE agency_domains FORCE ROW LEVEL SECURITY;
CREATE POLICY agency_domains_tenant_isolation ON agency_domains
    FOR ALL
    USING      (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());
