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
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { PriceAlerts } from './PriceAlerts';

function renderWithClient(ui: React.ReactElement) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

// Real wire shape: reality-server serializes the `rust_decimal::Decimal`
// price / percentage fields as JSON **strings** (serde-str), not numbers. The
// digit counts differ (6 vs 5), so a lexicographic string compare would rank
// "95000.00" > "180000.00" and mislabel this drop as an increase — the exact
// bug this fixture now guards against.
const priceDropAlert = {
  id: 'alert-1',
  favorite_id: 'fav-1',
  listing_id: 'listing-1',
  title: 'Sunny 2-bedroom apartment',
  alert_type: 'price_change',
  old_price: '180000.00',
  new_price: '95000.00',
  currency: 'EUR',
  change_percentage: '-47.2',
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
        alerts: [priceDropAlert],
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

  it('classifies a string-wire price drop as a drop, not an increase', async () => {
    // 180000 → 95000 is a drop. Because the wire delivers Decimals as strings
    // with differing digit counts, a lexicographic compare ('95000.00' >
    // '180000.00') would mislabel it as an increase. The `useTranslations` test
    // mock returns the key verbatim, so the direction label is the raw
    // 'drop' / 'increase' key.
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        alerts: [priceDropAlert],
        unread_count: 1,
        limit: 100,
        offset: 0,
      }),
    });
    vi.stubGlobal('fetch', fetchMock);

    renderWithClient(<PriceAlerts />);

    await waitFor(() => expect(screen.getByText('Sunny 2-bedroom apartment')).toBeInTheDocument());
    // The direction label must be the 'drop' translation key, never 'increase'.
    expect(screen.getByText('drop')).toBeInTheDocument();
    expect(screen.queryByText('increase')).not.toBeInTheDocument();
  });

  it('surfaces a visible error when the mark-read mutation fails', async () => {
    // GET the feed successfully (one unread alert), then fail the POST that
    // marks it read. Before this fix the rejected mutation was swallowed: the
    // row stayed unread with no user feedback. The regression bar is that a
    // failed mark-read now renders an inline, assertive role="alert" banner
    // (the shared `error.description` copy — the test next-intl mock echoes the
    // key, so the banner text is the raw 'description' key).
    const fetchMock = vi.fn((url: string) => {
      if (typeof url === 'string' && /\/alerts\/[^/]+\/read$/.test(url)) {
        return Promise.resolve({ ok: false, status: 500, json: async () => ({}) });
      }
      return Promise.resolve({
        ok: true,
        status: 200,
        json: async () => ({
          alerts: [priceDropAlert],
          unread_count: 1,
          limit: 100,
          offset: 0,
        }),
      });
    });
    vi.stubGlobal('fetch', fetchMock);

    renderWithClient(<PriceAlerts />);

    const markReadButton = await screen.findByRole('button', { name: 'markRead' });
    fireEvent.click(markReadButton);

    // The failed mutation must produce a user-visible, assertive alert. The
    // feed GET succeeded, so the only role="alert" is the action-error banner.
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('description');
    expect(alert).toHaveAttribute('aria-live', 'assertive');
    // The mark-read request was actually attempted (not silently skipped).
    expect(
      fetchMock.mock.calls.some(([u]) => typeof u === 'string' && /\/alerts\/[^/]+\/read$/.test(u))
    ).toBe(true);
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
