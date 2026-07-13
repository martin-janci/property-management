/**
 * Route-container wiring test for the "New Schedule" create flow (gap-81-1).
 *
 * Before this fix `ReportsPageRoute` rendered the "+ New Schedule" button and
 * the inline `ScheduleForm`, but never passed an `onCreateSchedule` handler and
 * fed the form an empty `reports` list — so the create flow was inert: the
 * report selector had no options and submitting called `undefined`.
 *
 * This test mounts the PRODUCTION route (`reportRoutes()` at `/reports`) and
 * asserts the container now:
 *   - feeds report definitions into the schedule form's report selector, and
 *   - invokes `useCreateSchedule().mutateAsync` with the form payload on submit
 *     and fires the success toast the route wires.
 *
 * Only the api-client hooks + `useToast` + `useAuth` are mocked.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactNode } from 'react';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { reportRoutes } from './reports';

const ORG_ID = 'org-1';

const mockShowToast = vi.fn();
const mockCreateMutateAsync = vi.fn();

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
  useReports: () => ({
    data: { reports: [{ id: 'rep-1', name: 'Monthly Revenue' }], total: 1 },
    isLoading: false,
  }),
  useReportSchedules: () => ({ data: { schedules: [], total: 0 }, isLoading: false }),
  useCreateSchedule: () => ({ mutateAsync: mockCreateMutateAsync }),
  useReportExecutionHistory: () => ({
    data: { executions: [], hasMore: false },
    isLoading: false,
    isError: false,
  }),
  useDownloadReport: () => ({ mutate: vi.fn() }),
  useRetryReportExecution: () => ({ mutateAsync: vi.fn() }),
  useUpdateScheduleCron: () => ({ mutateAsync: vi.fn() }),
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

describe('ReportsPageRoute create-schedule wiring (gap-81-1)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('creates a schedule via useCreateSchedule and fires the success toast', async () => {
    mockCreateMutateAsync.mockResolvedValue({ id: 'sched-new' });
    const user = userEvent.setup();

    renderReportsRoute();

    // Switch to the Schedules tab (only tab button matches while on dashboard).
    await user.click(await screen.findByRole('button', { name: /schedules/i }, { timeout: 5000 }));

    // Open the create form.
    await user.click(await screen.findByRole('button', { name: /new schedule/i }));

    // Fill the required fields. The report selector is populated from the
    // route's useReports data — empty before this fix.
    await user.selectOptions(screen.getByRole('combobox', { name: /report/i }), 'rep-1');
    await user.type(screen.getByLabelText(/schedule name/i), 'Weekly Revenue Summary');
    await user.type(screen.getByLabelText(/recipients/i), 'owner@example.com');

    await user.click(screen.getByRole('button', { name: /save schedule/i }));

    await waitFor(() => {
      expect(mockCreateMutateAsync).toHaveBeenCalledTimes(1);
    });
    expect(mockCreateMutateAsync).toHaveBeenCalledWith(
      expect.objectContaining({
        report_id: 'rep-1',
        name: 'Weekly Revenue Summary',
        recipients: ['owner@example.com'],
      })
    );
    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
    });
  });
});
