/**
 * Regression tests for the GDPR data layer (#3000).
 *
 * `gdprFetch` must go through the shared `authenticatedFetch` from
 * `@ppt/api-client` — the same authenticated client the generated SDK uses — so
 * the bearer token comes from the registered token provider and the base URL is
 * resolved centrally. Before this fix it read `localStorage['ppt_access_token']`
 * directly and used a separate base-URL env var, bypassing token refresh, MFA
 * retry, and consistent base-URL handling.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  authenticatedFetch: vi.fn(),
}));

vi.mock('@ppt/api-client', () => ({
  authenticatedFetch: mocks.authenticatedFetch,
}));

import { gdprFetch } from './gdprClient';

beforeEach(() => {
  vi.clearAllMocks();
  mocks.authenticatedFetch.mockResolvedValue({ ok: true, status: 200 } as Response);
});

describe('gdprFetch routes through the shared authenticated client (#3000)', () => {
  it('delegates to authenticatedFetch with the given path and init', async () => {
    const init: RequestInit = {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ format: 'json' }),
    };
    await gdprFetch('/api/v1/gdpr/export/request', init);

    expect(mocks.authenticatedFetch).toHaveBeenCalledTimes(1);
    expect(mocks.authenticatedFetch).toHaveBeenCalledWith('/api/v1/gdpr/export/request', init);
  });

  it('defaults init to an empty object when omitted', async () => {
    await gdprFetch('/api/v1/gdpr/privacy');
    expect(mocks.authenticatedFetch).toHaveBeenCalledWith('/api/v1/gdpr/privacy', {});
  });

  it('returns the Response produced by the shared client', async () => {
    const res = { ok: true, status: 200 } as Response;
    mocks.authenticatedFetch.mockResolvedValueOnce(res);
    await expect(gdprFetch('/api/v1/gdpr/export/history')).resolves.toBe(res);
  });

  it('does not read the bearer token straight from localStorage', async () => {
    const bareFetch = vi.fn();
    vi.stubGlobal('fetch', bareFetch);
    const getItem = vi.spyOn(Storage.prototype, 'getItem');

    await gdprFetch('/api/v1/gdpr/privacy');

    expect(getItem).not.toHaveBeenCalledWith('ppt_access_token');
    // And it never falls back to a bare, unauthenticated fetch.
    expect(bareFetch).not.toHaveBeenCalled();

    getItem.mockRestore();
    vi.unstubAllGlobals();
  });
});
