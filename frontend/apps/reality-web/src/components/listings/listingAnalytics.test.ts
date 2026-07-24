/**
 * Regression test for the Reality Portal listing-view instrumentation
 * (foundational for realtor conversion metrics). Locks in that every listing
 * view emits a canonical `listing.viewed` event carrying view-source,
 * filter-state, and anonymous session context — and that view-source /
 * filter-state are derived correctly from the referrer + current URL.
 */
/// <reference types="vitest/globals" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { __clearAnalyticsSinks, type AnalyticsEvent, registerAnalyticsSink } from '@/lib/analytics';
import { __resetEphemeralSessionId } from '@/lib/analytics-session';
import { deriveListingViewContext, LISTING_EVENTS, trackListingViewed } from './listingAnalytics';

const ORIGIN = 'https://rlt.sk';

let captured: AnalyticsEvent[];

beforeEach(() => {
  captured = [];
  registerAnalyticsSink((e) => captured.push(e));
  try {
    globalThis.sessionStorage?.clear();
  } catch {
    /* no sessionStorage in this env */
  }
  __resetEphemeralSessionId();
});

afterEach(() => {
  __clearAnalyticsSinks();
});

describe('deriveListingViewContext — view-source', () => {
  it('classifies the results page as `search`', () => {
    const ctx = deriveListingViewContext(null, `${ORIGIN}/sk/listings?city=Bratislava`, ORIGIN);
    expect(ctx.viewSource).toBe('search');
  });

  it('classifies a related listing referrer as `listing`', () => {
    const ctx = deriveListingViewContext(null, `${ORIGIN}/sk/listings/other-flat`, ORIGIN);
    expect(ctx.viewSource).toBe('listing');
  });

  it('classifies the home page, favorites and realtor referrers', () => {
    expect(deriveListingViewContext(null, `${ORIGIN}/sk`, ORIGIN).viewSource).toBe('home');
    expect(deriveListingViewContext(null, `${ORIGIN}/cs/favorites`, ORIGIN).viewSource).toBe(
      'favorites'
    );
    expect(deriveListingViewContext(null, `${ORIGIN}/en/realtor/xyz`, ORIGIN).viewSource).toBe(
      'realtor'
    );
  });

  it('treats a foreign origin as `external` and no referrer as `direct`', () => {
    expect(
      deriveListingViewContext(null, 'https://google.com/search?q=flat', ORIGIN).viewSource
    ).toBe('external');
    expect(deriveListingViewContext(null, '', ORIGIN).viewSource).toBe('direct');
  });

  it('lets an explicit ?source= on the current URL win', () => {
    const params = new URLSearchParams('source=newsletter');
    const ctx = deriveListingViewContext(params, `${ORIGIN}/sk/listings`, ORIGIN);
    expect(ctx.viewSource).toBe('newsletter');
  });
});

describe('deriveListingViewContext — filter-state', () => {
  it('extracts snake_case filters from the referrer query', () => {
    const referrer = `${ORIGIN}/sk/listings?city=Bratislava&transactionType=rent&priceMax=900&sortBy=price`;
    const ctx = deriveListingViewContext(null, referrer, ORIGIN);
    expect(ctx.filterState).toEqual({
      city: 'Bratislava',
      transaction_type: 'rent',
      price_max: '900',
      sort_by: 'price',
    });
  });

  it('ignores non-filter params and empty values', () => {
    const referrer = `${ORIGIN}/sk/listings?page=3&source=x&city=`;
    const ctx = deriveListingViewContext(null, referrer, ORIGIN);
    expect(ctx.filterState).toEqual({});
  });

  it('lets current-URL filters override referrer filters', () => {
    const params = new URLSearchParams('city=Kosice');
    const referrer = `${ORIGIN}/sk/listings?city=Bratislava&roomsMin=2`;
    const ctx = deriveListingViewContext(params, referrer, ORIGIN);
    expect(ctx.filterState).toEqual({ city: 'Kosice', rooms_min: '2' });
  });
});

describe('trackListingViewed', () => {
  it('emits listing.viewed with the full payload', () => {
    trackListingViewed({
      listingId: 'listing-1',
      slug: 'sunny-flat',
      transactionType: 'rent',
      searchParams: null,
      referrer: `${ORIGIN}/sk/listings?city=Bratislava`,
      origin: ORIGIN,
    });

    expect(captured).toHaveLength(1);
    const event = captured[0];
    expect(event.event).toBe(LISTING_EVENTS.listingViewed);
    expect(event.timestamp).toEqual(expect.any(String));
    expect(event.properties).toMatchObject({
      listing_id: 'listing-1',
      slug: 'sunny-flat',
      transaction_type: 'rent',
      view_source: 'search',
      filter_state: { city: 'Bratislava' },
    });
    expect(event.properties.session_id).toEqual(expect.any(String));
    expect((event.properties.session_id as string).length).toBeGreaterThan(0);
    expect(event.properties.referrer).toEqual(expect.any(String));
  });

  it('reuses one session id across views in the same session', () => {
    trackListingViewed({ listingId: 'a', slug: 'a', origin: ORIGIN });
    trackListingViewed({ listingId: 'b', slug: 'b', origin: ORIGIN });
    expect(captured).toHaveLength(2);
    expect(captured[0].properties.session_id).toBe(captured[1].properties.session_id);
  });

  it('falls back to view_source=direct with no referrer', () => {
    trackListingViewed({ listingId: 'c', slug: 'c', origin: ORIGIN });
    expect(captured[0].properties.view_source).toBe('direct');
  });
});
