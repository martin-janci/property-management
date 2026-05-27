/**
 * Shared MFA challenge handler registry.
 *
 * The api-server returns
 *   401 { error: "mfa_required" }
 * when a capability-gated endpoint requires a recent MFA verification.
 * This module is the seam between the api-client (which detects the error
 * in `authenticatedFetchJson`) and the UI (which knows how to prompt for a
 * TOTP code via a modal).
 *
 * The app-shell registers a handler at startup via `setMfaChallengeHandler`.
 * `authenticatedFetchJson` calls `requestMfaChallenge()` on detection; on
 * success the original request is retried exactly once.
 *
 * # Why a global setter rather than per-call config
 *
 * The api-client is consumed by hooks (`useAgencies`, `useHealthDashboard`,
 * etc.) that have no obvious place to thread an MFA UI plumbing argument. A
 * single startup registration mirrors the existing `setTokenProvider` pattern
 * from `auth/token-provider.ts`.
 *
 * The handler returns a Promise<boolean>:
 *   * `true`  — user completed MFA; retry the request.
 *   * `false` — user cancelled / failed; surface the original 401 to caller.
 *
 * NOTE: `admin/mfa-handler.ts` re-exports everything from this module for
 * backward compatibility — existing consumers that import from the admin
 * subpath continue to work unchanged.
 */

export type MfaChallengeHandler = () => Promise<boolean>;

let handler: MfaChallengeHandler | null = null;

/**
 * Register the MFA challenge handler. Called once during app
 * initialization, sibling to `setTokenProvider`. Calling twice replaces
 * the previous handler — useful for tests that swap it out.
 */
export function setMfaChallengeHandler(fn: MfaChallengeHandler | null): void {
  handler = fn;
}

/**
 * Trigger an MFA challenge. Returns `false` immediately if no handler
 * has been registered (degrade gracefully — without a UI seam there is
 * no way to prompt, so the original error must surface).
 */
export async function requestMfaChallenge(): Promise<boolean> {
  if (!handler) return false;
  try {
    return await handler();
  } catch {
    return false;
  }
}

export function hasMfaChallengeHandler(): boolean {
  return handler !== null;
}
