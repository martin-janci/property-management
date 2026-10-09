/**
 * FeaturedListings — favorite-toggle error-surfacing tests.
 *
 * Regression for the swallowed-mutation-error bug: the homepage featured
 * sections wire each card's heart button to `useToggleFavorite().mutate(...)`
 * with no `onError`/`isError` handling, so a failing POST/DELETE
 * `/api/v1/favorites/{id}` vanished silently — the heart UI diverges from
 * server state with zero user feedback. The fix mirrors the established
 * reality-web pattern already used on the favorites page (`mutation.isError`
 * → a `role="alert"` div showing the generic `error.description` message).
 *
 * These tests render the REAL FeaturedListings + ListingCard and drive the
 * real heart button, mocking only the data boundary (`fetch`). On `dev` the
 * failure test asserts nothing renders (no alert existed); with the fix it
 * finds the alert. `useTranslations` is mocked (src/test/setup.tsx) to echo
 * the key, so the alert text is the literal key `description`.
 */
/// <reference types="vitest/globals" />

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { FeaturedListings } from './FeaturedListings';

const LISTING_ID = 'listing-1';

// A single featured sale listing — enough shape for ListingCard to render and
// expose its heart button. The other two sections are empty.
function featuredPayload() {
  return {
    sale: [
      {
        id: LISTING_ID,
        title: 'Sunny 2-bedroom apartment',
        slug: 'sunny-2-bedroom-apartment',
        propertyType: 'apartment',
        transactionType: 'sale',
        status: 'active',
        price: 185000,
        currency: 'EUR',
        area: 72,
        rooms: 3,
        address: { city: 'Bratislava', country: 'SK' },
        isFeatured: true,
        isFavorite: false,
        createdAt: '2026-07-01T00:00:00Z',
        updatedAt: '2026-07-01T00:00:00Z',
      },
    ],
    rent: [],
    new: [],
  };
}

function renderFeatured() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <FeaturedListings />
    </QueryClientProvider>
  );
}

describe('FeaturedListings — favorite-toggle error handling', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('surfaces an error alert when the favorite toggle fails (no longer silently swallowed)', async () => {
    // GET featured succeeds; the favorite POST fails. The failure must reach
    // the user via a role="alert", not disappear.
    const fetchMock = vi.fn((_url: string, init?: RequestInit) => {
      const method = init?.method ?? 'GET';
      if (method === 'POST' || method === 'DELETE') {
        return Promise.resolve({ ok: false, status: 500, json: async () => ({}) });
      }
      return Promise.resolve({ ok: true, status: 200, json: async () => featuredPayload() });
    });
    vi.stubGlobal('fetch', fetchMock);

    renderFeatured();

    const heart = await screen.findByLabelText('Add to favorites');
    fireEvent.click(heart);

    // The failing toggle hit the favorites endpoint...
    await waitFor(() => {
      const call = fetchMock.mock.calls.find(([, i]) => (i?.method ?? 'GET') !== 'GET');
      expect(call).toBeDefined();
      expect(call?.[0]).toMatch(new RegExp(`/api/v1/favorites/${LISTING_ID}$`));
    });

    // ...and the user sees the generic error message instead of silence.
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('description');
  });

  it('shows no error alert while the toggle succeeds', async () => {
    const fetchMock = vi.fn((_url: string, init?: RequestInit) => {
      const method = init?.method ?? 'GET';
      if (method === 'POST' || method === 'DELETE') {
        return Promise.resolve({ ok: true, status: 200, json: async () => ({}) });
      }
      return Promise.resolve({ ok: true, status: 200, json: async () => featuredPayload() });
    });
    vi.stubGlobal('fetch', fetchMock);

    renderFeatured();

    const heart = await screen.findByLabelText('Add to favorites');
    fireEvent.click(heart);

    await waitFor(() => {
      expect(fetchMock.mock.calls.some(([, i]) => (i?.method ?? 'GET') !== 'GET')).toBe(true);
    });
    expect(screen.queryByRole('alert')).toBeNull();
  });
});
