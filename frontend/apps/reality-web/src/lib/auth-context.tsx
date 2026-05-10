'use client';

/**
 * Authentication context for Reality Portal SSO (Epic 10A-SSO).
 * Manages user session state from SSO with Property Management system.
 */

import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';
import { clearStoredSession } from './auth-api';
import { getAuthHeader, getSession } from './auth-token';
import { getApiBase } from './env';

/** User information from SSO. */
export interface SsoUser {
  user_id: string;
  email: string;
  name: string;
  avatar_url?: string;
}

/** Session information. */
export interface SessionInfo {
  user_id: string;
  email: string;
  name: string;
  expires_at: string;
}

/** Auth context state. */
interface AuthContextValue {
  /** Current user or null if not authenticated */
  user: SsoUser | null;
  /** Whether auth state is loading */
  isLoading: boolean;
  /** Whether user is authenticated */
  isAuthenticated: boolean;
  /** Initiate SSO login */
  login: (redirectUri?: string) => void;
  /** Logout from current session */
  logout: () => Promise<void>;
  /** Refresh session info */
  refreshSession: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

/** Auth provider props. */
interface AuthProviderProps {
  children: ReactNode;
}

/** Auth provider component. */
export function AuthProvider({ children }: AuthProviderProps) {
  // Hydrate optimistically from localStorage so the header doesn't
  // momentarily render the "Sign in" button between mount and the first
  // /sso/session fetch. The async checkSession below corrects this if
  // the stored token has been revoked server-side.
  const [user, setUser] = useState<SsoUser | null>(() => {
    const s = getSession();
    return s ? { user_id: s.user.id, email: s.user.email, name: s.user.name } : null;
  });
  const [isLoading, setIsLoading] = useState(true);

  const checkSession = useCallback(async () => {
    // Bearer-token path (form-login): /users/me requires Authorization
    // header which getAuthHeader() reads from localStorage.
    const stored = getSession();
    if (stored) {
      try {
        const meResp = await fetch(`${getApiBase()}/api/v1/users/me`, {
          credentials: 'include',
          headers: { ...getAuthHeader() },
        });
        if (meResp.ok) {
          const me: { id: string; email: string; name: string } = await meResp.json();
          setUser({ user_id: me.id, email: me.email, name: me.name });
          setIsLoading(false);
          return;
        }
        // Token rejected — wipe localStorage; fall through to cookie path.
        clearStoredSession();
      } catch {
        // Network blip — keep optimistic user; cookie path may still work.
      }
    }

    try {
      const response = await fetch(`${getApiBase()}/api/v1/sso/session`, {
        credentials: 'include',
      });

      if (response.ok) {
        const session: SessionInfo = await response.json();
        setUser({
          user_id: session.user_id,
          email: session.email,
          name: session.name,
        });
      } else {
        setUser(null);
      }
    } catch {
      setUser(null);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Check session on mount
  useEffect(() => {
    checkSession();
  }, [checkSession]);

  const login = useCallback((redirectUri?: string) => {
    // Send the user to the email/password form. The OAuth/SSO redirect
    // (`${getApiBase()}/api/v1/sso/login`) is left in place at the
    // backend, but the consent UI on api-server hasn't shipped — hitting
    // /oauth/authorize ends up on a JSON consent page instead of a real
    // login screen. The form-login flow at /[locale]/auth/login posts to
    // /api/v1/users/login, persists the bearer token via auth-token, and
    // returns to redirectUri (defaulting to /).
    //
    // Locale is taken from the current URL's first segment so the user
    // stays in their language; falls back to /sk because that's the
    // primary audience and matches the route layout default.
    const locale = (() => {
      const seg = window.location.pathname.split('/').filter(Boolean)[0];
      return /^[a-z]{2}$/.test(seg ?? '') ? seg : 'sk';
    })();
    const params = new URLSearchParams();
    if (redirectUri) {
      params.set('redirect', redirectUri);
    }
    const qs = params.toString();
    window.location.href = `/${locale}/auth/login${qs ? `?${qs}` : ''}`;
  }, []);

  const logout = useCallback(async () => {
    // Best-effort: tell the server (covers both bearer + cookie sessions).
    try {
      await fetch(`${getApiBase()}/api/v1/sso/logout`, {
        method: 'POST',
        credentials: 'include',
        headers: { ...getAuthHeader() },
      });
    } catch {
      // Ignore — local cleanup must still happen.
    }
    // Always clear local state regardless of network outcome.
    clearStoredSession();
    setUser(null);
  }, []);

  const refreshSession = useCallback(async () => {
    try {
      const response = await fetch(`${getApiBase()}/api/v1/sso/refresh`, {
        method: 'POST',
        credentials: 'include',
      });

      if (response.ok) {
        const session: SessionInfo = await response.json();
        setUser({
          user_id: session.user_id,
          email: session.email,
          name: session.name,
        });
      }
    } catch {
      // Ignore refresh errors
    }
  }, []);

  const value = useMemo(
    () => ({
      user,
      isLoading,
      isAuthenticated: user !== null,
      login,
      logout,
      refreshSession,
    }),
    [user, isLoading, login, logout, refreshSession]
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

/** Hook to access auth context. */
export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
