/**
 * ListingHeader price-formatting regression test.
 *
 * Reality Portal is a four-language portal (sk/cs/de/en). The listing price
 * must render with the active locale's number convention, not a hardcoded
 * `en-US`. Before the fix `formatPrice` always used `Intl.NumberFormat('en-US')`,
 * so a German visitor saw `€1,250,000` instead of `1.250.000 €`.
 *
 * This test forces the active locale to `de` via next-intl's `useLocale` and
 * asserts the price uses German grouping (dot separators). It fails against the
 * hardcoded-`en-US` implementation and passes once the component threads the
 * active locale through `@/lib/format.formatPrice`.
 */

import type { ListingDetail } from '@ppt/reality-api-client';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ListingHeader } from './ListingHeader';

// Force a non-English active locale. The component reads it via `useLocale`;
// translations are stubbed to echo their key.
vi.mock('next-intl', () => ({
  useLocale: () => 'de',
  useTranslations: () => (key: string) => key,
}));

const listing = {
  id: 'l1',
  slug: 'byt-bratislava',
  title: 'Byt v centre',
  transactionType: 'sale',
  propertyType: 'apartment',
  price: 1250000,
  currency: 'EUR',
  address: { city: 'Bratislava' },
} as unknown as ListingDetail;

describe('ListingHeader price formatting', () => {
  it('formats the price using the active locale (de), not a hardcoded en-US', () => {
    render(<ListingHeader listing={listing} />);

    // de-DE groups thousands with a dot: 1.250.000. The old en-US output was
    // 1,250,000 — the comma form must NOT appear.
    const price = screen.getByText(/1\.250\.000/);
    expect(price).toBeInTheDocument();
    expect(price.textContent).not.toMatch(/1,250,000/);
  });
});
