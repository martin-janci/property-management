/**
 * Reset Password Page (UC-14.4 confirmation step).
 *
 * Reads `token` from the query string and submits a new password via
 * authApi.resetPassword.
 */

import { AuthError } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useMemo, useState } from 'react';
import { Link, useNavigate, useSearchParams } from 'react-router-dom';
import { getAuthApi } from '../authApiClient';
import '../styles/AuthPage.css';

const MIN_PASSWORD_LENGTH = 8;

interface FormErrors {
  password?: string;
  confirmPassword?: string;
  general?: string;
}

function getErrorMessage(error: unknown): string {
  if (error instanceof AuthError) {
    switch (error.code) {
      case 'TOKEN_EXPIRED':
        return 'This reset link has expired. Please request a new one.';
      case 'TOKEN_INVALID':
        return 'This reset link is invalid. Please request a new one.';
      case 'WEAK_PASSWORD':
        return 'Password does not meet the strength requirements.';
      case 'NETWORK_ERROR':
        return 'Network error. Please check your connection and try again.';
      default:
        return error.message || 'Could not reset password. Please try again.';
    }
  }
  return 'An unexpected error occurred. Please try again.';
}

export function ResetPasswordPage() {
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
        setErrors({ general: 'Missing reset token. Please request a new link.' });
        return;
      }
      const next: FormErrors = {};
      if (!password) next.password = 'Password is required';
      else if (password.length < MIN_PASSWORD_LENGTH)
        next.password = `Password must be at least ${MIN_PASSWORD_LENGTH} characters`;
      if (confirmPassword !== password) next.confirmPassword = 'Passwords do not match';
      if (Object.keys(next).length > 0) {
        setErrors(next);
        return;
      }

      setIsSubmitting(true);
      try {
        await getAuthApi().resetPassword({ token, newPassword: password });
        setSubmitted(true);
      } catch (error) {
        setErrors({ general: getErrorMessage(error) });
      } finally {
        setIsSubmitting(false);
      }
    },
    [token, password, confirmPassword]
  );

  if (submitted) {
    return (
      <div className="auth-page">
        <div className="auth-container">
          <div className="auth-header">
            <h1 className="auth-title">Password updated</h1>
            <p className="auth-subtitle">You can now sign in with your new password.</p>
          </div>
          <button
            type="button"
            className="auth-submit"
            onClick={() => navigate('/login', { replace: true })}
          >
            Sign in
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
            <h1 className="auth-title">Invalid link</h1>
            <p className="auth-subtitle">
              The reset link is missing a token. Request a new password reset email.
            </p>
          </div>
          <div className="auth-links">
            <Link to="/forgot-password" className="auth-link">
              Request a new link
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
          <h1 className="auth-title">Set a new password</h1>
          <p className="auth-subtitle">Choose a strong password you haven't used before.</p>
        </div>

        <form className="auth-form" onSubmit={handleSubmit} noValidate>
          {errors.general && (
            <div className="auth-error-banner" role="alert" aria-live="polite">
              <span aria-hidden="true">!</span>
              <span>{errors.general}</span>
            </div>
          )}

          <div className="auth-field">
            <label htmlFor="password" className="auth-label">
              New password
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
              <span className="auth-field-error">{errors.password}</span>
            ) : (
              <span className="auth-help">At least {MIN_PASSWORD_LENGTH} characters.</span>
            )}
          </div>

          <div className="auth-field">
            <label htmlFor="confirmPassword" className="auth-label">
              Confirm password
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

ResetPasswordPage.displayName = 'ResetPasswordPage';
