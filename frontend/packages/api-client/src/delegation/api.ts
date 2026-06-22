/**
 * Delegation API client (Epic 3, Story 3.4).
 *
 * Thin transport over `/api/v1/delegations` (see
 * `backend/servers/api-server/src/routes/delegations.rs`). Uses the centralised
 * authenticated fetch helper so token + MFA + error handling stay consistent
 * with the rest of the api-client (lib/fetch.ts, #486).
 */

import { authenticatedFetchJson } from '../lib/fetch';
import type {
  CheckDelegationResult,
  CreateDelegationRequest,
  Delegation,
  DelegationSummary,
  ListDelegationsQuery,
} from './types';

const API_BASE = '/api/v1/delegations';

/** Build a `?a=b&c=d` query string, skipping empty/undefined values. */
function buildQueryString(params: Record<string, string | undefined | null>): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') {
      searchParams.append(key, value);
    }
  }
  const query = searchParams.toString();
  return query ? `?${query}` : '';
}

/**
 * Create a new delegation.
 *
 * `POST /api/v1/delegations` → 201 {@link Delegation}.
 */
export async function createDelegation(request: CreateDelegationRequest): Promise<Delegation> {
  return authenticatedFetchJson<Delegation>(API_BASE, {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

/**
 * List delegations the current user created (as owner).
 *
 * `GET /api/v1/delegations` → {@link DelegationSummary}[].
 */
export async function listMyDelegations(
  query: ListDelegationsQuery = {}
): Promise<DelegationSummary[]> {
  const qs = buildQueryString({ unit_id: query.unit_id, status: query.status });
  return authenticatedFetchJson<DelegationSummary[]>(`${API_BASE}${qs}`);
}

/**
 * List delegations the current user received (as delegate).
 *
 * `GET /api/v1/delegations/received` → {@link DelegationSummary}[].
 */
export async function listReceivedDelegations(): Promise<DelegationSummary[]> {
  return authenticatedFetchJson<DelegationSummary[]>(`${API_BASE}/received`);
}

/**
 * Fetch a single delegation by id (owner or delegate only).
 *
 * `GET /api/v1/delegations/{id}` → {@link Delegation}.
 */
export async function getDelegation(id: string): Promise<Delegation> {
  return authenticatedFetchJson<Delegation>(`${API_BASE}/${encodeURIComponent(id)}`);
}

/**
 * Accept a delegation invitation (delegate only).
 *
 * `POST /api/v1/delegations/{id}/accept` → {@link Delegation}.
 */
export async function acceptDelegation(id: string): Promise<Delegation> {
  return authenticatedFetchJson<Delegation>(`${API_BASE}/${encodeURIComponent(id)}/accept`, {
    method: 'POST',
  });
}

/**
 * Decline a delegation invitation (delegate only).
 *
 * `POST /api/v1/delegations/{id}/decline` → {@link Delegation}.
 */
export async function declineDelegation(id: string): Promise<Delegation> {
  return authenticatedFetchJson<Delegation>(`${API_BASE}/${encodeURIComponent(id)}/decline`, {
    method: 'POST',
  });
}

/**
 * Revoke an active delegation (owner only).
 *
 * `DELETE /api/v1/delegations/{id}/revoke` → {@link Delegation}.
 */
export async function revokeDelegation(id: string): Promise<Delegation> {
  return authenticatedFetchJson<Delegation>(`${API_BASE}/${encodeURIComponent(id)}/revoke`, {
    method: 'DELETE',
  });
}

/**
 * Check whether the current user holds a delegation for a unit + scope.
 *
 * `GET /api/v1/delegations/check/{unit_id}/{scope}` → {@link CheckDelegationResult}.
 */
export async function checkDelegation(
  unitId: string,
  scope: string
): Promise<CheckDelegationResult> {
  return authenticatedFetchJson<CheckDelegationResult>(
    `${API_BASE}/check/${encodeURIComponent(unitId)}/${encodeURIComponent(scope)}`
  );
}
