/**
 * Registration Page (UC-14.1).
 *
 * Collects email, password and name, calls authApi.register and shows a
 * confirmation prompting the user to verify their email.
 */

import { AuthError } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useState } from 'react';
import { Trans, useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router-dom';
import { trackRegistrationSubmitted } from '../analytics';
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

/**
 * Validates the form and returns i18n keys (relative to the `auth.register`
 * namespace) for any field that fails. Keys are resolved by the component so
 * the validator stays free of the `t` function.
 */
function validate(values: {
  email: string;
  password: string;
  confirmPassword: string;
  firstName: string;
  lastName: string;
}): FormErrors {
  const errors: FormErrors = {};
  if (!values.firstName.trim()) errors.firstName = 'firstNameRequired';
  if (!values.lastName.trim()) errors.lastName = 'lastNameRequired';
  if (!values.email.trim()) {
    errors.email = 'emailRequired';
  } else if (!EMAIL_RE.test(values.email.trim())) {
    errors.email = 'invalidEmail';
  }
  if (!values.password) {
    errors.password = 'passwordRequired';
  } else if (values.password.length < MIN_PASSWORD_LENGTH) {
    errors.password = 'passwordTooShort';
  }
  if (values.confirmPassword !== values.password) {
    errors.confirmPassword = 'passwordsDoNotMatch';
  }
  return errors;
}

/** Maps an error to an i18n key relative to the `auth.register` namespace. */
function getErrorKey(error: unknown): string {
  if (error instanceof AuthError) {
    switch (error.code) {
      case 'EMAIL_ALREADY_EXISTS':
        return 'emailExists';
      case 'WEAK_PASSWORD':
        return 'weakPassword';
      case 'NETWORK_ERROR':
        return 'networkError';
      default:
        return 'registrationFailed';
    }
  }
  return 'unexpectedError';
}

export function RegisterPage() {
  const { t } = useTranslation();
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
        // Signup-funnel: first (and earliest) leg — fired only after the
        // server accepts the registration (issue #2530).
        trackRegistrationSubmitted('email_password');
        setSubmitted(true);
      } catch (error) {
        setErrors({ general: getErrorKey(error) });
      } finally {
        setIsSubmitting(false);
      }
    },
    [email, password, confirmPassword, firstName, lastName]
  );

  /** Resolves a field/general error key into a localized message. */
  const errorText = (key?: string) =>
    key ? t(`auth.register.${key}`, { count: MIN_PASSWORD_LENGTH }) : '';

  if (submitted) {
    return (
      <div className="auth-page">
        <div className="auth-container">
          <div className="auth-header">
            <h1 className="auth-title">{t('auth.register.checkInboxTitle')}</h1>
            <p className="auth-subtitle">
              <Trans
                i18nKey="auth.register.checkInboxSubtitle"
                values={{ email }}
                components={{ 1: <strong /> }}
              />
            </p>
          </div>
          <button
            type="button"
            className="auth-submit"
            onClick={() => navigate('/login', { replace: true })}
          >
            {t('auth.register.backToSignIn')}
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
          <h1 className="auth-title">{t('auth.register.title')}</h1>
          <p className="auth-subtitle">{t('auth.register.subtitle')}</p>
        </div>

        <form className="auth-form" onSubmit={handleSubmit} noValidate>
          {errors.general && (
            <div className="auth-error-banner" role="alert" aria-live="polite">
              <span aria-hidden="true">!</span>
              <span>{errorText(errors.general)}</span>
            </div>
          )}

          <div className="auth-field-row">
            <div className="auth-field">
              <label htmlFor="firstName" className="auth-label">
                {t('auth.register.firstName')}
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
              {errors.firstName && (
                <span className="auth-field-error">{errorText(errors.firstName)}</span>
              )}
            </div>
            <div className="auth-field">
              <label htmlFor="lastName" className="auth-label">
                {t('auth.register.lastName')}
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
              {errors.lastName && (
                <span className="auth-field-error">{errorText(errors.lastName)}</span>
              )}
            </div>
          </div>

          <div className="auth-field">
            <label htmlFor="email" className="auth-label">
              {t('auth.register.email')}
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
            {errors.email && <span className="auth-field-error">{errorText(errors.email)}</span>}
          </div>

          <div className="auth-field">
            <label htmlFor="password" className="auth-label">
              {t('auth.register.password')}
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
              <span className="auth-field-error">{errorText(errors.password)}</span>
            ) : (
              <span className="auth-help">
                {t('auth.register.passwordHelp', { count: MIN_PASSWORD_LENGTH })}
              </span>
            )}
          </div>

          <div className="auth-field">
            <label htmlFor="confirmPassword" className="auth-label">
              {t('auth.register.confirmPassword')}
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
              <span className="auth-field-error">{errorText(errors.confirmPassword)}</span>
            )}
          </div>

          <button type="submit" className="auth-submit" disabled={disabled}>
            {isSubmitting ? (
              <>
                <span className="auth-spinner" aria-hidden="true" />
                <span>{t('auth.register.submitting')}</span>
              </>
            ) : (
              t('auth.register.submit')
            )}
          </button>
        </form>

        <div className="auth-links">
          <span>
            {t('auth.register.haveAccount')}{' '}
            <Link to="/login" className="auth-link">
              {t('auth.register.signIn')}
            </Link>
          </span>
        </div>
      </div>
    </div>
  );
}

RegisterPage.displayName = 'RegisterPage';
