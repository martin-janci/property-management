/**
 * Shared authenticated `fetch` wrapper for hooks that don't go through the
 * per-module `Api*` classes.
 *
 * Centralizing this here:
 *   - injects the Authorization header via the registered `tokenProvider`,
 *     instead of re-implementing the header logic in each hooks module
 *     (which was the criticism in #486),
 *   - normalises HTTP error → `Error` with a human-readable message,
 *   - handles 204 No Content uniformly,
 *   - intercepts `401 { error: "mfa_required" }` and delegates to the
 *     registered MFA challenge handler (see `./mfa-handler.ts`); on success
 *     the original request is retried exactly once — matching the behaviour
 *     of the former `admin/api.ts::apiRequest` so any module using this
 *     factory benefits from the same MFA flow without re-implementing it.
 *
 * Hooks should import { authenticatedFetchJson } from '../lib/fetch'.
 */

import { applyAuthHeaders, getToken } from '../auth';
import { client } from '../generated/client.gen';
import { requestMfaChallenge } from './mfa-handler';

/**
 * Error thrown by `authenticatedFetchJson` on a non-2xx response.
 *
 * Carries the HTTP `status` (and the server-provided `error` code, when present)
 * as first-class fields so callers can branch on them — e.g. mapping `403` to a
 * forbidden notice or `401` to a `/login` redirect. Before this existed the
 * helper threw a plain `Error`, so the status was lost and any status-based
 * routing downstream silently became dead code.
 */
export class ApiError extends Error {
  /** HTTP status code of the failed response. */
  readonly status: number;
  /** Machine-readable `error` field from the response body, if any. */
  readonly code?: string;

  constructor(status: number, message: string, code?: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
    // Restore prototype chain for `instanceof` across transpile targets.
    Object.setPrototypeOf(this, ApiError.prototype);
  }
}

/**
 * Header set for the JSON path. Deliberately injects ONLY `Authorization` and
 * NOT `X-Tenant-ID` — see #3006. The raw-`Response` primitive and the generated
 * client interceptor both send `X-Tenant-ID` too (via the shared
 * `applyAuthHeaders`), but the JSON helper has shipped Authorization-only since
 * it was introduced and its admin/notifications callers depend on that; adding
 * the tenant header here is a behavior change to make deliberately on its own,
 * not silently as part of this de-duplication. Pinned by a test in `fetch.test.ts`.
 */
function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

/**
 * Single-retry MFA decision shared by both fetch primitives (#3006).
 *
 * The api-server answers a capability-gated request that needs a recent MFA
 * verification with `401 { error: "mfa_required" }`. Both primitives detect
 * that marker — the JSON path from the body it has already parsed, the raw path
 * from a peeked clone — and then prompt the registered MFA handler exactly once.
 * Centralizing the decision here keeps the status check, the retry bound
 * (`alreadyRetried`), the marker key, and the handler call identical across
 * both, so changing any of them happens in one place. Callers pass the parsed
 * marker because they read the body differently; `errorMarker` is `undefined`
 * when the body was absent or unparsable.
 *
 * Returns `true` only when the caller should retry (a `mfa_required` 401 that
 * has not already been retried and whose handler resolved successfully).
 */
async function shouldRetryOnMfa401(
  status: number,
  alreadyRetried: boolean,
  errorMarker: string | undefined
): Promise<boolean> {
  if (status !== 401 || alreadyRetried || errorMarker !== 'mfa_required') {
    return false;
  }
  return requestMfaChallenge();
}

/**
 * Internal retry-aware fetch implementation. The `alreadyRetried` flag prevents
 * unbounded modal loops if the server keeps returning 401 after a successful
 * MFA round-trip. Kept private so it does not leak onto the public surface.
 */
async function fetchJsonInner<T>(
  url: string,
  init: RequestInit | undefined,
  alreadyRetried: boolean
): Promise<T> {
  const response = await fetch(url, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
      ...init?.headers,
    },
  });
  if (!response.ok) {
    // Read the body once — a 401 mfa_required carries the marker we need and
    // we still want it for the error message in the fall-through case.
    const err = (await response.json().catch(() => ({}))) as {
      code?: string;
      error?: string;
      message?: string;
    };

    if (await shouldRetryOnMfa401(response.status, alreadyRetried, err?.error)) {
      return fetchJsonInner<T>(url, init, true);
    }

    // The backend `ErrorResponse` carries the machine-readable code in `code`
    // (see backend/crates/common/src/errors.rs); `error` only ever holds the
    // legacy `mfa_required` marker. Preferring `code` here (issue #2403) means
    // callers migrated onto this shared helper still get a real `ApiError.code`
    // to map to a localized message, instead of `undefined`.
    throw new ApiError(
      response.status,
      err.message || err.code || err.error || `HTTP ${response.status}`,
      err.code ?? err.error
    );
  }
  if (response.status === 204) return undefined as unknown as T;
  return response.json() as Promise<T>;
}

/**
 * Fetch the given URL, automatically attaching the bearer token from the
 * registered token provider, and parsing the JSON response.
 *
 * Throws `ApiError` (carrying the HTTP `status` and, when present, the server
 * `error` code plus its `message`) on non-2xx responses. Returns
 * `undefined as unknown as T` for 204.
 *
 * When the server responds `401 { error: "mfa_required" }` and a handler has
 * been registered via `setMfaChallengeHandler`, the MFA modal is shown and
 * the request is retried once on success.
 */
export async function authenticatedFetchJson<T>(url: string, init?: RequestInit): Promise<T> {
  return fetchJsonInner<T>(url, init, false);
}

/**
 * Resolve a request path against the generated client's configured base URL
 * (set once in app bootstrap via `client.setConfig({ baseUrl })`), so a
 * raw-`fetch` caller hits the SAME origin as every generated `@ppt/api-client`
 * SDK call. Absolute URLs (with a scheme) pass through unchanged.
 */
function resolveApiUrl(path: string): string {
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(path)) {
    return path;
  }
  const baseUrl = client.getConfig().baseUrl ?? '';
  return `${baseUrl}${path}`;
}

/**
 * Internal retry-aware raw-fetch implementation shared by `authenticatedFetch`.
 * Mirrors `fetchJsonInner`'s single-retry MFA flow, but returns the untouched
 * `Response` so the caller can read the body itself.
 */
async function authenticatedFetchInner(
  path: string,
  init: RequestInit | undefined,
  alreadyRetried: boolean
): Promise<Response> {
  const headers = new Headers(init?.headers);
  // Fill Authorization + X-Tenant-ID from the shared providers via the single
  // source of truth (`auth/apply-headers.ts`), the same helper the generated
  // client's request interceptor uses — so the two raw paths cannot drift
  // (#3006). We never force a `Content-Type`, so multipart/FormData bodies keep
  // the browser-generated boundary.
  applyAuthHeaders(headers);

  const response = await fetch(resolveApiUrl(path), { ...init, headers });

  if (response.status === 401 && !alreadyRetried) {
    // Peek at a clone so the caller still gets a readable body on the final
    // response when this is a non-MFA 401.
    const marker = (await response
      .clone()
      .json()
      .catch(() => ({}))) as { error?: string };
    if (await shouldRetryOnMfa401(response.status, alreadyRetried, marker?.error)) {
      return authenticatedFetchInner(path, init, true);
    }
  }

  return response;
}

/**
 * Shared authenticated `fetch` that returns the raw `Response`.
 *
 * Unlike `authenticatedFetchJson`, it does not force a JSON `Content-Type`,
 * parse the body, or throw on non-2xx — so it fits multipart uploads and
 * endpoints whose callers read the `Response` themselves. It draws
 * Authorization + X-Tenant-ID from the same token/org providers as the
 * generated SDK, resolves the same configured base URL, and applies the same
 * `401 { error: "mfa_required" }` retry-once flow.
 *
 * Callers pass a path (resolved against the client base URL) or an absolute
 * URL, and check `response.ok` themselves.
 */
export function authenticatedFetch(path: string, init?: RequestInit): Promise<Response> {
  return authenticatedFetchInner(path, init, false);
}
