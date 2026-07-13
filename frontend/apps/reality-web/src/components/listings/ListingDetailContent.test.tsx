/**
 * ListingDetailContent Tests
 *
 * Regression guard for the SSR 500 where a partial/malformed 200 listing body
 * (truthy but missing nested fields) crashed rendering while dereferencing
 * `listing.address.city` or `Object.entries(listing.features)`. The component
 * must render such bodies without throwing (issue #2276).
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

  // Core regression: partial 200 bodies must NOT throw during (SSR) render.
  it('does not throw when features is missing (Object.entries crash)', () => {
    const partial = {
      ...validListing,
      features: undefined as unknown as ListingDetail['features'],
    };
    expect(() => render(<ListingDetailContent listing={partial} />)).not.toThrow();
    expect(screen.getByText('Beautiful Apartment')).toBeInTheDocument();
  });

  it('does not throw when photos is missing (PhotoGallery crash)', () => {
    const partial = {
      ...validListing,
      photos: undefined as unknown as ListingDetail['photos'],
    };
    expect(() => render(<ListingDetailContent listing={partial} />)).not.toThrow();
    expect(screen.getByText('Beautiful Apartment')).toBeInTheDocument();
  });

  it('does not throw when address is missing (address.city crash)', () => {
    const partial = {
      ...validListing,
      address: undefined as unknown as ListingDetail['address'],
    };
    expect(() => render(<ListingDetailContent listing={partial} />)).not.toThrow();
    expect(screen.getByText('Beautiful Apartment')).toBeInTheDocument();
  });

  it('does not throw on a near-empty body missing all nested fields', () => {
    const partial = {
      id: 'x',
      slug: 'x',
      title: 'Bare Listing',
    } as unknown as ListingDetail;
    expect(() => render(<ListingDetailContent listing={partial} />)).not.toThrow();
    expect(screen.getByText('Bare Listing')).toBeInTheDocument();
  });
});
