/**
 * Centralized auth interceptor for the generated `@hey-api` client (#1522).
 *
 * After the platform-wide TypeSpec change that made the per-operation
 * `Authorization` / `X-Tenant-ID` headers optional, the generated client no
 * longer forces callers to supply them. Registering this interceptor once at
 * app init injects both from the registered token/org providers, so features
 * that call the client directly (accounting, future migrations) stop
 * hand-rolling auth (and lose the brittle `as unknown as` header casts).
 *
 * It only fills a header that is NOT already set, so an explicit per-request
 * header (e.g. a deliberate cross-org `X-Tenant-ID`) still wins, and
 * unauthenticated requests (no token / no active org) are unchanged.
 */

import { applyAuthHeaders } from './apply-headers';

/**
 * Minimal structural shape of the generated client's request-interceptor
 * registry — avoids coupling this module to the generated `Client` type while
 * still accepting the real `client` instance.
 */
export interface AuthInterceptorClient {
  interceptors: {
    request: {
      use: (fn: (request: Request) => Request | Promise<Request>) => unknown;
    };
  };
}

/**
 * Register the auth request-interceptor on the given client. Call exactly once
 * during app initialization, after `client.setConfig(...)`.
 *
 * The header injection lives in `applyAuthHeaders` (`./apply-headers.ts`), the
 * single source of truth shared with the raw-`fetch` primitive (#3006), so this
 * path cannot drift from it.
 */
export function registerAuthInterceptors(client: AuthInterceptorClient): void {
  client.interceptors.request.use((request) => {
    applyAuthHeaders(request.headers);
    return request;
  });
}
