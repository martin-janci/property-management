/**
 * Change Password Page (UC-14.5).
 *
 * Authenticated user updates their password by providing the current and a
 * new password.
 */

import { AuthError } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Navigate } from 'react-router-dom';
import { useAuth } from '../../../contexts/AuthContext';
import { getAuthApi } from '../authApiClient';
import '../styles/AuthPage.css';

const MIN_PASSWORD_LENGTH = 8;

interface FormErrors {
  currentPassword?: string;
  newPassword?: string;
  confirmPassword?: string;
  general?: string;
}

/** Maps an error to an i18n key relative to the `auth.changePassword` namespace. */
function getErrorKey(error: unknown): string {
  if (error instanceof AuthError) {
    switch (error.code) {
      case 'INVALID_CREDENTIALS':
        return 'currentPasswordIncorrect';
      case 'WEAK_PASSWORD':
        return 'weakPassword';
      case 'SESSION_EXPIRED':
        return 'sessionExpired';
      case 'NETWORK_ERROR':
        return 'networkError';
      default:
        return 'changeFailed';
    }
  }
  return 'unexpectedError';
}

export function ChangePasswordPage() {
  const { t } = useTranslation();
  const { isAuthenticated, isLoading } = useAuth();

  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [errors, setErrors] = useState<FormErrors>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [successMessage, setSuccessMessage] = useState<string>();

  const handleSubmit = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setErrors({});
      setSuccessMessage(undefined);

      const next: FormErrors = {};
      if (!currentPassword) next.currentPassword = 'currentPasswordRequired';
      if (!newPassword) next.newPassword = 'newPasswordRequired';
      else if (newPassword.length < MIN_PASSWORD_LENGTH) next.newPassword = 'passwordTooShort';
      else if (newPassword === currentPassword) next.newPassword = 'sameAsCurrent';
      if (confirmPassword !== newPassword) next.confirmPassword = 'passwordsDoNotMatch';
      if (Object.keys(next).length > 0) {
        setErrors(next);
        return;
      }

      setIsSubmitting(true);
      try {
        await getAuthApi().changePassword({ currentPassword, newPassword });
        setSuccessMessage(t('auth.changePassword.success'));
        setCurrentPassword('');
        setNewPassword('');
        setConfirmPassword('');
      } catch (error) {
        setErrors({ general: getErrorKey(error) });
      } finally {
        setIsSubmitting(false);
      }
    },
    [currentPassword, newPassword, confirmPassword, t]
  );

  /** Resolves a field/general error key into a localized message. */
  const errorText = (key?: string) =>
    key ? t(`auth.changePassword.${key}`, { count: MIN_PASSWORD_LENGTH }) : '';

  if (isLoading) return null;
  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return (
    <div className="auth-page">
      <div className="auth-container">
        <div className="auth-header">
          <h1 className="auth-title">{t('auth.changePassword.title')}</h1>
          <p className="auth-subtitle">{t('auth.changePassword.subtitle')}</p>
        </div>

        <form className="auth-form" onSubmit={handleSubmit} noValidate>
          {errors.general && (
            <div className="auth-error-banner" role="alert" aria-live="polite">
              <span aria-hidden="true">!</span>
              <span>{errorText(errors.general)}</span>
            </div>
          )}
          {successMessage && (
            <div className="auth-success-banner" role="status" aria-live="polite">
              <span>{successMessage}</span>
            </div>
          )}

          <div className="auth-field">
            <label htmlFor="currentPassword" className="auth-label">
              {t('auth.changePassword.currentPassword')}
            </label>
            <input
              id="currentPassword"
              name="currentPassword"
              type="password"
              autoComplete="current-password"
              value={currentPassword}
              onChange={(e) => setCurrentPassword(e.target.value)}
              disabled={isSubmitting}
              className={`auth-input ${errors.currentPassword ? 'auth-input--error' : ''}`}
              aria-invalid={errors.currentPassword ? 'true' : 'false'}
            />
            {errors.currentPassword && (
              <span className="auth-field-error">{errorText(errors.currentPassword)}</span>
            )}
          </div>

          <div className="auth-field">
            <label htmlFor="newPassword" className="auth-label">
              {t('auth.changePassword.newPassword')}
            </label>
            <input
              id="newPassword"
              name="newPassword"
              type="password"
              autoComplete="new-password"
              value={newPassword}
              onChange={(e) => setNewPassword(e.target.value)}
              disabled={isSubmitting}
              className={`auth-input ${errors.newPassword ? 'auth-input--error' : ''}`}
              aria-invalid={errors.newPassword ? 'true' : 'false'}
            />
            {errors.newPassword ? (
              <span className="auth-field-error">{errorText(errors.newPassword)}</span>
            ) : (
              <span className="auth-help">
                {t('auth.changePassword.passwordHelp', { count: MIN_PASSWORD_LENGTH })}
              </span>
            )}
          </div>

          <div className="auth-field">
            <label htmlFor="confirmPassword" className="auth-label">
              {t('auth.changePassword.confirmPassword')}
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
                <span>{t('auth.changePassword.submitting')}</span>
              </>
            ) : (
              t('auth.changePassword.submit')
            )}
          </button>
        </form>
      </div>
    </div>
  );
}

ChangePasswordPage.displayName = 'ChangePasswordPage';
