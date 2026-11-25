import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/accountingClient', () => ({ getAccountingApiBase: () => 'http://acc.test' }));
const getAccessToken = vi.fn<() => string | null>();
vi.mock('@/lib/auth', () => ({ getAccessToken: () => getAccessToken() }));

import { authedFetch } from './authedFetch';

describe('authedFetch', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => [] });
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it('sends the SSO access token as a Bearer header', async () => {
    getAccessToken.mockReturnValue('jwt-1');

    await authedFetch('/invoices');

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('http://acc.test/api/v1/invoices');
    expect(init.headers.Authorization).toBe('Bearer jwt-1');
  });

  it('keeps the auth and content-type headers when the caller passes its own', async () => {
    getAccessToken.mockReturnValue('jwt-1');

    await authedFetch('/invoices', {
      method: 'POST',
      headers: { 'Idempotency-Key': 'k-1' },
      body: '{}',
    });

    const init = fetchMock.mock.calls[0][1];
    expect(init.method).toBe('POST');
    expect(init.headers).toEqual({
      'Content-Type': 'application/json',
      Authorization: 'Bearer jwt-1',
      'Idempotency-Key': 'k-1',
    });
  });

  it('omits Authorization when signed out and surfaces the failure status', async () => {
    getAccessToken.mockReturnValue(null);
    fetchMock.mockResolvedValue({ ok: false, status: 401 });

    await expect(authedFetch('/invoices')).rejects.toThrow('Request failed (401)');
    expect(fetchMock.mock.calls[0][1].headers.Authorization).toBeUndefined();
  });
});
