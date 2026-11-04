/**
 * Security regression: the refresh token must never be persisted to
 * `localStorage`.
 *
 * A refresh token in `localStorage` is readable by any script on the origin, so
 * an XSS foothold could exfiltrate it and mint access tokens indefinitely. The
 * token is instead held only in memory for the lifetime of the page; durable
 * cross-reload persistence is provided by the backend's HttpOnly `refresh_token`
 * cookie (which JS cannot read).
 *
 * These tests FAIL against the pre-fix implementation, which wrote the refresh
 * token to `localStorage` under `ppt_refresh_token` on every login / refresh and
 * never purged a value an older build left behind.
 *
 * Renders the REAL AuthProvider, mocking only the api-client network boundary.
 */
/// <reference types="vitest/globals" />

// jsdom under vitest 4 does not expose localStorage by default. Provide an
// in-memory shim before any SUT module is imported (mirrors the sibling
// AuthContext.*-refresh.test.tsx suites).
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
import { act, render, waitFor } from '@testing-library/react';
import type React from 'react';
import { useEffect } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AuthProvider, useAuth } from './AuthContext';

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

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
const USER_KEY = 'ppt_user';

function makeLoginResponse(): LoginResponse {
  return {
    accessToken: 'login-access-1',
    refreshToken: 'login-refresh-1',
    expiresIn: 3600,
    user: { id: 'u-1', email: 'manager@example.com', firstName: 'Mana', lastName: 'Ger' },
    tenants: [{ tenantId: 't-1', tenantName: 'Acme', role: 'manager' }],
  };
}

function makeQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity, staleTime: Infinity } },
  });
}

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

function renderAuthApp(): { ctxRef: { current: ReturnType<typeof useAuth> | null } } {
  const ctxRef: { current: ReturnType<typeof useAuth> | null } = { current: null };
  render(
    <QueryClientProvider client={makeQueryClient()}>
      <AuthProvider>
        <AuthHarness ctxRef={ctxRef} />
      </AuthProvider>
    </QueryClientProvider>
  );
  return { ctxRef };
}

describe('AuthContext — refresh token is never persisted to localStorage (XSS hardening)', () => {
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

  it('keeps the refresh token out of localStorage on login but still usable in memory', async () => {
    loginSpy.mockResolvedValue(makeLoginResponse());
    const { ctxRef } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    await act(async () => {
      await ctxRef.current?.login({ email: 'manager@example.com', password: 'pw' });
    });

    // Access token + user are still persisted (out of scope here), but the
    // refresh token must NOT be written to localStorage.
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBe('login-access-1');
    expect(localStorage.getItem(USER_KEY)).not.toBeNull();
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBeNull();

    // The session is authenticated and the in-memory refresh token is what a
    // same-page-load refresh uses (the backend prefers the HttpOnly cookie, but
    // the body still carries the live token here).
    expect(ctxRef.current?.isAuthenticated).toBe(true);
    refreshTokenSpy.mockResolvedValue({
      accessToken: 'login-access-2',
      refreshToken: 'login-refresh-2',
    });
    await act(async () => {
      await ctxRef.current?.refreshToken();
    });
    expect(refreshTokenSpy).toHaveBeenCalledWith({ refreshToken: 'login-refresh-1' });

    // The rotated refresh token is likewise not persisted to localStorage.
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBeNull();
  });

  it('purges a refresh token left in localStorage by an older build on boot', async () => {
    // Simulate upgrading over a previous (localStorage-based) build.
    localStorage.setItem(REFRESH_TOKEN_KEY, 'legacy-xss-reachable-token');
    localStorage.setItem(ACCESS_TOKEN_KEY, 'opaque-access');
    localStorage.setItem(USER_KEY, JSON.stringify({ id: 'u-1', email: 'a@b.c', role: 'manager' }));

    const { ctxRef } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    // The stale, JS-readable refresh token must be gone after boot.
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBeNull();
  });
});
