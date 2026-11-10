/// <reference types="vitest/globals" />
/**
 * Budget routes (Epic 24 / Story 52.4).
 *
 * `/financial/budgets` used to render a hard-coded empty list. These tests
 * mount the production route tables against a mocked `@ppt/api-client`
 * budgets module and check: the list renders server data and navigates to
 * create / detail, the create page posts the form and opens the new budget,
 * the detail page renders lines and drives the lifecycle actions, and the
 * create / detail screens are gated to manager roles.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Suspense } from 'react';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import type { AuthContextValue } from '../../contexts/AuthContext';
import { financialRoutes } from './financial';
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
  listBudgets: vi.fn(),
  getBudget: vi.fn(),
  createBudget: vi.fn(),
  deleteBudget: vi.fn(),
  submitBudget: vi.fn(),
  approveBudget: vi.fn(),
  activateBudget: vi.fn(),
  closeBudget: vi.fn(),
  getBudgetSummary: vi.fn(),
  listBudgetItems: vi.fn(),
  addBudgetItem: vi.fn(),
  deleteBudgetItem: vi.fn(),
  recordBudgetActual: vi.fn(),
  listBudgetCategories: vi.fn(),
  createBudgetCategory: vi.fn(),
}));

vi.mock('@ppt/api-client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@ppt/api-client')>()),
  ...api,
  useBuildings: () => ({
    data: { items: [{ id: 'bld-1', name: 'Tower A' }] },
    isLoading: false,
  }),
}));

const BUDGET = {
  id: 'bud-1',
  organizationId: 'org-1',
  buildingId: 'bld-1',
  fiscalYear: new Date().getFullYear(),
  name: 'Operating budget',
  status: 'draft' as const,
  totalAmount: 1000,
  notes: null,
  approvedBy: null,
  approvedAt: null,
  createdBy: 'user-1',
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const ITEM = {
  id: 'item-1',
  budgetId: 'bud-1',
  categoryId: 'cat-1',
  name: 'Stairwell cleaning',
  description: null,
  budgetedAmount: 1000,
  actualAmount: 250,
  varianceAmount: -750,
  variancePercent: -75,
  notes: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
};

const CATEGORY = {
  id: 'cat-1',
  organizationId: 'org-1',
  name: 'Services',
  description: null,
  parentId: null,
  sortOrder: 0,
  isSystem: false,
  createdAt: '2026-01-01T00:00:00Z',
};

const SUMMARY = {
  totalBudgeted: 1000,
  totalActual: 250,
  totalVariance: -750,
  variancePercent: -75,
  itemsOverBudget: 0,
  itemsUnderBudget: 1,
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
  return <div data-testid="location">{location.pathname}</div>;
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
            {financialRoutes()}
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
  api.listBudgets.mockResolvedValue([BUDGET]);
  api.getBudget.mockResolvedValue(BUDGET);
  api.getBudgetSummary.mockResolvedValue(SUMMARY);
  api.listBudgetItems.mockResolvedValue([ITEM]);
  api.listBudgetCategories.mockResolvedValue([CATEGORY]);
});

describe('budget create / detail routes — role gate', () => {
  it.each(['/financial/budgets/new', '/financial/budgets/bud-1'])(
    'denies a resident at %s',
    async (path) => {
      authState.current = authed('resident');
      renderAt(path);
      await waitFor(() => expect(screen.queryByText('loading…')).toBeNull());
      expect(screen.queryByRole('heading', { name: /new budget|operating budget/i })).toBeNull();
      expect(api.getBudget).not.toHaveBeenCalled();
      expect(api.createBudget).not.toHaveBeenCalled();
    }
  );

  it('sends anonymous visitors to login', () => {
    authState.current = authed(null);
    renderAt('/financial/budgets/bud-1');
    expect(screen.getByTestId('login-page')).toBeInTheDocument();
  });
});

describe('/financial/budgets', () => {
  it('lists budgets from the api with actual spend from the summary', async () => {
    renderAt('/financial/budgets');

    expect(await screen.findByText('Operating budget')).toBeInTheDocument();
    expect(api.listBudgets).toHaveBeenCalledWith({
      organizationId: 'org-1',
      fiscalYear: new Date().getFullYear(),
      buildingId: undefined,
    });
    await waitFor(() => expect(api.getBudgetSummary).toHaveBeenCalledWith('bud-1'));
    expect(screen.getByText('Draft')).toBeInTheDocument();
    expect(screen.getByText(`${BUDGET.fiscalYear} · Tower A`)).toBeInTheDocument();
  });

  it('refetches when the fiscal year filter changes', async () => {
    const user = userEvent.setup();
    renderAt('/financial/budgets');
    await screen.findByText('Operating budget');
    const year = new Date().getFullYear();
    await user.selectOptions(screen.getByLabelText('Fiscal year'), String(year + 1));
    await waitFor(() =>
      expect(api.listBudgets).toHaveBeenLastCalledWith(
        expect.objectContaining({ fiscalYear: year + 1 })
      )
    );
  });

  it('shows the empty state with a create button that opens the create page', async () => {
    api.listBudgets.mockResolvedValue([]);
    const user = userEvent.setup();
    renderAt('/financial/budgets');

    expect(await screen.findByText(/No budgets for/)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Create your first budget' }));
    await waitFor(() =>
      expect(screen.getByTestId('location')).toHaveTextContent('/financial/budgets/new')
    );
  });

  it('shows an error state with retry when the list fails', async () => {
    api.listBudgets.mockRejectedValueOnce(new Error('boom'));
    const user = userEvent.setup();
    renderAt('/financial/budgets');

    expect(await screen.findByText('Failed to load budgets')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Retry' }));
    expect(await screen.findByText('Operating budget')).toBeInTheDocument();
  });

  it('opens the detail page when a budget card is clicked', async () => {
    const user = userEvent.setup();
    renderAt('/financial/budgets');
    await user.click(await screen.findByRole('button', { name: /Operating budget/ }));
    await waitFor(() =>
      expect(screen.getByTestId('location')).toHaveTextContent('/financial/budgets/bud-1')
    );
  });
});

describe('/financial/budgets/new', () => {
  it('validates, creates the budget and navigates to its detail', async () => {
    api.createBudget.mockResolvedValue({ ...BUDGET, id: 'bud-new' });
    const user = userEvent.setup();
    renderAt('/financial/budgets/new');

    expect(await screen.findByRole('heading', { name: 'New budget' })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Create budget' }));
    expect(screen.getByText('Enter a budget name')).toBeInTheDocument();
    expect(api.createBudget).not.toHaveBeenCalled();

    await user.type(screen.getByLabelText(/^Budget name/), 'Operating budget 2027');
    await user.selectOptions(screen.getByLabelText(/^Building/), 'bld-1');
    await user.click(screen.getByRole('button', { name: 'Create budget' }));

    await waitFor(() =>
      expect(api.createBudget).toHaveBeenCalledWith({
        organizationId: 'org-1',
        name: 'Operating budget 2027',
        fiscalYear: new Date().getFullYear(),
        buildingId: 'bld-1',
        notes: null,
      })
    );
    await waitFor(() =>
      expect(screen.getByTestId('location')).toHaveTextContent('/financial/budgets/bud-new')
    );
    expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });
});

describe('/financial/budgets/:budgetId', () => {
  it('renders the budget with its items and totals', async () => {
    renderAt('/financial/budgets/bud-1');

    expect(await screen.findByRole('heading', { name: 'Operating budget' })).toBeInTheDocument();
    expect(api.getBudget).toHaveBeenCalledWith('org-1', 'bud-1');
    expect(screen.getByText('Stairwell cleaning')).toBeInTheDocument();
    const row = screen.getByText('Stairwell cleaning').closest('tr') as HTMLElement;
    expect(within(row).getByText('Services')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Submit for approval' })).toBeInTheDocument();
    // Lifecycle actions for later states are not offered on a draft.
    expect(screen.queryByRole('button', { name: 'Approve' })).toBeNull();
  });

  it('adds a line item in an existing category', async () => {
    api.addBudgetItem.mockResolvedValue(ITEM);
    const user = userEvent.setup();
    renderAt('/financial/budgets/bud-1');
    await screen.findByRole('heading', { name: 'Operating budget' });

    await user.type(screen.getByLabelText('Item'), 'Lift service');
    await user.type(screen.getByLabelText('Budgeted amount'), '1200.50');
    await user.click(screen.getByRole('button', { name: 'Add item' }));

    await waitFor(() =>
      expect(api.addBudgetItem).toHaveBeenCalledWith('bud-1', {
        categoryId: 'cat-1',
        name: 'Lift service',
        budgetedAmount: 1200.5,
        description: null,
      })
    );
    expect(api.createBudgetCategory).not.toHaveBeenCalled();
  });

  it('creates a category first when the organisation has none', async () => {
    api.listBudgetCategories.mockResolvedValue([]);
    api.listBudgetItems.mockResolvedValue([]);
    api.createBudgetCategory.mockResolvedValue({ ...CATEGORY, id: 'cat-new', name: 'Energy' });
    api.addBudgetItem.mockResolvedValue(ITEM);
    const user = userEvent.setup();
    renderAt('/financial/budgets/bud-1');
    await screen.findByRole('heading', { name: 'Operating budget' });

    await user.type(screen.getByLabelText('New category name'), 'Energy');
    await user.type(screen.getByLabelText('Item'), 'Common-area electricity');
    await user.type(screen.getByLabelText('Budgeted amount'), '800');
    await user.click(screen.getByRole('button', { name: 'Add item' }));

    await waitFor(() =>
      expect(api.createBudgetCategory).toHaveBeenCalledWith({
        organizationId: 'org-1',
        name: 'Energy',
      })
    );
    await waitFor(() =>
      expect(api.addBudgetItem).toHaveBeenCalledWith(
        'bud-1',
        expect.objectContaining({ categoryId: 'cat-new', budgetedAmount: 800 })
      )
    );
  });

  it('submits a draft for approval', async () => {
    api.submitBudget.mockResolvedValue({ ...BUDGET, status: 'pending_approval' });
    const user = userEvent.setup();
    renderAt('/financial/budgets/bud-1');
    await user.click(await screen.findByRole('button', { name: 'Submit for approval' }));
    await waitFor(() => expect(api.submitBudget).toHaveBeenCalledWith('org-1', 'bud-1'));
  });

  it('deletes a draft after confirmation and returns to the list', async () => {
    api.deleteBudget.mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderAt('/financial/budgets/bud-1');
    await user.click(await screen.findByRole('button', { name: 'Delete' }));
    expect(api.deleteBudget).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Confirm delete' }));
    await waitFor(() => expect(api.deleteBudget).toHaveBeenCalledWith('org-1', 'bud-1'));
    await waitFor(() =>
      expect(screen.getByTestId('location')).toHaveTextContent(/^\/financial\/budgets$/)
    );
  });

  it('records actual spend on an active budget and offers close', async () => {
    api.getBudget.mockResolvedValue({ ...BUDGET, status: 'active' });
    api.recordBudgetActual.mockResolvedValue({});
    const user = userEvent.setup();
    renderAt('/financial/budgets/bud-1');

    expect(await screen.findByRole('button', { name: 'Close budget' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Add item' })).toBeNull();
    await user.click(
      screen.getByRole('button', { name: 'Record actual spend for Stairwell cleaning' })
    );
    await user.type(screen.getByLabelText('Amount spent'), '99.90');
    await user.clear(screen.getByLabelText('Date'));
    await user.type(screen.getByLabelText('Date'), '2026-03-01');
    await user.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() =>
      expect(api.recordBudgetActual).toHaveBeenCalledWith('item-1', {
        amount: 99.9,
        transactionDate: '2026-03-01',
        description: null,
      })
    );
  });

  it.each([
    ['pending_approval', 'Approve', 'approveBudget'],
    ['approved', 'Activate', 'activateBudget'],
  ] as const)('a %s budget offers "%s"', async (status, label, fn) => {
    api.getBudget.mockResolvedValue({ ...BUDGET, status });
    api[fn].mockResolvedValue({ ...BUDGET });
    const user = userEvent.setup();
    renderAt('/financial/budgets/bud-1');
    await user.click(await screen.findByRole('button', { name: label }));
    await waitFor(() => expect(api[fn]).toHaveBeenCalledWith('org-1', 'bud-1'));
  });

  it('shows not-found when the budget cannot be loaded', async () => {
    api.getBudget.mockRejectedValue(new Error('404'));
    renderAt('/financial/budgets/missing');
    expect(await screen.findByText('Budget not found')).toBeInTheDocument();
  });
});
