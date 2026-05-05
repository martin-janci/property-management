/**
 * Registration Page (UC-14.1).
 *
 * Collects email, password and name, calls authApi.register and shows a
 * confirmation prompting the user to verify their email.
 */

import { AuthError } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router-dom';
import { getAuthApi } from '../authApiClient';
import '../styles/AuthPage.css';

interface FormErrors {
  email?: string;
  password?: string;
  confirmPassword?: string;
  firstName?: string;
  lastName?: string;
  general?: string;
}

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const MIN_PASSWORD_LENGTH = 8;

function validate(values: {
  email: string;
  password: string;
  confirmPassword: string;
  firstName: string;
  lastName: string;
}): FormErrors {
  const errors: FormErrors = {};
  if (!values.firstName.trim()) errors.firstName = 'First name is required';
  if (!values.lastName.trim()) errors.lastName = 'Last name is required';
  if (!values.email.trim()) {
    errors.email = 'Email is required';
  } else if (!EMAIL_RE.test(values.email.trim())) {
    errors.email = 'Enter a valid email address';
  }
  if (!values.password) {
    errors.password = 'Password is required';
  } else if (values.password.length < MIN_PASSWORD_LENGTH) {
    errors.password = `Password must be at least ${MIN_PASSWORD_LENGTH} characters`;
  }
  if (values.confirmPassword !== values.password) {
    errors.confirmPassword = 'Passwords do not match';
  }
  return errors;
}

function getErrorMessage(error: unknown): string {
  if (error instanceof AuthError) {
    switch (error.code) {
      case 'EMAIL_ALREADY_EXISTS':
        return 'An account with this email already exists.';
      case 'WEAK_PASSWORD':
        return 'Password does not meet the strength requirements.';
      case 'NETWORK_ERROR':
        return 'Network error. Please check your connection and try again.';
      default:
        return error.message || 'Registration failed. Please try again.';
    }
  }
  return 'An unexpected error occurred. Please try again.';
}

export function RegisterPage() {
  useTranslation();
  const navigate = useNavigate();

  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [errors, setErrors] = useState<FormErrors>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  const handleSubmit = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setErrors({});
      const validationErrors = validate({
        email,
        password,
        confirmPassword,
        firstName,
        lastName,
      });
      if (Object.keys(validationErrors).length > 0) {
        setErrors(validationErrors);
        return;
      }
      setIsSubmitting(true);
      try {
        await getAuthApi().register({
          email: email.trim(),
          password,
          firstName: firstName.trim(),
          lastName: lastName.trim(),
        });
        setSubmitted(true);
      } catch (error) {
        setErrors({ general: getErrorMessage(error) });
      } finally {
        setIsSubmitting(false);
      }
    },
    [email, password, confirmPassword, firstName, lastName]
  );

  if (submitted) {
    return (
      <div className="auth-page">
        <div className="auth-container">
          <div className="auth-header">
            <h1 className="auth-title">Check your inbox</h1>
            <p className="auth-subtitle">
              We sent a verification link to <strong>{email}</strong>. Click the link to activate
              your account.
            </p>
          </div>
          <button
            type="button"
            className="auth-submit"
            onClick={() => navigate('/login', { replace: true })}
          >
            Back to sign in
          </button>
        </div>
      </div>
    );
  }

  const disabled = isSubmitting;

  return (
    <div className="auth-page">
      <div className="auth-container">
        <div className="auth-header">
          <h1 className="auth-title">Create your account</h1>
          <p className="auth-subtitle">Get started with Property Management</p>
        </div>

        <form className="auth-form" onSubmit={handleSubmit} noValidate>
          {errors.general && (
            <div className="auth-error-banner" role="alert" aria-live="polite">
              <span aria-hidden="true">!</span>
              <span>{errors.general}</span>
            </div>
          )}

          <div className="auth-field-row">
            <div className="auth-field">
              <label htmlFor="firstName" className="auth-label">
                First name
              </label>
              <input
                id="firstName"
                name="firstName"
                type="text"
                autoComplete="given-name"
                value={firstName}
                onChange={(e) => setFirstName(e.target.value)}
                disabled={disabled}
                className={`auth-input ${errors.firstName ? 'auth-input--error' : ''}`}
                aria-invalid={errors.firstName ? 'true' : 'false'}
              />
              {errors.firstName && <span className="auth-field-error">{errors.firstName}</span>}
            </div>
            <div className="auth-field">
              <label htmlFor="lastName" className="auth-label">
                Last name
              </label>
              <input
                id="lastName"
                name="lastName"
                type="text"
                autoComplete="family-name"
                value={lastName}
                onChange={(e) => setLastName(e.target.value)}
                disabled={disabled}
                className={`auth-input ${errors.lastName ? 'auth-input--error' : ''}`}
                aria-invalid={errors.lastName ? 'true' : 'false'}
              />
              {errors.lastName && <span className="auth-field-error">{errors.lastName}</span>}
            </div>
          </div>

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
              disabled={disabled}
              className={`auth-input ${errors.email ? 'auth-input--error' : ''}`}
              aria-invalid={errors.email ? 'true' : 'false'}
            />
            {errors.email && <span className="auth-field-error">{errors.email}</span>}
          </div>

          <div className="auth-field">
            <label htmlFor="password" className="auth-label">
              Password
            </label>
            <input
              id="password"
              name="password"
              type="password"
              autoComplete="new-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              disabled={disabled}
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
              disabled={disabled}
              className={`auth-input ${errors.confirmPassword ? 'auth-input--error' : ''}`}
              aria-invalid={errors.confirmPassword ? 'true' : 'false'}
            />
            {errors.confirmPassword && (
              <span className="auth-field-error">{errors.confirmPassword}</span>
            )}
          </div>

          <button type="submit" className="auth-submit" disabled={disabled}>
            {isSubmitting ? (
              <>
                <span className="auth-spinner" aria-hidden="true" />
                <span>Creating account…</span>
              </>
            ) : (
              'Create account'
            )}
          </button>
        </form>

        <div className="auth-links">
          <span>
            Already have an account?{' '}
            <Link to="/login" className="auth-link">
              Sign in
            </Link>
          </span>
        </div>
      </div>
    </div>
  );
}

RegisterPage.displayName = 'RegisterPage';
