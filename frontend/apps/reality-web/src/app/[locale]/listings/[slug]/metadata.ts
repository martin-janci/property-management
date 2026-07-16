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
 * Build listing metadata.
 *
 * `getListing` normalizes the raw 200 body through `parseListingDetail`, so a
 * malformed / partial body has already become `null` here and the required
 * fields (`title`, `address.city`) are guaranteed present on a non-null
 * listing — the required-field / wrong-shape defense lives at that single
 * boundary. Fall back to safe default metadata only when the listing is absent;
 * still guard the genuinely optional `description` / `primaryPhoto`, which the
 * normalizer does not require.
 */
export function buildListingMetadata(listing: ListingDetail | null): Metadata {
  if (!listing) {
    return FALLBACK_METADATA;
  }

  const title = `${listing.title} - ${listing.address.city} | Reality Portal`;
  const description =
    typeof listing.description === 'string' ? listing.description.slice(0, 160) : undefined;
  const images = typeof listing.primaryPhoto?.url === 'string' ? [listing.primaryPhoto.url] : [];

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
