/**
 * Regression tests for the rentals mutation auth guard.
 *
 * Bug (code-review finding `code-review-ppt-web-core-rentals-auth-nonnull-mutation`):
 * the rentals mutations built their request headers with `auth!.authorization`
 * / `auth!.xTenantId`. Unlike the queries (which gate on `enabled: !!auth`),
 * a `mutationFn` runs unconditionally on `.mutate()`, so a mid-session token
 * loss (auth === null) made the dereference throw an opaque
 * `TypeError: Cannot read properties of null` with no auth semantics — nothing
 * mapped it to a session-expired UX.
 *
 * The fix routes every rentals request through `requireRentalsAuthHeaders`,
 * which throws a typed `AuthError('SESSION_EXPIRED')` when auth is missing, and
 * wires the mutations' `onError` to `handleRentalsAuthError`, which redirects to
 * /login (via AuthContext `logout`) — matching how the rest of the app handles
 * a dropped session.
 *
 * These tests prove the two seams the mutations rely on, plus one end-to-end
 * check that a real `useMutation` wired exactly as the route wires it settles
 * in a *handled* error state (not an uncaught throw) and triggers the redirect.
 */
import { AuthError } from '@ppt/api-client';
import { QueryClient, QueryClientProvider, useMutation } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { handleRentalsAuthError, requireRentalsAuthHeaders } from './rentals';

describe('requireRentalsAuthHeaders', () => {
  it('throws a typed AuthError(SESSION_EXPIRED) — not a raw TypeError — when auth is null', () => {
    let thrown: unknown;
    try {
      requireRentalsAuthHeaders(null);
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(AuthError);
    // The whole point of the fix: the missing-auth failure is a recognised auth
    // error, not the opaque TypeError the `auth!` dereference used to produce.
    expect(thrown).not.toBeInstanceOf(TypeError);
    expect((thrown as AuthError).code).toBe('SESSION_EXPIRED');
  });

  it('returns the generated-client header map when auth is present', () => {
    expect(
      requireRentalsAuthHeaders({ authorization: 'Bearer token-xyz', xTenantId: 'org-1' })
    ).toEqual({ Authorization: 'Bearer token-xyz', 'X-Tenant-ID': 'org-1' });
  });

  it('documents the pre-fix hazard: dereferencing null auth throws a raw TypeError', () => {
    // The old code did `auth!.authorization`; with auth === null that is a
    // TypeError with no auth semantics — exactly what the guard now replaces.
    expect(() => (null as unknown as { authorization: string }).authorization).toThrow(TypeError);
  });
});

describe('handleRentalsAuthError', () => {
  it('logs out (redirect to /login) on every session-loss AuthError code', () => {
    const logout = vi.fn();
    for (const code of ['SESSION_EXPIRED', 'TOKEN_EXPIRED', 'TOKEN_INVALID'] as const) {
      logout.mockClear();
      const handled = handleRentalsAuthError(new AuthError('gone', code), logout);
      expect(handled).toBe(true);
      expect(logout).toHaveBeenCalledTimes(1);
    }
  });

  it('leaves non-session errors alone (no logout)', () => {
    const logout = vi.fn();
    expect(handleRentalsAuthError(new AuthError('bad', 'INVALID_CREDENTIALS'), logout)).toBe(false);
    expect(handleRentalsAuthError(new Error('network down'), logout)).toBe(false);
    expect(handleRentalsAuthError(new TypeError('boom'), logout)).toBe(false);
    expect(logout).not.toHaveBeenCalled();
  });
});

describe('rentals mutation wired as the route wires it (mid-session token loss)', () => {
  function wrapper({ children }: { children: ReactNode }) {
    const client = new QueryClient({ defaultOptions: { mutations: { retry: false } } });
    return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
  }

  it('settles in a handled AuthError state (not thrown), never calls the API, and triggers logout', async () => {
    const apiCall = vi.fn().mockResolvedValue({ data: {} });
    const logout = vi.fn();
    // Mid-session token loss: the auth the mutation closes over is null.
    const auth = null;

    const { result } = renderHook(
      () =>
        useMutation({
          // Same shape as the production rentals mutations.
          mutationFn: () => apiCall({ headers: requireRentalsAuthHeaders(auth) }),
          onError: (error) => handleRentalsAuthError(error, logout),
        }),
      { wrapper }
    );

    // `.mutate()` must NOT surface an uncaught throw — TanStack Query catches
    // the AuthError thrown inside mutationFn and routes it to the error state.
    result.current.mutate();

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toBeInstanceOf(AuthError);
    expect(result.current.error).not.toBeInstanceOf(TypeError);
    expect((result.current.error as AuthError).code).toBe('SESSION_EXPIRED');
    // The guard short-circuits before any network call is attempted.
    expect(apiCall).not.toHaveBeenCalled();
    // The handled auth error drives the session-expired UX (redirect to /login).
    expect(logout).toHaveBeenCalledTimes(1);
  });
});
