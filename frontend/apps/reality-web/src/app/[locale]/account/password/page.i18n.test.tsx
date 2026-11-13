/**
 * Regression test for the change-password i18n gap
 * (code-review-reality-web-account-pages-no-i18n).
 *
 * Before the fix, this page hardcoded English user-facing strings directly in
 * `page.tsx` (e.g. "Change password", "Current password", "Update password",
 * the validation messages), so sk/cs/de/pl/hu visitors always saw English.
 * These tests pin that every rendered label now flows through
 * `useTranslations('pages.accountPassword')` — under the global next-intl mock
 * (src/test/setup.tsx) `t(key)` echoes the key, so a localized label renders as
 * its key and no raw English string survives. They fail on `dev`, where the
 * English literals exist.
 */
/// <reference types="vitest/globals" />

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@/components/ui', () => ({ Header: () => null, Footer: () => null }));
vi.mock('@/components/auth', () => ({
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));
vi.mock('@/lib/auth-api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/auth-api')>();
  return { ...actual, changePassword: vi.fn() };
});

import ChangePasswordPage from './page';

describe('change-password page i18n', () => {
  it('renders the heading through a translation key, not an English literal', () => {
    render(<ChangePasswordPage />);
    expect(screen.getByRole('heading', { name: 'title' })).toBeInTheDocument();
  });

  it('does not render any hardcoded English user-facing strings', () => {
    render(<ChangePasswordPage />);
    expect(screen.queryByText('Change password')).toBeNull();
    expect(screen.queryByText('Current password')).toBeNull();
    expect(screen.queryByText('Update password')).toBeNull();
    expect(screen.queryByText('Update the password used to sign in to your account.')).toBeNull();
  });

  it('surfaces validation errors through pages.accountPassword keys, not literals', () => {
    render(<ChangePasswordPage />);
    // Submit the empty form → the "required" validation keys render.
    fireEvent.submit(screen.getByRole('button', { name: 'submit' }).closest('form') as HTMLElement);
    expect(screen.getByText('currentRequired')).toBeInTheDocument();
    expect(screen.getByText('newRequired')).toBeInTheDocument();
    expect(screen.queryByText('Current password is required')).toBeNull();
  });
});
