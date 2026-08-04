/// <reference types="vitest/globals" />
/**
 * PaymentManagementPageRoute mutation-failure toast wiring.
 *
 * The money-movement mutations in `routes/groups/financial.tsx`
 * (`allocatePayment` / `autoMatchPayments`) originally had `onSuccess` but no
 * `onError`, so a rejected reconciliation call produced ZERO user feedback —
 * the click looked like a no-op. These tests mount the PRODUCTION financial
 * route at `/financial/payments`, drive the real page's "Auto-Match All"
 * button, and assert the route container surfaces an error toast when the
 * underlying api-client call rejects.
 *
 * Only the api-client functions + `useToast` + `useAuth` are mocked; the real
 * PaymentManagementPage renders so the button → onAutoMatch → mutation →
 * onError → toast path is exercised end-to-end.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { financialRoutes } from './financial';

// ── i18n: resolve keys to their defaultValue so labels render (keep the rest of
//    the real module so the components barrel's i18n init still works) ──
vi.mock('react-i18next', async (importOriginal) => ({
  ...(await importOriginal<typeof import('react-i18next')>()),
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
  }),
}));

// ── toast: capture, keep every other component export real (Button/Card/…) ──
const mockShowToast = vi.fn();
vi.mock('../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../components')>()),
  useToast: () => ({ showToast: mockShowToast }),
}));

// ── auth: manager user with an org so the queries enable ──
vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { id: 'user-1', organizationId: 'org-1', role: 'manager' } }),
}));

// ── api-client: one unallocated payment (so the reconciliation UI + Auto-Match
//    button render) and an auto-match call that rejects. ──
const mockAutoMatchPayments = vi.fn();
const mockAllocatePayment = vi.fn();

vi.mock('@ppt/api-client', () => ({
  listPayments: vi.fn().mockResolvedValue({ payments: [], total: 0 }),
  listUnallocatedPayments: vi.fn().mockResolvedValue([
    {
      id: 'pay-1',
      amount: 250,
      currency: 'EUR',
      payment_date: '2026-07-01',
      payment_method: 'bank_transfer',
      reference: 'REF-1',
    },
  ]),
  listInvoices: vi.fn().mockResolvedValue({ invoices: [], total: 0 }),
  allocatePayment: (...args: unknown[]) => mockAllocatePayment(...args),
  autoMatchPayments: (...args: unknown[]) => mockAutoMatchPayments(...args),
  // Other functions imported by the financial module graph but never invoked
  // by the payments route under test.
  getARAgingReport: vi.fn(),
  getBalanceSheet: vi.fn(),
  getCashFlowReport: vi.fn(),
  getIncomeStatement: vi.fn(),
  getOverdueInvoices: vi.fn(),
  downloadInvoicePdf: vi.fn(),
  exportReport: vi.fn(),
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

describe('PaymentManagementPageRoute money-movement onError wiring', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('surfaces an error toast when auto-match (money movement) rejects', async () => {
    mockAutoMatchPayments.mockRejectedValue(new Error('auto-match boom'));

    renderPaymentsRoute();

    const autoMatchBtn = await screen.findByRole(
      'button',
      { name: /auto-match all/i },
      { timeout: 5000 }
    );
    await userEvent.click(autoMatchBtn);

    expect(mockAutoMatchPayments).toHaveBeenCalledWith('org-1');
    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'error' }));
    });
  });
});
