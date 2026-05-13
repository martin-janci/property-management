'use client';

/**
 * Registration page (UC-47.1).
 * Calls reality-server `POST /api/v1/users/register`.
 */

import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useTranslations } from 'next-intl';
import { type FormEvent, useState } from 'react';
import { AuthApiError, register } from '@/lib/auth-api';

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const MIN_PASSWORD = 8;

interface FieldErrors {
  email?: string;
  password?: string;
  confirmPassword?: string;
  displayName?: string;
  terms?: string;
}

export default function RegisterPage() {
  const router = useRouter();
  const t = useTranslations('pages.register');
  const [displayName, setDisplayName] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [termsAccepted, setTermsAccepted] = useState(false);
  const [errors, setErrors] = useState<FieldErrors>({});
  const [generalError, setGeneralError] = useState<string>();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setGeneralError(undefined);
    const next: FieldErrors = {};
    if (!displayName.trim()) next.displayName = 'Display name is required';
    if (!email.trim()) next.email = 'Email is required';
    else if (!EMAIL_RE.test(email.trim())) next.email = 'Enter a valid email address';
    if (!password) next.password = 'Password is required';
    else if (password.length < MIN_PASSWORD)
      next.password = `Password must be at least ${MIN_PASSWORD} characters`;
    if (confirmPassword !== password) next.confirmPassword = 'Passwords do not match';
    if (!termsAccepted) next.terms = t('termsRequired');
    setErrors(next);
    if (Object.keys(next).length > 0) return;

    setIsSubmitting(true);
    try {
      await register({
        email: email.trim(),
        password,
        name: displayName.trim(),
      });
      setSubmitted(true);
    } catch (error) {
      if (error instanceof AuthApiError && error.status === 409) {
        setErrors({ email: 'An account with this email already exists.' });
      } else if (error instanceof AuthApiError) {
        setGeneralError(error.message);
      } else {
        setGeneralError('Registration failed. Please try again.');
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <main className="page">
      <div className="card">
        {submitted ? (
          <>
            <h1 className="title">{t('checkInbox')}</h1>
            <p className="subtitle">{t('checkInboxBody', { email })}</p>
            <button type="button" className="submit" onClick={() => router.push('/auth/login')}>
              {t('backToSignIn')}
            </button>
          </>
        ) : (
          <>
            <h1 className="title">{t('title')}</h1>
            <p className="subtitle">{t('description')}</p>

            <form className="form" onSubmit={handleSubmit} noValidate>
              {generalError && (
                <div className="alert" role="alert">
                  {generalError}
                </div>
              )}

              <label className="field">
                <span className="label">Display name</span>
                <input
                  type="text"
                  autoComplete="name"
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  disabled={isSubmitting}
                  className={`input ${errors.displayName ? 'input-error' : ''}`}
                />
                {errors.displayName && <span className="error">{errors.displayName}</span>}
              </label>

              <label className="field">
                <span className="label">Email</span>
                <input
                  type="email"
                  autoComplete="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  disabled={isSubmitting}
                  className={`input ${errors.email ? 'input-error' : ''}`}
                />
                {errors.email && <span className="error">{errors.email}</span>}
              </label>

              <label className="field">
                <span className="label">Password</span>
                <input
                  type="password"
                  autoComplete="new-password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  disabled={isSubmitting}
                  className={`input ${errors.password ? 'input-error' : ''}`}
                />
                {errors.password ? (
                  <span className="error">{errors.password}</span>
                ) : (
                  <span className="hint">At least {MIN_PASSWORD} characters.</span>
                )}
              </label>

              <label className="field">
                <span className="label">Confirm password</span>
                <input
                  type="password"
                  autoComplete="new-password"
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  disabled={isSubmitting}
                  className={`input ${errors.confirmPassword ? 'input-error' : ''}`}
                />
                {errors.confirmPassword && <span className="error">{errors.confirmPassword}</span>}
              </label>

              <label className="terms-field">
                <input
                  type="checkbox"
                  checked={termsAccepted}
                  onChange={(e) => setTermsAccepted(e.target.checked)}
                  disabled={isSubmitting}
                  className="terms-checkbox"
                  aria-invalid={errors.terms ? true : undefined}
                />
                <span className="terms-text">
                  {t.rich('termsLabel', {
                    termsLink: (chunks) => (
                      <Link href="/terms" className="link" target="_blank">
                        {chunks}
                      </Link>
                    ),
                    privacyLink: (chunks) => (
                      <Link href="/privacy" className="link" target="_blank">
                        {chunks}
                      </Link>
                    ),
                  })}
                </span>
              </label>
              {errors.terms && <span className="error">{errors.terms}</span>}

              <button type="submit" className="submit" disabled={isSubmitting}>
                {isSubmitting ? t('creating') : t('submit')}
              </button>

              <p className="meta">
                Already have an account?{' '}
                <Link href="/auth/login" className="link">
                  Sign in
                </Link>
              </p>
            </form>
          </>
        )}
      </div>

      <style jsx>{`
        .page { min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 24px; background: var(--ppt-bg-app); }
        .card { width: 100%; max-width: 460px; background: var(--ppt-bg-surface); border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,.1); padding: 32px; }
        .title { font-size: 1.5rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0 0 4px; text-align: center; }
        .subtitle { font-size: 14px; color: var(--ppt-fg-muted); margin: 0 0 24px; text-align: center; }
        .form { display: flex; flex-direction: column; gap: 16px; }
        .alert { padding: 12px 16px; background: var(--ppt-color-danger-light); color: var(--ppt-color-danger-dark); border: 1px solid var(--ppt-color-danger-light); border-radius: 8px; font-size: 14px; }
        .field { display: flex; flex-direction: column; gap: 6px; }
        .label { font-size: 14px; font-weight: 500; color: var(--ppt-fg-secondary); }
        .input { padding: 10px 12px; font-size: 16px; border: 1px solid var(--ppt-border-strong); border-radius: 8px; background: var(--ppt-bg-surface); color: var(--ppt-fg-primary); }
        .input:focus { outline: none; border-color: var(--ppt-color-primary); box-shadow: 0 0 0 3px rgba(37,99,235,.1); }
        .input-error { border-color: var(--ppt-color-danger-hover); }
        .error { color: var(--ppt-color-danger-hover); font-size: 12px; }
        .hint { color: var(--ppt-fg-muted); font-size: 12px; }
        .submit { margin-top: 8px; padding: 12px 16px; background: var(--ppt-color-primary); color: var(--ppt-fg-on-accent); border: none; border-radius: 8px; font-size: 16px; font-weight: 600; cursor: pointer; }
        .submit:hover:not(:disabled) { background: var(--ppt-color-primary-hover); }
        .submit:disabled { background: var(--ppt-brand-500); cursor: not-allowed; }
        .meta { text-align: center; font-size: 14px; color: var(--ppt-neutral-600); margin: 8px 0 0; }
        .terms-field { display: flex; align-items: flex-start; gap: 10px; font-size: 13px; color: var(--ppt-fg-secondary); line-height: 1.5; cursor: pointer; margin-top: 4px; }
        .terms-checkbox { width: 18px; height: 18px; margin-top: 2px; accent-color: var(--ppt-color-primary); flex-shrink: 0; cursor: pointer; }
        .terms-text { flex: 1; }
        .link { color: var(--ppt-color-primary); text-decoration: none; font-weight: 500; }
        .link:hover { text-decoration: underline; }
      `}</style>
    </main>
  );
}
