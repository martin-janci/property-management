/**
 * Regression test for #3039 — the `/sell` wizard "Publish" 405.
 *
 * `createListing()` used to POST to `/api/v1/listings`, but reality-server
 * mounts only GET handlers at that path (`routes/listings.rs`) — so axum
 * answered the POST with 405 Method Not Allowed and listing creation never
 * succeeded from reality-web. The real create route is
 * `POST /api/v1/my/listings` (`portal_listings::create_listing`).
 *
 * `page.publish.test.tsx` mocks `createListing`, so it could never have caught
 * the wrong URL — these tests stub `fetch` and pin the exact request the
 * helper issues.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createListing, getListing, type ListingDraft } from './realtor-api';

// Deterministic base URL so the regression assertion pins the exact path
// (getApiBase() otherwise resolves to http://localhost:8081 under jsdom).
vi.mock('./env', () => ({ getApiBase: () => '' }));

const DRAFT: ListingDraft = {
  title: 'Byt na Hlavnej',
  description: 'A listing',
  propertyType: 'apartment',
  transactionType: 'sale',
  price: 120000,
  currency: 'EUR',
  city: 'Bratislava',
};

describe('createListing endpoint (regression #3039)', () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 201,
      json: async () => ({ id: 'lst_1' }),
    });
    vi.stubGlobal('fetch', fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.clearAllMocks();
  });

  it('POSTs to the create route /api/v1/my/listings, not the GET-only /api/v1/listings', async () => {
    await createListing(DRAFT);

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];

    // The bug: a POST to /api/v1/listings hits a GET-only route → 405.
    expect(url).toBe('/api/v1/my/listings');
    expect(url).not.toBe('/api/v1/listings');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toMatchObject({
      city: 'Bratislava',
      transactionType: 'sale',
      propertyType: 'apartment',
    });
  });

  it('still reads a single public listing from GET /api/v1/listings/{id}', async () => {
    // Guards against an over-eager find/replace of the listings path: the
    // public read route is unchanged by the create-path fix.
    await getListing('lst_42');
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit | undefined];
    expect(url).toBe('/api/v1/listings/lst_42');
    expect(init?.method ?? 'GET').toBe('GET');
  });
});
