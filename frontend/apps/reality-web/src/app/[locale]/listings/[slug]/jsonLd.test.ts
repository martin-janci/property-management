/**
 * buildListingJsonLd Tests
 *
 * `buildListingJsonLd` consumes a guaranteed-shape `ListingDetail` (normalized
 * by `parseListingDetail` at the `getListing` boundary), so it no longer
 * defends against malformed / wrong-typed bodies — that crash-class coverage
 * lives in `listingSchema.test.ts`. These tests assert it emits correct
 * structured data and omits genuinely optional fields when absent.
 *
 * `isRenderableListing` (still defined here, consumed by the normalizer) keeps
 * its own required-field describe block below.
 */

import type { ListingDetail } from '@ppt/reality-api-client';
import { describe, expect, it } from 'vitest';
import { buildListingJsonLd, isRenderableListing } from './jsonLd';

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
  description: 'A lovely place to live with plenty of light.',
  features: {},
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

describe('buildListingJsonLd', () => {
  it('builds full JSON-LD from a valid listing', () => {
    const jsonLd = buildListingJsonLd(validListing) as Record<string, unknown>;
    expect(jsonLd).not.toBeNull();
    expect(jsonLd['@type']).toBe('RealEstateListing');
    expect(jsonLd.name).toBe('Beautiful Apartment');
    expect(jsonLd.description).toBe('A lovely place to live with plenty of light.');
    expect(jsonLd.image).toEqual(['https://cdn.example.com/p1.jpg']);
    expect(jsonLd.numberOfRooms).toBe(3);
    expect((jsonLd.address as Record<string, unknown>).addressLocality).toBe('Bratislava');
    expect((jsonLd.offers as Record<string, unknown>).price).toBe(150000);
    expect((jsonLd.floorSize as Record<string, unknown>).value).toBe(75);
  });

  it('omits the image field for an empty photos array', () => {
    const noPhotos: ListingDetail = { ...validListing, photos: [] };
    const jsonLd = buildListingJsonLd(noPhotos) as Record<string, unknown>;
    expect(jsonLd.name).toBe('Beautiful Apartment');
    expect('image' in jsonLd).toBe(false);
  });

  it('omits optional price/rooms/area blocks when those fields are absent', () => {
    const minimal = {
      ...validListing,
      price: undefined,
      rooms: undefined,
      area: undefined,
    } as unknown as ListingDetail;
    const jsonLd = buildListingJsonLd(minimal) as Record<string, unknown>;
    expect((jsonLd.address as Record<string, unknown>).addressLocality).toBe('Bratislava');
    expect('offers' in jsonLd).toBe(false);
    expect('numberOfRooms' in jsonLd).toBe(false);
    expect('floorSize' in jsonLd).toBe(false);
  });
});

describe('isRenderableListing', () => {
  it('accepts a valid listing', () => {
    expect(isRenderableListing(validListing)).toBe(true);
  });

  it('accepts a minimal body with title, slug and a well-formed address', () => {
    expect(
      isRenderableListing({
        title: 'Minimal',
        slug: 'minimal',
        address: { city: 'Bratislava', country: 'SK' },
      })
    ).toBe(true);
  });

  // These are exactly the partial 200 bodies getListing must reject so the
  // request falls back to ListingNotFound instead of crashing SSR (#2276).
  it('rejects null / undefined', () => {
    expect(isRenderableListing(null)).toBe(false);
    expect(isRenderableListing(undefined)).toBe(false);
  });

  it('rejects an empty object', () => {
    expect(isRenderableListing({})).toBe(false);
  });

  it('rejects a body missing address', () => {
    expect(isRenderableListing({ title: 't', slug: 's' })).toBe(false);
  });

  it('rejects a body whose address is missing city or country', () => {
    expect(isRenderableListing({ title: 't', slug: 's', address: { city: 'X' } })).toBe(false);
    expect(isRenderableListing({ title: 't', slug: 's', address: { country: 'SK' } })).toBe(false);
  });

  it('rejects a body missing title or slug', () => {
    const addr = { city: 'X', country: 'SK' };
    expect(isRenderableListing({ slug: 's', address: addr })).toBe(false);
    expect(isRenderableListing({ title: 't', address: addr })).toBe(false);
  });

  it('rejects non-object bodies', () => {
    expect(isRenderableListing('oops')).toBe(false);
    expect(isRenderableListing(42)).toBe(false);
    expect(isRenderableListing([])).toBe(false);
  });
});
