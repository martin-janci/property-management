'use client';

import { useQueryClient } from '@tanstack/react-query';
import * as LocalAuthentication from 'expo-local-authentication';
import * as SecureStore from 'expo-secure-store';
import type { ReactNode } from 'react';
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import { feedbackManager } from '../onboarding/FeedbackManager';
import { resetLocalData } from '../services/resetLocalData';
import { isJwtExpired } from '../utils/jwt';
import {
  buildSessionUser,
  normalizeUser,
  parseStoredUser,
  readTokenPair,
  TENANT_ID_KEY,
} from '../utils/session';

// Token storage keys
const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
const USER_KEY = 'ppt_user';
const BIOMETRIC_ENABLED_KEY = 'ppt_biometric_enabled';

// Treat an access token that expires within this many seconds as already stale,
// so a restored session refreshes it proactively instead of handing the app a
// bearer that dies on (or moments after) the first request.
const TOKEN_REFRESH_SKEW_SECONDS = 30;

/**
 * Raised by `refreshToken()` when the `/auth/refresh` request could not reach
 * the server (offline / DNS / timeout — i.e. `fetch` itself rejected), so the
 * server never reached a verdict on the refresh token.
 *
 * This is deliberately distinct from a server *rejection* (a non-OK response —
 * the refresh token is genuinely expired/revoked/reused). On a rejection the
 * session is dead and `refreshToken()` tears it down (`logout()`); on this
 * network error the session is NOT known to be dead, so callers keep the user
 * on their stale access token + offline cache instead of evicting them and
 * wiping their data. See the comments in `refreshToken()` / `initialize()`.
 */
class TokenRefreshNetworkError extends Error {
  constructor(options?: { cause?: unknown }) {
    super('Token refresh failed: server unreachable', options);
    this.name = 'TokenRefreshNetworkError';
  }
}

export interface User {
  id: string;
  email: string;
  firstName: string;
  lastName: string;
  role: 'owner' | 'tenant' | 'resident' | 'manager' | 'admin';
  buildingId?: string;
  unitId?: string;
  avatarUrl?: string;
  organizationId?: string;
  organizationName?: string;
}

export interface AuthState {
  isLoading: boolean;
  isAuthenticated: boolean;
  user: User | null;
  accessToken: string | null;
  biometricEnabled: boolean;
  biometricAvailable: boolean;
}

export interface AuthContextValue extends AuthState {
  login: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  refreshToken: () => Promise<void>;
  enableBiometric: () => Promise<boolean>;
  disableBiometric: () => Promise<void>;
  authenticateWithBiometric: () => Promise<boolean>;
  /**
   * Merge changes into the signed-in user and persist them to SecureStore.
   *
   * This is the single source of truth for mutating the cached user after a
   * successful server write (e.g. `PATCH /api/v1/auth/me` from ProfileScreen):
   * it keeps the in-memory context and the SecureStore `ppt_user` key (used to
   * restore the session on cold start / biometric unlock) in sync, so callers
   * must NOT write `USER_KEY` directly. No-ops when signed out.
   */
  updateUser: (changes: Partial<User>) => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}

interface AuthProviderProps {
  children: ReactNode;
  apiBaseUrl: string;
}

export function AuthProvider({ children, apiBaseUrl }: AuthProviderProps) {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  const [state, setState] = useState<AuthState>({
    isLoading: true,
    isAuthenticated: false,
    user: null,
    accessToken: null,
    biometricEnabled: false,
    biometricAvailable: false,
  });

  // Mirror the signed-in user so `updateUser()` can merge against the current
  // user synchronously. A `useCallback([])` can't close over fresh `state`, and
  // reading it via the `setState` updater's side effect is unreliable: React may
  // not run the updater synchronously (eager-state bailout only fires with an
  // empty update queue), so the persist that depended on it could silently be
  // skipped. The ref is always current.
  const userRef = useRef<User | null>(state.user);
  useEffect(() => {
    userRef.current = state.user;
  }, [state.user]);

  const login = useCallback(
    async (email: string, password: string) => {
      setState((prev) => ({ ...prev, isLoading: true }));

      try {
        const response = await fetch(`${apiBaseUrl}/api/v1/auth/login`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ email, password }),
        });

        if (!response.ok) {
          const error = await response.json();
          throw new Error(error.message || 'Login failed');
        }

        const data = await response.json();
        // The api-server answers in camelCase (`accessToken`, `refreshToken`,
        // `tenants[]`) and embeds no user object, so the user is derived from
        // the token claims plus the active tenant.
        const tokens = readTokenPair(data);
        if (!tokens) {
          throw new Error('Login failed');
        }
        const { accessToken, refreshToken } = tokens;
        const { user, tenantId } = buildSessionUser(data, accessToken);

        // Store tokens securely
        await SecureStore.setItemAsync(ACCESS_TOKEN_KEY, accessToken);
        await SecureStore.setItemAsync(REFRESH_TOKEN_KEY, refreshToken);
        await SecureStore.setItemAsync(USER_KEY, JSON.stringify(user));

        // A login always begins a fresh session, so no previous org's cached
        // data may survive it (issue #2361). Removing only the ['auth',
        // 'tenant-id'] key fixed tenant-id resolution (issue #2329) but left
        // every other cached query from the prior org in place — the mobile
        // read-query keys (['buildings','list'], ['documents','list',…],
        // ['faults','list'], etc.) are NOT tenant-scoped, so after a login that
        // follows a session which wasn't explicitly logged out, TanStack Query
        // would serve user A's data to user B until a background refetch (or
        // never, if offline). Clear the whole cache to mirror logout().
        //
        // queryClient.clear() only wipes the in-memory cache; the AsyncStorage
        // caches (offline queue/responses, home-screen widget data) outlive the
        // process, so purge those first or a prior org's data survives the
        // login (issue #2399, follow-up to #2361).
        await resetLocalData();
        queryClient.clear();

        // Remember the organisation so tenant-scoped requests can send
        // X-Tenant-ID (the access token has no `tenant_id` claim).
        if (tenantId) {
          await SecureStore.setItemAsync(TENANT_ID_KEY, tenantId);
        } else {
          await SecureStore.deleteItemAsync(TENANT_ID_KEY);
        }

        setState((prev) => ({
          ...prev,
          isLoading: false,
          isAuthenticated: true,
          user,
          accessToken,
        }));
      } catch (error) {
        setState((prev) => ({ ...prev, isLoading: false }));
        throw error;
      }
    },
    [apiBaseUrl, queryClient]
  );

  const logout = useCallback(async () => {
    try {
      // Clear stored tokens
      await SecureStore.deleteItemAsync(ACCESS_TOKEN_KEY);
      await SecureStore.deleteItemAsync(REFRESH_TOKEN_KEY);
      await SecureStore.deleteItemAsync(USER_KEY);
      await SecureStore.deleteItemAsync(TENANT_ID_KEY);

      // Wipe the whole query cache on logout (issue #2329). `useTenantId`
      // caches the tenant id forever (`staleTime: Infinity`) and nothing else
      // invalidated it, so after logout + login as a user in a *different* org
      // its consumers kept serving the previous org's tenant id until an app
      // restart — producing persistent cross-tenant 403s / wrong cache keys.
      // Clearing everything also ensures no other org's data survives logout.
      //
      // Purge the AsyncStorage-backed caches too (offline queue/responses,
      // widget data) — they persist across restarts, so clearing only the
      // in-memory query cache would leave a prior org's data (and a prior
      // user's queued writes) behind (issue #2399, follow-up to #2361).
      await resetLocalData();
      queryClient.clear();

      setState((prev) => ({
        ...prev,
        isAuthenticated: false,
        user: null,
        accessToken: null,
      }));
    } catch (error) {
      console.error('Failed to logout:', error);
      throw error;
    }
  }, [queryClient]);

  const refreshToken = useCallback(async () => {
    const storedRefreshToken = await SecureStore.getItemAsync(REFRESH_TOKEN_KEY);

    if (!storedRefreshToken) {
      // Nothing to refresh with — the session is unrecoverable, tear it down.
      await logout();
      throw new Error('No refresh token available');
    }

    let response: Response;
    try {
      response = await fetch(`${apiBaseUrl}/api/v1/auth/refresh`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ refreshToken: storedRefreshToken }),
      });
    } catch (error) {
      // Transport failure (offline / DNS / timeout): `fetch` rejected, so the
      // server never reached a verdict on the refresh token — the session is
      // NOT known to be dead. Do NOT log out here: `logout()` deletes the
      // (still-valid) tokens AND calls `resetLocalData()`, which wipes the
      // offline mutation queue, cached responses, NFC building-access
      // credentials and widget data. Evicting a user — and destroying their
      // offline data — merely because they opened the app without a network is
      // the exact opposite of what the offline-support layer exists for. Keep
      // the session intact and let the caller retry when connectivity returns;
      // raise a distinguishable error so callers can tell "offline" apart from
      // "refresh token rejected".
      throw new TokenRefreshNetworkError({ cause: error });
    }

    if (!response.ok) {
      // The server reached a verdict and rejected the refresh token (expired,
      // revoked, or already reused) — the session is genuinely dead, so tear it
      // down.
      await logout();
      throw new Error('Token refresh failed');
    }

    const tokens = readTokenPair(await response.json().catch(() => null));
    if (!tokens) {
      // Server responded OK but the body was unusable — treat it as a failed
      // refresh and tear the session down.
      await logout();
      throw new Error('Token refresh failed');
    }
    const { accessToken, refreshToken: newRefreshToken } = tokens;

    // Store new tokens
    await SecureStore.setItemAsync(ACCESS_TOKEN_KEY, accessToken);
    await SecureStore.setItemAsync(REFRESH_TOKEN_KEY, newRefreshToken);

    setState((prev) => ({
      ...prev,
      accessToken,
    }));
  }, [apiBaseUrl, logout]);

  const enableBiometric = useCallback(async (): Promise<boolean> => {
    if (!state.biometricAvailable) {
      return false;
    }

    try {
      const result = await LocalAuthentication.authenticateAsync({
        promptMessage: t('auth.biometric.enablePrompt'),
        cancelLabel: t('auth.biometric.cancel'),
        fallbackLabel: t('auth.biometric.usePasscode'),
      });

      if (result.success) {
        await SecureStore.setItemAsync(BIOMETRIC_ENABLED_KEY, 'true');
        setState((prev) => ({ ...prev, biometricEnabled: true }));
        return true;
      }

      return false;
    } catch (error) {
      console.error('Failed to enable biometric:', error);
      return false;
    }
  }, [state.biometricAvailable, t]);

  const disableBiometric = useCallback(async () => {
    await SecureStore.deleteItemAsync(BIOMETRIC_ENABLED_KEY);
    setState((prev) => ({ ...prev, biometricEnabled: false }));
  }, []);

  const updateUser = useCallback(async (changes: Partial<User>) => {
    // Build the next user synchronously so the persisted copy matches the
    // in-memory one exactly. Bail when signed out — there is nothing to merge
    // into and no session to persist.
    const current = userRef.current;
    if (!current) return;
    // Normalise the merged user through the same canonical shaping the restore
    // path uses (`normalizeUser`, shared with `parseStoredUser`), so a merged-in
    // field can't drift the in-memory or persisted user away from the shape a
    // later cold-start / biometric restore would re-derive — e.g. an arbitrary
    // `role` is collapsed through `toAppRole` and junk keys are dropped, instead
    // of being stored verbatim. `current` always has an id, so `normalizeUser`
    // never returns null here; fall back to the raw merge defensively so a valid
    // session is never dropped by an update.
    const merged = { ...current, ...changes };
    const nextUser = normalizeUser(merged) ?? merged;
    userRef.current = nextUser;
    setState((prev) => (prev.user ? { ...prev, user: nextUser } : prev));
    await SecureStore.setItemAsync(USER_KEY, JSON.stringify(nextUser));
  }, []);

  const authenticateWithBiometric = useCallback(async (): Promise<boolean> => {
    if (!state.biometricEnabled || !state.biometricAvailable) {
      return false;
    }

    try {
      const result = await LocalAuthentication.authenticateAsync({
        promptMessage: t('auth.biometric.authenticatePrompt'),
        cancelLabel: t('auth.biometric.cancel'),
        fallbackLabel: t('auth.biometric.usePassword'),
      });

      if (result.success) {
        // Biometric unlock restores the SAME stored user's session, so unlike
        // login()/logout() it deliberately does NOT call resetLocalData() /
        // queryClient.clear() — the existing cache still belongs to this user
        // and re-fetching it on every unlock would be wasteful and drop
        // offline data (issue #2399).
        // Check if we have stored credentials
        const accessToken = await SecureStore.getItemAsync(ACCESS_TOKEN_KEY);
        const userJson = await SecureStore.getItemAsync(USER_KEY);

        const user = parseStoredUser(userJson);
        if (accessToken && user) {
          // The device may have sat locked past the access-token TTL, so the
          // stored bearer can be expired even though the refresh token is still
          // valid. Refresh it before restoring the session — but preserve the
          // #2399 intent above: only the *token* is refreshed here, we still do
          // NOT refetch the user's cached data.
          let activeToken = accessToken;
          if (isJwtExpired(accessToken, TOKEN_REFRESH_SKEW_SECONDS)) {
            try {
              await refreshToken();
              activeToken = (await SecureStore.getItemAsync(ACCESS_TOKEN_KEY)) ?? accessToken;
            } catch (error) {
              if (!(error instanceof TokenRefreshNetworkError)) {
                // The server rejected the refresh token — refreshToken() already
                // logged out (clearing SecureStore). The session is dead, fall
                // back to login.
                return false;
              }
              // Offline: the refresh token may still be valid, we just can't
              // reach the server. Restore the session on the stale access token
              // so biometric unlock works without a network — apiRequest will
              // refresh it opportunistically (or the server will 401 and the
              // normal auth-gate will tear it down) once connectivity returns.
              activeToken = accessToken;
            }
          }
          setState((prev) => ({
            ...prev,
            isAuthenticated: true,
            user,
            accessToken: activeToken,
          }));
          return true;
        }
      }

      return false;
    } catch (error) {
      console.error('Biometric authentication failed:', error);
      return false;
    }
  }, [state.biometricEnabled, state.biometricAvailable, t, refreshToken]);

  // Check biometric availability and load any stored session on mount. This
  // runs after the callbacks above so it can reuse `refreshToken()` when a
  // restored access token has expired (rather than duplicating refresh logic).
  useEffect(() => {
    async function initialize() {
      try {
        // Check biometric availability
        const compatible = await LocalAuthentication.hasHardwareAsync();
        const enrolled = await LocalAuthentication.isEnrolledAsync();
        const biometricAvailable = compatible && enrolled;

        // Load stored tokens
        const accessToken = await SecureStore.getItemAsync(ACCESS_TOKEN_KEY);
        const userJson = await SecureStore.getItemAsync(USER_KEY);
        const biometricEnabled = await SecureStore.getItemAsync(BIOMETRIC_ENABLED_KEY);

        // Validate the persisted user rather than casting the raw blob: a
        // corrupt or tampered `ppt_user` (e.g. an arbitrary `role`) must drop
        // to the login screen instead of restoring a trusted session.
        const user = parseStoredUser(userJson);
        if (accessToken && user) {
          const biometricEnabledFlag = biometricEnabled === 'true';

          // Access tokens are short-lived: a cold start after the TTL would
          // otherwise restore `isAuthenticated: true` with an already-expired
          // bearer. Refresh it first when the stored token is expired/near
          // expiry, and only trust the session once a fresh token is in hand.
          if (isJwtExpired(accessToken, TOKEN_REFRESH_SKEW_SECONDS)) {
            try {
              await refreshToken();
              const refreshed = await SecureStore.getItemAsync(ACCESS_TOKEN_KEY);
              setState({
                isLoading: false,
                isAuthenticated: true,
                user,
                accessToken: refreshed,
                biometricEnabled: biometricEnabledFlag,
                biometricAvailable,
              });
            } catch (error) {
              if (error instanceof TokenRefreshNetworkError) {
                // Offline cold start: the stored access token is expired but the
                // refresh token may still be valid — we just can't reach the
                // server to rotate it. Keep the user signed in on the stale
                // access token so the app is usable offline (cached reads,
                // queued writes via useOfflineSupport). apiRequest refreshes it
                // opportunistically — or the server 401s and the normal
                // auth-gate tears the session down — once the network returns.
                // Logging out here would evict a user with a valid session and
                // wipe their offline data purely because they launched offline.
                setState({
                  isLoading: false,
                  isAuthenticated: true,
                  user,
                  accessToken,
                  biometricEnabled: biometricEnabledFlag,
                  biometricAvailable,
                });
              } else {
                // Server rejected the refresh token — refreshToken() already ran
                // logout() (clearing SecureStore + caches). Drop to the login
                // screen.
                setState((prev) => ({
                  ...prev,
                  isLoading: false,
                  isAuthenticated: false,
                  user: null,
                  accessToken: null,
                  biometricEnabled: biometricEnabledFlag,
                  biometricAvailable,
                }));
              }
            }
          } else {
            setState({
              isLoading: false,
              isAuthenticated: true,
              user,
              accessToken,
              biometricEnabled: biometricEnabledFlag,
              biometricAvailable,
            });
          }
        } else {
          setState((prev) => ({
            ...prev,
            isLoading: false,
            biometricAvailable,
          }));
        }
      } catch (error) {
        console.error('Failed to initialize auth:', error);
        setState((prev) => ({
          ...prev,
          isLoading: false,
        }));
      }
    }

    initialize();
  }, [refreshToken]);

  // Keep the global FeedbackManager singleton's bearer in sync with the signed-in
  // session. The singleton is constructed once at module load with no token and
  // nothing else wires it to auth state, so without this every
  // `POST /api/v1/feedback` would ship unauthenticated — dropped as a 401 (then
  // queued for infinite retry) or persisted without user attribution. Re-runs on
  // login, token refresh, and logout (accessToken -> null), mirroring how
  // WidgetSettingsScreen feeds the widget bridge. Clearing the token on logout
  // matters on shared devices: the next user must not inherit the previous
  // user's bearer.
  useEffect(() => {
    feedbackManager.setAuthToken(state.accessToken);
  }, [state.accessToken]);

  const value = useMemo<AuthContextValue>(
    () => ({
      ...state,
      login,
      logout,
      refreshToken,
      enableBiometric,
      disableBiometric,
      authenticateWithBiometric,
      updateUser,
    }),
    [
      state,
      login,
      logout,
      refreshToken,
      enableBiometric,
      disableBiometric,
      authenticateWithBiometric,
      updateUser,
    ]
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
