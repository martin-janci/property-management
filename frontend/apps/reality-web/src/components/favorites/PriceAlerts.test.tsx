/**
 * PriceAlerts tests
 *
 * Regression guard for Story 84.3 (gap-84-3): before this surface existed
 * there was no reality-web UI consuming the price-tracking feed at
 * `GET /api/v1/favorites/alerts`. These tests pin that the dedicated
 * price-alert surface fetches the canonical endpoint and renders the
 * price-change alerts it returns.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { PriceAlerts } from './PriceAlerts';

function renderWithClient(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

const priceChangeAlert = {
  id: 'alert-1',
  favorite_id: 'fav-1',
  listing_id: 'listing-1',
  title: 'Sunny 2-bedroom apartment',
  alert_type: 'price_change',
  old_price: 200000,
  new_price: 180000,
  currency: 'EUR',
  change_percentage: -10,
  previous_status: null,
  new_status: null,
  status: 'pending',
  created_at: '2026-07-01T00:00:00Z',
  processed_at: null,
};

describe('PriceAlerts', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('fetches the canonical favorites alerts endpoint and renders the alert', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        alerts: [priceChangeAlert],
        unread_count: 1,
        limit: 100,
        offset: 0,
      }),
    });
    vi.stubGlobal('fetch', fetchMock);

    renderWithClient(<PriceAlerts />);

    await waitFor(() => expect(fetchMock).toHaveBeenCalled());
    const calledUrl = fetchMock.mock.calls[0][0] as string;
    // Regression bar: the feed must be the canonical price-tracking endpoint.
    expect(calledUrl).toMatch(/\/api\/v1\/favorites\/alerts\?/);

    await waitFor(() => expect(screen.getByText('Sunny 2-bedroom apartment')).toBeInTheDocument());
    // Unread alert exposes a mark-read control.
    expect(screen.getByRole('button', { name: 'markRead' })).toBeInTheDocument();
  });

  it('renders the empty state when there are no alerts', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ alerts: [], unread_count: 0, limit: 100, offset: 0 }),
    });
    vi.stubGlobal('fetch', fetchMock);

    renderWithClient(<PriceAlerts />);

    await waitFor(() => expect(screen.getByText('emptyTitle')).toBeInTheDocument());
  });
});
