/**
 * Facilities and reports clients must send the shared bearer token and tenant
 * header; before they used bare `fetch`, so residents got 401 on every
 * facilities/bookings page.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearOrgProvider, setOrgProvider } from '../auth/org-provider';
import { clearTokenProvider, setTokenProvider } from '../auth/token-provider';
import { getReport } from '../reports/api';
import { listFacilities } from './api';

function okJson(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
  });
}

describe('facilities and reports api clients — authentication', () => {
  afterEach(() => {
    clearTokenProvider();
    clearOrgProvider();
    vi.restoreAllMocks();
  });

  it.each([
    ['listFacilities', () => listFacilities('b-1')],
    ['getReport', () => getReport('r-1')],
  ])('%s attaches the bearer token and tenant header', async (_name, call) => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({ data: [], items: [] }));
    setTokenProvider(() => 'tok-123');
    setOrgProvider(() => 'org-456');

    await call().catch(() => undefined);

    const init = vi.mocked(fetch).mock.calls[0][1] as RequestInit;
    const headers = new Headers(init.headers);
    expect(headers.get('Authorization')).toBe('Bearer tok-123');
    expect(headers.get('X-Tenant-ID')).toBe('org-456');
  });
});
