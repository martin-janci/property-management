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
export * from './forms';
// Export generated types and client
// These will be populated after running `pnpm generate`
export * from './generated';
export { client } from './generated/client.gen';
export * from './government-portal';
export * from './integrations';
export * from './iot';
export * from './leases';
export * from './messaging';
export * from './meters';
export * from './mfa';
export * from './migration';
export * from './neighbors';
export * from './news';
export * from './notification-preferences';
export * from './oauth-consent';
export * from './oauth-grants';
export * from './onboarding';
export * from './outages';
export * from './packages';
export * from './registry';
export * from './reports';
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
