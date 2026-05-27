/**
 * OAuth Callback Page (Story gap-79-2)
 *
 * Handles the redirect from the OAuth authorization server after SSO / OAuth:
 *   GET /auth/callback?code=<code>&state=<state>
 *
 * Flow:
 *   1. Extract `code` and `state` from the URL search params.
 *   2. Exchange the authorization code for tokens via
 *      POST /api/v1/auth/callback.
 *   3. Store access + refresh tokens in localStorage (same keys as
 *      AuthContext so the token provider picks them up automatically).
 *   4. Redirect to /dashboard on success.
 *   5. On error (missing code, OAuth error param, exchange failure) redirect
 *      to /login?error=<message>.
 *
 * Expired token on arrival: if the returned access token is already past its
 * `exp` claim (clock-skew edge case), the page calls AuthContext.refreshToken()
 * before navigating so the first protected API call does not immediately 401.
 */

import { useEffect, useRef, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useAuth } from '../../../contexts/AuthContext';

// Must match the key constants in AuthContext.tokenStorage.
const ACCESS_TOKEN_KEY = 'ppt_access_token';
const REFRESH_TOKEN_KEY = 'ppt_refresh_token';
const USER_KEY = 'ppt_user';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

interface OAuthCallbackApiResponse {
  accessToken: string;
  refreshToken: string;
  user?: {
    id: string;
    email: string;
    firstName?: string;
    lastName?: string;
    role?: string;
  };
}

type CallbackStatus = 'loading' | 'error';

/**
 * OAuthCallbackPage
 *
 * Mounted at `/auth/callback`. Must be a **public** route — the user is not
 * yet authenticated when the browser lands here.
 */
export function OAuthCallbackPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const { refreshToken } = useAuth();
  const [status, setStatus] = useState<CallbackStatus>('loading');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Guard against React StrictMode double-mount invoking the exchange twice.
  const exchanged = useRef(false);

  useEffect(() => {
    if (exchanged.current) return;
    exchanged.current = true;

    const code = searchParams.get('code');
    const state = searchParams.get('state');
    const oauthError = searchParams.get('error');

    // OAuth error param sent by the authorization server.
    if (oauthError) {
      const desc = searchParams.get('error_description') ?? oauthError;
      setStatus('error');
      setErrorMessage(desc);
      navigate(`/login?error=${encodeURIComponent(desc)}`, { replace: true });
      return;
    }

    if (!code) {
      const msg = 'Missing authorization code';
      setStatus('error');
      setErrorMessage(msg);
      navigate(`/login?error=${encodeURIComponent(msg)}`, { replace: true });
      return;
    }

    const exchangeCode = async () => {
      try {
        const resp = await fetch(`${API_BASE_URL}/api/v1/auth/callback`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ code, ...(state ? { state } : {}) }),
        });

        if (!resp.ok) {
          const body = await resp.json().catch(() => ({}));
          const msg: string =
            (body as { message?: string }).message ?? `Authentication failed (${resp.status})`;
          throw new Error(msg);
        }

        const data = (await resp.json()) as OAuthCallbackApiResponse;

        // Persist tokens so the token provider and AuthContext pick them up.
        try {
          localStorage.setItem(ACCESS_TOKEN_KEY, data.accessToken);
          localStorage.setItem(REFRESH_TOKEN_KEY, data.refreshToken);
          if (data.user) {
            localStorage.setItem(USER_KEY, JSON.stringify(data.user));
          }
        } catch {
          // localStorage unavailable; session will last until the tab is closed.
        }

        // If the returned access token is already expired (clock-skew edge
        // case) trigger a silent refresh before navigating to /dashboard so the
        // first protected API call does not immediately 401.
        try {
          const parts = data.accessToken.split('.');
          if (parts.length === 3) {
            const payload = JSON.parse(atob(parts[1].replace(/-/g, '+').replace(/_/g, '/'))) as {
              exp?: number;
            };
            if (payload.exp && Date.now() > payload.exp * 1000) {
              await refreshToken();
            }
          }
        } catch {
          // Malformed JWT — proceed; AuthContext will refresh on the next 401.
        }

        navigate('/dashboard', { replace: true });
      } catch (err) {
        const msg = err instanceof Error ? err.message : 'Authentication failed';
        setStatus('error');
        setErrorMessage(msg);
        navigate(`/login?error=${encodeURIComponent(msg)}`, { replace: true });
      }
    };

    exchangeCode();
  }, [searchParams, navigate, refreshToken]);

  // The page is ephemeral — visible only for the network round-trip duration.
  if (status === 'error') {
    return (
      <main aria-live="polite" data-testid="oauth-callback-error">
        <p>Authentication failed{errorMessage ? `: ${errorMessage}` : ''}.</p>
        <p>Redirecting to login...</p>
      </main>
    );
  }

  return (
    <main aria-live="polite" data-testid="oauth-callback-loading">
      <p>Completing sign-in, please wait...</p>
    </main>
  );
}
