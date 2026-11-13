/// <reference types="vitest/globals" />
/**
 * Financial dashboard fetch-error regression (code-review finding
 * code-review-ppt-web-core-route-query-silent-error).
 *
 * `FinancialDashboardPageRoute` destructured only `{ data, isLoading }` from its
 * getARAgingReport / getOverdueInvoices / listInvoices / listPayments queries
 * and dropped `isError`/`error`. On a failed query `data` is undefined, so every
 * metric collapsed to a zero/empty fallback (totalBalance 0, no invoices, no
 * payments) that is indistinguishable from an organization that legitimately has
 * no financial data — the error was silently swallowed.
 *
 * The fix threads the query error into `FinancialDashboardPage` so it renders a
 * dedicated error state (title + message + retry) instead of a zeroed-out
 * dashboard. These tests pin that contract:
 *
 *  - on a query failure the error state is shown (fails on `main`, where the
 *    zeroed dashboard leaks through with no error affordance);
 *  - the retry button re-issues the failed queries;
 *  - a successful (empty) result does NOT show the error state.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { type ReactNode, Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { financialRoutes } from './financial';

vi.mock('react-i18next', async (importOriginal) => ({
  ...(await importOriginal<typeof import('react-i18next')>()),
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) =>
      typeof opts?.defaultValue === 'string' ? opts.defaultValue : key,
    i18n: { language: 'en' },
  }),
}));

vi.mock('../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../components')>()),
  useToast: () => ({ showToast: vi.fn() }),
  ProtectedRoute: ({ children }: { children: ReactNode }) => children,
}));

vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { id: 'user-1', organizationId: 'org-1', role: 'manager' } }),
}));

const mockGetARAgingReport = vi.fn();
const mockGetOverdueInvoices = vi.fn();
const mockListInvoices = vi.fn();
const mockListPayments = vi.fn();

vi.mock('@ppt/api-client', () => ({
  getARAgingReport: (...args: unknown[]) => mockGetARAgingReport(...args),
  getOverdueInvoices: (...args: unknown[]) => mockGetOverdueInvoices(...args),
  listInvoices: (...args: unknown[]) => mockListInvoices(...args),
  listPayments: (...args: unknown[]) => mockListPayments(...args),
  getInvoice: vi.fn(),
  listUnallocatedPayments: vi.fn().mockResolvedValue([]),
  allocatePayment: vi.fn(),
  autoMatchPayments: vi.fn(),
  getBalanceSheet: vi.fn(),
  getCashFlowReport: vi.fn(),
  getIncomeStatement: vi.fn(),
  downloadInvoicePdf: vi.fn(),
  exportReport: vi.fn(),
  sendInvoice: vi.fn(),
  getBudgetSummary: vi.fn(),
  listBudgets: vi.fn(),
  budgetKeys: { list: () => ['budgets'], summary: (id: string) => ['budget', id] },
  useBuildings: () => ({ data: { items: [] } }),
}));

function renderAt(path: string) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{financialRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

beforeEach(() => {
  mockGetARAgingReport.mockReset();
  mockGetOverdueInvoices.mockReset();
  mockListInvoices.mockReset();
  mockListPayments.mockReset();
});

function mockAllReject() {
  mockGetARAgingReport.mockRejectedValue(new Error('boom'));
  mockGetOverdueInvoices.mockRejectedValue(new Error('boom'));
  mockListInvoices.mockRejectedValue(new Error('boom'));
  mockListPayments.mockRejectedValue(new Error('boom'));
}

function mockAllEmpty() {
  mockGetARAgingReport.mockResolvedValue({
    entries: [],
    totals: { current: 0, days_30: 0, days_60: 0, days_90_plus: 0, total: 0 },
  });
  mockGetOverdueInvoices.mockResolvedValue([]);
  mockListInvoices.mockResolvedValue({ invoices: [], total: 0 });
  mockListPayments.mockResolvedValue({ payments: [], total: 0 });
}

describe('FinancialDashboardPageRoute fetch error state', () => {
  it('shows the error state when a dashboard query fails', async () => {
    mockAllReject();
    renderAt('/financial');
    // The dedicated error alert, not a silently-zeroed dashboard.
    expect(await screen.findByText('financial.dashboard.loadFailed')).toBeInTheDocument();
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('retries the failed queries when retry is clicked', async () => {
    mockAllReject();
    renderAt('/financial');
    await screen.findByText('financial.dashboard.loadFailed');
    const before = mockGetARAgingReport.mock.calls.length;
    await userEvent.click(screen.getByRole('button', { name: 'financial.dashboard.retry' }));
    await waitFor(() => expect(mockGetARAgingReport.mock.calls.length).toBeGreaterThan(before));
  });

  it('does not show the error state on a successful empty result', async () => {
    mockAllEmpty();
    renderAt('/financial');
    await screen.findByRole('navigation', { name: 'financial.dashboard.quickNav.label' });
    expect(screen.queryByText('financial.dashboard.loadFailed')).not.toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
