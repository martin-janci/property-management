/**
 * AuthContext login + refresh-failure tests — Story 79.2 verification gap.
 *
 * The existing suites cover:
 *   - logout cache-purge + redirect (AuthContext.test.tsx, #712)
 *   - SSO /auth/callback happy/error paths + refresh rotation (AuthCallbackPage.test.tsx)
 *   - route-guard role gating + deriveActiveRole (ProtectedRoute.test.tsx, #482)
 *
 * Two Story-79.2 acceptance scenarios had no direct ppt-web coverage:
 *
 *   AC: "email/password login stores tokens and authenticates a guarded route"
 *       — `AuthContext.login()` was only exercised indirectly via the SSO path.
 *
 *   AC: "a revoked refresh token (or family-reuse replay) results in a clean
 *        logout, not an error loop"
 *       — `refreshToken()`'s failure branch (`tokenStorage.clear()` + `setUser(null)`
 *         + rethrow in refreshTokenInternal) had no test. This guards against the
 *         silent-refresh path looping on a 401 instead of dropping the session.
 *
 * Both render the REAL AuthProvider + ProtectedRoute, mocking only the
 * api-client network boundary (`login` / `refreshToken`).
 */
/// <reference types="vitest/globals" />

// jsdom under vitest 4 does not expose localStorage by default. Provide an
// in-memory shim before any SUT module is imported (mirrors the shim in the
// sibling AuthContext.test.tsx / AuthCallbackPage.test.tsx suites).
function makeStorageShim(): Storage {
  const store = new Map<string, string>();
  return {
    get length() {
      return store.size;
    },
    clear: () => store.clear(),
    getItem: (k) => (store.has(k) ? (store.get(k) as string) : null),
    key: (i) => Array.from(store.keys())[i] ?? null,
    removeItem: (k) => {
      store.delete(k);
    },
    setItem: (k, v) => {
      store.set(k, String(v));
    },
  };
}

function installStorage(name: 'localStorage' | 'sessionStorage') {
  const shim = makeStorageShim();
  if (typeof globalThis[name] === 'undefined') {
    Object.defineProperty(globalThis, name, { value: shim, configurable: true, writable: true });
  }
  if (typeof window !== 'undefined' && typeof window[name] === 'undefined') {
    Object.defineProperty(window, name, { value: shim, configurable: true, writable: true });
  }
}
installStorage('localStorage');
installStorage('sessionStorage');

import type { AuthApi, LoginResponse, RefreshTokenResponse } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, render, screen, waitFor } from '@testing-library/react';
import type React from 'react';
import { useEffect } from 'react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ProtectedRoute } from '../components/ProtectedRoute';
import {
  __clearAnalyticsSinks,
  type AnalyticsEvent,
  registerAnalyticsSink,
} from '../lib/analytics';
import { AuthProvider, useAuth } from './AuthContext';

// ---------------------------------------------------------------------------
// Mocks — only the api-client network boundary.
// ---------------------------------------------------------------------------

const loginSpy = vi.fn<(creds: { email: string; password: string }) => Promise<LoginResponse>>();
const refreshTokenSpy = vi.fn<(req: { refreshToken: string }) => Promise<RefreshTokenResponse>>();

const fakeAuthApi: Partial<AuthApi> = {
  login: loginSpy as unknown as AuthApi['login'],
  refreshToken: refreshTokenSpy as unknown as AuthApi['refreshToken'],
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
// Constants + helpers
// ---------------------------------------------------------------------------

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
const USER_KEY = 'ppt_user';
const TENANTS_KEY = 'ppt_tenants';

function makeLoginResponse(overrides: Partial<LoginResponse> = {}): LoginResponse {
  return {
    accessToken: 'login-access-1',
    refreshToken: 'login-refresh-1',
    expiresIn: 3600,
    user: {
      id: 'u-login-1',
      email: 'manager@example.com',
      firstName: 'Mana',
      lastName: 'Ger',
    },
    tenants: [{ tenantId: 't-1', tenantName: 'Acme', role: 'manager' }],
    ...overrides,
  };
}

function makeQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity, staleTime: Infinity } },
  });
}

/** Exposes the live auth context to the surrounding test scope. */
function AuthHarness({
  ctxRef,
}: {
  ctxRef: { current: ReturnType<typeof useAuth> | null };
}): React.ReactElement {
  const auth = useAuth();
  useEffect(() => {
    ctxRef.current = auth;
  });
  return <div data-testid="auth-state">{auth.isAuthenticated ? 'authed' : 'anon'}</div>;
}

function DashboardStub(): React.ReactElement {
  return <div>Dashboard page</div>;
}
function LoginStub(): React.ReactElement {
  return <div>Login page</div>;
}

interface RenderResult {
  ctxRef: { current: ReturnType<typeof useAuth> | null };
  queryClient: QueryClient;
}

/**
 * Render the real AuthProvider with a guarded /dashboard route so we can assert
 * both the context state and the route-guard outcome (redirect to /login).
 */
function renderAuthApp(initialEntry = '/dashboard'): RenderResult {
  const ctxRef: { current: ReturnType<typeof useAuth> | null } = { current: null };
  const queryClient = makeQueryClient();

  render(
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <MemoryRouter initialEntries={[initialEntry]}>
          <AuthHarness ctxRef={ctxRef} />
          <Routes>
            <Route path="/login" element={<LoginStub />} />
            <Route
              path="/dashboard"
              element={
                <ProtectedRoute>
                  <DashboardStub />
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

describe('AuthContext.login — email/password flow (Story 79.2)', () => {
  beforeEach(() => {
    loginSpy.mockReset();
    refreshTokenSpy.mockReset();
    localStorage.clear();
    sessionStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  it('persists tokens/user/tenants, authenticates, and unlocks a guarded route', async () => {
    const resp = makeLoginResponse();
    loginSpy.mockResolvedValue(resp);

    // Start unauthenticated on the guarded route — the guard must redirect to
    // /login until login() succeeds.
    const { ctxRef } = renderAuthApp('/dashboard');

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });
    expect(screen.getByText('Login page')).toBeInTheDocument();
    expect(ctxRef.current?.isAuthenticated).toBe(false);

    await act(async () => {
      await ctxRef.current?.login({ email: 'manager@example.com', password: 'pw' });
    });

    // login() called once with the credentials.
    expect(loginSpy).toHaveBeenCalledTimes(1);
    expect(loginSpy).toHaveBeenCalledWith({ email: 'manager@example.com', password: 'pw' });

    // Tokens + user + tenants persisted to localStorage.
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBe(resp.accessToken);
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBe(resp.refreshToken);
    const storedUser = JSON.parse(localStorage.getItem(USER_KEY) as string);
    expect(storedUser.id).toBe('u-login-1');
    // Role derived from the single tenant membership.
    expect(storedUser.role).toBe('manager');
    const storedTenants = JSON.parse(localStorage.getItem(TENANTS_KEY) as string);
    expect(storedTenants).toHaveLength(1);

    // Session is now authenticated.
    expect(ctxRef.current?.isAuthenticated).toBe(true);
  });

  it('does not authenticate or persist tokens when login rejects', async () => {
    loginSpy.mockRejectedValue(new Error('invalid_credentials'));
    const { ctxRef } = renderAuthApp('/dashboard');

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    await act(async () => {
      await expect(ctxRef.current?.login({ email: 'x@y.z', password: 'wrong' })).rejects.toThrow(
        'invalid_credentials'
      );
    });

    // No tokens written, session stays anonymous, guard still shows /login.
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBeNull();
    expect(ctxRef.current?.isAuthenticated).toBe(false);
    expect(screen.getByText('Login page')).toBeInTheDocument();
  });
});

describe('AuthContext.login — signup-funnel analytics (issue #2530)', () => {
  let captured: AnalyticsEvent[];

  beforeEach(() => {
    loginSpy.mockReset();
    refreshTokenSpy.mockReset();
    localStorage.clear();
    sessionStorage.clear();
    captured = [];
    registerAnalyticsSink((e) => captured.push(e));
  });

  afterEach(() => {
    __clearAnalyticsSinks();
    localStorage.clear();
    sessionStorage.clear();
  });

  it('emits signup.logged_in after a successful email/password login', async () => {
    loginSpy.mockResolvedValue(makeLoginResponse());
    const { ctxRef } = renderAuthApp('/dashboard');

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    await act(async () => {
      await ctxRef.current?.login({ email: 'manager@example.com', password: 'pw' });
    });

    const signupEvents = captured.filter((e) => e.event === 'signup.logged_in');
    expect(signupEvents).toHaveLength(1);
    expect(signupEvents[0].properties).toMatchObject({ method: 'email_password' });
  });

  it('does not emit signup.logged_in when login rejects', async () => {
    loginSpy.mockRejectedValue(new Error('invalid_credentials'));
    const { ctxRef } = renderAuthApp('/dashboard');

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    await act(async () => {
      await expect(ctxRef.current?.login({ email: 'x@y.z', password: 'wrong' })).rejects.toThrow(
        'invalid_credentials'
      );
    });

    expect(captured.some((e) => e.event === 'signup.logged_in')).toBe(false);
  });
});

describe('AuthContext.refreshToken — revoked/expired refresh token (Story 79.2)', () => {
  beforeEach(() => {
    loginSpy.mockReset();
    refreshTokenSpy.mockReset();
    localStorage.clear();
    sessionStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  function seedAuthedSession() {
    localStorage.setItem(ACCESS_TOKEN_KEY, 'stale-access');
    localStorage.setItem(REFRESH_TOKEN_KEY, 'revoked-refresh');
    localStorage.setItem(USER_KEY, JSON.stringify({ id: 'u-1', email: 'a@b.c', role: 'manager' }));
    localStorage.setItem(
      TENANTS_KEY,
      JSON.stringify([{ tenantId: 't-1', tenantName: 'Acme', role: 'manager' }])
    );
  }

  it('clears the session (clean logout) when refresh is rejected — no error loop', async () => {
    seedAuthedSession();
    // A revoked / family-reuse refresh token: the server rejects the rotation.
    refreshTokenSpy.mockRejectedValue(new Error('refresh_token_revoked'));

    const { ctxRef } = renderAuthApp('/dashboard');

    // Bootstraps as authenticated from the seeded stored session.
    await waitFor(() => {
      expect(ctxRef.current?.isAuthenticated).toBe(true);
    });
    expect(screen.getByText('Dashboard page')).toBeInTheDocument();

    // A silent refresh hits a revoked token. The failure branch must clear
    // storage + de-auth and rethrow ONCE (so the caller can react) rather than
    // leaving a half-cleared session that re-triggers refresh in a loop.
    await act(async () => {
      await expect(ctxRef.current?.refreshToken()).rejects.toThrow('refresh_token_revoked');
    });

    // Clean logout: storage purged, session anonymous, guard redirects to /login.
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(USER_KEY)).toBeNull();
    expect(localStorage.getItem(TENANTS_KEY)).toBeNull();

    await waitFor(() => {
      expect(ctxRef.current?.isAuthenticated).toBe(false);
    });
    await waitFor(() => {
      expect(screen.getByText('Login page')).toBeInTheDocument();
    });
    expect(screen.queryByText('Dashboard page')).not.toBeInTheDocument();

    // The rejected refresh was attempted exactly once — no retry storm.
    expect(refreshTokenSpy).toHaveBeenCalledTimes(1);
    expect(refreshTokenSpy).toHaveBeenCalledWith({ refreshToken: 'revoked-refresh' });
  });

  it('coalesces concurrent refreshToken() calls into a single network request', async () => {
    seedAuthedSession();
    refreshTokenSpy.mockResolvedValue({
      accessToken: 'fresh-access',
      refreshToken: 'fresh-refresh',
    });

    const { ctxRef } = renderAuthApp('/dashboard');
    await waitFor(() => {
      expect(ctxRef.current?.isAuthenticated).toBe(true);
    });

    // Fire two refreshes back-to-back; the in-flight guard must dedupe them so
    // a burst of 401s doesn't fan out into N refresh requests.
    await act(async () => {
      await Promise.all([ctxRef.current?.refreshToken(), ctxRef.current?.refreshToken()]);
    });

    expect(refreshTokenSpy).toHaveBeenCalledTimes(1);
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBe('fresh-access');
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBe('fresh-refresh');
    // Session remains authenticated after a successful rotation.
    expect(ctxRef.current?.isAuthenticated).toBe(true);
  });
});
