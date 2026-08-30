/**
 * Favorites page — AC-5 "watch price" toggle tests (Story 84.3, #2365).
 *
 * PR #2348 wired the per-favorite price-alert toggle
 * (`PATCH /api/v1/favorites/{listingId}` → `useUpdateFavorite` → the checkbox
 * on this page) but shipped it with no test that actually executes — the two
 * backend authz tests are `#[ignore]`-quarantined and there was no frontend
 * coverage at all. These tests render the REAL page and drive the toggle end
 * to end, pinning:
 *   1. the checkbox reflects the favorite's `price_alert_enabled` from GET,
 *   2. toggling it OFF fires `PATCH …/{listingId}` with
 *      `{ price_alert_enabled: false }`,
 *   3. the optimistic cache update (added for #2365) flips the checkbox
 *      immediately and rolls back when the PATCH fails.
 *
 * Only framework / heavy-UI boundaries are mocked (auth-context so
 * ProtectedRoute renders its children, and the ListingCard / Header / Footer
 * chrome). The favorites data path and `useUpdateFavorite` are the real thing.
 */
/// <reference types="vitest/globals" />

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

// ProtectedRoute (via useAuth) — pretend the visitor is authenticated so the
// grid renders instead of the login gate.
vi.mock('@/lib/auth-context', () => ({
  useAuth: () => ({
    isLoading: false,
    isAuthenticated: true,
    user: { id: 'u-1' },
    login: vi.fn(),
    logout: vi.fn(),
    refreshSession: vi.fn(),
  }),
}));

// Heavy presentational chrome — irrelevant to the toggle behaviour under test.
vi.mock('@/components/listings', () => ({
  ListingCard: ({ listing }: { listing: { title?: string } }) => (
    <div data-testid="listing-card">{listing?.title}</div>
  ),
}));
vi.mock('@/components/ui', () => ({
  Header: () => <header />,
  Footer: () => <footer />,
}));

import FavoritesPage from './page';

const LISTING_ID = 'listing-1';

// The real reality-server `GET /api/v1/favorites` response: the
// `{ favorites: [...] }` envelope (FavoritesResponse) with snake_case,
// flattened listing fields (PortalFavoriteWithListing — no nested `listing`,
// no `data` envelope) and Decimal prices as strings. Feeding this exact shape
// makes the test fail if the client's casing/envelope mapping regresses.
function favoritesPayload(priceAlertEnabled: boolean) {
  return {
    favorites: [
      {
        id: 'fav-1',
        listing_id: LISTING_ID,
        title: 'Sunny 2-bedroom apartment',
        current_price: '185000.00',
        original_price: '199000.00',
        currency: 'EUR',
        city: 'Bratislava',
        property_type: 'apartment',
        transaction_type: 'sale',
        photo_url: null,
        status: 'active',
        price_changed: true,
        price_change_percentage: '-7.04',
        price_alert_enabled: priceAlertEnabled,
        created_at: '2026-07-01T00:00:00Z',
      },
    ],
  };
}

function renderPage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <FavoritesPage />
    </QueryClientProvider>
  );
}

describe('FavoritesPage — AC-5 watch-price toggle (#2365)', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the checkbox checked from GET and fires PATCH { price_alert_enabled: false } on toggle-off', async () => {
    // GET starts subscribed; PATCH + the invalidation refetch persist the OFF
    // state, so the round-trip mirrors the real backend list DTO.
    let alertEnabled = true;
    const fetchMock = vi.fn((url: string, init?: RequestInit) => {
      const method = init?.method ?? 'GET';
      if (method === 'PATCH') {
        alertEnabled = JSON.parse(String(init?.body)).price_alert_enabled;
        return Promise.resolve({ ok: true, status: 200, json: async () => ({}) });
      }
      return Promise.resolve({
        ok: true,
        status: 200,
        json: async () => favoritesPayload(alertEnabled),
      });
    });
    vi.stubGlobal('fetch', fetchMock);

    renderPage();

    const checkbox = await screen.findByRole('checkbox');
    expect(checkbox).toBeChecked();

    fireEvent.click(checkbox);

    // PATCH hit the per-favorite endpoint with the unsubscribe body.
    await waitFor(() => {
      const patch = fetchMock.mock.calls.find(([, init]) => init?.method === 'PATCH');
      expect(patch).toBeDefined();
      expect(patch?.[0]).toMatch(new RegExp(`/api/v1/favorites/${LISTING_ID}$`));
      expect(JSON.parse(String(patch?.[1]?.body))).toEqual({ price_alert_enabled: false });
    });

    // After the round-trip the checkbox stays unchecked (state persisted).
    await waitFor(() => expect(checkbox).not.toBeChecked());
  });

  it('optimistically unchecks immediately and rolls back to checked when the PATCH fails', async () => {
    // Gate the PATCH so it stays pending long enough to observe the optimistic
    // flip deterministically, then reject it to exercise the rollback.
    let rejectPatch: (() => void) | undefined;
    const patchGate = new Promise<{ ok: boolean; status: number; json: () => Promise<unknown> }>(
      (resolve) => {
        rejectPatch = () => resolve({ ok: false, status: 500, json: async () => ({}) });
      }
    );
    const fetchMock = vi.fn((url: string, init?: RequestInit) => {
      if ((init?.method ?? 'GET') === 'PATCH') return patchGate;
      return Promise.resolve({ ok: true, status: 200, json: async () => favoritesPayload(true) });
    });
    vi.stubGlobal('fetch', fetchMock);

    renderPage();

    expect(await screen.findByRole('checkbox')).toBeChecked();

    fireEvent.click(screen.getByRole('checkbox'));

    // Optimistic update flips it off while the PATCH is still in flight.
    await waitFor(() => expect(screen.getByRole('checkbox')).not.toBeChecked());

    // Server rejects → onError rollback + onSettled refetch restore the checked
    // state, so the UI never lies about a change that didn't persist.
    rejectPatch?.();
    await waitFor(() => expect(screen.getByRole('checkbox')).toBeChecked());
  });

  it('surfaces an error alert when the update mutation fails (no longer silently swallowed)', async () => {
    // Regression for the swallowed-mutation-error bug: a failing PATCH must
    // reach the user via an alert, not disappear. On main this asserts nothing
    // renders (no `onError`/`isError` UI existed); with the fix the page shows
    // the generic `error.description` message. `useTranslations` is mocked to
    // echo the key, so the alert text is the literal key `description`.
    const fetchMock = vi.fn((url: string, init?: RequestInit) => {
      if ((init?.method ?? 'GET') === 'PATCH') {
        return Promise.resolve({ ok: false, status: 500, json: async () => ({}) });
      }
      return Promise.resolve({ ok: true, status: 200, json: async () => favoritesPayload(true) });
    });
    vi.stubGlobal('fetch', fetchMock);

    renderPage();

    fireEvent.click(await screen.findByRole('checkbox'));

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('description');
  });
});
