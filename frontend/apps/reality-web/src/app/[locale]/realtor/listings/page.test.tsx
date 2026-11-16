/**
 * Realtor "My listings" page — locale-aware price formatting regression (UC-51.3).
 *
 * The page's inline `formatPrice` helper hardcoded `Intl.NumberFormat('en', …)`,
 * so a listing's price was always rendered English-style (`€1,234,567.00`)
 * regardless of the visitor's active locale — on a six-language portal a Slovak
 * realtor saw English number grouping and a leading currency symbol instead of
 * the regional `1 234 567 €`. The fix routes price formatting through the shared
 * locale-aware `@/lib/format` helper + `useLocale()`, matching the convention
 * `ListingCard` already uses.
 *
 * This test renders the page under the `sk` locale and pins the Slovak-formatted
 * price; it fails against the old hardcoded-`'en'` implementation.
 */
/// <reference types="vitest/globals" />

import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { formatPrice } from '@/lib/format';
import skMessages from '../../../../../messages/sk.json';

// Override the global next-intl mock (src/test/setup.tsx pins `en`) so the page
// renders under `sk` and resolves real copy from sk.json — this pins both the
// locale-aware price output and that every key the page uses exists in sk.
vi.mock('next-intl', () => {
  const makeT = (namespace: string) => {
    const node = namespace
      .split('.')
      .reduce<unknown>((acc, part) => (acc as Record<string, unknown>)?.[part], skMessages);
    const t = (key: string, values?: Record<string, unknown>) => {
      let str = (node as Record<string, string> | undefined)?.[key] ?? key;
      if (values) {
        for (const [k, v] of Object.entries(values)) {
          str = str.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v));
        }
      }
      return str;
    };
    t.has = (key: string) => (node as Record<string, unknown> | undefined)?.[key] !== undefined;
    return t;
  };
  return {
    useLocale: () => 'sk',
    useTranslations: (namespace: string) => makeT(namespace),
  };
});

const useMyListings = vi.fn();
vi.mock('@ppt/reality-api-client', () => ({
  useMyListings: () => useMyListings(),
}));

vi.mock('@/components/auth', () => ({
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@/components/ui', () => ({
  Header: () => null,
  Footer: () => null,
}));

import MyListingsPage from './page';

function makeListing(overrides: Record<string, unknown> = {}) {
  return {
    id: 'listing-1',
    title: 'Pekný byt v centre',
    slug: 'pekny-byt-v-centre',
    propertyType: 'apartment',
    transactionType: 'sale',
    price: 1234567,
    currency: 'EUR',
    status: 'active',
    realtorId: 'realtor-1',
    realtorName: 'Jozef Testovací',
    views: 42,
    inquiries: 3,
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-02-01T00:00:00Z',
    ...overrides,
  };
}

describe('MyListingsPage — locale-aware price formatting', () => {
  it('formats a listing price using the active locale, not hardcoded English', () => {
    useMyListings.mockReturnValue({
      data: { listings: [makeListing()], total: 1 },
      isLoading: false,
      error: null,
    });

    render(<MyListingsPage />);

    // Slovak formatting: space grouping + trailing currency symbol. Normalize
    // the Intl narrow-no-break spaces (U+202F) to plain spaces so the string
    // matches testing-library's whitespace-normalized DOM text.
    const expected = formatPrice(1234567, 'sk', { currency: 'EUR' }).replace(/\s/g, ' ');
    expect(screen.getByText(expected)).toBeInTheDocument();

    // The old hardcoded-`'en'` output must not appear.
    expect(screen.queryByText('€1,234,567.00')).toBeNull();
  });
});
