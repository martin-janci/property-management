/**
 * Reality Portal direct auth API (UC-47).
 *
 * Thin fetch wrappers around `/api/v1/auth/*` endpoints exposed by
 * reality-server. Used by the email/password auth pages alongside the
 * existing SSO flow in `auth-context.tsx`.
 */

const API_BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081';

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
    response = await fetch(`${API_BASE}${path}`, {
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

export interface LoginResponse {
  accessToken: string;
  refreshToken: string;
  user: {
    id: string;
    email: string;
    displayName: string;
  };
}

export async function login(email: string, password: string): Promise<LoginResponse> {
  return postJson<LoginResponse>('/api/v1/auth/login', { email, password });
}

export async function register(input: {
  email: string;
  password: string;
  displayName: string;
}): Promise<{ id: string; email: string; displayName: string }> {
  return postJson('/api/v1/auth/register', input);
}

export async function requestPasswordReset(email: string): Promise<void> {
  await postJson<void>('/api/v1/auth/password-reset', { email });
}

export async function confirmPasswordReset(token: string, newPassword: string): Promise<void> {
  await postJson<void>('/api/v1/auth/password-reset/confirm', { token, newPassword });
}

export async function changePassword(currentPassword: string, newPassword: string): Promise<void> {
  await postJson<void>('/api/v1/auth/me/password', { currentPassword, newPassword });
}

export async function updateProfile(input: { displayName: string }): Promise<{
  id: string;
  email: string;
  displayName: string;
}> {
  return requestJson('PATCH', '/api/v1/auth/me', input);
}
