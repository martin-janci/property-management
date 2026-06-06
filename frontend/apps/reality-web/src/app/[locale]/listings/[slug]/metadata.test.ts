/**
 * buildListingMetadata Tests
 *
 * Regression guard for the SSR crash where a malformed/partial 200 body
 * (truthy but not a real ListingDetail) made generateMetadata throw while
 * reading nested fields (listing.address.city, listing.description.slice).
 * buildListingMetadata must degrade to safe fallback metadata instead.
 */

import type { ListingDetail } from '@ppt/reality-api-client';
import { describe, expect, it } from 'vitest';
import { buildListingMetadata } from './metadata';

const FALLBACK_TITLE = 'Listing Not Found - Reality Portal';

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
  address: { city: 'Bratislava', country: 'SK' },
  isFeatured: false,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  description: 'A lovely place to live with plenty of light.',
  features: {},
  photos: [],
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

describe('buildListingMetadata', () => {
  it('builds full metadata from a valid listing', () => {
    const meta = buildListingMetadata(validListing);
    expect(meta.title).toBe('Beautiful Apartment - Bratislava | Reality Portal');
    expect(meta.description).toBe('A lovely place to live with plenty of light.');
    expect(meta.openGraph?.images).toEqual(['https://cdn.example.com/p1.jpg']);
  });

  it('falls back when the body is null/undefined (not-found / network failure)', () => {
    expect(buildListingMetadata(null).title).toBe(FALLBACK_TITLE);
    expect(buildListingMetadata(undefined).title).toBe(FALLBACK_TITLE);
  });

  // The core regression: malformed but truthy 200 bodies must NOT throw.
  it('falls back on an empty object body without throwing', () => {
    expect(() => buildListingMetadata({})).not.toThrow();
    expect(buildListingMetadata({}).title).toBe(FALLBACK_TITLE);
  });

  it('falls back when address is missing (would have thrown on .city)', () => {
    const malformed = { title: 'Has title but no address' };
    expect(() => buildListingMetadata(malformed)).not.toThrow();
    expect(buildListingMetadata(malformed).title).toBe(FALLBACK_TITLE);
  });

  it('falls back when title is missing', () => {
    const malformed = { address: { city: 'Bratislava', country: 'SK' } };
    expect(buildListingMetadata(malformed).title).toBe(FALLBACK_TITLE);
  });

  it('falls back on non-object bodies (string / number / array)', () => {
    expect(buildListingMetadata('oops').title).toBe(FALLBACK_TITLE);
    expect(buildListingMetadata(42).title).toBe(FALLBACK_TITLE);
    expect(buildListingMetadata([]).title).toBe(FALLBACK_TITLE);
  });

  it('tolerates a missing/malformed description (would have thrown on .slice)', () => {
    const noDescription = { ...validListing, description: undefined as unknown as string };
    expect(() => buildListingMetadata(noDescription)).not.toThrow();
    const meta = buildListingMetadata(noDescription);
    expect(meta.title).toBe('Beautiful Apartment - Bratislava | Reality Portal');
    expect(meta.description).toBeUndefined();
  });

  it('omits og images when primaryPhoto is missing or malformed', () => {
    const noPhoto = { ...validListing, primaryPhoto: undefined };
    expect(buildListingMetadata(noPhoto).openGraph?.images).toEqual([]);
  });
});
