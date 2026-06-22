/**
 * Listing detail JSON-LD helpers.
 *
 * Kept in a standalone module (no component / next-intl imports) so the
 * defensive structured-data logic can be unit-tested in isolation, mirroring
 * the sibling `metadata.ts`.
 */

import type { ListingDetail } from '@ppt/reality-api-client';

/**
 * Build the RealEstateListing JSON-LD object defensively.
 *
 * `getListing` returns the raw JSON body of any 200 response, so a malformed
 * or partial 200 (e.g. `{}`, or a body missing `address` / `photos`) is truthy
 * but does not match `ListingDetail`. Dereferencing nested fields off such a
 * body (`listing.photos.map`, `listing.address.street`) would throw during SSR
 * and crash the request. Treat the body as `unknown`, require the minimum set
 * of fields, and return `null` whenever a required field is missing or has the
 * wrong shape. Optional fields (`photos`, `price`, `rooms`, `area`, …) are
 * emitted only when present.
 */
export function buildListingJsonLd(listing: unknown): object | null {
  if (!listing || typeof listing !== 'object') {
    return null;
  }

  const l = listing as Partial<ListingDetail>;
  const address = l.address;

  if (
    typeof l.title !== 'string' ||
    typeof l.slug !== 'string' ||
    !address ||
    typeof address !== 'object' ||
    typeof address.city !== 'string' ||
    typeof address.country !== 'string'
  ) {
    return null;
  }

  const jsonLd: Record<string, unknown> = {
    '@context': 'https://schema.org',
    '@type': 'RealEstateListing',
    name: l.title,
    url: `${process.env.NEXT_PUBLIC_SITE_URL || ''}/listings/${l.slug}`,
    address: {
      '@type': 'PostalAddress',
      streetAddress: address.street,
      addressLocality: address.city,
      addressRegion: address.district,
      postalCode: address.postalCode,
      addressCountry: address.country,
    },
  };

  if (typeof l.description === 'string') {
    jsonLd.description = l.description;
  }

  if (Array.isArray(l.photos)) {
    jsonLd.image = l.photos
      .map((p) => (p && typeof p.url === 'string' ? p.url : undefined))
      .filter((url): url is string => typeof url === 'string');
  }

  if (typeof l.price === 'number') {
    jsonLd.offers = {
      '@type': 'Offer',
      price: l.price,
      priceCurrency: typeof l.currency === 'string' ? l.currency : undefined,
      availability: l.status === 'active' ? 'InStock' : 'OutOfStock',
    };
  }

  if (typeof l.rooms === 'number') {
    jsonLd.numberOfRooms = l.rooms;
  }

  if (typeof l.area === 'number') {
    jsonLd.floorSize = {
      '@type': 'QuantitativeValue',
      value: l.area,
      unitCode: 'MTK',
    };
  }

  return jsonLd;
}
