/**
 * Route-container wiring test for the Schedules-list inline pause/resume toggle.
 *
 * Before this fix `ReportsPageRoute` rendered the Schedules list (real data from
 * `useReportSchedules`) but never passed an `onToggleSchedule` handler, so the
 * per-row active/paused toggle was a silent no-op — it flipped nothing on the
 * server. The dedicated backend endpoints already exist
 * (PUT /api/v1/reports/schedules/{id}/pause and .../resume), exposed via the
 * `usePauseSchedule` / `useResumeSchedule` hooks.
 *
 * This test mounts the PRODUCTION route (`reportRoutes()` at `/reports`) and
 * asserts the container now:
 *   - calls `useResumeSchedule().mutateAsync(id)` when a *paused* row is enabled,
 *   - calls `usePauseSchedule().mutateAsync(id)` when an *active* row is disabled,
 *   - and fires the matching localized success toast.
 *
 * Only the api-client hooks + `useToast` + `useAuth` are mocked.
 */

import type { ReportSchedule } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import i18n from 'i18next';
import type { ReactNode } from 'react';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { reportRoutes } from './reports';

const ORG_ID = 'org-1';

const mockShowToast = vi.fn();
const mockPauseMutateAsync = vi.fn();
const mockResumeMutateAsync = vi.fn();

function makeSchedule(overrides: Partial<ReportSchedule>): ReportSchedule {
  return {
    id: 'sched-1',
    name: 'Weekly Revenue Summary',
    report_id: 'faults',
    frequency: 'weekly',
    time: '08:00',
    timezone: 'UTC',
    format: 'pdf',
    recipients: ['owner@example.com'],
    is_active: true,
    ...overrides,
  } as ReportSchedule;
}

let SCHEDULES: ReportSchedule[] = [];

vi.mock('../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../components')>()),
  useToast: () => ({ showToast: mockShowToast }),
  ProtectedRoute: ({ children }: { children: ReactNode }) => children,
}));

vi.mock('../../contexts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../contexts')>()),
  useAuth: () => ({ user: { organizationId: ORG_ID } }),
}));

vi.mock('@ppt/api-client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@ppt/api-client')>()),
  useReportSchedules: () => ({
    data: { schedules: SCHEDULES, total: SCHEDULES.length },
    isLoading: false,
  }),
  useCreateSchedule: () => ({ mutateAsync: vi.fn() }),
  useUpdateScheduleCron: () => ({ mutateAsync: vi.fn() }),
  usePauseSchedule: () => ({ mutateAsync: mockPauseMutateAsync }),
  useResumeSchedule: () => ({ mutateAsync: mockResumeMutateAsync }),
  useReportExecutionHistory: () => ({
    data: { executions: [], hasMore: false },
    isLoading: false,
    isError: false,
  }),
  useDownloadReport: () => ({ mutate: vi.fn() }),
  useRetryReportExecution: () => ({ mutateAsync: vi.fn() }),
}));

function renderReportsRoute() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={['/reports']}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{reportRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

async function openSchedulesTab() {
  const user = userEvent.setup();
  renderReportsRoute();
  await user.click(await screen.findByRole('button', { name: /schedules/i }, { timeout: 5000 }));
  return user;
}

describe('ReportsPageRoute schedule toggle wiring (pause/resume)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    SCHEDULES = [];
  });

  it('pauses an active schedule via usePauseSchedule and fires the paused toast', async () => {
    SCHEDULES = [makeSchedule({ id: 'sched-active', is_active: true })];
    mockPauseMutateAsync.mockResolvedValue({ id: 'sched-active', is_active: false });

    const user = await openSchedulesTab();

    // An active row's toggle is labelled "Disable schedule".
    await user.click(await screen.findByRole('button', { name: /disable schedule/i }));

    await waitFor(() => {
      expect(mockPauseMutateAsync).toHaveBeenCalledWith('sched-active');
    });
    expect(mockResumeMutateAsync).not.toHaveBeenCalled();
    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'success', message: i18n.t('reports.schedule.paused') })
      );
    });
  });

  it('resumes a paused schedule via useResumeSchedule and fires the resumed toast', async () => {
    SCHEDULES = [makeSchedule({ id: 'sched-paused', is_active: false })];
    mockResumeMutateAsync.mockResolvedValue({ id: 'sched-paused', is_active: true });

    const user = await openSchedulesTab();

    // A paused row's toggle is labelled "Enable schedule".
    await user.click(await screen.findByRole('button', { name: /enable schedule/i }));

    await waitFor(() => {
      expect(mockResumeMutateAsync).toHaveBeenCalledWith('sched-paused');
    });
    expect(mockPauseMutateAsync).not.toHaveBeenCalled();
    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'success', message: i18n.t('reports.schedule.resumed') })
      );
    });
  });

  it('surfaces an error toast when the pause mutation rejects', async () => {
    SCHEDULES = [makeSchedule({ id: 'sched-active', is_active: true })];
    mockPauseMutateAsync.mockRejectedValue(new Error('network'));

    const user = await openSchedulesTab();

    await user.click(await screen.findByRole('button', { name: /disable schedule/i }));

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          message: i18n.t('reports.schedule.toggleFailed'),
        })
      );
    });
  });
});
