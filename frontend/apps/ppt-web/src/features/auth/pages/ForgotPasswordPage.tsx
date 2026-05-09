/**
 * Forgot Password Page (UC-14.4).
 *
 * Accepts an email and triggers a password-reset email via
 * authApi.requestPasswordReset.
 */

import { AuthError } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useState } from 'react';
import { Link } from 'react-router-dom';
import { getAuthApi } from '../authApiClient';
import '../styles/AuthPage.css';

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function ForgotPasswordPage() {
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
        setEmailError('Email is required');
        return;
      }
      if (!EMAIL_RE.test(trimmed)) {
        setEmailError('Enter a valid email address');
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
          setGeneralError('Network error. Please check your connection and try again.');
        } else {
          setSubmitted(true);
        }
      } finally {
        setIsSubmitting(false);
      }
    },
    [email]
  );

  if (submitted) {
    return (
      <div className="auth-page">
        <div className="auth-container">
          <div className="auth-header">
            <h1 className="auth-title">Check your inbox</h1>
            <p className="auth-subtitle">
              If an account exists for <strong>{email}</strong>, we've sent instructions to reset
              your password.
            </p>
          </div>
          <div className="auth-links">
            <Link to="/login" className="auth-link">
              Back to sign in
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
          <h1 className="auth-title">Reset your password</h1>
          <p className="auth-subtitle">Enter your email and we'll send you a reset link.</p>
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
              Email
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
                <span>Sending…</span>
              </>
            ) : (
              'Send reset link'
            )}
          </button>
        </form>

        <div className="auth-links">
          <Link to="/login" className="auth-link">
            Back to sign in
          </Link>
        </div>
      </div>
    </div>
  );
}

ForgotPasswordPage.displayName = 'ForgotPasswordPage';
