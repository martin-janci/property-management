/**
 * Route-container wiring tests for `ScheduleDetailPageRoute` (GH #1587).
 *
 * The sibling `features/reports/pages/ScheduleDetailPage.download-retry.test.tsx`
 * exercises the presentational page + the real hooks through a hand-rolled
 * `Harness`. But that Harness re-implements the route container's *failure*
 * handling and diverges from production exactly there: it calls
 * `downloadReport.mutate(id)` with no `onError`, and swallows the retry
 * rejection in local state instead of toasting. So the production route's
 * `onError`/toast wiring in `routes/groups/reports.tsx` was never asserted.
 *
 * These tests mount the PRODUCTION route (`reportRoutes()` at
 * `/reports/schedules/:scheduleId`) and assert the route container actually:
 *   - fires the download-failure error toast via the `mutate(..., { onError })`
 *     callback it passes, and
 *   - fires the retry-success toast after `retryExecution.mutateAsync` resolves.
 *
 * Only the api-client hooks + `useToast` are mocked (the schedule object is a
 * stub in the route, so the sole data dep is `useReportExecutionHistory`).
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactNode } from 'react';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { reportRoutes } from './reports';

const SCHEDULE_ID = 'sched-1';

const mockShowToast = vi.fn();
const mockDownloadMutate = vi.fn();
const mockRetryMutateAsync = vi.fn();

// Keep every other export real (Button/Card/etc. used by the lazy page); only
// override useToast (capture the toast) and ProtectedRoute (pass children
// through so we don't depend on auth state).
vi.mock('../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../components')>()),
  useToast: () => ({ showToast: mockShowToast }),
  ProtectedRoute: ({ children }: { children: ReactNode }) => children,
}));

// Override only the four hooks the route module consumes; preserve the rest of
// the api-client surface (types/utils the page may import).
vi.mock('@ppt/api-client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@ppt/api-client')>()),
  useReportExecutionHistory: () => ({
    data: {
      executions: [
        {
          id: 'exec-completed',
          scheduleId: SCHEDULE_ID,
          status: 'completed',
          startedAt: '2026-06-01T08:00:00Z',
          completedAt: '2026-06-01T08:00:05Z',
          fileName: 'report.pdf',
          fileSize: 1000,
          createdAt: '2026-06-01T08:00:00Z',
        },
        {
          id: 'exec-failed',
          scheduleId: SCHEDULE_ID,
          status: 'failed',
          startedAt: '2026-06-02T08:00:00Z',
          completedAt: '2026-06-02T08:00:02Z',
          error: { code: 'GEN_ERROR', message: 'Generation failed' },
          createdAt: '2026-06-02T08:00:00Z',
        },
      ],
      hasMore: false,
    },
    isLoading: false,
    isError: false,
  }),
  useDownloadReport: () => ({ mutate: mockDownloadMutate }),
  useRetryReportExecution: () => ({ mutateAsync: mockRetryMutateAsync }),
  useUpdateScheduleCron: () => ({ mutateAsync: vi.fn() }),
}));

function renderScheduleDetailRoute() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[`/reports/schedules/${SCHEDULE_ID}`]}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{reportRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe('ScheduleDetailPageRoute download/retry toast wiring (#1587)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('fires the download-failure error toast via the route container onError wiring', async () => {
    // Production calls downloadReport.mutate(id, { onError: () => showToast(error) }).
    // Drive that onError so the route's toast wiring (not the Harness's bare
    // mutate(id)) is exercised.
    mockDownloadMutate.mockImplementation(
      (_id: string, opts?: { onError?: (e: unknown) => void }) =>
        opts?.onError?.(new Error('download boom'))
    );

    renderScheduleDetailRoute();
    const downloadBtn = await screen.findByRole(
      'button',
      { name: /download report/i },
      { timeout: 5000 }
    );
    await userEvent.click(downloadBtn);

    expect(mockDownloadMutate).toHaveBeenCalledWith(
      'exec-completed',
      expect.objectContaining({ onError: expect.any(Function) })
    );
    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'error' }));
    });
  });

  it('fires the retry-success toast after the retry mutation resolves', async () => {
    mockRetryMutateAsync.mockResolvedValue(undefined);

    renderScheduleDetailRoute();
    const retryBtn = await screen.findByRole(
      'button',
      { name: /retry execution/i },
      { timeout: 5000 }
    );
    await userEvent.click(retryBtn);

    expect(mockRetryMutateAsync).toHaveBeenCalledWith('exec-failed');
    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
    });
  });
});
