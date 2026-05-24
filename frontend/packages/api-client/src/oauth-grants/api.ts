/**
 * OAuth User Grants API Client
 *
 * API functions for listing and revoking a user's OAuth grants (Epic 10A, Story 10A-3).
 * Wires to /api/v1/oauth/grants on api-server.
 *
 * Both endpoints require a valid Bearer token (the user's session).
 */

import { getToken } from '../auth';
import type { UserGrant } from './types';

const API_BASE = '/api/v1/oauth/grants';

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  if (!token) {
    throw new Error('OAuth grants API requires authentication');
  }
  return {
    'Content-Type': 'application/json',
    Authorization: `Bearer ${token}`,
  };
}

async function parseResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const body = (await response.json().catch(() => ({ message: response.statusText }))) as {
      message?: string;
      error?: string;
      code?: string;
    };
    const err = new Error(body.message || 'OAuth grants API request failed') as Error & {
      status: number;
      code: string;
    };
    err.status = response.status;
    err.code = body.error ?? body.code ?? 'UNKNOWN_ERROR';
    throw err;
  }
  return response.json() as Promise<T>;
}

/**
 * List all active OAuth grants for the authenticated user.
 *
 * GET /api/v1/oauth/grants
 *
 * Returns every third-party application the user has approved, along with
 * the granted scopes and a timestamp.
 */
export async function listUserGrants(): Promise<UserGrant[]> {
  const response = await fetch(API_BASE, {
    method: 'GET',
    headers: getAuthHeaders(),
  });
  return parseResponse<UserGrant[]>(response);
}

/**
 * Revoke the user's grant for a specific OAuth client.
 *
 * DELETE /api/v1/oauth/grants/{clientId}
 *
 * After revocation all access tokens and refresh tokens issued to this
 * client for this user are invalidated. The client will need to re-request
 * authorization to regain access.
 *
 * @param clientId - The OAuth client_id string (not the UUID).
 */
export async function revokeUserGrant(clientId: string): Promise<void> {
  const response = await fetch(`${API_BASE}/${encodeURIComponent(clientId)}`, {
    method: 'DELETE',
    headers: getAuthHeaders(),
  });
  if (!response.ok) {
    const body = (await response.json().catch(() => ({ message: response.statusText }))) as {
      message?: string;
      error?: string;
      code?: string;
    };
    const err = new Error(body.message || 'Failed to revoke grant') as Error & {
      status: number;
      code: string;
    };
    err.status = response.status;
    err.code = body.error ?? body.code ?? 'UNKNOWN_ERROR';
    throw err;
  }
  // 204 No Content — nothing to parse
}
