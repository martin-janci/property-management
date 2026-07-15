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

import { getToken } from '../auth';
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

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
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
      error?: string;
      message?: string;
    };

    if (response.status === 401 && err?.error === 'mfa_required' && !alreadyRetried) {
      const ok = await requestMfaChallenge();
      if (ok) {
        return fetchJsonInner<T>(url, init, true);
      }
    }

    throw new ApiError(
      response.status,
      err.message || err.error || `HTTP ${response.status}`,
      err.error
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
