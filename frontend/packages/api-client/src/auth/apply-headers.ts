/**
 * Single source of truth for the authenticated header set (#3006).
 *
 * Fills `Authorization` (bearer) and `X-Tenant-ID` on a `Headers` object from
 * the registered token/org providers, but only when the caller has not already
 * set them — so an explicit per-request header (e.g. a deliberate cross-org
 * `X-Tenant-ID`) still wins, and unauthenticated requests (no token / no active
 * org) are left untouched.
 *
 * Both raw request paths consume this so they cannot drift apart:
 *   - the generated client's request interceptor (`./interceptors.ts`), and
 *   - the raw-`Response` primitive `authenticatedFetch` (`../lib/fetch.ts`).
 *
 * Before this existed, each re-implemented the identical injection block, and
 * PR #3004's post-merge review (#3006) flagged that the copies had started to
 * drift. Any future change to how auth/tenant headers are derived now happens
 * here, once.
 *
 * NOTE: `authenticatedFetchJson` (`../lib/fetch.ts`) deliberately injects only
 * `Authorization` and not `X-Tenant-ID` — see the note at its `getAuthHeaders`.
 * It does NOT add the tenant header, but it DOES share the bearer-credential
 * derivation below (`bearerAuthorization`), so the one remaining copy of the
 * `Bearer <token>` construction is now folded into this module too. That
 * Authorization-only divergence is intentional and pinned by a test; revisit
 * it as its own behavior change, not here.
 */

import { getOrg } from './org-provider';
import { getToken } from './token-provider';

/**
 * The single source of truth for the bearer `Authorization` credential value.
 *
 * Reads the current token from the registered token provider and formats it as
 * a `Bearer` credential, or returns `undefined` when no token is registered.
 *
 * All three `@ppt/api-client` fetch primitives derive the Authorization header
 * from here, so the `Bearer <token>` construction lives in exactly one place:
 *   - `applyAuthHeaders` below (generated-client interceptor + raw
 *     `authenticatedFetch`), which layers `X-Tenant-ID` on top, and
 *   - the JSON-path `getAuthHeaders` in `../lib/fetch.ts`, which deliberately
 *     sends Authorization only (no `X-Tenant-ID`; see #3006).
 */
export function bearerAuthorization(): string | undefined {
  const token = getToken();
  return token ? `Bearer ${token}` : undefined;
}

/**
 * The single source of truth for the Authorization-only header *object* form of
 * the shared bearer credential: `{ Authorization: 'Bearer <token>' }`, or an
 * empty object when no token is registered.
 *
 * This is the object-shaped sibling of `bearerAuthorization` for the
 * spread-into-`HeadersInit` JSON path (`authenticatedFetchJson` in
 * `../lib/fetch.ts`), which deliberately sends Authorization and NOT
 * `X-Tenant-ID` (see #3006). It folds the last remaining copy of the
 * `token ? { Authorization: ... } : {}` wrap into this module, so the
 * object form is derived in exactly one place and cannot drift from the
 * credential it wraps.
 */
export function bearerAuthHeader(): Record<string, string> {
  const authorization = bearerAuthorization();
  return authorization ? { Authorization: authorization } : {};
}

/**
 * Mutates `headers` in place, setting `Authorization` and `X-Tenant-ID` from the
 * registered providers unless already present.
 */
export function applyAuthHeaders(headers: Headers): void {
  if (!headers.has('Authorization')) {
    const authorization = bearerAuthorization();
    if (authorization) {
      headers.set('Authorization', authorization);
    }
  }
  if (!headers.has('X-Tenant-ID')) {
    const org = getOrg();
    if (org) {
      headers.set('X-Tenant-ID', org);
    }
  }
}
