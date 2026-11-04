/**
 * Contract tests for the hand-rolled meters API client: the consumption call
 * used by the ppt-web meter comparison view, and bearer-token propagation (the
 * module previously used a bare `fetch` without the Authorization header, so
 * every `AuthUser`-guarded meters route answered 401).
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearTokenProvider, setTokenProvider } from '../auth/token-provider';
import { getConsumptionHistory, listMetersForBuilding } from './api';

const ID = '33333333-3333-3333-3333-333333333333';

describe('meters api client — request wiring contract', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({}),
    } as Response);
    setTokenProvider(() => 'test-token');
  });

  afterEach(() => {
    vi.restoreAllMocks();
    clearTokenProvider();
  });

  it('getConsumptionHistory → GET /api/v1/meters/{id}/consumption?from&to with bearer token', async () => {
    await getConsumptionHistory(ID, { from: '2026-01-01', to: '2026-06-30' });
    const [url, init] = vi.mocked(fetch).mock.calls[0];
    expect(String(url)).toBe(`/api/v1/meters/${ID}/consumption?from=2026-01-01&to=2026-06-30`);
    expect(((init as RequestInit).method ?? 'GET').toUpperCase()).toBe('GET');
    expect(((init as RequestInit).headers as Record<string, string>).Authorization).toBe(
      'Bearer test-token'
    );
  });

  it('listMetersForBuilding carries the bearer token', async () => {
    await listMetersForBuilding(ID, {});
    const [, init] = vi.mocked(fetch).mock.calls[0];
    expect(((init as RequestInit).headers as Record<string, string>).Authorization).toBe(
      'Bearer test-token'
    );
  });
});
