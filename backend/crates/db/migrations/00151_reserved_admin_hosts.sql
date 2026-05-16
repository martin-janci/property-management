-- 00151 — Reserve admin.rlt.sk hosts at the schema level.
-- (Originally numbered 00147; Phase 6 PR #265 landed first and used 147+148,
--  so this file is renumbered to 00151 — the next free slot after 00150.)
--
-- Phase 5 of the multitenancy roadmap moved super-admin into a dedicated
-- frontend app served at admin.rlt.sk (staging: admin.staging.rlt.sk).
--
-- What `reserved_platform_hosts` actually does:
--   - The `agency_domains` CHECK constraint
--     (`agency_domains_host_not_reserved`) calls
--     `is_reserved_platform_host()` and refuses any INSERT where an agency
--     tries to claim one of these hosts. So adding `admin.rlt.sk` here
--     prevents a future agency-onboarding bug from letting a tenant point
--     their custom domain at the admin host.
--
-- What this table does NOT do:
--   - The api-server `host_tenant_middleware` (api-core) does NOT consult
--     this table when resolving the request's Host header. Platform-host
--     recognition is driven by the `PLATFORM_HOST` env var (comma-separated
--     list); see `crates/api-core/src/middleware/host_tenant.rs` →
--     `load_platform_hosts`. For admin.rlt.sk to be resolved as
--     `TenantSource::PlatformHost` at runtime, the deploy-server injects
--     `admin.<reality_apex>` into that env var on the api-server +
--     reality-server containers — see `BlueGreenSpec::build_service_envs`.
--
-- See:
--   * docs/superpowers/specs/2026-05-16-admin-web-separate-app-design.md
--   * backend/crates/api-core/src/middleware/host_tenant.rs
--   * backend/servers/deploy-server/src/infra/blue_green.rs
--   * 00135_listings_publish_state.sql (introduces reserved_platform_hosts)
--
-- Idempotent: ON CONFLICT keeps re-applies safe. Re-applying yields
-- INSERT 0 0 once the rows exist.

INSERT INTO reserved_platform_hosts (host, reason) VALUES
  ('admin.rlt.sk',         'super-admin control plane (prod)'),
  ('admin.staging.rlt.sk', 'super-admin control plane (staging)')
ON CONFLICT (host) DO NOTHING;
