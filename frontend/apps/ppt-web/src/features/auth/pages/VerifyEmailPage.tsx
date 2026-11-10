/**
 * Verify Email Page (UC-14.1 confirmation step).
 *
 * Landing page for the link in the verification e-mail
 * (`{APP_BASE_URL}/verify-email?token=…`). It confirms the token once via
 * `GET /api/v1/auth/verify-email` and, on failure, lets the user request a
 * fresh link via `POST /api/v1/auth/resend-verification`.
 */

import { AuthError } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useSearchParams } from 'react-router-dom';
import { getAuthApi } from '../authApiClient';
import '../styles/AuthPage.css';

type Status = 'verifying' | 'success' | 'expired' | 'invalid' | 'error';

/** Maps a verification failure to a page status. */
export function statusFromError(error: unknown): Status {
  if (error instanceof AuthError) {
    // The server answers INVALID_TOKEN / TOKEN_EXPIRED; TOKEN_INVALID is the
    // client-side spelling of the same thing.
    const code = error.code as string;
    if (code === 'TOKEN_EXPIRED') return 'expired';
    if (code === 'INVALID_TOKEN' || code === 'TOKEN_INVALID') return 'invalid';
  }
  return 'error';
}

export function VerifyEmailPage() {
  const { t } = useTranslation();
  const [searchParams] = useSearchParams();
  const token = searchParams.get('token') ?? '';
  const [status, setStatus] = useState<Status>(token ? 'verifying' : 'invalid');
  const [email, setEmail] = useState('');
  const [resendState, setResendState] = useState<'idle' | 'sending' | 'sent' | 'failed'>('idle');
  // Tokens are single-use: guard against React StrictMode's double effect run,
  // which would otherwise consume the token and then report it as invalid.
  const attempted = useRef<string | null>(null);

  useEffect(() => {
    if (!token || attempted.current === token) return;
    attempted.current = token;
    getAuthApi()
      .verifyEmail(token)
      .then(() => setStatus('success'))
      .catch((error: unknown) => setStatus(statusFromError(error)));
  }, [token]);

  const handleResend = useCallback(
    async (event: React.FormEvent) => {
      event.preventDefault();
      if (!email.trim()) return;
      setResendState('sending');
      try {
        await getAuthApi().resendVerification(email.trim());
        setResendState('sent');
      } catch {
        setResendState('failed');
      }
    },
    [email]
  );

  if (status === 'verifying') {
    return (
      <div className="auth-page">
        <div className="auth-container" role="status" aria-live="polite" aria-busy="true">
          <div className="auth-header">
            <h1 className="auth-title">{t('auth.verifyEmail.verifyingTitle')}</h1>
            <p className="auth-subtitle">{t('auth.verifyEmail.verifyingSubtitle')}</p>
          </div>
        </div>
      </div>
    );
  }

  if (status === 'success') {
    return (
      <div className="auth-page">
        <div className="auth-container">
          <div className="auth-header">
            <h1 className="auth-title">{t('auth.verifyEmail.successTitle')}</h1>
            <p className="auth-subtitle">{t('auth.verifyEmail.successSubtitle')}</p>
          </div>
          <Link to="/login" className="auth-submit">
            {t('auth.verifyEmail.signIn')}
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="auth-page">
      <div className="auth-container">
        <div className="auth-header">
          <h1 className="auth-title">{t('auth.verifyEmail.failedTitle')}</h1>
          <p className="auth-subtitle">{t(`auth.verifyEmail.${status}`)}</p>
        </div>

        {resendState === 'sent' ? (
          <div className="auth-success-banner" role="status" aria-live="polite">
            <span>{t('auth.verifyEmail.resendSent')}</span>
          </div>
        ) : (
          <form className="auth-form" onSubmit={handleResend} noValidate>
            {resendState === 'failed' && (
              <div className="auth-error-banner" role="alert" aria-live="polite">
                <span>{t('auth.verifyEmail.resendFailed')}</span>
              </div>
            )}
            <div className="auth-field">
              <label htmlFor="verify-email-address" className="auth-label">
                {t('auth.emailAddress')}
              </label>
              <input
                id="verify-email-address"
                type="email"
                autoComplete="email"
                className="auth-input"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                required
              />
            </div>
            <button
              type="submit"
              className="auth-submit"
              disabled={resendState === 'sending' || !email.trim()}
            >
              {resendState === 'sending'
                ? t('auth.verifyEmail.resending')
                : t('auth.verifyEmail.resend')}
            </button>
          </form>
        )}

        <div className="auth-links">
          <Link to="/login" className="auth-link">
            {t('auth.backToLogin')}
          </Link>
        </div>
      </div>
    </div>
  );
}
