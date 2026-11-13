/**
 * URL query-param parsers for the public listings search page (`page.tsx`).
 *
 * The listings page hydrates its filters and pagination straight from
 * `searchParams`, which is attacker-/user-controlled (shared links, manual URL
 * edits, stale bookmarks). The previous inline `Number(...)` / `Number(x || 1)`
 * coercions let a malformed param (`?page=abc`, `?priceMin=foo`) collapse to
 * `NaN` and flow into `useListings(filters, currentPage)` — producing a
 * `page=NaN` request and NaN-valued filters. These helpers guard every parse at
 * the edge so a bad param degrades gracefully instead of poisoning the query.
 */

/**
 * Parse a numeric filter query-string value, returning `undefined` for missing,
 * empty, or non-finite (`NaN` / `Infinity`) inputs so a bad URL param is simply
 * omitted from the filter set rather than passed through as `NaN`.
 */
export function parseFiniteNumber(value: string | null): number | undefined {
  if (value === null || value.trim() === '') return undefined;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

/**
 * Parse the `page` query param into a positive integer, defaulting to `1` for
 * missing, malformed (`?page=abc`), fractional, zero/negative, or otherwise
 * out-of-range values.
 */
export function parsePageParam(value: string | null): number {
  const parsed = Number(value);
  return Number.isInteger(parsed) && parsed >= 1 ? parsed : 1;
}
