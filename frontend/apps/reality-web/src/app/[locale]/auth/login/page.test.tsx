/**
 * Login page redirect tests — reality-web `/[locale]/auth/login`.
 *
 * Regression guard for the code-review finding "reality web auth nav drops
 * locale": the post-login redirect must go through the locale-aware
 * `useRouter` from `@/i18n/routing` so the active `/[locale]/…` prefix is
 * preserved. Using `next/navigation`'s router drops the locale and bounces
 * non-default locales (cs/de/en/hu/pl) back to the default — exactly the bug
 * the SSO callback page already fixed.
 *
 * The page renders the REAL `LoginPage`, mocking only the framework boundary
 * (`@/i18n/routing` + `next/navigation`) and the auth side effects
 * (`@/lib/auth-api` login + `@/lib/auth-context` useAuth).
 */
/// <reference types="vitest/globals" />

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// ---------------------------------------------------------------------------
// Framework-boundary mocks.
//
//   - `@/i18n/routing`: capture the locale-aware `router.replace` spy. The
//     post-login redirect MUST use this one so the locale prefix survives.
//   - `next/navigation`: feed `?redirect=` into `useSearchParams` and expose a
//     separate `replace` spy so we can assert the locale-dropping router is NOT
//     the one that performs the redirect.
// ---------------------------------------------------------------------------

const localeAwareReplaceSpy = vi.fn<(href: string) => void>();
const nextNavReplaceSpy = vi.fn<(href: string) => void>();
let currentParams = new URLSearchParams();

vi.mock('@/i18n/routing', () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: localeAwareReplaceSpy,
    back: vi.fn(),
    forward: vi.fn(),
    refresh: vi.fn(),
    prefetch: vi.fn(),
  }),
  Link: ({ children, href }: { children: React.ReactNode; href: string }) => (
    <a href={href}>{children}</a>
  ),
  usePathname: () => '/en/auth/login',
  getPathname: () => '/en/auth/login',
  redirect: vi.fn(),
}));

vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: nextNavReplaceSpy,
    back: vi.fn(),
    forward: vi.fn(),
    refresh: vi.fn(),
    prefetch: vi.fn(),
  }),
  usePathname: () => '/en/auth/login',
  useSearchParams: () => currentParams,
  useParams: () => ({ locale: 'en' }),
}));

const loginMock = vi.fn<(email: string, password: string) => Promise<unknown>>();
vi.mock('@/lib/auth-api', () => ({
  login: (email: string, password: string) => loginMock(email, password),
  // AuthApiError is referenced by the page's catch block.
  AuthApiError: class AuthApiError extends Error {
    status: number;
    constructor(message: string, status: number) {
      super(message);
      this.status = status;
    }
  },
}));

const refreshSessionMock = vi.fn<() => Promise<void>>();
vi.mock('@/lib/auth-context', () => ({
  useAuth: () => ({ refreshSession: refreshSessionMock }),
}));

import LoginPage from './page';

function fillAndSubmit() {
  // Global next-intl test mock returns the key itself, so labels === i18n keys.
  fireEvent.change(screen.getByLabelText('emailLabel'), {
    target: { value: 'user@example.com' },
  });
  fireEvent.change(screen.getByLabelText('passwordLabel'), {
    target: { value: 'hunter2hunter2' },
  });
  fireEvent.submit(
    screen.getByRole('button', { name: 'submit' }).closest('form') as HTMLFormElement
  );
}

describe('LoginPage — post-login redirect preserves locale', () => {
  beforeEach(() => {
    localeAwareReplaceSpy.mockReset();
    nextNavReplaceSpy.mockReset();
    loginMock.mockReset().mockResolvedValue({});
    refreshSessionMock.mockReset().mockResolvedValue(undefined);
    currentParams = new URLSearchParams();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('redirects through the locale-aware router (NOT next/navigation) after login', async () => {
    currentParams = new URLSearchParams('?redirect=%2Fdashboard%2Fagency');

    render(<LoginPage />);
    fillAndSubmit();

    // The redirect must go through `@/i18n/routing`'s router so the active
    // `/[locale]/…` prefix is preserved.
    await waitFor(() => expect(localeAwareReplaceSpy).toHaveBeenCalledWith('/dashboard/agency'));
    // And must NOT go through the locale-dropping `next/navigation` router.
    expect(nextNavReplaceSpy).not.toHaveBeenCalled();
  });

  it('falls back to "/" through the locale-aware router when no redirect is given', async () => {
    render(<LoginPage />);
    fillAndSubmit();

    await waitFor(() => expect(localeAwareReplaceSpy).toHaveBeenCalledWith('/'));
    expect(nextNavReplaceSpy).not.toHaveBeenCalled();
  });
});
