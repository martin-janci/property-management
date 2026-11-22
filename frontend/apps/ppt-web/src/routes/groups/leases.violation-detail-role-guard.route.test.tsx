/// <reference types="vitest/globals" />
/**
 * Leases route group — violation-detail role guard (regression).
 *
 * The violation detail route (`/leases/violations/:violationId`) hosts BOTH
 * audiences: a manager resolves the violation, while the cited tenant disputes
 * it (files an appeal). `ViolationDetailPageRoute` already gates those actions
 * in-page (`isManager` → resolve, `!isManager` → dispute), so the route itself
 * must be reachable by any authenticated member — not just managers.
 *
 * Regression: the route was wrapped in
 * `<ProtectedRoute requiredRoles={[...MANAGER_ROLES]}>`, so a tenant hitting
 * the page got "Access Denied" and the dispute branch was dead for its only
 * audience. These tests exercise the REAL `<ProtectedRoute>` (only the auth
 * source + api-client are stubbed) to pin that:
 *  - a resident reaches the page and sees the dispute action (no Access Denied),
 *    and the manager-only resolve action is NOT offered to them;
 *  - a manager still reaches the page and sees resolve (and not dispute).
 *
 * The sibling `leases.actions.route.test.tsx` stubs `<ProtectedRoute>` to a
 * pass-through, so it covers the in-page action wiring but cannot catch the
 * route-level guard — hence this dedicated real-guard test.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AuthContextValue } from '../../contexts/AuthContext';

vi.mock('react-i18next', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-i18next')>();
  return {
    ...actual,
    useTranslation: () => ({
      t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
      i18n: { language: 'en' },
    }),
  };
});

// Keep the REAL <ProtectedRoute>; only stub the toast hook (no ToastProvider here).
const mockShowToast = vi.hoisted(() => vi.fn());
vi.mock('../../components', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../components')>();
  return { ...actual, useToast: () => ({ showToast: mockShowToast }) };
});

// Auth is consumed both by the route wrapper (`../../contexts`) and by the real
// <ProtectedRoute> (`../../contexts/AuthContext`) — mock both to the same role.
const authState = vi.hoisted(() => ({ role: 'resident' as string }));
function auth(): AuthContextValue {
  return {
    isAuthenticated: true,
    isLoading: false,
    user: {
      id: 'u-tenant',
      email: 'jan@example.sk',
      role: authState.role,
    } as AuthContextValue['user'],
    login: vi.fn(),
    loginWithSsoCode: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    getAccessToken: () => null,
    setUser: vi.fn(),
  };
}
vi.mock('../../contexts/AuthContext', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts/AuthContext')>();
  return { ...actual, useAuth: () => auth() };
});
vi.mock('../../contexts', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts')>();
  return { ...actual, useAuth: () => auth() };
});

vi.mock('../../hooks/useOrganization', () => ({
  useOrganization: () => ({ organizationId: 'org-1', organizationName: 'Správa Domov s.r.o.' }),
}));

const apiViolation = {
  id: 'viol-1',
  organization_id: 'org-1',
  violation_number: 'V-001',
  category: 'noise',
  severity: 'minor',
  status: 'reported', // maps to UI 'open' → dispute is available to the tenant
  title: 'Hluk po 22:00',
  description: 'Hlasná hudba po nočnom kľude',
  occurred_at: '2026-10-20T22:30:00Z',
  reported_at: '2026-10-21T08:00:00Z',
  violator_name: 'Ján Novák',
  violator_unit: '3B',
  created_at: '2026-10-21T08:00:00Z',
  updated_at: '2026-10-21T08:00:00Z',
};

vi.mock('@ppt/api-client', () => {
  const idle = () => ({ mutateAsync: vi.fn(), isPending: false });
  return {
    listBuildingUnits: vi.fn(),
    createUploadUrl: vi.fn(),
    uploadFileToPresignedUrl: vi.fn(),
    useBuildings: () => ({ data: { items: [] }, isLoading: false }),
    useOrganizationMembers: () => ({ data: { members: [], total: 0 }, isLoading: false }),
    useLease: () => ({ data: undefined, isLoading: false }),
    useLeases: () => ({ data: { items: [], total: 0 }, isLoading: false }),
    useLeaseStatistics: () => ({ data: undefined, isLoading: false }),
    useExpiringLeases: () => ({ data: undefined, isLoading: false }),
    useLeaseTemplates: () => ({ data: [], isLoading: false }),
    useApplication: () => ({ data: undefined, isLoading: false }),
    useApplications: () => ({ data: { items: [], total: 0 }, isLoading: false }),
    useViolation: () => ({ data: apiViolation, isLoading: false }),
    useViolations: () => ({ data: [], isLoading: false }),
    useViolationEvidence: () => ({ data: [] }),
    useCreateLease: idle,
    useRecordLeasePayment: idle,
    useCreateLeaseTemplate: idle,
    useUpdateLeaseTemplate: idle,
    useDeleteLeaseTemplate: idle,
    useInitiateScreening: idle,
    useReviewApplication: idle,
    useCreateViolation: idle,
    useUpdateViolation: idle,
    useCreateViolationAppeal: idle,
    useAddViolationEvidence: idle,
    useUpdateLease: idle,
    useRenewLease: idle,
    useTerminateLease: idle,
    useCreateLeaseAmendment: idle,
  };
});

import { leaseRoutes } from './leases';

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

beforeEach(() => {
  vi.clearAllMocks();
  authState.role = 'resident';
});

describe('violation detail route — reachable by the tenant it is meant for', () => {
  it('lets a resident open the violation and dispute it (no Access Denied)', async () => {
    authState.role = 'resident';
    renderAt('/leases/violations/viol-1');

    // The dispute action — the resident's whole reason for being here — renders.
    expect(
      await screen.findByRole('button', { name: 'leases.violations.dispute' })
    ).toBeInTheDocument();
    // The route guard did not bounce the tenant.
    expect(screen.queryByText('Access Denied')).not.toBeInTheDocument();
    // Manager-only action stays hidden from the tenant.
    expect(screen.queryByRole('button', { name: 'leases.violations.resolve' })).toBeNull();
  });

  it('still lets a manager open the violation and resolve it', async () => {
    authState.role = 'manager';
    renderAt('/leases/violations/viol-1');

    expect(
      await screen.findByRole('button', { name: 'leases.violations.resolve' })
    ).toBeInTheDocument();
    expect(screen.queryByText('Access Denied')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'leases.violations.dispute' })).toBeNull();
  });
});
