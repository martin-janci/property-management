/// <reference types="vitest/globals" />
/**
 * Token-freshness regression tests for the neighbors hooks
 * (code-review-ppt-web-core-neighbors-stale-token-usememo).
 *
 * `createNeighborsApi` bakes the `Authorization: Bearer <token>` header into
 * the returned client at construction time, so the client MUST be rebuilt
 * whenever the auth token rotates. `useNeighbors` / `usePrivacySettings`
 * previously built the client inside `useMemo(..., [])` — an empty dependency
 * array — which captured the mount-time token forever. After a refresh the
 * hooks kept sending the STALE Bearer for the rest of the component lifetime.
 *
 * These tests pin the fix: the client is keyed on the token read at render
 * time, so a rotated token reaches the wire on the next request. Run against
 * the buggy (empty-dep) version they fail — the second request carries the old
 * `token-A` instead of the refreshed `token-B`.
 */

import { setTokenProvider } from '@ppt/api-client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import type { ReactNode } from 'react';
import { DEFAULT_PRIVACY_SETTINGS } from '../types';
import { useNeighbors, usePrivacySettings } from './useNeighbors';

const mockFetch = vi.fn();
global.fetch = mockFetch;

/** A generic 2xx JSON response with just enough body for the mappers. */
function okJson(body: unknown): Response {
  return {
    ok: true,
    status: 200,
    json: async () => body,
  } as unknown as Response;
}

/** Read the Authorization header off the fetch call whose URL matches. */
function authHeaderFor(urlFragment: string): string | undefined {
  const call = [...mockFetch.mock.calls]
    .reverse()
    .find(([url]) => typeof url === 'string' && url.includes(urlFragment));
  const headers = (call?.[1] as RequestInit | undefined)?.headers as
    | Record<string, string>
    | undefined;
  return headers?.Authorization;
}

function setup() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  return { wrapper };
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.resetAllMocks();
});

describe('neighbors hooks use the current auth token after a refresh', () => {
  it('useNeighbors sends the refreshed Bearer token on the next request', async () => {
    let token = 'token-A';
    setTokenProvider(() => token);
    mockFetch.mockResolvedValue(okJson({ neighbors: [], total: 0 }));

    const { wrapper } = setup();
    const { rerender } = renderHook(({ buildingId }) => useNeighbors(buildingId), {
      initialProps: { buildingId: 'building-1' },
      wrapper,
    });

    await waitFor(() => expect(authHeaderFor('/buildings/building-1/')).toBeDefined());
    expect(authHeaderFor('/buildings/building-1/')).toBe('Bearer token-A');

    // Token rotates (e.g. a refresh-token exchange) and the component re-renders.
    token = 'token-B';
    rerender({ buildingId: 'building-2' });

    await waitFor(() => expect(authHeaderFor('/buildings/building-2/')).toBeDefined());
    expect(authHeaderFor('/buildings/building-2/')).toBe('Bearer token-B');
  });

  it('usePrivacySettings mutates with the refreshed Bearer token, not the stale one', async () => {
    let token = 'token-A';
    setTokenProvider(() => token);
    mockFetch.mockResolvedValue(
      okJson({ settings: { profileVisibility: 'visible', showContactInfo: true } })
    );

    const { wrapper } = setup();
    const { result, rerender } = renderHook(() => usePrivacySettings(), { wrapper });

    // Initial GET carries the mount-time token.
    await waitFor(() => expect(authHeaderFor('/users/me/privacy')).toBe('Bearer token-A'));

    // Token rotates and the component re-renders.
    token = 'token-B';
    rerender();

    await result.current.updateSettings(DEFAULT_PRIVACY_SETTINGS);

    await waitFor(() => {
      const putCall = [...mockFetch.mock.calls]
        .reverse()
        .find(([, init]) => (init as RequestInit | undefined)?.method === 'PUT');
      expect(putCall).toBeDefined();
      const headers = (putCall as unknown[])[1] as RequestInit;
      const authHeader = (headers.headers as Record<string, string>).Authorization;
      expect(authHeader).toBe('Bearer token-B');
    });
  });
});
