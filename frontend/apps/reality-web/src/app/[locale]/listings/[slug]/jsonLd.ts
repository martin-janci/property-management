/**
 * Listing detail JSON-LD helpers.
 *
 * Kept in a standalone module (no component / next-intl imports) so the
 * defensive structured-data logic can be unit-tested in isolation, mirroring
 * the sibling `metadata.ts`.
 */

import type { ListingDetail } from '@ppt/reality-api-client';

/**
 * Type-guard for the minimum shape the listing detail page can render.
 *
 * `getListing` returns the raw JSON body of any 200 response, so a malformed
 * or partial 200 (e.g. `{}`, or a body missing `address`) is truthy but does
 * not match `ListingDetail`. Dereferencing nested fields off such a body
 * (`listing.address.city`, `Object.entries(listing.features)`) throws during
 * SSR and crashes the request with a 500. This predicate requires the minimum
 * set of fields — title, slug, and a well-formed `address` with `city` and
 * `country` — and is the single source of truth for "is this body renderable".
 * It is consumed both by `buildListingJsonLd` (below) and by `getListing`, so
 * a body that fails validation falls back to `ListingNotFound` instead of
 * crashing SSR.
 */
export function isRenderableListing(listing: unknown): listing is ListingDetail {
  if (!listing || typeof listing !== 'object') {
    return false;
  }

  const l = listing as Partial<ListingDetail>;
  const address = l.address;

  return (
    typeof l.title === 'string' &&
    typeof l.slug === 'string' &&
    !!address &&
    typeof address === 'object' &&
    typeof address.city === 'string' &&
    typeof address.country === 'string'
  );
}

/**
 * Build the RealEstateListing JSON-LD object defensively.
 *
 * Reuses {@link isRenderableListing} for the required-field check, then emits
 * optional fields (`photos`, `price`, `rooms`, `area`, …) only when present.
 * Returns `null` whenever a required field is missing or has the wrong shape.
 */
export function buildListingJsonLd(listing: unknown): object | null {
  if (!isRenderableListing(listing)) {
    return null;
  }

  const l = listing as Partial<ListingDetail>;
  const address = l.address;

  if (!address) {
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
