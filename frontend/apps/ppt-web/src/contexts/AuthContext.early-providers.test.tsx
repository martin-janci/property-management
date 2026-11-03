/**
 * Regression: the api-client token / org providers must be registered before
 * any child's passive effect runs. React runs passive effects child-first and
 * TanStack Query fetches from one, so registering in AuthProvider's
 * `useEffect` let the first request after a hard reload go out without
 * `Authorization` / `X-Tenant-ID` (401 until the query retried).
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

import { bearerAuthorization, getOrg } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, waitFor } from '@testing-library/react';
import { useEffect } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { AuthProvider, useAuth } from './AuthContext';

const ACCESS_TOKEN = 'access-token-early';

/** Captures the auth headers visible from its *first* passive effect. */
function EarlyFetcher({ seen }: { seen: Array<{ auth: string | null; org: string | null }> }) {
  const { user } = useAuth();
  useEffect(() => {
    if (user) seen.push({ auth: bearerAuthorization() ?? null, org: getOrg() ?? null });
  }, [user]);
  return null;
}

describe('AuthContext — providers are ready for the first child request', () => {
  afterEach(() => {
    localStorage.clear();
  });

  it('exposes token and org to a child effect on the restoring render', async () => {
    localStorage.setItem('ppt_access_token', ACCESS_TOKEN);
    localStorage.setItem(
      'ppt_user',
      JSON.stringify({ id: 'u-1', email: 'a@example.com', organizationId: 'org-1' })
    );
    const seen: Array<{ auth: string | null; org: string | null }> = [];
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });

    render(
      <QueryClientProvider client={queryClient}>
        <AuthProvider>
          <EarlyFetcher seen={seen} />
        </AuthProvider>
      </QueryClientProvider>
    );

    await waitFor(() => expect(seen.length).toBeGreaterThan(0));
    expect(seen[0]).toEqual({ auth: `Bearer ${ACCESS_TOKEN}`, org: 'org-1' });
  });
});
