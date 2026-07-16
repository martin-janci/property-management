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
 * Build the RealEstateListing JSON-LD object.
 *
 * Consumes a guaranteed-shape `ListingDetail` — `getListing` runs the raw 200
 * body through `parseListingDetail`, which validates the required fields (via
 * {@link isRenderableListing}) and coerces `photos` to an array — so the
 * required-field / non-array-photos defenses live at that single boundary, not
 * here. This helper still emits the genuinely optional fields (`price`, `rooms`,
 * `area`, `description`) only when present.
 */
export function buildListingJsonLd(listing: ListingDetail): object {
  const { address } = listing;

  const jsonLd: Record<string, unknown> = {
    '@context': 'https://schema.org',
    '@type': 'RealEstateListing',
    name: listing.title,
    url: `${process.env.NEXT_PUBLIC_SITE_URL || ''}/listings/${listing.slug}`,
    address: {
      '@type': 'PostalAddress',
      streetAddress: address.street,
      addressLocality: address.city,
      addressRegion: address.district,
      postalCode: address.postalCode,
      addressCountry: address.country,
    },
  };

  if (typeof listing.description === 'string') {
    jsonLd.description = listing.description;
  }

  // `photos` is a guaranteed array (normalized upstream); still filter out any
  // element whose `url` is not a string.
  const imageUrls = listing.photos
    .map((p) => (p && typeof p.url === 'string' ? p.url : undefined))
    .filter((url): url is string => typeof url === 'string');
  if (imageUrls.length > 0) {
    jsonLd.image = imageUrls;
  }

  if (typeof listing.price === 'number') {
    jsonLd.offers = {
      '@type': 'Offer',
      price: listing.price,
      priceCurrency: typeof listing.currency === 'string' ? listing.currency : undefined,
      availability: listing.status === 'active' ? 'InStock' : 'OutOfStock',
    };
  }

  if (typeof listing.rooms === 'number') {
    jsonLd.numberOfRooms = listing.rooms;
  }

  if (typeof listing.area === 'number') {
    jsonLd.floorSize = {
      '@type': 'QuantitativeValue',
      value: listing.area,
      unitCode: 'MTK',
    };
  }

  return jsonLd;
}
