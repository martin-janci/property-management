'use client';

/**
 * SSO callback page (Epic 10A-SSO).
 * Handles the redirect from PM OAuth and validates the session.
 */

import { useRouter, useSearchParams } from 'next/navigation';
import { useTranslations } from 'next-intl';
import { type CSSProperties, Suspense, useEffect, useState } from 'react';

/**
 * sessionStorage key holding the one-shot CSRF nonce issued when the SSO flow
 * starts. Exported so the page and its tests single-source the contract (a
 * rename here can't silently drift the test) — GH #1826 finding-3.
 */
export const SSO_STATE_KEY = 'sso_state';

const styles: Record<string, CSSProperties> = {
  container: {
    minHeight: '100vh',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  content: {
    textAlign: 'center',
    padding: '32px',
  },
  errorTitle: {
    fontSize: '1.5rem',
    fontWeight: 'bold',
    color: 'var(--ppt-color-danger-hover)',
    marginBottom: '16px',
  },
  errorMessage: {
    color: 'var(--ppt-neutral-600)',
    marginBottom: '24px',
  },
  button: {
    display: 'inline-block',
    padding: '8px 24px',
    backgroundColor: 'var(--ppt-color-primary)',
    color: 'var(--ppt-fg-on-accent)',
    borderRadius: '8px',
    textDecoration: 'none',
  },
  spinner: {
    width: '48px',
    height: '48px',
    border: '3px solid var(--ppt-border-default)',
    borderTopColor: 'var(--ppt-color-primary)',
    borderRadius: '50%',
    animation: 'spin 1s linear infinite',
    margin: '0 auto 16px',
  },
  loadingText: {
    color: 'var(--ppt-neutral-600)',
  },
};

function LoadingSpinner() {
  return (
    <main style={styles.container}>
      <div style={styles.content}>
        <div style={styles.spinner} />
        <p style={styles.loadingText}>Loading...</p>
        <style>{`
          @keyframes spin {
            from { transform: rotate(0deg); }
            to { transform: rotate(360deg); }
          }
        `}</style>
      </div>
    </main>
  );
}

function SsoCallbackContent() {
  const t = useTranslations('auth.callback');
  const router = useRouter();
  const searchParams = useSearchParams();
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Check for error in URL params
    const errorParam = searchParams.get('error');
    const errorDescription = searchParams.get('error_description');

    if (errorParam) {
      setError(errorDescription || errorParam);
      return;
    }

    // Verify CSRF state token.
    //
    // Whenever a nonce was issued at the start of the flow (`savedState`
    // present), a matching `state` is REQUIRED: a missing or mismatched value
    // is a failure, not a pass. The previous `state && savedState && …` form
    // skipped the check entirely when `state` was absent/empty, so an
    // attacker-supplied callback with no `state` bypassed the nonce check
    // (GH #1826 finding-2). When no nonce was saved (server-cookie-only flow,
    // nothing to verify) the check is correctly a no-op.
    const state = searchParams.get('state');
    const savedState = sessionStorage.getItem(SSO_STATE_KEY);

    if (savedState && state !== savedState) {
      setError(t('invalidState'));
      return;
    }

    // Clear saved state
    sessionStorage.removeItem(SSO_STATE_KEY);

    // Redirect to home - session cookie should already be set by the server.
    // Only accept same-origin relative paths to prevent open-redirect attacks:
    //   - must start with "/" (path-only)
    //   - must NOT start with "//" or "/\" (protocol-relative URL escape)
    const rawRedirect = searchParams.get('redirect_uri');
    const isSafeRedirect =
      typeof rawRedirect === 'string' &&
      rawRedirect.startsWith('/') &&
      !rawRedirect.startsWith('//') &&
      !rawRedirect.startsWith('/\\');
    router.replace(isSafeRedirect ? rawRedirect : '/');
  }, [router, searchParams, t]);

  if (error) {
    return (
      <main style={styles.container}>
        <div style={styles.content}>
          <h1 style={styles.errorTitle}>{t('title')}</h1>
          <p style={styles.errorMessage}>{error}</p>
          <a href="/" style={styles.button}>
            {t('returnHome')}
          </a>
        </div>
      </main>
    );
  }

  return (
    <main style={styles.container}>
      <div style={styles.content}>
        <div style={styles.spinner} />
        <p style={styles.loadingText}>Completing login...</p>
        <style>{`
          @keyframes spin {
            from { transform: rotate(0deg); }
            to { transform: rotate(360deg); }
          }
        `}</style>
      </div>
    </main>
  );
}

export default function SsoCallbackPage() {
  return (
    <Suspense fallback={<LoadingSpinner />}>
      <SsoCallbackContent />
    </Suspense>
  );
}
