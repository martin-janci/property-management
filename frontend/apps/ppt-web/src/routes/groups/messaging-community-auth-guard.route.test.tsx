/// <reference types="vitest/globals" />
/**
 * Messaging + Community route-group auth-guard regression coverage.
 *
 * Code-review finding `code-review-ppt-web-core-route-auth-guard-gap`:
 * auth gating in ppt-web is strictly per-route via `<ProtectedRoute>` — there is
 * NO global guard around `<AppRoutes />`. An earlier Tier-1d signal flagged 7
 * feature route groups (accounting, financial, messaging, faults, announcements,
 * community, neighbors) as rendering their `<Route>` elements with no auth gate,
 * so an unauthenticated visitor rendered the page chrome instead of being
 * redirected to /login (and no return URL was captured).
 *
 * That gap is already closed on `dev`: every group now wraps its routes in
 * `<ProtectedRoute>`. Five of the seven already carry redirect-to-login
 * regression coverage (accounting.role-gate, neighbors.privacy-auth, and
 * announcements/faults/financial via core-route-groups-auth-guard). The two
 * remaining groups — **messaging** (private threads / inbox, the sensitive
 * subset) and **community** — had NO auth-guard regression test, so a future
 * refactor could silently drop the `<ProtectedRoute>` wrapper on either without
 * tripping a test. This file closes that coverage gap.
 *
 * Each test mounts the real route-group fragment and asserts the auth-only guard
 * on the group's list-entry route: an unauthenticated visitor is redirected to
 * /login (the content page does NOT render), and an authenticated user of any
 * role (these are per-user / resident-accessible features — auth-only, NOT
 * role-gated) reaches the page. The unauthenticated cases fail if the
 * `<ProtectedRoute>` wrapper is removed from the group fragment.
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

// Stub the lazy page components with markers so the (hypothetical) unguarded
// render surfaces a known testid without any real data wiring; the real
// ProtectedRoute wrapper is what we exercise. Keep every other export real.
vi.mock('../lazyRoutes', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lazyRoutes')>();
  const marker = (testid: string) => () => <div data-testid={testid} />;
  return {
    ...actual,
    // messaging
    MessagesPage: marker('messages-page'),
    NewMessagePage: marker('new-message-page'),
    ThreadDetailPage: marker('thread-detail-page'),
    // community
    FeedPage: marker('community-feed-page'),
    GroupsPage: marker('community-groups-page'),
    MarketplacePage: marker('community-marketplace-page'),
  };
});

// useToast needs a ToastProvider; override it so the messaging route wrappers
// that call it render without one. Keep ProtectedRoute (same barrel) real.
vi.mock('../../components', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../components')>();
  return { ...actual, useToast: () => ({ showToast: vi.fn() }) };
});

// MessagesPageRoute calls these messaging data hooks on an authenticated render
// (ProtectedRoute mounts its children only when authed). Stub them network-free
// so the authed test completes without a provider or a real fetch.
vi.mock('../../features/messaging/hooks/useMessaging', async (importOriginal) => {
  const actual =
    await importOriginal<typeof import('../../features/messaging/hooks/useMessaging')>();
  const mutation = () => ({ mutateAsync: vi.fn(), mutate: vi.fn(), isPending: false });
  return {
    ...actual,
    useThreads: () => ({ threads: [], total: 0, isLoading: false }),
    useUnreadCount: () => ({ data: { unreadCount: 0 } }),
    useDeleteThread: mutation,
    useArchiveThread: mutation,
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
import { communityRoutes } from './community';
import { messagingRoutes } from './messaging';

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
            {messagingRoutes()}
            {communityRoutes()}
          </Routes>
        </Suspense>
      </MemoryRouter>
    </QueryClientProvider>
  );
}

const ENTRY_ROUTES: Array<{ path: string; testid: string }> = [
  { path: '/messages', testid: 'messages-page' },
  { path: '/community', testid: 'community-feed-page' },
];

describe('messaging + community route groups auth guard (unauthenticated)', () => {
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

describe('messaging + community route groups auth guard (authenticated, any role)', () => {
  // These are per-user / resident-accessible features: auth-only, NOT
  // role-gated. A resident must still reach them once signed in.
  it.each(ENTRY_ROUTES)('lets an authenticated resident reach $path', async ({ path, testid }) => {
    mockAuth = authFor('tenant');
    renderAt(path);
    expect(await screen.findByTestId(testid)).toBeInTheDocument();
    expect(screen.queryByText('Login Page')).not.toBeInTheDocument();
  });
});
