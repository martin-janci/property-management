/**
 * Contract tests for the execution / export transport (gap audit 3.1).
 *
 * The api-server serialises executions, history pages and download links in
 * snake_case, while the UI types are camelCase. Before the normalisation the
 * history table rendered `undefined` start times and the download anchor got
 * `download="undefined"`. These pin the mapping and the new schedule actions.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  deleteSchedule,
  getReportExecutionDownloadUrl,
  getReportExecutionHistory,
  isSyncExport,
  runScheduleNow,
} from './api';

function okJson(body: unknown, status = 200): Response {
  return { ok: true, status, json: async () => body } as Response;
}

function lastCall(): { url: string; method: string } {
  const calls = vi.mocked(fetch).mock.calls;
  const call = calls[calls.length - 1];
  if (!call) throw new Error('fetch was not called');
  const resolved = new URL(String(call[0]), 'http://origin.invalid');
  const method = ((call[1] as RequestInit | undefined)?.method ?? 'GET').toUpperCase();
  return { url: resolved.pathname + resolved.search, method };
}

const SNAKE_EXECUTION = {
  id: 'exec-1',
  schedule_id: 'sched-1',
  status: 'failed',
  started_at: '2026-11-25T08:00:00Z',
  completed_at: '2026-11-25T08:00:02Z',
  duration_ms: 2000,
  file_key: null,
  file_name: null,
  file_size: null,
  error_code: 'STORAGE_NOT_CONFIGURED',
  error_message: 'File storage is not configured',
  error_details: null,
  created_at: '2026-11-25T08:00:00Z',
};

describe('report execution transport', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('maps a snake_case history page to the camelCase UI shape', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({ executions: [SNAKE_EXECUTION], total: 3, has_more: true })
    );
    const page = await getReportExecutionHistory({ scheduleId: 'sched-1', limit: 1 });
    expect(page.hasMore).toBe(true);
    expect(page.total).toBe(3);
    expect(page.executions[0]).toMatchObject({
      id: 'exec-1',
      scheduleId: 'sched-1',
      startedAt: '2026-11-25T08:00:00Z',
      durationMs: 2000,
      error: { code: 'STORAGE_NOT_CONFIGURED', message: 'File storage is not configured' },
    });
    expect(page.executions[0].fileName).toBeUndefined();
  });

  it('maps the download link fields', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({
        url: 'https://s3.example/report.pdf?sig=1',
        expires_at: '2026-11-25T09:00:00Z',
        file_name: 'weekly-faults.pdf',
        content_type: 'application/pdf',
      })
    );
    const link = await getReportExecutionDownloadUrl('exec-1');
    expect(link).toEqual({
      url: 'https://s3.example/report.pdf?sig=1',
      expiresAt: '2026-11-25T09:00:00Z',
      fileName: 'weekly-faults.pdf',
      contentType: 'application/pdf',
    });
  });

  it('runScheduleNow → POST /schedules/{id}/run and returns the queued execution', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      okJson({ ...SNAKE_EXECUTION, status: 'pending', error_code: null, error_message: null }, 202)
    );
    const execution = await runScheduleNow('sched-1');
    expect(lastCall()).toEqual({ url: '/api/v1/reports/schedules/sched-1/run', method: 'POST' });
    expect(execution.status).toBe('pending');
    expect(execution.error).toBeUndefined();
  });

  it('deleteSchedule → DELETE /schedules/{id} without parsing the 204 body', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: true,
      status: 204,
      json: async () => {
        throw new Error('no body');
      },
    } as unknown as Response);
    await expect(deleteSchedule('sched-1')).resolves.toBeUndefined();
    expect(lastCall()).toEqual({ url: '/api/v1/reports/schedules/sched-1', method: 'DELETE' });
  });

  it('tells inline exports from queued ones', () => {
    expect(
      isSyncExport({ data: 'AAA=', filename: 'f.csv', content_type: 'text/csv', format: 'csv' })
    ).toBe(true);
    expect(
      isSyncExport({
        download_url: '/api/v1/reports/export/job-1/status',
        format: 'pdf',
        expires_at: '2026-11-26T00:00:00Z',
      })
    ).toBe(false);
  });
});
