/**
 * Shared-comparison listing summary shape normalizer.
 *
 * The shared-comparison loader (`ComparisonUrlHandler`) fetches each shared id
 * from reality-server's canonical `/api/v1/listings/{id}` endpoint and used to
 * hand the raw 200 body straight to the comparison via an unchecked
 * `response.json() as Promise<ListingSummary>` cast — trusting the wire shape
 * blindly. A malformed or partial 200 (an upstream/proxy partial body, an HTML
 * error page served with a 200, or a `null`/primitive/array JSON body) was then
 * added to the comparison typed as `ListingSummary`, so a downstream deref
 * (`listing.address?.city`, `listing.primaryPhoto.thumbnailUrl`,
 * `Number(listing.price)`) rendered garbage or threw inside `ComparisonView`.
 *
 * `parseListingSummary` rejects the bad shape at the API boundary instead,
 * mirroring the listing detail path's `parseListingDetail` /
 * `isRenderableListing` guards (and the realtor-api `parseJsonBody` guard): a
 * body that is not a well-formed summary returns `null` so the loader drops
 * that single id and keeps the rest of the comparison intact, rather than
 * silently trusting it (#2276 / #2281 / #2341 class).
 */

import type { ListingSummary } from '@ppt/reality-api-client';

/**
 * Validate a raw 200 body into a renderable `ListingSummary`, or `null`.
 *
 * Requires the minimum identity + addressing shape the comparison relies on:
 * `id`, `title`, `slug` strings and a well-formed `address` object with string
 * `city` and `country`. The optional summary fields (`price`, `area`, `rooms`,
 * `primaryPhoto`, …) are left to the component's existing per-field defenses.
 */
export function parseListingSummary(body: unknown): ListingSummary | null {
  if (!body || typeof body !== 'object' || Array.isArray(body)) {
    return null;
  }

  const l = body as Partial<ListingSummary>;
  const address = l.address;

  if (
    typeof l.id !== 'string' ||
    typeof l.title !== 'string' ||
    typeof l.slug !== 'string' ||
    !address ||
    typeof address !== 'object' ||
    typeof address.city !== 'string' ||
    typeof address.country !== 'string'
  ) {
    return null;
  }

  return body as ListingSummary;
}
