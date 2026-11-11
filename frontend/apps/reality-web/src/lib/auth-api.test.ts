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
vi.mock('./auth-token', () => ({
  getAuthHeader: () => ({}),
  setSession: (...args: unknown[]) => setSession(...args),
  clearSession: vi.fn(),
}));

import { AuthApiError, login, register } from './auth-api';

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
