/**
 * Regression test for the listing-edit i18n gap
 * (code-review-reality-web-listing-edit-i18n-sk).
 *
 * Before the fix, this wizard hardcoded Slovak user-facing strings directly in
 * `page.tsx` and `_mock.ts` (e.g. "Upraviť inzerát", "Typ transakcie",
 * "Predaj"), so cs/de/en/pl/hu visitors always saw Slovak. These tests pin that
 * every rendered label now flows through `useTranslations('pages.listingEdit')`
 * instead of a literal — under the global next-intl test mock (src/test/setup.tsx)
 * `t(key)` returns the key, so a localized label renders as its key and no raw
 * Slovak string survives. They fail on `main`, where the Slovak literals exist.
 */

import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ListingEditPage from './page';

// Provide the dynamic `[id]` route param the wizard needs to load (the global
// setup mock only supplies `locale`).
vi.mock('next/navigation', () => ({
  useParams: () => ({ locale: 'en', id: '42' }),
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), back: vi.fn() }),
  usePathname: () => '/en/account/listings/42/edit',
  useSearchParams: () => new URLSearchParams(),
}));

// Header/Footer + ProtectedRoute pull in the auth context, irrelevant here.
vi.mock('@/components/ui', () => ({ Header: () => null, Footer: () => null }));
vi.mock('@/components/auth', () => ({
  ProtectedRoute: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// Resolve the listing fetch so the wizard renders step 1; keep the real
// RealtorApiError (the page uses `instanceof`), stub only the network calls.
vi.mock('@/lib/realtor-api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/realtor-api')>();
  return {
    ...actual,
    getMyListing: vi.fn().mockResolvedValue({
      id: '42',
      status: 'active',
      transactionType: 'sale',
      propertyType: 'apartment',
      street: 'Pribinova 25',
      city: 'Bratislava',
      price: 245000,
      currency: 'EUR',
    }),
    updateListing: vi.fn(),
  };
});

describe('listing-edit page i18n', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the heading through a translation key, not a Slovak literal', async () => {
    render(<ListingEditPage />);
    // h1 is `t('title')` → under the mock renders the key "title".
    expect(await screen.findByRole('heading', { name: 'title' })).toBeInTheDocument();
  });

  it('does not render any hardcoded Slovak user-facing strings', async () => {
    render(<ListingEditPage />);
    await screen.findByRole('heading', { name: 'title' });
    // These literals were hardcoded in page.tsx / _mock.ts on `main`.
    expect(screen.queryByText('Upraviť inzerát')).toBeNull();
    expect(screen.queryByText('Typ transakcie')).toBeNull();
    expect(screen.queryByText('Typ nehnuteľnosti')).toBeNull();
    expect(screen.queryByText('Predaj')).toBeNull();
    expect(screen.queryByText('Prenájom')).toBeNull();
    expect(screen.queryByText('Byt')).toBeNull();
  });

  it('drives step labels and option labels through the pages.listingEdit namespace', async () => {
    render(<ListingEditPage />);
    await screen.findByRole('heading', { name: 'title' });
    // Field labels + transaction option labels are translation keys now.
    expect(screen.getByText('fields.transactionType')).toBeInTheDocument();
    expect(screen.getByText('fields.propertyType')).toBeInTheDocument();
    expect(screen.getByText('transactionOptions.sale')).toBeInTheDocument();
    expect(screen.getByText('propertyOptions.apartment')).toBeInTheDocument();
    // The aside step list reads step titles from the locale-agnostic _mock keys.
    expect(screen.getByText('steps.type.title')).toBeInTheDocument();
  });
});
