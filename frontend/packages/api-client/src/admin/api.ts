/**
 * Admin API Client — Phase 5.
 *
 * Functions for the Super-admin Control Plane endpoints exposed under
 * `/api/v1/admin/*`. Auth via the shared token provider, identical to the
 * other api-client modules.
 *
 * # N9: MFA challenge interceptor
 *
 * `apiRequest` is the choke point. When the api-server returns
 *   401 { error: "mfa_required" }
 * we hand off to the registered MFA handler (see `mfa-handler.ts`),
 * which is wired by the app shell to a modal. On success we retry the
 * original request once; on cancel / failure we surface the original
 * error so the calling hook reports it normally.
 */

import { getToken } from '../auth';
import { requestMfaChallenge } from './mfa-handler';
import type { AdminPaginatedResponse, Agency, ListAgenciesParams } from './types';

const _win = typeof window !== 'undefined' ? (window as unknown as Record<string, unknown>) : {};
const API_BASE = `${_win.__API_BASE_URL__ ? String(_win.__API_BASE_URL__) : ''}/api/v1/admin`;

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

/**
 * Single-request POST/GET/PUT/DELETE wrapper.
 *
 * The `_alreadyRetried` parameter is internal — it guards the
 * mfa_required retry loop so a server stuck in a 401 cycle cannot
 * spawn unbounded modals.
 */
async function apiRequest<T>(
  url: string,
  options: RequestInit = {},
  _alreadyRetried = false
): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
      ...options.headers,
    },
  });

  if (!response.ok) {
    // Read the body once — a 401 mfa_required carries the marker we need
    // and we still want it for the error message in the fall-through case.
    const error = (await response.json().catch(() => ({}))) as {
      error?: string;
      message?: string;
    };

    if (response.status === 401 && error?.error === 'mfa_required' && !_alreadyRetried) {
      const ok = await requestMfaChallenge();
      if (ok) {
        return apiRequest<T>(url, options, true);
      }
    }

    throw new Error(error.message || error.error || `HTTP error ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

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
  return apiRequest<AdminPaginatedResponse<Agency>>(`${API_BASE}/agencies${qs}`, { signal });
}

/**
 * POST /api/v1/admin/agencies/{id}/suspend — suspend an agency.
 *
 * MFA-gated server-side; the api-client's mfa_required interceptor
 * handles the challenge and retry transparently.
 */
export async function suspendAgency(agencyId: string): Promise<void> {
  await apiRequest<void>(`${API_BASE}/agencies/${agencyId}/suspend`, {
    method: 'POST',
  });
}
