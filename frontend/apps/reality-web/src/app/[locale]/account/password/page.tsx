'use client';

/**
 * Change password page (UC-47.4 authenticated variant).
 * Calls reality-server `POST /api/v1/auth/me/password`.
 */

import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import { AuthApiError, changePassword } from '@/lib/auth-api';
import Link from 'next/link';
import { type FormEvent, useState } from 'react';

const MIN_PASSWORD = 8;

interface FieldErrors {
  currentPassword?: string;
  newPassword?: string;
  confirmPassword?: string;
}

function ChangePasswordForm() {
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
    if (!currentPassword) next.currentPassword = 'Current password is required';
    if (!newPassword) next.newPassword = 'New password is required';
    else if (newPassword.length < MIN_PASSWORD)
      next.newPassword = `Password must be at least ${MIN_PASSWORD} characters`;
    else if (newPassword === currentPassword)
      next.newPassword = 'New password must differ from the current password';
    if (confirmPassword !== newPassword) next.confirmPassword = 'Passwords do not match';
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
        setErrors({ currentPassword: 'Current password is incorrect.' });
      } else if (err instanceof AuthApiError) {
        setGeneralError(err.message);
      } else {
        setGeneralError('Could not change password. Please try again.');
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <form className="form" onSubmit={handleSubmit} noValidate>
      <h1 className="title">Change password</h1>
      <p className="subtitle">Update the password used to sign in to your account.</p>

      {generalError && (
        <div className="alert" role="alert">
          {generalError}
        </div>
      )}
      {success && (
        <div className="success" role="status">
          Password updated.
        </div>
      )}

      <label className="field">
        <span className="label">Current password</span>
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
        <span className="label">New password</span>
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
          <span className="hint">At least {MIN_PASSWORD} characters.</span>
        )}
      </label>

      <label className="field">
        <span className="label">Confirm new password</span>
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
        {isSubmitting ? 'Updating…' : 'Update password'}
      </button>

      <p className="meta">
        <Link href="/account" className="link">
          Back to account
        </Link>
      </p>

      <style jsx>{`
        .form {
          max-width: 460px; margin: 32px auto; padding: 32px;
          background: #fff; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,.1);
          display: flex; flex-direction: column; gap: 16px;
        }
        .title { font-size: 1.5rem; font-weight: 700; color: #111827; margin: 0 0 4px; }
        .subtitle { font-size: 14px; color: #6b7280; margin: 0 0 8px; }
        .alert { padding: 12px 16px; background: #fef2f2; color: #b91c1c; border: 1px solid #fecaca; border-radius: 8px; font-size: 14px; }
        .success { padding: 12px 16px; background: #ecfdf5; color: #047857; border: 1px solid #a7f3d0; border-radius: 8px; font-size: 14px; }
        .field { display: flex; flex-direction: column; gap: 6px; }
        .label { font-size: 14px; font-weight: 500; color: #374151; }
        .input { padding: 10px 12px; font-size: 16px; border: 1px solid #d1d5db; border-radius: 8px; background: #fff; color: #111827; }
        .input:focus { outline: none; border-color: #2563eb; box-shadow: 0 0 0 3px rgba(37,99,235,.1); }
        .input-error { border-color: #dc2626; }
        .error { color: #dc2626; font-size: 12px; }
        .hint { color: #6b7280; font-size: 12px; }
        .submit { margin-top: 8px; padding: 12px 16px; background: #2563eb; color: #fff; border: none; border-radius: 8px; font-size: 16px; font-weight: 600; cursor: pointer; }
        .submit:hover:not(:disabled) { background: #1d4ed8; }
        .submit:disabled { background: #93c5fd; cursor: not-allowed; }
        .meta { text-align: center; margin: 0; }
        .link { color: #2563eb; text-decoration: none; font-weight: 500; font-size: 14px; }
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
        .page { min-height: 100vh; display: flex; flex-direction: column; background: #f9fafb; }
        main { flex: 1; }
      `}</style>
    </div>
  );
}
