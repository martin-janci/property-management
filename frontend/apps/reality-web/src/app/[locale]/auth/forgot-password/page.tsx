'use client';

/**
 * Forgot password page (UC-47.4 request step).
 * Calls reality-server `/api/v1/auth/password-reset` to email a reset link.
 */

import { AuthApiError, requestPasswordReset } from '@/lib/auth-api';
import Link from 'next/link';
import { type FormEvent, useState } from 'react';

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export default function ForgotPasswordPage() {
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
    if (!trimmed) return setEmailError('Email is required');
    if (!EMAIL_RE.test(trimmed)) return setEmailError('Enter a valid email address');

    setIsSubmitting(true);
    try {
      await requestPasswordReset(trimmed);
      setSubmitted(true);
    } catch (error) {
      // Don't leak account existence on most errors; only surface network failures.
      if (error instanceof AuthApiError && error.code === 'NETWORK_ERROR') {
        setGeneralError('Network error. Please check your connection and try again.');
      } else {
        setSubmitted(true);
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
            <h1 className="title">Check your inbox</h1>
            <p className="subtitle">
              If an account exists for <strong>{email}</strong>, we've sent instructions to reset
              your password.
            </p>
            <Link href="/auth/login" className="link center">
              Back to sign in
            </Link>
          </>
        ) : (
          <>
            <h1 className="title">Reset your password</h1>
            <p className="subtitle">Enter your email and we'll send you a reset link.</p>
            <form className="form" onSubmit={handleSubmit} noValidate>
              {generalError && (
                <div className="alert" role="alert">
                  {generalError}
                </div>
              )}
              <label className="field">
                <span className="label">Email</span>
                <input
                  type="email"
                  autoComplete="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  disabled={isSubmitting}
                  className={`input ${emailError ? 'input-error' : ''}`}
                />
                {emailError && <span className="error">{emailError}</span>}
              </label>
              <button type="submit" className="submit" disabled={isSubmitting}>
                {isSubmitting ? 'Sending…' : 'Send reset link'}
              </button>
              <p className="meta">
                Remembered your password?{' '}
                <Link href="/auth/login" className="link">
                  Sign in
                </Link>
              </p>
            </form>
          </>
        )}
      </div>

      <style jsx>{`
        .page { min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 24px; background: #f9fafb; }
        .card { width: 100%; max-width: 420px; background: #fff; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,.1); padding: 32px; }
        .title { font-size: 1.5rem; font-weight: 700; color: #111827; margin: 0 0 4px; text-align: center; }
        .subtitle { font-size: 14px; color: #6b7280; margin: 0 0 24px; text-align: center; }
        .form { display: flex; flex-direction: column; gap: 16px; }
        .alert { padding: 12px 16px; background: #fef2f2; color: #b91c1c; border: 1px solid #fecaca; border-radius: 8px; font-size: 14px; }
        .field { display: flex; flex-direction: column; gap: 6px; }
        .label { font-size: 14px; font-weight: 500; color: #374151; }
        .input { padding: 10px 12px; font-size: 16px; border: 1px solid #d1d5db; border-radius: 8px; background: #fff; color: #111827; }
        .input:focus { outline: none; border-color: #2563eb; box-shadow: 0 0 0 3px rgba(37,99,235,.1); }
        .input-error { border-color: #dc2626; }
        .error { color: #dc2626; font-size: 12px; }
        .submit { margin-top: 8px; padding: 12px 16px; background: #2563eb; color: #fff; border: none; border-radius: 8px; font-size: 16px; font-weight: 600; cursor: pointer; }
        .submit:hover:not(:disabled) { background: #1d4ed8; }
        .submit:disabled { background: #93c5fd; cursor: not-allowed; }
        .meta { text-align: center; font-size: 14px; color: #4b5563; margin: 8px 0 0; }
        .link { color: #2563eb; text-decoration: none; font-weight: 500; }
        .link:hover { text-decoration: underline; }
        .center { display: block; text-align: center; }
      `}</style>
    </main>
  );
}
