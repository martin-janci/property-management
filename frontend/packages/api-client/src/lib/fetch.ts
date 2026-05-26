/**
 * Shared authenticated `fetch` wrapper for hooks that don't go through the
 * per-module `Api*` classes.
 *
 * Centralizing this here:
 *   - injects the Authorization header via the registered `tokenProvider`,
 *     instead of re-implementing the header logic in each hooks module
 *     (which was the criticism in #486),
 *   - normalises HTTP error → `Error` with a human-readable message,
 *   - handles 204 No Content uniformly,
 *   - gives us a single seam to add 401-refresh / retry / telemetry later
 *     without revisiting every callsite.
 *
 * Hooks should import { authenticatedFetchJson } from '../lib/fetch'.
 */

import { getToken } from '../auth';

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

/**
 * Fetch the given URL, automatically attaching the bearer token from the
 * registered token provider, and parsing the JSON response.
 *
 * Throws `Error` (with the server-provided `message` if any) on non-2xx
 * responses. Returns `undefined as unknown as T` for 204.
 */
export async function authenticatedFetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
      ...init?.headers,
    },
  });
  if (!response.ok) {
    const err = await response.json().catch(() => ({ message: 'Unknown error' }));
    throw new Error((err as { message?: string }).message || `HTTP ${response.status}`);
  }
  if (response.status === 204) return undefined as unknown as T;
  return response.json() as Promise<T>;
}
