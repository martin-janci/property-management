-- 00147 — Reserve admin.rlt.sk hosts for the super-admin control plane.
--
-- Phase 5 of the multitenancy roadmap moved super-admin into a dedicated
-- frontend app served at admin.rlt.sk (staging: admin.staging.rlt.sk). The
-- api-server `host_tenant_middleware` (api-core) refuses any request whose
-- Host header is not recognised. Without these rows, every admin SPA call
-- would fail with `404 Unknown tenant host` before reaching the admin
-- router.
--
-- Resolution result: TenantSource::PlatformHost (nil UUID, global_read
-- context). The admin router does not consume the org id; it dispatches on
-- the principal's capabilities, never on tenant identity. PlatformHost is
-- the correct resolution.
--
-- See:
--   * docs/superpowers/specs/2026-05-16-admin-web-separate-app-design.md
--   * backend/crates/api-core/src/middleware/host_tenant.rs
--   * 00135_listings_publish_state.sql (introduces reserved_platform_hosts)
--
-- Idempotent: ON CONFLICT keeps re-applies safe. Re-applying yields
-- INSERT 0 0 once the rows exist.

INSERT INTO reserved_platform_hosts (host, reason) VALUES
  ('admin.rlt.sk',         'super-admin control plane (prod)'),
  ('admin.staging.rlt.sk', 'super-admin control plane (staging)')
ON CONFLICT (host) DO NOTHING;
