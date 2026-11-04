/// <reference types="vitest/globals" />
/**
 * Manager-only fault actions are hidden from residents (round-3 audit #14).
 *
 * Owners and tenants used to see "Edit" and "Triage" on every new fault in
 * `/faults`, and `/faults/:id/edit` opened for them only for the save to fail
 * with 403 "Manager role required". The list now offers those actions only to
 * manager roles, and the edit route is wrapped in the real <ProtectedRoute>
 * with MANAGER_ROLES.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { Suspense } from 'react';
import { MemoryRouter, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AuthContextValue } from '../../contexts/AuthContext';
import { MANAGER_ROLES } from '../shared';

const authState = vi.hoisted(() => ({ role: 'manager' as string | undefined }));

function auth(): AuthContextValue {
  return {
    isAuthenticated: true,
    isLoading: false,
    user: { id: 'u1', email: 'x@y.z', role: authState.role } as AuthContextValue['user'],
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
vi.mock('../../components', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../components')>();
  return { ...actual, useToast: () => ({ showToast: vi.fn() }) };
});

vi.mock('@ppt/api-client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@ppt/api-client')>()),
  getToken: () => 'test-token',
  useFaults: () => ({
    data: {
      faults: [
        {
          id: 'f-1',
          building_id: 'b-1',
          unit_id: null,
          title: 'Dripping tap in unit 3B',
          category: 'plumbing',
          priority: 'medium',
          status: 'new',
          created_at: '2026-06-01T00:00:00Z',
        },
      ],
      count: 1,
    },
    isLoading: false,
    error: null,
    refetch: vi.fn(),
  }),
  useDeleteFault: () => ({ mutateAsync: vi.fn() }),
  useFaultStatistics: () => ({ data: undefined }),
  useBuildings: () => ({ data: undefined }),
  useCreateFault: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useFault: () => ({ data: undefined, isLoading: true, error: null }),
  useTriageFault: () => ({ mutate: vi.fn() }),
  useResolveFault: () => ({ mutate: vi.fn() }),
  useConfirmFault: () => ({ mutate: vi.fn() }),
  useReopenFault: () => ({ mutate: vi.fn() }),
  useAddComment: () => ({ mutate: vi.fn() }),
  useAddAttachment: () => ({ mutate: vi.fn() }),
  useDeleteAttachment: () => ({ mutate: vi.fn() }),
  useUpdateFault: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

import { faultRoutes } from './faults';

function renderAt(path: string) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[path]}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>{faultRoutes()}</Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe('fault list actions per role', () => {
  beforeEach(() => {
    authState.role = 'manager';
  });

  it.each(['owner', 'tenant', 'resident'])('hides Edit and Triage from "%s"', async (role) => {
    authState.role = role;
    renderAt('/faults');
    await screen.findByText('Dripping tap in unit 3B', undefined, { timeout: 3000 });
    expect(screen.queryByRole('button', { name: 'Edit' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Triage' })).not.toBeInTheDocument();
    // Reading the fault stays available.
    expect(screen.getByRole('button', { name: 'View' })).toBeInTheDocument();
  });

  it.each([...MANAGER_ROLES])('offers Edit and Triage to "%s"', async (role) => {
    authState.role = role;
    renderAt('/faults');
    await screen.findByText('Dripping tap in unit 3B', undefined, { timeout: 3000 });
    expect(screen.getByRole('button', { name: 'Edit' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Triage' })).toBeInTheDocument();
  });
});

describe('/faults/:faultId/edit route guard', () => {
  it.each(['owner', 'tenant'])('denies "%s"', async (role) => {
    authState.role = role;
    renderAt('/faults/f-1/edit');
    expect(await screen.findByText('Access Denied')).toBeInTheDocument();
  });

  it('admits a manager', async () => {
    authState.role = 'manager';
    renderAt('/faults/f-1/edit');
    // The edit page itself renders (still loading the fault), not the denial.
    expect(await screen.findByText('Loading…')).toBeInTheDocument();
    expect(screen.queryByText('Access Denied')).not.toBeInTheDocument();
  });
});
