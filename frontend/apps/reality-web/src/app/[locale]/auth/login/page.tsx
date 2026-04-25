'use client';

/**
 * Email/password login page (UC-47.2).
 *
 * Calls reality-server `/api/v1/auth/login` directly. The existing SSO
 * callback flow remains intact for federated sign-ins.
 */

import { AuthApiError, login } from '@/lib/auth-api';
import Link from 'next/link';
import { useRouter, useSearchParams } from 'next/navigation';
import { type FormEvent, Suspense, useState } from 'react';

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

function LoginForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const redirectTo = searchParams.get('redirect') ?? '/';

  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [emailError, setEmailError] = useState<string>();
  const [passwordError, setPasswordError] = useState<string>();
  const [generalError, setGeneralError] = useState<string>();
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setEmailError(undefined);
    setPasswordError(undefined);
    setGeneralError(undefined);

    let invalid = false;
    if (!email.trim()) {
      setEmailError('Email is required');
      invalid = true;
    } else if (!EMAIL_RE.test(email.trim())) {
      setEmailError('Enter a valid email address');
      invalid = true;
    }
    if (!password) {
      setPasswordError('Password is required');
      invalid = true;
    }
    if (invalid) return;

    setIsSubmitting(true);
    try {
      await login(email.trim(), password);
      const safe = redirectTo.startsWith('/') && !redirectTo.startsWith('//') ? redirectTo : '/';
      router.replace(safe);
    } catch (error) {
      if (error instanceof AuthApiError) {
        if (error.status === 401) setGeneralError('Incorrect email or password.');
        else setGeneralError(error.message);
      } else {
        setGeneralError('Sign in failed. Please try again.');
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
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
          name="email"
          autoComplete="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          disabled={isSubmitting}
          className={`input ${emailError ? 'input-error' : ''}`}
        />
        {emailError && <span className="error">{emailError}</span>}
      </label>

      <label className="field">
        <span className="label">Password</span>
        <input
          type="password"
          name="password"
          autoComplete="current-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          disabled={isSubmitting}
          className={`input ${passwordError ? 'input-error' : ''}`}
        />
        {passwordError && <span className="error">{passwordError}</span>}
      </label>

      <div className="row">
        <Link href="/auth/forgot-password" className="muted-link">
          Forgot password?
        </Link>
      </div>

      <button type="submit" className="submit" disabled={isSubmitting}>
        {isSubmitting ? 'Signing in…' : 'Sign in'}
      </button>

      <p className="meta">
        Don't have an account?{' '}
        <Link href="/auth/register" className="link">
          Create one
        </Link>
      </p>

      <style jsx>{`
        .form { display: flex; flex-direction: column; gap: 16px; }
        .alert {
          padding: 12px 16px; background: #fef2f2; color: #b91c1c;
          border: 1px solid #fecaca; border-radius: 8px; font-size: 14px;
        }
        .field { display: flex; flex-direction: column; gap: 6px; }
        .label { font-size: 14px; font-weight: 500; color: #374151; }
        .input {
          padding: 10px 12px; font-size: 16px; border: 1px solid #d1d5db;
          border-radius: 8px; background: #fff; color: #111827;
        }
        .input:focus { outline: none; border-color: #2563eb; box-shadow: 0 0 0 3px rgba(37,99,235,.1); }
        .input-error { border-color: #dc2626; }
        .error { color: #dc2626; font-size: 12px; }
        .row { display: flex; justify-content: flex-end; }
        .muted-link { font-size: 14px; color: #2563eb; text-decoration: none; }
        .muted-link:hover { text-decoration: underline; }
        .submit {
          margin-top: 8px; padding: 12px 16px; background: #2563eb; color: #fff;
          border: none; border-radius: 8px; font-size: 16px; font-weight: 600; cursor: pointer;
        }
        .submit:hover:not(:disabled) { background: #1d4ed8; }
        .submit:disabled { background: #93c5fd; cursor: not-allowed; }
        .meta { text-align: center; font-size: 14px; color: #4b5563; margin: 8px 0 0; }
        .link { color: #2563eb; text-decoration: none; font-weight: 500; }
        .link:hover { text-decoration: underline; }
      `}</style>
    </form>
  );
}

export default function LoginPage() {
  return (
    <main className="page">
      <div className="card">
        <h1 className="title">Sign in</h1>
        <p className="subtitle">Welcome back to Reality Portal.</p>
        <Suspense fallback={<p>Loading…</p>}>
          <LoginForm />
        </Suspense>
      </div>
      <style jsx>{`
        .page {
          min-height: 100vh; display: flex; align-items: center; justify-content: center;
          padding: 24px; background: #f9fafb;
        }
        .card {
          width: 100%; max-width: 420px; background: #fff; border-radius: 12px;
          box-shadow: 0 1px 3px rgba(0,0,0,.1); padding: 32px;
        }
        .title { font-size: 1.5rem; font-weight: 700; color: #111827; margin: 0 0 4px; text-align: center; }
        .subtitle { font-size: 14px; color: #6b7280; margin: 0 0 24px; text-align: center; }
      `}</style>
    </main>
  );
}
