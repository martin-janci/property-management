/**
 * Request-wiring contract tests for the report-schedule transport (issue #2324).
 *
 * PR #2292 wired the ppt-web create-schedule flow to list hooks whose transport
 * fns targeted routes/methods the backend never registered:
 *   - `listSchedules()` hit `GET /api/v1/reports/schedules`, which the router
 *     registered POST-only (→ 405), so the Schedules tab never loaded.
 *   - the report selector was fed by a `/definitions` list route that does not
 *     exist at all.
 * The route-container test mocked those hooks wholesale, so the mismatch was
 * invisible to CI. These tests spy on the real fetch layer and pin the exact
 * URL + HTTP method each transport fn emits, so a route/method drift fails CI
 * instead of shipping. The backend list route is added alongside this change
 * (`backend/servers/api-server/src/routes/reports.rs` — `GET /schedules`).
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createSchedule, listSchedules } from './api';

/** Minimal ok JSON response stub. */
function okJson(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as Response;
}

/** Read the (url, method) pair the client passed to the last fetch call. */
function lastCall(): { url: string; method: string } {
  const calls = vi.mocked(fetch).mock.calls;
  const call = calls[calls.length - 1];
  if (!call) throw new Error('fetch was not called');
  const url = String(call[0]);
  const method = ((call[1] as RequestInit | undefined)?.method ?? 'GET').toUpperCase();
  return { url, method };
}

describe('report-schedule api client — request wiring contract (issue #2324)', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(okJson({ schedules: [], total: 0 }));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('listSchedules → GET /api/v1/reports/schedules (matches the backend GET route)', async () => {
    await listSchedules({ organization_id: 'org-1' });
    const { url, method } = lastCall();
    // Must be GET — the backend registered this exact path+verb (a POST-only
    // route here is the #2324 regression that 405'd the Schedules list).
    expect(method).toBe('GET');
    expect(url.startsWith('/api/v1/reports/schedules?')).toBe(true);
    expect(url).toContain('organization_id=org-1');
  });

  it('listSchedules serializes report_id / is_active filters', async () => {
    await listSchedules({ organization_id: 'org-1', report_id: 'rep-9', is_active: false });
    const { url } = lastCall();
    expect(url).toContain('report_id=rep-9');
    expect(url).toContain('is_active=false');
  });

  it('createSchedule → POST /api/v1/reports/schedules', async () => {
    await createSchedule('org-1', {
      report_id: 'a0000000-0000-4000-8000-000000000001',
      name: 'Weekly faults',
      frequency: 'weekly',
      day_of_week: 1,
      time: '09:00',
      recipients: ['owner@example.com'],
    });
    const { url, method } = lastCall();
    expect(method).toBe('POST');
    expect(url).toBe('/api/v1/reports/schedules');
  });
});
