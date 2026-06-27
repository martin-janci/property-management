-- Epic 72: Regional Legal Compliance (SK/CZ)
-- Migration: 00196_create_regional_compliance.sql

-- Custom enum types
DO $$ BEGIN
    CREATE TYPE jurisdiction AS ENUM ('slovakia', 'czechia');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- ============================================================================
-- Per-organisation jurisdiction config
-- ============================================================================
CREATE TABLE IF NOT EXISTS regional_compliance_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    jurisdiction jurisdiction NOT NULL DEFAULT 'slovakia',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id)
);

-- ============================================================================
-- Static jurisdiction rules (seeded below)
-- ============================================================================
CREATE TABLE IF NOT EXISTS jurisdiction_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    jurisdiction jurisdiction NOT NULL,
    decision_type VARCHAR(100) NOT NULL,
    required_quorum_percentage NUMERIC(7,4) NOT NULL,
    legal_reference TEXT NOT NULL,
    UNIQUE (jurisdiction, decision_type)
);

-- Seed SK rules (zakon 182/1993 Z.z.)
INSERT INTO jurisdiction_rules (jurisdiction, decision_type, required_quorum_percentage, legal_reference)
VALUES
    ('slovakia', 'simple_majority',         50.01,  'SS 14 ods. 1 zakona 182/1993 Z.z.'),
    ('slovakia', 'two_thirds_majority',     66.67,  'SS 14 ods. 2 zakona 182/1993 Z.z.'),
    ('slovakia', 'three_quarters_majority', 75.00,  'SS 14 ods. 3 zakona 182/1993 Z.z.'),
    ('slovakia', 'unanimous',              100.00,  'SS 14 ods. 4 zakona 182/1993 Z.z.'),
    ('czechia',  'simple_majority',         50.01,  'SS 1206 zakona 89/2012 Sb.'),
    ('czechia',  'qualified_majority',      66.67,  'SS 1208 zakona 89/2012 Sb.'),
    ('czechia',  'three_quarters_majority', 75.00,  'SS 1210 zakona 89/2012 Sb.'),
    ('czechia',  'all_owners',             100.00,  'SS 1214 zakona 89/2012 Sb.')
ON CONFLICT (jurisdiction, decision_type) DO NOTHING;

-- ============================================================================
-- Slovak voting config (per building)
-- ============================================================================
CREATE TABLE IF NOT EXISTS slovak_voting_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    building_id UUID NOT NULL REFERENCES buildings(id) ON DELETE CASCADE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    default_decision_type VARCHAR(100) NOT NULL DEFAULT 'simple_majority',
    use_ownership_weight BOOLEAN NOT NULL DEFAULT TRUE,
    min_notice_days INTEGER NOT NULL DEFAULT 15,
    allow_proxy_voting BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id, building_id)
);

-- ============================================================================
-- Slovak accounting config (per organisation)
-- ============================================================================
CREATE TABLE IF NOT EXISTS slovak_accounting_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    export_format VARCHAR(50) NOT NULL DEFAULT 'pohoda_xml',
    ico VARCHAR(20),
    dic VARCHAR(20),
    ic_dph VARCHAR(20),
    default_iban VARCHAR(34),
    account_mapping JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id)
);

-- ============================================================================
-- Slovak GDPR config (per organisation)
-- ============================================================================
CREATE TABLE IF NOT EXISTS slovak_gdpr_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    dpo_name VARCHAR(255) NOT NULL,
    dpo_email VARCHAR(255) NOT NULL,
    dpo_phone VARCHAR(50),
    org_address TEXT,
    processing_purposes JSONB NOT NULL DEFAULT '[]',
    consent_texts JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id)
);

-- ============================================================================
-- Slovak GDPR consents (per user × category)
-- ============================================================================
CREATE TABLE IF NOT EXISTS slovak_gdpr_consents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    category VARCHAR(100) NOT NULL,
    granted BOOLEAN NOT NULL,
    ip_address VARCHAR(45),
    user_agent TEXT,
    consent_version VARCHAR(50) NOT NULL,
    consented_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    withdrawn_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_slovak_gdpr_consents_user ON slovak_gdpr_consents (user_id, category);
CREATE INDEX IF NOT EXISTS idx_slovak_gdpr_consents_org  ON slovak_gdpr_consents (organization_id);

-- ============================================================================
-- Czech SVJ config (per building)
-- ============================================================================
CREATE TABLE IF NOT EXISTS czech_svj_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    building_id UUID NOT NULL REFERENCES buildings(id) ON DELETE CASCADE,
    org_type VARCHAR(50) NOT NULL DEFAULT 'svj',
    ico VARCHAR(20) NOT NULL,
    has_stanovy BOOLEAN NOT NULL DEFAULT FALSE,
    stanovy_document_id UUID,
    stanovy_effective_date DATE,
    default_decision_type VARCHAR(100) NOT NULL DEFAULT 'simple_majority',
    use_ownership_weight BOOLEAN NOT NULL DEFAULT TRUE,
    notary_threshold_czk NUMERIC(15,2),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (organization_id, building_id)
);

-- ============================================================================
-- Enable RLS on all tenant-scoped tables
-- ============================================================================
ALTER TABLE regional_compliance_configs ENABLE ROW LEVEL SECURITY;
ALTER TABLE regional_compliance_configs FORCE ROW LEVEL SECURITY;

ALTER TABLE slovak_voting_configs ENABLE ROW LEVEL SECURITY;
ALTER TABLE slovak_voting_configs FORCE ROW LEVEL SECURITY;

ALTER TABLE slovak_accounting_configs ENABLE ROW LEVEL SECURITY;
ALTER TABLE slovak_accounting_configs FORCE ROW LEVEL SECURITY;

ALTER TABLE slovak_gdpr_configs ENABLE ROW LEVEL SECURITY;
ALTER TABLE slovak_gdpr_configs FORCE ROW LEVEL SECURITY;

ALTER TABLE slovak_gdpr_consents ENABLE ROW LEVEL SECURITY;
ALTER TABLE slovak_gdpr_consents FORCE ROW LEVEL SECURITY;

ALTER TABLE czech_svj_configs ENABLE ROW LEVEL SECURITY;
ALTER TABLE czech_svj_configs FORCE ROW LEVEL SECURITY;

-- ============================================================================
-- RLS policies: tenant isolation on all org-scoped tables
-- ============================================================================
CREATE POLICY regional_compliance_configs_tenant_isolation ON regional_compliance_configs
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

CREATE POLICY slovak_voting_configs_tenant_isolation ON slovak_voting_configs
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

CREATE POLICY slovak_accounting_configs_tenant_isolation ON slovak_accounting_configs
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

CREATE POLICY slovak_gdpr_configs_tenant_isolation ON slovak_gdpr_configs
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

CREATE POLICY slovak_gdpr_consents_tenant_isolation ON slovak_gdpr_consents
    FOR ALL
    USING (is_super_admin() OR organization_id IS NULL OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id IS NULL OR organization_id = get_current_org_id());

CREATE POLICY czech_svj_configs_tenant_isolation ON czech_svj_configs
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());
