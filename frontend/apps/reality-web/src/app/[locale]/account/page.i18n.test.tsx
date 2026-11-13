/**
 * Regression test for the account-dashboard i18n gap
 * (code-review-reality-web-account-pages-no-i18n).
 *
 * Before the fix, this page hardcoded English user-facing strings directly in
 * `page.tsx` (e.g. "Account", "Sign out", the section titles/descriptions), so
 * sk/cs/de/pl/hu visitors always saw English. These tests pin that every
 * rendered label now flows through `useTranslations('pages.account')` instead
 * of a literal — under the next-intl mock `t(key)` / `t.rich(key)` echo the key,
 * so a localized label renders as its key and no raw English string survives.
 * They fail on `dev`, where the English literals exist.
 */
/// <reference types="vitest/globals" />

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

// Override the global next-intl mock (src/test/setup.tsx) to add `t.rich`,
// which the "signed in as" line uses — the setup mock's `t` has no `.rich`.
// Both `t(key)` and `t.rich(key, …)` echo the key verbatim, so the assertions
// pin message KEYS rather than any one locale's copy.
vi.mock('next-intl', () => {
  const t = Object.assign((key: string) => key, { rich: (key: string) => key });
  return {
    useTranslations: () => t,
    useLocale: () => 'en',
    useMessages: () => ({}),
    NextIntlClientProvider: ({ children }: { children: React.ReactNode }) => children,
  };
});

vi.mock('@/components/ui', () => ({ Header: () => null, Footer: () => null }));
vi.mock('@/components/auth', () => ({
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));
vi.mock('@/lib/auth-context', () => ({
  useAuth: () => ({
    user: { email: 'owner@example.com', name: 'Owner' },
    logout: vi.fn(),
  }),
}));

import AccountPage from './page';

describe('account dashboard page i18n', () => {
  it('renders the heading through a translation key, not an English literal', () => {
    render(<AccountPage />);
    expect(screen.getByRole('heading', { name: 'title' })).toBeInTheDocument();
  });

  it('does not render any hardcoded English user-facing strings', () => {
    render(<AccountPage />);
    expect(screen.queryByText('Account')).toBeNull();
    expect(screen.queryByText('Sign out')).toBeNull();
    expect(screen.queryByText('Saved searches')).toBeNull();
    expect(screen.queryByText('Update your name, contact details and avatar.')).toBeNull();
  });

  it('drives the sign-out button and section labels through pages.account', () => {
    render(<AccountPage />);
    expect(screen.getByRole('button', { name: 'signOut' })).toBeInTheDocument();
    expect(screen.getByText('sections.profile.title')).toBeInTheDocument();
    expect(screen.getByText('sections.savedSearches.description')).toBeInTheDocument();
  });
});
