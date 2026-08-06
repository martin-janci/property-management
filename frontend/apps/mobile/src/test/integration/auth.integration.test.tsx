/**
 * AuthContext integration tests.
 *
 * These tests drive the real `AuthProvider` lifecycle end-to-end with the
 * Expo SecureStore / LocalAuthentication / global fetch APIs replaced by
 * Jest mocks. They cover:
 *
 * - Boot from empty SecureStore (unauthenticated).
 * - Boot from existing tokens (auto-authenticated).
 * - login() — POST /api/v1/auth/login, persistence, state transition.
 * - login() failure — error propagated, state restored.
 * - logout() — clears SecureStore and resets state.
 * - refreshToken() — uses stored refresh token, handles failure by logging out.
 * - Biometric enable / disable / authenticateWithBiometric flows.
 *
 * The AuthProvider talks to a real (mocked) global `fetch`, so the test
 * verifies the wiring between the provider, the storage layer, and the API
 * payload contract.
 */

import AsyncStorage from '@react-native-async-storage/async-storage';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, renderHook, waitFor } from '@testing-library/react-native';
import * as LocalAuthentication from 'expo-local-authentication';
import * as SecureStore from 'expo-secure-store';
import type { ReactNode } from 'react';
import { AuthProvider, useAuth } from '../../contexts/AuthContext';
import csLocale from '../../locales/cs.json';
import deLocale from '../../locales/de.json';
import enLocale from '../../locales/en.json';
import huLocale from '../../locales/hu.json';
import plLocale from '../../locales/pl.json';
import skLocale from '../../locales/sk.json';
import { makeJwt } from '../jwt';

/** Current time in whole seconds — the unit JWT `exp` claims use. */
const nowSec = () => Math.floor(Date.now() / 1000);
/** A well-formed access token whose `exp` is already in the past. */
const expiredToken = () => makeJwt({ sub: 'u-1', exp: nowSec() - 60 });
/** A well-formed access token valid for another hour. */
const validToken = () => makeJwt({ sub: 'u-1', exp: nowSec() + 3600 });

// AuthProvider now purges the AsyncStorage-backed tenant caches on
// login/logout (issue #2399), so swap the shared jest stub for a real
// in-memory store we can seed and assert against.
jest.mock('@react-native-async-storage/async-storage', () => {
  const store = new Map<string, string>();
  return {
    __store: store,
    getItem: jest.fn(async (k: string) => store.get(k) ?? null),
    setItem: jest.fn(async (k: string, v: string) => {
      store.set(k, v);
    }),
    removeItem: jest.fn(async (k: string) => {
      store.delete(k);
    }),
    getAllKeys: jest.fn(async () => Array.from(store.keys())),
    removeMany: jest.fn(async (keys: string[]) => {
      for (const k of keys) store.delete(k);
    }),
  };
});

const asyncStore = (AsyncStorage as unknown as { __store: Map<string, string> }).__store;

const API_BASE = 'http://test.local';

// AuthProvider calls `useQueryClient` (issue #2329 — it clears the query cache
// on logout / after login), so it must render under a QueryClientProvider, just
// like it does in the real app tree. A fresh client per test keeps them
// isolated.
let queryClient: QueryClient;

const wrapper = ({ children }: { children: ReactNode }) => (
  <QueryClientProvider client={queryClient}>
    <AuthProvider apiBaseUrl={API_BASE}>{children}</AuthProvider>
  </QueryClientProvider>
);

const mockedSecureStore = SecureStore as jest.Mocked<typeof SecureStore>;
const mockedLocalAuth = LocalAuthentication as unknown as {
  hasHardwareAsync: jest.Mock;
  isEnrolledAsync: jest.Mock;
  authenticateAsync: jest.Mock;
};

jest.mock('expo-local-authentication', () => ({
  hasHardwareAsync: jest.fn(),
  isEnrolledAsync: jest.fn(),
  authenticateAsync: jest.fn(),
}));

const sampleUser = {
  id: 'u-1',
  email: 'jane@example.com',
  firstName: 'Jane',
  lastName: 'Doe',
  role: 'owner' as const,
};

/** Build a synthetic SecureStore where keys can be primed and updated. */
function primeSecureStore(initial: Record<string, string | null> = {}) {
  const store = new Map<string, string>();
  for (const [k, v] of Object.entries(initial)) {
    if (v !== null) store.set(k, v);
  }

  mockedSecureStore.getItemAsync.mockImplementation(async (key) => store.get(key) ?? null);
  mockedSecureStore.setItemAsync.mockImplementation(async (key, value) => {
    store.set(key, value);
  });
  mockedSecureStore.deleteItemAsync.mockImplementation(async (key) => {
    store.delete(key);
  });

  return store;
}

function mockFetchOnce(body: unknown, status = 200) {
  (globalThis.fetch as jest.Mock).mockResolvedValueOnce({
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  });
}

describe('AuthContext integration', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    asyncStore.clear();
    queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    globalThis.fetch = jest.fn() as jest.Mock;
    primeSecureStore();
    mockedLocalAuth.hasHardwareAsync.mockResolvedValue(true);
    mockedLocalAuth.isEnrolledAsync.mockResolvedValue(true);
  });

  describe('initialization', () => {
    it('boots into the unauthenticated state when no tokens are stored', async () => {
      const { result } = renderHook(() => useAuth(), { wrapper });

      await waitFor(() => expect(result.current.isLoading).toBe(false));
      expect(result.current.isAuthenticated).toBe(false);
      expect(result.current.user).toBeNull();
      expect(result.current.accessToken).toBeNull();
      expect(result.current.biometricAvailable).toBe(true);
    });

    it('restores the session when SecureStore already holds tokens', async () => {
      primeSecureStore({
        ppt_access_token: 'stored-access',
        ppt_user: JSON.stringify(sampleUser),
        ppt_biometric_enabled: 'true',
      });

      const { result } = renderHook(() => useAuth(), { wrapper });

      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));
      expect(result.current.accessToken).toBe('stored-access');
      expect(result.current.user).toEqual(sampleUser);
      expect(result.current.biometricEnabled).toBe(true);
    });

    it('reports biometricAvailable=false when hardware is missing', async () => {
      mockedLocalAuth.hasHardwareAsync.mockResolvedValue(false);

      const { result } = renderHook(() => useAuth(), { wrapper });

      await waitFor(() => expect(result.current.isLoading).toBe(false));
      expect(result.current.biometricAvailable).toBe(false);
    });
  });

  // Regression for the code-review finding: the cold-start `initialize()` effect
  // used to restore `isAuthenticated: true` whenever a stored access token +
  // user existed, without checking the token's `exp`. Access tokens are
  // short-lived, so a cold start after the TTL restored an already-expired
  // bearer. The same gap existed on the biometric-unlock path.
  describe('stale access-token restore (exp check)', () => {
    it('refreshes an expired stored access token on cold start before restoring the session', async () => {
      const store = primeSecureStore({
        ppt_access_token: expiredToken(),
        ppt_refresh_token: 'stored-refresh',
        ppt_user: JSON.stringify(sampleUser),
      });
      // initialize() must exchange the expired bearer via /auth/refresh.
      mockFetchOnce({ access_token: 'refreshed-access', refresh_token: 'refreshed-refresh' });

      const { result } = renderHook(() => useAuth(), { wrapper });

      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));
      // The refresh endpoint was hit on cold start (not the login endpoint).
      expect(globalThis.fetch).toHaveBeenCalledWith(
        `${API_BASE}/api/v1/auth/refresh`,
        expect.objectContaining({ method: 'POST' })
      );
      const [, init] = (globalThis.fetch as jest.Mock).mock.calls.at(-1) ?? [];
      expect(JSON.parse(init.body)).toEqual({ refresh_token: 'stored-refresh' });
      // The session is trusted only with the freshly-issued token.
      expect(result.current.accessToken).toBe('refreshed-access');
      expect(result.current.user).toEqual(sampleUser);
      expect(store.get('ppt_access_token')).toBe('refreshed-access');
      expect(store.get('ppt_refresh_token')).toBe('refreshed-refresh');
    });

    it('drops to the login screen when the cold-start refresh of an expired token fails', async () => {
      const store = primeSecureStore({
        ppt_access_token: expiredToken(),
        ppt_refresh_token: 'stored-refresh',
        ppt_user: JSON.stringify(sampleUser),
      });
      mockFetchOnce({}, 401); // refresh rejected

      const { result } = renderHook(() => useAuth(), { wrapper });

      await waitFor(() => expect(result.current.isLoading).toBe(false));
      // Never restore an expired session when it cannot be refreshed.
      expect(result.current.isAuthenticated).toBe(false);
      expect(result.current.user).toBeNull();
      expect(result.current.accessToken).toBeNull();
      // The failed refresh cascaded through logout(), clearing SecureStore.
      expect(store.has('ppt_access_token')).toBe(false);
      expect(store.has('ppt_refresh_token')).toBe(false);
    });

    it('restores the session without a refresh when the stored access token is still valid', async () => {
      const token = validToken();
      primeSecureStore({
        ppt_access_token: token,
        ppt_refresh_token: 'stored-refresh',
        ppt_user: JSON.stringify(sampleUser),
      });

      const { result } = renderHook(() => useAuth(), { wrapper });

      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));
      expect(result.current.accessToken).toBe(token);
      // A valid token must NOT trigger a network round-trip on boot.
      expect(globalThis.fetch).not.toHaveBeenCalled();
    });
  });

  describe('login', () => {
    it('persists tokens and flips into the authenticated state on success', async () => {
      const store = primeSecureStore();
      // The api-server returns snake_case JSON keys; keep the test aligned
      // with the real contract so AuthContext's mapping is exercised.
      mockFetchOnce({
        access_token: 'new-access',
        refresh_token: 'new-refresh',
        user: sampleUser,
      });

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isLoading).toBe(false));

      await act(async () => {
        await result.current.login('jane@example.com', 'pw');
      });

      expect(globalThis.fetch).toHaveBeenCalledWith(
        `${API_BASE}/api/v1/auth/login`,
        expect.objectContaining({
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
        })
      );

      const [, init] = (globalThis.fetch as jest.Mock).mock.calls[0];
      expect(JSON.parse(init.body)).toEqual({
        email: 'jane@example.com',
        password: 'pw',
      });

      expect(result.current.isAuthenticated).toBe(true);
      expect(result.current.accessToken).toBe('new-access');
      expect(result.current.user).toEqual(sampleUser);

      // Tokens must be persisted to SecureStore.
      expect(store.get('ppt_access_token')).toBe('new-access');
      expect(store.get('ppt_refresh_token')).toBe('new-refresh');
      expect(JSON.parse(store.get('ppt_user') ?? '{}')).toEqual(sampleUser);
    });

    it('evicts a stale cached tenant id on login (issue #2329)', async () => {
      primeSecureStore();
      // A tenant id left over from a prior session that was never explicitly
      // logged out — must not survive into the new login.
      queryClient.setQueryData(['auth', 'tenant-id'], 'org-stale');
      mockFetchOnce({
        access_token: 'new-access',
        refresh_token: 'new-refresh',
        user: sampleUser,
      });

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isLoading).toBe(false));

      await act(async () => {
        await result.current.login('jane@example.com', 'pw');
      });

      expect(queryClient.getQueryData(['auth', 'tenant-id'])).toBeUndefined();
    });

    it('clears non-tenant-scoped cached data on login so no prior org data survives (issue #2361)', async () => {
      primeSecureStore();
      // Mobile read-query keys are NOT tenant-scoped, so a login that follows a
      // session which wasn't explicitly logged out must wipe the whole cache —
      // otherwise the previous org's list/detail data is served under identical
      // keys until a background refetch (or forever, if offline).
      queryClient.setQueryData(['auth', 'tenant-id'], 'org-stale');
      queryClient.setQueryData(['documents', 'list'], [{ id: 'doc-A' }]);
      queryClient.setQueryData(['buildings', 'list'], [{ id: 'b-A' }]);
      mockFetchOnce({
        access_token: 'new-access',
        refresh_token: 'new-refresh',
        user: sampleUser,
      });

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isLoading).toBe(false));

      await act(async () => {
        await result.current.login('jane@example.com', 'pw');
      });

      expect(queryClient.getQueryData(['auth', 'tenant-id'])).toBeUndefined();
      expect(queryClient.getQueryData(['documents', 'list'])).toBeUndefined();
      expect(queryClient.getQueryData(['buildings', 'list'])).toBeUndefined();
    });

    it('throws on bad credentials and leaves the state unauthenticated', async () => {
      primeSecureStore();
      mockFetchOnce({ message: 'Invalid credentials' }, 401);

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isLoading).toBe(false));

      await expect(
        act(async () => {
          await result.current.login('jane@example.com', 'wrong');
        })
      ).rejects.toThrow('Invalid credentials');

      expect(result.current.isAuthenticated).toBe(false);
      expect(result.current.accessToken).toBeNull();
      // No tokens were written.
      expect(mockedSecureStore.setItemAsync).not.toHaveBeenCalledWith(
        'ppt_access_token',
        expect.anything()
      );
    });
  });

  describe('logout', () => {
    it('clears all auth keys from SecureStore and resets state', async () => {
      const store = primeSecureStore({
        ppt_access_token: 'access',
        ppt_refresh_token: 'refresh',
        ppt_user: JSON.stringify(sampleUser),
      });

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));

      await act(async () => {
        await result.current.logout();
      });

      expect(result.current.isAuthenticated).toBe(false);
      expect(result.current.user).toBeNull();
      expect(result.current.accessToken).toBeNull();
      expect(store.has('ppt_access_token')).toBe(false);
      expect(store.has('ppt_refresh_token')).toBe(false);
      expect(store.has('ppt_user')).toBe(false);
    });

    it('clears the query cache so no previous-tenant data survives (issue #2329)', async () => {
      primeSecureStore({
        ppt_access_token: 'access',
        ppt_user: JSON.stringify(sampleUser),
      });

      // Seed a cached tenant id the way `useTenantId` would (staleTime:Infinity,
      // so nothing else would ever evict it).
      queryClient.setQueryData(['auth', 'tenant-id'], 'org-old');
      // ...and some unrelated tenant-scoped data.
      queryClient.setQueryData(['buildings', 'org-old'], [{ id: 'b-1' }]);

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));

      await act(async () => {
        await result.current.logout();
      });

      expect(queryClient.getQueryData(['auth', 'tenant-id'])).toBeUndefined();
      expect(queryClient.getQueryData(['buildings', 'org-old'])).toBeUndefined();
    });
  });

  describe('cross-user session reset (issue #2399)', () => {
    // Seed the persisted, tenant-scoped AsyncStorage namespaces the way a live
    // session would (offline responses/queue + home-screen widget data).
    function seedPersistedTenantCaches(tag: string) {
      asyncStore.set('ppt_cache_faults_list', JSON.stringify([{ id: `f-${tag}` }]));
      asyncStore.set('ppt_offline_queue', JSON.stringify([{ id: `q-${tag}` }]));
      asyncStore.set('ppt_last_sync', '111');
      asyncStore.set('@ppt/widget_data', JSON.stringify({ w1: { data: {} } }));
      asyncStore.set('@ppt/widget_configs', JSON.stringify([{ id: 'w1', buildingId: `b-${tag}` }]));
    }

    const persistedKeys = [
      'ppt_cache_faults_list',
      'ppt_offline_queue',
      'ppt_last_sync',
      '@ppt/widget_data',
      '@ppt/widget_configs',
    ];

    it('purges in-memory AND persisted caches across login A -> logout -> login B', async () => {
      const store = primeSecureStore();

      // --- login as user A ---
      mockFetchOnce({ access_token: 'a-access', refresh_token: 'a-refresh', user: sampleUser });
      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isLoading).toBe(false));
      await act(async () => {
        await result.current.login('jane@example.com', 'pw');
      });

      // Populate A's in-memory + persisted caches during the session.
      queryClient.setQueryData(['buildings', 'list'], [{ id: 'b-A' }]);
      seedPersistedTenantCaches('A');

      // --- logout ---
      await act(async () => {
        await result.current.logout();
      });

      // Both cache layers are wiped by logout.
      expect(queryClient.getQueryData(['buildings', 'list'])).toBeUndefined();
      for (const key of persistedKeys) {
        expect(asyncStore.has(key)).toBe(false);
      }

      // Belt-and-braces: even if data leaks back in before the next login (or
      // the previous session was never explicitly logged out), login must purge.
      queryClient.setQueryData(['documents', 'list'], [{ id: 'doc-A' }]);
      seedPersistedTenantCaches('A2');

      // --- login as a DIFFERENT user B ---
      const userB = { ...sampleUser, id: 'u-2', email: 'bob@example.com' };
      mockFetchOnce({ access_token: 'b-access', refresh_token: 'b-refresh', user: userB });
      await act(async () => {
        await result.current.login('bob@example.com', 'pw');
      });

      // B sees no trace of A in either layer.
      expect(result.current.user).toEqual(userB);
      expect(queryClient.getQueryData(['documents', 'list'])).toBeUndefined();
      for (const key of persistedKeys) {
        expect(asyncStore.has(key)).toBe(false);
      }
      // B's own freshly-issued token is persisted.
      expect(store.get('ppt_access_token')).toBe('b-access');
    });
  });

  describe('refreshToken', () => {
    it('exchanges the stored refresh token for a new access token', async () => {
      const store = primeSecureStore({
        ppt_access_token: 'old-access',
        ppt_refresh_token: 'old-refresh',
        ppt_user: JSON.stringify(sampleUser),
      });

      mockFetchOnce({
        access_token: 'rotated-access',
        refresh_token: 'rotated-refresh',
      });

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));

      await act(async () => {
        await result.current.refreshToken();
      });

      const [url, init] = (globalThis.fetch as jest.Mock).mock.calls.at(-1) ?? [];
      expect(url).toBe(`${API_BASE}/api/v1/auth/refresh`);
      // Backend's RefreshTokenRequest uses snake_case.
      expect(JSON.parse(init.body)).toEqual({ refresh_token: 'old-refresh' });

      expect(result.current.accessToken).toBe('rotated-access');
      expect(store.get('ppt_access_token')).toBe('rotated-access');
      expect(store.get('ppt_refresh_token')).toBe('rotated-refresh');
    });

    it('logs out when the server rejects the refresh', async () => {
      const store = primeSecureStore({
        ppt_access_token: 'old-access',
        ppt_refresh_token: 'old-refresh',
        ppt_user: JSON.stringify(sampleUser),
      });

      mockFetchOnce({}, 401);

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));

      let caught: unknown;
      await act(async () => {
        try {
          await result.current.refreshToken();
        } catch (e) {
          caught = e;
        }
      });

      // The refresh threw and the inner logout() ran.
      expect(caught).toBeInstanceOf(Error);
      // SecureStore was cleared by the cascading logout.
      expect(store.has('ppt_access_token')).toBe(false);
      expect(store.has('ppt_refresh_token')).toBe(false);
      // Reactive state may need one more tick after the inner logout's setState.
      await waitFor(() => expect(result.current.accessToken).toBeNull());
      expect(result.current.isAuthenticated).toBe(false);
    });

    it('rejects when no refresh token is stored', async () => {
      primeSecureStore({
        ppt_access_token: 'access',
        ppt_user: JSON.stringify(sampleUser),
      });

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));

      await expect(
        act(async () => {
          await result.current.refreshToken();
        })
      ).rejects.toThrow(/refresh token/i);
    });
  });

  describe('biometric flow', () => {
    it('enableBiometric stores the flag when biometric prompt succeeds', async () => {
      const store = primeSecureStore();
      mockedLocalAuth.authenticateAsync.mockResolvedValue({ success: true });

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isLoading).toBe(false));

      let enabled: boolean | undefined;
      await act(async () => {
        enabled = await result.current.enableBiometric();
      });

      expect(enabled).toBe(true);
      expect(result.current.biometricEnabled).toBe(true);
      expect(store.get('ppt_biometric_enabled')).toBe('true');

      // The OS biometric dialog copy must come from i18n, not hardcoded
      // English (code-review-mobile-rn-biometric-prompt-i18n). The shared
      // react-i18next mock returns the key verbatim (`t(key) => key`), so we
      // assert on the keys and that the old literals are gone.
      expect(mockedLocalAuth.authenticateAsync).toHaveBeenCalledWith({
        promptMessage: 'auth.biometric.enablePrompt',
        cancelLabel: 'auth.biometric.cancel',
        fallbackLabel: 'auth.biometric.usePasscode',
      });
      const enableArg = mockedLocalAuth.authenticateAsync.mock.calls[0][0];
      expect(enableArg.promptMessage).not.toBe('Enable biometric login');
      expect(enableArg.fallbackLabel).not.toBe('Use passcode');
    });

    it('enableBiometric returns false when prompt is cancelled', async () => {
      primeSecureStore();
      mockedLocalAuth.authenticateAsync.mockResolvedValue({ success: false });

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isLoading).toBe(false));

      let enabled: boolean | undefined;
      await act(async () => {
        enabled = await result.current.enableBiometric();
      });

      expect(enabled).toBe(false);
      expect(result.current.biometricEnabled).toBe(false);
    });

    it('disableBiometric removes the stored flag', async () => {
      const store = primeSecureStore({
        ppt_biometric_enabled: 'true',
        ppt_access_token: 'a',
        ppt_user: JSON.stringify(sampleUser),
      });

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));

      await act(async () => {
        await result.current.disableBiometric();
      });

      expect(result.current.biometricEnabled).toBe(false);
      expect(store.has('ppt_biometric_enabled')).toBe(false);
    });

    it('authenticateWithBiometric restores stored credentials on success', async () => {
      primeSecureStore({
        ppt_access_token: 'stored',
        ppt_user: JSON.stringify(sampleUser),
        ppt_biometric_enabled: 'true',
      });
      mockedLocalAuth.authenticateAsync.mockResolvedValue({ success: true });

      const { result } = renderHook(() => useAuth(), { wrapper });
      // Already authenticated from the stored tokens.
      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));

      // Synthetically log out (simulate the account screen flow), then
      // re-auth via biometric.
      await act(async () => {
        await result.current.logout();
      });
      // logout cleared SecureStore - we need to re-prime to test biometric
      // restoration of stored credentials. Re-add them as if a previous
      // session had persisted them across the lock screen.
      mockedSecureStore.getItemAsync.mockImplementation(async (key) => {
        if (key === 'ppt_access_token') return 'stored';
        if (key === 'ppt_user') return JSON.stringify(sampleUser);
        return null;
      });

      let ok: boolean | undefined;
      await act(async () => {
        ok = await result.current.authenticateWithBiometric();
      });

      expect(ok).toBe(true);
      expect(result.current.isAuthenticated).toBe(true);
      expect(result.current.user).toEqual(sampleUser);

      // Unlock dialog copy is i18n-sourced, not hardcoded English
      // (code-review-mobile-rn-biometric-prompt-i18n).
      expect(mockedLocalAuth.authenticateAsync).toHaveBeenCalledWith({
        promptMessage: 'auth.biometric.authenticatePrompt',
        cancelLabel: 'auth.biometric.cancel',
        fallbackLabel: 'auth.biometric.usePassword',
      });
      const authArg = mockedLocalAuth.authenticateAsync.mock.calls[0][0];
      expect(authArg.promptMessage).not.toBe('Login to Property Management');
      expect(authArg.fallbackLabel).not.toBe('Use password');
    });

    it('authenticateWithBiometric deliberately preserves persisted caches (issue #2399)', async () => {
      primeSecureStore({
        ppt_access_token: 'stored',
        ppt_user: JSON.stringify(sampleUser),
        ppt_biometric_enabled: 'true',
      });
      mockedLocalAuth.authenticateAsync.mockResolvedValue({ success: true });
      // A persisted cache entry belonging to the SAME stored user.
      asyncStore.set('ppt_cache_faults_list', JSON.stringify([{ id: 'f-1' }]));

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));

      await act(async () => {
        await result.current.authenticateWithBiometric();
      });

      // Biometric unlock restores the same session, so — unlike login/logout —
      // it must NOT purge the cache (resetLocalData is intentionally not called).
      expect(result.current.isAuthenticated).toBe(true);
      expect(asyncStore.has('ppt_cache_faults_list')).toBe(true);
    });

    it('authenticateWithBiometric refreshes an expired stored token on unlock without refetching (#2399)', async () => {
      // Boot with a still-valid token so mount restores cleanly (no refresh).
      primeSecureStore({
        ppt_access_token: validToken(),
        ppt_refresh_token: 'stored-refresh',
        ppt_user: JSON.stringify(sampleUser),
        ppt_biometric_enabled: 'true',
      });
      mockedLocalAuth.authenticateAsync.mockResolvedValue({ success: true });
      // A same-user persisted cache entry that must survive the unlock.
      asyncStore.set('ppt_cache_faults_list', JSON.stringify([{ id: 'f-1' }]));

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));

      // Simulate the device sitting locked past the access-token TTL: the stored
      // bearer is now expired, but the refresh token is still valid.
      const store = primeSecureStore({
        ppt_access_token: expiredToken(),
        ppt_refresh_token: 'stored-refresh',
        ppt_user: JSON.stringify(sampleUser),
        ppt_biometric_enabled: 'true',
      });
      mockFetchOnce({ access_token: 'unlocked-access', refresh_token: 'unlocked-refresh' });

      let ok: boolean | undefined;
      await act(async () => {
        ok = await result.current.authenticateWithBiometric();
      });

      expect(ok).toBe(true);
      expect(result.current.isAuthenticated).toBe(true);
      // The token was refreshed on unlock...
      expect(globalThis.fetch).toHaveBeenCalledWith(
        `${API_BASE}/api/v1/auth/refresh`,
        expect.objectContaining({ method: 'POST' })
      );
      expect(result.current.accessToken).toBe('unlocked-access');
      expect(store.get('ppt_access_token')).toBe('unlocked-access');
      // ...but the user's cached data was preserved (only the token refreshed,
      // no resetLocalData / user refetch — #2399 intent kept intact).
      expect(asyncStore.has('ppt_cache_faults_list')).toBe(true);
      expect(globalThis.fetch).not.toHaveBeenCalledWith(
        `${API_BASE}/api/v1/auth/login`,
        expect.anything()
      );
    });

    it('authenticateWithBiometric returns false when the expired-token refresh fails', async () => {
      primeSecureStore({
        ppt_access_token: validToken(),
        ppt_refresh_token: 'stored-refresh',
        ppt_user: JSON.stringify(sampleUser),
        ppt_biometric_enabled: 'true',
      });
      mockedLocalAuth.authenticateAsync.mockResolvedValue({ success: true });

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isAuthenticated).toBe(true));

      // Log out, then present an expired stored token whose refresh is rejected.
      await act(async () => {
        await result.current.logout();
      });
      primeSecureStore({
        ppt_access_token: expiredToken(),
        ppt_refresh_token: 'stored-refresh',
        ppt_user: JSON.stringify(sampleUser),
        ppt_biometric_enabled: 'true',
      });
      mockFetchOnce({}, 401);

      let ok: boolean | undefined;
      await act(async () => {
        ok = await result.current.authenticateWithBiometric();
      });

      expect(ok).toBe(false);
      await waitFor(() => expect(result.current.isAuthenticated).toBe(false));
    });

    it('authenticateWithBiometric is a no-op when biometric is not enabled', async () => {
      primeSecureStore();

      const { result } = renderHook(() => useAuth(), { wrapper });
      await waitFor(() => expect(result.current.isLoading).toBe(false));

      let ok: boolean | undefined;
      await act(async () => {
        ok = await result.current.authenticateWithBiometric();
      });

      expect(ok).toBe(false);
      expect(mockedLocalAuth.authenticateAsync).not.toHaveBeenCalled();
    });
  });
});

// Resource-layer guard: the biometric prompt keys referenced by AuthContext
// must exist in every shipped locale and carry a real, non-English translation
// (code-review-mobile-rn-biometric-prompt-i18n). Mirrors the pattern in
// FaultsListScreen.i18n.test.tsx.
describe('auth.biometric prompt keys resolve to real, locale-distinct translations', () => {
  const BIOMETRIC_KEYS = [
    'enablePrompt',
    'authenticatePrompt',
    'cancel',
    'usePasscode',
    'usePassword',
  ] as const;

  it('en carries the expected English copy', () => {
    expect(enLocale.auth.biometric.enablePrompt).toBe('Enable biometric login');
    expect(enLocale.auth.biometric.authenticatePrompt).toBe('Login to Property Management');
    expect(enLocale.auth.biometric.cancel).toBe('Cancel');
    expect(enLocale.auth.biometric.usePasscode).toBe('Use passcode');
    expect(enLocale.auth.biometric.usePassword).toBe('Use password');
  });

  it('every shipped locale defines all biometric keys as non-empty strings', () => {
    for (const locale of [enLocale, skLocale, csLocale, deLocale, plLocale, huLocale]) {
      for (const key of BIOMETRIC_KEYS) {
        const value = locale.auth.biometric[key];
        expect(typeof value).toBe('string');
        expect(value.length).toBeGreaterThan(0);
      }
    }
  });

  it('non-English locales translate the prompt copy away from English', () => {
    // `cancel` is a proper noun-ish UI word that can legitimately collide in
    // some languages, so only assert divergence on the descriptive prompts.
    for (const locale of [skLocale, csLocale, deLocale, plLocale, huLocale]) {
      expect(locale.auth.biometric.enablePrompt).not.toBe(enLocale.auth.biometric.enablePrompt);
      expect(locale.auth.biometric.authenticatePrompt).not.toBe(
        enLocale.auth.biometric.authenticatePrompt
      );
    }
  });
});
