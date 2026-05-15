/**
 * Admin API Client — Phase 5.
 *
 * Functions for the Super-admin Control Plane endpoints exposed under
 * `/api/v1/admin/*`. Auth via the shared token provider, identical to the
 * other api-client modules.
 */

import { getToken } from '../auth';
import type {
  AdminPaginatedResponse,
  Agency,
  ListAgenciesParams,
} from './types';

const _win = typeof window !== 'undefined' ? (window as unknown as Record<string, unknown>) : {};
const API_BASE = `${_win.__API_BASE_URL__ ? String(_win.__API_BASE_URL__) : ''}/api/v1/admin`;

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function apiRequest<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.message || `HTTP error ${response.status}`);
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
  signal?: AbortSignal,
): Promise<AdminPaginatedResponse<Agency>> {
  const qs = buildQueryString(params || {});
  return apiRequest<AdminPaginatedResponse<Agency>>(`${API_BASE}/agencies${qs}`, { signal });
}
