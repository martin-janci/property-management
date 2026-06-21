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

import { getOrg } from './org-provider';
import { getToken } from './token-provider';

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
 */
export function registerAuthInterceptors(client: AuthInterceptorClient): void {
  client.interceptors.request.use((request) => {
    if (!request.headers.has('Authorization')) {
      const token = getToken();
      if (token) {
        request.headers.set('Authorization', `Bearer ${token}`);
      }
    }
    if (!request.headers.has('X-Tenant-ID')) {
      const org = getOrg();
      if (org) {
        request.headers.set('X-Tenant-ID', org);
      }
    }
    return request;
  });
}
