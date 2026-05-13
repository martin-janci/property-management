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

// Module-level dedup for the unauthenticated /sso/session probe. In dev,
// React StrictMode double-fires the mount effect and Next.js can remount
// the locale layout once after hydration, which together produced four
// network hits to /sso/session per pageload (each surfacing as a 401
// console error for anonymous users). With this guard, concurrent callers
// share the same in-flight Response and a short result cache hands back
// the *parsed* outcome (ok + session, or unauth) so a burst of remounts
// within 2 s doesn't issue another request *and* doesn't get misread as
// "logged out" the way an early `null` return did.
//
// (The previous version returned `null` while the cache was warm; that
// confused checkSession() into calling setUser(null) on every remount,
// which could blink a logged-in user back to anonymous. Copilot review
// caught this.)
interface ProbeResult {
  ok: boolean;
  session?: SessionInfo;
}
let _inflightSessionProbe: Promise<ProbeResult> | null = null;
let _cachedProbeResult: ProbeResult | null = null;
let _cachedProbeAt = 0;
const PROBE_CACHE_MS = 2000;

async function probeSsoSession(apiBase: string): Promise<ProbeResult> {
  const now = Date.now();
  if (_inflightSessionProbe) return _inflightSessionProbe;
  if (_cachedProbeResult && now - _cachedProbeAt < PROBE_CACHE_MS) {
    return _cachedProbeResult;
  }
  _inflightSessionProbe = (async () => {
    try {
      const response = await fetch(`${apiBase}/api/v1/sso/session`, {
        credentials: 'include',
      });
      if (response.ok) {
        const session = (await response.json()) as SessionInfo;
        return { ok: true, session };
      }
      return { ok: false };
    } catch {
      // Network blip — treat as transient, NOT as "logged out". Don't cache
      // so the next mount can retry; return ok=false so the current caller
      // doesn't clobber any pre-existing optimistic state.
      return { ok: false };
    }
  })().finally(() => {
    _cachedProbeAt = Date.now();
    _inflightSessionProbe = null;
  });
  const result = await _inflightSessionProbe;
  _cachedProbeResult = result;
  return result;
}

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
  // Start `true` only when there's no stored bearer session — that's the
  // case where consumers (e.g. ProtectedRoute) need to wait for the cookie
  // /sso/session check before they decide between rendering "Sign in
  // required" and the gated content. With a stored session we already
  // have an optimistic user, so loading is effectively done. The
  // checkSession() below flips this to `false` in its finally.
  const [isLoading, setIsLoading] = useState(() => getSession() === null);

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

    const result = await probeSsoSession(getApiBase());
    if (result.ok && result.session) {
      const { session } = result;
      setUser({
        user_id: session.user_id,
        email: session.email,
        name: session.name,
      });
    } else {
      setUser(null);
    }
    setIsLoading(false);
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
    // Bearer-token path first — covers form-login, where the page just
    // wrote a new token to localStorage and needs the header to update
    // without a hard reload.
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
          return;
        }
        // Definitive auth failure — drop the stale token so we don't keep
        // sending it on subsequent fetches. Any other status (5xx, etc.)
        // is treated as transient: leave optimistic state intact and try
        // the cookie path below.
        if (meResp.status === 401 || meResp.status === 403) {
          clearStoredSession();
          setUser(null);
        }
      } catch {
        // Network blip — fall through to cookie path; keep optimistic state.
      }
    }

    // Cookie/SSO path (kept intact for the eventual end-to-end OAuth flow).
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
