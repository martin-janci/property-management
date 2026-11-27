/**
 * Regression test for the unguarded success-body parse in `auth-api.ts`.
 *
 * `requestJson<T>` used to end with `return (await response.json()) as T` — an
 * unchecked cast that trusts the wire blindly. A malformed or partial 2xx (an
 * upstream/proxy truncation, an HTML error page served with a 200, or a
 * `null`/primitive/array JSON body) then either threw a raw `SyntaxError` that
 * no auth page catches via `instanceof AuthApiError`, or was handed back typed
 * as the response type so the first deref (`result.token`, `result.user`)
 * crashed deep in a component.
 *
 * These tests stub `fetch` and assert the helper now rejects a malformed 2xx
 * body with a typed `AuthApiError` (mirroring `realtor-api.ts`'s
 * `parseJsonBody` guard), while still passing a well-formed object through.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Deterministic base URL and no localStorage side effects from the token store.
vi.mock('./env', () => ({ getApiBase: () => '' }));
const setSession = vi.fn();
// Controllable so a test can assert the bearer token is merged into the
// outgoing request headers; defaults to no auth header (anonymous).
const getAuthHeader = vi.fn<() => Record<string, string>>(() => ({}));
vi.mock('./auth-token', () => ({
  getAuthHeader: () => getAuthHeader(),
  setSession: (...args: unknown[]) => setSession(...args),
  clearSession: vi.fn(),
}));

import {
  AuthApiError,
  confirmPasswordReset,
  login,
  register,
  requestPasswordReset,
  updateProfile,
} from './auth-api';

describe('auth-api requestJson success-body guard', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it('rejects a 2xx body that is not valid JSON with a typed AuthApiError', async () => {
    // The core regression: before the guard this threw a raw SyntaxError, which
    // the auth pages' `instanceof AuthApiError` handling never catches.
    fetchMock.mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => {
        throw new SyntaxError('Unexpected token < in JSON');
      },
    });
    const err = await register({
      email: 'e@example.com',
      password: 'pw',
      name: 'Eva',
    }).catch((e) => e);
    expect(err).toBeInstanceOf(AuthApiError);
    expect(err.code).toBe('MALFORMED_RESPONSE');
    expect(err.status).toBe(200);
  });

  it('rejects a null 2xx body as a malformed response', async () => {
    fetchMock.mockResolvedValue({ ok: true, status: 200, json: async () => null });
    await expect(
      register({ email: 'e@example.com', password: 'pw', name: 'Eva' })
    ).rejects.toMatchObject({ name: 'AuthApiError', code: 'MALFORMED_RESPONSE' });
  });

  it('rejects a primitive 2xx body as a malformed response', async () => {
    fetchMock.mockResolvedValue({ ok: true, status: 200, json: async () => 'oops' });
    await expect(
      register({ email: 'e@example.com', password: 'pw', name: 'Eva' })
    ).rejects.toBeInstanceOf(AuthApiError);
  });

  it('rejects an array 2xx body as a malformed response', async () => {
    fetchMock.mockResolvedValue({ ok: true, status: 200, json: async () => [] });
    await expect(
      register({ email: 'e@example.com', password: 'pw', name: 'Eva' })
    ).rejects.toBeInstanceOf(AuthApiError);
  });

  it('preserves success behaviour: a well-formed login object passes through and sets the session', async () => {
    const body = {
      token: 'tok_1',
      expires_at: '2026-01-01T00:00:00Z',
      user: { id: 'u1', email: 'e@example.com', name: 'Eva' },
    };
    fetchMock.mockResolvedValue({ ok: true, status: 200, json: async () => body });
    await expect(login('e@example.com', 'pw')).resolves.toEqual(body);
    expect(setSession).toHaveBeenCalledWith({
      token: 'tok_1',
      user: body.user,
      expiresAt: '2026-01-01T00:00:00Z',
    });
  });
});

describe('auth-api password reset', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ message: 'ok' }),
    });
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  // Both helpers used to throw a local 501 without calling the server, although
  // reality-server implements the routes.
  it('requests a reset link from POST /users/password-reset', async () => {
    await requestPasswordReset('eva@example.com');

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('/api/v1/users/password-reset');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body)).toEqual({ email: 'eva@example.com' });
  });

  it('confirms with the snake_case body the server deserializes', async () => {
    await confirmPasswordReset('tok-1', 'N3w-password!');

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('/api/v1/users/password-reset/confirm');
    expect(JSON.parse(init.body)).toEqual({ token: 'tok-1', new_password: 'N3w-password!' });
  });
});

// The non-ok / network / 204 branches of `requestJson` carry no coverage, yet
// they are exactly what the recent churn (i18n error copy, gap-audit wiring)
// kept editing. Lock in the error-surface contract the auth pages depend on:
// a typed `AuthApiError` with the right `status` / `code` / `serverMessage`.
describe('auth-api error + transport surface', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
    getAuthHeader.mockReturnValue({});
  });

  it('maps a JSON error body to AuthApiError carrying status, code and serverMessage', async () => {
    fetchMock.mockResolvedValue({
      ok: false,
      status: 401,
      json: async () => ({ message: 'Invalid email or password.', code: 'INVALID_CREDENTIALS' }),
    });
    const err = await login('e@example.com', 'bad').catch((e) => e);
    expect(err).toBeInstanceOf(AuthApiError);
    expect(err.status).toBe(401);
    expect(err.code).toBe('INVALID_CREDENTIALS');
    // UI surfaces translate off `code`, falling back to the backend-owned copy.
    expect(err.serverMessage).toBe('Invalid email or password.');
    // A failed login must never persist a session.
    expect(setSession).not.toHaveBeenCalled();
  });

  it('falls back to a default message (no code/serverMessage) when the error body is not JSON', async () => {
    // e.g. a 502 HTML page from an upstream proxy.
    fetchMock.mockResolvedValue({
      ok: false,
      status: 502,
      json: async () => {
        throw new SyntaxError('Unexpected token < in JSON');
      },
    });
    const err = await register({ email: 'e@example.com', password: 'pw', name: 'Eva' }).catch(
      (e) => e
    );
    expect(err).toBeInstanceOf(AuthApiError);
    expect(err.status).toBe(502);
    expect(err.message).toBe('Request failed with status 502');
    expect(err.code).toBeUndefined();
    expect(err.serverMessage).toBeUndefined();
  });

  it('maps a fetch rejection (offline/DNS) to a NETWORK_ERROR with status 0', async () => {
    fetchMock.mockRejectedValue(new TypeError('Failed to fetch'));
    const err = await requestPasswordReset('eva@example.com').catch((e) => e);
    expect(err).toBeInstanceOf(AuthApiError);
    expect(err.status).toBe(0);
    expect(err.code).toBe('NETWORK_ERROR');
  });

  it('resolves a 204 No Content response as undefined without reading the body', async () => {
    const json = vi.fn(async () => {
      throw new Error('body must not be read on 204');
    });
    fetchMock.mockResolvedValue({ ok: true, status: 204, json });
    await expect(updateProfile({ name: 'Eva' })).resolves.toBeUndefined();
    expect(json).not.toHaveBeenCalled();
  });

  it('merges the stored bearer token into the request headers alongside the JSON defaults', async () => {
    getAuthHeader.mockReturnValue({ Authorization: 'Bearer tok_1' });
    fetchMock.mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ id: 'u1', email: 'e@example.com', name: 'Eva' }),
    });
    await updateProfile({ name: 'Eva' });

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('/api/v1/users/me');
    expect(init.method).toBe('PUT');
    // credentials:'include' covers the SSO cookie flow; the bearer header is the
    // additive form-login path — both must survive on every authed request.
    expect(init.credentials).toBe('include');
    expect(init.headers.Authorization).toBe('Bearer tok_1');
    expect(init.headers['Content-Type']).toBe('application/json');
  });
});
