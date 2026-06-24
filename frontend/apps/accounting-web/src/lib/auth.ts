/**
 * accounting-web auth token access.
 *
 * accounting-server is a resource server: every authenticated call carries a
 * Bearer JWT that api-server issued (EPIC-ACC-17 STORY-002). The PPT web apps
 * persist the SSO access token in localStorage under a `<prefix>_access_token`
 * key (see @ppt/shared tokenStorage). accounting-web reuses the shared `ppt`
 * prefix so a single sign-on session is honoured across the suite.
 *
 * This module is the ONLY place that knows where the token lives; the SDK is
 * told how to read it via configureAccountingApi(getAccessToken) in the (app)
 * layout provider, keeping the SDK storage-agnostic.
 */

/** localStorage key for the SSO access token (shared `ppt` prefix). */
export const ACCESS_TOKEN_KEY = 'ppt_access_token';

/**
 * Read the current Bearer access token, or null when unauthenticated or on the
 * server (no localStorage). Safe to call during SSR — returns null there.
 */
export function getAccessToken(): string | null {
  if (typeof window === 'undefined') return null;
  try {
    return window.localStorage.getItem(ACCESS_TOKEN_KEY);
  } catch {
    // localStorage can throw in privacy modes / sandboxed iframes.
    return null;
  }
}

/** True when an access token is present (best-effort, client-side only). */
export function isAuthenticated(): boolean {
  return getAccessToken() !== null;
}
