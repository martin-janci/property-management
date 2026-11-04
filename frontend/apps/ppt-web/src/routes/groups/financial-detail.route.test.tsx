/// <reference types="vitest/globals" />
/**
 * Financial create / detail routes.
 *
 * `/financial/invoices` and `/financial/payments` navigate to
 * `/financial/invoices/new`, `/financial/invoices/:invoiceId` and
 * `/financial/payments/new`, but none of those paths were registered, so every
 * "New invoice" / invoice-row / "Record payment" click landed on not-found.
 * These tests mount the production `financialDetailRoutes()` table and check
 * each path renders its real component against mocked api-client calls and is
 * gated to manager roles. They also lock in that the budgets page no longer
 * offers create / detail buttons, which have no route behind them.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Suspense } from 'react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import type { AuthContextValue } from '../../contexts/AuthContext';
import { BudgetManagementPage } from '../../features/financial';
import { financialDetailRoutes } from './financial-detail';

const authState = vi.hoisted(() => ({ current: null as unknown as AuthContextValue }));

vi.mock('../../contexts/AuthContext', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts/AuthContext')>();
  return { ...actual, useAuth: () => authState.current };
});

const mockShowToast = vi.fn();
vi.mock('../../components', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../components')>()),
  useToast: () => ({ showToast: mockShowToast }),
}));

const api = vi.hoisted(() => ({
  createInvoice: vi.fn(),
  getInvoice: vi.fn(),
  sendInvoice: vi.fn(),
  downloadInvoicePdf: vi.fn(),
  recordPayment: vi.fn(),
  listInvoices: vi.fn(),
  listUnits: vi.fn(),
}));

vi.mock('@ppt/api-client', () => ({
  createInvoice: api.createInvoice,
  getInvoice: api.getInvoice,
  sendInvoice: api.sendInvoice,
  downloadInvoicePdf: api.downloadInvoicePdf,
  recordPayment: api.recordPayment,
  listInvoices: api.listInvoices,
  getOrg: () => 'org-1',
  getToken: () => 'token',
  useBuildings: () => ({
    data: { items: [{ id: 'bld-1', name: 'Tower A' }] },
    isLoading: false,
  }),
  createBuildingsApi: () => ({ listUnits: api.listUnits }),
}));

const INVOICE = {
  invoice: {
    id: 'inv-1',
    organization_id: 'org-1',
    unit_id: 'unit-1',
    invoice_number: 'INV-2026-001',
    status: 'draft',
    issue_date: '2026-10-01',
    due_date: '2026-11-01',
    currency: 'EUR',
    subtotal: 100,
    tax_amount: 0,
    total: 100,
    amount_paid: 0,
    balance_due: 100,
    notes: null,
    pdf_file_path: null,
    created_at: '2026-10-01T00:00:00Z',
    updated_at: '2026-10-01T00:00:00Z',
  },
  items: [
    {
      id: 'item-1',
      invoice_id: 'inv-1',
      description: 'Monthly service fee',
      quantity: 1,
      unit_price: 100,
      tax_rate: null,
      amount: 100,
    },
  ],
  payments: [],
};

function authed(role: string | null): AuthContextValue {
  return {
    login: vi.fn(),
    loginWithSsoCode: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    getAccessToken: () => null,
    setUser: vi.fn(),
    isAuthenticated: role !== null,
    isLoading: false,
    user:
      role === null
        ? null
        : ({
            id: 'user-1',
            email: 'manager@example.com',
            role,
            organizationId: 'org-1',
          } as AuthContextValue['user']),
  } as AuthContextValue;
}

function LocationProbe() {
  const location = useLocation();
  return <div data-testid="location">{`${location.pathname}${location.search}`}</div>;
}

function renderAt(entry: string) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[entry]}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>
            <Route path="/login" element={<div data-testid="login-page" />} />
            <Route path="/financial/invoices" element={<div data-testid="invoices-list" />} />
            <Route path="/financial/payments" element={<div data-testid="payments-list" />} />
            {financialDetailRoutes()}
          </Routes>
          <LocationProbe />
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  authState.current = authed('manager');
  api.listUnits.mockResolvedValue({
    units: [{ id: 'unit-1', building_id: 'bld-1', designation: '101' }],
    total: 1,
    offset: 0,
    limit: 100,
  });
  api.listInvoices.mockResolvedValue({ invoices: [INVOICE.invoice], total: 1 });
  api.getInvoice.mockResolvedValue(INVOICE);
});

describe('financial create / detail routes — role gate', () => {
  it.each(['/financial/invoices/new', '/financial/invoices/inv-1', '/financial/payments/new'])(
    'denies a resident at %s',
    async (path) => {
      authState.current = authed('resident');
      renderAt(path);
      // Give lazy chunks a chance to resolve before asserting absence.
      await waitFor(() => expect(screen.queryByText('loading…')).toBeNull());
      expect(screen.queryByRole('heading', { name: /new invoice|record payment/i })).toBeNull();
      expect(screen.queryByText('Monthly service fee')).toBeNull();
      expect(api.getInvoice).not.toHaveBeenCalled();
    }
  );

  it('sends anonymous visitors to login', () => {
    authState.current = authed(null);
    renderAt('/financial/invoices/new');
    expect(screen.getByTestId('login-page')).toBeInTheDocument();
  });
});

describe('/financial/invoices/new', () => {
  it('renders the invoice form and creates an invoice, then opens its detail', async () => {
    api.createInvoice.mockResolvedValue(INVOICE);
    const user = userEvent.setup();
    renderAt('/financial/invoices/new');

    expect(await screen.findByRole('heading', { name: 'New invoice' })).toBeInTheDocument();
    const unitSelect = await screen.findByLabelText(/^Unit \*$/);
    expect(screen.getByRole('option', { name: 'Tower A - 101' })).toBeInTheDocument();

    await user.selectOptions(unitSelect, 'unit-1');
    await user.type(screen.getByLabelText(/^Due date/), '2026-12-01');
    await user.type(screen.getAllByLabelText('Description')[0], 'Monthly service fee');
    await user.click(screen.getByRole('button', { name: 'Create invoice' }));

    await waitFor(() =>
      expect(api.createInvoice).toHaveBeenCalledWith(
        'org-1',
        'user-1',
        expect.objectContaining({ unit_id: 'unit-1', due_date: '2026-12-01' })
      )
    );
    await waitFor(() =>
      expect(screen.getByTestId('location')).toHaveTextContent('/financial/invoices/inv-1')
    );
    expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });
});

describe('/financial/invoices/:invoiceId', () => {
  it('renders the invoice detail and routes "Record payment" to the payment form', async () => {
    const user = userEvent.setup();
    renderAt('/financial/invoices/inv-1');

    expect(
      await screen.findByRole('heading', { name: 'Invoice INV-2026-001' })
    ).toBeInTheDocument();
    expect(api.getInvoice).toHaveBeenCalledWith('inv-1');
    expect(screen.getByText('Monthly service fee')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Record payment' }));
    await waitFor(() =>
      expect(screen.getByTestId('location')).toHaveTextContent(
        '/financial/payments/new?unitId=unit-1'
      )
    );
  });

  it('shows a not-found message when the invoice cannot be loaded', async () => {
    api.getInvoice.mockRejectedValue(new Error('404'));
    renderAt('/financial/invoices/missing');
    expect(await screen.findByText('Invoice not found')).toBeInTheDocument();
  });
});

describe('/financial/payments/new', () => {
  it('renders the payment form (unit preselected from ?unitId) and records a payment', async () => {
    api.recordPayment.mockResolvedValue({ payment: { id: 'pay-1' }, allocations: [] });
    const user = userEvent.setup();
    renderAt('/financial/payments/new?unitId=unit-1');

    expect(await screen.findByRole('heading', { name: 'Record payment' })).toBeInTheDocument();
    const unitSelect = (await screen.findByLabelText(/^Unit \*$/)) as HTMLSelectElement;
    expect(unitSelect.value).toBe('unit-1');
    // The unit's open invoice is offered for allocation.
    expect(screen.getByText('INV-2026-001')).toBeInTheDocument();

    await user.type(screen.getByLabelText(/^Amount/), '100');
    await user.selectOptions(screen.getByLabelText(/^Payment method/), 'bank_transfer');
    await user.click(screen.getByRole('button', { name: 'Record payment' }));

    await waitFor(() =>
      expect(api.recordPayment).toHaveBeenCalledWith(
        'org-1',
        'user-1',
        expect.objectContaining({ unit_id: 'unit-1', amount: 100, payment_method: 'bank_transfer' })
      )
    );
    await waitFor(() =>
      expect(screen.getByTestId('location')).toHaveTextContent('/financial/payments')
    );
  });
});

describe('BudgetManagementPage without create/detail handlers', () => {
  it('hides the create buttons (budget create/detail screens are not routed)', () => {
    render(
      <BudgetManagementPage
        budgets={[]}
        currentYear={2026}
        summary={{ totalBudget: 0, totalActual: 0, overallVariance: 0, currency: 'EUR' }}
        buildings={[]}
        onYearChange={() => {}}
        onBuildingChange={() => {}}
      />
    );
    expect(screen.queryByRole('button', { name: /create/i })).toBeNull();
  });
});
