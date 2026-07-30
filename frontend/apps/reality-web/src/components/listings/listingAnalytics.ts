/**
 * Listing-view instrumentation for the Reality Portal (foundational for the
 * realtor conversion funnel).
 *
 * Every public listing-detail view emits one canonical `listing.viewed` event
 * carrying the three dimensions the data team needs to attribute conversions:
 *
 *   - **view-source**   — where the visit came from (search results, a related
 *                         listing, the home page, favorites, an external site,
 *                         or a direct hit), so views can be bucketed by channel.
 *   - **filter-state**  — the search filters that were active when the visitor
 *                         clicked through, so we can correlate filters → views.
 *   - **session context** — an anonymous per-tab session id (plus referrer) so a
 *                         visitor's views stitch into one browsing session.
 *
 * `deriveListingViewContext` is a pure function (no DOM access) so the
 * source/filter derivation is unit-testable; the React layer only supplies the
 * current `URLSearchParams`, `document.referrer`, and origin.
 */

import { trackEvent } from '@/lib/analytics';
import { getSessionContext } from '@/lib/analytics-session';

/** Canonical listing event names — keep in sync with the data-team schema. */
export const LISTING_EVENTS = {
  listingViewed: 'listing.viewed',
} as const;

export type ListingEventName = (typeof LISTING_EVENTS)[keyof typeof LISTING_EVENTS];

/**
 * Search filter query keys → snake_case property names. These mirror the
 * params written by the listings results page (`getFiltersFromUrl` /
 * `updateFilters`). `page`/`source` are intentionally excluded — they are
 * pagination/attribution, not filters.
 */
export const FILTER_PARAM_KEYS: Record<string, string> = {
  q: 'q',
  transactionType: 'transaction_type',
  propertyType: 'property_type',
  priceMin: 'price_min',
  priceMax: 'price_max',
  areaMin: 'area_min',
  areaMax: 'area_max',
  roomsMin: 'rooms_min',
  city: 'city',
  district: 'district',
  sortBy: 'sort_by',
  sortOrder: 'sort_order',
};

/** Minimal read interface satisfied by the DOM `URLSearchParams`. */
export interface ReadableParams {
  get(key: string): string | null;
}

export type ViewSource =
  | 'search'
  | 'listing'
  | 'home'
  | 'favorites'
  | 'realtor'
  | 'internal'
  | 'external'
  | 'direct'
  | 'unknown';

/**
 * The bounded set of canonical view-source buckets. An explicit `?source=`
 * query param is untrusted visitor input, so it is validated against this set
 * before being emitted — otherwise arbitrary URL values (e.g. per-campaign
 * marketing slugs or crafted junk) would inflate the `view_source` analytics
 * dimension's cardinality without bound. Keep in sync with the `ViewSource`
 * union above and the data-team schema.
 */
export const VIEW_SOURCES: ReadonlySet<ViewSource> = new Set<ViewSource>([
  'search',
  'listing',
  'home',
  'favorites',
  'realtor',
  'internal',
  'external',
  'direct',
  'unknown',
]);

/** Type guard: is an arbitrary string one of the known `ViewSource` buckets? */
export function isViewSource(value: string | null | undefined): value is ViewSource {
  return value != null && VIEW_SOURCES.has(value as ViewSource);
}

export interface ListingViewContext {
  viewSource: ViewSource;
  filterState: Record<string, string>;
}

/** Strip a locale prefix (`/sk`, `/cs`, …) so path matching is locale-agnostic. */
function normalizePath(pathname: string): string {
  const stripped = pathname.replace(/^\/[a-z]{2}(?=\/|$)/, '');
  return stripped === '' ? '/' : stripped;
}

/** Classify an internal referrer path into a coarse view-source bucket. */
function sourceFromInternalPath(pathname: string): ViewSource {
  const path = normalizePath(pathname);
  if (path === '/') return 'home';
  if (path.startsWith('/favorites')) return 'favorites';
  if (path.startsWith('/realtor')) return 'realtor';
  if (path.startsWith('/listings/')) return 'listing'; // detail → related detail
  if (path === '/listings') return 'search';
  return 'internal';
}

/** Collect the active filter params from a params source into snake_case keys. */
function collectFilters(params: ReadableParams | null): Record<string, string> {
  const out: Record<string, string> = {};
  if (!params) return out;
  for (const [queryKey, propKey] of Object.entries(FILTER_PARAM_KEYS)) {
    const value = params.get(queryKey);
    if (value != null && value !== '') out[propKey] = value;
  }
  return out;
}

/**
 * Derive `{ viewSource, filterState }` from the current page's search params,
 * the referrer URL, and the current origin. Pure — no DOM/globals touched.
 *
 * Precedence for view-source: a *recognised* explicit `?source=` on the current
 * URL wins (curated/marketing links), otherwise it is inferred from the
 * referrer. An explicit `?source=` that is not one of the known {@link ViewSource}
 * buckets is untrusted, unbounded-cardinality input and is mapped to `unknown`
 * rather than emitted verbatim. Filter-state is merged from the referrer's query
 * (the search that led here) and any filters forwarded onto the current URL,
 * with the current URL winning.
 */
export function deriveListingViewContext(
  searchParams: ReadableParams | null,
  referrer: string,
  origin: string
): ListingViewContext {
  let referrerParams: URLSearchParams | null = null;
  let referrerSource: ViewSource = 'direct';

  if (referrer) {
    try {
      const url = new URL(referrer);
      const sameOrigin = !!origin && url.origin === origin;
      if (sameOrigin) {
        referrerParams = url.searchParams;
        referrerSource = sourceFromInternalPath(url.pathname);
      } else {
        referrerSource = 'external';
      }
    } catch {
      referrerSource = 'external';
    }
  }

  const explicit = searchParams?.get('source');
  // An explicit `?source=` is untrusted visitor input: only honour it when it
  // names a known bucket, map any other non-empty value to `unknown`, and fall
  // back to the referrer-derived source when it is absent/empty. This keeps the
  // emitted `view_source` dimension's cardinality bounded.
  let viewSource: ViewSource;
  if (explicit) {
    viewSource = isViewSource(explicit) ? explicit : 'unknown';
  } else {
    viewSource = referrerSource;
  }

  // Referrer filters first, current-URL filters override (forward-compatible
  // with links that explicitly carry the originating filters).
  const filterState = { ...collectFilters(referrerParams), ...collectFilters(searchParams) };

  return { viewSource, filterState };
}

export interface TrackListingViewedInput {
  listingId: string;
  slug: string;
  transactionType?: string;
  /** Current page search params (`useSearchParams()`), if any. */
  searchParams?: ReadableParams | null;
  /** `document.referrer` — passed in so the emit path stays testable. */
  referrer?: string;
  /** `window.location.origin` — used to detect internal vs external referrers. */
  origin?: string;
}

/**
 * Emit the canonical `listing.viewed` event with view-source, filter-state, and
 * anonymous session context. Fire once per listing view (e.g. from a mount
 * effect keyed on the listing id). Never throws.
 */
export function trackListingViewed(input: TrackListingViewedInput): void {
  const { viewSource, filterState } = deriveListingViewContext(
    input.searchParams ?? null,
    input.referrer ?? '',
    input.origin ?? ''
  );
  const session = getSessionContext();

  trackEvent(LISTING_EVENTS.listingViewed, {
    listing_id: input.listingId,
    slug: input.slug,
    transaction_type: input.transactionType,
    view_source: viewSource,
    filter_state: filterState,
    session_id: session.sessionId,
    referrer: session.referrer,
  });
}
