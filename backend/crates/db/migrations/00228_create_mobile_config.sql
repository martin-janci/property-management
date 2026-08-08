-- Migration: 00228_create_mobile_config.sql
--
-- Backs the admin-console mobile-config Save flow
-- (`PATCH /api/v1/admin/mobile-config`). Stores platform-global mobile app
-- configuration:
--   * force-update version floors (`min_ios_version`, `min_android_version`)
--   * React Native feature flags (`flags` JSONB, e.g. `{"rn.new_onboarding": true}`)
--
-- Design: a SINGLE-ROW (singleton) platform-global table, mirroring the
-- operator-config pattern of `metric_thresholds` (00031) — no
-- `organization_id`, no RLS. This is a platform-operator surface; access is
-- gated at the application layer by `Capability::MobileConfigWrite` on the
-- route (the same model `metric_thresholds` uses). The singleton is enforced
-- by a boolean primary key pinned to TRUE.

CREATE TABLE IF NOT EXISTS mobile_config (
    -- Singleton guard: exactly one row (`id = TRUE`).
    id BOOLEAN PRIMARY KEY DEFAULT TRUE,
    min_ios_version VARCHAR(32),
    min_android_version VARCHAR(32),
    flags JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- `updated_by` references users(id); `ON DELETE SET NULL` so a deleted
    -- operator does not cascade-delete the config row.
    updated_by UUID REFERENCES users(id) ON DELETE SET NULL,
    CONSTRAINT mobile_config_singleton CHECK (id = TRUE)
);

-- Seed the singleton row so reads (and the merge-on-write path) always find a
-- row. The write path also upserts defensively via ON CONFLICT.
INSERT INTO mobile_config (id) VALUES (TRUE) ON CONFLICT (id) DO NOTHING;

COMMENT ON TABLE mobile_config IS
    'Platform-global mobile app config (force-update floors + RN feature flags). Singleton row; written via admin console PATCH /api/v1/admin/mobile-config, gated by the mobile_config_write capability.';
COMMENT ON COLUMN mobile_config.flags IS
    'React Native feature flags as a JSON object of flag-key -> value. Patched (merged) on write.';
COMMENT ON COLUMN mobile_config.updated_by IS
    'User ID of the operator who last wrote this config (null after operator deletion).';
