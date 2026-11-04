import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../auth', () => ({ bearerAuthHeader: () => ({ Authorization: 'Bearer t' }) }));

import { listDisputes } from './api';

const row = (i: number) => ({
  id: `d${i}`,
  reference_number: `DSP-${i}`,
  category: 'noise',
  title: `Dispute ${i}`,
  status: 'filed',
  priority: 'high',
  filed_by_name: 'Jana',
  assigned_to_name: null,
  party_count: 2,
  created_at: '2026-11-01T00:00:00Z',
  updated_at: '2026-11-02T00:00:00Z',
});

const fetchMock = vi.fn();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal('fetch', fetchMock);
});
afterEach(() => {
  vi.unstubAllGlobals();
});

function respond(body: unknown) {
  fetchMock.mockResolvedValue({ ok: true, status: 200, json: async () => body });
}

describe('listDisputes', () => {
  it('calls GET /api/v1/disputes with snake_case filters and limit+1', async () => {
    respond([]);
    await listDisputes('org-1', {
      status: 'filed',
      category: 'noise',
      priority: 'high',
      search: ' loud ',
      page: 2,
      limit: 10,
    });
    const url = new URL(fetchMock.mock.calls[0][0], 'http://x');
    expect(url.pathname).toBe('/api/v1/disputes');
    expect(Object.fromEntries(url.searchParams)).toEqual({
      organization_id: 'org-1',
      status: 'filed',
      category: 'noise',
      priority: 'high',
      search: 'loud',
      limit: '11',
      offset: '10',
    });
  });

  it('maps rows to camelCase and detects another page', async () => {
    respond([row(1), row(2), row(3)]);
    const res = await listDisputes('org-1', { page: 1, limit: 2 });
    expect(res.hasMore).toBe(true);
    expect(res.data).toHaveLength(2);
    expect(res.total).toBe(3);
    expect(res.data[0]).toEqual({
      id: 'd1',
      referenceNumber: 'DSP-1',
      category: 'noise',
      title: 'Dispute 1',
      status: 'filed',
      priority: 'high',
      filedByName: 'Jana',
      assignedToName: undefined,
      partyCount: 2,
      createdAt: '2026-11-01T00:00:00Z',
      updatedAt: '2026-11-02T00:00:00Z',
    });
  });

  it('reports an exact total on the last page', async () => {
    respond([row(1)]);
    const res = await listDisputes('org-1', { page: 3, limit: 10 });
    expect(res.hasMore).toBe(false);
    expect(res.total).toBe(21);
  });
});
