/// <reference types="vitest/globals" />
/**
 * Financial dashboard navigation + data wiring (nav-fixes).
 *
 *  - Quick-nav links to invoices / payments / budgets / reports.
 *  - Recent payments come from listPayments (limit 5) instead of a hard `[]`.
 *  - Overdue invoice "View" opens `/financial/invoices/:invoiceId`, which
 *    deep-links the InvoiceDetail modal (getInvoice) over the invoice list.
 *  - Building / date-range filters are hidden (no API support → inert).
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
    // Echo interpolation values so assertions can find the rendered data.
    t: (key: string, opts?: Record<string, unknown>) => {
      if (typeof opts?.defaultValue === 'string') return opts.defaultValue;
      const values = Object.values(opts ?? {}).filter((v) => typeof v !== 'object');
      return values.length ? `${key} ${values.join(' ')}` : key;
    },
    i18n: { language: 'en' },
  }),
}));

vi.mock('../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../components')>()),
  useToast: () => ({ showToast: vi.fn() }),
  // Role gating is covered by the ProtectedRoute tests; render children here.
  ProtectedRoute: ({ children }: { children: ReactNode }) => children,
}));

const authRole = vi.hoisted(() => ({ current: 'manager' }));

vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { id: 'user-1', organizationId: 'org-1', role: authRole.current } }),
}));

const overdue = {
  id: 'inv-9',
  invoice_number: 'INV-9',
  currency: 'EUR',
  balance_due: 120,
  due_date: '2026-09-01',
  status: 'overdue',
};

const mockListPayments = vi.fn();
const mockGetInvoice = vi.fn();

vi.mock('@ppt/api-client', () => ({
  getARAgingReport: vi.fn().mockResolvedValue({
    entries: [],
    totals: { current: 0, days_30: 0, days_60: 0, days_90_plus: 0, total: 0 },
  }),
  getOverdueInvoices: vi.fn(async () => [overdue]),
  listInvoices: vi.fn().mockResolvedValue({ invoices: [], total: 0 }),
  listPayments: (...args: unknown[]) => mockListPayments(...args),
  getInvoice: (...args: unknown[]) => mockGetInvoice(...args),
  listUnallocatedPayments: vi.fn().mockResolvedValue([]),
  allocatePayment: vi.fn(),
  autoMatchPayments: vi.fn(),
  getBalanceSheet: vi.fn(),
  getCashFlowReport: vi.fn(),
  getIncomeStatement: vi.fn(),
  downloadInvoicePdf: vi.fn(),
  exportReport: vi.fn(),
  sendInvoice: vi.fn(),
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
  authRole.current = 'manager';
  mockListPayments.mockReset().mockResolvedValue({
    payments: [
      {
        id: 'pay-1',
        amount: 250,
        currency: 'EUR',
        payment_date: '2026-10-01',
        payment_method: 'bank_transfer',
        status: 'completed',
        reference: 'REF-RECENT-1',
      },
    ],
    total: 1,
  });
  mockGetInvoice.mockReset().mockResolvedValue({
    invoice: {
      ...overdue,
      amount_paid: 0,
      subtotal: 120,
      tax_amount: 0,
      total: 120,
      issue_date: '2026-08-01',
    },
    items: [],
    payments: [],
  });
});

describe('FinancialDashboardPageRoute', () => {
  it('links to every finance sub-section', async () => {
    renderAt('/financial');
    const nav = await screen.findByRole('navigation', {
      name: 'financial.dashboard.quickNav.label',
    });
    const hrefs = Array.from(nav.querySelectorAll('a')).map((a) => a.getAttribute('href'));
    expect(hrefs).toEqual([
      '/financial/invoices',
      '/financial/payments',
      '/financial/budgets',
      '/financial/reports',
    ]);
  });

  it.each(['owner', 'tenant'])(
    'shows "%s" only the payments card (invoices, budgets, reports are manager-only)',
    async (role) => {
      authRole.current = role;
      renderAt('/financial');
      const nav = await screen.findByRole('navigation', {
        name: 'financial.dashboard.quickNav.label',
      });
      const hrefs = Array.from(nav.querySelectorAll('a')).map((a) => a.getAttribute('href'));
      expect(hrefs).toEqual(['/financial/payments']);
    }
  );

  it('shows recent payments from listPayments (limit 5)', async () => {
    renderAt('/financial');
    expect(await screen.findByText(/REF-RECENT-1/)).toBeInTheDocument();
    expect(mockListPayments).toHaveBeenCalledWith({ organization_id: 'org-1', limit: 5 });
  });

  it('hides the inert building filter', async () => {
    renderAt('/financial');
    await screen.findByRole('navigation', { name: 'financial.dashboard.quickNav.label' });
    expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
  });

  it('overdue "View" opens the invoice detail route', async () => {
    renderAt('/financial');
    await screen.findByText('INV-9');
    await userEvent.click(screen.getByRole('button', { name: /^(View|financial\.view)$/ }));
    await waitFor(() => expect(mockGetInvoice).toHaveBeenCalledWith('inv-9'));
    expect(await screen.findByText(/INV-9/, { selector: 'h2' })).toBeInTheDocument();
  });
});
