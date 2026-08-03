/**
 * Regression: AuthProvider must configure the hand-rolled axios client in
 * lib/api.ts so getApiClient() requests carry the Authorization header.
 *
 * Before this fix, configureApiClient() was never called in production code —
 * only in unit tests — so getApiClient() returned a default instance whose
 * request interceptor had no token getter. Every request made through
 * getApiClient() (sentiment, predictive-maintenance, …) therefore went out
 * with NO Authorization header, even for a fully authenticated user.
 *
 * This test renders <AuthProvider> with a seeded, authenticated session and
 * asserts that a request issued via getApiClient() is stamped with
 * `Authorization: Bearer <access-token>`. It fails on the unfixed code
 * (header undefined) and passes once AuthProvider wires configureApiClient.
 */
/// <reference types="vitest/globals" />

// jsdom under vitest 4 does not expose localStorage by default (mirrors the
// polyfill in AuthContext.test.tsx). Provide an in-memory shim before any SUT
// module is imported so tokenStorage.* can read/write.
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

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, waitFor } from '@testing-library/react';
import type { AxiosAdapter, AxiosResponse } from 'axios';
import { AxiosHeaders } from 'axios';
import type React from 'react';
import { useEffect } from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { getApiClient, resetApiClient } from '../lib/api';
import { AuthProvider, useAuth } from './AuthContext';

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const USER_KEY = 'ppt_user';
const ACCESS_TOKEN = 'access-token-abc123';

function seedAuthedSession() {
  localStorage.setItem(ACCESS_TOKEN_KEY, ACCESS_TOKEN);
  localStorage.setItem(
    USER_KEY,
    JSON.stringify({ id: 'u-1', email: 'alice@example.com', name: 'Alice' })
  );
}

/** Adapter that records the Authorization header and returns 200. */
function recordingAdapter(): {
  adapter: AxiosAdapter;
  authHeaders: () => Array<string | undefined>;
} {
  const seenAuth: Array<string | undefined> = [];
  const adapter: AxiosAdapter = (config) => {
    const headers = AxiosHeaders.from(config.headers);
    seenAuth.push(headers.get('Authorization') as string | undefined);
    const response: AxiosResponse = {
      data: { ok: true },
      status: 200,
      statusText: 'OK',
      headers: {},
      config,
    };
    return Promise.resolve(response);
  };
  return { adapter, authHeaders: () => seenAuth };
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

describe('AuthContext — configures getApiClient() token provider', () => {
  beforeEach(() => {
    resetApiClient();
    seedAuthedSession();
  });

  afterEach(() => {
    resetApiClient();
    localStorage.clear();
  });

  it('sends the Authorization header on getApiClient() requests once authed', async () => {
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
    // that calls configureApiClient).
    await waitFor(() => {
      expect(ctxRef.current?.isAuthenticated).toBe(true);
    });

    // Install the recording adapter on the *configured* instance and issue a
    // request the way feature hooks do.
    const instance = getApiClient();
    const { adapter, authHeaders } = recordingAdapter();
    instance.defaults.adapter = adapter;

    const res = await instance.get('/widgets');
    expect(res.status).toBe(200);

    // The core assertion: the request carried the bearer token from storage.
    expect(authHeaders()).toHaveLength(1);
    expect(authHeaders()[0]).toBe(`Bearer ${ACCESS_TOKEN}`);
  });
});
