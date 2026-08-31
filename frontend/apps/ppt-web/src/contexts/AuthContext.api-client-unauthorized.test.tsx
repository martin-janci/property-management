/**
 * Regression: AuthProvider must wire the `onUnauthorized` callback into the
 * hand-rolled axios client (lib/api.ts) so a 401 from getApiClient() actually
 * triggers session recovery / teardown.
 *
 * Before this fix, configureApiClient() was called with only `getToken` —
 * `onUnauthorized` was never passed. The response interceptor's
 * `if (401 && onUnauthorizedCallback)` branch was therefore dead, so every
 * getApiClient() consumer (sentiment, predictive-maintenance, …) stayed stuck
 * on an expired access token with no logout / redirect and no refresh attempt.
 *
 * This test seeds an authenticated session that has NO refresh token (so the
 * recovery path is a straight logout with no network), renders <AuthProvider>,
 * then makes a getApiClient() request return 401. It asserts the session is
 * torn down (isAuthenticated → false). It fails on the unfixed code (the 401
 * is inert and the session stays authed) and passes once AuthProvider wires
 * onUnauthorized.
 */
/// <reference types="vitest/globals" />

// The in-memory `localStorage` polyfill (jsdom under vitest omits it) and the
// per-test storage cleanup are provided globally by `src/test/setup.ts`
// (wired via `setupFiles` in vitest.config.ts), so this file no longer needs
// its own shim — it just reads/writes `localStorage` directly.

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, waitFor } from '@testing-library/react';
import type { AxiosAdapter, AxiosResponse } from 'axios';
import { AxiosError } from 'axios';
import type React from 'react';
import { useEffect } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { getApiClient, resetApiClient } from '../lib/api';
import { AuthProvider, useAuth } from './AuthContext';

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const USER_KEY = 'ppt_user';
const ACCESS_TOKEN = 'expired-access-token';

/** Seed an authed session that has NO refresh token → recovery is a logout. */
function seedAuthedSessionWithoutRefresh() {
  localStorage.setItem(ACCESS_TOKEN_KEY, ACCESS_TOKEN);
  localStorage.setItem(
    USER_KEY,
    JSON.stringify({ id: 'u-1', email: 'alice@example.com', name: 'Alice' })
  );
}

/** Adapter that always responds 401 (token expired / revoked). */
function unauthorizedAdapter(): AxiosAdapter {
  return (config) => {
    const response: AxiosResponse = {
      data: { error: 'UNAUTHORIZED', message: 'expired' },
      status: 401,
      statusText: 'Unauthorized',
      headers: {},
      config,
    };
    return Promise.reject(
      new AxiosError('Unauthorized', 'ERR_BAD_REQUEST', config, null, response)
    );
  };
}

function AuthProbe({
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

describe('AuthContext — wires getApiClient() onUnauthorized (401) handler', () => {
  beforeEach(() => {
    resetApiClient();
    seedAuthedSessionWithoutRefresh();
  });

  afterEach(() => {
    resetApiClient();
    localStorage.clear();
  });

  it('tears down the session when a getApiClient() request returns 401', async () => {
    const ctxRef: { current: ReturnType<typeof useAuth> | null } = { current: null };
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false, gcTime: 0 } },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <AuthProvider>
          <AuthProbe ctxRef={ctxRef} />
        </AuthProvider>
      </QueryClientProvider>
    );

    // Wait for AuthProvider to bootstrap the authed state (and run the effect
    // that calls configureApiClient with onUnauthorized wired).
    await waitFor(() => {
      expect(ctxRef.current?.isAuthenticated).toBe(true);
    });

    // Install the 401 adapter on the *configured* instance and issue a request
    // the way feature hooks do.
    const instance = getApiClient();
    instance.defaults.adapter = unauthorizedAdapter();

    await expect(instance.get('/widgets')).rejects.toMatchObject({ status: 401 });

    // Core assertion: the wired onUnauthorized handler ran and, with no refresh
    // token to recover with, logged the user out. On the unfixed code the 401
    // branch was dead and the session would stay authed.
    await waitFor(() => {
      expect(ctxRef.current?.isAuthenticated).toBe(false);
    });
    expect(localStorage.getItem(ACCESS_TOKEN_KEY)).toBeNull();
  });
});
