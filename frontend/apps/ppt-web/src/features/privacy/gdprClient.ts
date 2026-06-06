/**
 * Authenticated fetch helper for the user-facing GDPR endpoints
 * (/api/v1/gdpr/*). Mirrors the conventions in features/auth/authApiClient.ts:
 * a configurable base URL (VITE_API_BASE_URL) plus the bearer token read from
 * localStorage. Every GDPR handler requires the `AuthUser` extractor, so a bare
 * `fetch` with no Authorization header returns 401 in production (gap-sweep).
 */

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';
const ACCESS_TOKEN_KEY = 'ppt_access_token';

function readAccessToken(): string | undefined {
  try {
    return localStorage.getItem(ACCESS_TOKEN_KEY) ?? undefined;
  } catch {
    return undefined;
  }
}

/**
 * Issue an authenticated request against an API path. Attaches the bearer
 * token (when present) and resolves the configured API base URL.
 */
export function gdprFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const token = readAccessToken();
  const headers = new Headers(init.headers);
  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
  }
  return fetch(`${API_BASE_URL}${path}`, { ...init, headers });
}
