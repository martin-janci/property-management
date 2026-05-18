-- Phase 3: Per-tenant feature flags + kill switches.
--
-- Defends against operational leak #22 from the Phase 0 brainstorming:
-- "no per-tenant kill switch" — without this we can't disable a single
-- agency's feature without a full deploy. The reality-web layout consumes
-- these via /tenant-config and special-cases `building_disabled` (when
-- enabled, the tenant gets a 503 page).
--
-- Distinct from the existing global `feature_flags` (00030) — that table
-- gates platform-wide rollouts; this one gates a feature for a single
-- organization.
--
-- Schema:
--   organization_id, flag_key  → composite PK (one row per org × flag).
--   enabled                    → simple on/off switch.
--   value                      → optional JSON payload for typed flag values
--                                (rollout percentage, variant string, etc).
--
-- RLS: standard tenant-isolation pattern.

CREATE TABLE IF NOT EXISTS tenant_feature_flags (
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    flag_key        VARCHAR(100) NOT NULL,
    enabled         BOOLEAN NOT NULL DEFAULT FALSE,
    value           JSONB,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by      UUID,
    PRIMARY KEY (organization_id, flag_key)
);

CREATE INDEX IF NOT EXISTS idx_tenant_feature_flags_org
    ON tenant_feature_flags(organization_id);

CREATE TRIGGER update_tenant_feature_flags_updated_at
    BEFORE UPDATE ON tenant_feature_flags
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

ALTER TABLE tenant_feature_flags ENABLE ROW LEVEL SECURITY;
ALTER TABLE tenant_feature_flags FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_feature_flags_isolation ON tenant_feature_flags
    FOR ALL
    USING      (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

COMMENT ON TABLE tenant_feature_flags IS
    'Per-tenant feature flags + kill switches (Phase 3, defends leak #22). '
    'Distinct from the global feature_flags table (00030).';
COMMENT ON COLUMN tenant_feature_flags.flag_key IS
    'Stable string key. Reserved keys consumed by the frontend: '
    '"building_disabled" (renders 503 instead of normal app).';

-- Seed `building_disabled` (default off) for every existing organization.
-- Future organizations get the row lazily on first upsert via the admin
-- endpoint — they don't need a row to read the default-off behavior, but
-- having one for existing tenants makes the kill switch immediately
-- discoverable in the admin UI.
INSERT INTO tenant_feature_flags (organization_id, flag_key, enabled)
SELECT id, 'building_disabled', FALSE
FROM organizations
WHERE status != 'deleted'
ON CONFLICT (organization_id, flag_key) DO NOTHING;
