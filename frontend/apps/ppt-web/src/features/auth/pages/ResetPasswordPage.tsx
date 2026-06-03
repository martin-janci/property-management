/**
 * Reset Password Page (UC-14.4 confirmation step).
 *
 * Reads `token` from the query string and submits a new password via
 * authApi.resetPassword.
 */

import { AuthError } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate, useSearchParams } from 'react-router-dom';
import { getAuthApi } from '../authApiClient';
import '../styles/AuthPage.css';

const MIN_PASSWORD_LENGTH = 8;

interface FormErrors {
  password?: string;
  confirmPassword?: string;
  general?: string;
}

/** Maps an error to an i18n key relative to the `auth.resetPassword` namespace. */
function getErrorKey(error: unknown): string {
  if (error instanceof AuthError) {
    switch (error.code) {
      case 'TOKEN_EXPIRED':
        return 'tokenExpired';
      case 'TOKEN_INVALID':
        return 'tokenInvalid';
      case 'WEAK_PASSWORD':
        return 'weakPassword';
      case 'NETWORK_ERROR':
        return 'networkError';
      default:
        return 'resetFailed';
    }
  }
  return 'unexpectedError';
}

export function ResetPasswordPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const token = useMemo(() => searchParams.get('token') ?? '', [searchParams]);

  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [errors, setErrors] = useState<FormErrors>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  const handleSubmit = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setErrors({});

      if (!token) {
        setErrors({ general: 'missingToken' });
        return;
      }
      const next: FormErrors = {};
      if (!password) next.password = 'passwordRequired';
      else if (password.length < MIN_PASSWORD_LENGTH) next.password = 'passwordTooShort';
      if (confirmPassword !== password) next.confirmPassword = 'passwordsDoNotMatch';
      if (Object.keys(next).length > 0) {
        setErrors(next);
        return;
      }

      setIsSubmitting(true);
      try {
        await getAuthApi().resetPassword({ token, newPassword: password });
        setSubmitted(true);
      } catch (error) {
        setErrors({ general: getErrorKey(error) });
      } finally {
        setIsSubmitting(false);
      }
    },
    [token, password, confirmPassword]
  );

  /** Resolves a field/general error key into a localized message. */
  const errorText = (key?: string) =>
    key ? t(`auth.resetPassword.${key}`, { count: MIN_PASSWORD_LENGTH }) : '';

  if (submitted) {
    return (
      <div className="auth-page">
        <div className="auth-container">
          <div className="auth-header">
            <h1 className="auth-title">{t('auth.resetPassword.successTitle')}</h1>
            <p className="auth-subtitle">{t('auth.resetPassword.successSubtitle')}</p>
          </div>
          <button
            type="button"
            className="auth-submit"
            onClick={() => navigate('/login', { replace: true })}
          >
            {t('auth.resetPassword.signIn')}
          </button>
        </div>
      </div>
    );
  }

  if (!token) {
    return (
      <div className="auth-page">
        <div className="auth-container">
          <div className="auth-header">
            <h1 className="auth-title">{t('auth.resetPassword.invalidLinkTitle')}</h1>
            <p className="auth-subtitle">{t('auth.resetPassword.invalidLinkSubtitle')}</p>
          </div>
          <div className="auth-links">
            <Link to="/forgot-password" className="auth-link">
              {t('auth.resetPassword.requestNewLink')}
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
          <h1 className="auth-title">{t('auth.resetPassword.title')}</h1>
          <p className="auth-subtitle">{t('auth.resetPassword.subtitle')}</p>
        </div>

        <form className="auth-form" onSubmit={handleSubmit} noValidate>
          {errors.general && (
            <div className="auth-error-banner" role="alert" aria-live="polite">
              <span aria-hidden="true">!</span>
              <span>{errorText(errors.general)}</span>
            </div>
          )}

          <div className="auth-field">
            <label htmlFor="password" className="auth-label">
              {t('auth.resetPassword.newPassword')}
            </label>
            <input
              id="password"
              name="password"
              type="password"
              autoComplete="new-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              disabled={isSubmitting}
              className={`auth-input ${errors.password ? 'auth-input--error' : ''}`}
              aria-invalid={errors.password ? 'true' : 'false'}
            />
            {errors.password ? (
              <span className="auth-field-error">{errorText(errors.password)}</span>
            ) : (
              <span className="auth-help">
                {t('auth.resetPassword.passwordHelp', { count: MIN_PASSWORD_LENGTH })}
              </span>
            )}
          </div>

          <div className="auth-field">
            <label htmlFor="confirmPassword" className="auth-label">
              {t('auth.resetPassword.confirmPassword')}
            </label>
            <input
              id="confirmPassword"
              name="confirmPassword"
              type="password"
              autoComplete="new-password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              disabled={isSubmitting}
              className={`auth-input ${errors.confirmPassword ? 'auth-input--error' : ''}`}
              aria-invalid={errors.confirmPassword ? 'true' : 'false'}
            />
            {errors.confirmPassword && (
              <span className="auth-field-error">{errorText(errors.confirmPassword)}</span>
            )}
          </div>

          <button type="submit" className="auth-submit" disabled={isSubmitting}>
            {isSubmitting ? (
              <>
                <span className="auth-spinner" aria-hidden="true" />
                <span>{t('auth.resetPassword.submitting')}</span>
              </>
            ) : (
              t('auth.resetPassword.submit')
            )}
          </button>
        </form>
      </div>
    </div>
  );
}

ResetPasswordPage.displayName = 'ResetPasswordPage';
