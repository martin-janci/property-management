/**
 * Leases route group — lease lifecycle actions + URL-persisted list filters.
 *
 * Previously Edit / Renew / Terminate / Add amendment on the lease detail page
 * just navigated to the same page, the leases + applications list filters were
 * `() => {}` no-ops and the violations "Export report" button did nothing.
 * These tests render the real route wrappers + feature pages (MemoryRouter +
 * QueryClientProvider, `@ppt/api-client` hooks mocked) and pin:
 *  - each lifecycle action calls its mutation with the lease id + request body
 *    (URL + method are pinned by `packages/api-client/src/leases/api.test.ts`),
 *    shows success / error toasts, and closed leases hide the actions;
 *  - list filters reach the server query (`status`, `expiring_within_days`,
 *    `unit_id`, `limit`/`offset`) and round-trip through the URL;
 *  - the export button is hidden (no endpoint).
 */

/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { Suspense } from 'react';
import { MemoryRouter, Routes, useLocation } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    i18n: { language: 'en' },
  }),
}));

const mockShowToast = vi.hoisted(() => vi.fn());
vi.mock('../../components', () => ({
  useToast: () => ({ showToast: mockShowToast }),
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { role: 'manager' } }),
}));

vi.mock('../../hooks/useOrganization', () => ({
  useOrganization: () => ({ organizationId: 'org-1', organizationName: 'Správa Domov s.r.o.' }),
}));

const { mutations, mockUseLease, mockUseLeases, mockUseApplications, mockListBuildingUnits } =
  vi.hoisted(() => ({
    mutations: {
      updateLease: vi.fn(),
      renewLease: vi.fn(),
      terminateLease: vi.fn(),
      createAmendment: vi.fn(),
    },
    mockUseLease: vi.fn(),
    mockUseLeases: vi.fn(),
    mockUseApplications: vi.fn(),
    mockListBuildingUnits: vi.fn(),
  }));

const mutationHook = (fn: ReturnType<typeof vi.fn>) => () => ({
  mutateAsync: fn,
  isPending: false,
});

vi.mock('@ppt/api-client', () => {
  const idle = () => ({ mutateAsync: vi.fn(), isPending: false });
  return {
    listBuildingUnits: (...a: unknown[]) => mockListBuildingUnits(...a),
    createUploadUrl: vi.fn(),
    uploadFileToPresignedUrl: vi.fn(),
    useBuildings: () => ({
      data: { items: [{ id: 'b-1', name: 'Bytový dom Hlavná 5' }] },
      isLoading: false,
    }),
    useOrganizationMembers: () => ({ data: { members: [], total: 0 }, isLoading: false }),
    useLease: (...a: unknown[]) => mockUseLease(...a),
    useLeases: (...a: unknown[]) => mockUseLeases(...a),
    useLeaseStatistics: () => ({ data: undefined, isLoading: false }),
    useExpiringLeases: () => ({ data: undefined, isLoading: false }),
    useLeaseTemplates: () => ({ data: [], isLoading: false }),
    useApplication: () => ({ data: undefined, isLoading: false }),
    useApplications: (...a: unknown[]) => mockUseApplications(...a),
    useViolation: () => ({ data: undefined, isLoading: false }),
    useViolations: () => ({ data: [], isLoading: false }),
    useViolationEvidence: () => ({ data: [] }),
    useCreateLease: idle,
    useRecordLeasePayment: idle,
    useCreateLeaseTemplate: idle,
    useUpdateLeaseTemplate: idle,
    useInitiateScreening: idle,
    useReviewApplication: idle,
    useCreateViolation: idle,
    useUpdateViolation: idle,
    useCreateViolationAppeal: idle,
    useAddViolationEvidence: idle,
    useUpdateLease: () => mutationHook(mutations.updateLease)(),
    useRenewLease: () => mutationHook(mutations.renewLease)(),
    useTerminateLease: () => mutationHook(mutations.terminateLease)(),
    useCreateLeaseAmendment: () => mutationHook(mutations.createAmendment)(),
  };
});

import { leaseRoutes, readLeaseListParams, toLeaseListQuery, writeLeaseListParams } from './leases';

function LocationProbe() {
  const location = useLocation();
  return <div data-testid="location">{`${location.pathname}${location.search}`}</div>;
}

function renderAt(path: string) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[path]}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{leaseRoutes()}</Routes>
        </Suspense>
        <LocationProbe />
      </MemoryRouter>
    </QueryClientProvider>
  );
}

const location = () => screen.getByTestId('location').textContent;

function apiLeaseDetails(status = 'active') {
  return {
    lease: {
      id: 'lease-1',
      organization_id: 'org-1',
      unit_id: 'unit-1',
      tenant_user_id: 'u-tenant',
      tenant_name: 'Ján Novák',
      tenant_email: 'jan@example.sk',
      tenant_phone: '+421900111222',
      status,
      start_date: '2026-01-01',
      end_date: '2026-12-31',
      term_months: 12,
      monthly_rent: '850.00',
      security_deposit: '1700.00',
      rent_due_day: 5,
      parking_spaces: 1,
      storage_units: 0,
      pets_allowed: false,
      smoking_allowed: false,
      max_occupants: 3,
      notes: 'Kľúče odovzdané',
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
    },
    unit_name: '3B',
    building_name: 'Bytový dom Hlavná 5',
    amendments: [],
    upcoming_payments: [],
    reminders: [],
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  for (const fn of Object.values(mutations)) fn.mockResolvedValue({});
  mockUseLease.mockReturnValue({ data: apiLeaseDetails(), isLoading: false });
  mockUseLeases.mockReturnValue({ data: { items: [], total: 0 }, isLoading: false });
  mockUseApplications.mockReturnValue({ data: { items: [], total: 0 }, isLoading: false });
  mockListBuildingUnits.mockResolvedValue({
    units: [{ id: 'unit-1', building_id: 'b-1', designation: '3B' }],
    total: 1,
    offset: 0,
    limit: 100,
  });
});

// ============================================================================
// Lease detail — lifecycle actions
// ============================================================================

describe('LeaseDetailPageRoute — edit (PUT /leases/{id})', () => {
  it('pre-fills the dialog and sends the UpdateLease body', async () => {
    renderAt('/leases/lease-1');
    fireEvent.click(await screen.findByRole('button', { name: 'common.edit' }));
    const dialog = screen.getByRole('dialog', { name: 'leases.actions.edit.title' });
    expect(within(dialog).getByLabelText('leases.actions.edit.tenantPhone')).toHaveValue(
      '+421900111222'
    );
    fireEvent.change(within(dialog).getByLabelText('leases.actions.edit.parkingSpaces'), {
      target: { value: '2' },
    });
    fireEvent.click(within(dialog).getByLabelText('leases.actions.edit.petsAllowed'));
    fireEvent.click(within(dialog).getByRole('button', { name: 'leases.actions.edit.submit' }));

    await waitFor(() => expect(mutations.updateLease).toHaveBeenCalledTimes(1));
    expect(mutations.updateLease).toHaveBeenCalledWith({
      id: 'lease-1',
      body: {
        tenant_phone: '+421900111222',
        parking_spaces: 2,
        storage_units: 0,
        max_occupants: 3,
        pets_allowed: true,
        smoking_allowed: false,
        notes: 'Kľúče odovzdané',
      },
    });
    await waitFor(() =>
      expect(screen.queryByRole('dialog', { name: 'leases.actions.edit.title' })).toBeNull()
    );
    expect(mockShowToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'success', message: 'leases.actions.edit.success' })
    );
  });

  it('rejects a negative count without calling the API', async () => {
    renderAt('/leases/lease-1');
    fireEvent.click(await screen.findByRole('button', { name: 'common.edit' }));
    fireEvent.change(screen.getByLabelText('leases.actions.edit.storageUnits'), {
      target: { value: '-1' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'leases.actions.edit.submit' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('leases.actions.edit.invalidNumber');
    expect(mutations.updateLease).not.toHaveBeenCalled();
  });

  it('keeps the dialog open and shows an error toast when the update fails', async () => {
    mutations.updateLease.mockRejectedValueOnce(new Error('Forbidden'));
    renderAt('/leases/lease-1');
    fireEvent.click(await screen.findByRole('button', { name: 'common.edit' }));
    fireEvent.click(screen.getByRole('button', { name: 'leases.actions.edit.submit' }));
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', message: 'Forbidden' })
      )
    );
    expect(screen.getByRole('dialog', { name: 'leases.actions.edit.title' })).toBeInTheDocument();
  });
});

describe('LeaseDetailPageRoute — renew (POST /leases/{id}/renew)', () => {
  it('defaults to a 12-month renewal at the current rent and opens the new lease', async () => {
    mutations.renewLease.mockResolvedValueOnce({ id: 'lease-2' });
    renderAt('/leases/lease-1');
    fireEvent.click(await screen.findByRole('button', { name: 'leases.renew' }));
    const dialog = screen.getByRole('dialog', { name: 'leases.actions.renew.title' });
    expect(within(dialog).getByLabelText('leases.actions.renew.newEndDate')).toHaveValue(
      '2027-12-31'
    );
    fireEvent.change(within(dialog).getByLabelText(/leases\.actions\.renew\.newMonthlyRent/), {
      target: { value: '900' },
    });
    fireEvent.click(within(dialog).getByRole('button', { name: 'leases.actions.renew.submit' }));

    await waitFor(() => expect(mutations.renewLease).toHaveBeenCalledTimes(1));
    expect(mutations.renewLease).toHaveBeenCalledWith({
      id: 'lease-1',
      body: {
        new_end_date: '2027-12-31',
        new_monthly_rent: 900,
        new_security_deposit: undefined,
        term_months: 12,
        notes: undefined,
      },
    });
    await waitFor(() => expect(location()).toBe('/leases/lease-2'));
  });

  it('rejects an end date that does not extend the lease', async () => {
    renderAt('/leases/lease-1');
    fireEvent.click(await screen.findByRole('button', { name: 'leases.renew' }));
    fireEvent.change(screen.getByLabelText('leases.actions.renew.newEndDate'), {
      target: { value: '2026-12-15' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'leases.actions.renew.submit' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'leases.actions.renew.endDateInvalid'
    );
    expect(mutations.renewLease).not.toHaveBeenCalled();
  });

  it('shows an error toast when the renewal fails', async () => {
    mutations.renewLease.mockRejectedValueOnce(new Error('Failed to renew lease'));
    renderAt('/leases/lease-1');
    fireEvent.click(await screen.findByRole('button', { name: 'leases.renew' }));
    fireEvent.click(screen.getByRole('button', { name: 'leases.actions.renew.submit' }));
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', message: 'Failed to renew lease' })
      )
    );
    expect(location()).toBe('/leases/lease-1');
  });

  it('opens the renew dialog from a ?action=renew deep link and clears the param', async () => {
    renderAt('/leases/lease-1?action=renew');
    expect(
      await screen.findByRole('dialog', { name: 'leases.actions.renew.title' })
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'common.cancel' }));
    await waitFor(() => expect(location()).toBe('/leases/lease-1'));
  });
});

describe('LeaseDetailPageRoute — terminate (POST /leases/{id}/terminate)', () => {
  async function fillTerminateForm() {
    renderAt('/leases/lease-1');
    fireEvent.click(await screen.findByRole('button', { name: 'leases.terminate' }));
    fireEvent.change(screen.getByLabelText('leases.actions.terminate.reason'), {
      target: { value: 'mutual_agreement' },
    });
    fireEvent.change(screen.getByLabelText('leases.actions.terminate.effectiveDate'), {
      target: { value: '2026-11-30' },
    });
    fireEvent.change(screen.getByLabelText('leases.detail.notes'), {
      target: { value: 'Dohoda o skončení nájmu' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'leases.actions.terminate.continue' }));
  }

  it('requires a reason', async () => {
    renderAt('/leases/lease-1');
    fireEvent.click(await screen.findByRole('button', { name: 'leases.terminate' }));
    fireEvent.click(screen.getByRole('button', { name: 'leases.actions.terminate.continue' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'leases.actions.terminate.reasonRequired'
    );
    expect(mutations.terminateLease).not.toHaveBeenCalled();
  });

  it('asks for confirmation, then sends the TerminateLease body', async () => {
    await fillTerminateForm();
    const confirm = await screen.findByRole('alertdialog');
    expect(mutations.terminateLease).not.toHaveBeenCalled();
    fireEvent.click(
      within(confirm).getByRole('button', { name: 'leases.actions.terminate.confirm' })
    );
    await waitFor(() => expect(mutations.terminateLease).toHaveBeenCalledTimes(1));
    expect(mutations.terminateLease).toHaveBeenCalledWith({
      id: 'lease-1',
      body: {
        termination_reason: 'mutual_agreement',
        termination_notes: 'Dohoda o skončení nájmu',
        effective_date: '2026-11-30',
      },
    });
    await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull());
    expect(mockShowToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'success', message: 'leases.actions.terminate.success' })
    );
  });

  it('going back from the confirmation keeps the filled form', async () => {
    await fillTerminateForm();
    const confirm = await screen.findByRole('alertdialog');
    fireEvent.click(within(confirm).getByRole('button', { name: 'common.back' }));
    expect(screen.getByLabelText('leases.actions.terminate.reason')).toHaveValue(
      'mutual_agreement'
    );
    expect(mutations.terminateLease).not.toHaveBeenCalled();
  });

  it('shows an error toast when termination fails', async () => {
    mutations.terminateLease.mockRejectedValueOnce(new Error('Failed to terminate lease'));
    await fillTerminateForm();
    fireEvent.click(
      within(await screen.findByRole('alertdialog')).getByRole('button', {
        name: 'leases.actions.terminate.confirm',
      })
    );
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', message: 'Failed to terminate lease' })
      )
    );
  });
});

describe('LeaseDetailPageRoute — add amendment (POST /leases/{id}/amendments)', () => {
  async function openAmendment() {
    renderAt('/leases/lease-1');
    fireEvent.click(
      await screen.findByRole('button', { name: /leases\.detail\.tabs\.amendments/ })
    );
    fireEvent.click(screen.getByRole('button', { name: 'leases.detail.addAmendment' }));
  }

  it('sends title, description, date and the rent change', async () => {
    await openAmendment();
    fireEvent.change(screen.getByLabelText('leases.actions.amendment.amendmentTitle'), {
      target: { value: 'Úprava nájomného' },
    });
    fireEvent.change(screen.getByLabelText('leases.actions.amendment.description'), {
      target: { value: 'Indexácia podľa inflácie' },
    });
    fireEvent.change(screen.getByLabelText('leases.detail.effectiveDate'), {
      target: { value: '2027-01-01' },
    });
    fireEvent.change(screen.getByLabelText(/leases\.actions\.amendment\.newMonthlyRent/), {
      target: { value: '880' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'leases.actions.amendment.submit' }));
    await waitFor(() => expect(mutations.createAmendment).toHaveBeenCalledTimes(1));
    expect(mutations.createAmendment).toHaveBeenCalledWith({
      id: 'lease-1',
      body: {
        title: 'Úprava nájomného',
        description: 'Indexácia podľa inflácie',
        effective_date: '2027-01-01',
        changes: { monthly_rent: '880' },
      },
    });
  });

  it('requires a title', async () => {
    await openAmendment();
    fireEvent.click(screen.getByRole('button', { name: 'leases.actions.amendment.submit' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'leases.actions.amendment.titleRequired'
    );
    expect(mutations.createAmendment).not.toHaveBeenCalled();
  });

  it('shows an error toast when the amendment fails', async () => {
    mutations.createAmendment.mockRejectedValueOnce(new Error('Failed to create amendment'));
    await openAmendment();
    fireEvent.change(screen.getByLabelText('leases.actions.amendment.amendmentTitle'), {
      target: { value: 'Parkovanie' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'leases.actions.amendment.submit' }));
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', message: 'Failed to create amendment' })
      )
    );
  });
});

describe('LeaseDetailPageRoute — closed leases', () => {
  it('hides edit / renew / terminate on a terminated lease and ignores ?action=', async () => {
    mockUseLease.mockReturnValue({ data: apiLeaseDetails('terminated'), isLoading: false });
    renderAt('/leases/lease-1?action=terminate');
    expect(await screen.findByText('leases.status.terminated')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'common.edit' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'leases.renew' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'leases.terminate' })).toBeNull();
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('renders the translated not-found notice', async () => {
    mockUseLease.mockReturnValue({ data: undefined, isLoading: false });
    renderAt('/leases/lease-1');
    expect(await screen.findByText('leases.notice.notFound.lease')).toBeInTheDocument();
    expect(screen.getByText('leases.notice.notFoundBody.lease')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'leases.notice.back.lease' }));
    expect(location()).toBe('/leases/list');
  });
});

// ============================================================================
// List filters
// ============================================================================

describe('LeasesListPageRoute — filters', () => {
  const lastLeasesQuery = () => mockUseLeases.mock.calls.at(-1)?.[0];

  it('fetches the first page with no filters by default', async () => {
    renderAt('/leases/list');
    await screen.findByText('leases.list.title');
    expect(lastLeasesQuery()).toEqual({
      organization_id: 'org-1',
      status: undefined,
      expiring_within_days: undefined,
      limit: 12,
      offset: 0,
    });
  });

  it('maps the status filter to the backend value and persists it in the URL', async () => {
    renderAt('/leases/list');
    fireEvent.change(await screen.findByLabelText('leases.list.filterByStatus'), {
      target: { value: 'pending_signatures' },
    });
    await waitFor(() => expect(location()).toBe('/leases/list?status=pending_signatures'));
    expect(lastLeasesQuery()).toEqual(
      expect.objectContaining({ status: 'pending_signature', offset: 0 })
    );
  });

  it('restores filters + page from the URL', async () => {
    renderAt('/leases/list?status=active&expiring=60&page=3');
    expect(await screen.findByLabelText('leases.list.filterByStatus')).toHaveValue('active');
    expect(screen.getByLabelText('leases.list.filterByExpiry')).toHaveValue('60');
    expect(lastLeasesQuery()).toEqual({
      organization_id: 'org-1',
      status: 'active',
      expiring_within_days: 60,
      limit: 12,
      offset: 24,
    });
  });

  it('pages through the server results', async () => {
    mockUseLeases.mockReturnValue({
      data: {
        total: 30,
        items: [
          {
            id: 'lease-1',
            unit_id: 'unit-1',
            unit_name: '3B',
            building_name: 'Bytový dom Hlavná 5',
            tenant_name: 'Ján Novák',
            tenant_email: 'jan@example.sk',
            start_date: '2026-01-01',
            end_date: '2026-12-31',
            monthly_rent: '850.00',
            status: 'active',
            days_until_expiry: 57,
          },
        ],
      },
      isLoading: false,
    });
    renderAt('/leases/list');
    fireEvent.click(await screen.findByRole('button', { name: 'common.next' }));
    await waitFor(() => expect(location()).toBe('/leases/list?page=2'));
    expect(lastLeasesQuery()).toEqual(expect.objectContaining({ offset: 12 }));
  });

  it('hides search and building filters the backend cannot apply', async () => {
    renderAt('/leases/list');
    await screen.findByText('leases.list.title');
    expect(screen.queryByPlaceholderText('leases.list.searchPlaceholder')).toBeNull();
    expect(screen.queryByLabelText('leases.list.filterByBuilding')).toBeNull();
  });

  it('routes card renew / terminate to the detail dialog deep link', async () => {
    mockUseLeases.mockReturnValue({
      data: {
        total: 1,
        items: [
          {
            id: 'lease-1',
            unit_id: 'unit-1',
            unit_name: '3B',
            building_name: 'Bytový dom Hlavná 5',
            tenant_name: 'Ján Novák',
            tenant_email: 'jan@example.sk',
            start_date: '2026-01-01',
            end_date: '2026-12-31',
            monthly_rent: '850.00',
            status: 'active',
            days_until_expiry: 57,
          },
        ],
      },
      isLoading: false,
    });
    renderAt('/leases/list');
    fireEvent.click(await screen.findByRole('button', { name: 'leases.terminate' }));
    await screen.findByRole('dialog', { name: 'leases.actions.terminate.title' });
  });
});

describe('lease list URL helpers', () => {
  it('round-trips filters and drops unknown values', () => {
    const { filters, page } = readLeaseListParams(
      new URLSearchParams('status=bogus&expiring=-5&page=0')
    );
    expect(filters).toEqual({ status: undefined, expiringWithinDays: undefined });
    expect(page).toBe(1);
    expect(
      writeLeaseListParams({
        status: 'active',
        expiringWithinDays: 30,
        page: 2,
        pageSize: 12,
      }).toString()
    ).toBe('status=active&expiring=30&page=2');
  });

  it('turns expiring_soon into active leases ending within 90 days', () => {
    expect(toLeaseListQuery({ status: 'expiring_soon' }, 1)).toEqual({
      status: 'active',
      expiring_within_days: 90,
      limit: 12,
      offset: 0,
    });
    expect(toLeaseListQuery({ status: 'expiring_soon', expiringWithinDays: 30 }, 2)).toEqual(
      expect.objectContaining({ expiring_within_days: 30, offset: 12 })
    );
  });
});

describe('ApplicationsPageRoute — filters', () => {
  const lastQuery = () => mockUseApplications.mock.calls.at(-1)?.[0];

  it('sends status + unit filters to the server and persists them in the URL', async () => {
    renderAt('/leases/applications');
    const unitSelect = await screen.findByLabelText('leases.applications.filterByUnit');
    await screen.findByRole('option', { name: /3B/ });
    fireEvent.change(screen.getByLabelText('leases.applications.filterByStatus'), {
      target: { value: 'screening' },
    });
    await waitFor(() => expect(location()).toBe('/leases/applications?status=screening'));
    fireEvent.change(unitSelect, { target: { value: 'unit-1' } });
    await waitFor(() =>
      expect(location()).toBe('/leases/applications?status=screening&unit=unit-1')
    );
    expect(lastQuery()).toEqual({
      organization_id: 'org-1',
      status: 'screening_pending',
      unit_id: 'unit-1',
      limit: 12,
      offset: 0,
    });
  });

  it('restores filters from the URL and hides the unsupported search', async () => {
    renderAt('/leases/applications?status=approved&page=2');
    expect(await screen.findByLabelText('leases.applications.filterByStatus')).toHaveValue(
      'approved'
    );
    expect(screen.queryByPlaceholderText('leases.applications.searchPlaceholder')).toBeNull();
    expect(lastQuery()).toEqual(
      expect.objectContaining({ status: 'approved', unit_id: undefined, offset: 12 })
    );
  });
});

describe('ViolationsPageRoute', () => {
  it('hides the export button (no export endpoint)', async () => {
    renderAt('/leases/violations');
    await screen.findByText('leases.violations.title');
    expect(screen.queryByText('leases.violations.exportReport')).toBeNull();
  });
});
