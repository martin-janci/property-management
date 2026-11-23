/**
 * Unit tests for the mobile app's core fetch/auth module.
 *
 * `useApi.ts` owns tenant-scoping for *every* api-server request: it decodes
 * the JWT client-side (via the shared `utils/jwt` module) to pull the
 * `tenant_id` claim and injects it as the `X-Tenant-ID` header that the
 * server's RLS extractor requires. These tests pin the module-specific
 * behavior:
 *
 * - `getTenantId`: reads the token from SecureStore and extracts the claim,
 *   yielding `null` (so no header is injected) when the token/claim is missing.
 *   The exhaustive base64url decode edge-cases live with the shared unit in
 *   `utils/jwt.test.ts`; here we only cover the SecureStore integration.
 * - `apiRequest`: composes `Authorization` / `X-Tenant-ID` headers, prefixes
 *   relative paths with the API base URL, surfaces server error messages,
 *   handles 204, and stays unauthenticated when the keystore read throws.
 * - `useTenantId`: resolves the claim reactively and re-derives it after the
 *   query cache is cleared (the logout/login cross-tenant swap of issue #2329).
 *
 * `expo-secure-store` and `expo-constants` are mocked globally in
 * `src/test/setup.ts`; `config/api` and `fetch` are stubbed here.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react-native';
import * as SecureStore from 'expo-secure-store';
import { createElement, type ReactNode } from 'react';
import { makeJwt } from '../test/jwt';
import { apiRequest, getTenantId, useTenantId } from './useApi';

// Fixed base URL so the relative-path composition is deterministic (the real
// resolver is env/platform-dependent).
jest.mock('../config/api', () => ({
  getApiBaseUrl: () => 'https://api.mocked',
}));

const mockedGetItem = SecureStore.getItemAsync as jest.Mock;
const mockedSetItem = SecureStore.setItemAsync as jest.Mock;

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';

/** Prime the SecureStore mock so `getAccessToken()` returns `token`. */
function primeToken(token: string | null): void {
  mockedGetItem.mockImplementation(async (key: string) =>
    key === ACCESS_TOKEN_KEY ? token : null
  );
}

beforeEach(() => {
  jest.clearAllMocks();
});

describe('getTenantId (SecureStore integration)', () => {
  it('extracts the tenant_id claim from the stored token', async () => {
    primeToken(makeJwt({ sub: 'u-1', tenant_id: 'org-42' }));
    await expect(getTenantId()).resolves.toBe('org-42');
  });

  it('returns null when the payload has no tenant_id claim', async () => {
    primeToken(makeJwt({ sub: 'u-1', role: 'owner' }));
    await expect(getTenantId()).resolves.toBeNull();
  });

  it('returns null when tenant_id is present but not a string', async () => {
    primeToken(makeJwt({ tenant_id: 12345 }));
    await expect(getTenantId()).resolves.toBeNull();
  });

  it('returns null (no throw) for a malformed token', async () => {
    primeToken('not-a-jwt');
    await expect(getTenantId()).resolves.toBeNull();
  });

  it('returns null when no token is stored', async () => {
    primeToken(null);
    await expect(getTenantId()).resolves.toBeNull();
  });

  it('returns null (no auth) when the keystore read throws', async () => {
    mockedGetItem.mockRejectedValue(new Error('keystore locked'));
    await expect(getTenantId()).resolves.toBeNull();
  });
});

describe('apiRequest', () => {
  let fetchMock: jest.Mock;
  let originalFetch: typeof globalThis.fetch;

  /** Build a minimal `Response`-like object for the fetch mock. */
  function fakeResponse(opts: { status?: number; ok?: boolean; json?: () => Promise<unknown> }) {
    const status = opts.status ?? 200;
    return {
      ok: opts.ok ?? (status >= 200 && status < 300),
      status,
      json: opts.json ?? (async () => ({})),
    };
  }

  beforeEach(() => {
    originalFetch = globalThis.fetch;
    fetchMock = jest.fn();
    globalThis.fetch = fetchMock as unknown as typeof fetch;
  });

  afterEach(() => {
    // Restore the original fetch so the stub can't leak across files (harmless
    // under per-file jest isolation, but keeps the suite future-proof).
    globalThis.fetch = originalFetch;
  });

  /** Return the headers object fetch was called with on its first call. */
  function firstCallHeaders(): Record<string, string> {
    const [, init] = fetchMock.mock.calls[0];
    return init.headers as Record<string, string>;
  }

  it('injects Authorization and X-Tenant-ID headers from the token', async () => {
    primeToken(makeJwt({ sub: 'u-1', tenant_id: 'org-42' }));
    fetchMock.mockResolvedValueOnce(fakeResponse({ json: async () => ({ ok: true }) }));

    const result = await apiRequest<{ ok: boolean }>('https://api.test/v1/outages');

    expect(result).toEqual({ ok: true });
    const headers = firstCallHeaders();
    expect(headers.Authorization).toMatch(/^Bearer header\./);
    expect(headers['X-Tenant-ID']).toBe('org-42');
    expect(headers.Accept).toBe('application/json');
  });

  it('omits X-Tenant-ID when the token has no tenant_id claim', async () => {
    primeToken(makeJwt({ sub: 'u-1' }));
    fetchMock.mockResolvedValueOnce(fakeResponse({ json: async () => ({}) }));

    await apiRequest('https://api.test/v1/me');

    const headers = firstCallHeaders();
    expect(headers.Authorization).toBeDefined();
    expect(headers).not.toHaveProperty('X-Tenant-ID');
  });

  it('proceeds unauthenticated (no Authorization / X-Tenant-ID) when the keystore read throws', async () => {
    // Pins the current design: a SecureStore failure is swallowed and the
    // request goes out anonymous rather than failing (issue #2329, finding 3).
    mockedGetItem.mockRejectedValue(new Error('keystore locked'));
    fetchMock.mockResolvedValueOnce(fakeResponse({ json: async () => ({}) }));

    await apiRequest('https://api.test/v1/me');

    const headers = firstCallHeaders();
    expect(headers).not.toHaveProperty('Authorization');
    expect(headers).not.toHaveProperty('X-Tenant-ID');
  });

  it('prefixes a relative path with the configured API base URL', async () => {
    primeToken(makeJwt({ tenant_id: 'org-42' }));
    fetchMock.mockResolvedValueOnce(fakeResponse({ json: async () => ({}) }));

    await apiRequest('/api/v1/buildings');

    const [url] = fetchMock.mock.calls[0];
    expect(url).toBe('https://api.mocked/api/v1/buildings');
  });

  it('leaves an absolute URL untouched', async () => {
    primeToken(makeJwt({ tenant_id: 'org-42' }));
    fetchMock.mockResolvedValueOnce(fakeResponse({ json: async () => ({}) }));

    await apiRequest('https://api.test/v1/outages');

    const [url] = fetchMock.mock.calls[0];
    expect(url).toBe('https://api.test/v1/outages');
  });

  it('sends the stored token as-is without a client-side exp check', async () => {
    // The module does no `exp` check: refresh is driven by a *server* 401, not
    // by inspecting the token locally. On a 2xx the stored token flows through
    // untouched even if its `exp` is in the past (the 401→refresh→retry path is
    // covered in the "401 token refresh" suite below).
    const expired = makeJwt({ tenant_id: 'org-42', exp: 1 });
    primeToken(expired);
    fetchMock.mockResolvedValueOnce(fakeResponse({ json: async () => ({}) }));

    await apiRequest('https://api.test/v1/me');

    const headers = firstCallHeaders();
    expect(headers.Authorization).toBe(`Bearer ${expired}`);
    expect(headers['X-Tenant-ID']).toBe('org-42');
  });

  it('sets Content-Type and serializes the body for a mutation', async () => {
    primeToken(makeJwt({ tenant_id: 'org-42' }));
    fetchMock.mockResolvedValueOnce(fakeResponse({ json: async () => ({ id: 1 }) }));

    await apiRequest('https://api.test/v1/faults', {
      method: 'POST',
      body: { title: 'Leak' },
    });

    const [, init] = fetchMock.mock.calls[0];
    expect((init.headers as Record<string, string>)['Content-Type']).toBe('application/json');
    expect(init.body).toBe(JSON.stringify({ title: 'Leak' }));
  });

  it('surfaces the server error message from a JSON error body', async () => {
    primeToken(makeJwt({ tenant_id: 'org-42' }));
    fetchMock.mockResolvedValueOnce(
      fakeResponse({
        status: 403,
        ok: false,
        json: async () => ({ message: 'Forbidden for tenant' }),
      })
    );

    await expect(apiRequest('https://api.test/v1/buildings')).rejects.toThrow(
      'Forbidden for tenant'
    );
  });

  it('falls back to the HTTP status when the error body is not JSON', async () => {
    primeToken(makeJwt({ tenant_id: 'org-42' }));
    fetchMock.mockResolvedValueOnce(
      fakeResponse({
        status: 500,
        ok: false,
        json: async () => {
          throw new Error('Unexpected token < in JSON');
        },
      })
    );

    await expect(apiRequest('https://api.test/v1/buildings')).rejects.toThrow('HTTP 500');
  });

  it('returns undefined for a 204 No-Content response without parsing a body', async () => {
    primeToken(makeJwt({ tenant_id: 'org-42' }));
    const json = jest.fn();
    fetchMock.mockResolvedValueOnce(fakeResponse({ status: 204, json }));

    const result = await apiRequest('https://api.test/v1/faults/1', { method: 'DELETE' });

    expect(result).toBeUndefined();
    expect(json).not.toHaveBeenCalled();
  });
});

describe('apiRequest — 401 token refresh', () => {
  let fetchMock: jest.Mock;
  let originalFetch: typeof globalThis.fetch;

  function fakeResponse(opts: { status?: number; ok?: boolean; json?: () => Promise<unknown> }) {
    const status = opts.status ?? 200;
    return {
      ok: opts.ok ?? (status >= 200 && status < 300),
      status,
      json: opts.json ?? (async () => ({})),
    };
  }

  /**
   * Back the SecureStore mock with mutable in-memory token slots so a refresh
   * that writes new tokens is reflected by the next `getItemAsync` read (the
   * retried request must pick up the fresh bearer).
   */
  function primeTokenStore(initial: { access: string | null; refresh: string | null }): {
    access: string | null;
    refresh: string | null;
  } {
    const store = { ...initial };
    mockedGetItem.mockImplementation(async (key: string) => {
      if (key === ACCESS_TOKEN_KEY) return store.access;
      if (key === REFRESH_TOKEN_KEY) return store.refresh;
      return null;
    });
    mockedSetItem.mockImplementation(async (key: string, value: string) => {
      if (key === ACCESS_TOKEN_KEY) store.access = value;
      if (key === REFRESH_TOKEN_KEY) store.refresh = value;
    });
    return store;
  }

  beforeEach(() => {
    originalFetch = globalThis.fetch;
    fetchMock = jest.fn();
    globalThis.fetch = fetchMock as unknown as typeof fetch;
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('refreshes the token and retries once on a 401, then succeeds with the fresh bearer', async () => {
    const freshAccess = makeJwt({ sub: 'u-1', tenant_id: 'org-42' });
    primeTokenStore({
      access: makeJwt({ sub: 'u-1', tenant_id: 'org-42', exp: 1 }),
      refresh: 'refresh-abc',
    });
    fetchMock
      .mockResolvedValueOnce(
        fakeResponse({ status: 401, ok: false, json: async () => ({ message: 'token expired' }) })
      )
      .mockResolvedValueOnce(
        fakeResponse({
          json: async () => ({ accessToken: freshAccess, refreshToken: 'refresh-def' }),
        })
      )
      .mockResolvedValueOnce(fakeResponse({ json: async () => ({ ok: true }) }));

    const result = await apiRequest<{ ok: boolean }>('/api/v1/outages');

    expect(result).toEqual({ ok: true });
    expect(fetchMock).toHaveBeenCalledTimes(3);
    // The refresh hit the refresh endpoint with the stored refresh token.
    expect(fetchMock.mock.calls[1][0]).toBe('https://api.mocked/api/v1/auth/refresh');
    expect(JSON.parse(fetchMock.mock.calls[1][1].body)).toEqual({ refreshToken: 'refresh-abc' });
    // The rotated tokens were persisted...
    expect(mockedSetItem).toHaveBeenCalledWith(ACCESS_TOKEN_KEY, freshAccess);
    expect(mockedSetItem).toHaveBeenCalledWith(REFRESH_TOKEN_KEY, 'refresh-def');
    // ...and the retried request carried the fresh bearer.
    const retryHeaders = fetchMock.mock.calls[2][1].headers as Record<string, string>;
    expect(retryHeaders.Authorization).toBe(`Bearer ${freshAccess}`);
  });

  it('surfaces the 401 without retrying when no refresh token is stored', async () => {
    primeTokenStore({ access: makeJwt({ tenant_id: 'org-42' }), refresh: null });
    fetchMock.mockResolvedValueOnce(
      fakeResponse({ status: 401, ok: false, json: async () => ({ message: 'Unauthorized' }) })
    );

    await expect(apiRequest('/api/v1/me')).rejects.toThrow('Unauthorized');
    // One request, no refresh attempt (nothing to refresh with).
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('surfaces the original 401 when the refresh endpoint itself fails', async () => {
    primeTokenStore({ access: makeJwt({ tenant_id: 'org-42' }), refresh: 'refresh-abc' });
    fetchMock
      .mockResolvedValueOnce(
        fakeResponse({ status: 401, ok: false, json: async () => ({ message: 'token expired' }) })
      )
      .mockResolvedValueOnce(
        fakeResponse({
          status: 401,
          ok: false,
          json: async () => ({ message: 'refresh rejected' }),
        })
      );

    await expect(apiRequest('/api/v1/me')).rejects.toThrow('token expired');
    // Resource + refresh, then it gives up (no infinite retry).
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it('retries at most once — a second 401 after a successful refresh is surfaced', async () => {
    primeTokenStore({ access: makeJwt({ tenant_id: 'org-42' }), refresh: 'refresh-abc' });
    fetchMock
      .mockResolvedValueOnce(
        fakeResponse({ status: 401, ok: false, json: async () => ({ message: 'token expired' }) })
      )
      .mockResolvedValueOnce(
        fakeResponse({
          json: async () => ({ accessToken: makeJwt({ tenant_id: 'org-42' }), refreshToken: 'r2' }),
        })
      )
      .mockResolvedValueOnce(
        fakeResponse({
          status: 401,
          ok: false,
          json: async () => ({ message: 'still unauthorized' }),
        })
      );

    await expect(apiRequest('/api/v1/me')).rejects.toThrow('still unauthorized');
    expect(fetchMock).toHaveBeenCalledTimes(3);
  });
});

describe('useTenantId', () => {
  function makeWrapper(client: QueryClient) {
    return ({ children }: { children: ReactNode }) =>
      createElement(QueryClientProvider, { client }, children);
  }

  function freshClient(): QueryClient {
    return new QueryClient({ defaultOptions: { queries: { retry: false } } });
  }

  it('resolves the tenant id from the stored token', async () => {
    primeToken(makeJwt({ tenant_id: 'org-A' }));
    const { result } = await renderHook(() => useTenantId(), {
      wrapper: makeWrapper(freshClient()),
    });

    await waitFor(() => expect(result.current.tenantId).toBe('org-A'));
  });

  it('derives the new tenant from a fresh (post-logout) cache after a token swap', async () => {
    // The issue #2329 fix: AuthContext.logout() clears the query cache (pinned
    // in the auth integration suite), so a subsequent login as a user in a
    // *different* org resolves the new tenant instead of the `staleTime:
    // Infinity` cached value. Modeled here as a fresh client + swapped token —
    // the empty post-logout cache no longer holds the previous org's id.
    primeToken(makeJwt({ tenant_id: 'org-B' }));
    const { result } = await renderHook(() => useTenantId(), {
      wrapper: makeWrapper(freshClient()),
    });

    await waitFor(() => expect(result.current.tenantId).toBe('org-B'));
  });
});
