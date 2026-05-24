/**
 * Admin Module — Super-admin Control Plane (Phase 5 / Epic 10A-2).
 *
 * API client + hooks for `/api/v1/admin/*` endpoints.
 */

export {
  acknowledgeHealthAlert,
  fetchHealthAlerts,
  fetchHealthDashboard,
  fetchMetricHistory,
  getOAuthClient,
  listAgencies,
  listOAuthClients,
  regenerateOAuthClientSecret,
  registerOAuthClient,
  revokeOAuthClient,
  suspendAgency,
  updateHealthThreshold,
  updateOAuthClient,
} from './api';
export {
  adminKeys,
  useAcknowledgeAlert,
  useAgencies,
  useHealthAlerts,
  useHealthDashboard,
  useMetricHistory,
  useOAuthClient,
  useOAuthClients,
  useRegenerateOAuthClientSecret,
  useRegisterOAuthClient,
  useRevokeOAuthClient,
  useUpdateHealthThreshold,
  useUpdateOAuthClient,
} from './hooks';
export {
  hasMfaChallengeHandler,
  type MfaChallengeHandler,
  setMfaChallengeHandler,
} from './mfa-handler';
export type {
  AdminPaginatedResponse,
  Agency,
  AgencyStatus,
  CurrentMetric,
  HealthDashboard,
  KnownOAuthScope,
  ListAgenciesParams,
  MetricAlert,
  MetricDataPoint,
  MetricHistory,
  MetricStats,
  MetricStatus,
  MetricThreshold,
  OAuthClientSummary,
  RegenerateSecretResponse,
  RegisterOAuthClientRequest,
  RegisterOAuthClientResponse,
  TimeRange,
  UpdateOAuthClientRequest,
  UpdateThresholdRequest,
} from './types';
export { KNOWN_OAUTH_SCOPES } from './types';
