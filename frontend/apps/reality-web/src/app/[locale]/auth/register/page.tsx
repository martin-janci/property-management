'use client';

/**
 * Registration page (UC-47.1).
 * Calls reality-server `/api/v1/auth/register`.
 */

import { AuthApiError, register } from '@/lib/auth-api';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { type FormEvent, useState } from 'react';

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const MIN_PASSWORD = 8;

interface FieldErrors {
  email?: string;
  password?: string;
  confirmPassword?: string;
  displayName?: string;
}

export default function RegisterPage() {
  const router = useRouter();
  const [displayName, setDisplayName] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
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
    setErrors(next);
    if (Object.keys(next).length > 0) return;

    setIsSubmitting(true);
    try {
      await register({
        email: email.trim(),
        password,
        displayName: displayName.trim(),
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
            <h1 className="title">Check your inbox</h1>
            <p className="subtitle">
              We sent a verification email to <strong>{email}</strong>. Click the link to activate
              your account.
            </p>
            <button type="button" className="submit" onClick={() => router.push('/auth/login')}>
              Back to sign in
            </button>
          </>
        ) : (
          <>
            <h1 className="title">Create your account</h1>
            <p className="subtitle">Save listings, set alerts and contact agents.</p>

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
                {errors.confirmPassword && (
                  <span className="error">{errors.confirmPassword}</span>
                )}
              </label>

              <button type="submit" className="submit" disabled={isSubmitting}>
                {isSubmitting ? 'Creating account…' : 'Create account'}
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
        .page { min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 24px; background: #f9fafb; }
        .card { width: 100%; max-width: 460px; background: #fff; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,.1); padding: 32px; }
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
        .hint { color: #6b7280; font-size: 12px; }
        .submit { margin-top: 8px; padding: 12px 16px; background: #2563eb; color: #fff; border: none; border-radius: 8px; font-size: 16px; font-weight: 600; cursor: pointer; }
        .submit:hover:not(:disabled) { background: #1d4ed8; }
        .submit:disabled { background: #93c5fd; cursor: not-allowed; }
        .meta { text-align: center; font-size: 14px; color: #4b5563; margin: 8px 0 0; }
        .link { color: #2563eb; text-decoration: none; font-weight: 500; }
        .link:hover { text-decoration: underline; }
      `}</style>
    </main>
  );
}
