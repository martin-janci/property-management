'use client';

/**
 * Forgot password page (UC-47.4 request step).
 * Calls reality-server `/api/v1/auth/password-reset` to email a reset link.
 */

import Link from 'next/link';
import { type FormEvent, useState } from 'react';
import { AuthApiError, requestPasswordReset } from '@/lib/auth-api';

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
      if (error instanceof AuthApiError) {
        // Surface real errors to the user instead of silently showing the
        // "Check your inbox" confirmation. NOT_IMPLEMENTED comes from the
        // wrapper while reality-server still lacks a password-reset
        // endpoint; network failures should also be visible.
        if (error.code === 'NOT_IMPLEMENTED' || error.status === 501) {
          setGeneralError(error.message);
        } else if (error.code === 'NETWORK_ERROR') {
          setGeneralError('Network error. Please check your connection and try again.');
        } else if (error.status >= 500) {
          setGeneralError(error.message || 'Server error. Please try again later.');
        } else {
          // 4xx and other client-side errors: don't leak whether an account
          // exists for this email, fall through to the confirmation screen.
          setSubmitted(true);
        }
      } else {
        setGeneralError('Could not send reset email. Please try again.');
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
