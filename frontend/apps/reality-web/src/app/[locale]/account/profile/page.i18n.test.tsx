/**
 * Regression test for the profile-edit i18n gap
 * (code-review-reality-web-account-pages-no-i18n).
 *
 * Before the fix, this page hardcoded English user-facing strings directly in
 * `page.tsx` (e.g. "Edit profile", "Display name", "Save changes"), so
 * sk/cs/de/pl/hu visitors always saw English. These tests pin that every
 * rendered label now flows through `useTranslations('pages.accountProfile')` —
 * under the global next-intl mock (src/test/setup.tsx) `t(key)` echoes the key,
 * so a localized label renders as its key and no raw English string survives.
 * They fail on `dev`, where the English literals exist.
 */
/// <reference types="vitest/globals" />

import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@/components/ui', () => ({ Header: () => null, Footer: () => null }));
vi.mock('@/components/auth', () => ({
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));
vi.mock('@/lib/auth-context', () => ({
  useAuth: () => ({
    user: { email: 'owner@example.com', name: 'Owner' },
    refreshSession: vi.fn(),
  }),
}));
vi.mock('@/lib/auth-api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/auth-api')>();
  return { ...actual, updateProfile: vi.fn() };
});

import ProfileEditPage from './page';

describe('profile-edit page i18n', () => {
  it('renders the heading through a translation key, not an English literal', () => {
    render(<ProfileEditPage />);
    expect(screen.getByRole('heading', { name: 'title' })).toBeInTheDocument();
  });

  it('does not render any hardcoded English user-facing strings', () => {
    render(<ProfileEditPage />);
    expect(screen.queryByText('Edit profile')).toBeNull();
    expect(screen.queryByText('Display name')).toBeNull();
    expect(screen.queryByText('Save changes')).toBeNull();
    expect(screen.queryByText('Contact support to change the email on this account.')).toBeNull();
  });

  it('drives the field labels and submit button through pages.accountProfile', () => {
    render(<ProfileEditPage />);
    expect(screen.getByText('displayNameLabel')).toBeInTheDocument();
    expect(screen.getByText('emailLabel')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'save' })).toBeInTheDocument();
  });

  // Residual gap (code-review-reality-web-realtor-account-i18n-gap): the save
  // error path rendered `AuthApiError.message` — developer-facing English —
  // directly. It must now route through the `authError.*` catalog so every
  // locale sees localized copy. Under the next-intl mock a translated message
  // renders as its key, so the raw English dev copy must not appear.
  it('shows a translated error, not raw English dev copy, when the save fails', async () => {
    const { AuthApiError, updateProfile } = await import('@/lib/auth-api');
    vi.mocked(updateProfile).mockRejectedValueOnce(
      new AuthApiError('Network error. Please check your connection.', 0, 'NETWORK_ERROR')
    );
    const { container } = render(<ProfileEditPage />);
    const nameInput = container.querySelector('input[autocomplete="name"]') as HTMLInputElement;
    fireEvent.change(nameInput, { target: { value: 'New name' } });
    fireEvent.submit(nameInput.closest('form') as HTMLElement);

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('errorNetwork');
    expect(alert).not.toHaveTextContent('Network error. Please check your connection.');
  });
});
