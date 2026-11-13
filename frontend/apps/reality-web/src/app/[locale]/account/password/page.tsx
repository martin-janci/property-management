'use client';

/**
 * Change password page (UC-47.4 authenticated variant).
 * Calls reality-server `POST /api/v1/auth/me/password`.
 */

import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { type FormEvent, useState } from 'react';
import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import { AuthApiError, changePassword } from '@/lib/auth-api';

const MIN_PASSWORD = 8;

interface FieldErrors {
  currentPassword?: string;
  newPassword?: string;
  confirmPassword?: string;
}

function ChangePasswordForm() {
  const t = useTranslations('pages.accountPassword');
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [errors, setErrors] = useState<FieldErrors>({});
  const [generalError, setGeneralError] = useState<string>();
  const [success, setSuccess] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setGeneralError(undefined);
    setSuccess(false);
    const next: FieldErrors = {};
    if (!currentPassword) next.currentPassword = t('currentRequired');
    if (!newPassword) next.newPassword = t('newRequired');
    else if (newPassword.length < MIN_PASSWORD)
      next.newPassword = t('minLength', { min: MIN_PASSWORD });
    else if (newPassword === currentPassword) next.newPassword = t('mustDiffer');
    if (confirmPassword !== newPassword) next.confirmPassword = t('mismatch');
    setErrors(next);
    if (Object.keys(next).length > 0) return;

    setIsSubmitting(true);
    try {
      await changePassword(currentPassword, newPassword);
      setSuccess(true);
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
    } catch (err) {
      if (err instanceof AuthApiError && err.status === 401) {
        setErrors({ currentPassword: t('currentIncorrect') });
      } else if (err instanceof AuthApiError) {
        setGeneralError(err.message);
      } else {
        setGeneralError(t('updateFailed'));
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <form className="form" onSubmit={handleSubmit} noValidate>
      <h1 className="title">{t('title')}</h1>
      <p className="subtitle">{t('subtitle')}</p>

      {generalError && (
        <div className="alert" role="alert">
          {generalError}
        </div>
      )}
      {success && (
        <div className="success" role="status">
          {t('updated')}
        </div>
      )}

      <label className="field">
        <span className="label">{t('currentLabel')}</span>
        <input
          type="password"
          autoComplete="current-password"
          value={currentPassword}
          onChange={(e) => setCurrentPassword(e.target.value)}
          disabled={isSubmitting}
          className={`input ${errors.currentPassword ? 'input-error' : ''}`}
        />
        {errors.currentPassword && <span className="error">{errors.currentPassword}</span>}
      </label>

      <label className="field">
        <span className="label">{t('newLabel')}</span>
        <input
          type="password"
          autoComplete="new-password"
          value={newPassword}
          onChange={(e) => setNewPassword(e.target.value)}
          disabled={isSubmitting}
          className={`input ${errors.newPassword ? 'input-error' : ''}`}
        />
        {errors.newPassword ? (
          <span className="error">{errors.newPassword}</span>
        ) : (
          <span className="hint">{t('hint', { min: MIN_PASSWORD })}</span>
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
          className={`input ${errors.confirmPassword ? 'input-error' : ''}`}
        />
        {errors.confirmPassword && <span className="error">{errors.confirmPassword}</span>}
      </label>

      <button type="submit" className="submit" disabled={isSubmitting}>
        {isSubmitting ? t('submitting') : t('submit')}
      </button>

      <p className="meta">
        <Link href="/account" className="link">
          {t('backToAccount')}
        </Link>
      </p>

      <style jsx>{`
        .form {
          max-width: 460px; margin: 32px auto; padding: 32px;
          background: var(--ppt-bg-surface); border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,.1);
          display: flex; flex-direction: column; gap: 16px;
        }
        .title { font-size: 1.5rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0 0 4px; }
        .subtitle { font-size: 14px; color: var(--ppt-fg-muted); margin: 0 0 8px; }
        .alert { padding: 12px 16px; background: var(--ppt-color-danger-light); color: var(--ppt-color-danger-dark); border: 1px solid var(--ppt-color-danger-light); border-radius: 8px; font-size: 14px; }
        .success { padding: 12px 16px; background: var(--ppt-color-success-light); color: var(--ppt-color-success-hover); border: 1px solid var(--ppt-color-success-light); border-radius: 8px; font-size: 14px; }
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
        .meta { text-align: center; margin: 0; }
        .link { color: var(--ppt-color-primary); text-decoration: none; font-weight: 500; font-size: 14px; }
        .link:hover { text-decoration: underline; }
      `}</style>
    </form>
  );
}

export default function ChangePasswordPage() {
  return (
    <div className="page">
      <Header />
      <main>
        <ProtectedRoute>
          <ChangePasswordForm />
        </ProtectedRoute>
      </main>
      <Footer />
      <style jsx>{`
        .page { min-height: 100vh; display: flex; flex-direction: column; background: var(--ppt-bg-app); }
        main { flex: 1; }
      `}</style>
    </div>
  );
}
