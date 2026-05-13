/**
 * CookieConsentBanner — behaviour tests.
 *
 * Verifies the three states the component owns:
 *  1. shows on first visit (storage empty)
 *  2. hides + writes "accepted" when Accept all is clicked
 *  3. hides + writes "essential" when Reject optional is clicked
 *  4. does NOT show when storage already has a recorded choice
 *
 * Storage key contract (`ppt-cookie-consent`) must stay stable — any
 * future analytics/marketing tag-loader will read it as the gate.
 */

import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CookieConsentBanner } from './CookieConsentBanner';

vi.mock('next-intl', () => ({
  useTranslations: () => (key: string) => key,
}));

vi.mock('@/i18n/routing', () => ({
  Link: ({ children, href }: { children: React.ReactNode; href: string }) => (
    <a href={href}>{children}</a>
  ),
}));

const STORAGE_KEY = 'ppt-cookie-consent';

describe('CookieConsentBanner', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    window.localStorage.clear();
  });

  it('shows on first visit when storage is empty', () => {
    render(<CookieConsentBanner />);
    // The translation mock returns the key itself, so look for "title".
    expect(screen.getByRole('region', { name: 'title' })).toBeInTheDocument();
    expect(screen.getByText('accept')).toBeInTheDocument();
    expect(screen.getByText('reject')).toBeInTheDocument();
  });

  it('does NOT show when storage already has a choice', () => {
    window.localStorage.setItem(STORAGE_KEY, 'accepted');
    render(<CookieConsentBanner />);
    expect(screen.queryByRole('region', { name: 'title' })).not.toBeInTheDocument();
  });

  it('persists "accepted" and hides when Accept all is clicked', () => {
    render(<CookieConsentBanner />);
    fireEvent.click(screen.getByText('accept'));
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('accepted');
    expect(screen.queryByRole('region', { name: 'title' })).not.toBeInTheDocument();
  });

  it('persists "essential" and hides when Reject optional is clicked', () => {
    render(<CookieConsentBanner />);
    fireEvent.click(screen.getByText('reject'));
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('essential');
    expect(screen.queryByRole('region', { name: 'title' })).not.toBeInTheDocument();
  });

  it('also treats the "essential" stored value as already-decided', () => {
    window.localStorage.setItem(STORAGE_KEY, 'essential');
    render(<CookieConsentBanner />);
    expect(screen.queryByRole('region', { name: 'title' })).not.toBeInTheDocument();
  });

  it('Escape key registers as rejecting optional cookies (conservative default)', () => {
    render(<CookieConsentBanner />);
    const region = screen.getByRole('region', { name: 'title' });
    fireEvent.keyDown(region, { key: 'Escape' });
    expect(window.localStorage.getItem(STORAGE_KEY)).toBe('essential');
    expect(screen.queryByRole('region', { name: 'title' })).not.toBeInTheDocument();
  });
});
