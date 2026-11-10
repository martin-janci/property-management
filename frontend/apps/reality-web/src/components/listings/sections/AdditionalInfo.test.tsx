/**
 * AdditionalInfo date-formatting regression test.
 *
 * Reality Portal is a multi-language portal (sk/cs/de/en/…). The listing
 * "available from" / "listed" dates must render with the active locale's date
 * convention, not a fixed/ambient one. Before the fix the section called
 * `new Date(...).toLocaleDateString()` with NO locale argument, so the output
 * followed the runtime's default locale (e.g. `12/31/2025`, US m/d/y) instead
 * of the visitor's active locale (`31.12.2025` for `de`).
 *
 * This is the date half of the i18n gap whose numeric half is covered by
 * `ListingHeader.test.tsx` and `agency/[slug]/page.test.tsx`. It forces the
 * active locale to `de` via next-intl's `useLocale` and asserts the dates use
 * German day-first dot grouping. It fails against the locale-less
 * `toLocaleDateString()` implementation and passes once the component threads
 * the active locale through `@/lib/format.formatDate`.
 */

import type { ListingDetail } from '@ppt/reality-api-client';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { AdditionalInfo } from './AdditionalInfo';

// Force a non-default active locale. The component reads it via `useLocale`;
// translations are stubbed to echo their key.
vi.mock('next-intl', () => ({
  useLocale: () => 'de',
  useTranslations: () => (key: string) => key,
}));

const listing = {
  id: 'l1',
  slug: 'byt-bratislava',
  availableFrom: '2025-12-31T00:00:00Z',
  createdAt: '2025-12-31T00:00:00Z',
  currency: 'EUR',
} as unknown as ListingDetail;

describe('AdditionalInfo date formatting', () => {
  it('formats dates using the active locale (de), not an ambient/default locale', () => {
    render(<AdditionalInfo listing={listing} />);

    // de-DE renders 31.12.2025 (day-first, dot-separated). The locale-less
    // default (en-US) renders 12/31/2025 — the slash/US form must NOT appear.
    const dates = screen.getAllByText(/31\.12\.2025/);
    expect(dates.length).toBeGreaterThanOrEqual(2);
    for (const el of dates) {
      expect(el.textContent).not.toMatch(/12\/31\/2025/);
    }
  });
});
