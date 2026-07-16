/**
 * parseListingDetail Tests
 *
 * Consolidated regression guard for the SSR-500 crash class (#2276 / #2281 /
 * #2341) where a malformed / partial 200 listing body — truthy but missing
 * required nested fields, or carrying *wrong-typed* collection fields — crashed
 * rendering while dereferencing `listing.address.city`,
 * `Object.entries(listing.features)`, or `listing.photos.map`.
 *
 * The shape-validation logic used to be hand-rolled across four render sites,
 * each with its own notion of "valid". `parseListingDetail` is now the single
 * normalizer at the `getListing` boundary, so this one suite covers the crash
 * class for every downstream consumer (metadata, JSON-LD, ListingDetailContent).
 */

import type { ListingDetail } from '@ppt/reality-api-client';
import { describe, expect, it } from 'vitest';
import { parseListingDetail } from './listingSchema';

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

describe('parseListingDetail', () => {
  it('returns a normalized listing for a valid body, preserving features/photos', () => {
    const parsed = parseListingDetail(validListing);
    expect(parsed).not.toBeNull();
    expect(parsed?.title).toBe('Beautiful Apartment');
    expect(parsed?.features).toEqual({ balcony: true, parking: false });
    expect(parsed?.photos).toHaveLength(1);
  });

  it('accepts a minimal renderable body (title, slug, well-formed address)', () => {
    const parsed = parseListingDetail({
      title: 'Minimal',
      slug: 'minimal',
      address: { city: 'Bratislava', country: 'SK' },
    });
    expect(parsed).not.toBeNull();
    // Missing collection fields are coerced to safe empties.
    expect(parsed?.features).toEqual({});
    expect(parsed?.photos).toEqual([]);
  });

  // These are exactly the partial 200 bodies getListing must reject so the
  // request falls back to ListingNotFound instead of crashing SSR (#2276).
  it('returns null for null / undefined', () => {
    expect(parseListingDetail(null)).toBeNull();
    expect(parseListingDetail(undefined)).toBeNull();
  });

  it('returns null for an empty object', () => {
    expect(parseListingDetail({})).toBeNull();
  });

  it('returns null when address is missing', () => {
    expect(parseListingDetail({ title: 't', slug: 's' })).toBeNull();
  });

  it('returns null when address is missing city or country', () => {
    expect(parseListingDetail({ title: 't', slug: 's', address: { city: 'X' } })).toBeNull();
    expect(parseListingDetail({ title: 't', slug: 's', address: { country: 'SK' } })).toBeNull();
  });

  it('returns null when title or slug is missing', () => {
    const address = { city: 'X', country: 'SK' };
    expect(parseListingDetail({ slug: 's', address })).toBeNull();
    expect(parseListingDetail({ title: 't', address })).toBeNull();
  });

  it('returns null for non-object bodies (string / number / array)', () => {
    expect(parseListingDetail('oops')).toBeNull();
    expect(parseListingDetail(42)).toBeNull();
    expect(parseListingDetail([])).toBeNull();
  });

  // #2341: a partial 200 can carry *wrong-typed* (not just missing) collection
  // fields. `features: "x"` fed to `Object.entries` yields per-character garbage
  // entries; a non-array `photos` (e.g. `{}`) makes `.length` / `.map` throw.
  // Nullish-coalescing (`?? {}` / `?? []`) does not catch a truthy wrong type —
  // the normalizer must coerce both to a safe empty shape.
  it('coerces a string features to {} (was #2341 Object.entries crash)', () => {
    const parsed = parseListingDetail({
      ...validListing,
      features: 'x' as unknown as ListingDetail['features'],
    });
    expect(parsed).not.toBeNull();
    expect(parsed?.features).toEqual({});
  });

  it('coerces an undefined features to {}', () => {
    const parsed = parseListingDetail({
      ...validListing,
      features: undefined as unknown as ListingDetail['features'],
    });
    expect(parsed?.features).toEqual({});
  });

  it('coerces a non-array photos ({}) to [] (was #2341 PhotoGallery crash)', () => {
    const parsed = parseListingDetail({
      ...validListing,
      photos: {} as unknown as ListingDetail['photos'],
    });
    expect(parsed).not.toBeNull();
    expect(parsed?.photos).toEqual([]);
  });

  it('coerces an undefined photos to []', () => {
    const parsed = parseListingDetail({
      ...validListing,
      photos: undefined as unknown as ListingDetail['photos'],
    });
    expect(parsed?.photos).toEqual([]);
  });
});
