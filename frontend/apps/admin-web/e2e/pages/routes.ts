/**
 * admin-web route paths.
 *
 * admin-web is NOT in `@ppt/sitemap` (the sitemap only ships ppt-web /
 * reality-web route data), so the sitemap-driven page objects can't resolve
 * these for us. This module is the single source of truth for admin-web paths
 * — page objects reference it instead of hard-coding strings, so a route move
 * is a one-line change here.
 *
 * Paths mirror the `<Route>` table in `apps/admin-web/src/App.tsx`.
 */
export const ADMIN_ROUTES = {
  login: '/login',
  dashboard: '/',
  agencies: '/tenants/agencies',
  tenantLifecycle: '/tenants/lifecycle',
  users: '/identity/users',
  memberships: '/identity/memberships',
  capabilities: '/identity/capabilities',
  oauthClients: '/identity/oauth-clients',
  audit: '/ops/audit',
  impersonation: '/ops/impersonation',
  featureFlags: '/ops/feature-flags',
  platformSettings: '/platform/settings',
  mobileConfig: '/platform/mobile',
  platformHealth: '/platform/health',
  announcements: '/platform/announcements',
  supportData: '/platform/support-data',
  onboarding: '/platform/onboarding',
} as const;

export type AdminRouteKey = keyof typeof ADMIN_ROUTES;
