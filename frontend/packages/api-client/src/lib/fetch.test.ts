/**
 * Unit tests for `authenticatedFetchJson` — the shared MFA-aware fetch factory.
 *
 * Regression coverage for PR #471 reviewer issue: health-page API calls were
 * using a bespoke inline fetchJson() that did NOT intercept
 * `401 { error: "mfa_required" }`, so the MFA modal was never shown. The fix
 * consolidated all API calls through this factory (lib/fetch.ts); these tests
 * pin that the factory handles every 401/MFA scenario correctly.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearTokenProvider, setTokenProvider } from '../auth/token-provider';
import { ApiError, authenticatedFetchJson } from './fetch';
import { setMfaChallengeHandler } from './mfa-handler';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mockOkResponse(body: unknown, status = 200): Response {
  return {
    ok: true,
    status,
    json: async () => body,
  } as Response;
}

function mockErrResponse(status: number, body: unknown): Response {
  return {
    ok: false,
    status,
    json: async () => body,
  } as Response;
}

function mock204Response(): Response {
  return {
    ok: true,
    status: 204,
    json: async () => {
      throw new Error('no body');
    },
  } as unknown as Response;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('authenticatedFetchJson', () => {
  beforeEach(() => {
    setTokenProvider(() => 'test-token');
    setMfaChallengeHandler(null);
    vi.spyOn(globalThis, 'fetch');
  });

  afterEach(() => {
    clearTokenProvider();
    setMfaChallengeHandler(null);
    vi.restoreAllMocks();
  });

  it('returns parsed JSON on 200', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mockOkResponse({ id: 1 }));
    const result = await authenticatedFetchJson<{ id: number }>('/api/v1/test');
    expect(result).toEqual({ id: 1 });
  });

  it('returns undefined on 204 No Content', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mock204Response());
    const result = await authenticatedFetchJson<void>('/api/v1/test');
    expect(result).toBeUndefined();
  });

  it('injects Authorization header from token provider', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mockOkResponse({}));
    await authenticatedFetchJson('/api/v1/test');
    const called = vi.mocked(fetch).mock.calls[0];
    const headers = (called[1] as RequestInit).headers as Record<string, string>;
    expect(headers.Authorization).toBe('Bearer test-token');
  });

  it('does not set Authorization header when no token is registered', async () => {
    clearTokenProvider();
    vi.mocked(fetch).mockResolvedValueOnce(mockOkResponse({}));
    await authenticatedFetchJson('/api/v1/test');
    const called = vi.mocked(fetch).mock.calls[0];
    const headers = (called[1] as RequestInit).headers as Record<string, string>;
    expect(headers.Authorization).toBeUndefined();
  });

  it('throws an Error with server message on non-2xx responses', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      mockErrResponse(403, { message: 'Forbidden', error: 'forbidden' })
    );
    await expect(authenticatedFetchJson('/api/v1/test')).rejects.toThrow('Forbidden');
  });

  it('throws HTTP <status> when server body has no message', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mockErrResponse(500, {}));
    await expect(authenticatedFetchJson('/api/v1/test')).rejects.toThrow('HTTP 500');
  });

  it('throws an ApiError carrying the HTTP status and error code (403)', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      mockErrResponse(403, { message: 'Forbidden', error: 'forbidden' })
    );
    const err = (await authenticatedFetchJson('/api/v1/test').catch((e) => e)) as ApiError;
    expect(err).toBeInstanceOf(ApiError);
    expect(err).toBeInstanceOf(Error);
    expect(err.status).toBe(403);
    expect(err.code).toBe('forbidden');
    expect(err.message).toBe('Forbidden');
  });

  // issue #2403: the backend `ErrorResponse` carries its machine-readable code
  // in `code` (not `error`, which only holds the `mfa_required` marker). The
  // helper must expose that `code` on `ApiError.code` so callers can map it to a
  // localized message — before the fix it read `err.error` and lost the code.
  it('exposes the ErrorResponse `code` field on ApiError.code', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      mockErrResponse(400, { code: 'TOO_MANY_RECIPIENTS', message: 'Too many recipients' })
    );
    const err = (await authenticatedFetchJson('/api/v1/test').catch((e) => e)) as ApiError;
    expect(err).toBeInstanceOf(ApiError);
    expect(err.status).toBe(400);
    expect(err.code).toBe('TOO_MANY_RECIPIENTS');
    expect(err.message).toBe('Too many recipients');
  });

  it('propagates a 401 status on ApiError (session expired, non-mfa)', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      mockErrResponse(401, { error: 'unauthorized', message: 'Unauthorized' })
    );
    const err = (await authenticatedFetchJson('/api/v1/test').catch((e) => e)) as ApiError;
    expect(err).toBeInstanceOf(ApiError);
    expect(err.status).toBe(401);
    expect(err.code).toBe('unauthorized');
  });

  // ---------- MFA interception --------------------------------------------

  it('surfaces 401 mfa_required as error when no handler is registered', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      mockErrResponse(401, { error: 'mfa_required', message: 'MFA required' })
    );
    await expect(authenticatedFetchJson('/api/v1/test')).rejects.toThrow();
  });

  it('shows MFA modal and retries on 401 mfa_required when handler returns true', async () => {
    const mockHandler = vi.fn().mockResolvedValue(true);
    setMfaChallengeHandler(mockHandler);

    vi.mocked(fetch)
      .mockResolvedValueOnce(mockErrResponse(401, { error: 'mfa_required' }))
      .mockResolvedValueOnce(mockOkResponse({ ok: true }));

    const result = await authenticatedFetchJson<{ ok: boolean }>('/api/v1/health/dashboard');
    expect(mockHandler).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(result).toEqual({ ok: true });
  });

  it('throws after MFA modal is cancelled (handler returns false)', async () => {
    const mockHandler = vi.fn().mockResolvedValue(false);
    setMfaChallengeHandler(mockHandler);

    vi.mocked(fetch).mockResolvedValueOnce(
      mockErrResponse(401, { error: 'mfa_required', message: 'MFA required' })
    );

    await expect(authenticatedFetchJson('/api/v1/health/dashboard')).rejects.toThrow();
    expect(mockHandler).toHaveBeenCalledTimes(1);
    // Must NOT retry when user cancelled
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it('does not retry a second time on repeated 401 after successful MFA (no modal loop)', async () => {
    const mockHandler = vi.fn().mockResolvedValue(true);
    setMfaChallengeHandler(mockHandler);

    // Both calls return 401 mfa_required (server bug / misconfiguration)
    vi.mocked(fetch)
      .mockResolvedValueOnce(mockErrResponse(401, { error: 'mfa_required' }))
      .mockResolvedValueOnce(mockErrResponse(401, { error: 'mfa_required' }));

    await expect(authenticatedFetchJson('/api/v1/health/dashboard')).rejects.toThrow();
    // Handler called once, fetch called twice (original + one retry), no further retries
    expect(mockHandler).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it('throws without calling handler on non-mfa 401', async () => {
    const mockHandler = vi.fn().mockResolvedValue(true);
    setMfaChallengeHandler(mockHandler);

    vi.mocked(fetch).mockResolvedValueOnce(
      mockErrResponse(401, { error: 'unauthorized', message: 'Unauthorized' })
    );

    await expect(authenticatedFetchJson('/api/v1/health/dashboard')).rejects.toThrow(
      'Unauthorized'
    );
    expect(mockHandler).not.toHaveBeenCalled();
  });
});
