/**
 * Regression tests for the MFA verify request base URL.
 *
 * `verifyMfaCode` must issue its POST against the configured API base (the
 * generated client's `baseUrl`, set once in `main.tsx` via
 * `client.setConfig({ baseUrl })`) — the same origin every other core request
 * uses (e.g. `gdprClient`, community). Before this fix `App.tsx` issued a raw
 * `fetch('/api/v1/auth/mfa/verify')` with a hand-built bearer header, which
 * resolved against the *web* origin and therefore hit the wrong host whenever
 * `VITE_API_URL` points at a different origin than the SPA.
 */

import { client } from '@ppt/api-client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { verifyMfaCode } from './mfaVerify';

const PREV_BASE_URL = client.getConfig().baseUrl;

beforeEach(() => {
  client.setConfig({ baseUrl: 'https://api.example.test' });
});

afterEach(() => {
  client.setConfig({ baseUrl: PREV_BASE_URL });
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('verifyMfaCode hits the configured API base, not the web origin', () => {
  it('sends the verify POST to the configured API base', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200 } as Response);
    vi.stubGlobal('fetch', fetchMock);

    await verifyMfaCode('123456');

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const calledUrl = String(fetchMock.mock.calls[0]?.[0]);
    expect(calledUrl).toBe('https://api.example.test/api/v1/auth/mfa/verify');
  });

  it('resolves true on a 2xx response and false on a non-2xx response', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce({ ok: true, status: 200 } as Response)
      .mockResolvedValueOnce({ ok: false, status: 400 } as Response);
    vi.stubGlobal('fetch', fetchMock);

    await expect(verifyMfaCode('123456')).resolves.toBe(true);
    await expect(verifyMfaCode('000000')).resolves.toBe(false);
  });

  it('resolves false when the request throws (network error)', async () => {
    const fetchMock = vi.fn().mockRejectedValue(new Error('network'));
    vi.stubGlobal('fetch', fetchMock);

    await expect(verifyMfaCode('123456')).resolves.toBe(false);
  });
});
