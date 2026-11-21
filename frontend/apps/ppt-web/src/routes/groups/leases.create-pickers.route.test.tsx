/**
 * Leases route group — Create Lease picker loading / empty / error states.
 *
 * The create-lease unit + tenant pickers are fed by live queries
 * (`useBuildings` → per-building `listBuildingUnits`, and
 * `useOrganizationMembers`). This pins that the route surfaces the three
 * non-happy states instead of rendering silent empty dropdowns:
 *  - a retryable error notice when a required picker query fails,
 *  - an inline hint under the unit picker when no units exist,
 *  - an inline hint under the tenant picker when no eligible tenants exist.
 */

/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    i18n: { language: 'en' },
  }),
}));

vi.mock('../../components', () => ({
  useToast: () => ({ showToast: vi.fn() }),
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { role: 'manager' } }),
}));

vi.mock('../../hooks/useOrganization', () => ({
  useOrganization: () => ({ organizationId: 'org-1', organizationName: 'Org s.r.o.' }),
}));

// Configurable picker-source mocks (driven per test via these refs).
const state = vi.hoisted(() => ({
  buildings: { data: undefined as unknown, isLoading: false, isError: false, refetch: vi.fn() },
  members: { data: undefined as unknown, isLoading: false, isError: false, refetch: vi.fn() },
  units: [] as Array<{ id: string; building_id: string; designation: string }>,
}));
const mockListBuildingUnits = vi.hoisted(() => vi.fn());

vi.mock('@ppt/api-client', () => ({
  useBuildings: () => state.buildings,
  useOrganizationMembers: () => state.members,
  useLeaseTemplates: () => ({ data: [], isLoading: false, isError: false, refetch: vi.fn() }),
  listBuildingUnits: (...args: unknown[]) => mockListBuildingUnits(...args),
  useCreateLease: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

import { leaseRoutes } from './leases';

function renderNew() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={['/leases/new']}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{leaseRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  state.buildings = { data: { items: [] }, isLoading: false, isError: false, refetch: vi.fn() };
  state.members = {
    data: { total: 0, members: [] },
    isLoading: false,
    isError: false,
    refetch: vi.fn(),
  };
  mockListBuildingUnits.mockResolvedValue({ units: [], total: 0, offset: 0, limit: 100 });
});

describe('CreateLeasePageRoute — picker states', () => {
  it('shows a retryable error notice when the buildings (unit) query fails', async () => {
    state.buildings = { data: undefined, isLoading: false, isError: true, refetch: vi.fn() };

    renderNew();

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Could not load the unit and tenant options');
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
    // The form itself must not render while the pickers are in an error state.
    expect(screen.queryByLabelText(/leases\.form\.unit/)).not.toBeInTheDocument();
  });

  it('shows a retryable error notice when the members (tenant) query fails', async () => {
    state.members = { data: undefined, isLoading: false, isError: true, refetch: vi.fn() };

    renderNew();

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Could not load the unit and tenant options'
    );
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
  });

  it('renders the form with empty-state hints when no units or tenants exist', async () => {
    renderNew();

    // Form is rendered (loading + error both false), pickers present but empty.
    const unitSelect = await screen.findByLabelText(/leases\.form\.unit/);
    expect(unitSelect).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByText('leases.form.noUnits')).toBeInTheDocument();
    expect(screen.getByText('leases.form.noTenants')).toBeInTheDocument();
  });
});
