-- Migration: 00229_create_platform_settings.sql
--
-- Global, operator-controlled platform settings backing the admin-web
-- `/admin/platform` page (`PATCH /api/v1/platform-admin/settings`).
--
-- Unlike `tenant_settings` (00137), which is per-organization and RLS-isolated,
-- these settings are platform-wide: a single row shared across the whole
-- deployment. Access is gated entirely at the application layer by the
-- platform-admin capability middleware (`RequireCapability(SiteSettingsWrite)`
-- + a validated super-admin token), so this table follows the same no-RLS
-- pattern as the other platform-operator tables (e.g. health monitoring,
-- 00074): there is no per-tenant row to isolate.

CREATE TABLE IF NOT EXISTS platform_settings (
    -- Singleton guard: exactly one row, always id = 1.
    id INTEGER PRIMARY KEY DEFAULT 1,
    maintenance_mode BOOLEAN NOT NULL DEFAULT false,
    signup_enabled BOOLEAN NOT NULL DEFAULT true,
    support_email TEXT NOT NULL DEFAULT '',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- `updated_by` references users(id); `ON DELETE SET NULL` so a removed
    -- operator does not cascade-delete the settings row.
    updated_by UUID REFERENCES users(id) ON DELETE SET NULL,
    CONSTRAINT platform_settings_singleton CHECK (id = 1)
);

-- Seed the singleton row so reads always return defaults before the first write.
INSERT INTO platform_settings (id) VALUES (1) ON CONFLICT (id) DO NOTHING;

COMMENT ON TABLE platform_settings IS
    'Global operator-controlled platform settings (singleton row). Written via PATCH /api/v1/platform-admin/settings.';
