/**
 * Outage responses are snake_case on the wire (`db::models::outage` has no
 * serde rename, apart from `creatorName`). The client must map them to the
 * camelCase types so list and detail views show their fields.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  getOutage,
  getOutageDashboard,
  getOutageStatistics,
  getUnreadCount,
  listActiveOutages,
  listOutages,
  normalizeOutageDetails,
  normalizeOutageSummary,
  resolveOutage,
} from './api';

const ID = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
const B1 = '11111111-1111-1111-1111-111111111111';
const B2 = '22222222-2222-2222-2222-222222222222';

const rawOutage = {
  id: ID,
  organization_id: 'org-1',
  created_by: 'user-1',
  title: 'Water shutoff',
  description: 'Pipe repair',
  commodity: 'water',
  severity: 'critical',
  status: 'planned',
  building_ids: [B1, B2],
  scheduled_start: '2026-11-05T08:00:00Z',
  scheduled_end: '2026-11-05T12:00:00Z',
  actual_start: null,
  actual_end: null,
  external_reference: 'BVS-42',
  supplier_name: 'BVS',
  resolution_notes: null,
  created_at: '2026-11-01T10:00:00Z',
  updated_at: '2026-11-02T10:00:00Z',
};

const rawSummary = {
  id: ID,
  title: 'Water shutoff',
  commodity: 'water',
  severity: 'critical',
  status: 'planned',
  scheduled_start: '2026-11-05T08:00:00Z',
  scheduled_end: null,
  actual_start: null,
  actual_end: null,
};

function okJson(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as Response;
}

beforeEach(() => {
  vi.stubGlobal('fetch', vi.fn());
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('outage list responses', () => {
  it('maps snake_case summaries to camelCase', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(okJson({ outages: [rawSummary], count: 1, total: 7 }));
    const result = await listOutages();
    expect(result.count).toBe(1);
    expect(result.total).toBe(7);
    expect(result.outages[0]).toMatchObject({
      id: ID,
      title: 'Water shutoff',
      commodity: 'water',
      severity: 'critical',
      status: 'planned',
      scheduledStart: '2026-11-05T08:00:00Z',
      scheduledEnd: undefined,
      affectedBuildingsCount: 0,
    });
  });

  it('normalizes the active list too', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      okJson({
        outages: [{ ...rawSummary, status: 'ongoing', actual_start: '2026-11-05T08:05:00Z' }],
        count: 1,
        total: 1,
      })
    );
    const result = await listActiveOutages();
    expect(result.outages[0]?.actualStart).toBe('2026-11-05T08:05:00Z');
  });

  it('keeps camelCase summaries working', () => {
    expect(
      normalizeOutageSummary({
        ...rawSummary,
        scheduled_start: undefined,
        scheduledStart: 'x',
        affectedBuildingsCount: 3,
      })
    ).toMatchObject({ scheduledStart: 'x', affectedBuildingsCount: 3 });
  });
});

describe('outage detail responses', () => {
  it('maps the detail payload, building names and creator', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      okJson({
        outage: {
          ...rawOutage,
          creatorName: 'Ing. Peter Horváth',
          building_names: ['Hlavná 1', 'Hlavná 2'],
          notification_count: 10,
          read_count: 4,
        },
      })
    );
    const outage = await getOutage(ID);
    expect(outage).toMatchObject({
      id: ID,
      organizationId: 'org-1',
      createdBy: 'user-1',
      buildingIds: [B1, B2],
      scheduledStart: '2026-11-05T08:00:00Z',
      scheduledEnd: '2026-11-05T12:00:00Z',
      externalReference: 'BVS-42',
      supplierName: 'BVS',
      createdAt: '2026-11-01T10:00:00Z',
      updatedAt: '2026-11-02T10:00:00Z',
      creatorName: 'Ing. Peter Horváth',
    });
    expect(outage.actualStart).toBeUndefined();
    expect(outage.resolutionNotes).toBeUndefined();
    expect(outage.buildings).toEqual([
      { id: B1, name: 'Hlavná 1', address: '' },
      { id: B2, name: 'Hlavná 2', address: '' },
    ]);
  });

  it('parses building_ids sent as a JSON string', () => {
    expect(normalizeOutageDetails({ ...rawOutage, building_ids: `["${B1}"]` }).buildingIds).toEqual(
      [B1]
    );
  });

  it('maps the outage in action responses', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      okJson({
        message: 'Outage resolved',
        outage: { ...rawOutage, status: 'resolved', resolution_notes: 'Fixed' },
      })
    );
    const result = await resolveOutage(ID, { resolutionNotes: 'Fixed' });
    expect(result.message).toBe('Outage resolved');
    expect(result.outage).toMatchObject({
      status: 'resolved',
      resolutionNotes: 'Fixed',
      buildingIds: [B1, B2],
    });
  });
});

describe('outage statistics, dashboard and unread count', () => {
  it('maps the server statistics counters', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      okJson({ statistics: { total: 9, planned: 3, ongoing: 2, resolved: 3, cancelled: 1 } })
    );
    expect(await getOutageStatistics()).toEqual({
      total: 9,
      planned: 3,
      ongoing: 2,
      resolved: 3,
      cancelled: 1,
    });
  });

  it('maps the dashboard counters, commodity breakdown and upcoming list', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      okJson({
        dashboard: {
          active_count: 5,
          planned_count: 3,
          ongoing_count: 2,
          by_commodity: [
            { commodity: 'water', count: 4 },
            { commodity: 'gas', count: 1 },
          ],
          upcoming: [rawSummary],
        },
      })
    );
    const dashboard = await getOutageDashboard();
    expect(dashboard).toMatchObject({
      activeCount: 5,
      plannedCount: 3,
      ongoingCount: 2,
      byCommodity: [
        { commodity: 'water', count: 4 },
        { commodity: 'gas', count: 1 },
      ],
    });
    expect(dashboard.upcoming).toHaveLength(1);
    expect(dashboard.upcoming[0]?.scheduledStart).toBe('2026-11-05T08:00:00Z');
  });

  it('maps unread_count', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(okJson({ unread_count: 4 }));
    expect(await getUnreadCount()).toEqual({ unreadCount: 4 });
  });
});
