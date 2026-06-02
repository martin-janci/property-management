/**
 * Admin API Client — Phase 5 / Epic 10A-2.
 *
 * Functions for the Super-admin Control Plane endpoints exposed under
 * `/api/v1/admin/*`. Auth via the shared token provider, identical to the
 * other api-client modules.
 *
 * # MFA challenge interceptor
 *
 * All API calls go through `authenticatedFetchJson` from `../lib/fetch`,
 * which is the shared factory for all @ppt/api-client modules. It handles
 *   401 { error: "mfa_required" }
 * by delegating to the registered MFA handler (see `mfa-handler.ts`),
 * wired by the app shell to a modal. On success the request is retried once;
 * on cancel / failure the original error surfaces to the calling hook.
 */

import { authenticatedFetchJson } from '../lib/fetch';
import type {
  AdminPaginatedResponse,
  Agency,
  AgencyDetail,
  CreateSystemAnnouncementRequest,
  CreateSystemAnnouncementResponse,
  HealthDashboard,
  ListAgenciesParams,
  ListSystemAnnouncementsParams,
  MetricAlert,
  MetricHistory,
  MetricThreshold,
  OAuthClientSummary,
  RegenerateSecretResponse,
  RegisterOAuthClientRequest,
  RegisterOAuthClientResponse,
  SupportDataResponse,
  SystemAnnouncement,
  TimeRange,
  UpdateOAuthClientRequest,
  UpdateSystemAnnouncementRequest,
  UpdateThresholdRequest,
} from './types';

const _win = typeof window !== 'undefined' ? (window as unknown as Record<string, unknown>) : {};
const API_BASE = `${_win.__API_BASE_URL__ ? String(_win.__API_BASE_URL__) : ''}/api/v1/admin`;
const HEALTH_BASE = `${_win.__API_BASE_URL__ ? String(_win.__API_BASE_URL__) : ''}/api/v1/platform-admin/health`;

function buildQueryString(params: object): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null) {
      searchParams.append(key, String(value));
    }
  }
  const queryString = searchParams.toString();
  return queryString ? `?${queryString}` : '';
}

/**
 * GET /api/v1/admin/agencies — paginated list of tenant agencies.
 */
export async function listAgencies(
  params?: ListAgenciesParams,
  signal?: AbortSignal
): Promise<AdminPaginatedResponse<Agency>> {
  const qs = buildQueryString(params || {});
  return authenticatedFetchJson<AdminPaginatedResponse<Agency>>(`${API_BASE}/agencies${qs}`, {
    signal,
  });
}

/**
 * GET /api/v1/admin/agencies/{id} — detailed view of a single agency.
 *
 * Returns the widened `AgencyDetail` shape (created/updated timestamps plus
 * member/building/unit counts) used by the org drill-in page.
 */
export async function getAgency(agencyId: string, signal?: AbortSignal): Promise<AgencyDetail> {
  return authenticatedFetchJson<AgencyDetail>(
    `${API_BASE}/agencies/${encodeURIComponent(agencyId)}`,
    { signal }
  );
}

/**
 * POST /api/v1/admin/agencies/{id}/suspend — suspend an agency.
 *
 * MFA-gated server-side; the api-client's mfa_required interceptor
 * handles the challenge and retry transparently.
 */
export async function suspendAgency(agencyId: string): Promise<void> {
  await authenticatedFetchJson<void>(`${API_BASE}/agencies/${agencyId}/suspend`, {
    method: 'POST',
  });
}

// ============================================================
// OAuth Client Management (Epic 10A-2)
// ============================================================

/**
 * GET /api/v1/admin/oauth/clients — list all registered OAuth clients.
 */
export async function listOAuthClients(signal?: AbortSignal): Promise<OAuthClientSummary[]> {
  return authenticatedFetchJson<OAuthClientSummary[]>(`${API_BASE}/oauth/clients`, { signal });
}

/**
 * GET /api/v1/admin/oauth/clients/{id} — get a single OAuth client.
 */
export async function getOAuthClient(
  id: string,
  signal?: AbortSignal
): Promise<OAuthClientSummary> {
  return authenticatedFetchJson<OAuthClientSummary>(`${API_BASE}/oauth/clients/${id}`, { signal });
}

/**
 * POST /api/v1/admin/oauth/clients — register a new OAuth client.
 * Returns the plaintext client_secret (shown only once).
 */
export async function registerOAuthClient(
  request: RegisterOAuthClientRequest
): Promise<RegisterOAuthClientResponse> {
  return authenticatedFetchJson<RegisterOAuthClientResponse>(`${API_BASE}/oauth/clients`, {
    method: 'POST',
    body: JSON.stringify({
      name: request.name,
      description: request.description,
      redirect_uris: request.redirectUris,
      scopes: request.scopes,
      is_confidential: request.isConfidential,
      rotate_refresh_tokens: request.rotateRefreshTokens,
    }),
  });
}

/**
 * PATCH /api/v1/admin/oauth/clients/{id} — update an OAuth client.
 *
 * When `data.auditReason` is set (required for scope changes) it is forwarded
 * as `reason` in the JSON body so the backend writes it to the audit log.
 */
export async function updateOAuthClient(
  id: string,
  data: UpdateOAuthClientRequest
): Promise<OAuthClientSummary> {
  // Normalize at the client boundary so whitespace-only reasons (which are
  // truthy) don't produce empty audit-log entries for any caller.
  const auditReason = data.auditReason?.trim();
  return authenticatedFetchJson<OAuthClientSummary>(`${API_BASE}/oauth/clients/${id}`, {
    method: 'PATCH',
    body: JSON.stringify({
      name: data.name,
      description: data.description,
      redirect_uris: data.redirectUris,
      scopes: data.scopes,
      is_active: data.isActive,
      rotate_refresh_tokens: data.rotateRefreshTokens,
      // Audit trail: forwarded to the backend audit_log writer.
      ...(auditReason ? { reason: auditReason } : {}),
    }),
  });
}

/**
 * DELETE /api/v1/admin/oauth/clients/{id} — revoke/deactivate an OAuth client.
 */
export async function revokeOAuthClient(id: string): Promise<void> {
  await authenticatedFetchJson<void>(`${API_BASE}/oauth/clients/${id}`, {
    method: 'DELETE',
  });
}

/**
 * POST /api/v1/admin/oauth/clients/{id}/regenerate-secret
 * Regenerates the client secret. Returns plaintext (shown only once).
 */
export async function regenerateOAuthClientSecret(id: string): Promise<RegenerateSecretResponse> {
  return authenticatedFetchJson<RegenerateSecretResponse>(
    `${API_BASE}/oauth/clients/${id}/regenerate-secret`,
    { method: 'POST' }
  );
}

// ============================================================
// Platform Health Monitoring (Epic 10B.3)
// All requests go through authenticatedFetchJson, which carries the MFA
// challenge-response interceptor (401 mfa_required → modal → retry once).
// ============================================================

export async function fetchHealthDashboard(signal?: AbortSignal): Promise<HealthDashboard> {
  return authenticatedFetchJson<HealthDashboard>(`${HEALTH_BASE}/dashboard`, { signal });
}

export async function fetchHealthAlerts(
  activeOnly: boolean,
  signal?: AbortSignal
): Promise<MetricAlert[]> {
  return authenticatedFetchJson<MetricAlert[]>(`${HEALTH_BASE}/alerts?active_only=${activeOnly}`, {
    signal,
  });
}

export async function acknowledgeHealthAlert(alertId: string): Promise<void> {
  await authenticatedFetchJson<void>(
    `${HEALTH_BASE}/alerts/${encodeURIComponent(alertId)}/acknowledge`,
    { method: 'POST' }
  );
}

export async function fetchMetricHistory(
  metricName: string,
  range: TimeRange,
  signal?: AbortSignal
): Promise<MetricHistory> {
  const n = encodeURIComponent(metricName);
  return authenticatedFetchJson<MetricHistory>(
    `${HEALTH_BASE}/metrics/${n}/history?range=${range}`,
    { signal }
  );
}

export async function updateHealthThreshold(
  metricName: string,
  data: UpdateThresholdRequest
): Promise<MetricThreshold> {
  return authenticatedFetchJson<MetricThreshold>(
    `${HEALTH_BASE}/thresholds/${encodeURIComponent(metricName)}`,
    { method: 'PUT', body: JSON.stringify(data) }
  );
}

// ============================================================
// System Announcements (Epic 10B.4)
// ============================================================

const SYSANN_BASE = `${_win.__API_BASE_URL__ ? String(_win.__API_BASE_URL__) : ''}/api/v1/platform-admin/announcements`;

export async function listSystemAnnouncements(
  params?: ListSystemAnnouncementsParams,
  signal?: AbortSignal
): Promise<SystemAnnouncement[]> {
  const qs = buildQueryString(params || {});
  return authenticatedFetchJson<SystemAnnouncement[]>(`${SYSANN_BASE}${qs}`, { signal });
}

export async function getSystemAnnouncement(
  id: string,
  signal?: AbortSignal
): Promise<SystemAnnouncement> {
  return authenticatedFetchJson<SystemAnnouncement>(`${SYSANN_BASE}/${encodeURIComponent(id)}`, {
    signal,
  });
}

export async function createSystemAnnouncement(
  data: CreateSystemAnnouncementRequest
): Promise<CreateSystemAnnouncementResponse> {
  return authenticatedFetchJson<CreateSystemAnnouncementResponse>(SYSANN_BASE, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function updateSystemAnnouncement(
  id: string,
  data: UpdateSystemAnnouncementRequest
): Promise<SystemAnnouncement> {
  return authenticatedFetchJson<SystemAnnouncement>(`${SYSANN_BASE}/${encodeURIComponent(id)}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

export async function deleteSystemAnnouncement(id: string): Promise<void> {
  await authenticatedFetchJson<void>(`${SYSANN_BASE}/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}

// ============================================================
// Support Data (Epic 10B.5)
// ============================================================

const SUPPORT_DATA_BASE = `${_win.__API_BASE_URL__ ? String(_win.__API_BASE_URL__) : ''}/api/v1/platform-admin`;

/**
 * GET /api/v1/platform-admin/support-data
 * Returns platform-wide tenant diagnostics (user counts, active sessions,
 * fault summary). Requires `audit_read` capability.
 */
export async function fetchSupportData(signal?: AbortSignal): Promise<SupportDataResponse> {
  return authenticatedFetchJson<SupportDataResponse>(`${SUPPORT_DATA_BASE}/support-data`, {
    signal,
  });
}
