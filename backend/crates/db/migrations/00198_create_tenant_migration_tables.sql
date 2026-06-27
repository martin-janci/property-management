-- Epic 66: Platform Migration & Data Import Backing Tables.
--
-- Create backing tables for import templates, import jobs, import row errors,
-- and migration exports with Row Level Security (RLS) policies.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'import_data_type') THEN
        CREATE TYPE import_data_type AS ENUM (
            'buildings',
            'units',
            'residents',
            'financials',
            'faults',
            'documents',
            'meters',
            'votes',
            'custom'
        );
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'import_job_status') THEN
        CREATE TYPE import_job_status AS ENUM (
            'pending',
            'validating',
            'validated',
            'validation_failed',
            'importing',
            'completed',
            'partially_completed',
            'failed',
            'cancelled'
        );
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'migration_export_status') THEN
        CREATE TYPE migration_export_status AS ENUM (
            'pending',
            'processing',
            'ready',
            'downloaded',
            'expired',
            'failed'
        );
    END IF;
END$$;

-- 1. Import Templates Table
CREATE TABLE import_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    data_type import_data_type NOT NULL,
    field_mappings JSONB NOT NULL,
    is_system_template BOOLEAN NOT NULL DEFAULT false,
    version INT NOT NULL DEFAULT 1,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_import_templates_organization_id ON import_templates(organization_id);

-- 2. Import Jobs Table
CREATE TABLE import_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    template_id UUID NOT NULL REFERENCES import_templates(id) ON DELETE CASCADE,
    status import_job_status NOT NULL DEFAULT 'pending',
    original_filename VARCHAR(255) NOT NULL,
    file_path VARCHAR(1000) NOT NULL,
    file_size_bytes BIGINT NOT NULL,
    total_rows INT,
    processed_rows INT NOT NULL DEFAULT 0,
    successful_rows INT NOT NULL DEFAULT 0,
    failed_rows INT NOT NULL DEFAULT 0,
    skipped_rows INT NOT NULL DEFAULT 0,
    validation_errors JSONB,
    import_errors JSONB,
    options JSONB,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_import_jobs_organization_id ON import_jobs(organization_id);
CREATE INDEX idx_import_jobs_template_id ON import_jobs(template_id);

-- 3. Import Row Errors Table
CREATE TABLE import_row_errors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    row_number INT NOT NULL,
    "column" VARCHAR(255),
    message TEXT NOT NULL,
    error_code VARCHAR(100) NOT NULL,
    original_value TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_import_row_errors_job_id ON import_row_errors(job_id);
CREATE INDEX idx_import_row_errors_organization_id ON import_row_errors(organization_id);

-- 4. Migration Exports Table
CREATE TABLE migration_exports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    status migration_export_status NOT NULL DEFAULT 'pending',
    categories JSONB NOT NULL,
    privacy_options JSONB NOT NULL,
    output_format VARCHAR(50) NOT NULL DEFAULT 'zip',
    file_path VARCHAR(1000),
    file_size_bytes BIGINT,
    file_hash VARCHAR(64),
    download_token UUID,
    download_count INT NOT NULL DEFAULT 0,
    downloaded_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_migration_exports_organization_id ON migration_exports(organization_id);

-- ============================================
-- Row Level Security (RLS) isolation
-- ============================================

ALTER TABLE import_templates ENABLE ROW LEVEL SECURITY;
ALTER TABLE import_templates FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS import_templates_tenant_isolation ON import_templates;
CREATE POLICY import_templates_tenant_isolation ON import_templates
    FOR ALL
    USING (is_super_admin() OR organization_id IS NULL OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id IS NULL OR organization_id = get_current_org_id());

ALTER TABLE import_jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE import_jobs FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS import_jobs_tenant_isolation ON import_jobs;
CREATE POLICY import_jobs_tenant_isolation ON import_jobs
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

ALTER TABLE import_row_errors ENABLE ROW LEVEL SECURITY;
ALTER TABLE import_row_errors FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS import_row_errors_tenant_isolation ON import_row_errors;
CREATE POLICY import_row_errors_tenant_isolation ON import_row_errors
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

ALTER TABLE migration_exports ENABLE ROW LEVEL SECURITY;
ALTER TABLE migration_exports FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS migration_exports_tenant_isolation ON migration_exports;
CREATE POLICY migration_exports_tenant_isolation ON migration_exports
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- Seed System Templates
-- ============================================

INSERT INTO import_templates (id, organization_id, name, description, data_type, field_mappings, is_system_template, version, is_active, created_by)
VALUES
('00000000-0000-0000-0000-000000000001', NULL, 'Buildings Import', 'Import building master data', 'buildings', '[]', true, 1, true, NULL),
('00000000-0000-0000-0000-000000000002', NULL, 'Units Import', 'Import unit data', 'units', '[]', true, 1, true, NULL),
('00000000-0000-0000-0000-000000000003', NULL, 'Residents Import', 'Import resident data', 'residents', '[]', true, 1, true, NULL)
ON CONFLICT (id) DO NOTHING;
