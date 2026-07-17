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

import { ApiError } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import i18n from 'i18next';
import type { ReactNode } from 'react';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { afterAll, beforeEach, describe, expect, it, vi } from 'vitest';
import sk from '../../../messages/sk.json';
import { BUILTIN_REPORTS } from '../../features/reports/builtinReports';
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

    // Fill the required fields. The report selector is now populated from the
    // fixed built-in report set (issue #2324) — there is no report-definitions
    // endpoint, so the dropdown no longer depends on a network call.
    const builtin = BUILTIN_REPORTS[0];
    await user.selectOptions(screen.getByRole('combobox', { name: /report/i }), builtin.id);
    await user.type(screen.getByLabelText(/schedule name/i), 'Weekly Revenue Summary');
    await user.type(screen.getByLabelText(/recipients/i), 'owner@example.com');

    await user.click(screen.getByRole('button', { name: /save schedule/i }));

    await waitFor(() => {
      expect(mockCreateMutateAsync).toHaveBeenCalledTimes(1);
    });
    expect(mockCreateMutateAsync).toHaveBeenCalledWith(
      expect.objectContaining({
        report_id: builtin.id,
        name: 'Weekly Revenue Summary',
        recipients: ['owner@example.com'],
      })
    );
    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
    });
  });

  // issue #2370: the create-failure toast must show a *localized* message keyed
  // on the backend error `code`, not the raw English `ApiError.message`.
  it('shows a localized error toast keyed on the backend error code (not raw English)', async () => {
    const rawBackendMessage = 'Schedule name must not be empty';
    mockCreateMutateAsync.mockRejectedValue(new ApiError(400, rawBackendMessage, 'EMPTY_NAME'));
    const user = userEvent.setup();

    renderReportsRoute();

    await user.click(await screen.findByRole('button', { name: /schedules/i }, { timeout: 5000 }));
    await user.click(await screen.findByRole('button', { name: /new schedule/i }));

    const builtin = BUILTIN_REPORTS[0];
    await user.selectOptions(screen.getByRole('combobox', { name: /report/i }), builtin.id);
    await user.type(screen.getByLabelText(/schedule name/i), 'Weekly Revenue Summary');
    await user.type(screen.getByLabelText(/recipients/i), 'owner@example.com');

    await user.click(screen.getByRole('button', { name: /save schedule/i }));

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'error' }));
    });
    const toastArg = mockShowToast.mock.calls.at(-1)?.[0] as { message: string };
    // The localized copy for EMPTY_NAME, not the untranslated server literal.
    expect(toastArg.message).toBe(i18n.t('reports.schedule.createErrors.emptyName'));
    expect(toastArg.message).not.toBe(rawBackendMessage);
  });

  // issue #2403: the backend emits create-schedule error codes beyond the
  // original 4 (EMPTY_NAME/INVALID_FREQUENCY/INVALID_RECIPIENT_EMAIL/
  // INVALID_TIMEZONE). Previously TOO_MANY_RECIPIENTS/INVALID_TIME/INVALID_FORMAT
  // fell through to the generic "Failed to create schedule." — now they map to
  // their own localized copy.
  it('maps a newly-covered backend error code to its localized message', async () => {
    mockCreateMutateAsync.mockRejectedValue(
      new ApiError(400, 'Too many recipients', 'TOO_MANY_RECIPIENTS')
    );
    const user = userEvent.setup();

    renderReportsRoute();

    await user.click(await screen.findByRole('button', { name: /schedules/i }, { timeout: 5000 }));
    await user.click(await screen.findByRole('button', { name: /new schedule/i }));

    const builtin = BUILTIN_REPORTS[0];
    await user.selectOptions(screen.getByRole('combobox', { name: /report/i }), builtin.id);
    await user.type(screen.getByLabelText(/schedule name/i), 'Weekly Revenue Summary');
    await user.type(screen.getByLabelText(/recipients/i), 'owner@example.com');

    await user.click(screen.getByRole('button', { name: /save schedule/i }));

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'error' }));
    });
    const toastArg = mockShowToast.mock.calls.at(-1)?.[0] as { message: string };
    expect(toastArg.message).toBe(i18n.t('reports.schedule.createErrors.tooManyRecipients'));
    expect(toastArg.message).not.toBe(i18n.t('reports.schedule.createFailed'));
  });

  // issue #2370: the report selector labels must be translated per-locale, not
  // rendered from the hardcoded English `name`.
  describe('localized report selector labels', () => {
    afterAll(async () => {
      await i18n.changeLanguage('en');
    });

    it('renders the report options in the active (non-English) locale', async () => {
      i18n.addResourceBundle('sk', 'translation', sk, true, true);
      await i18n.changeLanguage('sk');
      const user = userEvent.setup();

      renderReportsRoute();

      await user.click(
        await screen.findByRole('button', { name: /schedules|naplánova/i }, { timeout: 5000 })
      );
      await user.click(await screen.findByRole('button', { name: /new schedule|naplánova/i }));

      // The Slovak label for the "faults" built-in report — proves the option
      // is resolved via i18n, not the English `BuiltinReport.name`.
      const skFaults = sk.reports.builtin.faults;
      expect(skFaults).not.toBe(BUILTIN_REPORTS[0].name);
      await waitFor(() => {
        expect(screen.getByRole('option', { name: skFaults })).toBeInTheDocument();
      });
    });

    // issue #2403: the static form chrome around the report selector (placeholder
    // option, submit button, …) was hardcoded English — in a non-English locale
    // it must now render translated copy too.
    it('renders the static form chrome in the active (non-English) locale', async () => {
      i18n.addResourceBundle('sk', 'translation', sk, true, true);
      await i18n.changeLanguage('sk');
      const user = userEvent.setup();

      renderReportsRoute();

      await user.click(
        await screen.findByRole('button', { name: /schedules|naplánova/i }, { timeout: 5000 })
      );
      await user.click(await screen.findByRole('button', { name: /new schedule|naplánova/i }));

      // The empty "Select a report" placeholder option — localized, not the
      // hardcoded English literal.
      const skSelect = sk.reports.schedule.form.selectReport;
      expect(skSelect).not.toBe('Select a report');
      await waitFor(() => {
        expect(screen.getByRole('option', { name: skSelect })).toBeInTheDocument();
      });
      // The submit button is localized too.
      expect(
        screen.getByRole('button', { name: sk.reports.schedule.form.save })
      ).toBeInTheDocument();
    });
  });
});
