/// <reference types="vitest/globals" />
/**
 * Core management route-group auth-guard regression coverage.
 *
 * Code-review finding `code-review-ppt-web-core-route-groups-no-protectedroute`:
 * auth gating in ppt-web is strictly per-route via `<ProtectedRoute>` — there is
 * NO global guard around `<AppRoutes />` (App.tsx mounts it inside
 * RouteErrorBoundary + Suspense only). Four content route groups still mounted
 * their list/detail `<Route>` elements bare, with no `<ProtectedRoute>` wrapper,
 * so an unauthenticated visitor rendered the page chrome instead of being
 * redirected to /login (and no return URL was captured):
 *
 *   - announcements: /announcements, /announcements/:id
 *   - faults:        /faults, /faults/new, /faults/:id
 *   - financial:     /financial, /financial/payments
 *   - outages:       /outages, /outages/:id
 *
 * This is inconsistent with every guarded peer group (buildings, leases, voting,
 * community, disputes, messaging, neighbors, …) and with the mgmt-routes fix
 * (`code-review-ppt-web-core-mgmt-routes-unguarded`). The outages wrapper's
 * inline `<AuthRequiredGate>` only covers the authenticated-but-no-org edge case
 * and never redirects, so it is not an auth guard either.
 *
 * These tests mount the real route-group fragments for the four groups and
 * assert the auth-only guard on each group's list entry route: an
 * unauthenticated visitor is redirected to /login, and an authenticated user of
 * any role (these are per-user / resident-accessible features, so auth-only, NOT
 * role-gated) reaches the page. They fail on the pre-fix route table (the page
 * renders, or the non-redirecting AuthRequiredGate shows, for an anonymous
 * visitor) and pass once each route is wrapped in `<ProtectedRoute>`.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { Suspense } from 'react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import type { AuthContextValue } from '../../contexts/AuthContext';

// Mutable auth mock injected per-test. ProtectedRoute reads useAuth from
// ../../contexts/AuthContext; the group files read it from the ../../contexts
// barrel which re-exports the same module, so mock both to the same source.
let mockAuth: AuthContextValue;

vi.mock('../../contexts/AuthContext', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts/AuthContext')>();
  return { ...actual, useAuth: () => mockAuth };
});
vi.mock('../../contexts', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../contexts')>();
  return { ...actual, useAuth: () => mockAuth };
});

// i18n pass-through so ProtectedRoute's spinner/labels don't need a provider.
vi.mock('react-i18next', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-i18next')>();
  return {
    ...actual,
    useTranslation: () => ({
      t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    }),
  };
});

// Stub the lazy page components with markers so the pre-fix render surfaces a
// known testid without any real data wiring; the real ProtectedRoute wrapper is
// what we exercise. Keep every other export real.
vi.mock('../lazyRoutes', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lazyRoutes')>();
  const marker = (testid: string) => () => <div data-testid={testid} />;
  return {
    ...actual,
    AnnouncementsPage: marker('announcements-page'),
    FaultsPage: marker('faults-page'),
    FinancialDashboardPage: marker('financial-page'),
    OutagesPage: marker('outages-page'),
  };
});

// useToast needs a ToastProvider; override it so the route wrappers that call it
// render without one. Keep ProtectedRoute + AuthRequiredGate (same barrel) real.
vi.mock('../../components', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../components')>();
  return { ...actual, useToast: () => ({ showToast: vi.fn() }) };
});

// useBuildings is pulled in by the buildings feature barrel the faults wrapper
// touches; stub it network-free.
vi.mock('../../features/buildings/hooks', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../features/buildings/hooks')>();
  return { ...actual, useUnits: () => ({ data: undefined, isLoading: false }) };
});

// Safe, network-free stubs for the @ppt/api-client data hooks + imperative
// fetchers the list-entry wrappers call, so the pre-fix (unguarded) render
// completes without throwing. Mutations expose the no-op shape the pages read.
vi.mock('@ppt/api-client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@ppt/api-client')>();
  const query = (data: unknown) => ({
    data,
    isLoading: false,
    isError: false,
    error: null,
    refetch: vi.fn(),
  });
  const mutation = () => ({ mutateAsync: vi.fn(), mutate: vi.fn(), isPending: false });
  const emptyList = { items: [], data: [], announcements: [], outages: [], total: 0 };
  return {
    ...actual,
    // announcements
    useAnnouncements: () => query(emptyList),
    usePublishedAnnouncements: () => query(emptyList),
    usePinnedAnnouncements: () => query(emptyList),
    useDeleteAnnouncement: mutation,
    usePublishAnnouncement: mutation,
    useArchiveAnnouncement: mutation,
    usePinAnnouncement: mutation,
    // faults
    useFaults: () => query(emptyList),
    useFaultStatistics: () => query(undefined),
    // outages
    useOutages: () => query(emptyList),
    // financial dashboard imperative fetchers (queries are enabled:!!orgId, but
    // guard against them being invoked once a test supplies an orgId).
    getARAgingReport: vi.fn().mockResolvedValue({ buckets: [] }),
    getOverdueInvoices: vi.fn().mockResolvedValue({ items: [] }),
    listInvoices: vi.fn().mockResolvedValue({ items: [] }),
    listPayments: vi.fn().mockResolvedValue({ items: [] }),
  };
});

function authFor(role: string | null): AuthContextValue {
  return {
    user: role
      ? ({
          id: 'u1',
          email: 'x@y.z',
          role,
          organizationId: 'org-1',
        } as AuthContextValue['user'])
      : null,
    isAuthenticated: role !== null,
    isLoading: false,
    login: vi.fn(),
    loginWithSsoCode: vi.fn(),
    logout: vi.fn(),
    refreshToken: vi.fn(),
    getAccessToken: () => null,
    setUser: vi.fn(),
  };
}

// Import after the mocks are registered.
import { announcementRoutes } from './announcements';
import { faultRoutes } from './faults';
import { financialRoutes } from './financial';
import { outageRoutes } from './outages';

function renderAt(entry: string) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={[entry]}>
        <Suspense fallback={<div>loading…</div>}>
          <Routes>
            <Route path="/login" element={<div>Login Page</div>} />
            {announcementRoutes()}
            {faultRoutes()}
            {financialRoutes()}
            {outageRoutes()}
          </Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

const ENTRY_ROUTES: Array<{ path: string; testid: string }> = [
  { path: '/announcements', testid: 'announcements-page' },
  { path: '/faults', testid: 'faults-page' },
  { path: '/financial', testid: 'financial-page' },
  { path: '/outages', testid: 'outages-page' },
];

describe('core content route groups auth guard (unauthenticated)', () => {
  it.each(ENTRY_ROUTES)(
    'redirects an anonymous visitor from $path to /login',
    async ({ path, testid }) => {
      mockAuth = authFor(null);
      renderAt(path);
      expect(await screen.findByText('Login Page')).toBeInTheDocument();
      // The content page must NOT render for an unauthenticated user.
      expect(screen.queryByTestId(testid)).not.toBeInTheDocument();
    }
  );
});

describe('core content route groups auth guard (authenticated, any role)', () => {
  // These are per-user / resident-accessible features: auth-only, NOT
  // role-gated. A resident must still reach them once signed in.
  it.each(ENTRY_ROUTES)('lets an authenticated resident reach $path', async ({ path, testid }) => {
    mockAuth = authFor('tenant');
    renderAt(path);
    expect(await screen.findByTestId(testid)).toBeInTheDocument();
    expect(screen.queryByText('Login Page')).not.toBeInTheDocument();
  });
});
