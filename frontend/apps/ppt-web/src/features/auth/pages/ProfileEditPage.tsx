/**
 * Profile Edit Page (UC-14.6).
 *
 * Lets the authenticated user view and update their basic profile fields.
 * Persists optimistically to localStorage; the backend `PATCH /me` endpoint
 * isn't exposed through @ppt/api-client yet so this is a UI-only scaffold.
 */

import type { AuthUser } from '@ppt/api-client';
import type React from 'react';
import { useCallback, useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../../../contexts/AuthContext';
import '../styles/AuthPage.css';

const USER_KEY = 'ppt_user';

interface FormErrors {
  firstName?: string;
  lastName?: string;
  general?: string;
}

export function ProfileEditPage() {
  const navigate = useNavigate();
  const { user, isAuthenticated, isLoading } = useAuth();

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
      if (!firstName.trim()) next.firstName = 'First name is required';
      if (!lastName.trim()) next.lastName = 'Last name is required';
      if (Object.keys(next).length > 0) {
        setErrors(next);
        return;
      }

      setIsSubmitting(true);
      try {
        const updated: AuthUser = {
          ...(user as AuthUser),
          firstName: firstName.trim(),
          lastName: lastName.trim(),
        };
        try {
          localStorage.setItem(USER_KEY, JSON.stringify(updated));
        } catch {
          // Storage unavailable; the in-memory update still applies.
        }
        setSavedMessage('Profile updated.');
      } catch {
        setErrors({ general: 'Could not save profile. Please try again.' });
      } finally {
        setIsSubmitting(false);
      }
    },
    [firstName, lastName, user]
  );

  if (isLoading) return null;
  if (!isAuthenticated || !user) {
    navigate('/login', { replace: true });
    return null;
  }

  return (
    <div className="auth-page">
      <div className="auth-container">
        <div className="auth-header">
          <h1 className="auth-title">Edit profile</h1>
          <p className="auth-subtitle">Update your name as it appears in the app.</p>
        </div>

        <form className="auth-form" onSubmit={handleSubmit} noValidate>
          {errors.general && (
            <div className="auth-error-banner" role="alert" aria-live="polite">
              <span aria-hidden="true">!</span>
              <span>{errors.general}</span>
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
                First name
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
                disabled={isSubmitting}
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
              value={user.email}
              disabled
              className="auth-input"
            />
            <span className="auth-help">Contact support to change the email on this account.</span>
          </div>

          <button type="submit" className="auth-submit" disabled={isSubmitting}>
            {isSubmitting ? (
              <>
                <span className="auth-spinner" aria-hidden="true" />
                <span>Saving…</span>
              </>
            ) : (
              'Save changes'
            )}
          </button>
        </form>

        <div className="auth-links">
          <Link to="/settings/password" className="auth-link">
            Change password
          </Link>
          <Link to="/settings/two-factor" className="auth-link">
            Two-factor authentication
          </Link>
        </div>
      </div>
    </div>
  );
}

ProfileEditPage.displayName = 'ProfileEditPage';
