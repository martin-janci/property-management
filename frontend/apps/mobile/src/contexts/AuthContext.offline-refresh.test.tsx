/**
 * AuthContext — offline token-refresh regression test
 * (code-review-mobile-rn-refresh-offline-logout).
 *
 * `refreshToken()` used to wrap the whole `/auth/refresh` round-trip in a single
 * try/catch whose catch unconditionally called `logout()`. When the device is
 * offline, `fetch` rejects with a network error — which was caught and treated
 * exactly like a server rejection, so a transient offline blip force-logged the
 * user out AND wiped their offline data (`resetLocalData()` purges the offline
 * mutation queue, cached responses, NFC building-access credentials and widget
 * data). On a cold start, opening the app offline with an expired access token
 * destroyed an otherwise-valid session irrecoverably.
 *
 * The fix distinguishes a transport failure (offline — session preserved) from a
 * server rejection (refresh token genuinely dead — logout). This suite locks
 * that contract and fails before the fix (IG3):
 *   1. an offline refresh must NOT delete the tokens (no logout);
 *   2. a 401 from the server MUST still log out;
 *   3. an offline cold start with an expired token keeps the user signed in.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react-native';
import * as SecureStore from 'expo-secure-store';
import type { ReactNode } from 'react';

import * as jwt from '../utils/jwt';
import { AuthProvider, useAuth } from './AuthContext';

// Biometric probing in initialize() must resolve deterministically.
jest.mock('expo-local-authentication', () => ({
  hasHardwareAsync: jest.fn(async () => false),
  isEnrolledAsync: jest.fn(async () => false),
  authenticateAsync: jest.fn(async () => ({ success: false })),
}));

// Controllable per test. Default false so the mount-time initialize() restores
// the session WITHOUT a network refresh; tests that exercise the cold-start
// refresh path flip it to true.
jest.mock('../utils/jwt', () => ({ isJwtExpired: jest.fn(() => false) }));

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
const USER_KEY = 'ppt_user';

const getItemAsync = jest.mocked(SecureStore.getItemAsync);
const setItemAsync = jest.mocked(SecureStore.setItemAsync);
const deleteItemAsync = jest.mocked(SecureStore.deleteItemAsync);
const isJwtExpired = jest.mocked(jwt.isJwtExpired);

const storedUser = {
  id: 'u1',
  email: 'owner@example.com',
  firstName: 'Olivia',
  lastName: 'Owner',
  role: 'owner',
};

function wrapper({ children }: { children: ReactNode }) {
  const client = new QueryClient();
  return (
    <QueryClientProvider client={client}>
      <AuthProvider apiBaseUrl="https://api.test.local">{children}</AuthProvider>
    </QueryClientProvider>
  );
}

let originalFetch: typeof globalThis.fetch;

beforeEach(() => {
  originalFetch = globalThis.fetch;
  getItemAsync.mockReset();
  setItemAsync.mockReset();
  deleteItemAsync.mockReset();
  isJwtExpired.mockReset();
  isJwtExpired.mockReturnValue(false);

  getItemAsync.mockImplementation(async (key: string) => {
    if (key === ACCESS_TOKEN_KEY) return 'restored-access-token';
    if (key === REFRESH_TOKEN_KEY) return 'stored-refresh-token';
    if (key === USER_KEY) return JSON.stringify(storedUser);
    return null;
  });
  setItemAsync.mockResolvedValue(undefined);
  deleteItemAsync.mockResolvedValue(undefined);
});

afterEach(() => {
  // Restore the real fetch so the stub can't leak across test files.
  globalThis.fetch = originalFetch;
});

describe('AuthContext → offline token refresh', () => {
  it('does NOT log the user out when the refresh request fails because the device is offline', async () => {
    // `fetch` rejecting (rather than returning a non-OK Response) is exactly how
    // an offline / unreachable-server request manifests in React Native.
    const fetchMock = jest.fn().mockRejectedValue(new TypeError('Network request failed'));
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    const { result } = await renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await result.current.refreshToken().catch(() => {});
    });

    expect(fetchMock).toHaveBeenCalled();
    // logout() — the only thing that deletes the access token — must NOT run on
    // a transport failure. A still-valid session (and the offline cache that
    // resetLocalData would wipe) has to survive an offline refresh.
    expect(deleteItemAsync).not.toHaveBeenCalledWith(ACCESS_TOKEN_KEY);
  });

  it('DOES log the user out when the server rejects the refresh token (401)', async () => {
    const fetchMock = jest.fn().mockResolvedValue({ ok: false, status: 401 });
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    const { result } = await renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    await act(async () => {
      await result.current.refreshToken().catch(() => {});
    });

    // A genuine server rejection means the refresh token is dead — the session
    // must be torn down (logout deletes the access token).
    expect(deleteItemAsync).toHaveBeenCalledWith(ACCESS_TOKEN_KEY);
  });

  it('keeps a restored session usable offline when the access token is expired but the refresh is unreachable', async () => {
    isJwtExpired.mockReturnValue(true);
    const fetchMock = jest.fn().mockRejectedValue(new TypeError('Network request failed'));
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    const { result } = await renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    // Offline cold start with an expired access token must NOT evict the user:
    // the refresh token may still be valid, so keep them on the stale token +
    // offline cache instead of logging out and wiping their data.
    expect(result.current.isAuthenticated).toBe(true);
    expect(deleteItemAsync).not.toHaveBeenCalledWith(ACCESS_TOKEN_KEY);
  });

  it('restores the session on the freshly rotated token when the stored one is expired and the server is reachable', async () => {
    // Expired cold start + a reachable server: initialize() must refresh the
    // token and restore the session on the ROTATED bearer read back from
    // SecureStore (not the stale stored one). A stateful store makes the
    // post-refresh read-back return the rotated token.
    isJwtExpired.mockReturnValue(true);

    const store: Record<string, string | null> = {
      [ACCESS_TOKEN_KEY]: 'stale-access-token',
      [REFRESH_TOKEN_KEY]: 'stored-refresh-token',
      [USER_KEY]: JSON.stringify(storedUser),
    };
    getItemAsync.mockImplementation(async (key: string) => store[key] ?? null);
    setItemAsync.mockImplementation(async (key: string, value: string) => {
      store[key] = value;
    });

    const fetchMock = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        accessToken: 'rotated-access-token',
        refreshToken: 'rotated-refresh-token',
      }),
    });
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    const { result } = await renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    expect(result.current.isAuthenticated).toBe(true);
    expect(result.current.accessToken).toBe('rotated-access-token');
    // The rotated pair is persisted, and no logout happened.
    expect(setItemAsync).toHaveBeenCalledWith(ACCESS_TOKEN_KEY, 'rotated-access-token');
    expect(setItemAsync).toHaveBeenCalledWith(REFRESH_TOKEN_KEY, 'rotated-refresh-token');
    expect(deleteItemAsync).not.toHaveBeenCalledWith(ACCESS_TOKEN_KEY);
  });
});
