/**
 * Shared-comparison `?ids=` query-parameter normalizer.
 *
 * Next.js App Router types every `searchParams` value as
 * `string | string[] | undefined`: a single query parameter arrives as a
 * `string`, a repeated one (`?ids=a&ids=b`) as a `string[]`, and an absent one
 * as `undefined`. The compare page used to assert the shape as
 * `{ ids?: string }` — an unvalidated cast — and call `params.ids?.split(',')`
 * straight on it. A crafted or duplicated `ids` param therefore delivered an
 * array, `.split` does not exist on `string[]`, and the server component threw
 * `TypeError: params.ids.split is not a function`, crashing the whole
 * `/compare` route with a 500.
 *
 * `parseSharedIdsParam` narrows the raw value at the boundary instead of
 * trusting the cast: an array is folded into the same comma-joined form a
 * single value uses, the result is split on the `,` delimiter, and empty
 * segments are dropped. An `undefined`/empty value yields `[]`.
 */
export function parseSharedIdsParam(raw: string | string[] | undefined): string[] {
  if (raw === undefined) {
    return [];
  }
  const joined = Array.isArray(raw) ? raw.join(',') : raw;
  return joined.split(',').filter(Boolean);
}
