'use client';

/**
 * Authentication context for Reality Portal SSO (Epic 10A-SSO).
 * Manages user session state from SSO with Property Management system.
 */

import {
  type ReactNode,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react';
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
  const [user, setUser] = useState<SsoUser | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const checkSession = useCallback(async () => {
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
    const params = new URLSearchParams();
    if (redirectUri) {
      params.set('redirect_uri', redirectUri);
    }
    // Generate CSRF state token
    const state = crypto.randomUUID();
    sessionStorage.setItem('sso_state', state);
    params.set('state', state);

    window.location.href = `${getApiBase()}/api/v1/sso/login?${params.toString()}`;
  }, []);

  const logout = useCallback(async () => {
    try {
      await fetch(`${getApiBase()}/api/v1/sso/logout`, {
        method: 'POST',
        credentials: 'include',
      });
    } finally {
      setUser(null);
    }
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
