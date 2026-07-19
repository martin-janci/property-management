-- Layout & Content Manager control plane (spec: docs/superpowers/specs/2026-07-19-layout-content-manager-design.md §7)
--
-- layout_configs / layout_config_versions / layout_kill_flags / layout_registry_manifests
-- are platform-admin-owned GLOBAL tables: no RLS by design (precedent: feature_flags,
-- migration 00030). Access control is enforced at the application layer (superadmin
-- routes); the resolved read path serves them to all authenticated users.
--
-- layout_tenant_overrides is org-scoped: ENABLE + FORCE RLS with the standard
-- tenant-isolation policy.

CREATE TABLE IF NOT EXISTS layout_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    screen TEXT NOT NULL UNIQUE,
    draft JSONB NOT NULL DEFAULT '{}'::jsonb,
    published JSONB,
    published_version INTEGER NOT NULL DEFAULT 0,
    rails JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS layout_config_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    screen TEXT NOT NULL,
    version INTEGER NOT NULL,
    config JSONB NOT NULL,
    published_by UUID,
    published_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (screen, version)
);

CREATE INDEX IF NOT EXISTS idx_layout_config_versions_screen
    ON layout_config_versions (screen, version DESC);

CREATE TABLE IF NOT EXISTS layout_kill_flags (
    screen TEXT NOT NULL,
    section_type TEXT NOT NULL,
    killed_by UUID,
    killed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (screen, section_type)
);

CREATE TABLE IF NOT EXISTS layout_registry_manifests (
    platform TEXT PRIMARY KEY,
    manifest JSONB NOT NULL,
    updated_by UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS layout_tenant_overrides (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    screen TEXT NOT NULL,
    override_config JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_by UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, screen)
);

ALTER TABLE layout_tenant_overrides ENABLE ROW LEVEL SECURITY;
ALTER TABLE layout_tenant_overrides FORCE ROW LEVEL SECURITY;
CREATE POLICY layout_tenant_overrides_tenant_isolation ON layout_tenant_overrides
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());
