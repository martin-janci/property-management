/**
 * `/reports/schedules/:id` shows the real schedule from the organization's
 * schedule list, and a not-found state for an unknown id instead of made-up
 * details ("Schedule 0000…", "Monthly at 08:00", active, PDF) (round-3
 * audit #24).
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import i18n from 'i18next';
import { type ReactNode, Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import sk from '../../../messages/sk.json';
import { reportRoutes } from './reports';

const h = vi.hoisted(() => ({
  schedules: { data: undefined as unknown, isLoading: false, isError: false },
  executionHistoryOptions: [] as Array<{ enabled?: boolean }>,
}));

vi.mock('../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../components')>()),
  useToast: () => ({ showToast: vi.fn() }),
  ProtectedRoute: ({ children }: { children: ReactNode }) => children,
}));

vi.mock('../../contexts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../contexts')>()),
  useAuth: () => ({ user: { id: 'u1', organizationId: 'org-1' } }),
}));

vi.mock('@ppt/api-client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@ppt/api-client')>()),
  useReportSchedules: () => h.schedules,
  useReportExecutionHistory: (_params: unknown, options: { enabled?: boolean }) => {
    h.executionHistoryOptions.push(options);
    return { data: { executions: [], hasMore: false }, isLoading: false, isError: false };
  },
  useDownloadReport: () => ({ mutate: vi.fn() }),
  useRetryReportExecution: () => ({ mutateAsync: vi.fn() }),
  useUpdateScheduleCron: () => ({ mutateAsync: vi.fn() }),
}));

const schedule = {
  id: 'sched-1',
  report_id: 'rep-1',
  organization_id: 'org-1',
  name: 'Monthly Building Report',
  frequency: 'weekly',
  time: '07:30',
  timezone: 'UTC',
  format: 'xlsx',
  recipients: [],
  is_active: true,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

function renderAt(id: string) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[`/reports/schedules/${id}`]}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{reportRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

beforeEach(async () => {
  i18n.addResourceBundle('sk', 'translation', sk, true, true);
  await i18n.changeLanguage('sk');
  h.schedules = { data: { schedules: [schedule], total: 1 }, isLoading: false, isError: false };
  h.executionHistoryOptions = [];
});

afterEach(async () => {
  await i18n.changeLanguage('en');
});

describe('schedule detail route', () => {
  it('shows a not-found state for an unknown schedule id', async () => {
    renderAt('00000000-0000-0000-0000-000000000000');
    expect(await screen.findByText('Plán sa nenašiel', {}, { timeout: 5000 })).toBeTruthy();
    expect(screen.queryByText(/Schedule 00000000/)).toBeNull();
    // No execution-history polling for a schedule that doesn't exist.
    expect(h.executionHistoryOptions.every((o) => o.enabled === false)).toBe(true);
  });

  it('renders the real schedule with a translated frequency line', async () => {
    renderAt('sched-1');
    expect(
      await screen.findByRole('heading', { name: 'Monthly Building Report' }, { timeout: 5000 })
    ).toBeTruthy();
    expect(screen.getByText(/ o 07:30$/)).toBeTruthy();
    expect(screen.queryByText(/ at 07:30/)).toBeNull();
  });

  it('shows a spinner, not placeholder details, while schedules load', async () => {
    h.schedules = { data: undefined, isLoading: true, isError: false };
    renderAt('sched-1');
    expect(await screen.findByRole('status', {}, { timeout: 5000 })).toBeTruthy();
    expect(screen.queryByText('Plán sa nenašiel')).toBeNull();
  });
});
