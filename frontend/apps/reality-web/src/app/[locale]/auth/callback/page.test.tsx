/**
 * SSO callback page integration tests — Story 79.2 (Epic 10A-SSO), reality-web.
 *
 * Closes `gap-79-2-auth-callback-e2e` for reality-web. The ppt-web SSO
 * callback already has `AuthCallbackPage.test.tsx`, but the reality-web
 * `/[locale]/auth/callback` page (cookie-session SSO consumer) shipped without
 * an integration test exercising its callback handling end-to-end.
 *
 * These tests render the REAL `SsoCallbackPage` and drive it through every
 * branch of its `useEffect`, mocking only the framework boundary
 * (`next/navigation` — `useRouter` + `useSearchParams`) and using real
 * `sessionStorage` for the CSRF state token.
 *
 * Covered guarantees:
 *   1. Happy path: no error, valid/absent state, no redirect_uri → replace('/').
 *   2. Safe redirect: a same-origin relative `redirect_uri` is honoured.
 *   3. Open-redirect protection: protocol-relative ("//evil.com") and
 *      backslash-escape ("/\\evil.com") redirect targets are rejected,
 *      falling back to '/'.
 *   4. CSRF: a state-token mismatch renders the "Login Failed" error and does
 *      NOT redirect.
 *   5. State cleanup: the saved `sso_state` nonce is consumed on success.
 *   6. Provider error: `?error=` (with and without `error_description`)
 *      renders the error and does NOT redirect.
 */
/// <reference types="vitest/globals" />

import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// ---------------------------------------------------------------------------
// Framework-boundary mock: next/navigation.
//
// The global setup (src/test/setup.tsx) mocks next/navigation with a static
// useRouter/useSearchParams. Override it here so each test can (a) capture the
// router.replace spy to assert redirect targets and (b) feed arbitrary query
// params into useSearchParams.
// ---------------------------------------------------------------------------

const replaceSpy = vi.fn<(href: string) => void>();
let currentParams = new URLSearchParams();

vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: replaceSpy,
    back: vi.fn(),
    forward: vi.fn(),
    refresh: vi.fn(),
    prefetch: vi.fn(),
  }),
  usePathname: () => '/en/auth/callback',
  useSearchParams: () => currentParams,
  useParams: () => ({ locale: 'en' }),
}));

import SsoCallbackPage, { SSO_STATE_KEY } from './page';

function renderCallback(search: string) {
  currentParams = new URLSearchParams(search);
  return render(<SsoCallbackPage />);
}

describe('SsoCallbackPage — reality-web SSO /auth/callback flow (Story 79.2)', () => {
  beforeEach(() => {
    replaceSpy.mockReset();
    sessionStorage.clear();
  });

  afterEach(() => {
    sessionStorage.clear();
  });

  it('redirects to / on success and consumes the saved state nonce', async () => {
    sessionStorage.setItem(SSO_STATE_KEY, 'nonce-1');

    renderCallback('?state=nonce-1');

    await waitFor(() => expect(replaceSpy).toHaveBeenCalledWith('/'));
    // The one-shot CSRF nonce is cleared after a successful callback.
    expect(sessionStorage.getItem(SSO_STATE_KEY)).toBeNull();
    // No error UI on the happy path.
    // With the global next-intl test mock, `t(key)` returns the key itself,
    // so the error title renders as its i18n key `title`.
    expect(screen.queryByText('title')).not.toBeInTheDocument();
  });

  it('redirects when NO nonce was issued (server-cookie-only flow, nothing to verify)', async () => {
    // No `sso_state` in sessionStorage → no nonce was issued, so there is
    // nothing to validate and the server-set-cookie flow proceeds.
    renderCallback('');

    await waitFor(() => expect(replaceSpy).toHaveBeenCalledWith('/'));
    // With the global next-intl test mock, `t(key)` returns the key itself,
    // so the error title renders as its i18n key `title`.
    expect(screen.queryByText('title')).not.toBeInTheDocument();
  });

  it('FAILS when a nonce was issued but the callback carries no state (CSRF bypass closed, #1826)', async () => {
    // A nonce was saved at flow start, but the callback arrives with no
    // `state`. The pre-fix guard skipped validation here (state absent), letting
    // an attacker-supplied callback through. It must now fail closed.
    sessionStorage.setItem(SSO_STATE_KEY, 'nonce-1');

    renderCallback('');

    await waitFor(() => {
      expect(screen.getByText('title')).toBeInTheDocument();
    });
    expect(screen.getByText('invalidState')).toBeInTheDocument();
    expect(replaceSpy).not.toHaveBeenCalled();
  });

  it('honours a same-origin relative redirect_uri', async () => {
    sessionStorage.setItem(SSO_STATE_KEY, 'nonce-1');

    renderCallback('?state=nonce-1&redirect_uri=%2Fdashboard%2Fagency');

    await waitFor(() => expect(replaceSpy).toHaveBeenCalledWith('/dashboard/agency'));
  });

  it('rejects a protocol-relative redirect_uri (open-redirect guard)', async () => {
    renderCallback('?redirect_uri=%2F%2Fevil.example.com');

    await waitFor(() => expect(replaceSpy).toHaveBeenCalledWith('/'));
    expect(replaceSpy).not.toHaveBeenCalledWith('//evil.example.com');
  });

  it('rejects a backslash-escaped redirect_uri (open-redirect guard)', async () => {
    renderCallback('?redirect_uri=%2F%5Cevil.example.com');

    await waitFor(() => expect(replaceSpy).toHaveBeenCalledWith('/'));
    expect(replaceSpy).not.toHaveBeenCalledWith('/\\evil.example.com');
  });

  it('shows Login Failed and does not redirect on a state-token mismatch', async () => {
    sessionStorage.setItem(SSO_STATE_KEY, 'nonce-1');

    renderCallback('?state=tampered');

    await waitFor(() => {
      expect(screen.getByText('title')).toBeInTheDocument();
    });
    expect(screen.getByText('invalidState')).toBeInTheDocument();
    expect(replaceSpy).not.toHaveBeenCalled();
    // A recovery affordance back to home is rendered (i18n key `returnHome`).
    expect(screen.getByRole('link', { name: 'returnHome' })).toBeInTheDocument();
  });

  it('surfaces a provider error_description and does not redirect', async () => {
    renderCallback('?error=access_denied&error_description=User%20declined%20consent');

    await waitFor(() => {
      expect(screen.getByText('User declined consent')).toBeInTheDocument();
    });
    expect(screen.getByText('title')).toBeInTheDocument();
    expect(replaceSpy).not.toHaveBeenCalled();
  });

  it('falls back to the raw error code when no error_description is present', async () => {
    renderCallback('?error=server_error');

    await waitFor(() => {
      expect(screen.getByText('server_error')).toBeInTheDocument();
    });
    expect(replaceSpy).not.toHaveBeenCalled();
  });
});
