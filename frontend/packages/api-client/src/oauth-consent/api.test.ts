/**
 * Unit tests for the OAuth consent API client (Epic 10A).
 *
 * Pins the request shaping that the backend's `AuthorizeQuery` / `ConsentForm`
 * extractors require:
 *   - GET sends snake_case query params and omits unset optionals.
 *   - POST is application/x-www-form-urlencoded (NOT the default JSON), carries
 *     the `consent` decision, and round-trips PKCE + state.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearTokenProvider, setTokenProvider } from '../auth/token-provider';
import { setMfaChallengeHandler } from '../lib/mfa-handler';
import { fetchConsentPageData, submitConsent } from './api';
import type { AuthorizeRequestParams } from './types';

function mockOkResponse(body: unknown, status = 200): Response {
  return { ok: true, status, json: async () => body } as Response;
}

const baseParams: AuthorizeRequestParams = {
  responseType: 'code',
  clientId: 'client-abc',
  redirectUri: 'https://app.example.com/cb',
  scope: 'profile email',
  state: 'xyz',
  codeChallenge: 'chal',
  codeChallengeMethod: 'S256',
};

describe('oauth-consent api', () => {
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

  it('fetchConsentPageData GETs with snake_case query params', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      mockOkResponse({
        clientId: 'client-abc',
        clientName: 'Example App',
        clientDescription: null,
        scopes: [{ name: 'profile', description: 'Your profile' }],
        redirectUri: 'https://app.example.com/cb',
        state: 'xyz',
      })
    );

    const data = await fetchConsentPageData(baseParams);
    expect(data.clientName).toBe('Example App');

    const [url, init] = vi.mocked(fetch).mock.calls[0];
    expect(init?.method).toBe('GET');
    const parsed = new URL(String(url), 'http://localhost');
    expect(parsed.searchParams.get('response_type')).toBe('code');
    expect(parsed.searchParams.get('client_id')).toBe('client-abc');
    expect(parsed.searchParams.get('redirect_uri')).toBe('https://app.example.com/cb');
    expect(parsed.searchParams.get('scope')).toBe('profile email');
    expect(parsed.searchParams.get('code_challenge')).toBe('chal');
    expect(parsed.searchParams.get('code_challenge_method')).toBe('S256');
  });

  it('fetchConsentPageData omits unset optional params from the query', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mockOkResponse({}));
    await fetchConsentPageData({
      responseType: 'code',
      clientId: 'c',
      redirectUri: 'https://x/cb',
    });
    const [url] = vi.mocked(fetch).mock.calls[0];
    const parsed = new URL(String(url), 'http://localhost');
    expect(parsed.searchParams.has('scope')).toBe(false);
    expect(parsed.searchParams.has('state')).toBe(false);
    expect(parsed.searchParams.has('code_challenge')).toBe(false);
  });

  it('submitConsent POSTs as form-urlencoded with the decision', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mockOkResponse({ code: 'auth-code', state: 'xyz' }));

    const res = await submitConsent(baseParams, 'approve');
    expect(res.code).toBe('auth-code');

    const [, init] = vi.mocked(fetch).mock.calls[0];
    expect(init?.method).toBe('POST');
    const headers = (init?.headers ?? {}) as Record<string, string>;
    expect(headers['Content-Type']).toBe('application/x-www-form-urlencoded');

    const body = new URLSearchParams(String(init?.body));
    expect(body.get('client_id')).toBe('client-abc');
    expect(body.get('redirect_uri')).toBe('https://app.example.com/cb');
    expect(body.get('scope')).toBe('profile email');
    expect(body.get('state')).toBe('xyz');
    expect(body.get('code_challenge')).toBe('chal');
    expect(body.get('consent')).toBe('approve');
  });

  it('submitConsent sends consent=deny on a deny decision', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mockOkResponse({ code: '', state: null }));
    await submitConsent(baseParams, 'deny');
    const [, init] = vi.mocked(fetch).mock.calls[0];
    const body = new URLSearchParams(String(init?.body));
    expect(body.get('consent')).toBe('deny');
  });
});
