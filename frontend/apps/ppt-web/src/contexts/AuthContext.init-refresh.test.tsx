/**
 * AuthContext cold-boot init refresh — role-staleness regression (#574 gap).
 *
 * Context: #574 taught the runtime silent-refresh path (refreshTokenInternal)
 * to re-derive the user's role from the fresh access token + persisted tenant
 * memberships, so a server-side role change (e.g. a promotion to org_admin)
 * propagates on the next token refresh without a full re-login.
 *
 * The gap this suite guards: the AuthProvider mount/init effect had its OWN
 * inline copy of the refresh call for the cold-boot case (a persisted refresh
 * token but no live access token — e.g. the access token was cleared or the
 * tab was closed past its lifetime). That inline path rotated the tokens but
 * re-hydrated `user` straight from storage, bypassing deriveActiveRole — so on
 * a cold boot the user kept a STALE role until the next runtime refresh.
 *
 * The fix routes the init cold-boot branch through refreshTokenInternal, the
 * same routine the runtime path uses. This suite asserts that a cold boot
 * picks up the fresh role.
 *
 * Renders the REAL AuthProvider, mocking only the api-client network boundary.
 */
/// <reference types="vitest/globals" />

// jsdom under vitest 4 does not expose localStorage by default. Provide an
// in-memory shim before any SUT module is imported (mirrors the shim in the
// sibling AuthContext.login-refresh.test.tsx suite).
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
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
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

describe('AuthContext cold-boot init — role re-derivation (#574 gap)', () => {
  beforeEach(() => {
    refreshTokenSpy.mockReset();
    localStorage.clear();
    sessionStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  /**
   * Seed a cold-boot session: a persisted refresh token, a stored user with a
   * STALE role, and tenants reflecting a server-side promotion — but NO live
   * access token, so the init effect must take the cold-boot refresh branch.
   */
  function seedColdBootSession() {
    localStorage.setItem(REFRESH_TOKEN_KEY, 'cold-refresh');
    localStorage.setItem(USER_KEY, JSON.stringify({ id: 'u-1', email: 'a@b.c', role: 'manager' }));
    // Server has since promoted this membership to org_admin.
    localStorage.setItem(
      TENANTS_KEY,
      JSON.stringify([{ tenantId: 't-1', tenantName: 'Acme', role: 'org_admin' }])
    );
  }

  it('re-derives the fresh role on cold boot instead of keeping the stale stored role', async () => {
    seedColdBootSession();
    // Rotation returns a fresh access token whose tenant_id claim selects the
    // (now promoted) membership.
    refreshTokenSpy.mockResolvedValue({
      accessToken: makeJwt({ sub: 'u-1', tenant_id: 't-1' }),
      refreshToken: 'fresh-refresh',
    });

    const { ctxRef } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    // Cold-boot refresh went through refreshTokenInternal exactly once.
    expect(refreshTokenSpy).toHaveBeenCalledTimes(1);
    expect(refreshTokenSpy).toHaveBeenCalledWith({ refreshToken: 'cold-refresh' });

    // Session is authenticated and the role reflects the server-side promotion,
    // NOT the stale 'manager' stored on the user record. This is the assertion
    // that fails against the pre-fix inline init path.
    await waitFor(() => {
      expect(ctxRef.current?.isAuthenticated).toBe(true);
    });
    expect(ctxRef.current?.user?.role).toBe('org_admin');

    // Fresh tokens + the updated user role are persisted for the next boot.
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBe('fresh-refresh');
    const storedUser = JSON.parse(localStorage.getItem(USER_KEY) as string);
    expect(storedUser.role).toBe('org_admin');
  });

  it('clears the session cleanly when the cold-boot refresh is rejected', async () => {
    seedColdBootSession();
    refreshTokenSpy.mockRejectedValue(new Error('refresh_token_revoked'));

    const { ctxRef } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    // A revoked cold-boot refresh must leave a clean anonymous session — storage
    // purged, no half-authenticated state.
    expect(ctxRef.current?.isAuthenticated).toBe(false);
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(USER_KEY)).toBeNull();
    expect(localStorage.getItem(TENANTS_KEY)).toBeNull();
  });
});

/**
 * Boot-time access-token expiry check.
 *
 * Before the fix, initializeAuth marked the session authenticated whenever a
 * stored user + access token were present — the comment claimed an `exp` check
 * that did not exist. An expired-but-present access token therefore skipped the
 * boot-time silent refresh: the session went authenticated, the first request
 * 401'd, and only THEN a refresh fired — an authenticated -> 401 -> refresh
 * flicker on every cold boot with a stale token.
 *
 * The fix validates the stored access token's `exp` at cold boot and, when it
 * is expired (or within the clock-skew buffer), routes through the silent
 * refresh path instead of trusting the stale token.
 */
describe('AuthContext cold-boot init — access-token exp validation', () => {
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

  it('routes an expired-but-present access token through the silent refresh (no flicker)', async () => {
    // Stored access token whose `exp` is well in the past, plus a live refresh
    // token — the exact expired-but-present cold boot the fix targets.
    seedUser('manager');
    localStorage.setItem(ACCESS_TOKEN_KEY, makeJwt({ sub: 'u-1', exp: NOW_SECONDS - 60 }));
    localStorage.setItem(REFRESH_TOKEN_KEY, 'cold-refresh');
    refreshTokenSpy.mockResolvedValue({
      accessToken: makeJwt({ sub: 'u-1', tenant_id: 't-1', exp: NOW_SECONDS + 3600 }),
      refreshToken: 'fresh-refresh',
    });

    const { ctxRef } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    // The boot took the silent-refresh path exactly once instead of trusting
    // the stale token — this is the assertion that fails against the pre-fix
    // "mark authenticated regardless of exp" behaviour.
    expect(refreshTokenSpy).toHaveBeenCalledTimes(1);
    expect(refreshTokenSpy).toHaveBeenCalledWith({ refreshToken: 'cold-refresh' });

    await waitFor(() => {
      expect(ctxRef.current?.isAuthenticated).toBe(true);
    });
    // Role reflects the fresh token's tenant, and rotated tokens are persisted.
    expect(ctxRef.current?.user?.role).toBe('org_admin');
    expect(localStorage.getItem(REFRESH_TOKEN_KEY)).toBe('fresh-refresh');
  });

  it('near-expiry (inside the skew buffer) access token also triggers a refresh', async () => {
    seedUser('manager');
    // exp is in the future but within the ~30s skew buffer — treated as stale.
    localStorage.setItem(ACCESS_TOKEN_KEY, makeJwt({ sub: 'u-1', exp: NOW_SECONDS + 5 }));
    localStorage.setItem(REFRESH_TOKEN_KEY, 'cold-refresh');
    refreshTokenSpy.mockResolvedValue({
      accessToken: makeJwt({ sub: 'u-1', tenant_id: 't-1', exp: NOW_SECONDS + 3600 }),
      refreshToken: 'fresh-refresh',
    });

    const { ctxRef } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    expect(refreshTokenSpy).toHaveBeenCalledTimes(1);
    await waitFor(() => {
      expect(ctxRef.current?.isAuthenticated).toBe(true);
    });
  });

  it('a still-valid access token marks the session authenticated WITHOUT a refresh', async () => {
    seedUser('manager');
    // exp comfortably in the future — no network round-trip should happen.
    localStorage.setItem(ACCESS_TOKEN_KEY, makeJwt({ sub: 'u-1', exp: NOW_SECONDS + 3600 }));
    localStorage.setItem(REFRESH_TOKEN_KEY, 'cold-refresh');

    const { ctxRef } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    expect(refreshTokenSpy).not.toHaveBeenCalled();
    expect(ctxRef.current?.isAuthenticated).toBe(true);
  });

  it('an expired access token with NO refresh token leaves a clean anonymous session', async () => {
    seedUser('manager');
    localStorage.setItem(ACCESS_TOKEN_KEY, makeJwt({ sub: 'u-1', exp: NOW_SECONDS - 60 }));
    // No refresh token — nothing can recover the session.

    const { ctxRef } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    expect(refreshTokenSpy).not.toHaveBeenCalled();
    expect(ctxRef.current?.isAuthenticated).toBe(false);
    // Stale tokens are purged rather than left to 401 on the next request.
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBeNull();
  });

  it('an opaque (non-JWT) access token is trusted at boot — exp cannot be proven stale', async () => {
    // Preserves the legacy behaviour for tokens we cannot decode: they are NOT
    // treated as expired, and the runtime 401 interceptor stays the backstop.
    seedUser('manager');
    localStorage.setItem(ACCESS_TOKEN_KEY, 'opaque-access-token');
    localStorage.setItem(REFRESH_TOKEN_KEY, 'cold-refresh');

    const { ctxRef } = renderAuthApp();

    await waitFor(() => {
      expect(ctxRef.current?.isLoading).toBe(false);
    });

    expect(refreshTokenSpy).not.toHaveBeenCalled();
    expect(ctxRef.current?.isAuthenticated).toBe(true);
  });
});
