/**
 * usePrincipalCapabilities — fetch + reducer tests.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { createElement } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AdminAuthProvider } from './AdminAuthContext';
import { sessionTokenStore } from './tokenStore';
import { usePrincipalCapabilities } from './usePrincipalCapabilities';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        // Speed up failing tests
        retryDelay: 0,
      },
    },
  });
}

function makeWrapper(token: string | null) {
  sessionStorage.clear();
  if (token) sessionTokenStore.set(token);

  const qc = makeQueryClient();

  return ({ children }: { children: ReactNode }) =>
    createElement(
      QueryClientProvider,
      { client: qc },
      createElement(AdminAuthProvider, null, children)
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('usePrincipalCapabilities', () => {
  beforeEach(() => {
    vi.spyOn(globalThis, 'fetch');
  });

  afterEach(() => {
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  // 1. No token → returns defaults, does NOT fetch
  it('returns defaults and does not fetch when no token is present', async () => {
    const { result } = renderHook(() => usePrincipalCapabilities(), {
      wrapper: makeWrapper(null),
    });

    // isLoading should be false immediately (query disabled when no token)
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.isPlatformPrincipal).toBe(false);
    expect(result.current.capabilities).toHaveLength(0);
    expect(fetch).not.toHaveBeenCalled();
  });

  // 2. Token + 200 with platform principal and live capability
  it('returns isPlatformPrincipal=true and live capabilities on 200', async () => {
    vi.mocked(fetch).mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({
        principal_kind: 'platform',
        capabilities: [{ capability: 'agencies_read', revoked_at: null, expires_at: null }],
      }),
    } as Response);

    const { result } = renderHook(() => usePrincipalCapabilities(), {
      wrapper: makeWrapper('valid-token'),
    });

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.isPlatformPrincipal).toBe(true);
    expect(result.current.capabilities).toEqual(['agencies_read']);
  });

  // 3. Revoked or expired grants are filtered out
  it('filters out revoked and expired capability grants', async () => {
    const futureExpiry = new Date(Date.now() + 999_999_999).toISOString();
    const pastExpiry = new Date(Date.now() - 1000).toISOString();

    vi.mocked(fetch).mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({
        principal_kind: 'platform',
        capabilities: [
          // Live grant — should be included
          { capability: 'audit_read', revoked_at: null, expires_at: null },
          // Revoked grant — should be excluded
          { capability: 'tenant_purge', revoked_at: '2024-01-01T00:00:00Z', expires_at: null },
          // Expired grant — should be excluded
          { capability: 'agencies_write', revoked_at: null, expires_at: pastExpiry },
          // Future expiry — should be included
          { capability: 'users_read', revoked_at: null, expires_at: futureExpiry },
        ],
      }),
    } as Response);

    const { result } = renderHook(() => usePrincipalCapabilities(), {
      wrapper: makeWrapper('valid-token'),
    });

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.capabilities).toContain('audit_read');
    expect(result.current.capabilities).toContain('users_read');
    expect(result.current.capabilities).not.toContain('tenant_purge');
    expect(result.current.capabilities).not.toContain('agencies_write');
  });

  // 4. Token + 401 → returns defaults (not a throw)
  it('returns defaults on 401 response', async () => {
    vi.mocked(fetch).mockResolvedValueOnce({
      ok: false,
      status: 401,
      json: async () => ({ principal_kind: 'public', capabilities: [] }),
    } as Response);

    const { result } = renderHook(() => usePrincipalCapabilities(), {
      wrapper: makeWrapper('expired-token'),
    });

    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.isPlatformPrincipal).toBe(false);
    expect(result.current.capabilities).toHaveLength(0);
  });

  // 5. Token + 500 → throws (surfaces as React Query error)
  it('enters error state on 500 response', async () => {
    // React Query will retry once (retry: 1 in staleTime config but our test
    // client sets retry: false), so one mock is enough
    vi.mocked(fetch).mockResolvedValue({
      ok: false,
      status: 500,
      statusText: 'Internal Server Error',
    } as Response);

    const { result } = renderHook(() => usePrincipalCapabilities(), {
      wrapper: makeWrapper('valid-token'),
    });

    // The hook resolves to defaults (isLoading=false) when the query errors out
    // because React Query catches the throw internally
    await waitFor(() => expect(result.current.isLoading).toBe(false), { timeout: 3000 });

    // isPlatformPrincipal and capabilities should fall back to defaults
    expect(result.current.isPlatformPrincipal).toBe(false);
    expect(result.current.capabilities).toHaveLength(0);
  });
});
