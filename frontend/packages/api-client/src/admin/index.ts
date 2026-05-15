/**
 * Admin Module — Super-admin Control Plane (Phase 5).
 *
 * API client + hooks for `/api/v1/admin/*` endpoints. Currently surfaces
 * the agency list; will grow to cover users, audit, feature flags, and
 * platform settings as those endpoints land.
 */

export { listAgencies, suspendAgency } from './api';
export { adminKeys, useAgencies } from './hooks';
export {
  hasMfaChallengeHandler,
  type MfaChallengeHandler,
  setMfaChallengeHandler,
} from './mfa-handler';
export type {
  AdminPaginatedResponse,
  Agency,
  AgencyStatus,
  ListAgenciesParams,
} from './types';
