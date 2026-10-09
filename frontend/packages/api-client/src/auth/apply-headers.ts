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
 * `Authorization` and not `X-Tenant-ID` — see the note at its `getAuthHeaders`
 * — so it does not route through this helper. That divergence is intentional
 * and pinned by a test; revisit it as its own behavior change, not here.
 */

import { getOrg } from './org-provider';
import { getToken } from './token-provider';

/**
 * Mutates `headers` in place, setting `Authorization` and `X-Tenant-ID` from the
 * registered providers unless already present.
 */
export function applyAuthHeaders(headers: Headers): void {
  if (!headers.has('Authorization')) {
    const token = getToken();
    if (token) {
      headers.set('Authorization', `Bearer ${token}`);
    }
  }
  if (!headers.has('X-Tenant-ID')) {
    const org = getOrg();
    if (org) {
      headers.set('X-Tenant-ID', org);
    }
  }
}
