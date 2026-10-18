/**
 * Regression tests for `checkSession` in AuthProvider (reality-web).
 *
 * Bug (`code-review-reality-web-auth-ctx-transient-logout`): the bearer-token
 * path in `checkSession` treated ANY non-ok `/users/me` response as a token
 * rejection — it called `clearStoredSession()` and fell through to the cookie
 * probe, which in turn calls `setUser(null)` on its own failure. So a transient
 * 5xx (or a network blip) from the backend wiped a logged-in user's session.
 *
 * These render the REAL `AuthProvider` with a stored bearer session seeded into
 * localStorage, mocking only the `fetch` boundary, and assert:
 *   1. A transient 500 from `/users/me` keeps the user logged in, does NOT
 *      clear the stored token, and does NOT fall through to the cookie probe.
 *   2. A genuine 401 from `/users/me` DOES clear the session (fall through to
 *      cookie probe → unauthenticated → logged out).
 *   3. A network error (fetch rejects) keeps the user logged in.
 */
/// <reference types="vitest/globals" />

import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AuthProvider, useAuth } from './auth-context';

const TOKEN_KEY = 'ppt-portal-token';
const USER_KEY = 'ppt-portal-user';
const EXPIRES_KEY = 'ppt-portal-expires';

function seedStoredSession() {
  const future = new Date(Date.now() + 60 * 60 * 1000).toISOString();
  window.localStorage.setItem(TOKEN_KEY, 'stored-bearer-token');
  window.localStorage.setItem(
    USER_KEY,
    JSON.stringify({ id: 'u-1', email: 'stored@example.com', name: 'Stored User' })
  );
  window.localStorage.setItem(EXPIRES_KEY, future);
}

function Probe() {
  const { user, isAuthenticated, isLoading } = useAuth();
  return (
    <div>
      <span data-testid="auth">{isAuthenticated ? 'yes' : 'no'}</span>
      <span data-testid="email">{user?.email ?? '-'}</span>
      <span data-testid="loading">{isLoading ? 'yes' : 'no'}</span>
    </div>
  );
}

function urlOf(input: RequestInfo | URL): string {
  return typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
}

beforeEach(() => {
  window.localStorage.clear();
  vi.restoreAllMocks();
});

afterEach(() => {
  window.localStorage.clear();
});

describe('AuthProvider checkSession — transient vs genuine auth failure', () => {
  it('keeps the session on a transient 5xx from /users/me (regression)', async () => {
    seedStoredSession();
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = urlOf(input);
      if (url.includes('/users/me')) {
        return new Response('upstream error', { status: 500 });
      }
      // The cookie probe must NOT be reached on a transient failure.
      throw new Error(`unexpected fetch to ${url}`);
    });
    vi.stubGlobal('fetch', fetchMock);

    render(
      <AuthProvider>
        <Probe />
      </AuthProvider>
    );

    await waitFor(() => expect(screen.getByTestId('loading').textContent).toBe('no'));

    // Still logged in with the optimistic user.
    expect(screen.getByTestId('auth').textContent).toBe('yes');
    expect(screen.getByTestId('email').textContent).toBe('stored@example.com');
    // Token preserved.
    expect(window.localStorage.getItem(TOKEN_KEY)).toBe('stored-bearer-token');
    // Only the /users/me call happened — no fall-through to /sso/session.
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('keeps the session on a network error from /users/me', async () => {
    seedStoredSession();
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = urlOf(input);
      if (url.includes('/users/me')) {
        throw new TypeError('network down');
      }
      throw new Error(`unexpected fetch to ${url}`);
    });
    vi.stubGlobal('fetch', fetchMock);

    render(
      <AuthProvider>
        <Probe />
      </AuthProvider>
    );

    await waitFor(() => expect(screen.getByTestId('loading').textContent).toBe('no'));

    expect(screen.getByTestId('auth').textContent).toBe('yes');
    expect(screen.getByTestId('email').textContent).toBe('stored@example.com');
    expect(window.localStorage.getItem(TOKEN_KEY)).toBe('stored-bearer-token');
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('logs the user out on a genuine 401 from /users/me', async () => {
    seedStoredSession();
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = urlOf(input);
      if (url.includes('/users/me')) {
        return new Response('unauthorized', { status: 401 });
      }
      // Cookie probe also unauthenticated.
      if (url.includes('/sso/session')) {
        return new Response('unauthorized', { status: 401 });
      }
      throw new Error(`unexpected fetch to ${url}`);
    });
    vi.stubGlobal('fetch', fetchMock);

    render(
      <AuthProvider>
        <Probe />
      </AuthProvider>
    );

    await waitFor(() => expect(screen.getByTestId('auth').textContent).toBe('no'));

    expect(screen.getByTestId('email').textContent).toBe('-');
    // Stale token cleared.
    expect(window.localStorage.getItem(TOKEN_KEY)).toBeNull();
  });
});
