/**
 * Client-side JWT helpers.
 *
 * The mobile app never *verifies* JWTs (that happens server-side); it only
 * decodes the payload to read claims such as `tenant_id`, which is injected as
 * the `X-Tenant-ID` header that the api-server's RLS extractor requires.
 *
 * This module is the single source of truth for that decode. It replaces two
 * byte-for-byte copies that previously lived in `hooks/useApi.ts` and
 * `screens/documents/DocumentUploadScreen.tsx` (a divergence in one would have
 * silently passed the other's test suite — see GitHub #2327).
 */

/**
 * Decode a JWT payload without verifying the signature.
 *
 * Returns `null` if the token is malformed or doesn't contain a payload.
 */
export function decodeJwtPayload(token: string): Record<string, unknown> | null {
  try {
    const parts = token.split('.');
    if (parts.length < 2) return null;
    // Convert base64url → base64 so `atob` (shipped by the React Native /
    // jest-expo runtime) can decode it, re-adding the stripped `=` padding.
    const padded = parts[1].replace(/-/g, '+').replace(/_/g, '/');
    const padding = '='.repeat((4 - (padded.length % 4)) % 4);
    const json = atob(padded + padding);
    return JSON.parse(json) as Record<string, unknown>;
  } catch {
    return null;
  }
}

/** Pull the `tenant_id` claim out of a JWT, or `null` if absent/non-string. */
export function extractTenantId(token: string): string | null {
  const payload = decodeJwtPayload(token);
  const value = payload?.tenant_id;
  return typeof value === 'string' ? value : null;
}

/**
 * Report whether a JWT's `exp` claim is at/past `now + skewSeconds`.
 *
 * `skewSeconds` (default 0) lets callers treat a token that is *about* to
 * expire as already expired, so a proactive refresh happens before the bearer
 * goes stale rather than after the first 401.
 *
 * Access tokens are short-lived, so a cold start (or a biometric unlock after
 * the device sat locked) can restore a token whose TTL already elapsed. Callers
 * use this to decide whether to `refreshToken()` before trusting a restored
 * session (see AuthContext `initialize` / `authenticateWithBiometric`).
 *
 * Semantics are deliberately narrow: this returns `true` only for a token that
 * carries a *readable numeric* `exp` in the past. A malformed token, or one
 * with a missing/non-numeric `exp`, returns `false` — we can't prove it is
 * expired, so we defer to the server's own 401 (which drives the normal refresh
 * path) rather than eagerly discarding a session at boot.
 */
export function isJwtExpired(token: string, skewSeconds = 0): boolean {
  const payload = decodeJwtPayload(token);
  const exp = payload?.exp;
  if (typeof exp !== 'number') return false;
  const nowSeconds = Date.now() / 1000;
  return exp <= nowSeconds + skewSeconds;
}
