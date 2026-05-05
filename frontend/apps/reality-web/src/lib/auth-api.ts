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
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
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
  return (await response.json()) as T;
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
  return postJson<LoginResponse>('/api/v1/users/login', { email, password });
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
