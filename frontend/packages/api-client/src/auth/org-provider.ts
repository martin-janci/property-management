/**
 * Active-Organization Provider
 *
 * Companion to the token provider (`./token-provider`). Lets the api-client's
 * auth interceptor read the caller's active organization id (sent as the
 * `X-Tenant-ID` header) without importing the app's React `AuthContext`.
 *
 * The application registers a getter once during init (see `AuthContext`), and
 * the auth interceptor (`./interceptors`) calls `getOrg()` per request. Returns
 * `null` when no org is active (no header is then added). (#1522)
 */

/** Returns the current active organization id, or `null`. */
export type OrgProvider = () => string | null;

let globalOrgProvider: OrgProvider | null = null;

/** Register the global active-org provider (call once at app init). */
export function setOrgProvider(provider: OrgProvider): void {
  globalOrgProvider = provider;
}

/** Clear the global active-org provider (call on logout / unmount). */
export function clearOrgProvider(): void {
  globalOrgProvider = null;
}

/** Get the current active organization id via the registered provider. */
export function getOrg(): string | null {
  if (!globalOrgProvider) {
    return null;
  }
  return globalOrgProvider();
}
