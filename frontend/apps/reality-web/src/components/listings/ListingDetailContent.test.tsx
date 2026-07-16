/**
 * ListingDetailContent Tests
 *
 * `ListingDetailContent` renders a guaranteed-shape `ListingDetail | null`:
 * `getListing` runs every 200 body through `parseListingDetail`, which validates
 * required fields and coerces `features` / `photos`. The malformed / wrong-typed
 * body crash-class coverage (#2276 / #2281 / #2341) now lives at that single
 * normalizer — see `listingSchema.test.ts`. These tests assert the happy path
 * and the null (not-found) fallback.
 */

import type { ListingDetail } from '@ppt/reality-api-client';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ListingDetailContent } from './ListingDetailContent';

// ContactForm (rendered in the sidebar) uses a TanStack Query mutation hook.
// Stub the API client so the component tree renders without a QueryClient.
vi.mock('@ppt/reality-api-client', () => ({
  useCreateInquiry: () => ({
    mutateAsync: vi.fn(),
    isPending: false,
    isError: false,
  }),
}));

// Header/Footer pull in the i18n routing + auth-context tree, which is
// irrelevant to the partial-body render behaviour under test.
vi.mock('@/components/ui', () => ({
  Header: () => null,
  Footer: () => null,
}));

const validListing: ListingDetail = {
  id: 'listing-1',
  slug: 'beautiful-apartment',
  title: 'Beautiful Apartment',
  propertyType: 'apartment',
  transactionType: 'sale',
  status: 'active',
  price: 150000,
  currency: 'EUR',
  area: 75,
  rooms: 3,
  address: {
    street: 'Main St 1',
    city: 'Bratislava',
    district: 'Old Town',
    postalCode: '81101',
    country: 'SK',
  },
  isFeatured: false,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  description: 'A lovely place to live.',
  features: { balcony: true, parking: false },
  photos: [
    {
      id: 'p1',
      url: 'https://cdn.example.com/p1.jpg',
      thumbnailUrl: 'https://cdn.example.com/p1-thumb.jpg',
      isPrimary: true,
      order: 0,
    },
  ],
  agent: { id: 'a1', name: 'Agent', email: 'a@example.com' },
  viewCount: 0,
  favoriteCount: 0,
  primaryPhoto: {
    id: 'p1',
    url: 'https://cdn.example.com/p1.jpg',
    thumbnailUrl: 'https://cdn.example.com/p1-thumb.jpg',
    isPrimary: true,
    order: 0,
  },
};

describe('ListingDetailContent', () => {
  it('renders a valid listing', () => {
    render(<ListingDetailContent listing={validListing} />);
    expect(screen.getByText('Beautiful Apartment')).toBeInTheDocument();
  });

  it('renders ListingNotFound when listing is null', () => {
    render(<ListingDetailContent listing={null} />);
    // next-intl is mocked to echo the key.
    expect(screen.getByText('notFound')).toBeInTheDocument();
  });

  it('renders a normalized listing whose features/photos are empty', () => {
    const empty: ListingDetail = { ...validListing, features: {}, photos: [] };
    expect(() => render(<ListingDetailContent listing={empty} />)).not.toThrow();
    expect(screen.getByText('Beautiful Apartment')).toBeInTheDocument();
  });
});
