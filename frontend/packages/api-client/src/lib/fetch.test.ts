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
import { clearOrgProvider, setOrgProvider } from '../auth/org-provider';
import { clearTokenProvider, setTokenProvider } from '../auth/token-provider';
import { client } from '../generated/client.gen';
import { ApiError, authenticatedFetch, authenticatedFetchJson } from './fetch';
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

// ---------------------------------------------------------------------------
// authenticatedFetch — raw-Response primitive (#3000)
// ---------------------------------------------------------------------------

/**
 * Response mock that supports `.clone()` (needed by the 401 marker peek). The
 * clone shares the same lazily-provided body, which is fine for these tests.
 */
function mockRawResponse(status: number, body: unknown = {}): Response {
  const res = {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
    clone() {
      return res;
    },
  } as unknown as Response;
  return res;
}

describe('authenticatedFetch (raw-Response primitive)', () => {
  const originalBaseUrl = client.getConfig().baseUrl;

  beforeEach(() => {
    setTokenProvider(() => 'test-token');
    setOrgProvider(() => 'org-7');
    setMfaChallengeHandler(null);
    client.setConfig({ baseUrl: '' });
    vi.spyOn(globalThis, 'fetch');
  });

  afterEach(() => {
    clearTokenProvider();
    clearOrgProvider();
    setMfaChallengeHandler(null);
    client.setConfig({ baseUrl: originalBaseUrl });
    vi.restoreAllMocks();
  });

  it('returns the raw Response without throwing on non-2xx', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mockRawResponse(500, { error: 'boom' }));
    const res = await authenticatedFetch('/api/v1/accounting/statements', { method: 'POST' });
    expect(res.ok).toBe(false);
    expect(res.status).toBe(500);
  });

  it('injects Authorization + X-Tenant-ID from the shared providers', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mockRawResponse(200));
    await authenticatedFetch('/api/v1/gdpr/privacy');
    const [, init] = vi.mocked(fetch).mock.calls[0];
    const headers = (init as RequestInit).headers as Headers;
    expect(headers.get('Authorization')).toBe('Bearer test-token');
    expect(headers.get('X-Tenant-ID')).toBe('org-7');
  });

  it('does not force a JSON Content-Type (multipart boundary survives)', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mockRawResponse(200));
    const body = new FormData();
    body.append('file', new File(['x'], 'f.csv'));
    await authenticatedFetch('/api/v1/accounting/statements', { method: 'POST', body });
    const [, init] = vi.mocked(fetch).mock.calls[0];
    const headers = (init as RequestInit).headers as Headers;
    expect(headers.has('Content-Type')).toBe(false);
    expect((init as RequestInit).body).toBeInstanceOf(FormData);
  });

  it('omits auth headers when there is no session', async () => {
    clearTokenProvider();
    clearOrgProvider();
    vi.mocked(fetch).mockResolvedValueOnce(mockRawResponse(200));
    await authenticatedFetch('/api/v1/gdpr/privacy');
    const [, init] = vi.mocked(fetch).mock.calls[0];
    const headers = (init as RequestInit).headers as Headers;
    expect(headers.has('Authorization')).toBe(false);
    expect(headers.has('X-Tenant-ID')).toBe(false);
  });

  it('resolves the path against the generated client base URL', async () => {
    client.setConfig({ baseUrl: 'https://api.example.test' });
    vi.mocked(fetch).mockResolvedValueOnce(mockRawResponse(200));
    await authenticatedFetch('/api/v1/gdpr/privacy');
    const [url] = vi.mocked(fetch).mock.calls[0];
    expect(String(url)).toBe('https://api.example.test/api/v1/gdpr/privacy');
  });

  it('retries once on 401 mfa_required when the handler resolves true', async () => {
    const handler = vi.fn().mockResolvedValue(true);
    setMfaChallengeHandler(handler);
    vi.mocked(fetch)
      .mockResolvedValueOnce(mockRawResponse(401, { error: 'mfa_required' }))
      .mockResolvedValueOnce(mockRawResponse(200));

    const res = await authenticatedFetch('/api/v1/accounting/statements', { method: 'POST' });
    expect(handler).toHaveBeenCalledTimes(1);
    expect(fetch).toHaveBeenCalledTimes(2);
    expect(res.ok).toBe(true);
  });

  it('does not retry a non-mfa 401', async () => {
    const handler = vi.fn().mockResolvedValue(true);
    setMfaChallengeHandler(handler);
    vi.mocked(fetch).mockResolvedValueOnce(mockRawResponse(401, { error: 'unauthorized' }));

    const res = await authenticatedFetch('/api/v1/gdpr/privacy');
    expect(handler).not.toHaveBeenCalled();
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(res.status).toBe(401);
  });
});
