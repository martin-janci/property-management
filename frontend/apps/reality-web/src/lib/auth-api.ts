/**
 * Reality Portal direct auth API (UC-47).
 *
 * Thin fetch wrappers around `/api/v1/users/*` endpoints exposed by
 * reality-server (see `backend/servers/reality-server/src/routes/users.rs`).
 * Used by the email/password auth pages alongside the existing SSO flow in
 * `auth-context.tsx`.
 *
 * Note: reality-server does not expose password-reset endpoints yet. The
 * `requestPasswordReset` / `confirmPasswordReset` / `changePassword`
 * helpers throw an informative `AuthApiError` until the backend ships.
 */

import { clearSession, getAuthHeader, setSession } from './auth-token';
import { getApiBase } from './env';

export class AuthApiError extends Error {
  constructor(
    message: string,
    public readonly status: number,
    public readonly code?: string
  ) {
    super(message);
    this.name = 'AuthApiError';
  }
}

async function postJson<T>(path: string, body: unknown): Promise<T> {
  return requestJson<T>('POST', path, body);
}

async function requestJson<T>(method: string, path: string, body?: unknown): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`${getApiBase()}${path}`, {
      method,
      // credentials:'include' kept for the cookie/SSO path; bearer header is
      // additive — reality-server's extractor checks Authorization first then
      // falls back to cookie, so both flows can coexist.
      credentials: 'include',
      headers: {
        'Content-Type': 'application/json',
        ...getAuthHeader(),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new AuthApiError('Network error. Please check your connection.', 0, 'NETWORK_ERROR');
  }

  if (!response.ok) {
    let message = `Request failed with status ${response.status}`;
    let code: string | undefined;
    try {
      const data = await response.json();
      if (typeof data?.message === 'string') message = data.message;
      if (typeof data?.code === 'string') code = data.code;
    } catch {
      // Non-JSON error body; fall through with default message.
    }
    throw new AuthApiError(message, response.status, code);
  }

  if (response.status === 204) {
    return undefined as T;
  }
  return parseJsonBody<T>(response);
}

/**
 * Validate + return the parsed success body.
 *
 * `requestJson<T>` used to return `(await response.json()) as T` — an unchecked
 * cast that trusts the wire blindly. A malformed or partial 2xx (an
 * upstream/proxy truncation, an HTML error page served with a 200, or a
 * `null`/primitive/array JSON body) then either threw a raw `SyntaxError` that
 * no caller catches via `instanceof AuthApiError`, or was handed back typed as
 * `T` so the first deref (`result.token`, `result.user`) crashed deep in a
 * component with no hint the API boundary was the culprit.
 *
 * Every endpoint this module wraps returns a JSON *object*, so the boundary
 * invariant is "the body is a non-null, non-array object". Guarding it here —
 * the single point all helpers funnel through — mirrors `realtor-api.ts`'s
 * `parseJsonBody`: reject the bad shape at the edge with a typed `AuthApiError`
 * (so the auth pages' existing `instanceof AuthApiError` handling shows a clean
 * message) instead of letting it crash downstream.
 */
async function parseJsonBody<T>(response: Response): Promise<T> {
  let body: unknown;
  try {
    body = await response.json();
  } catch {
    throw new AuthApiError(
      'Malformed response from server.',
      response.status,
      'MALFORMED_RESPONSE'
    );
  }
  if (!body || typeof body !== 'object' || Array.isArray(body)) {
    throw new AuthApiError(
      'Malformed response from server.',
      response.status,
      'MALFORMED_RESPONSE'
    );
  }
  return body as T;
}

/**
 * User info returned by reality-server on login and `/users/me`.
 *
 * Matches the server contract exactly (snake_case wire fields are kept
 * as-is so the wrapper stays a pure pass-through).
 */
export interface PortalUser {
  id: string;
  email: string;
  name: string;
  profile_image_url?: string | null;
  is_linked_to_pm?: boolean;
}

/** Login response shape from `POST /api/v1/users/login`. */
export interface LoginResponse {
  token: string;
  expires_at: string;
  user: PortalUser;
}

export async function login(email: string, password: string): Promise<LoginResponse> {
  const result = await postJson<LoginResponse>('/api/v1/users/login', { email, password });
  // Persist so the auth-context picks up the user on next render and so
  // subsequent fetch() calls include the bearer token via getAuthHeader().
  setSession({
    token: result.token,
    user: result.user,
    expiresAt: result.expires_at,
  });
  return result;
}

/** Clears the stored portal session. Used by auth-context.logout(). */
export function clearStoredSession(): void {
  clearSession();
}

/**
 * Registers a new portal user via `POST /api/v1/users/register`.
 *
 * Takes `name` (the server's field) rather than `displayName`.
 */
export async function register(input: {
  email: string;
  password: string;
  name: string;
}): Promise<{ message: string; user_id: string }> {
  return postJson('/api/v1/users/register', input);
}

export async function requestPasswordReset(_email: string): Promise<void> {
  // TODO: wire when reality-server exposes /api/v1/users/password-reset.
  throw new AuthApiError(
    'Password reset is not available yet. Please contact support.',
    501,
    'NOT_IMPLEMENTED'
  );
}

export async function confirmPasswordReset(_token: string, _newPassword: string): Promise<void> {
  // TODO: wire when reality-server exposes /api/v1/users/password-reset/confirm.
  throw new AuthApiError(
    'Password reset is not available yet. Please contact support.',
    501,
    'NOT_IMPLEMENTED'
  );
}

export async function changePassword(
  _currentPassword: string,
  _newPassword: string
): Promise<void> {
  // TODO: wire when reality-server exposes /api/v1/users/me/password.
  throw new AuthApiError(
    'Changing your password from the portal is not available yet. Please contact support.',
    501,
    'NOT_IMPLEMENTED'
  );
}

/**
 * Updates the signed-in user's profile via `PUT /api/v1/users/me`.
 *
 * Accepts any of `name`, `profile_image_url` or `locale`; callers typically
 * only send `name` from the account UI.
 */
export async function updateProfile(input: {
  name?: string;
  profile_image_url?: string | null;
  locale?: string;
}): Promise<PortalUser> {
  return requestJson<PortalUser>('PUT', '/api/v1/users/me', input);
}
