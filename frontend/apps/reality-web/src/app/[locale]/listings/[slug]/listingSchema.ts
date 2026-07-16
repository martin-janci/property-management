/**
 * Listing detail shape normalizer.
 *
 * Single source of truth for "is this 200 body a renderable listing, and if so,
 * what is its guaranteed shape?". `getListing` returns the raw JSON body of any
 * 200 response, so a malformed or partial 200 (an upstream/proxy partial
 * response, or a listing carrying wrong-typed collection fields) is truthy but
 * does not match `ListingDetail`. Dereferencing such a body during SSR
 * (`Object.entries(listing.features)`, `listing.photos.map`) throws and crashes
 * the request with a 500 (#2276 / #2281 / #2341).
 *
 * Before this normalizer the "is this body well-formed?" logic was hand-rolled
 * across four render sites (`getListing`, `buildListingMetadata`,
 * `buildListingJsonLd`, `ListingDetailContent`), each with a slightly different
 * notion of "valid" — a latent-regression surface where any new nested deref
 * re-opened the same crash class. `parseListingDetail` consolidates it: it
 * validates the required fields once (via {@link isRenderableListing}) and
 * coerces the two wrong-typeable collection fields — `features` → object-or-`{}`,
 * `photos` → array-or-`[]` — so every consumer downstream of `getListing`
 * receives a guaranteed-shape `ListingDetail` and can drop its inline defenses.
 */

import type { ListingDetail } from '@ppt/reality-api-client';
import { isRenderableListing } from './jsonLd';

/**
 * Validate + normalize a raw 200 body into a renderable `ListingDetail`.
 *
 * Returns `null` when the body is missing the minimum renderable shape (title,
 * slug, and a well-formed `address` with `city` + `country`) — the caller then
 * falls back to `ListingNotFound` (404) instead of crashing SSR. When it
 * returns a listing, `features` is guaranteed to be a plain object and `photos`
 * a real array, so consumers can `Object.entries(listing.features)` and
 * `listing.photos.map(...)` without their own type-guards.
 */
export function parseListingDetail(body: unknown): ListingDetail | null {
  if (!isRenderableListing(body)) {
    return null;
  }

  // `features` / `photos` are typed as present on `ListingDetail`, but a partial
  // 200 can carry *wrong-typed* values (e.g. `features: "x"`, `photos: {}`), not
  // just null/undefined. `features: "x"` makes `Object.entries` emit
  // per-character garbage entries; a non-array `photos` makes `.length` / `.map`
  // throw. Coerce both to a safe empty shape when malformed (#2341).
  const features =
    body.features && typeof body.features === 'object' && !Array.isArray(body.features)
      ? body.features
      : ({} as ListingDetail['features']);
  const photos = Array.isArray(body.photos) ? body.photos : [];

  return { ...body, features, photos };
}
