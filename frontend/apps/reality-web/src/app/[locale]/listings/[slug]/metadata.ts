/**
 * Listing detail metadata helpers.
 *
 * Kept in a standalone module (no component / next-intl imports) so the
 * defensive metadata logic can be unit-tested in isolation.
 */

import type { ListingDetail } from '@ppt/reality-api-client';
import type { Metadata } from 'next';

/** Metadata used when the listing can't be resolved or is malformed. */
export const FALLBACK_METADATA: Metadata = {
  title: 'Listing Not Found - Reality Portal',
};

/**
 * Build listing metadata defensively.
 *
 * `getListing` returns the raw JSON body of any 200 response, so a malformed
 * or partial 200 (e.g. `{}`, or a body missing `title` / `address` /
 * `description`) is truthy but does not match `ListingDetail`. Reading nested
 * fields off such a body (`listing.address.city`, `listing.description.slice`)
 * would throw during SSR and crash the request. Treat the body as `unknown`
 * and fall back to safe default metadata whenever a required field is missing
 * or the wrong shape.
 */
export function buildListingMetadata(listing: unknown): Metadata {
  if (!listing || typeof listing !== 'object') {
    return FALLBACK_METADATA;
  }

  const l = listing as Partial<ListingDetail>;
  const city = l.address?.city;
  if (typeof l.title !== 'string' || typeof city !== 'string') {
    return FALLBACK_METADATA;
  }

  const title = `${l.title} - ${city} | Reality Portal`;
  const description = typeof l.description === 'string' ? l.description.slice(0, 160) : undefined;
  const images = typeof l.primaryPhoto?.url === 'string' ? [l.primaryPhoto.url] : [];

  return {
    title,
    description,
    openGraph: {
      title,
      description,
      type: 'website',
      images,
    },
  };
}
