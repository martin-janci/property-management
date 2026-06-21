/**
 * Tests for the financial API client's auth wiring (#975).
 *
 * The hand-written `fetchApi` does NOT go through the generated @hey-api client,
 * so it doesn't get the centralized auth interceptor (#1616). Before this fix it
 * sent no Authorization / X-Tenant-ID, so every Epic 52 financial request 401'd.
 * These pin that requests now carry auth from the shared providers and target the
 * configured base URL.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  getToken: vi.fn(),
  getOrg: vi.fn(),
  getConfig: vi.fn(),
}));

vi.mock('../auth', () => ({ getToken: mocks.getToken, getOrg: mocks.getOrg }));
vi.mock('../generated/client.gen', () => ({ client: { getConfig: mocks.getConfig } }));

import { getOverdueInvoices, listAccounts } from './api';

beforeEach(() => {
  vi.clearAllMocks();
  mocks.getConfig.mockReturnValue({ baseUrl: 'https://api.example.test' });
});

describe('financial fetchApi auth wiring', () => {
  it('sends Authorization + X-Tenant-ID from the providers and prefixes the base URL', async () => {
    mocks.getToken.mockReturnValue('tok-123');
    mocks.getOrg.mockReturnValue('org-9');
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => [] });
    vi.stubGlobal('fetch', fetchMock);

    await listAccounts({ organization_id: 'org-9' });

    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toContain('https://api.example.test/api/v1/financial/accounts');
    expect(init.headers.Authorization).toBe('Bearer tok-123');
    expect(init.headers['X-Tenant-ID']).toBe('org-9');
    // The bug was a missing header / `Bearer null`.
    expect(init.headers.Authorization).not.toContain('null');

    vi.unstubAllGlobals();
  });

  it('omits auth headers when there is no session rather than sending Bearer null', async () => {
    mocks.getToken.mockReturnValue(null);
    mocks.getOrg.mockReturnValue(null);
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, json: async () => [] });
    vi.stubGlobal('fetch', fetchMock);

    await listAccounts({ organization_id: 'org-9' });

    const [, init] = fetchMock.mock.calls[0];
    expect(init.headers.Authorization).toBeUndefined();
    expect(init.headers['X-Tenant-ID']).toBeUndefined();

    vi.unstubAllGlobals();
  });
});

describe('financial decimal coercion', () => {
  beforeEach(() => {
    mocks.getToken.mockReturnValue('t');
    mocks.getOrg.mockReturnValue('o');
  });

  it('coerces wire decimal STRINGS to numbers (list of accounts)', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => [{ id: 'a', balance: '100.50', opening_balance: '0.00' }],
      })
    );

    const accounts = await listAccounts({ organization_id: 'o' });
    expect(accounts[0].balance).toBe(100.5);
    expect(typeof accounts[0].balance).toBe('number');
    expect(accounts[0].opening_balance).toBe(0);

    vi.unstubAllGlobals();
  });

  it('coerces invoice money fields so arithmetic works (no string concat)', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => [
          { id: 'i1', total: '60.00', balance_due: '60.00', amount_paid: '0.00' },
          { id: 'i2', total: '40.00', balance_due: '40.00', amount_paid: '0.00' },
        ],
      })
    );

    const invoices = await getOverdueInvoices('o');
    const outstanding = invoices.reduce((sum, inv) => sum + inv.balance_due, 0);
    expect(outstanding).toBe(100); // 60 + 40, not "060.0040.00"

    vi.unstubAllGlobals();
  });
});
