/**
 * Unauthorized (401) handler registry for the generated `@hey-api` client.
 *
 * The request path already injects the bearer token (`./interceptors.ts` +
 * `./apply-headers.ts`), but until now nothing recovered a request whose access
 * token had expired mid-session: the generated client surfaced the raw 401 and
 * the only escape was a full re-login. The hand-rolled axios client in
 * `ppt-web/src/lib/api.ts` already did a silent refresh-and-replay on 401; this
 * registry is the generated client's half of the same contract, so features
 * that call the generated SDK directly get the identical recovery.
 *
 * The application (AuthContext) registers a single-flight handler here. On a
 * 401, `registerAuthInterceptors`' response interceptor awaits it and, when it
 * resolves a fresh access token, replays the failed request once with that
 * token. A `null`/`undefined` result (or a throw) means the session could not
 * be recovered and the original 401 is allowed to surface — the handler owns any
 * teardown (logout / redirect).
 *
 * Mirrors the `token-provider` / `org-provider` registry pattern so there is a
 * single, swappable source of truth the API modules depend on instead of
 * reaching into React state directly.
 */

/**
 * Handler invoked when a request comes back 401.
 *
 * Expected to perform a *single-flight* token refresh (so concurrent 401s share
 * one refresh round-trip) and resolve with the rotated access token, or
 * `null`/`undefined` when the session cannot be recovered.
 */
export type UnauthorizedHandler = () => Promise<string | null | undefined>;

/**
 * Global unauthorized handler instance. Registered once by the app during init;
 * `null` when unset (e.g. unauthenticated boot), in which case a 401 surfaces
 * unchanged.
 */
let globalUnauthorizedHandler: UnauthorizedHandler | null = null;

/**
 * Set the global unauthorized handler. Call once during app initialization with
 * the AuthContext's single-flight refresh routine.
 */
export function setUnauthorizedHandler(handler: UnauthorizedHandler): void {
  globalUnauthorizedHandler = handler;
}

/**
 * Clear the global unauthorized handler. Call on logout / when the auth context
 * unmounts, so a 401 after teardown surfaces instead of attempting a refresh.
 */
export function clearUnauthorizedHandler(): void {
  globalUnauthorizedHandler = null;
}

/**
 * Get the current unauthorized handler, or `null` when none is registered.
 */
export function getUnauthorizedHandler(): UnauthorizedHandler | null {
  return globalUnauthorizedHandler;
}

/**
 * Whether an unauthorized handler is registered. Useful for debugging / tests.
 */
export function hasUnauthorizedHandler(): boolean {
  return globalUnauthorizedHandler !== null;
}
