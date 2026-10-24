/**
 * My Unit (resident-facing) API client — Epic 3, Story 3.6.
 *
 * Thin wrapper over `GET /api/v1/users/me/units`. The endpoint resolves the
 * caller's own active residencies server-side, so there is no id parameter to
 * pass (and no IDOR surface) and no other residents' PII in the response.
 */

import { bearerAuthHeader } from '../auth';
import type { MyUnit } from './types';

const API_BASE = '/api/v1/users/me/units';

async function apiRequest<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...bearerAuthHeader(),
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.message || `HTTP error ${response.status}`);
  }

  return response.json();
}

/** Fetch the authenticated resident's unit(s). */
export async function getMyUnits(): Promise<MyUnit[]> {
  return apiRequest<MyUnit[]>(API_BASE);
}
