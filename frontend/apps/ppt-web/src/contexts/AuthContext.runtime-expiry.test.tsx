/**
 * AuthContext runtime access-token expiry — proactive-refresh regression.
 *
 * Context: the cold-boot init path already validates the stored access token's
 * `exp` claim and routes an expired-but-present token through the silent refresh
 * (see AuthContext.init-refresh.test.tsx). But the RUNTIME token provider —
 * `getAccessToken`, registered via setTokenProvider / configureApiClient and
 * read on every outgoing request — returned the stored token verbatim with no
 * expiry check. An access token that lapsed MID-SESSION was therefore handed out
 * as if valid and only recovered reactively: the request went out with the stale
 * token, 401'd, and only then did the interceptor fire a refresh.
 *
 * The fix makes `getAccessToken` decode the token's `exp` and, when it is
 * expired (or within the clock-skew buffer) and the session looks recoverable,
 * kick off a single-flight silent refresh proactively — before the next request
 * fails. It stays fire-and-forget (the call still returns the current token
 * synchronously), and opaque / non-`exp` tokens are never treated as expired.
 *
 * Renders the REAL AuthProvider, mocking only the api-client network boundary.
 */
/// <reference types="vitest/globals" />

// jsdom under vitest 4 does not expose localStorage by default. Provide an
// in-memory shim before any SUT module is imported (mirrors the sibling
// AuthContext.init-refresh.test.tsx suite).
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

import type { AuthApi, RefreshTokenResponse } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, waitFor } from '@testing-library/react';
import type React from 'react';
import { useEffect } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AuthProvider, useAuth } from './AuthContext';

// ---------------------------------------------------------------------------
// Mocks — only the api-client network boundary.
// ---------------------------------------------------------------------------

const refreshTokenSpy = vi.fn<(req: { refreshToken: string }) => Promise<RefreshTokenResponse>>();

const fakeAuthApi: Partial<AuthApi> = {
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
const USER_KEY = 'ppt_user';
const TENANTS_KEY = 'ppt_tenants';

/** Build a (structurally valid, unsigned) JWT carrying the given claims. */
function makeJwt(claims: Record<string, unknown>): string {
  const b64url = (obj: unknown) => Buffer.from(JSON.stringify(obj)).toString('base64url');
  return `${b64url({ alg: 'none', typ: 'JWT' })}.${b64url(claims)}.sig`;
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

function makeQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity, staleTime: Infinity } },
  });
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('AuthContext runtime token provider — proactive expiry refresh', () => {
  const NOW_SECONDS = Math.floor(Date.now() / 1000);

  beforeEach(() => {
    refreshTokenSpy.mockReset();
    localStorage.clear();
    sessionStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  function seedUser(role = 'manager') {
    localStorage.setItem(USER_KEY, JSON.stringify({ id: 'u-1', email: 'a@b.c', role }));
    localStorage.setItem(
      TENANTS_KEY,
      JSON.stringify([{ tenantId: 't-1', tenantName: 'Acme', role: 'org_admin' }])
    );
  }

  /**
   * Boot with a VALID access token (so init does not refresh), then let it lapse
   * mid-session and read it back through the token provider. The read must kick
   * off a single-flight refresh rather than hand the stale token out unchanged.
   */
  it('triggers a silent refresh when getAccessToken is read with an expired token mid-session', async () => {
    seedUser('manager');
    localStorage.setItem(ACCESS_TOKEN_KEY, makeJwt({ sub: 'u-1', exp: NOW_SECONDS + 3600 }));
    refreshTokenSpy.mockResolvedValue({
      accessToken: makeJwt({ sub: 'u-1', tenant_id: 't-1', exp: NOW_SECONDS + 3600 }),
      refreshToken: 'fresh-refresh',
    });

    const { ctxRef } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });
    // Boot trusted the still-valid token — no refresh yet.
    expect(refreshTokenSpy).not.toHaveBeenCalled();
    expect(ctxRef.current?.isAuthenticated).toBe(true);

    // The access token lapses mid-session (e.g. the tab idled past its TTL).
    localStorage.setItem(ACCESS_TOKEN_KEY, makeJwt({ sub: 'u-1', exp: NOW_SECONDS - 60 }));

    // Reading the token through the provider (as every outgoing request does)
    // must proactively start the refresh. The current token is still returned
    // synchronously. This is the assertion that fails pre-fix.
    const returned = ctxRef.current?.getAccessToken();
    expect(returned).not.toBeNull();

    await waitFor(() => {
      expect(refreshTokenSpy).toHaveBeenCalledTimes(1);
    });
    expect(refreshTokenSpy).toHaveBeenCalledWith({ refreshToken: '' });
  });

  it('does NOT refresh when the token read through the provider is still valid', async () => {
    seedUser('manager');
    localStorage.setItem(ACCESS_TOKEN_KEY, makeJwt({ sub: 'u-1', exp: NOW_SECONDS + 3600 }));

    const { ctxRef } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    // Repeated reads of a fresh token stay side-effect free.
    ctxRef.current?.getAccessToken();
    ctxRef.current?.getAccessToken();

    expect(refreshTokenSpy).not.toHaveBeenCalled();
    expect(ctxRef.current?.isAuthenticated).toBe(true);
  });

  it('does NOT refresh on an expired opaque (non-JWT) token — exp cannot be proven stale', async () => {
    seedUser('manager');
    // A still-valid JWT at boot so the session is authenticated without a
    // refresh, then swap in an opaque token that cannot be decoded.
    localStorage.setItem(ACCESS_TOKEN_KEY, makeJwt({ sub: 'u-1', exp: NOW_SECONDS + 3600 }));

    const { ctxRef } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    localStorage.setItem(ACCESS_TOKEN_KEY, 'opaque-access-token');
    const returned = ctxRef.current?.getAccessToken();

    // Opaque token is returned untouched and the runtime 401 interceptor stays
    // the backstop — no proactive refresh.
    expect(returned).toBe('opaque-access-token');
    expect(refreshTokenSpy).not.toHaveBeenCalled();
  });
});
