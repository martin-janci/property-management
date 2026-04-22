'use client';

/**
 * SSO callback page (Epic 10A-SSO).
 * Handles the redirect from PM OAuth and validates the session.
 */

import { useRouter, useSearchParams } from 'next/navigation';
import { type CSSProperties, Suspense, useEffect, useState } from 'react';

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
    color: '#dc2626',
    marginBottom: '16px',
  },
  errorMessage: {
    color: '#4b5563',
    marginBottom: '24px',
  },
  button: {
    display: 'inline-block',
    padding: '8px 24px',
    backgroundColor: '#2563eb',
    color: '#fff',
    borderRadius: '8px',
    textDecoration: 'none',
  },
  spinner: {
    width: '48px',
    height: '48px',
    border: '3px solid #e5e7eb',
    borderTopColor: '#2563eb',
    borderRadius: '50%',
    animation: 'spin 1s linear infinite',
    margin: '0 auto 16px',
  },
  loadingText: {
    color: '#4b5563',
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

    // Verify CSRF state token
    const state = searchParams.get('state');
    const savedState = sessionStorage.getItem('sso_state');

    if (state && savedState && state !== savedState) {
      setError('Invalid state token. Please try logging in again.');
      return;
    }

    // Clear saved state
    sessionStorage.removeItem('sso_state');

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
  }, [router, searchParams]);

  if (error) {
    return (
      <main style={styles.container}>
        <div style={styles.content}>
          <h1 style={styles.errorTitle}>Login Failed</h1>
          <p style={styles.errorMessage}>{error}</p>
          <a href="/" style={styles.button}>
            Return Home
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
