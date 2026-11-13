/// <reference types="vitest/globals" />
/**
 * Management route-group auth-guard regression coverage.
 *
 * Code-review finding `code-review-ppt-web-core-mgmt-routes-unguarded`:
 * auth gating in ppt-web is strictly per-route via `<ProtectedRoute>` — there
 * is NO global guard around `<AppRoutes />` (App.tsx mounts it inside
 * RouteErrorBoundary + Suspense only). A whole class of authenticated
 * management route groups mounted their `<Route>` elements bare, with no
 * `<ProtectedRoute>` wrapper, so an unauthenticated visitor rendered the page
 * chrome instead of being redirected to /login (and no return-URL was
 * captured) — inconsistent with every guarded peer group (buildings, leases,
 * voting, faults, …) and with the `/settings/privacy` fix.
 *
 * These tests mount the real route-group fragments for the four still-unguarded
 * groups — community (Epic 42), disputes (Epic 77/80), messaging (UC-07) and
 * neighbors (Epic 6) — and assert the auth-only guard: an unauthenticated
 * visitor to each group's entry route is redirected to /login, and an
 * authenticated user of any role (these are per-user / resident-accessible
 * features, so auth-only, NOT role-gated) reaches the page. They fail on the
 * pre-fix route table (the page renders for an anonymous visitor) and pass once
 * each route is wrapped in `<ProtectedRoute>`.
 */
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
    FeedPage: marker('community-feed'),
    MessagesPage: marker('messages-page'),
    NeighborsPage: marker('neighbors-page'),
    DisputesPage: marker('disputes-page'),
  };
});

// useToast needs a ToastProvider; override it so the route wrappers that call it
// render without one. Keep ProtectedRoute (same barrel) real.
vi.mock('../../components', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../components')>();
  return { ...actual, useToast: () => ({ showToast: vi.fn() }) };
});

// Safe, network-free stubs for the data hooks the entry-route wrappers call, so
// the pre-fix (unguarded) render completes without throwing.
vi.mock('../../features/messaging/hooks/useMessaging', () => ({
  useThreads: () => ({ threads: [], total: 0, isLoading: false }),
  useUnreadCount: () => ({ data: { unreadCount: 0 } }),
  useDeleteThread: () => ({ mutateAsync: vi.fn() }),
  useArchiveThread: () => ({ mutateAsync: vi.fn() }),
}));
vi.mock('../../features/neighbors', () => ({
  useNeighbors: () => ({ neighbors: [], isLoading: false, error: null }),
  usePrivacySettings: () => ({
    settings: undefined,
    isLoading: false,
    isSubmitting: false,
    error: null,
    updateSettings: vi.fn(),
  }),
}));
vi.mock('@ppt/api-client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@ppt/api-client')>();
  return {
    ...actual,
    useBuildings: () => ({ data: { items: [] }, isLoading: false }),
    useDisputes: () => ({ data: { data: [], total: 0 }, isLoading: false, error: null }),
  };
});

function authFor(role: string | null): AuthContextValue {
  return {
    user: role
      ? ({ id: 'u1', email: 'x@y.z', role, organizationId: 'org-1' } as AuthContextValue['user'])
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
import { communityRoutes } from './community';
import { disputeRoutes } from './disputes';
import { messagingRoutes } from './messaging';
import { neighborRoutes } from './neighbors';

function renderAt(entry: string) {
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <Suspense fallback={<div>loading…</div>}>
        <Routes>
          <Route path="/login" element={<div>Login Page</div>} />
          {communityRoutes()}
          {disputeRoutes()}
          {messagingRoutes()}
          {neighborRoutes()}
        </Routes>
      </Suspense>
    </MemoryRouter>
  );
}

const ENTRY_ROUTES: Array<{ path: string; testid: string }> = [
  { path: '/community', testid: 'community-feed' },
  { path: '/disputes', testid: 'disputes-page' },
  { path: '/messages', testid: 'messages-page' },
  { path: '/neighbors', testid: 'neighbors-page' },
];

describe('management route groups auth guard (unauthenticated)', () => {
  it.each(ENTRY_ROUTES)(
    'redirects an anonymous visitor from $path to /login',
    async ({ path, testid }) => {
      mockAuth = authFor(null);
      renderAt(path);
      expect(await screen.findByText('Login Page')).toBeInTheDocument();
      // The management page must NOT render for an unauthenticated user.
      expect(screen.queryByTestId(testid)).not.toBeInTheDocument();
    }
  );
});

describe('management route groups auth guard (authenticated, any role)', () => {
  // These are per-user / resident-accessible features: auth-only, NOT
  // role-gated. A resident must still reach them once signed in.
  it('lets an authenticated resident reach /community', async () => {
    mockAuth = authFor('tenant');
    renderAt('/community');
    expect(await screen.findByTestId('community-feed')).toBeInTheDocument();
    expect(screen.queryByText('Login Page')).not.toBeInTheDocument();
  });
});
