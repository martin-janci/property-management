/**
 * API Client for Property Management System
 *
 * Generated from OpenAPI specification.
 * Run `pnpm generate` to regenerate after API changes.
 */

// Export domain-specific modules
export * from './admin';
export * from './advanced-notifications';
export * from './announcements';
export type { AuthUser } from './auth';
// Export auth token provider for secure token management
export * from './auth';
export * from './budgets';
export * from './buildings';
// Re-export the generated client through `./client-default`, which normalizes
// the generator's compiled-in absolute base URL (`http://localhost:8080`) to a
// safe relative/same-origin default at package load — so consumers that don't
// call `client.setConfig()` (e.g. admin-web) never hit an absolute dev host.
export { client } from './client-default';
export * from './community';
export * from './compliance';
export * from './critical-notifications';
export * from './disputes';
export * from './documents';
export * from './ecosystem';
export * from './emergency';
export * from './esignature';
export * from './facilities';
export * from './faults';
export * from './financial';
// Resolve the name collision between the analytics reports module (Epic 81) and
// the financial statement reports module (Story 11.7), which both export
// `ReportType` and `exportReport`. The financial Story 11.7 symbols are the
// canonical barrel names (actively consumed by the financial Reports screen);
// an explicit re-export takes precedence over the two ambiguous `export *`s.
// The Epic 81 analytics counterparts are preserved here under distinct names so
// nothing is silently dropped from the public surface.
export { exportReport, type ReportType } from './financial';
export * from './forms';
// Export generated types and client
// These will be populated after running `pnpm generate`
export * from './generated';
export * from './government-portal';
export * from './granular-notifications';
export * from './integrations';
export * from './iot';
export * from './layout';
export * from './leases';
// Shared authenticated-fetch helpers. `ApiError` carries HTTP `status` so callers
// can branch on 401/403; `authenticatedFetch` is the raw-`Response` primitive for
// callers that can't use the generated SDK — multipart uploads and endpoints
// whose callers read the `Response` themselves (see `./lib/fetch`).
// `authenticatedFetchJson` is the relative-path JSON sibling the per-module
// `Api*` classes use internally (centralized Authorization injection + MFA
// challenge retry + `ApiError` normalization); exported so ad-hoc app call
// sites (e.g. admin-web hooks) stop hand-rolling `fetch(..., { Authorization })`.
export { ApiError, authenticatedFetch, authenticatedFetchJson } from './lib/fetch';
export * from './members';
export * from './messaging';
export * from './meters';
export * from './mfa';
export * from './migration';
export * from './my-units';
export * from './neighbors';
export * from './news';
export * from './notification-preferences';
export * from './oauth-consent';
export * from './oauth-grants';
export * from './onboarding';
export * from './outages';
export * from './packages';
export * from './person-months';
export * from './portfolio-performance';
export * from './registry';
export * from './rentals';
export * from './reports';
export {
  exportReport as exportAnalyticsReport,
  type ReportType as AnalyticsReportType,
} from './reports';
export * from './syndication';
export * from './templates';
export * from './voting';
export * from './workflow-automation';

// API client configuration
export interface ApiConfig {
  baseUrl: string;
  accessToken?: string;
  tenantId?: string;
}

// Create configured API client
export function createApiClient(config: ApiConfig) {
  return {
    baseUrl: config.baseUrl,
    headers: {
      ...(config.accessToken && { Authorization: `Bearer ${config.accessToken}` }),
      ...(config.tenantId && { 'X-Tenant-ID': config.tenantId }),
    },
  };
}
