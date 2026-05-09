/**
 * Change Password Page (UC-14.5).
 *
 * Authenticated user updates their password by providing the current and a
 * new password.
 */

import { AuthError } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useState } from 'react';
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

function getErrorMessage(error: unknown): string {
  if (error instanceof AuthError) {
    switch (error.code) {
      case 'INVALID_CREDENTIALS':
        return 'Current password is incorrect.';
      case 'WEAK_PASSWORD':
        return 'Password does not meet the strength requirements.';
      case 'SESSION_EXPIRED':
        return 'Your session has expired. Please sign in again.';
      case 'NETWORK_ERROR':
        return 'Network error. Please check your connection and try again.';
      default:
        return error.message || 'Could not change password. Please try again.';
    }
  }
  return 'An unexpected error occurred. Please try again.';
}

export function ChangePasswordPage() {
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
      if (!currentPassword) next.currentPassword = 'Current password is required';
      if (!newPassword) next.newPassword = 'New password is required';
      else if (newPassword.length < MIN_PASSWORD_LENGTH)
        next.newPassword = `Password must be at least ${MIN_PASSWORD_LENGTH} characters`;
      else if (newPassword === currentPassword)
        next.newPassword = 'New password must differ from the current password';
      if (confirmPassword !== newPassword) next.confirmPassword = 'Passwords do not match';
      if (Object.keys(next).length > 0) {
        setErrors(next);
        return;
      }

      setIsSubmitting(true);
      try {
        await getAuthApi().changePassword({ currentPassword, newPassword });
        setSuccessMessage('Password updated successfully.');
        setCurrentPassword('');
        setNewPassword('');
        setConfirmPassword('');
      } catch (error) {
        setErrors({ general: getErrorMessage(error) });
      } finally {
        setIsSubmitting(false);
      }
    },
    [currentPassword, newPassword, confirmPassword]
  );

  if (isLoading) return null;
  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  return (
    <div className="auth-page">
      <div className="auth-container">
        <div className="auth-header">
          <h1 className="auth-title">Change password</h1>
          <p className="auth-subtitle">Update the password used to sign in to your account.</p>
        </div>

        <form className="auth-form" onSubmit={handleSubmit} noValidate>
          {errors.general && (
            <div className="auth-error-banner" role="alert" aria-live="polite">
              <span aria-hidden="true">!</span>
              <span>{errors.general}</span>
            </div>
          )}
          {successMessage && (
            <div className="auth-success-banner" role="status" aria-live="polite">
              <span>{successMessage}</span>
            </div>
          )}

          <div className="auth-field">
            <label htmlFor="currentPassword" className="auth-label">
              Current password
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
              <span className="auth-field-error">{errors.currentPassword}</span>
            )}
          </div>

          <div className="auth-field">
            <label htmlFor="newPassword" className="auth-label">
              New password
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
              <span className="auth-field-error">{errors.newPassword}</span>
            ) : (
              <span className="auth-help">At least {MIN_PASSWORD_LENGTH} characters.</span>
            )}
          </div>

          <div className="auth-field">
            <label htmlFor="confirmPassword" className="auth-label">
              Confirm new password
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
              <span className="auth-field-error">{errors.confirmPassword}</span>
            )}
          </div>

          <button type="submit" className="auth-submit" disabled={isSubmitting}>
            {isSubmitting ? (
              <>
                <span className="auth-spinner" aria-hidden="true" />
                <span>Updating…</span>
              </>
            ) : (
              'Update password'
            )}
          </button>
        </form>
      </div>
    </div>
  );
}

ChangePasswordPage.displayName = 'ChangePasswordPage';
