/**
 * Forgot Password Page (UC-14.4).
 *
 * Accepts an email and triggers a password-reset email via
 * authApi.requestPasswordReset.
 */

import { AuthError } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useState } from 'react';
import { Trans, useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { getAuthApi } from '../authApiClient';
import '../styles/AuthPage.css';

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function ForgotPasswordPage() {
  const { t } = useTranslation();
  const [email, setEmail] = useState('');
  const [emailError, setEmailError] = useState<string>();
  const [generalError, setGeneralError] = useState<string>();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  const handleSubmit = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setEmailError(undefined);
      setGeneralError(undefined);

      const trimmed = email.trim();
      if (!trimmed) {
        setEmailError(t('auth.forgotPassword.emailRequired'));
        return;
      }
      if (!EMAIL_RE.test(trimmed)) {
        setEmailError(t('auth.forgotPassword.invalidEmail'));
        return;
      }

      setIsSubmitting(true);
      try {
        await getAuthApi().requestPasswordReset({ email: trimmed });
        setSubmitted(true);
      } catch (error) {
        // We intentionally don't leak whether an email exists; show the
        // confirmation for most errors, but surface network errors.
        if (error instanceof AuthError && error.code === 'NETWORK_ERROR') {
          setGeneralError(t('auth.forgotPassword.networkError'));
        } else {
          setSubmitted(true);
        }
      } finally {
        setIsSubmitting(false);
      }
    },
    [email, t]
  );

  if (submitted) {
    return (
      <div className="auth-page">
        <div className="auth-container">
          <div className="auth-header">
            <h1 className="auth-title">{t('auth.forgotPassword.checkInboxTitle')}</h1>
            <p className="auth-subtitle">
              <Trans
                i18nKey="auth.forgotPassword.checkInboxSubtitle"
                values={{ email }}
                components={{ 1: <strong /> }}
              />
            </p>
          </div>
          <div className="auth-links">
            <Link to="/login" className="auth-link">
              {t('auth.forgotPassword.backToSignIn')}
            </Link>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="auth-page">
      <div className="auth-container">
        <div className="auth-header">
          <h1 className="auth-title">{t('auth.forgotPassword.title')}</h1>
          <p className="auth-subtitle">{t('auth.forgotPassword.subtitle')}</p>
        </div>

        <form className="auth-form" onSubmit={handleSubmit} noValidate>
          {generalError && (
            <div className="auth-error-banner" role="alert" aria-live="polite">
              <span aria-hidden="true">!</span>
              <span>{generalError}</span>
            </div>
          )}

          <div className="auth-field">
            <label htmlFor="email" className="auth-label">
              {t('auth.forgotPassword.email')}
            </label>
            <input
              id="email"
              name="email"
              type="email"
              autoComplete="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              disabled={isSubmitting}
              className={`auth-input ${emailError ? 'auth-input--error' : ''}`}
              aria-invalid={emailError ? 'true' : 'false'}
            />
            {emailError && <span className="auth-field-error">{emailError}</span>}
          </div>

          <button type="submit" className="auth-submit" disabled={isSubmitting}>
            {isSubmitting ? (
              <>
                <span className="auth-spinner" aria-hidden="true" />
                <span>{t('auth.forgotPassword.submitting')}</span>
              </>
            ) : (
              t('auth.forgotPassword.submit')
            )}
          </button>
        </form>

        <div className="auth-links">
          <Link to="/login" className="auth-link">
            {t('auth.forgotPassword.backToSignIn')}
          </Link>
        </div>
      </div>
    </div>
  );
}

ForgotPasswordPage.displayName = 'ForgotPasswordPage';
