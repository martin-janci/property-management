/**
 * Regression test for the realtor analytics i18n gap
 * (code-review-reality-web-realtor-hub-analytics-i18n).
 *
 * The user-facing string labels on this page (UC-51.10) already flow through
 * `useTranslations('pages.realtorAnalytics')` with full key parity across all
 * six locales (pinned by src/i18n/realtor-workspace-messages.test.ts). The
 * residual i18n gap was the *numbers*: the StatCards formatted their values
 * with bare `Number#toLocaleString()` (which silently follows the Node/browser
 * runtime locale, not the visitor's active next-intl locale) and rendered the
 * conversion rate as a hardcoded `${(x * 100).toFixed(1)}%` literal — so a DE
 * realtor saw EN-grouped numbers (`12,345`) and an EN decimal point.
 *
 * These tests render the page under a non-English active locale and assert the
 * metrics are grouped/percent-formatted for that locale via `@/lib/format`
 * `formatNumber(value, locale)`. They fail against the pre-fix page, which
 * emitted runtime-default formatting regardless of locale.
 */

import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import RealtorAnalyticsPage from './page';

// Drive the page under the German locale so locale-correct grouping (`12.345`)
// and percent formatting (`12,3 %`) are observable and distinguishable from the
// English defaults (`12,345` / `12.3%`). key->key keeps label assertions stable.
vi.mock('next-intl', () => ({
  useTranslations: () => (key: string) => key,
  useLocale: () => 'de',
}));

// Chrome + auth guard are irrelevant to the formatting assertions.
vi.mock('@/components/ui', () => ({ Header: () => null, Footer: () => null }));
vi.mock('@/components/auth', () => ({
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// Error copy helper is unused on the success path but imported by the page.
vi.mock('@/lib/realtor-error', () => ({
  useRealtorErrorMessage: () => (_err: unknown, fallbackKey: string) => fallbackKey,
}));

const getMyRealtorAnalytics = vi.fn();
vi.mock('@/lib/realtor-api', () => ({
  getMyRealtorAnalytics: () => getMyRealtorAnalytics(),
}));

describe('realtor analytics page i18n (number formatting)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getMyRealtorAnalytics.mockResolvedValue({
      totalListings: 12,
      activeListings: 8,
      totalViews: 12345,
      totalInquiries: 1234,
      conversionRate: 0.123,
    });
  });

  it('groups metric numbers for the active locale, not the runtime default', async () => {
    render(<RealtorAnalyticsPage />);

    // de-DE groups thousands with a dot: 12345 -> "12.345", 1234 -> "1.234".
    expect(await screen.findByText('12.345')).toBeInTheDocument();
    expect(screen.getByText('1.234')).toBeInTheDocument();

    // The English grouping must never leak to a DE realtor.
    expect(screen.queryByText('12,345')).not.toBeInTheDocument();
  });

  it('renders the conversion rate as a locale-formatted percent, not a hardcoded "%"', async () => {
    render(<RealtorAnalyticsPage />);

    // de-DE percent of 0.123 -> "12,3 %" (comma decimal + NBSP before the sign);
    // tolerate any whitespace between the number and the percent sign.
    await waitFor(() => {
      expect(
        screen.getByText((text) => /12,3\s*%/.test(text)),
        'conversion rate should be locale-formatted (de: "12,3 %")'
      ).toBeInTheDocument();
    });

    // The pre-fix hardcoded English form must be gone.
    expect(screen.queryByText('12.3%')).not.toBeInTheDocument();
  });
});
