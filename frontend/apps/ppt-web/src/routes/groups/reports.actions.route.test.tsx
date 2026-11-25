/**
 * Route-container wiring for the Reports page actions (gap audit 3.1):
 *   - "Generate and download" on the Reports tab calls the export hook with the
 *     selected built-in report, format and period;
 *   - "Run now" on a schedule row queues a run;
 *   - "Delete" asks for confirmation and only then deletes.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactNode } from 'react';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import i18n from '../../i18n';
import { reportRoutes } from './reports';

const ORG_ID = 'org-1';
const SCHEDULE_ID = 'sched-1';

const mockShowToast = vi.fn();
const mockGenerate = vi.fn();
const mockRunNow = vi.fn();
const mockDelete = vi.fn();

vi.mock('../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../components')>()),
  useToast: () => ({ showToast: mockShowToast }),
  ProtectedRoute: ({ children }: { children: ReactNode }) => children,
}));

vi.mock('../../contexts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../contexts')>()),
  useAuth: () => ({ user: { id: 'u1', organizationId: ORG_ID } }),
}));

vi.mock('@ppt/api-client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@ppt/api-client')>()),
  useReportSchedules: () => ({
    data: {
      schedules: [
        {
          id: SCHEDULE_ID,
          report_id: 'a0000000-0000-4000-8000-000000000001',
          organization_id: ORG_ID,
          name: 'Weekly faults',
          frequency: 'weekly',
          time: '08:00',
          timezone: 'UTC',
          format: 'pdf',
          recipients: ['owner@example.com'],
          is_active: true,
          created_at: '2026-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
        },
      ],
      total: 1,
    },
    isLoading: false,
  }),
  useCreateSchedule: () => ({ mutateAsync: vi.fn() }),
  useUpdateScheduleCron: () => ({ mutateAsync: vi.fn() }),
  usePauseSchedule: () => ({ mutateAsync: vi.fn() }),
  useResumeSchedule: () => ({ mutateAsync: vi.fn() }),
  useReportExecutionHistory: () => ({ data: undefined, isLoading: false }),
  useDownloadReport: () => ({ mutate: vi.fn() }),
  useRetryReportExecution: () => ({ mutateAsync: vi.fn() }),
  useDeleteSchedule: () => ({ mutateAsync: mockDelete }),
  useRunScheduleNow: () => ({ mutateAsync: mockRunNow }),
  useGenerateReport: () => ({ mutateAsync: mockGenerate, isPending: false }),
}));

function renderReports() {
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

describe('ReportsPageRoute actions', () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await i18n.changeLanguage('en');
  });

  it('generates the selected built-in report for the chosen period', async () => {
    mockGenerate.mockResolvedValue({ fileName: 'faults.csv', async: false });
    const user = userEvent.setup();
    renderReports();

    await user.click(
      await screen.findByRole(
        'button',
        { name: new RegExp(`${i18n.t('reports.page.tabs.reports')}$`) },
        {
          timeout: 5000,
        }
      )
    );
    await user.selectOptions(
      screen.getByRole('combobox', { name: i18n.t('reports.page.generate.report') }),
      'voting'
    );
    await user.click(screen.getByRole('radio', { name: 'CSV' }));
    await user.click(screen.getByRole('button', { name: i18n.t('reports.page.generate.submit') }));

    await waitFor(() => expect(mockGenerate).toHaveBeenCalledTimes(1));
    expect(mockGenerate.mock.calls[0][0]).toMatchObject({
      organization_id: ORG_ID,
      report_type: 'voting',
      format: 'csv',
    });
    expect(mockGenerate.mock.calls[0][0].from_date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }))
    );
  }, 15_000);

  it('queues a run when "Run now" is clicked', async () => {
    mockRunNow.mockResolvedValue({ id: 'exec-1', scheduleId: SCHEDULE_ID, status: 'pending' });
    const user = userEvent.setup();
    renderReports();

    await user.click(await screen.findByTitle('Run now', {}, { timeout: 5000 }));

    await waitFor(() => expect(mockRunNow).toHaveBeenCalledWith(SCHEDULE_ID));
    expect(mockShowToast).toHaveBeenCalledWith(
      expect.objectContaining({ message: i18n.t('reports.schedule.runQueued') })
    );
  }, 15_000);

  it('deletes a schedule only after confirmation', async () => {
    mockDelete.mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderReports();

    await user.click(await screen.findByTitle('Delete', {}, { timeout: 5000 }));
    expect(mockDelete).not.toHaveBeenCalled();

    const dialog = await screen.findByRole('alertdialog');
    await user.click(
      within(dialog).getByRole('button', { name: i18n.t('reports.page.deleteSchedule.confirm') })
    );

    await waitFor(() => expect(mockDelete).toHaveBeenCalledWith(SCHEDULE_ID));
    expect(mockShowToast).toHaveBeenCalledWith(
      expect.objectContaining({ message: i18n.t('reports.schedule.deleted') })
    );
  }, 15_000);
});
