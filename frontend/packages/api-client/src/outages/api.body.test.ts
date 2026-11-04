/**
 * Outage request bodies and list filters match the api-server's snake_case
 * structs (round-3 audit #9: create failed with 422
 * `missing field scheduled_start`).
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  cancelOutage,
  createOutage,
  listOutages,
  resolveOutage,
  startOutage,
  toOutageTimestamp,
  toOutageWriteBody,
  updateOutage,
} from './api';

const ID = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';

function okJson(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as Response;
}

function lastCall(): { url: URL; body: Record<string, unknown> } {
  const calls = vi.mocked(fetch).mock.calls;
  const call = calls[calls.length - 1];
  if (!call) throw new Error('fetch was not called');
  const init = call[1] as RequestInit | undefined;
  return {
    url: new URL(String(call[0]), 'http://origin.invalid'),
    body: init?.body ? JSON.parse(String(init.body)) : {},
  };
}

beforeEach(() => {
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => okJson({ id: ID, message: 'ok' }))
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('toOutageWriteBody', () => {
  it('maps every field to snake_case and drops unset ones', () => {
    expect(
      toOutageWriteBody({
        title: 'Water',
        commodity: 'water',
        severity: 'major',
        buildingIds: ['b1'],
        scheduledStart: '2026-01-01T08:00:00Z',
        scheduledEnd: '2026-01-01T12:00:00Z',
        externalReference: 'REF-1',
        supplierName: 'BVS',
      })
    ).toEqual({
      title: 'Water',
      commodity: 'water',
      severity: 'major',
      building_ids: ['b1'],
      scheduled_start: '2026-01-01T08:00:00Z',
      scheduled_end: '2026-01-01T12:00:00Z',
      external_reference: 'REF-1',
      supplier_name: 'BVS',
    });
  });
});

describe('toOutageTimestamp', () => {
  it('keeps values that already carry an offset', () => {
    expect(toOutageTimestamp('2026-01-01T08:00:00Z')).toBe('2026-01-01T08:00:00Z');
    expect(toOutageTimestamp('2026-01-01T08:00:00+01:00')).toBe('2026-01-01T08:00:00+01:00');
  });

  it('reads a datetime-local value as local time and sends UTC', () => {
    expect(toOutageTimestamp('2026-01-01T08:00')).toBe(new Date(2026, 0, 1, 8, 0).toISOString());
  });

  it('passes empty values through as undefined', () => {
    expect(toOutageTimestamp('')).toBeUndefined();
    expect(toOutageTimestamp(undefined)).toBeUndefined();
  });
});

describe('outage client requests', () => {
  it('createOutage posts scheduled_start', async () => {
    await createOutage({
      title: 'Power',
      commodity: 'electricity',
      severity: 'critical',
      scheduledStart: '2026-01-01T08:00:00Z',
    });
    const { url, body } = lastCall();
    expect(url.pathname).toBe('/api/v1/outages');
    expect(body).toEqual({
      title: 'Power',
      commodity: 'electricity',
      severity: 'critical',
      scheduled_start: '2026-01-01T08:00:00Z',
    });
  });

  it('createOutage converts datetime-local input to RFC 3339', async () => {
    await createOutage({
      title: 'Gas',
      commodity: 'gas',
      severity: 'minor',
      scheduledStart: '2026-05-01T09:30',
    });
    expect(lastCall().body.scheduled_start).toBe(new Date(2026, 4, 1, 9, 30).toISOString());
  });

  it('updateOutage sends only the changed fields in snake_case', async () => {
    await updateOutage(ID, { scheduledEnd: '2026-01-02T08:00:00Z' });
    expect(lastCall().body).toEqual({ scheduled_end: '2026-01-02T08:00:00Z' });
  });

  it('status changes send snake_case bodies', async () => {
    await startOutage(ID, { actualStart: '2026-01-01T08:05:00Z' });
    expect(lastCall().body).toEqual({ actual_start: '2026-01-01T08:05:00Z' });
    await resolveOutage(ID, { actualEnd: '2026-01-01T11:00:00Z', resolutionNotes: 'fixed' });
    expect(lastCall().body).toEqual({
      actual_end: '2026-01-01T11:00:00Z',
      resolution_notes: 'fixed',
    });
    await cancelOutage(ID, { reason: 'supplier postponed' });
    expect(lastCall().body).toEqual({ reason: 'supplier postponed' });
    await startOutage(ID);
    expect(lastCall().body).toEqual({});
  });

  it('listOutages sends snake_case filters', async () => {
    vi.mocked(fetch).mockResolvedValue(okJson({ outages: [], count: 0, total: 0 }));
    await listOutages({ buildingId: 'b1', activeOnly: true, fromDate: '2026-01-01', limit: 10 });
    const sp = lastCall().url.searchParams;
    expect(sp.get('building_id')).toBe('b1');
    expect(sp.get('active_only')).toBe('true');
    expect(sp.get('from_date')).toBe('2026-01-01');
    expect(sp.get('limit')).toBe('10');
    expect(sp.has('buildingId')).toBe(false);
  });
});
