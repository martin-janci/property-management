/** Returns true when localStorage is accessible (guards against private-mode and SSR). */
export function isLocalStorageAvailable(): boolean {
  try {
    if (typeof window === 'undefined') return false;
    const key = '__storage_test__';
    window.localStorage.setItem(key, key);
    window.localStorage.removeItem(key);
    return true;
  } catch {
    return false;
  }
}

// ============================================================================
// Auth-scoped localStorage (purged on logout)
// ============================================================================

/**
 * localStorage key for the in-progress dispute-filing draft (Epic 80, AC-4).
 * Written by `useDraftStorage` on FileDisputePage; re-exported there as
 * `DISPUTE_DRAFT_KEY` so there is a single source of truth for the key string.
 */
export const DISPUTE_FILING_DRAFT_KEY = 'ppt-dispute-filing-draft';

/**
 * localStorage keys that hold user-/tenant-scoped data persisted OUTSIDE the
 * TanStack Query cache (which `AuthContext.logout` purges separately via
 * {@link AUTHED_QUERY_KEY_ROOTS}). These are feature-local persisted stores —
 * e.g. the dispute-filing draft — that would otherwise survive logout and leak
 * into the next session on a shared workstation. `AuthContext.logout` calls
 * {@link purgeAuthedLocalStorage} to drop every key listed here.
 *
 * The auth token keys (`ppt_access_token`, `ppt_user`, `ppt_tenants`, …) are
 * NOT listed here — they are owned and cleared by `tokenStorage.clear()`.
 *
 * When you add a new auth-scoped localStorage key, add it here too.
 */
export const AUTHED_LOCAL_STORAGE_KEYS = [DISPUTE_FILING_DRAFT_KEY] as const;

/**
 * Remove every {@link AUTHED_LOCAL_STORAGE_KEYS} entry from localStorage.
 * Best-effort and bounded: only the listed keys are touched, and storage
 * errors (private mode, SSR) are swallowed so logout never throws.
 */
export function purgeAuthedLocalStorage(): void {
  if (!isLocalStorageAvailable()) return;
  for (const key of AUTHED_LOCAL_STORAGE_KEYS) {
    try {
      window.localStorage.removeItem(key);
    } catch {
      // Storage unavailable / quota — best-effort.
    }
  }
}
