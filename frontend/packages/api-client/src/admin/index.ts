/**
 * Admin Module — Super-admin Control Plane (Phase 5).
 *
 * API client + hooks for `/api/v1/admin/*` endpoints. Currently surfaces
 * the agency list; will grow to cover users, audit, feature flags, and
 * platform settings as those endpoints land.
 */

export { listAgencies } from './api';
export { adminKeys, useAgencies } from './hooks';
export type {
  AdminPaginatedResponse,
  Agency,
  AgencyStatus,
  ListAgenciesParams,
} from './types';
