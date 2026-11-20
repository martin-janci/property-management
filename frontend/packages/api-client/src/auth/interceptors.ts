/**
 * Centralized auth interceptors for the generated `@hey-api` client (#1522).
 *
 * After the platform-wide TypeSpec change that made the per-operation
 * `Authorization` / `X-Tenant-ID` headers optional, the generated client no
 * longer forces callers to supply them. Registering these interceptors once at
 * app init injects both from the registered token/org providers, so features
 * that call the client directly (accounting, future migrations) stop
 * hand-rolling auth (and lose the brittle `as unknown as` header casts).
 *
 * It only fills a header that is NOT already set, so an explicit per-request
 * header (e.g. a deliberate cross-org `X-Tenant-ID`) still wins, and
 * unauthenticated requests (no token / no active org) are unchanged.
 *
 * Runtime token refresh (no-runtime-token-refresh fix): the response
 * interceptor recovers a request whose access token expired mid-session. On a
 * 401 it awaits the registered single-flight unauthorized handler and, when a
 * rotated access token comes back, replays the original request exactly once
 * with that token — mirroring the hand-rolled axios client's 401 behaviour in
 * `ppt-web/src/lib/api.ts` so the generated-SDK path no longer forces a
 * re-login on expiry.
 */

import { applyAuthHeaders } from './apply-headers';
import { getUnauthorizedHandler } from './unauthorized-handler';

/** HTTP status that triggers the silent refresh-and-replay. */
const HTTP_UNAUTHORIZED = 401;

/**
 * Minimal structural shape of the per-request options object the generated
 * client threads into both interceptor phases. We only need `fetch` to replay
 * the request; the same object identity is shared between the request and
 * response interceptors for a given request, so it doubles as the replay key.
 */
interface ReplayRequestOptions {
  fetch?: (request: Request) => Promise<Response>;
}

/**
 * Minimal structural shape of the generated client's interceptor registries —
 * avoids coupling this module to the generated `Client` type while still
 * accepting the real `client` instance. `response` is optional so lightweight
 * test fakes that only exercise the request path still satisfy the type.
 */
export interface AuthInterceptorClient {
  interceptors: {
    request: {
      use: (fn: (request: Request, options?: unknown) => Request | Promise<Request>) => unknown;
    };
    response?: {
      use: (
        fn: (
          response: Response,
          request: Request,
          options: ReplayRequestOptions
        ) => Response | Promise<Response>
      ) => unknown;
    };
  };
}

/**
 * Register the auth request + response interceptors on the given client. Call
 * exactly once during app initialization, after `client.setConfig(...)`.
 *
 * The header injection lives in `applyAuthHeaders` (`./apply-headers.ts`), the
 * single source of truth shared with the raw-`fetch` primitive (#3006), so the
 * request path cannot drift from it. The 401 refresh path is wired to the
 * handler registered via `setUnauthorizedHandler` (`./unauthorized-handler.ts`);
 * when no handler is registered, a 401 surfaces unchanged.
 */
export function registerAuthInterceptors(client: AuthInterceptorClient): void {
  // Pre-fetch request clones, keyed by the per-request options object the client
  // threads through both interceptor phases. A clone captured here (before fetch
  // consumes the body) is what the 401 handler replays; a WeakMap keeps it only
  // until the request settles.
  const replayClones = new WeakMap<object, Request>();
  // Replay-once guard: a replayed request that ALSO 401s must surface, never
  // loop. Keyed by the same per-request options object.
  const retried = new WeakSet<object>();

  client.interceptors.request.use((request, options) => {
    applyAuthHeaders(request.headers);
    // Stash a clone for a potential 401 replay. Cloning now (before the body is
    // consumed by fetch) is the only point at which a faithful copy of a
    // request with a body can be taken.
    if (options && typeof options === 'object') {
      try {
        replayClones.set(options, request.clone());
      } catch {
        // Body already disturbed / unclonable — this request just won't be
        // replayable; the 401 surfaces as before.
      }
    }
    return request;
  });

  client.interceptors.response?.use(async (response, _request, options) => {
    if (response.status !== HTTP_UNAUTHORIZED) {
      return response;
    }

    const key = options as unknown;
    if (!key || typeof key !== 'object') {
      return response;
    }
    if (retried.has(key)) {
      // Already replayed once and still 401 — give up, surface the 401.
      return response;
    }

    const handler = getUnauthorizedHandler();
    if (!handler) {
      return response;
    }

    const clone = replayClones.get(key);
    if (!clone) {
      // Nothing to replay (e.g. an unclonable body) — still try to recover the
      // session so a subsequent request succeeds, but surface this 401.
      try {
        await handler();
      } catch {
        // Teardown is the handler's responsibility.
      }
      return response;
    }

    retried.add(key);

    let rotatedToken: string | null | undefined;
    try {
      // Single-flight: concurrent 401s all await the one in-flight refresh
      // inside the handler.
      rotatedToken = await handler();
    } catch {
      // Refresh failed; the handler has torn down the session. Surface the 401.
      return response;
    }

    if (!rotatedToken) {
      // Session could not be recovered — surface the 401; the handler owns any
      // logout / redirect.
      return response;
    }

    // Replay the original request once with the rotated token. Set the header
    // explicitly rather than relying on the request interceptor (which does not
    // run on this manual re-fetch).
    const replay = clone.clone();
    replay.headers.set('Authorization', `Bearer ${rotatedToken}`);

    // Assign to a local before calling — calling `options.fetch(...)` as a
    // method throws "Illegal invocation" for the native fetch (see the same
    // pattern in the generated client).
    const fetchFn = options.fetch ?? globalThis.fetch;
    try {
      return await fetchFn(replay);
    } catch {
      // Network error on replay — surface the original 401.
      return response;
    }
  });
}
