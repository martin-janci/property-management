-- Phase 3: Hosting & Theming.
-- Extend `agency_branding` (00108) with the columns the runtime-theming
-- pipeline needs:
--   * `font_family`     — CSS font-family stack (e.g. "Inter, sans-serif").
--   * `css_vars`        — JSONB blob of arbitrary `--ppt-*` design-token
--                         overrides; serialized into the HTML root style attr
--                         by the reality-web layout.
--   * `organization_id` — link to `organizations` (the canonical Phase 1 RLS
--                         tenant, separate from the legacy `reality_agencies`
--                         model `agency_branding` originally hung off).
--                         Nullable so existing rows don't break; the Phase 3
--                         /tenant-config endpoint reads branding via this
--                         column.
--
-- 00108's `secondary_color TEXT` already exists; we don't redeclare it.
--
-- RLS: 00108 did NOT add an RLS policy. Tenant isolation for branding is
-- enforced here, scoped by `organization_id` (matching the standard pattern
-- from Phase 0). Rows with NULL `organization_id` are visible to super-admin
-- only — that's fine for the legacy `agency_id`-keyed rows that pre-date
-- this column.

ALTER TABLE agency_branding
    ADD COLUMN IF NOT EXISTS font_family     VARCHAR(100),
    ADD COLUMN IF NOT EXISTS css_vars        JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

-- One branding row per organization. Partial-unique so legacy rows with
-- NULL organization_id (still keyed by agency_id) don't conflict with each
-- other.
CREATE UNIQUE INDEX IF NOT EXISTS idx_agency_branding_org
    ON agency_branding(organization_id)
    WHERE organization_id IS NOT NULL;

-- Make agency_id nullable so rows can be created keyed only by organization_id.
-- Existing rows keep their agency_id; new tenant-created rows omit it.
ALTER TABLE agency_branding ALTER COLUMN agency_id DROP NOT NULL;

ALTER TABLE agency_branding ENABLE ROW LEVEL SECURITY;
ALTER TABLE agency_branding FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS agency_branding_tenant_isolation ON agency_branding;
CREATE POLICY agency_branding_tenant_isolation ON agency_branding
    FOR ALL
    USING (
        is_super_admin()
        OR (organization_id IS NOT NULL AND organization_id = get_current_org_id())
    )
    WITH CHECK (
        is_super_admin()
        OR (organization_id IS NOT NULL AND organization_id = get_current_org_id())
    );
