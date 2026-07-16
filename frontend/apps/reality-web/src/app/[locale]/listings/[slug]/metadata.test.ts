/**
 * buildListingMetadata Tests
 *
 * `buildListingMetadata` consumes a guaranteed-shape `ListingDetail | null`
 * (normalized by `parseListingDetail` at the `getListing` boundary): a
 * malformed / partial body has already become `null`, so the malformed-body
 * crash-class coverage lives in `listingSchema.test.ts`. These tests assert it
 * falls back for a null listing and still guards the genuinely optional
 * `description` / `primaryPhoto` that the normalizer does not require.
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

  it('falls back when the listing is null (not-found / network failure)', () => {
    expect(buildListingMetadata(null).title).toBe(FALLBACK_TITLE);
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
