-- Migration: 00137_create_tenant_settings.sql
-- Phase 5: Super-admin Control Plane
--
-- Per-tenant key/value settings store for the admin console. Backs
-- `SettingsStore` in `admin-core`. All writes go through `SettingsStore::set`,
-- which records an audit row alongside the update.
--
-- Schema choice notes:
--   * `value JSONB NOT NULL` — settings are arbitrarily-typed JSON blobs;
--     the trait layer carries the typed view.
--   * Composite PK on `(organization_id, key)` — exactly one row per setting
--     per tenant.
--   * RLS-enabled. Tenant members read only their org's rows; super-admins
--     bypass.
--
-- Phase 5 owns this migration. Phase 3 owns `tenant_config` (00133-ish) which
-- is a different shape (read-mostly cached config). Settings here are
-- operator-controlled (via the admin console), config there is tenant-owned.

CREATE TABLE IF NOT EXISTS tenant_settings (
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    key VARCHAR(128) NOT NULL,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- `updated_by` references users(id); `ON DELETE SET NULL` so a deleted
    -- operator does not cascade-delete their setting writes.
    updated_by UUID REFERENCES users(id) ON DELETE SET NULL,
    PRIMARY KEY (organization_id, key)
);

CREATE INDEX IF NOT EXISTS idx_tenant_settings_key ON tenant_settings(key);
CREATE INDEX IF NOT EXISTS idx_tenant_settings_updated_at ON tenant_settings(updated_at DESC);

-- Auto-update updated_at on UPDATE.
CREATE TRIGGER trg_tenant_settings_updated_at
    BEFORE UPDATE ON tenant_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- RLS.
ALTER TABLE tenant_settings ENABLE ROW LEVEL SECURITY;

-- Super-admin (platform principal) sees all rows.
CREATE POLICY tenant_settings_super_admin ON tenant_settings
    FOR ALL
    USING (is_super_admin());

-- Tenant members see only their own org's rows. Writes still go via the
-- application layer (SettingsStore) which gates on capabilities; the policy
-- only prevents cross-tenant reads.
CREATE POLICY tenant_settings_tenant_read ON tenant_settings
    FOR SELECT
    USING (organization_id = get_current_org_id());

COMMENT ON TABLE tenant_settings IS
    'Phase 5: per-tenant key/value settings managed by the admin console. All writes audited.';
COMMENT ON COLUMN tenant_settings.value IS
    'Arbitrary JSON value; typed view lives in the application layer (admin-core::SettingsStore).';
COMMENT ON COLUMN tenant_settings.updated_by IS
    'User ID of the operator who last wrote this setting (null after operator deletion).';
