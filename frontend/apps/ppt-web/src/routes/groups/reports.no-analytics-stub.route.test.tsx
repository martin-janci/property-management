/**
 * Route-container regression: the reports route must NOT render the unwired
 * "Dashboard / analytics" surface.
 *
 * Before this fix `ReportsPageRoute` fed the `ReportsPage` dashboard tab with
 * hardcoded empty/zero analytics objects (`kpis={[]}`, `buildings={[]}`,
 * `trendAnalysis={…all-zero…}`, `trendLines={[]}`, `periodComparison={…}`).
 * No backend endpoint exists for any of it — the api-client analytics/trends
 * client functions (`getAnalyticsSummary`, `getBuildingAnalytics`,
 * `getTrendAnalysis`, `getTrendLines`, `comparePeriods`) targeted
 * `/api/v1/reports/analytics/*` and `/api/v1/reports/trends/*` routes that the
 * reports router never registers, and nothing referenced them. The result was
 * a Potemkin "analytics" tab that always rendered empty KPI cards and blank
 * charts (code-review-ppt-web-core-reports-route-stub-analytics).
 *
 * This test mounts the PRODUCTION route (`reportRoutes()` at `/reports`) and
 * asserts the stub analytics surface is gone:
 *   - no "Dashboard" tab button,
 *   - no "Key metrics" analytics section heading,
 * while the real, backend-wired Schedules surface still renders.
 *
 * Only the api-client hooks + `useToast` + `useAuth` are mocked.
 */

import type { ReportSchedule } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import i18n from 'i18next';
import type { ReactNode } from 'react';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { reportRoutes } from './reports';

const ORG_ID = 'org-1';

const mockShowToast = vi.fn();

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
  usePauseSchedule: () => ({ mutateAsync: vi.fn() }),
  useResumeSchedule: () => ({ mutateAsync: vi.fn() }),
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

describe('ReportsPageRoute — no unwired analytics stub', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    SCHEDULES = [];
  });

  it('does not render the stubbed Dashboard / analytics tab', async () => {
    renderReportsRoute();

    // The real, backend-wired Schedules tab still renders (proves the route mounted).
    expect(await screen.findByRole('button', { name: /schedules/i })).toBeInTheDocument();

    // The unwired analytics surface is gone: no "Dashboard" tab...
    expect(
      screen.queryByRole('button', { name: i18n.t('reports.page.tabs.dashboard') })
    ).toBeNull();
    // ...and no "Key metrics" analytics section heading anywhere on the page.
    expect(screen.queryByText(i18n.t('reports.page.keyMetrics'))).toBeNull();
    expect(screen.queryByText(i18n.t('reports.page.buildingPerformance'))).toBeNull();
  });
});
