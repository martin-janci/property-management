/**
 * OAuth / SSO Callback Page.
 *
 * Handles the redirect from an SSO provider after the user consents.
 * Reads the `code` and `state` query params, calls
 * AuthContext.loginWithSsoCode() to exchange them for PPT JWT tokens,
 * stores the tokens via tokenProvider, and redirects to the dashboard
 * (or any stored return URL).
 *
 * Route: /auth/callback
 *
 * @see Story 79.2 - Authentication Flow Implementation
 * @see gap-79-2-auth-sso-completion
 */

import { getAndClearReturnUrl, getAndClearSsoState } from '@ppt/shared';
import type { TFunction } from 'i18next';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { AuthError, useAuth } from '../contexts/AuthContext';
import './LoginPage.css';

// ============================================================================
// Types
// ============================================================================

type CallbackStatus = 'pending' | 'success' | 'error';

// ============================================================================
// Helpers
// ============================================================================

/**
 * Derive the redirect_uri that was registered with the SSO provider.
 * Must match what the backend used when redirecting the user.
 */
function getRedirectUri(): string {
  return `${window.location.origin}/auth/callback`;
}

/**
 * Map an SSO code-exchange failure to a user-facing, translated message.
 *
 * SECURITY / i18n: never surface a raw `Error.message` to the user. On the
 * primary SSO auth surface that leaks internal/network detail (upstream host,
 * stack frames, HTTP bodies) and renders untranslated English on sk/cs/de/pl/hu
 * locales. We map known `AuthError` codes to existing translated keys and fall
 * back to the generic `auth.ssoCallbackFailed` for everything else.
 *
 * Mirrors the `getErrorMessage` convention in `LoginPage.tsx`
 * (code-review-ppt-web-ui-login-raw-message), minus the `return error.message`
 * branch — the whole point here is to never echo it.
 */
function getSsoErrorMessage(error: unknown, t: TFunction): string {
  const genericFallback = t('auth.ssoCallbackFailed', {
    defaultValue: 'SSO login failed. Please try again.',
  });

  if (error instanceof AuthError) {
    switch (error.code) {
      case 'INVALID_CREDENTIALS':
        return t('auth.invalidCredentials');
      case 'ACCOUNT_LOCKED':
        return t('auth.accountLocked');
      case 'ACCOUNT_DISABLED':
        return t('auth.accountDisabled');
      case 'SESSION_EXPIRED':
      case 'TOKEN_EXPIRED':
      case 'TOKEN_INVALID':
        return t('auth.sessionExpired');
      case 'NETWORK_ERROR':
        return t('auth.networkError');
      default:
        return genericFallback;
    }
  }

  // A non-AuthError rejection (thrown Error, network layer, etc.). Detect the
  // common network-failure shape so the user gets the actionable connectivity
  // message, but still never echo the raw `message` for anything else.
  if (
    error instanceof Error &&
    (error.message.includes('fetch') || error.message.includes('network'))
  ) {
    return t('auth.networkError');
  }

  return genericFallback;
}

// ============================================================================
// Component
// ============================================================================

/**
 * AuthCallbackPage.
 *
 * Lifecycle:
 *  1. Read `code` + `state` from URL search params.
 *  2. Validate both params are present; show error if not.
 *  3. Call AuthContext.loginWithSsoCode() which calls
 *     /api/v1/auth/sso/callback, stores access + refresh tokens via
 *     tokenProvider, and sets the authenticated user.
 *  4. Redirect to return URL (or /dashboard) on success.
 *  5. Show user-friendly error and a "back to login" link on failure.
 */
export function AuthCallbackPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { loginWithSsoCode, isAuthenticated, isLoading } = useAuth();

  const [status, setStatus] = useState<CallbackStatus>('pending');
  const [errorMessage, setErrorMessage] = useState<string>('');

  // Guard against double-invocation in React StrictMode.
  const exchangeStarted = useRef(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: loginWithSsoCode/navigate are stable; searchParams changes every render; we deliberately re-run when isLoading flips false so the exchange waits for hydration
  useEffect(() => {
    // Wait for AuthProvider to finish hydrating stored session state (#751).
    // Running before hydration completes risks treating an already-authenticated
    // user as a fresh callback (isAuthenticated still false) and, after the
    // one-shot state-nonce check below clears the nonce, surfacing a spurious
    // "state mismatch" error instead of just redirecting.
    if (isLoading) return;

    // If already authenticated (e.g. browser back after successful SSO,
    // or a valid stored session restored during hydration), just redirect
    // without re-exchanging.
    if (isAuthenticated) {
      const returnUrl = getAndClearReturnUrl();
      navigate(returnUrl || '/dashboard', { replace: true });
      return;
    }

    if (exchangeStarted.current) return;
    exchangeStarted.current = true;

    const code = searchParams.get('code');
    const state = searchParams.get('state');
    const errorParam = searchParams.get('error');
    const errorDescription = searchParams.get('error_description');

    // The SSO provider itself may return an error in the redirect.
    if (errorParam) {
      setStatus('error');
      setErrorMessage(
        errorDescription
          ? decodeURIComponent(errorDescription)
          : t('auth.ssoProviderError', { defaultValue: 'The identity provider returned an error.' })
      );
      return;
    }

    if (!code || !state) {
      setStatus('error');
      setErrorMessage(
        t('auth.callbackMissingParams', {
          defaultValue: 'Invalid callback URL: missing code or state parameters.',
        })
      );
      return;
    }

    // OIDC §3.1.2.7 — verify client-side state nonce BEFORE any network call.
    // getAndClearSsoState() atomically reads + removes the nonce so it cannot
    // be replayed even when the comparison fails. If the stored nonce is absent
    // (direct navigation, tab-napping, or replay attempt) we abort immediately.
    const storedState = getAndClearSsoState();
    if (!storedState || storedState !== state) {
      setStatus('error');
      setErrorMessage(
        t('auth.stateValidationFailed', {
          defaultValue:
            'Authentication failed: state parameter mismatch. Please try signing in again.',
        })
      );
      return;
    }

    const exchange = async () => {
      try {
        await loginWithSsoCode({ code, state, redirectUri: getRedirectUri() });
        setStatus('success');
        const returnUrl = getAndClearReturnUrl();
        navigate(returnUrl || '/dashboard', { replace: true });
      } catch (err) {
        setStatus('error');
        // Never render `err.message` verbatim — map it to a translated,
        // user-safe message (code-review-ppt-web-ui-authcallback-raw-err-message).
        setErrorMessage(getSsoErrorMessage(err, t));
      }
    };

    exchange();
  }, [isLoading, isAuthenticated]);

  // ── Pending state ─────────────────────────────────────────────────────────
  if (status === 'pending') {
    return (
      <div className="login-page">
        <div className="login-loading" role="status" aria-live="polite" aria-busy="true">
          <div className="login-spinner" aria-hidden="true" />
          <p className="login-subtitle">
            {t('auth.completingSignIn', { defaultValue: 'Completing sign-in…' })}
          </p>
        </div>
      </div>
    );
  }

  // ── Success state (redirect in flight) ────────────────────────────────────
  if (status === 'success') {
    return (
      <div className="login-page">
        <div className="login-loading" role="status" aria-live="polite">
          <div className="login-spinner" aria-hidden="true" />
          <p className="login-subtitle">
            {t('auth.redirecting', { defaultValue: 'Redirecting…' })}
          </p>
        </div>
      </div>
    );
  }

  // ── Error state ───────────────────────────────────────────────────────────
  return (
    <div className="login-page">
      <div className="login-container">
        <div className="login-header">
          <h1 className="login-title">
            {t('auth.signInFailed', { defaultValue: 'Sign-in failed' })}
          </h1>
        </div>

        <div className="login-error-banner" role="alert">
          <span className="login-error-icon" aria-hidden="true">
            !
          </span>
          <span>{errorMessage}</span>
        </div>

        <div className="login-form">
          <button
            type="button"
            className="login-submit"
            onClick={() => navigate('/login', { replace: true })}
          >
            {t('auth.backToLogin', { defaultValue: 'Back to login' })}
          </button>
        </div>
      </div>
    </div>
  );
}

AuthCallbackPage.displayName = 'AuthCallbackPage';
