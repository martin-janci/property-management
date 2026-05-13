'use client';

/**
 * Password reset confirmation page (UC-47.4 confirm step).
 * Reads `token` from the URL and submits a new password.
 */

import Link from 'next/link';
import { useRouter, useSearchParams } from 'next/navigation';
import { useTranslations } from 'next-intl';
import { type FormEvent, Suspense, useState } from 'react';
import { AuthApiError, confirmPasswordReset } from '@/lib/auth-api';

const MIN_PASSWORD = 8;

function ResetForm() {
  const t = useTranslations('pages.resetPassword');
  const router = useRouter();
  const searchParams = useSearchParams();
  const token = searchParams.get('token') ?? '';

  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [passwordError, setPasswordError] = useState<string>();
  const [confirmError, setConfirmError] = useState<string>();
  const [generalError, setGeneralError] = useState<string>();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [success, setSuccess] = useState(false);

  if (!token) {
    return (
      <div className="empty">
        <h1 className="title">{t('invalidTitle')}</h1>
        <p className="subtitle">{t('invalidBody')}</p>
        <Link href="/auth/forgot-password" className="link center">
          {t('requestNew')}
        </Link>
        <style jsx>{`
          .empty { padding: 8px 0; }
          .title { font-size: 1.5rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0 0 4px; text-align: center; }
          .subtitle { font-size: 14px; color: var(--ppt-fg-muted); margin: 0 0 16px; text-align: center; }
          .link { color: var(--ppt-color-primary); text-decoration: none; font-weight: 500; }
          .link:hover { text-decoration: underline; }
          .center { display: block; text-align: center; }
        `}</style>
      </div>
    );
  }

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setPasswordError(undefined);
    setConfirmError(undefined);
    setGeneralError(undefined);

    let invalid = false;
    if (!password) {
      setPasswordError(t('passwordRequired'));
      invalid = true;
    } else if (password.length < MIN_PASSWORD) {
      setPasswordError(t('passwordTooShort'));
      invalid = true;
    }
    if (confirmPassword !== password) {
      setConfirmError(t('passwordsMismatch'));
      invalid = true;
    }
    if (invalid) return;

    setIsSubmitting(true);
    try {
      await confirmPasswordReset(token, password);
      setSuccess(true);
    } catch (error) {
      if (error instanceof AuthApiError) {
        setGeneralError(error.message);
      } else {
        setGeneralError(t('genericError'));
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  if (success) {
    return (
      <div className="ok">
        <h1 className="title">{t('successTitle')}</h1>
        <p className="subtitle">{t('successBody')}</p>
        <button type="button" className="submit" onClick={() => router.replace('/auth/login')}>
          {t('goToSignIn')}
        </button>
        <style jsx>{`
          .ok { padding: 8px 0; }
          .title { font-size: 1.5rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0 0 4px; text-align: center; }
          .subtitle { font-size: 14px; color: var(--ppt-fg-muted); margin: 0 0 24px; text-align: center; }
          .submit { width: 100%; padding: 12px 16px; background: var(--ppt-color-primary); color: var(--ppt-fg-on-accent); border: none; border-radius: 8px; font-size: 16px; font-weight: 600; cursor: pointer; }
          .submit:hover { background: var(--ppt-color-primary-hover); }
        `}</style>
      </div>
    );
  }

  return (
    <form className="form" onSubmit={handleSubmit} noValidate>
      <h1 className="title">{t('title')}</h1>
      <p className="subtitle">{t('description')}</p>

      {generalError && (
        <div className="alert" role="alert">
          {generalError}
        </div>
      )}

      <label className="field">
        <span className="label">{t('passwordLabel')}</span>
        <input
          type="password"
          autoComplete="new-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          disabled={isSubmitting}
          aria-invalid={passwordError ? true : undefined}
          className={`input ${passwordError ? 'input-error' : ''}`}
        />
        {passwordError ? (
          <span className="error">{passwordError}</span>
        ) : (
          <span className="hint">{t('passwordHint')}</span>
        )}
      </label>

      <label className="field">
        <span className="label">{t('confirmLabel')}</span>
        <input
          type="password"
          autoComplete="new-password"
          value={confirmPassword}
          onChange={(e) => setConfirmPassword(e.target.value)}
          disabled={isSubmitting}
          aria-invalid={confirmError ? true : undefined}
          className={`input ${confirmError ? 'input-error' : ''}`}
        />
        {confirmError && <span className="error">{confirmError}</span>}
      </label>

      <button type="submit" className="submit" disabled={isSubmitting}>
        {isSubmitting ? t('submitting') : t('submit')}
      </button>

      <style jsx>{`
        .form { display: flex; flex-direction: column; gap: 16px; }
        .title { font-size: 1.5rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0; text-align: center; }
        .subtitle { font-size: 14px; color: var(--ppt-fg-muted); margin: 0 0 8px; text-align: center; }
        .alert { padding: 12px 16px; background: var(--ppt-color-danger-light); color: #b91c1c; border: 1px solid #fecaca; border-radius: 8px; font-size: 14px; }
        .field { display: flex; flex-direction: column; gap: 6px; }
        .label { font-size: 14px; font-weight: 500; color: var(--ppt-fg-secondary); }
        .input { padding: 10px 12px; font-size: 16px; border: 1px solid var(--ppt-border-strong); border-radius: 8px; background: var(--ppt-bg-surface); color: var(--ppt-fg-primary); }
        .input:focus-visible { outline: var(--ppt-focus-ring-width) solid var(--ppt-focus-ring-color); outline-offset: var(--ppt-focus-ring-offset); border-color: var(--ppt-color-primary); }
        .input-error { border-color: var(--ppt-color-danger-hover); }
        .error { color: var(--ppt-color-danger-hover); font-size: 12px; }
        .hint { color: var(--ppt-fg-muted); font-size: 12px; }
        .submit { margin-top: 8px; padding: 12px 16px; background: var(--ppt-color-primary); color: var(--ppt-fg-on-accent); border: none; border-radius: 8px; font-size: 16px; font-weight: 600; cursor: pointer; }
        .submit:hover:not(:disabled) { background: var(--ppt-color-primary-hover); }
        .submit:disabled { opacity: 0.5; cursor: not-allowed; }
      `}</style>
    </form>
  );
}

export default function ResetPasswordPage() {
  return (
    <main className="page">
      <div className="card">
        <Suspense fallback={<p>Loading…</p>}>
          <ResetForm />
        </Suspense>
      </div>
      <style jsx>{`
        .page { min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 24px; background: var(--ppt-bg-app); }
        .card { width: 100%; max-width: 420px; background: var(--ppt-bg-surface); border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,.1); padding: 32px; }
      `}</style>
    </main>
  );
}
