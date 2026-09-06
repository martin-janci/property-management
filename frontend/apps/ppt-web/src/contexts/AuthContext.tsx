/**
 * Authentication Context for ppt-web.
 *
 * Provides authentication state and methods throughout the application.
 * Handles login, logout, token refresh, and session management.
 *
 * Uses @ppt/api-client for API communication and integrates with
 * the token provider for secure token management across all API modules.
 *
 * @see Story 79.2 - Authentication Flow Implementation
 * @see Story 81.1 - Wire AuthContext to API client
 */

import {
  AuthError,
  type AuthErrorCode,
  type AuthUser,
  clearOrgProvider,
  clearTokenProvider,
  createAuthApi,
  type SsoCallbackRequest,
  setOrgProvider,
  setTokenProvider,
  type TenantMembership,
  type TenantRole,
} from '@ppt/api-client';
import { useQueryClient } from '@tanstack/react-query';
import type React from 'react';
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { trackSignupLoggedIn } from '../features/auth/analytics';
import { configureApiClient, resetApiClient } from '../lib/api';
import { AUTHED_QUERY_KEY_ROOTS } from '../lib/queryKeys';

export type { AuthErrorCode, AuthUser };
// Re-export types from api-client for convenience
export { AuthError };

// ============================================================================
// Types
// ============================================================================

/** Authentication state */
export interface AuthState {
  /** Currently authenticated user, null if not authenticated */
  user: AuthUser | null;
  /** Whether the user is authenticated */
  isAuthenticated: boolean;
  /** Whether authentication state is being loaded/checked */
  isLoading: boolean;
}

/** Login credentials */
export interface LoginCredentials {
  email: string;
  password: string;
}

/** Authentication context value */
export interface AuthContextValue extends AuthState {
  /** Log in with email and password */
  login: (credentials: LoginCredentials) => Promise<void>;
  /**
   * Complete an SSO / OAuth callback flow.
   *
   * Called by AuthCallbackPage (/auth/callback) after the provider redirects
   * back with `?code=…&state=…`. Exchanges the code for PPT JWT tokens via
   * POST /api/v1/auth/sso/callback, stores them via tokenProvider, and
   * updates the authenticated user in state.
   */
  loginWithSsoCode: (request: SsoCallbackRequest) => Promise<void>;
  /** Log out the current user */
  logout: () => Promise<void>;
  /** Refresh the access token */
  refreshToken: () => Promise<string | null>;
  /** Get the current access token */
  getAccessToken: () => string | null;
  /**
   * Update the cached user object (in-memory state + storage). Used by
   * profile-edit screens so the UI reflects edits without forcing a reload.
   */
  setUser: (user: AuthUser) => void;
}

// ============================================================================
// Constants
// ============================================================================

// API base URL - prefer environment configuration for different environments (dev/staging/prod)
const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';
const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
const USER_KEY = 'ppt_user';
/**
 * Persisted tenant membership list so refreshTokenInternal can call
 * deriveActiveRole and propagate server-side role promotions without
 * requiring a full re-login. Set on every login / SSO callback, cleared on
 * logout. See issue #574.
 */
const TENANTS_KEY = 'ppt_tenants';

// ============================================================================
// Context
// ============================================================================

const AuthContext = createContext<AuthContextValue | null>(null);

// ============================================================================
// Token Storage (localStorage for MVP, httpOnly cookies later)
// ============================================================================

const tokenStorage = {
  getAccessToken: (): string | null => {
    try {
      return localStorage.getItem(ACCESS_TOKEN_KEY);
    } catch {
      return null;
    }
  },

  setAccessToken: (token: string): void => {
    try {
      localStorage.setItem(ACCESS_TOKEN_KEY, token);
    } catch {
      // Storage unavailable
    }
  },

  getRefreshToken: (): string | null => {
    try {
      return localStorage.getItem(REFRESH_TOKEN_KEY);
    } catch {
      return null;
    }
  },

  setRefreshToken: (token: string): void => {
    try {
      localStorage.setItem(REFRESH_TOKEN_KEY, token);
    } catch {
      // Storage unavailable
    }
  },

  getUser: (): AuthUser | null => {
    try {
      const userJson = localStorage.getItem(USER_KEY);
      return userJson ? JSON.parse(userJson) : null;
    } catch {
      return null;
    }
  },

  setUser: (user: AuthUser): void => {
    try {
      localStorage.setItem(USER_KEY, JSON.stringify(user));
    } catch {
      // Storage unavailable
    }
  },

  getTenants: (): TenantMembership[] | null => {
    try {
      const raw = localStorage.getItem(TENANTS_KEY);
      return raw ? (JSON.parse(raw) as TenantMembership[]) : null;
    } catch {
      return null;
    }
  },

  setTenants: (tenants: TenantMembership[]): void => {
    try {
      localStorage.setItem(TENANTS_KEY, JSON.stringify(tenants));
    } catch {
      // Storage unavailable
    }
  },

  clear: (): void => {
    try {
      localStorage.removeItem(ACCESS_TOKEN_KEY);
      localStorage.removeItem(REFRESH_TOKEN_KEY);
      localStorage.removeItem(USER_KEY);
      localStorage.removeItem(TENANTS_KEY);
    } catch {
      // Storage unavailable
    }
  },
};

// ============================================================================
// Role derivation
// ============================================================================

/**
 * Privilege order for picking the "best" tenant role when the JWT does not
 * resolve to a specific tenant. Highest-privilege first.
 */
const ROLE_PRIORITY: readonly TenantRole[] = [
  'super_admin',
  'org_admin',
  'manager',
  'technical_manager',
  'property_manager',
  'real_estate_agent',
  'owner',
  'owner_delegate',
  'tenant',
  'resident',
  'guest',
] as const;

/** Best-effort decode of the unverified payload of a JWT. */
function decodeJwtPayload(token: string | null | undefined): Record<string, unknown> | null {
  if (!token) return null;
  const parts = token.split('.');
  if (parts.length < 2) return null;
  try {
    const b64 = parts[1].replace(/-/g, '+').replace(/_/g, '/');
    const padded = b64 + '==='.slice((b64.length + 3) % 4);
    const json = atob(padded);
    return JSON.parse(json) as Record<string, unknown>;
  } catch {
    return null;
  }
}

/**
 * Clock-skew / near-expiry buffer (seconds). An access token whose `exp` claim
 * is within this window of "now" is treated as already expired at cold boot, so
 * the init path routes through the silent refresh instead of marking the
 * session authenticated with a token the very next request would 401 on.
 */
const TOKEN_EXPIRY_SKEW_SECONDS = 30;

/**
 * True when the JWT is expired, or within TOKEN_EXPIRY_SKEW_SECONDS of expiring.
 *
 * Used at cold boot to decide whether a stored access token can be trusted to
 * mark the session authenticated, or whether we must route through the silent
 * refresh path first (avoiding an authenticated -> 401 -> refresh flicker).
 *
 * A token we cannot decode, or one that carries no numeric `exp` claim, is
 * treated as NOT expired here: we can't prove it is stale (it may be an opaque
 * token), and the runtime 401 interceptor remains the backstop for such cases.
 */
function isAccessTokenExpired(token: string | null | undefined): boolean {
  const claims = decodeJwtPayload(token);
  if (!claims) return false;
  const exp = claims.exp;
  if (typeof exp !== 'number') return false;
  const nowSeconds = Date.now() / 1000;
  return exp <= nowSeconds + TOKEN_EXPIRY_SKEW_SECONDS;
}

/**
 * Pick the membership matching `tenant_id` from the JWT; if that's missing,
 * fall back to the highest-privilege role across all memberships. Returns
 * `undefined` if the user has no memberships.
 */
export function deriveActiveRole(
  accessToken: string | null | undefined,
  tenants: TenantMembership[] | undefined
): TenantRole | undefined {
  if (!tenants || tenants.length === 0) return undefined;

  const claims = decodeJwtPayload(accessToken);
  const tenantId =
    claims && typeof claims.tenant_id === 'string' ? (claims.tenant_id as string) : null;
  if (tenantId) {
    const match = tenants.find((t) => t.tenantId === tenantId);
    if (match) return match.role;
  }

  // Embedded `role` claim wins next.
  if (claims && typeof claims.role === 'string') {
    const claimRole = claims.role as TenantRole;
    if (ROLE_PRIORITY.includes(claimRole)) return claimRole;
  }

  // Highest privilege available — preferable to insertion-order tenants[0].
  for (const role of ROLE_PRIORITY) {
    if (tenants.some((t) => t.role === role)) return role;
  }
  return tenants[0].role;
}

// ============================================================================
// API Client Instance
// ============================================================================

/**
 * Create a function that returns a configured auth API client.
 * The accessToken is dynamically retrieved from storage.
 */
const getAuthApi = () =>
  createAuthApi({
    baseUrl: API_BASE_URL,
    accessToken: tokenStorage.getAccessToken() ?? undefined,
  });

// ============================================================================
// Provider Component
// ============================================================================

interface AuthProviderProps {
  children: React.ReactNode;
}

/**
 * Authentication Provider Component.
 *
 * Wraps the application to provide authentication context.
 * Handles token storage, refresh, and session management.
 *
 * Integrates with @ppt/api-client's token provider to ensure
 * all API modules can access the current authentication token.
 */
export function AuthProvider({ children }: AuthProviderProps) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  // TanStack Query client — used to clear the cache on logout so stale
  // user-scoped data never leaks into the next session.
  const queryClient = useQueryClient();

  // Track if a token refresh is in progress to prevent concurrent refreshes
  const isRefreshing = useRef(false);
  const refreshPromise = useRef<Promise<string | null> | null>(null);

  // Holds the latest 401 handler so the configureApiClient effect below can be
  // registered exactly once (stable deps) yet always invoke the current logic.
  // `handleUnauthorized` closes over `refreshToken`/`logout`, whose identities
  // change across renders; routing through a ref avoids re-running the
  // configure effect (which would tear down and recreate the axios instance)
  // on every one of those changes.
  const onUnauthorizedRef = useRef<() => Promise<string | null>>(async () => null);

  // Derived state
  const isAuthenticated = user !== null;

  /**
   * Get the current access token.
   * This is also used as the token provider for all API modules.
   */
  const getAccessToken = useCallback((): string | null => {
    return tokenStorage.getAccessToken();
  }, []);

  /**
   * Set up the token provider for all API modules.
   * This ensures that all API calls automatically include the auth token.
   */
  useEffect(() => {
    // Register the token provider with the generated @ppt/api-client modules.
    setTokenProvider(getAccessToken);

    // Also configure the hand-rolled axios client in lib/api.ts. This is the
    // instance returned by getApiClient() and used directly by feature hooks
    // (sentiment, predictive-maintenance, …). Its request interceptor reads a
    // module-level token getter that is ONLY set via configureApiClient(); if
    // this call is missing, getApiClient() falls back to a default instance
    // with no token getter and every request goes out WITHOUT an Authorization
    // header. Wiring it here — alongside setTokenProvider — keeps a single
    // source of truth for the access token (getAccessToken).
    //
    // `onUnauthorized` fires from the response interceptor on a 401. It is
    // routed through `onUnauthorizedRef` (kept current by the effect below) so
    // this effect stays single-run: the actual handler attempts a silent token
    // refresh and, failing that, tears down the session so ProtectedRoute
    // redirects to login. Without this, the interceptor's 401 branch was dead
    // and getApiClient() consumers stayed stuck on an expired access token.
    configureApiClient({
      getToken: getAccessToken,
      onUnauthorized: () => onUnauthorizedRef.current(),
    });

    // Clean up on unmount
    return () => {
      clearTokenProvider();
      resetApiClient();
    };
  }, [getAccessToken]);

  // Register the active-org provider so the api-client's auth interceptor can
  // send X-Tenant-ID (#1522). Re-registered when the user/org changes so the
  // provider always reports the current organization.
  useEffect(() => {
    setOrgProvider(() => user?.organizationId ?? null);
    return () => {
      clearOrgProvider();
    };
  }, [user]);

  /**
   * Internal token refresh implementation using the API client.
   */
  const refreshTokenInternal = useCallback(async (): Promise<string | null> => {
    const refreshTokenValue = tokenStorage.getRefreshToken();
    if (!refreshTokenValue) {
      return null;
    }

    try {
      const authApi = getAuthApi();
      const response = await authApi.refreshToken({ refreshToken: refreshTokenValue });

      tokenStorage.setAccessToken(response.accessToken);
      tokenStorage.setRefreshToken(response.refreshToken);

      // Re-derive the role from the fresh access token using the persisted
      // tenant-membership list. Using deriveActiveRole (rather than reading
      // the role JWT claim directly) correctly handles multi-tenant users
      // whose active tenant changed on the server — the JWT tenant_id claim
      // selects the right membership, falling back to highest-privilege when
      // absent. See #574.
      const storedUser = tokenStorage.getUser();
      if (storedUser) {
        const storedTenants = tokenStorage.getTenants();
        const derivedRole = deriveActiveRole(response.accessToken, storedTenants ?? undefined);
        // Always re-hydrate `user` from the stored record (applying the freshly
        // derived role when one is available). Restoring the user here — rather
        // than only when a role could be derived — is what lets the cold-boot
        // init path funnel through this routine safely: it both refreshes the
        // role (#574) and re-authenticates the session in one place.
        const updated = derivedRole != null ? { ...storedUser, role: derivedRole } : storedUser;
        tokenStorage.setUser(updated);
        setUser(updated);
      }

      return response.accessToken;
    } catch (error) {
      // Refresh failed, clear auth state
      tokenStorage.clear();
      setUser(null);
      throw error;
    }
  }, []);

  /**
   * Initialize auth state from storage on mount.
   * Note: We intentionally only run this on mount to prevent loops.
   */
  useEffect(() => {
    const initializeAuth = async () => {
      try {
        const storedUser = tokenStorage.getUser();
        const accessToken = tokenStorage.getAccessToken();
        const refreshTokenValue = tokenStorage.getRefreshToken();

        if (storedUser && accessToken && !isAccessTokenExpired(accessToken)) {
          // Stored access token is present AND still valid (its `exp` claim is
          // in the future, past the clock-skew buffer). Safe to mark the
          // session authenticated without a network round-trip.
          setUser(storedUser);
        } else if (refreshTokenValue) {
          // Cold-boot refresh. Two cases funnel here:
          //   1. No live access token at all (tab closed past its lifetime, or
          //      the access token was cleared) — the classic cold boot.
          //   2. A stored access token that is expired or near-expiry — before
          //      this guard, initializeAuth marked the session authenticated
          //      from the stale token (the comment claimed an `exp` check that
          //      did not exist), so the first authenticated request 401'd and
          //      only THEN triggered a refresh: an authenticated -> 401 ->
          //      refresh flicker on every cold boot with an expired token.
          //      Routing through the silent refresh up front avoids it.
          //
          // Either way we go through refreshTokenInternal (the SAME routine the
          // runtime silent-refresh path uses) rather than calling
          // authApi.refreshToken inline: it re-derives the role from the fresh
          // access token + persisted tenant memberships (#574) and owns its own
          // failure cleanup (clear + de-auth + rethrow).
          try {
            await refreshTokenInternal();
          } catch {
            // refreshTokenInternal already cleared storage and reset the user.
          }
        } else if (accessToken) {
          // An expired/near-expiry access token with NO refresh token can't be
          // recovered — leave the session anonymous rather than authenticating
          // on a token the next request would 401 on. Purge the stale tokens.
          tokenStorage.clear();
        }
      } catch {
        // Clear any invalid stored data
        tokenStorage.clear();
      } finally {
        setIsLoading(false);
      }
    };

    initializeAuth();
    // refreshTokenInternal is a stable useCallback([]) reference, so this
    // effect still runs exactly once on mount (no re-run loop).
  }, [refreshTokenInternal]);

  /**
   * Refresh the access token with request queuing.
   * Prevents multiple concurrent refresh requests.
   */
  const refreshToken = useCallback(async (): Promise<string | null> => {
    // If already refreshing, return the existing promise
    if (isRefreshing.current && refreshPromise.current) {
      return refreshPromise.current;
    }

    isRefreshing.current = true;
    refreshPromise.current = refreshTokenInternal().finally(() => {
      isRefreshing.current = false;
      refreshPromise.current = null;
    });

    return refreshPromise.current;
  }, [refreshTokenInternal]);

  /**
   * Log in with email and password using the API client.
   */
  const login = useCallback(async (credentials: LoginCredentials): Promise<void> => {
    setIsLoading(true);

    try {
      const authApi = getAuthApi();
      const response = await authApi.login(credentials);

      // Derive role from the JWT `tenant_id` claim (or, failing that, the
      // highest-privilege membership) instead of `tenants[0]`. See #482.
      const derivedRole =
        response.user.role ?? deriveActiveRole(response.accessToken, response.tenants);
      const userWithRole: AuthUser =
        derivedRole != null ? { ...response.user, role: derivedRole } : response.user;

      // Store tokens, user, and tenant memberships.
      // Tenants are persisted so that refreshTokenInternal can call
      // deriveActiveRole on subsequent refreshes without a re-login (#574).
      tokenStorage.setAccessToken(response.accessToken);
      tokenStorage.setRefreshToken(response.refreshToken);
      tokenStorage.setUser(userWithRole);
      if (response.tenants) {
        tokenStorage.setTenants(response.tenants);
      }

      setUser(userWithRole);
      // Signup-funnel: the "first login" leg between email verification and
      // the onboarding tour (issue #2530). Fired only after tokens/user are
      // persisted and the session is authenticated.
      trackSignupLoggedIn('email_password');
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * Complete an SSO / OAuth callback flow.
   *
   * @param request - { code, state, redirectUri } from /auth/callback
   */
  const loginWithSsoCode = useCallback(async (request: SsoCallbackRequest): Promise<void> => {
    setIsLoading(true);

    try {
      const authApi = getAuthApi();
      const response = await authApi.exchangeSsoCode(request);

      const derivedRole =
        response.user.role ?? deriveActiveRole(response.accessToken, response.tenants);
      const userWithRole: AuthUser =
        derivedRole != null ? { ...response.user, role: derivedRole } : response.user;

      tokenStorage.setAccessToken(response.accessToken);
      tokenStorage.setRefreshToken(response.refreshToken);
      tokenStorage.setUser(userWithRole);
      // Persist tenant memberships for deriveActiveRole on subsequent
      // refreshes — mirrors the same persist step in login(). See #574.
      if (response.tenants) {
        tokenStorage.setTenants(response.tenants);
      }

      setUser(userWithRole);
    } catch (err) {
      // Roll back any partial writes so state is never incoherent.
      // Mirrors the cleanup pattern in logout() and refreshTokenInternal().
      tokenStorage.clear();
      setUser(null);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * Log out the current user using the API client.
   *
   * Session cleanup sequence:
   *  1. Clear localStorage tokens (immediate — prevents any further API calls
   *     from including a bearer token).
   *  2. Reset React state so the UI reflects the unauthenticated state.
   *  3. Purge session-scoped queries from the TanStack Query cache so
   *     user-bound data never leaks into the next session. We iterate the
   *     explicit `AUTHED_QUERY_KEY_ROOTS` list and remove each subtree —
   *     anything outside that list (router-internal caches, third-party
   *     library caches, future lookup tables) is left untouched.
   *  4. Best-effort server-side token revocation (fire-and-forget).
   *
   * @see Issue #712 — replaced `queryClient.clear()` with a scoped removal.
   */
  const logout = useCallback(async (): Promise<void> => {
    const refreshTokenValue = tokenStorage.getRefreshToken();

    // 1 & 2 — Clear local state first for immediate UI feedback.
    tokenStorage.clear();
    setUser(null);

    // 3 — Remove every auth-scoped query subtree. Each root is the first
    // segment of a query key (see lib/queryKeys.ts for the catalogue);
    // `removeQueries({ queryKey: [root] })` does a prefix match.
    for (const root of AUTHED_QUERY_KEY_ROOTS) {
      queryClient.removeQueries({ queryKey: [root] });
    }

    // 4 — Attempt to invalidate the refresh token on the server.
    if (refreshTokenValue) {
      try {
        const authApi = getAuthApi();
        await authApi.logout({ refreshToken: refreshTokenValue });
      } catch {
        // Ignore errors - we've already cleared local state
      }
    }
  }, [queryClient]);

  /**
   * Handle a 401 surfaced by the shared axios client (getApiClient). The
   * access token has expired or been revoked, so every in-flight feature-hook
   * request is failing. Recovery mirrors the intended `onUnauthorized` design:
   *
   *  - If a refresh token is available, perform a single silent refresh
   *    (deduplicated via `refreshToken`'s in-flight guard, so concurrent 401s
   *    all await one refresh round-trip) and return the rotated access token.
   *    The interceptor awaits this and replays the failed request once with the
   *    new token; on failure `refreshTokenInternal` has already cleared storage
   *    and de-authenticated the user, and we resolve `null` so the interceptor
   *    lets the original request reject.
   *  - If there is no refresh token, nothing can recover the session — log out
   *    so ProtectedRoute redirects to the login screen, and resolve `null`.
   *
   * Returns the rotated access token on success, or `null` when the session
   * could not be recovered.
   */
  const handleUnauthorized = useCallback(async (): Promise<string | null> => {
    if (tokenStorage.getRefreshToken()) {
      try {
        // Single-flight: concurrent 401s share the one in-flight refresh.
        return await refreshToken();
      } catch {
        // refreshTokenInternal already cleared storage + de-authenticated.
        return null;
      }
    }
    await logout();
    return null;
  }, [refreshToken, logout]);

  // Keep the ref read by the configureApiClient effect pointed at the current
  // handler, so a 401 always runs the latest refresh/logout closures without
  // re-registering the axios interceptors.
  useEffect(() => {
    onUnauthorizedRef.current = handleUnauthorized;
  }, [handleUnauthorized]);

  /**
   * Update the in-memory user and persist to storage. Lets profile-edit
   * surfaces refresh the cached `user` so the UI doesn't show stale data
   * until the next reload.
   */
  const updateUser = useCallback((next: AuthUser) => {
    tokenStorage.setUser(next);
    setUser(next);
  }, []);

  // Memoize the context value to prevent unnecessary re-renders
  const contextValue = useMemo<AuthContextValue>(
    () => ({
      user,
      isAuthenticated,
      isLoading,
      login,
      loginWithSsoCode,
      logout,
      refreshToken,
      getAccessToken,
      setUser: updateUser,
    }),
    [
      user,
      isAuthenticated,
      isLoading,
      login,
      loginWithSsoCode,
      logout,
      refreshToken,
      getAccessToken,
      updateUser,
    ]
  );

  return <AuthContext.Provider value={contextValue}>{children}</AuthContext.Provider>;
}

AuthProvider.displayName = 'AuthProvider';

// ============================================================================
// Hook
// ============================================================================

/**
 * Hook to access authentication context.
 *
 * @throws Error if used outside of AuthProvider
 * @returns The authentication context value
 *
 * @example
 * ```tsx
 * function MyComponent() {
 *   const { user, isAuthenticated, login, logout } = useAuth();
 *
 *   if (!isAuthenticated) {
 *     return <LoginForm onSubmit={login} />;
 *   }
 *
 *   return <div>Welcome, {user.firstName}!</div>;
 *   }
 * ```
 */
export function useAuth(): AuthContextValue {
  const context = useContext(AuthContext);

  if (!context) {
    throw new Error(
      'useAuth must be used within an AuthProvider. ' +
        'Ensure your component is wrapped in <AuthProvider>.'
    );
  }

  return context;
}
