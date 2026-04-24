'use client';

/**
 * Password reset confirmation page (UC-47.4 confirm step).
 * Reads `token` from the URL and submits a new password.
 */

import { AuthApiError, confirmPasswordReset } from '@/lib/auth-api';
import Link from 'next/link';
import { useRouter, useSearchParams } from 'next/navigation';
import { type FormEvent, Suspense, useState } from 'react';

const MIN_PASSWORD = 8;

function ResetForm() {
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
        <h1 className="title">Invalid link</h1>
        <p className="subtitle">
          This reset link is missing a token. Request a new password reset email.
        </p>
        <Link href="/auth/forgot-password" className="link center">
          Request a new link
        </Link>
        <style jsx>{`
          .empty { padding: 8px 0; }
          .title { font-size: 1.5rem; font-weight: 700; color: #111827; margin: 0 0 4px; text-align: center; }
          .subtitle { font-size: 14px; color: #6b7280; margin: 0 0 16px; text-align: center; }
          .link { color: #2563eb; text-decoration: none; font-weight: 500; }
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
      setPasswordError('Password is required');
      invalid = true;
    } else if (password.length < MIN_PASSWORD) {
      setPasswordError(`Password must be at least ${MIN_PASSWORD} characters`);
      invalid = true;
    }
    if (confirmPassword !== password) {
      setConfirmError('Passwords do not match');
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
        setGeneralError('Could not reset password. Please try again.');
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  if (success) {
    return (
      <div className="ok">
        <h1 className="title">Password updated</h1>
        <p className="subtitle">You can now sign in with your new password.</p>
        <button
          type="button"
          className="submit"
          onClick={() => router.replace('/auth/login')}
        >
          Sign in
        </button>
        <style jsx>{`
          .ok { padding: 8px 0; }
          .title { font-size: 1.5rem; font-weight: 700; color: #111827; margin: 0 0 4px; text-align: center; }
          .subtitle { font-size: 14px; color: #6b7280; margin: 0 0 24px; text-align: center; }
          .submit { width: 100%; padding: 12px 16px; background: #2563eb; color: #fff; border: none; border-radius: 8px; font-size: 16px; font-weight: 600; cursor: pointer; }
          .submit:hover { background: #1d4ed8; }
        `}</style>
      </div>
    );
  }

  return (
    <form className="form" onSubmit={handleSubmit} noValidate>
      <h1 className="title">Set a new password</h1>
      <p className="subtitle">Choose a strong password you haven't used before.</p>

      {generalError && (
        <div className="alert" role="alert">
          {generalError}
        </div>
      )}

      <label className="field">
        <span className="label">New password</span>
        <input
          type="password"
          autoComplete="new-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          disabled={isSubmitting}
          className={`input ${passwordError ? 'input-error' : ''}`}
        />
        {passwordError ? (
          <span className="error">{passwordError}</span>
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
          className={`input ${confirmError ? 'input-error' : ''}`}
        />
        {confirmError && <span className="error">{confirmError}</span>}
      </label>

      <button type="submit" className="submit" disabled={isSubmitting}>
        {isSubmitting ? 'Updating…' : 'Update password'}
      </button>

      <style jsx>{`
        .form { display: flex; flex-direction: column; gap: 16px; }
        .title { font-size: 1.5rem; font-weight: 700; color: #111827; margin: 0; text-align: center; }
        .subtitle { font-size: 14px; color: #6b7280; margin: 0 0 8px; text-align: center; }
        .alert { padding: 12px 16px; background: #fef2f2; color: #b91c1c; border: 1px solid #fecaca; border-radius: 8px; font-size: 14px; }
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
        .page { min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 24px; background: #f9fafb; }
        .card { width: 100%; max-width: 420px; background: #fff; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,.1); padding: 32px; }
      `}</style>
    </main>
  );
}
