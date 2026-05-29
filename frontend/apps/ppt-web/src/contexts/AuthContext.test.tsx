/**
 * AuthContext logout flow tests — Issue #712 follow-up to PR #630.
 *
 * Covers the three guarantees of the logout cache-purge:
 *   1. Session is cleared (user becomes unauthenticated) so guarded routes
 *      redirect to /login.
 *   2. User-scoped (protected) cached queries are removed from the
 *      TanStack Query cache.
 *   3. Public queries (those whose first key segment is `PUBLIC_QUERY_PREFIX`)
 *      survive logout — avoiding the blank-screen re-fetch flash.
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
import { PUBLIC_QUERY_PREFIX } from '../lib/queryKeys';
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

  it('removes session-scoped cached queries while preserving public queries', async () => {
    const { ctxRef, queryClient } = renderAuthApp();

    // Wait for AuthProvider to bootstrap the authed state from storage.
    await waitFor(() => {
      expect(ctxRef.current?.isAuthenticated).toBe(true);
    });

    // Seed three cache entries:
    //   - protected (user-scoped)
    //   - protected (another resource)
    //   - public (must survive logout)
    queryClient.setQueryData(['user', 'profile'], { id: 'u-1', name: 'Alice' });
    queryClient.setQueryData(['faults', 'list', { page: 1 }], [{ id: 'f-1' }]);
    queryClient.setQueryData([PUBLIC_QUERY_PREFIX, 'feature-flags'], { newFaultsUi: true });

    // Sanity: all three are present before logout.
    expect(queryClient.getQueryData(['user', 'profile'])).toBeDefined();
    expect(queryClient.getQueryData(['faults', 'list', { page: 1 }])).toBeDefined();
    expect(queryClient.getQueryData([PUBLIC_QUERY_PREFIX, 'feature-flags'])).toBeDefined();

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

    // (b) Session-scoped queries are gone.
    expect(queryClient.getQueryData(['user', 'profile'])).toBeUndefined();
    expect(queryClient.getQueryData(['faults', 'list', { page: 1 }])).toBeUndefined();

    // (c) Public queries survive — no blank-screen re-fetch on next login.
    expect(queryClient.getQueryData([PUBLIC_QUERY_PREFIX, 'feature-flags'])).toEqual({
      newFaultsUi: true,
    });

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
