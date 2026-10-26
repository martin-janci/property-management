/**
 * Agency profile page price-formatting regression test.
 *
 * Reality Portal is a six-language portal (sk/cs/de/en/pl/hu). The listing
 * price on the public agency profile must render with the active locale's
 * number convention, not a hardcoded `sk-SK`. Before the fix the page called
 * `listing.price.toLocaleString('sk-SK')`, so a German visitor saw Slovak
 * space-grouping (`1 250 000`) instead of German dot-grouping (`1.250.000 €`).
 *
 * This is the numeric half of the i18n gap that PR #3155 left behind after it
 * extracted the chrome copy (issue #3162). It mirrors
 * `components/listings/sections/ListingHeader.test.tsx` (PR #3140): force the
 * active locale to `de` via next-intl's `useLocale` and assert the price uses
 * German grouping. It fails against the hardcoded-`sk-SK` implementation and
 * passes once the page threads the active locale through
 * `@/lib/format.formatPrice`.
 */

import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import AgencyProfilePage from './page';

// Force a non-Slovak active locale. The page reads it via `useLocale`;
// translations are stubbed to echo their key (+ interpolation passthrough).
vi.mock('next-intl', () => ({
  useLocale: () => 'de',
  useTranslations: () => (key: string) => key,
}));

vi.mock('next/navigation', () => ({
  useParams: () => ({ slug: 'acme-reality' }),
}));

// Keep the i18n <Link> and chrome components inert so the test exercises only
// the page's own rendering (no next-intl router / provider context needed).
vi.mock('@/i18n/routing', () => ({
  Link: ({ children, ...props }: { children: React.ReactNode }) => <a {...props}>{children}</a>,
}));
vi.mock('@/components/ui', () => ({
  Header: () => null,
  Footer: () => null,
}));

const agency = {
  id: 'ag1',
  slug: 'acme-reality',
  name: 'Acme Reality',
};

const listing = {
  id: 'l1',
  slug: 'byt-bratislava',
  title: 'Byt v centre',
  transactionType: 'sale',
  price: 1250000,
  currency: 'EUR',
};

vi.mock('@ppt/reality-api-client', () => ({
  useAgencyBySlug: () => ({ data: agency, isLoading: false, isError: false }),
  useRealtors: () => ({ data: [] }),
  useAgencyListings: () => ({ data: { listings: [listing] } }),
}));

describe('AgencyProfilePage price formatting', () => {
  it('formats the listing price using the active locale (de), not hardcoded sk-SK', () => {
    render(<AgencyProfilePage />);

    // de-DE groups thousands with a dot: 1.250.000. The old sk-SK output was
    // 1 250 000 (space grouping) — the space form must NOT appear.
    const price = screen.getByText(/1\.250\.000/);
    expect(price).toBeInTheDocument();
    expect(price.textContent).not.toMatch(/1[\s  ]250[\s  ]000/);
  });
});
