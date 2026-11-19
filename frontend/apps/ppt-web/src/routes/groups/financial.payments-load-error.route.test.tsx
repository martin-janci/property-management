/// <reference types="vitest/globals" />
/**
 * PaymentManagementPageRoute query-load-failure surfacing.
 *
 * The payments list query in `routes/groups/financial.tsx` originally read only
 * `{ data, isLoading }` and never looked at the query's error. A rejected
 * `listPayments` fetch therefore fell through to the `?? []` / `?? 0`
 * fallbacks, so the page rendered an EMPTY payments table and €0 "Total
 * received" / "Pending reconciliation" metrics — a failed financial fetch was
 * indistinguishable from a genuinely empty account.
 *
 * This test mounts the PRODUCTION financial route at `/financial/payments`,
 * rejects the `listPayments` api-client call, and asserts the route surfaces an
 * error state (not a silent €0 dashboard). Only api-client + `useToast` +
 * `useAuth` are mocked; the real PaymentManagementPage renders.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { type ReactNode, Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { financialRoutes } from './financial';

// ── i18n: resolve keys to their defaultValue (or the bare key) ──
vi.mock('react-i18next', async (importOriginal) => ({
  ...(await importOriginal<typeof import('react-i18next')>()),
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
  }),
}));

// ── toast: capture, keep every other component export real. ProtectedRoute is
//    neutralised to a pass-through so this test exercises the page's query-load
//    error surfacing, not the route's auth gate (covered by
//    core-route-groups-auth-guard.route.test.tsx); mirrors the sibling
//    financial.dashboard-error test. ──
const mockShowToast = vi.fn();
vi.mock('../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../components')>()),
  useToast: () => ({ showToast: mockShowToast }),
  ProtectedRoute: ({ children }: { children: ReactNode }) => children,
}));

// ── auth: manager user with an org so the queries enable ──
vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { id: 'user-1', organizationId: 'org-1', role: 'manager' } }),
}));

const mockListPayments = vi.fn();

vi.mock('@ppt/api-client', () => ({
  listPayments: (...args: unknown[]) => mockListPayments(...args),
  listUnallocatedPayments: vi.fn().mockResolvedValue([]),
  listInvoices: vi.fn().mockResolvedValue({ invoices: [], total: 0 }),
  allocatePayment: vi.fn(),
  autoMatchPayments: vi.fn(),
  getARAgingReport: vi.fn(),
  getBalanceSheet: vi.fn(),
  getCashFlowReport: vi.fn(),
  getIncomeStatement: vi.fn(),
  getOverdueInvoices: vi.fn(),
  downloadInvoicePdf: vi.fn(),
  exportReport: vi.fn(),
  getInvoice: vi.fn(),
  sendInvoice: vi.fn(),
}));

function renderPaymentsRoute() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={['/financial/payments']}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{financialRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe('PaymentManagementPageRoute load-error surfacing', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('shows an error state (not silent €0 metrics) when the payments query rejects', async () => {
    mockListPayments.mockRejectedValue(new Error('payments boom'));

    renderPaymentsRoute();

    // The route must surface the failure as an error state, not swallow it into
    // an empty/zero dashboard.
    const alert = await screen.findByRole('alert', {}, { timeout: 5000 });
    expect(alert).toHaveTextContent('financial.payments.loadFailed');

    // And it must NOT render the metric cards populated with misleading zeros.
    expect(screen.queryByText('financial.payments.totalReceived')).not.toBeInTheDocument();
  });
});
