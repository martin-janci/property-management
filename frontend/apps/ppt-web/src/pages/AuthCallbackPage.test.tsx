/**
 * AuthCallbackPage integration tests — Story 79.2 SSO /auth/callback flow.
 *
 * Test gap closed: `test-gap-79-2-auth-callback-e2e`. PRs #568/#609/#642
 * landed the SSO code-exchange + token-storage + role-derivation flow without
 * an integration test that drives the *page* end-to-end against the real
 * `AuthProvider`. The existing `AuthContext.test.tsx` covers logout only.
 *
 * These tests render the REAL `AuthCallbackPage` inside the REAL `AuthProvider`
 * and a `MemoryRouter`, mocking only the api-client network boundary
 * (`exchangeSsoCode` / `refreshToken`). Everything else is exercised for real:
 *   - SSO state-nonce verification via `@ppt/shared` (real sessionStorage).
 *   - Token + user + tenant storage via `tokenStorage` (real localStorage).
 *   - Post-exchange refresh path that reads the persisted refresh token.
 *   - Redirect to the stored return URL (or /dashboard).
 *   - Error rendering for provider errors / missing params / state mismatch /
 *     exchange failure (and that no tokens are persisted on failure).
 *
 * Covered guarantees:
 *   1. Happy path: code+state → exchangeSsoCode → access/refresh/user/tenants
 *      written to localStorage → redirect to /dashboard.
 *   2. Return URL: a stored return URL wins over the /dashboard default.
 *   3. Refresh flow: after callback stores a refresh token, a subsequent
 *      AuthProvider mount rehydrates the session and a refreshToken() call
 *      rotates the access token (token-storage + refresh integration).
 *   4. State mismatch: stored nonce ≠ URL state → error, no network call,
 *      nonce consumed.
 *   5. Missing params: no code/state → error, no network call.
 *   6. Provider error: ?error= in the redirect → error, no network call.
 *   7. Exchange failure: backend rejects → error shown, tokens NOT persisted
 *      (rollback), session stays anonymous.
 */
/// <reference types="vitest/globals" />

// jsdom under vitest 4 does not expose localStorage/sessionStorage reliably.
// Provide in-memory shims before any SUT module is imported so both
// tokenStorage.* (localStorage) and the @ppt/shared SSO helpers
// (sessionStorage) can read/write. Mirrors the shim in AuthContext.test.tsx.
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

import type {
  AuthApi,
  LoginResponse,
  RefreshTokenResponse,
  SsoCallbackRequest,
} from '@ppt/api-client';
import { setSsoState } from '@ppt/shared';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import type React from 'react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AuthProvider } from '../contexts/AuthContext';
import { AuthCallbackPage } from './AuthCallbackPage';

// ---------------------------------------------------------------------------
// Mocks — only the api-client network boundary.
// ---------------------------------------------------------------------------

const exchangeSsoCodeSpy = vi.fn<(req: SsoCallbackRequest) => Promise<LoginResponse>>();
const refreshTokenSpy = vi.fn<(req: { refreshToken: string }) => Promise<RefreshTokenResponse>>();

const fakeAuthApi: Partial<AuthApi> = {
  exchangeSsoCode: exchangeSsoCodeSpy as unknown as AuthApi['exchangeSsoCode'],
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

const STATE_NONCE = 'state-nonce-abc123';
const AUTH_CODE = 'auth-code-xyz789';

function makeLoginResponse(overrides: Partial<LoginResponse> = {}): LoginResponse {
  return {
    accessToken: 'sso-access-token-1',
    refreshToken: 'sso-refresh-token-1',
    expiresIn: 3600,
    user: {
      id: 'u-sso-1',
      email: 'sso.user@example.com',
      firstName: 'Sso',
      lastName: 'User',
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

function DashboardStub(): React.ReactElement {
  return <div>Dashboard page</div>;
}
function SettingsStub(): React.ReactElement {
  return <div>Settings page</div>;
}
function LoginStub(): React.ReactElement {
  return <div>Login page</div>;
}

/**
 * Render the real AuthCallbackPage at /auth/callback with the given query
 * string, wrapped in the real AuthProvider + a MemoryRouter that also has
 * /dashboard, /settings and /login routes so we can assert the redirect target.
 */
function renderCallback(search: string) {
  const queryClient = makeQueryClient();
  return render(
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <MemoryRouter initialEntries={[`/auth/callback${search}`]}>
          <Routes>
            <Route path="/auth/callback" element={<AuthCallbackPage />} />
            <Route path="/dashboard" element={<DashboardStub />} />
            <Route path="/settings" element={<SettingsStub />} />
            <Route path="/login" element={<LoginStub />} />
          </Routes>
        </MemoryRouter>
      </AuthProvider>
    </QueryClientProvider>
  );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('AuthCallbackPage — SSO /auth/callback flow (Story 79.2)', () => {
  beforeEach(() => {
    exchangeSsoCodeSpy.mockReset();
    refreshTokenSpy.mockReset();
    localStorage.clear();
    sessionStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  it('exchanges code+state, persists tokens/user/tenants, and redirects to /dashboard', async () => {
    setSsoState(STATE_NONCE);
    const resp = makeLoginResponse();
    exchangeSsoCodeSpy.mockResolvedValue(resp);

    renderCallback(`?code=${AUTH_CODE}&state=${STATE_NONCE}`);

    // Redirect to the default dashboard once the exchange resolves.
    await waitFor(() => {
      expect(screen.getByText('Dashboard page')).toBeInTheDocument();
    });

    // exchangeSsoCode called once with code, state and the derived redirectUri.
    expect(exchangeSsoCodeSpy).toHaveBeenCalledTimes(1);
    const arg = exchangeSsoCodeSpy.mock.calls[0][0];
    expect(arg.code).toBe(AUTH_CODE);
    expect(arg.state).toBe(STATE_NONCE);
    expect(arg.redirectUri).toContain('/auth/callback');

    // Tokens + user + tenants persisted to localStorage (the storage flow).
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBe(resp.accessToken);
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBe(resp.refreshToken);
    const storedUser = JSON.parse(localStorage.getItem(USER_KEY) as string);
    expect(storedUser.id).toBe('u-sso-1');
    // Role derived from the single tenant membership.
    expect(storedUser.role).toBe('manager');
    const storedTenants = JSON.parse(localStorage.getItem(TENANTS_KEY) as string);
    expect(storedTenants).toHaveLength(1);
    expect(storedTenants[0].tenantId).toBe('t-1');

    // The one-shot state nonce has been consumed.
    expect(sessionStorage.getItem('sso_state')).toBeNull();
  });

  it('redirects to the stored return URL when present', async () => {
    setSsoState(STATE_NONCE);
    // setReturnUrl sanitizes; write directly to the sanitized key it reads.
    sessionStorage.setItem('auth_return_url', '/settings');
    exchangeSsoCodeSpy.mockResolvedValue(makeLoginResponse());

    renderCallback(`?code=${AUTH_CODE}&state=${STATE_NONCE}`);

    await waitFor(() => {
      expect(screen.getByText('Settings page')).toBeInTheDocument();
    });
    expect(screen.queryByText('Dashboard page')).not.toBeInTheDocument();
    // Return URL consumed (single-use).
    expect(sessionStorage.getItem('auth_return_url')).toBeNull();
  });

  it('persists a refresh token that a later mount uses to rotate the access token', async () => {
    // 1. Complete the SSO callback so a refresh token is stored.
    setSsoState(STATE_NONCE);
    exchangeSsoCodeSpy.mockResolvedValue(
      makeLoginResponse({ accessToken: 'access-v1', refreshToken: 'refresh-v1' })
    );

    const first = renderCallback(`?code=${AUTH_CODE}&state=${STATE_NONCE}`);
    await waitFor(() => {
      expect(screen.getByText('Dashboard page')).toBeInTheDocument();
    });
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBe('refresh-v1');
    first.unmount();

    // 2. A fresh AuthProvider mount with only a refresh token (no user/access)
    //    must call the API to rotate the token — the refresh-flow integration.
    localStorage.removeItem(ACCESS_TOKEN_KEY);
    localStorage.removeItem(USER_KEY);
    refreshTokenSpy.mockResolvedValue({
      accessToken: 'access-v2',
      refreshToken: 'refresh-v2',
    });

    render(
      <QueryClientProvider client={makeQueryClient()}>
        <AuthProvider>
          <MemoryRouter initialEntries={['/dashboard']}>
            <Routes>
              <Route path="/dashboard" element={<DashboardStub />} />
            </Routes>
          </MemoryRouter>
        </AuthProvider>
      </QueryClientProvider>
    );

    await waitFor(() => {
      expect(refreshTokenSpy).toHaveBeenCalledWith({ refreshToken: 'refresh-v1' });
    });
    await waitFor(() => {
      expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBe('access-v2');
    });
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBe('refresh-v2');
  });

  it('rejects a state-nonce mismatch without any network call and consumes the nonce', async () => {
    setSsoState(STATE_NONCE);
    exchangeSsoCodeSpy.mockResolvedValue(makeLoginResponse());

    renderCallback(`?code=${AUTH_CODE}&state=tampered-state`);

    await waitFor(() => {
      expect(screen.getByText(/state parameter mismatch/i)).toBeInTheDocument();
    });
    expect(exchangeSsoCodeSpy).not.toHaveBeenCalled();
    // Nonce consumed even on mismatch (no replay).
    expect(sessionStorage.getItem('sso_state')).toBeNull();
    // No tokens persisted.
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBeNull();
  });

  it('shows an error and makes no network call when code/state are missing', async () => {
    setSsoState(STATE_NONCE);

    renderCallback('?code=only-code');

    await waitFor(() => {
      expect(screen.getByText(/missing code or state/i)).toBeInTheDocument();
    });
    expect(exchangeSsoCodeSpy).not.toHaveBeenCalled();
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBeNull();
  });

  it('surfaces a provider-returned error without attempting an exchange', async () => {
    setSsoState(STATE_NONCE);

    renderCallback('?error=access_denied&error_description=User%20declined%20consent');

    await waitFor(() => {
      expect(screen.getByText('User declined consent')).toBeInTheDocument();
    });
    expect(exchangeSsoCodeSpy).not.toHaveBeenCalled();
  });

  it('rolls back token storage and stays anonymous when the exchange fails', async () => {
    setSsoState(STATE_NONCE);
    exchangeSsoCodeSpy.mockRejectedValue(new Error('invalid_grant'));

    renderCallback(`?code=${AUTH_CODE}&state=${STATE_NONCE}`);

    await waitFor(() => {
      expect(screen.getByText('invalid_grant')).toBeInTheDocument();
    });

    expect(exchangeSsoCodeSpy).toHaveBeenCalledTimes(1);
    // No partial token writes survive a failed exchange (rollback in
    // loginWithSsoCode clears storage).
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(USER_KEY)).toBeNull();

    // "Back to login" recovery affordance is present.
    expect(screen.getByRole('button', { name: /back to login/i })).toBeInTheDocument();
    expect(screen.queryByText('Dashboard page')).not.toBeInTheDocument();
  });
});
