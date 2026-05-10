/**
 * Local storage for the email/password (`/api/v1/users/login`) JWT token +
 * portal user. The OAuth/SSO cookie flow is unrelated and stays in
 * `auth-context.tsx`; this helper just makes the form-login path
 * persist across reloads and let `Authorization: Bearer …` go on
 * authenticated requests.
 *
 * Storage keys are namespaced (`ppt-portal-*`) so they don't collide with
 * any other localStorage usage in the app.
 *
 * SSR-safe: every helper checks for `window` first.
 */

const TOKEN_KEY = 'ppt-portal-token';
const USER_KEY = 'ppt-portal-user';
const EXPIRES_KEY = 'ppt-portal-expires';

export interface StoredPortalUser {
  id: string;
  email: string;
  name: string;
  profile_image_url?: string | null;
  is_linked_to_pm?: boolean;
}

export interface StoredSession {
  token: string;
  user: StoredPortalUser;
  /** ISO 8601 expiry timestamp returned by the server. */
  expiresAt: string;
}

function hasStorage(): boolean {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';
}

export function setSession(s: StoredSession): void {
  if (!hasStorage()) return;
  try {
    window.localStorage.setItem(TOKEN_KEY, s.token);
    window.localStorage.setItem(USER_KEY, JSON.stringify(s.user));
    window.localStorage.setItem(EXPIRES_KEY, s.expiresAt);
  } catch {
    // Quota / disabled / private mode — non-fatal, just won't persist.
  }
}

export function getSession(): StoredSession | null {
  if (!hasStorage()) return null;
  try {
    const token = window.localStorage.getItem(TOKEN_KEY);
    const userRaw = window.localStorage.getItem(USER_KEY);
    const expiresAt = window.localStorage.getItem(EXPIRES_KEY);
    if (!token || !userRaw || !expiresAt) return null;
    // Cheap client-side expiry check — server still validates on each request.
    if (new Date(expiresAt).getTime() <= Date.now()) {
      clearSession();
      return null;
    }
    return { token, user: JSON.parse(userRaw) as StoredPortalUser, expiresAt };
  } catch {
    return null;
  }
}

export function clearSession(): void {
  if (!hasStorage()) return;
  try {
    window.localStorage.removeItem(TOKEN_KEY);
    window.localStorage.removeItem(USER_KEY);
    window.localStorage.removeItem(EXPIRES_KEY);
  } catch {
    // Best-effort.
  }
}

/**
 * Returns headers to merge into a fetch() init when the user has a stored
 * portal session. Empty object otherwise — safe to spread unconditionally.
 */
export function getAuthHeader(): Record<string, string> {
  const s = getSession();
  return s ? { Authorization: `Bearer ${s.token}` } : {};
}
