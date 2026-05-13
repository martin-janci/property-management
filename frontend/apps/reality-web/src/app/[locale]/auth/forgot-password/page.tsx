'use client';

/**
 * Forgot password page (UC-47.4 request step).
 * Calls reality-server `/api/v1/auth/password-reset` to email a reset link.
 */

import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { type FormEvent, useState } from 'react';
import { AuthApiError, requestPasswordReset } from '@/lib/auth-api';

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export default function ForgotPasswordPage() {
  const t = useTranslations('pages.forgotPassword');
  const [email, setEmail] = useState('');
  const [emailError, setEmailError] = useState<string>();
  const [generalError, setGeneralError] = useState<string>();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setEmailError(undefined);
    setGeneralError(undefined);
    const trimmed = email.trim();
    if (!trimmed) return setEmailError(t('emailRequired'));
    if (!EMAIL_RE.test(trimmed)) return setEmailError(t('emailInvalid'));

    setIsSubmitting(true);
    try {
      await requestPasswordReset(trimmed);
      setSubmitted(true);
    } catch (error) {
      if (error instanceof AuthApiError) {
        if (error.code === 'NOT_IMPLEMENTED' || error.status === 501) {
          setGeneralError(error.message);
        } else if (error.code === 'NETWORK_ERROR') {
          setGeneralError(t('networkError'));
        } else if (error.status >= 500) {
          setGeneralError(error.message || t('serverError'));
        } else {
          // 4xx: don't leak whether an account exists — confirmation screen.
          setSubmitted(true);
        }
      } else {
        setGeneralError(t('genericError'));
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
            <Link href="/auth/login" className="link center">
              {t('backToSignIn')}
            </Link>
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
                <span className="label">{t('emailLabel')}</span>
                <input
                  type="email"
                  autoComplete="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  disabled={isSubmitting}
                  aria-invalid={emailError ? true : undefined}
                  className={`input ${emailError ? 'input-error' : ''}`}
                />
                {emailError && <span className="error">{emailError}</span>}
              </label>
              <button type="submit" className="submit" disabled={isSubmitting}>
                {isSubmitting ? t('submitting') : t('submit')}
              </button>
              <p className="meta">
                {t('remembered')}{' '}
                <Link href="/auth/login" className="link">
                  {t('signIn')}
                </Link>
              </p>
            </form>
          </>
        )}
      </div>

      <style jsx>{`
        .page { min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 24px; background: var(--ppt-bg-app); }
        .card { width: 100%; max-width: 420px; background: var(--ppt-bg-surface); border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,.1); padding: 32px; }
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
        .submit { margin-top: 8px; padding: 12px 16px; background: var(--ppt-color-primary); color: var(--ppt-fg-on-accent); border: none; border-radius: 8px; font-size: 16px; font-weight: 600; cursor: pointer; }
        .submit:hover:not(:disabled) { background: var(--ppt-color-primary-hover); }
        .submit:disabled { background: var(--ppt-brand-500); cursor: not-allowed; }
        .meta { text-align: center; font-size: 14px; color: var(--ppt-neutral-600); margin: 8px 0 0; }
        .link { color: var(--ppt-color-primary); text-decoration: none; font-weight: 500; }
        .link:hover { text-decoration: underline; }
        .center { display: block; text-align: center; }
      `}</style>
    </main>
  );
}
