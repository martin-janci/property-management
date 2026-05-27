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

import { getAndClearReturnUrl } from '@ppt/shared';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';
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
  const { loginWithSsoCode, isAuthenticated } = useAuth();

  const [status, setStatus] = useState<CallbackStatus>('pending');
  const [errorMessage, setErrorMessage] = useState<string>('');

  // Guard against double-invocation in React StrictMode.
  const exchangeStarted = useRef(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: intentional mount-only run; loginWithSsoCode/navigate are stable; searchParams changes every render
  useEffect(() => {
    // If already authenticated (e.g. browser back after successful SSO),
    // just redirect without re-exchanging.
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

    const exchange = async () => {
      try {
        await loginWithSsoCode({ code, state, redirectUri: getRedirectUri() });
        setStatus('success');
        const returnUrl = getAndClearReturnUrl();
        navigate(returnUrl || '/dashboard', { replace: true });
      } catch (err) {
        setStatus('error');
        setErrorMessage(
          err instanceof Error
            ? err.message
            : t('auth.ssoCallbackFailed', {
                defaultValue: 'SSO login failed. Please try again.',
              })
        );
      }
    };

    exchange();
  }, []);

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
