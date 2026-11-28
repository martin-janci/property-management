/**
 * AuthContext — biometric-unlock token-refresh behaviour.
 *
 * `authenticateWithBiometric()` restores the SAME stored user's session after a
 * successful biometric prompt. When the device has sat locked past the
 * access-token TTL the stored bearer can be expired, so the unlock path refreshes
 * it first — sharing the exact same expired-token contract as the cold-start
 * `initialize()` path (both now route through the private `ensureFreshAccessToken`
 * helper):
 *   1. offline refresh → keep the session on the stale token, unlock succeeds,
 *      NO logout (issue #2399);
 *   2. server rejection (401) → the session is dead, unlock fails (and the
 *      refresh-driven logout clears the access token);
 *   3. reachable server → restore on the freshly rotated token.
 *
 * This suite pins that second call site so the shared-helper extraction can't
 * silently change biometric-unlock behaviour.
 */
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react-native';
import * as SecureStore from 'expo-secure-store';
import type { ReactNode } from 'react';

import * as jwt from '../utils/jwt';
import { AuthProvider, useAuth } from './AuthContext';

// Biometric hardware present + enrolled, and the prompt succeeds, so
// authenticateWithBiometric() reaches the token-refresh step.
jest.mock('expo-local-authentication', () => ({
  hasHardwareAsync: jest.fn(async () => true),
  isEnrolledAsync: jest.fn(async () => true),
  authenticateAsync: jest.fn(async () => ({ success: true })),
}));

// Controllable per test. Default false so the mount-time initialize() restores
// the session WITHOUT a network refresh; each test flips it to true right before
// invoking the biometric unlock so only the unlock path exercises the refresh.
jest.mock('../utils/jwt', () => ({ isJwtExpired: jest.fn(() => false) }));

const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
const USER_KEY = 'ppt_user';
const BIOMETRIC_ENABLED_KEY = 'ppt_biometric_enabled';

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
    if (key === BIOMETRIC_ENABLED_KEY) return 'true';
    return null;
  });
  setItemAsync.mockResolvedValue(undefined);
  deleteItemAsync.mockResolvedValue(undefined);
});

afterEach(() => {
  globalThis.fetch = originalFetch;
});

describe('AuthContext → biometric unlock token refresh', () => {
  it('unlocks on the stale token (no logout) when the refresh is unreachable (offline)', async () => {
    const { result } = await renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.biometricEnabled).toBe(true);
    expect(result.current.biometricAvailable).toBe(true);

    // Now the stored access token is expired and the device is offline.
    isJwtExpired.mockReturnValue(true);
    const fetchMock = jest.fn().mockRejectedValue(new TypeError('Network request failed'));
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    let unlocked: boolean | undefined;
    await act(async () => {
      unlocked = await result.current.authenticateWithBiometric();
    });

    expect(unlocked).toBe(true);
    expect(result.current.isAuthenticated).toBe(true);
    // Offline refresh must keep the stale token and must NOT log out.
    expect(result.current.accessToken).toBe('restored-access-token');
    expect(deleteItemAsync).not.toHaveBeenCalledWith(ACCESS_TOKEN_KEY);
  });

  it('fails the unlock and tears the session down when the server rejects the refresh token (401)', async () => {
    const { result } = await renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    isJwtExpired.mockReturnValue(true);
    const fetchMock = jest.fn().mockResolvedValue({ ok: false, status: 401 });
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    let unlocked: boolean | undefined;
    await act(async () => {
      unlocked = await result.current.authenticateWithBiometric();
    });

    // A genuine server rejection means the session is dead: unlock fails and the
    // refresh-driven logout() deletes the access token.
    expect(unlocked).toBe(false);
    expect(deleteItemAsync).toHaveBeenCalledWith(ACCESS_TOKEN_KEY);
  });

  it('restores the session on the freshly rotated token when the server is reachable', async () => {
    const store: Record<string, string | null> = {
      [ACCESS_TOKEN_KEY]: 'stale-access-token',
      [REFRESH_TOKEN_KEY]: 'stored-refresh-token',
      [USER_KEY]: JSON.stringify(storedUser),
      [BIOMETRIC_ENABLED_KEY]: 'true',
    };
    getItemAsync.mockImplementation(async (key: string) => store[key] ?? null);
    setItemAsync.mockImplementation(async (key: string, value: string) => {
      store[key] = value;
    });

    const { result } = await renderHook(() => useAuth(), { wrapper });
    await waitFor(() => expect(result.current.isLoading).toBe(false));

    isJwtExpired.mockReturnValue(true);
    const fetchMock = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        accessToken: 'rotated-access-token',
        refreshToken: 'rotated-refresh-token',
      }),
    });
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    let unlocked: boolean | undefined;
    await act(async () => {
      unlocked = await result.current.authenticateWithBiometric();
    });

    expect(unlocked).toBe(true);
    expect(result.current.isAuthenticated).toBe(true);
    expect(result.current.accessToken).toBe('rotated-access-token');
    expect(deleteItemAsync).not.toHaveBeenCalledWith(ACCESS_TOKEN_KEY);
  });
});
