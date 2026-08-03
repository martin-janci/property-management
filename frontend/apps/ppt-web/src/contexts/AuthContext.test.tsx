/**
 * AuthContext logout flow tests — Issue #712 follow-up to PR #630.
 *
 * Covers the three guarantees of the logout cache-purge:
 *   1. Session is cleared (user becomes unauthenticated) so guarded routes
 *      redirect to /login.
 *   2. User-scoped (protected) cached queries are removed from the
 *      TanStack Query cache.
 *   3. Queries whose first key segment is NOT in `AUTHED_QUERY_KEY_ROOTS`
 *      survive logout — only the explicit auth-scoped subtrees are purged.
 */
/// <reference types="vitest/globals" />

// jsdom under vitest 4 does not expose localStorage by default (TEST-203 in
// shared-auth covers the same gap). Provide an in-memory polyfill before any
// SUT module is imported so tokenStorage.* can read/write.
const __memStore = new Map<string, string>();
const __localStorageShim: Storage = {
  get length() {
    return __memStore.size;
  },
  clear: () => __memStore.clear(),
  getItem: (k) => (__memStore.has(k) ? (__memStore.get(k) as string) : null),
  key: (i) => Array.from(__memStore.keys())[i] ?? null,
  removeItem: (k) => {
    __memStore.delete(k);
  },
  setItem: (k, v) => {
    __memStore.set(k, String(v));
  },
};
if (typeof globalThis.localStorage === 'undefined') {
  Object.defineProperty(globalThis, 'localStorage', {
    value: __localStorageShim,
    configurable: true,
    writable: true,
  });
}
if (typeof window !== 'undefined' && typeof window.localStorage === 'undefined') {
  Object.defineProperty(window, 'localStorage', {
    value: __localStorageShim,
    configurable: true,
    writable: true,
  });
}

import type { AuthApi, LogoutRequest } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, render, screen, waitFor } from '@testing-library/react';
import type React from 'react';
import { useEffect } from 'react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ProtectedRoute } from '../components/ProtectedRoute';
import { AUTHED_QUERY_KEY_ROOTS } from '../lib/queryKeys';
import { AuthProvider, useAuth } from './AuthContext';

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

// Mock the api-client so logout() does not hit the network. We capture the
// LogoutRequest to verify the refresh-token revocation is at least attempted.
const logoutSpy = vi.fn<(req: LogoutRequest) => Promise<void>>().mockResolvedValue(undefined);
const fakeAuthApi: Partial<AuthApi> = {
  logout: logoutSpy as unknown as AuthApi['logout'],
};

vi.mock('@ppt/api-client', async () => {
  const actual = await vi.importActual<typeof import('@ppt/api-client')>('@ppt/api-client');
  return {
    ...actual,
    createAuthApi: vi.fn(() => fakeAuthApi),
    setTokenProvider: vi.fn(),
    clearTokenProvider: vi.fn(),
  };
});

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
const USER_KEY = 'ppt_user';

function seedAuthedSession() {
  localStorage.setItem(ACCESS_TOKEN_KEY, 'access-token-xyz');
  localStorage.setItem(REFRESH_TOKEN_KEY, 'refresh-token-xyz');
  localStorage.setItem(
    USER_KEY,
    JSON.stringify({ id: 'u-1', email: 'alice@example.com', name: 'Alice' })
  );
}

function makeQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: Infinity, staleTime: Infinity },
    },
  });
}

/**
 * Exposes the auth context via a ref-like object so tests can drive logout()
 * without simulating a button click. Renders the page name so we can assert
 * which route is active.
 */
function AuthHarness({
  ctxRef,
}: {
  ctxRef: { current: ReturnType<typeof useAuth> | null };
}): React.ReactElement {
  const auth = useAuth();
  // Expose the latest context to the surrounding test scope.
  useEffect(() => {
    ctxRef.current = auth;
  });
  return <div data-testid="auth-state">{auth.isAuthenticated ? 'authed' : 'anon'}</div>;
}

function LoginPage(): React.ReactElement {
  return <div>Login page</div>;
}

function DashboardPage(): React.ReactElement {
  return <div>Dashboard page</div>;
}

interface RenderResult {
  ctxRef: { current: ReturnType<typeof useAuth> | null };
  queryClient: QueryClient;
}

function renderAuthApp(): RenderResult {
  const ctxRef: { current: ReturnType<typeof useAuth> | null } = { current: null };
  const queryClient = makeQueryClient();

  render(
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <MemoryRouter initialEntries={['/dashboard']}>
          <AuthHarness ctxRef={ctxRef} />
          <Routes>
            <Route path="/login" element={<LoginPage />} />
            <Route
              path="/dashboard"
              element={
                <ProtectedRoute>
                  <DashboardPage />
                </ProtectedRoute>
              }
            />
          </Routes>
        </MemoryRouter>
      </AuthProvider>
    </QueryClientProvider>
  );

  return { ctxRef, queryClient };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('AuthContext.logout — Issue #712', () => {
  beforeEach(() => {
    seedAuthedSession();
    logoutSpy.mockClear();
  });

  afterEach(() => {
    localStorage.clear();
  });

  it('removes auth-scoped subtrees while leaving non-auth-scoped cache intact', async () => {
    const { ctxRef, queryClient } = renderAuthApp();

    // Wait for AuthProvider to bootstrap the authed state from storage.
    await waitFor(() => {
      expect(ctxRef.current?.isAuthenticated).toBe(true);
    });

    // Sanity: AUTHED_QUERY_KEY_ROOTS must cover the real query keys we
    // expect to evict. If a root is dropped from the source list, this
    // assertion will fail loudly rather than letting data silently leak.
    for (const root of [
      'user',
      'faults',
      'announcements',
      'ai-chat',
      'developer',
      // Tenant-scoped analytics dashboards — regression guard for the
      // logout cache-purge gap where these roots leaked across sessions
      // on shared workstations.
      'predictive-maintenance',
      'sentiment',
      'notification-analytics',
    ]) {
      expect(AUTHED_QUERY_KEY_ROOTS).toContain(root);
    }

    // Seed auth-scoped entries using REAL query keys from the codebase —
    // factory keys (`['user','profile']`, `['faults','list',…]`,
    // `['announcements',…]`) and ad-hoc keys (`['ai-chat',…]`,
    // `['developer','apiKeys']`) — plus a non-auth-scoped key
    // (`['router','breadcrumbs']`) whose root is intentionally absent
    // from AUTHED_QUERY_KEY_ROOTS and must survive logout.
    queryClient.setQueryData(['user', 'profile'], { id: 'u-1', name: 'Alice' });
    queryClient.setQueryData(['faults', 'list', { page: 1 }], [{ id: 'f-1' }]);
    queryClient.setQueryData(['announcements', 'unread-count'], 3);
    queryClient.setQueryData(['ai-chat', 'sessions'], [{ id: 's-1' }]);
    queryClient.setQueryData(['developer', 'apiKeys'], [{ id: 'k-1' }]);
    // Tenant-scoped analytics dashboards — real key factories from
    // predictiveKeys / sentimentKeys / notificationAnalyticsKeys.
    queryClient.setQueryData(['predictive-maintenance', 'needing-maintenance'], [{ id: 'e-1' }]);
    queryClient.setQueryData(['sentiment', 'dashboard'], { score: 0.42 });
    queryClient.setQueryData(['notification-analytics', {}], { delivered: 10 });
    queryClient.setQueryData(['router', 'breadcrumbs'], ['home']);

    // Sanity: all entries present before logout.
    expect(queryClient.getQueryData(['user', 'profile'])).toBeDefined();
    expect(queryClient.getQueryData(['faults', 'list', { page: 1 }])).toBeDefined();
    expect(queryClient.getQueryData(['announcements', 'unread-count'])).toBeDefined();
    expect(queryClient.getQueryData(['ai-chat', 'sessions'])).toBeDefined();
    expect(queryClient.getQueryData(['developer', 'apiKeys'])).toBeDefined();
    expect(
      queryClient.getQueryData(['predictive-maintenance', 'needing-maintenance'])
    ).toBeDefined();
    expect(queryClient.getQueryData(['sentiment', 'dashboard'])).toBeDefined();
    expect(queryClient.getQueryData(['notification-analytics', {}])).toBeDefined();
    expect(queryClient.getQueryData(['router', 'breadcrumbs'])).toBeDefined();

    // Trigger logout via the live context.
    await act(async () => {
      await ctxRef.current?.logout();
    });

    // (a) Session cleared — ProtectedRoute should now redirect to /login.
    await waitFor(() => {
      expect(screen.getByText('Login page')).toBeInTheDocument();
    });
    expect(screen.queryByText('Dashboard page')).not.toBeInTheDocument();
    expect(ctxRef.current?.isAuthenticated).toBe(false);

    // (b) Every auth-scoped subtree is gone — both factory keys and ad-hoc.
    expect(queryClient.getQueryData(['user', 'profile'])).toBeUndefined();
    expect(queryClient.getQueryData(['faults', 'list', { page: 1 }])).toBeUndefined();
    expect(queryClient.getQueryData(['announcements', 'unread-count'])).toBeUndefined();
    expect(queryClient.getQueryData(['ai-chat', 'sessions'])).toBeUndefined();
    expect(queryClient.getQueryData(['developer', 'apiKeys'])).toBeUndefined();
    // Tenant-scoped analytics dashboards must not survive logout.
    expect(
      queryClient.getQueryData(['predictive-maintenance', 'needing-maintenance'])
    ).toBeUndefined();
    expect(queryClient.getQueryData(['sentiment', 'dashboard'])).toBeUndefined();
    expect(queryClient.getQueryData(['notification-analytics', {}])).toBeUndefined();

    // (c) Non-auth-scoped cache survives — the purge is bounded to the
    // AUTHED_QUERY_KEY_ROOTS list, not a blanket `queryClient.clear()`.
    expect(queryClient.getQueryData(['router', 'breadcrumbs'])).toEqual(['home']);

    // Token storage is cleared.
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(USER_KEY)).toBeNull();

    // Server-side token revocation is at least attempted.
    expect(logoutSpy).toHaveBeenCalledTimes(1);
    expect(logoutSpy).toHaveBeenCalledWith({ refreshToken: 'refresh-token-xyz' });
  });

  it('still completes local logout when server-side revocation fails', async () => {
    logoutSpy.mockRejectedValueOnce(new Error('network down'));
    const { ctxRef, queryClient } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isAuthenticated).toBe(true);
    });

    queryClient.setQueryData(['user', 'profile'], { id: 'u-1' });

    await act(async () => {
      await ctxRef.current?.logout();
    });

    expect(ctxRef.current?.isAuthenticated).toBe(false);
    expect(queryClient.getQueryData(['user', 'profile'])).toBeUndefined();
    await waitFor(() => {
      expect(screen.getByText('Login page')).toBeInTheDocument();
    });
  });
});
