/**
 * Admin Module — Super-admin Control Plane (Phase 5 / Epic 10A-2).
 *
 * API client + hooks for `/api/v1/admin/*` endpoints.
 */

export {
  getOAuthClient,
  listAgencies,
  listOAuthClients,
  regenerateOAuthClientSecret,
  registerOAuthClient,
  revokeOAuthClient,
  suspendAgency,
  updateOAuthClient,
} from './api';
export {
  adminKeys,
  useAgencies,
  useOAuthClient,
  useOAuthClients,
  useRegenerateOAuthClientSecret,
  useRegisterOAuthClient,
  useRevokeOAuthClient,
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
  KnownOAuthScope,
  ListAgenciesParams,
  OAuthClientSummary,
  RegenerateSecretResponse,
  RegisterOAuthClientRequest,
  RegisterOAuthClientResponse,
  UpdateOAuthClientRequest,
} from './types';
export { KNOWN_OAUTH_SCOPES } from './types';
