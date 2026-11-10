/**
 * Leases route group — action wiring tests.
 *
 * Each previously no-op lease action is now wired to its backend mutation.
 * These tests render the real route wrappers + feature pages (MemoryRouter +
 * QueryClientProvider, `@ppt/api-client` hooks mocked) and pin, per action:
 *  - the mutation is called with the right ids + request body,
 *  - success / error feedback goes through the toast,
 *  - controls without a backend endpoint (template delete) are hidden.
 *
 * Endpoint URL + method for each mutation are pinned by the api-client
 * contract tests (`packages/api-client/src/leases/api.test.ts`).
 */

/// <reference types="vitest/globals" />
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// ── i18n: return the key (or defaultValue) so assertions are locale-agnostic ──
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    i18n: { language: 'en' },
  }),
}));

// ── toast + route guard ──
const mockShowToast = vi.hoisted(() => vi.fn());
vi.mock('../../components', () => ({
  useToast: () => ({ showToast: mockShowToast }),
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// ── auth role ──
const roleRef = vi.hoisted(() => ({ role: 'manager' }));
vi.mock('../../contexts', () => ({
  useAuth: () => ({ user: { role: roleRef.role } }),
}));

// ── organization ──
vi.mock('../../hooks/useOrganization', () => ({
  useOrganization: () => ({ organizationId: 'org-1', organizationName: 'Správa Domov s.r.o.' }),
}));

// ── api-client ──
const {
  mutations,
  mockUseLease,
  mockUseLeaseTemplates,
  mockUseApplication,
  mockUseViolation,
  mockUseViolationEvidence,
  mockListBuildingUnits,
  mockCreateUploadUrl,
  mockUploadToPresigned,
  mutationHook,
} = vi.hoisted(() => {
  const mutations = {
    createLease: vi.fn(),
    recordPayment: vi.fn(),
    createTemplate: vi.fn(),
    updateTemplate: vi.fn(),
    initiateScreening: vi.fn(),
    reviewApplication: vi.fn(),
    updateViolation: vi.fn(),
    createAppeal: vi.fn(),
    addEvidence: vi.fn(),
  };
  const mutationHook = (fn: ReturnType<typeof vi.fn>) => () => ({
    mutateAsync: fn,
    isPending: false,
  });
  return {
    mutations,
    mutationHook,
    mockUseLease: vi.fn(),
    mockUseLeaseTemplates: vi.fn(),
    mockUseApplication: vi.fn(),
    mockUseViolation: vi.fn(),
    mockUseViolationEvidence: vi.fn(),
    mockListBuildingUnits: vi.fn(),
    mockCreateUploadUrl: vi.fn(),
    mockUploadToPresigned: vi.fn(),
  };
});

vi.mock('@ppt/api-client', () => ({
  listBuildingUnits: (...args: unknown[]) => mockListBuildingUnits(...args),
  createUploadUrl: (...args: unknown[]) => mockCreateUploadUrl(...args),
  uploadFileToPresignedUrl: (...args: unknown[]) => mockUploadToPresigned(...args),
  useBuildings: () => ({
    data: { items: [{ id: 'b-1', name: 'Bytový dom Hlavná 5' }] },
    isLoading: false,
  }),
  useOrganizationMembers: () => ({
    data: {
      total: 2,
      members: [
        {
          user_id: 'u-tenant',
          name: 'Ján Novák',
          email: 'jan@example.sk',
          role_name: 'Resident',
          role_type: 'resident',
        },
        {
          user_id: 'u-mgr',
          name: 'Eva Správcová',
          email: 'eva@example.sk',
          role_name: 'Manager',
          role_type: 'manager',
        },
      ],
    },
    isLoading: false,
  }),
  useLease: (...a: unknown[]) => mockUseLease(...a),
  useLeases: () => ({ data: { items: [], total: 0 }, isLoading: false }),
  useLeaseStatistics: () => ({ data: undefined, isLoading: false }),
  useExpiringLeases: () => ({ data: undefined, isLoading: false }),
  useLeaseTemplates: (...a: unknown[]) => mockUseLeaseTemplates(...a),
  useApplication: (...a: unknown[]) => mockUseApplication(...a),
  useApplications: () => ({
    data: {
      total: 1,
      items: [
        {
          id: 'app-1',
          unit_id: 'unit-1',
          unit_name: '3B',
          building_name: 'Bytový dom Hlavná 5',
          applicant_name: 'Ján Novák',
          applicant_email: 'jan@example.sk',
          status: 'submitted',
        },
      ],
    },
    isLoading: false,
  }),
  useViolation: (...a: unknown[]) => mockUseViolation(...a),
  useViolations: () => ({ data: [], isLoading: false }),
  useViolationEvidence: (...a: unknown[]) => mockUseViolationEvidence(...a),
  useCreateLease: mutationHook(mutations.createLease),
  useRecordLeasePayment: mutationHook(mutations.recordPayment),
  useCreateLeaseTemplate: mutationHook(mutations.createTemplate),
  useUpdateLeaseTemplate: mutationHook(mutations.updateTemplate),
  useInitiateScreening: mutationHook(mutations.initiateScreening),
  useReviewApplication: mutationHook(mutations.reviewApplication),
  useCreateViolation: mutationHook(vi.fn()),
  useUpdateViolation: mutationHook(mutations.updateViolation),
  useCreateViolationAppeal: mutationHook(mutations.createAppeal),
  useAddViolationEvidence: mutationHook(mutations.addEvidence),
  useUpdateLease: mutationHook(vi.fn()),
  useRenewLease: mutationHook(vi.fn()),
  useTerminateLease: mutationHook(vi.fn()),
  useCreateLeaseAmendment: mutationHook(vi.fn()),
}));

import { leaseRoutes, mapScreeningToUi, selectTenantCandidates } from './leases';

function renderAt(path: string) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[path]}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{leaseRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

const apiLease = {
  id: 'lease-1',
  organization_id: 'org-1',
  unit_id: 'unit-1',
  tenant_user_id: 'u-tenant',
  tenant_name: 'Ján Novák',
  tenant_email: 'jan@example.sk',
  status: 'active',
  start_date: '2026-01-01',
  end_date: '2026-12-31',
  monthly_rent: '850.00',
  security_deposit: '1700.00',
  rent_due_day: 5,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

const apiPayment = {
  id: 'pay-1',
  lease_id: 'lease-1',
  organization_id: 'org-1',
  due_date: '2026-11-05',
  amount: '850.00',
  payment_type: 'rent',
  is_late: false,
  created_at: '2026-10-01T00:00:00Z',
  updated_at: '2026-10-01T00:00:00Z',
};

const apiTemplate = {
  id: 'tpl-1',
  organization_id: 'org-1',
  name: 'Štandardná nájomná zmluva',
  content_html: '<p>Zmluva</p>',
  is_default: false,
  is_active: true,
  version: 1,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

const apiApplication = {
  id: 'app-1',
  organization_id: 'org-1',
  unit_id: 'unit-1',
  status: 'submitted',
  applicant_name: 'Ján Novák',
  applicant_email: 'jan@example.sk',
  created_at: '2026-10-01T00:00:00Z',
  updated_at: '2026-10-01T00:00:00Z',
};

const apiViolation = {
  id: 'viol-1',
  organization_id: 'org-1',
  violation_number: 'V-001',
  category: 'noise',
  severity: 'minor',
  status: 'reported',
  title: 'Hluk po 22:00',
  description: 'Hlasná hudba po nočnom kľude',
  occurred_at: '2026-10-20T22:30:00Z',
  reported_at: '2026-10-21T08:00:00Z',
  witness_count: 0,
  offense_number: 1,
  created_at: '2026-10-21T08:00:00Z',
  updated_at: '2026-10-21T08:00:00Z',
};

beforeEach(() => {
  vi.clearAllMocks();
  roleRef.role = 'manager';
  for (const fn of Object.values(mutations)) fn.mockResolvedValue({});
  mockListBuildingUnits.mockResolvedValue({
    units: [{ id: 'unit-1', building_id: 'b-1', designation: '3B' }],
    total: 1,
    offset: 0,
    limit: 100,
  });
  mockUseLeaseTemplates.mockReturnValue({ data: [apiTemplate], isLoading: false });
  mockUseViolationEvidence.mockReturnValue({ data: [] });
});

// ============================================================================
// Create lease — unit + tenant pickers
// ============================================================================

describe('CreateLeasePageRoute', () => {
  async function fillAndSubmit() {
    renderAt('/leases/new');
    const unitSelect = (await screen.findByLabelText(/leases\.form\.unit/)) as HTMLSelectElement;
    await screen.findByRole('option', { name: /3B/ });
    fireEvent.change(unitSelect, { target: { value: 'unit-1' } });
    fireEvent.change(screen.getByLabelText(/leases\.form\.tenant/), {
      target: { value: 'u-tenant' },
    });
    fireEvent.change(screen.getByLabelText(/leases\.form\.startDate/), {
      target: { value: '2026-12-01' },
    });
    fireEvent.change(screen.getByLabelText(/leases\.form\.endDate/), {
      target: { value: '2027-11-30' },
    });
    fireEvent.change(screen.getByLabelText(/leases\.form\.rentAmount/), {
      target: { value: '850' },
    });
    fireEvent.submit(unitSelect.closest('form') as HTMLFormElement);
  }

  it('lists building units and non-manager org members as tenants', async () => {
    renderAt('/leases/new');
    expect(await screen.findByRole('option', { name: /3B/ })).toBeInTheDocument();
    expect(mockListBuildingUnits).toHaveBeenCalledWith('b-1', { limit: 100 }, expect.anything());
    expect(screen.getByRole('option', { name: /Ján Novák/ })).toBeInTheDocument();
    expect(screen.queryByRole('option', { name: /Eva Správcová/ })).not.toBeInTheDocument();
  });

  it('submits the lease with the selected unit, tenant identity and landlord', async () => {
    await fillAndSubmit();
    await waitFor(() => expect(mutations.createLease).toHaveBeenCalledTimes(1));
    expect(mutations.createLease).toHaveBeenCalledWith(
      expect.objectContaining({
        organization_id: 'org-1',
        unit_id: 'unit-1',
        tenant_user_id: 'u-tenant',
        tenant_name: 'Ján Novák',
        tenant_email: 'jan@example.sk',
        landlord_name: 'Správa Domov s.r.o.',
        start_date: '2026-12-01',
        end_date: '2027-11-30',
        term_months: 11,
        monthly_rent: 850,
      })
    );
    expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });

  it('shows an error toast when the create call fails', async () => {
    mutations.createLease.mockRejectedValueOnce(new Error('Unit already leased'));
    await fillAndSubmit();
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', message: 'Unit already leased' })
      )
    );
  });
});

describe('selectTenantCandidates', () => {
  it('drops every manager-level role', () => {
    const out = selectTenantCandidates([
      { user_id: 'a', name: 'A', email: 'a@x', role_name: '', role_type: 'org_admin' },
      { user_id: 'b', name: 'B', email: 'b@x', role_name: '', role_type: 'owner' },
      { user_id: 'c', name: 'C', email: 'c@x', role_name: '', role_type: 'property_manager' },
    ]);
    expect(out).toEqual([{ id: 'b', name: 'B', email: 'b@x' }]);
  });
});

// ============================================================================
// Lease detail — record payment
// ============================================================================

describe('LeaseDetailPageRoute — record payment', () => {
  beforeEach(() => {
    mockUseLease.mockReturnValue({
      data: {
        lease: apiLease,
        unit_name: '3B',
        building_name: 'Bytový dom Hlavná 5',
        amendments: [],
        upcoming_payments: [apiPayment],
        reminders: [],
      },
      isLoading: false,
    });
  });

  async function openModal() {
    renderAt('/leases/lease-1');
    await userEvent.click(
      await screen.findByRole('button', { name: /leases\.detail\.tabs\.payments/ })
    );
    await userEvent.click(screen.getByRole('button', { name: 'leases.payments.recordPayment' }));
    return screen.getByRole('dialog');
  }

  it('records the payment against the scheduled payment id', async () => {
    const dialog = await openModal();
    fireEvent.change(within(dialog).getByLabelText(/leases\.payments\.recordModal\.amount/), {
      target: { value: '800' },
    });
    fireEvent.change(within(dialog).getByLabelText('leases.payments.recordModal.method'), {
      target: { value: 'cash' },
    });
    await userEvent.type(within(dialog).getByLabelText('leases.payments.reference'), 'VS2026');
    fireEvent.change(within(dialog).getByLabelText('leases.payments.paidDate'), {
      target: { value: '2026-11-03' },
    });
    await userEvent.click(
      within(dialog).getByRole('button', { name: 'leases.payments.recordPayment' })
    );

    await waitFor(() => expect(mutations.recordPayment).toHaveBeenCalledTimes(1));
    expect(mutations.recordPayment).toHaveBeenCalledWith({
      paymentId: 'pay-1',
      leaseId: 'lease-1',
      body: {
        paid_amount: 800,
        payment_method: 'cash',
        payment_reference: 'VS2026',
        paid_at: '2026-11-03T00:00:00Z',
      },
    });
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('rejects a non-positive amount without calling the API', async () => {
    const dialog = await openModal();
    fireEvent.change(within(dialog).getByLabelText(/leases\.payments\.recordModal\.amount/), {
      target: { value: '0' },
    });
    await userEvent.click(
      within(dialog).getByRole('button', { name: 'leases.payments.recordPayment' })
    );
    expect(within(dialog).getByRole('alert')).toHaveTextContent(
      'leases.payments.recordModal.amountInvalid'
    );
    expect(mutations.recordPayment).not.toHaveBeenCalled();
  });

  it('keeps the dialog open and toasts on failure', async () => {
    mutations.recordPayment.mockRejectedValueOnce(new Error('boom'));
    const dialog = await openModal();
    await userEvent.click(
      within(dialog).getByRole('button', { name: 'leases.payments.recordPayment' })
    );
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', message: 'boom' })
      )
    );
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });
});

// ============================================================================
// Templates — create / edit / toggle, delete hidden
// ============================================================================

describe('TemplatesPageRoute', () => {
  it('creates a template with the organization scope and mapped fields', async () => {
    renderAt('/leases/templates');
    await userEvent.click(
      await screen.findByRole('button', { name: 'leases.templates.createNew' })
    );
    // Rent / currency defaults are not persisted by the API → hidden.
    expect(screen.queryByLabelText('leases.templates.form.defaultRent')).not.toBeInTheDocument();
    await userEvent.type(screen.getByLabelText(/leases\.templates\.form\.name/), 'Krátkodobá');
    await userEvent.type(screen.getByLabelText(/leases\.templates\.form\.content/), 'Obsah');
    fireEvent.change(screen.getByLabelText(/leases\.templates\.form\.durationMonths/), {
      target: { value: '6' },
    });
    await userEvent.click(screen.getByRole('button', { name: 'common.create' }));

    await waitFor(() => expect(mutations.createTemplate).toHaveBeenCalledTimes(1));
    expect(mutations.createTemplate).toHaveBeenCalledWith(
      expect.objectContaining({
        organization_id: 'org-1',
        name: 'Krátkodobá',
        content_html: 'Obsah',
        default_term_months: 6,
      })
    );
    expect(mockShowToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });

  it('deactivates a template via PUT is_active=false and hides Delete', async () => {
    renderAt('/leases/templates');
    await screen.findByText('Štandardná nájomná zmluva');
    expect(screen.queryByRole('button', { name: 'common.delete' })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'leases.templates.deactivate' }));
    await waitFor(() =>
      expect(mutations.updateTemplate).toHaveBeenCalledWith({
        id: 'tpl-1',
        body: { organization_id: 'org-1', is_active: false },
      })
    );
  });

  it('toasts an error when the update fails', async () => {
    mutations.updateTemplate.mockRejectedValueOnce(new Error('Organization ID mismatch'));
    renderAt('/leases/templates');
    await userEvent.click(
      await screen.findByRole('button', { name: 'leases.templates.deactivate' })
    );
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({ type: 'error', message: 'Organization ID mismatch' })
      )
    );
  });
});

// ============================================================================
// Application detail — screening + review
// ============================================================================

describe('ApplicationDetailPageRoute', () => {
  beforeEach(() => {
    mockUseApplication.mockReturnValue({ data: apiApplication, isLoading: false });
  });

  it('initiates screening with the selected types and renders returned rows', async () => {
    mutations.initiateScreening.mockResolvedValueOnce([
      {
        id: 'scr-1',
        application_id: 'app-1',
        organization_id: 'org-1',
        screening_type: 'credit_check',
        status: 'pending_consent',
        created_at: '2026-11-04T00:00:00Z',
        updated_at: '2026-11-04T00:00:00Z',
      },
    ]);
    renderAt('/leases/applications/app-1');
    expect(await screen.findByText(/Bytový dom Hlavná 5 - 3B/)).toBeInTheDocument();
    await userEvent.click(
      screen.getByRole('checkbox', { name: 'leases.applications.screeningTypes.credit_check' })
    );
    await userEvent.click(
      screen.getByRole('button', { name: 'leases.applications.startScreening' })
    );

    await waitFor(() =>
      expect(mutations.initiateScreening).toHaveBeenCalledWith({
        id: 'app-1',
        body: { organization_id: 'org-1', screening_types: ['credit_check'] },
      })
    );
    expect(await screen.findByText(/leases\.applications\.requestedOn/)).toBeInTheDocument();
  });

  it('approves the application with the review notes', async () => {
    renderAt('/leases/applications/app-1');
    await userEvent.click(
      await screen.findByRole('button', { name: 'leases.applications.review' })
    );
    await userEvent.type(
      screen.getByPlaceholderText('leases.applications.reviewNotesPlaceholder'),
      'Príjem overený'
    );
    await userEvent.click(screen.getByRole('button', { name: 'leases.applications.approve' }));
    await waitFor(() =>
      expect(mutations.reviewApplication).toHaveBeenCalledWith({
        id: 'app-1',
        body: { status: 'approved', decision_notes: 'Príjem overený' },
      })
    );
  });

  it('toasts the server error when review is forbidden', async () => {
    mutations.reviewApplication.mockRejectedValueOnce(
      new Error('Only managers can review tenant applications')
    );
    renderAt('/leases/applications/app-1');
    await userEvent.click(
      await screen.findByRole('button', { name: 'leases.applications.review' })
    );
    await userEvent.click(screen.getByRole('button', { name: 'leases.applications.reject' }));
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          message: 'Only managers can review tenant applications',
        })
      )
    );
    expect(mutations.reviewApplication).toHaveBeenCalledWith({
      id: 'app-1',
      body: { status: 'rejected', decision_notes: undefined },
    });
  });
});

describe('mapScreeningToUi', () => {
  it('maps consent / failure states onto the UI union', () => {
    const base = {
      id: 's',
      application_id: 'a',
      organization_id: 'o',
      screening_type: 'rental_history',
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
    };
    expect(mapScreeningToUi({ ...base, status: 'consent_received' }).status).toBe('pending');
    expect(mapScreeningToUi({ ...base, status: 'expired' }).status).toBe('failed');
    expect(mapScreeningToUi({ ...base, status: 'completed', passed: true }).result).toBe('pass');
  });
});

// ============================================================================
// Violation detail — resolve / dispute / evidence
// ============================================================================

describe('ViolationDetailPageRoute', () => {
  beforeEach(() => {
    mockUseViolation.mockReturnValue({ data: apiViolation, isLoading: false });
  });

  it('manager resolves the violation with notes (PUT status=resolved)', async () => {
    renderAt('/leases/violations/viol-1');
    await userEvent.click(await screen.findByRole('button', { name: 'leases.violations.resolve' }));
    expect(screen.queryByRole('button', { name: 'leases.violations.dispute' })).toBeNull();
    await userEvent.type(screen.getByLabelText('leases.violations.resolutionNotes'), 'Upozornený');
    await userEvent.click(screen.getByRole('button', { name: 'leases.violations.confirmResolve' }));
    await waitFor(() =>
      expect(mutations.updateViolation).toHaveBeenCalledWith({
        id: 'viol-1',
        body: { status: 'resolved', resolution_notes: 'Upozornený' },
      })
    );
  });

  it('a resident disputes the violation by filing an appeal', async () => {
    roleRef.role = 'resident';
    renderAt('/leases/violations/viol-1');
    await userEvent.click(await screen.findByRole('button', { name: 'leases.violations.dispute' }));
    expect(screen.queryByRole('button', { name: 'leases.violations.resolve' })).toBeNull();
    await userEvent.type(
      screen.getByLabelText('leases.violations.disputeReason'),
      'Nebol som doma'
    );
    await userEvent.click(screen.getByRole('button', { name: 'leases.violations.confirmDispute' }));
    await waitFor(() =>
      expect(mutations.createAppeal).toHaveBeenCalledWith({
        id: 'viol-1',
        body: { reason: 'Nebol som doma' },
      })
    );
  });

  it('uploads evidence to storage, then registers it on the violation', async () => {
    mockCreateUploadUrl.mockResolvedValue({
      url: 'https://s3.example/put',
      file_key: 'org-1/evidence/photo.jpg',
      content_type: 'image/jpeg',
      method: 'PUT',
    });
    mockUploadToPresigned.mockResolvedValue(undefined);
    const { container } = renderAt('/leases/violations/viol-1');
    await userEvent.click(
      await screen.findByRole('button', { name: /leases\.violations\.tabs\.evidence/ })
    );
    const input = container.querySelector('input[type="file"]') as HTMLInputElement;
    const file = new File(['abc'], 'photo.jpg', { type: 'image/jpeg' });
    await userEvent.upload(input, file);

    await waitFor(() => expect(mutations.addEvidence).toHaveBeenCalledTimes(1));
    expect(mockCreateUploadUrl).toHaveBeenCalledWith({
      file_name: 'photo.jpg',
      mime_type: 'image/jpeg',
      size_bytes: 3,
    });
    expect(mockUploadToPresigned).toHaveBeenCalledWith(
      'https://s3.example/put',
      file,
      'image/jpeg'
    );
    expect(mutations.addEvidence).toHaveBeenCalledWith({
      id: 'viol-1',
      body: {
        file_name: 'photo.jpg',
        file_type: 'image/jpeg',
        file_size: 3,
        storage_path: 'org-1/evidence/photo.jpg',
      },
    });
  });

  it('toasts and does not register evidence when the storage upload fails', async () => {
    mockCreateUploadUrl.mockResolvedValue({
      url: 'https://s3.example/put',
      file_key: 'k',
      content_type: 'image/jpeg',
      method: 'PUT',
    });
    mockUploadToPresigned.mockRejectedValue(new Error('Upload to storage failed (HTTP 403)'));
    const { container } = renderAt('/leases/violations/viol-1');
    await userEvent.click(
      await screen.findByRole('button', { name: /leases\.violations\.tabs\.evidence/ })
    );
    const input = container.querySelector('input[type="file"]') as HTMLInputElement;
    await userEvent.upload(input, new File(['x'], 'p.jpg', { type: 'image/jpeg' }));
    await waitFor(() =>
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          message: 'Upload to storage failed (HTTP 403)',
        })
      )
    );
    expect(mutations.addEvidence).not.toHaveBeenCalled();
  });

  it('renders evidence returned by the evidence list endpoint', async () => {
    mockUseViolationEvidence.mockReturnValue({
      data: [
        {
          id: 'ev-1',
          violation_id: 'viol-1',
          file_name: 'hluk.mp3',
          file_type: 'audio/mpeg',
          file_size: 2048,
          created_at: '2026-10-21T09:00:00Z',
        },
      ],
    });
    renderAt('/leases/violations/viol-1');
    await userEvent.click(
      await screen.findByRole('button', { name: /leases\.violations\.tabs\.evidence/ })
    );
    expect(screen.getByText('hluk.mp3')).toBeInTheDocument();
  });
});
