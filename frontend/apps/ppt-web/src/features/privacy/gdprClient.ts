/**
 * Authenticated fetch helper for the user-facing GDPR endpoints
 * (/api/v1/gdpr/*).
 *
 * Delegates to the shared `authenticatedFetch` from `@ppt/api-client` — the same
 * authenticated client the generated SDK and every other feature module use — so
 * the bearer token comes from the registered token provider (not a bespoke
 * `localStorage` read), the base URL matches every other API call, and a
 * `401 { error: "mfa_required" }` triggers the shared MFA retry. Every GDPR
 * handler requires the `AuthUser` extractor, so a request with a missing or
 * stale Authorization header returns 401 in production (gap-sweep, #3000).
 */

import { authenticatedFetch } from '@ppt/api-client';

/**
 * Issue an authenticated request against a GDPR API path. Returns the raw
 * `Response` so callers keep their existing `.ok` / `.json()` / `.text()`
 * handling. Auth headers, token refresh, and base-URL resolution are handled by
 * the shared client.
 */
export function gdprFetch(path: string, init: RequestInit = {}): Promise<Response> {
  return authenticatedFetch(path, init);
}
