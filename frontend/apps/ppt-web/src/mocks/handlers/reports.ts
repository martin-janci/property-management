/**
 * MSW handlers for reports execution history API (Story 81.2).
 *
 * Used in dev/test when the real api-server endpoint is not present.
 */

import type { ReportExecution, ReportExecutionHistoryResponse } from '@ppt/api-client';
import { HttpResponse, http } from 'msw';

// ---------------------------------------------------------------------------
// Seed data
// ---------------------------------------------------------------------------

function makeExecution(
  id: string,
  scheduleId: string,
  status: ReportExecution['status'],
  daysAgo: number,
  opts?: {
    fileName?: string;
    fileSize?: number;
    durationMs?: number;
    error?: ReportExecution['error'];
  }
): ReportExecution {
  const startedAt = new Date(Date.now() - daysAgo * 86_400_000).toISOString();
  const completedAt =
    status === 'pending' || status === 'running'
      ? undefined
      : new Date(new Date(startedAt).getTime() + (opts?.durationMs ?? 12_500)).toISOString();

  return {
    id,
    scheduleId,
    status,
    startedAt,
    completedAt,
    durationMs: completedAt ? (opts?.durationMs ?? 12_500) : undefined,
    fileName: status === 'completed' ? (opts?.fileName ?? `report-${id}.pdf`) : undefined,
    fileSize: status === 'completed' ? (opts?.fileSize ?? 204_800) : undefined,
    error:
      status === 'failed'
        ? (opts?.error ?? { code: 'QUERY_TIMEOUT', message: 'Query timed out after 30s' })
        : undefined,
    createdAt: startedAt,
  };
}

const SEED_EXECUTIONS: ReportExecution[] = [
  makeExecution('exec-01', 'schedule-demo', 'completed', 0, {
    fileName: 'monthly-report-may-2026.pdf',
    fileSize: 512_000,
    durationMs: 8_400,
  }),
  makeExecution('exec-02', 'schedule-demo', 'running', 0),
  makeExecution('exec-03', 'schedule-demo', 'completed', 1, {
    fileName: 'monthly-report-apr-2026.pdf',
    fileSize: 487_000,
    durationMs: 9_200,
  }),
  makeExecution('exec-04', 'schedule-demo', 'failed', 2, {
    error: {
      code: 'QUERY_TIMEOUT',
      message: 'Query timed out after 30s',
      details: 'SELECT * FROM ledger timed out',
    },
  }),
  makeExecution('exec-05', 'schedule-demo', 'completed', 3, {
    fileName: 'monthly-report-mar-2026.pdf',
    fileSize: 462_000,
    durationMs: 7_800,
  }),
  makeExecution('exec-06', 'schedule-demo', 'skipped', 4),
  makeExecution('exec-07', 'schedule-demo', 'completed', 5, {
    fileName: 'monthly-report-feb-2026.pdf',
    fileSize: 440_000,
    durationMs: 11_000,
  }),
  makeExecution('exec-08', 'schedule-demo', 'cancelled', 6),
  makeExecution('exec-09', 'schedule-demo', 'completed', 7, {
    fileName: 'monthly-report-jan-2026.pdf',
    fileSize: 395_000,
    durationMs: 6_500,
  }),
  makeExecution('exec-10', 'schedule-demo', 'failed', 8, {
    error: {
      code: 'STORAGE_ERROR',
      message: 'S3 upload failed',
      details: 'AccessDenied on bucket ppt-reports',
    },
  }),
  makeExecution('exec-11', 'schedule-demo', 'completed', 9, {
    fileName: 'monthly-report-dec-2025.pdf',
    fileSize: 378_000,
    durationMs: 7_200,
  }),
  makeExecution('exec-12', 'schedule-demo', 'completed', 10, {
    fileName: 'monthly-report-nov-2025.pdf',
    fileSize: 360_000,
    durationMs: 6_800,
  }),
];

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

export const reportsHandlers = [
  // GET /api/v1/reports/schedules/:scheduleId/executions
  http.get('/api/v1/reports/schedules/:scheduleId/executions', ({ request, params }) => {
    const { scheduleId } = params as { scheduleId: string };
    const url = new URL(request.url);

    const status = url.searchParams.get('status') as ReportExecution['status'] | null;
    const dateFrom = url.searchParams.get('date_from');
    const dateTo = url.searchParams.get('date_to');
    const limit = Number(url.searchParams.get('limit') ?? '20');
    const offset = Number(url.searchParams.get('offset') ?? '0');

    // Return demo data for any scheduleId (fallback to seed set)
    let results = SEED_EXECUTIONS.filter((e) => e.scheduleId === scheduleId);
    if (results.length === 0) {
      results = SEED_EXECUTIONS.map((e) => ({ ...e, scheduleId }));
    }

    if (status) {
      results = results.filter((e) => e.status === status);
    }
    if (dateFrom) {
      const from = new Date(dateFrom);
      results = results.filter((e) => new Date(e.startedAt) >= from);
    }
    if (dateTo) {
      const to = new Date(dateTo);
      to.setHours(23, 59, 59, 999);
      results = results.filter((e) => new Date(e.startedAt) <= to);
    }

    const total = results.length;
    const page = results.slice(offset, offset + limit);

    const body: ReportExecutionHistoryResponse = {
      executions: page,
      total,
      hasMore: offset + limit < total,
    };

    return HttpResponse.json(body);
  }),

  // POST /api/v1/reports/executions/:executionId/retry
  http.post('/api/v1/reports/executions/:executionId/retry', ({ params }) => {
    const { executionId } = params as { executionId: string };
    const original = SEED_EXECUTIONS.find((e) => e.id === executionId);
    const scheduleId = original?.scheduleId ?? 'schedule-demo';

    const retried: ReportExecution = makeExecution(
      `${executionId}-retry-${Date.now()}`,
      scheduleId,
      'pending',
      0
    );
    return HttpResponse.json(retried, { status: 201 });
  }),

  // GET /api/v1/reports/executions/:executionId/download
  http.get('/api/v1/reports/executions/:executionId/download', ({ params }) => {
    const { executionId } = params as { executionId: string };
    const execution = SEED_EXECUTIONS.find((e) => e.id === executionId);
    const fileName = execution?.fileName ?? `report-${executionId}.pdf`;

    return HttpResponse.json({
      url: `https://storage.example.com/reports/${fileName}?token=stub`,
      expiresAt: new Date(Date.now() + 15 * 60_000).toISOString(),
      fileName,
      contentType: 'application/pdf',
    });
  }),
];
