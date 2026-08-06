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
export * from './buildings';
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
export { client } from './generated/client.gen';
export * from './government-portal';
export * from './granular-notifications';
export * from './integrations';
export * from './iot';
export * from './layout';
export * from './leases';
// Shared authenticated-fetch error type — carries HTTP `status` so callers can
// branch on 401/403 (see `./lib/fetch`).
export { ApiError } from './lib/fetch';
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
