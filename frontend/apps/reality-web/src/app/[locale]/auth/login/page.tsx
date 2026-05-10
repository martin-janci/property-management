'use client';

/**
 * Email/password login page (UC-47.2).
 *
 * Calls reality-server `/api/v1/users/login` via auth-api.login(). The
 * existing SSO callback flow remains intact for federated sign-ins.
 *
 * Form fields use the modern form primitives from `@/components/forms`
 * (floating labels + custom focus ring), matching the design bundle's
 * `m-field / m-input` rules.
 *
 * All user-visible strings come from `messages/<locale>.json -> auth.login`.
 * If you add a new string, add it to all 6 locale files (sk/cs/de/en/hu/pl).
 *
 * Internal links use the locale-aware `Link` from `src/i18n/routing` so
 * non-default locales (cs/de/en/hu/pl) keep their `/[locale]/...` prefix.
 */

import { useRouter, useSearchParams } from 'next/navigation';
import { useTranslations } from 'next-intl';
import { type FormEvent, Suspense, useState } from 'react';
import { FieldInput } from '@/components/forms';
import { Link } from '@/i18n/routing';
import { AuthApiError, login } from '@/lib/auth-api';
import { useAuth } from '@/lib/auth-context';

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

function LoginForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const redirectTo = searchParams.get('redirect') ?? '/';
  const t = useTranslations('auth.login');
  // refreshSession() picks up the bearer token written to localStorage by
  // auth-api.login() so the header updates without a hard reload.
  const { refreshSession } = useAuth();

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
      setEmailError(t('emailRequired'));
      invalid = true;
    } else if (!EMAIL_RE.test(email.trim())) {
      setEmailError(t('emailInvalid'));
      invalid = true;
    }
    if (!password) {
      setPasswordError(t('passwordRequired'));
      invalid = true;
    }
    if (invalid) return;

    setIsSubmitting(true);
    try {
      await login(email.trim(), password);
      // Update auth-context state from the freshly-stored token so the
      // header re-renders to "logged in" before the navigation happens.
      await refreshSession();
      const safe = redirectTo.startsWith('/') && !redirectTo.startsWith('//') ? redirectTo : '/';
      router.replace(safe);
    } catch (error) {
      if (error instanceof AuthApiError) {
        if (error.status === 401) setGeneralError(t('incorrectCredentials'));
        else setGeneralError(error.message);
      } else {
        setGeneralError(t('genericError'));
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

      <FieldInput
        type="email"
        name="email"
        autoComplete="email"
        label={t('emailLabel')}
        value={email}
        onChange={(e) => setEmail(e.target.value)}
        disabled={isSubmitting}
        error={emailError}
        required
      />

      <FieldInput
        type="password"
        name="password"
        autoComplete="current-password"
        label={t('passwordLabel')}
        value={password}
        onChange={(e) => setPassword(e.target.value)}
        disabled={isSubmitting}
        error={passwordError}
        required
      />

      <div className="row">
        <Link href="/auth/forgot-password" className="muted-link">
          {t('forgotPassword')}
        </Link>
      </div>

      <button type="submit" className="submit" disabled={isSubmitting}>
        {isSubmitting ? t('submitting') : t('submit')}
      </button>

      <p className="meta">
        {t('noAccount')}{' '}
        <Link href="/auth/register" className="link">
          {t('createOne')}
        </Link>
      </p>

      <style jsx>{`
        .form {
          display: flex;
          flex-direction: column;
          gap: 14px;
        }
        .alert {
          padding: 12px 16px;
          background: var(--ppt-color-danger-light);
          color: var(--ppt-color-danger-dark);
          border: 1px solid var(--ppt-color-danger);
          border-radius: 8px;
          font-size: 14px;
        }
        .row {
          display: flex;
          justify-content: flex-end;
          margin-top: -4px;
        }
        .muted-link {
          font-size: 13.5px;
          color: var(--ppt-color-primary);
          text-decoration: none;
        }
        .muted-link:hover {
          text-decoration: underline;
        }
        .submit {
          margin-top: 8px;
          padding: 12px 16px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
          border-radius: 8px;
          font-size: 15px;
          font-weight: 600;
          font-family: var(--ppt-font-family);
          cursor: pointer;
          transition: background var(--ppt-transition-fast);
        }
        .submit:hover:not(:disabled) {
          background: var(--ppt-color-primary-hover);
        }
        .submit:focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }
        .submit:disabled {
          background: var(--ppt-brand-500);
          cursor: not-allowed;
          opacity: 0.7;
        }
        .meta {
          text-align: center;
          font-size: 14px;
          color: var(--ppt-fg-muted);
          margin: 8px 0 0;
        }
        .link {
          color: var(--ppt-color-primary);
          text-decoration: none;
          font-weight: 500;
        }
        .link:hover {
          text-decoration: underline;
        }
      `}</style>
    </form>
  );
}

export default function LoginPage() {
  const t = useTranslations('auth.login');
  return (
    <main className="page">
      <div className="card">
        <h1 className="title">{t('title')}</h1>
        <p className="subtitle">{t('subtitle')}</p>
        <Suspense fallback={<p>{t('loading')}</p>}>
          <LoginForm />
        </Suspense>
      </div>
      <style jsx>{`
        .page {
          min-height: 100vh;
          display: flex;
          align-items: center;
          justify-content: center;
          padding: 24px;
          background: var(--ppt-bg-app);
        }
        .card {
          width: 100%;
          max-width: 420px;
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 12px;
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
          padding: 32px;
        }
        .title {
          font-size: 1.5rem;
          font-weight: 700;
          color: var(--ppt-fg-primary);
          margin: 0 0 4px;
          text-align: center;
        }
        .subtitle {
          font-size: 14px;
          color: var(--ppt-fg-muted);
          margin: 0 0 24px;
          text-align: center;
        }
      `}</style>
    </main>
  );
}
