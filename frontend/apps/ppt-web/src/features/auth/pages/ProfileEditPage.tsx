/**
 * Profile Edit Page (UC-14.6).
 *
 * Lets the authenticated user view and update their basic profile fields.
 * Persists the change to the backend (`PATCH /me` via
 * `getAuthApi().updateProfile`) and then updates the in-memory auth state and
 * storage cache through `useAuth().setUser`, so other consumers (header,
 * dashboards) see the new name without a reload and the edit survives a token
 * refresh / re-login / other devices.
 */

import type { AuthUser } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, Navigate } from 'react-router-dom';
import { useAuth } from '../../../contexts/AuthContext';
import { getAuthApi } from '../authApiClient';
import '../styles/AuthPage.css';

interface FormErrors {
  firstName?: string;
  lastName?: string;
  general?: string;
}

export function ProfileEditPage() {
  const { t } = useTranslation();
  const { user, isAuthenticated, isLoading, setUser } = useAuth();

  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');
  const [errors, setErrors] = useState<FormErrors>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [savedMessage, setSavedMessage] = useState<string>();

  useEffect(() => {
    if (user) {
      setFirstName(user.firstName ?? '');
      setLastName(user.lastName ?? '');
    }
  }, [user]);

  const handleSubmit = useCallback(
    async (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setErrors({});
      setSavedMessage(undefined);

      const next: FormErrors = {};
      if (!firstName.trim()) next.firstName = 'firstNameRequired';
      if (!lastName.trim()) next.lastName = 'lastNameRequired';
      if (Object.keys(next).length > 0) {
        setErrors(next);
        return;
      }

      setIsSubmitting(true);
      try {
        const trimmedFirst = firstName.trim();
        const trimmedLast = lastName.trim();

        // Persist to the backend first. The backend only round-trips a single
        // `displayName`, so we send the combined name and keep the split
        // first/last from the trimmed inputs locally.
        await getAuthApi().updateProfile({
          displayName: `${trimmedFirst} ${trimmedLast}`,
        });

        const updated: AuthUser = {
          ...(user as AuthUser),
          firstName: trimmedFirst,
          lastName: trimmedLast,
        };
        // setUser persists to storage AND updates in-memory auth state, so
        // every consumer of the context re-renders with the new name.
        setUser(updated);
        setSavedMessage(t('auth.profile.success'));
      } catch {
        setErrors({ general: 'saveFailed' });
      } finally {
        setIsSubmitting(false);
      }
    },
    [firstName, lastName, user, setUser, t]
  );

  /** Resolves a field/general error key into a localized message. */
  const errorText = (key?: string) => (key ? t(`auth.profile.${key}`) : '');

  if (isLoading) return null;
  if (!isAuthenticated || !user) {
    return <Navigate to="/login" replace />;
  }

  return (
    <div className="auth-page">
      <div className="auth-container">
        <div className="auth-header">
          <h1 className="auth-title">{t('auth.profile.title')}</h1>
          <p className="auth-subtitle">{t('auth.profile.subtitle')}</p>
        </div>

        <form className="auth-form" onSubmit={handleSubmit} noValidate>
          {errors.general && (
            <div className="auth-error-banner" role="alert" aria-live="polite">
              <span aria-hidden="true">!</span>
              <span>{errorText(errors.general)}</span>
            </div>
          )}
          {savedMessage && (
            <div className="auth-success-banner" role="status" aria-live="polite">
              <span>{savedMessage}</span>
            </div>
          )}

          <div className="auth-field-row">
            <div className="auth-field">
              <label htmlFor="firstName" className="auth-label">
                {t('auth.profile.firstName')}
              </label>
              <input
                id="firstName"
                name="firstName"
                type="text"
                autoComplete="given-name"
                value={firstName}
                onChange={(e) => setFirstName(e.target.value)}
                disabled={isSubmitting}
                className={`auth-input ${errors.firstName ? 'auth-input--error' : ''}`}
                aria-invalid={errors.firstName ? 'true' : 'false'}
              />
              {errors.firstName && (
                <span className="auth-field-error">{errorText(errors.firstName)}</span>
              )}
            </div>
            <div className="auth-field">
              <label htmlFor="lastName" className="auth-label">
                {t('auth.profile.lastName')}
              </label>
              <input
                id="lastName"
                name="lastName"
                type="text"
                autoComplete="family-name"
                value={lastName}
                onChange={(e) => setLastName(e.target.value)}
                disabled={isSubmitting}
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
              {t('auth.profile.email')}
            </label>
            <input
              id="email"
              name="email"
              type="email"
              autoComplete="email"
              value={user.email}
              disabled
              className="auth-input"
            />
            <span className="auth-help">{t('auth.profile.emailHelp')}</span>
          </div>

          <button type="submit" className="auth-submit" disabled={isSubmitting}>
            {isSubmitting ? (
              <>
                <span className="auth-spinner" aria-hidden="true" />
                <span>{t('auth.profile.submitting')}</span>
              </>
            ) : (
              t('auth.profile.submit')
            )}
          </button>
        </form>

        <div className="auth-links">
          <Link to="/settings/password" className="auth-link">
            {t('auth.profile.changePassword')}
          </Link>
          <Link to="/settings/two-factor" className="auth-link">
            {t('auth.profile.twoFactor')}
          </Link>
        </div>
      </div>
    </div>
  );
}

ProfileEditPage.displayName = 'ProfileEditPage';
