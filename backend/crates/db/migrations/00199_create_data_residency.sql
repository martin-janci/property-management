-- Migration 00195: Create Data Residency tables and seed reference regions (Epic 146).

-- Create custom enum types if they do not exist
CREATE TYPE data_region AS ENUM (
    'eu_west',
    'eu_central',
    'us_east',
    'us_west',
    'apac_southeast',
    'apac_south',
    'uk_south',
    'ch_north'
);

CREATE TYPE data_type_category AS ENUM (
    'personal_data',
    'financial_data',
    'documents',
    'audit_logs',
    'communications',
    'analytics',
    'all'
);

CREATE TYPE residency_status AS ENUM (
    'active',
    'migrating',
    'pending',
    'suspended'
);

CREATE TYPE access_type AS ENUM (
    'read',
    'write',
    'delete',
    'migration'
);

CREATE TYPE residency_audit_event AS ENUM (
    'configuration_created',
    'configuration_updated',
    'region_changed',
    'migration_started',
    'migration_completed',
    'compliance_check_performed',
    'cross_region_access',
    'override_added',
    'override_removed'
);

-- 1. Create residency_regions table
CREATE TABLE residency_regions (
    region data_region PRIMARY KEY,
    display_name VARCHAR(255) NOT NULL,
    location_code VARCHAR(255) NOT NULL,
    compliance_frameworks TEXT[] NOT NULL,
    available BOOLEAN NOT NULL DEFAULT TRUE
);

-- 2. Create data_residency_config table
CREATE TABLE data_residency_config (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL UNIQUE REFERENCES organizations(id) ON DELETE CASCADE,
    primary_region VARCHAR(50) NOT NULL,
    backup_region VARCHAR(50),
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    allow_cross_region_access BOOLEAN NOT NULL DEFAULT FALSE,
    data_type_overrides JSONB,
    compliance_notes TEXT,
    last_verified_at TIMESTAMPTZ,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 3. Create residency_audit_log table
CREATE TABLE residency_audit_log (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    event_type VARCHAR(100) NOT NULL,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    previous_state JSONB,
    new_state JSONB,
    details JSONB,
    ip_address VARCHAR(45),
    user_agent TEXT,
    record_hash VARCHAR(64) NOT NULL,
    previous_hash VARCHAR(64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 4. Create cross_region_access_log table
CREATE TABLE cross_region_access_log (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    data_type VARCHAR(100) NOT NULL,
    source_region VARCHAR(50) NOT NULL,
    access_region VARCHAR(50) NOT NULL,
    access_type VARCHAR(50) NOT NULL,
    resource_id VARCHAR(255),
    reason TEXT,
    ip_address VARCHAR(45),
    accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 5. Create data_routing_rule table
CREATE TABLE data_routing_rule (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    data_type VARCHAR(100) NOT NULL,
    write_region VARCHAR(50) NOT NULL,
    read_region VARCHAR(50) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 6. Create compliance_verification_result table
CREATE TABLE compliance_verification_result (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    is_compliant BOOLEAN NOT NULL,
    verified_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    data_locations JSONB NOT NULL,
    issues JSONB,
    details JSONB,
    verified_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE
);

-- Seed available reference regions (EU, CZ, SK)
INSERT INTO residency_regions (region, display_name, location_code, compliance_frameworks, available)
VALUES
    ('eu_west', 'European Union (Frankfurt)', 'eu-west-1', ARRAY['GDPR', 'EU Data Residency', 'Schrems II Compliant'], TRUE),
    ('eu_central', 'Czech Republic (Prague)', 'cz-prague', ARRAY['GDPR', 'Czech Act on Data Protection'], TRUE),
    ('ch_north', 'Slovakia (Bratislava)', 'sk-bratislava', ARRAY['GDPR', 'Slovak Act on Data Protection'], TRUE),
    ('us_east', 'US East (Virginia)', 'us-east-1', ARRAY['SOC 2', 'HIPAA Eligible', 'FedRAMP'], TRUE),
    ('us_west', 'US West (Oregon)', 'us-west-2', ARRAY['SOC 2', 'HIPAA Eligible', 'FedRAMP'], TRUE),
    ('apac_southeast', 'APAC Southeast (Singapore)', 'ap-southeast-1', ARRAY['PDPA (Singapore)'], TRUE),
    ('apac_south', 'APAC South (Sydney)', 'ap-south-1', ARRAY['Privacy Act (Australia)'], TRUE),
    ('uk_south', 'UK South (London)', 'uk-south-1', ARRAY['UK GDPR', 'Data Protection Act 2018'], TRUE);

-- Enable RLS
ALTER TABLE residency_regions ENABLE ROW LEVEL SECURITY;
ALTER TABLE residency_regions FORCE ROW LEVEL SECURITY;

ALTER TABLE data_residency_config ENABLE ROW LEVEL SECURITY;
ALTER TABLE data_residency_config FORCE ROW LEVEL SECURITY;

ALTER TABLE residency_audit_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE residency_audit_log FORCE ROW LEVEL SECURITY;

ALTER TABLE cross_region_access_log ENABLE ROW LEVEL SECURITY;
ALTER TABLE cross_region_access_log FORCE ROW LEVEL SECURITY;

ALTER TABLE data_routing_rule ENABLE ROW LEVEL SECURITY;
ALTER TABLE data_routing_rule FORCE ROW LEVEL SECURITY;

ALTER TABLE compliance_verification_result ENABLE ROW LEVEL SECURITY;
ALTER TABLE compliance_verification_result FORCE ROW LEVEL SECURITY;

-- Define RLS Policies
CREATE POLICY residency_regions_read ON residency_regions
    FOR SELECT USING (TRUE);

CREATE POLICY residency_regions_write ON residency_regions
    FOR ALL USING (is_super_admin()) WITH CHECK (is_super_admin());

CREATE POLICY data_residency_config_tenant_isolation ON data_residency_config
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

CREATE POLICY residency_audit_log_tenant_isolation ON residency_audit_log
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

CREATE POLICY cross_region_access_log_tenant_isolation ON cross_region_access_log
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

CREATE POLICY data_routing_rule_tenant_isolation ON data_routing_rule
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

CREATE POLICY compliance_verification_result_tenant_isolation ON compliance_verification_result
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());
