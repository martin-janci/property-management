/**
 * ComparisonUrlHandler Tests
 *
 * Regression guard for the shared-comparison 404 bug (Epic 51 Story 51.3):
 * the handler used to fetch a non-existent Next.js route `/api/listings/${id}`,
 * so every shared comparison URL 404'd and silently rendered the error state.
 * The fix routes the fetch through reality-server's canonical endpoint
 * `${getApiBase()}/api/v1/listings/${id}`.
 */

import type { ListingSummary } from '@ppt/reality-api-client';
import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const addToComparison = vi.fn();

// Mock the comparison context: empty comparison so the loader runs.
vi.mock('../../lib/comparison-context', () => ({
  useComparison: () => ({ listings: [], addToComparison }),
}));

// Pin getApiBase so the asserted URL is deterministic in jsdom.
vi.mock('../../lib/env', () => ({
  getApiBase: () => 'https://api.test',
}));

// Stub next-intl so the component's localized strings resolve in jsdom
// without a NextIntlClientProvider. The handler renders t('loading') /
// t('loadError'); returning the key keeps the i18n wiring under test.
vi.mock('next-intl', () => ({
  useTranslations: () => (key: string) => `comparison.${key}`,
}));

import { ComparisonUrlHandler } from './ComparisonUrlHandler';

const mockListing: ListingSummary = {
  id: 'abc',
  slug: 'abc-listing',
  title: 'Listing abc',
  price: 100000,
  currency: 'EUR',
  transactionType: 'sale',
  propertyType: 'apartment',
  status: 'active',
  area: 60,
  rooms: 2,
  floor: 1,
  address: { city: 'Bratislava', district: 'Old Town', country: 'Slovakia' },
  isFavorite: false,
  isFeatured: false,
  createdAt: '2024-01-01T00:00:00Z',
  updatedAt: '2024-01-01T00:00:00Z',
} as ListingSummary;

describe('ComparisonUrlHandler', () => {
  beforeEach(() => {
    addToComparison.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('fetches each shared listing via the canonical /api/v1/listings path', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => mockListing,
    });
    vi.stubGlobal('fetch', fetchMock);

    render(<ComparisonUrlHandler sharedIds={['abc']} />);

    await waitFor(() => expect(fetchMock).toHaveBeenCalled());

    const calledUrl = fetchMock.mock.calls[0][0] as string;
    // Regression bar: the wrong URL (/api/listings/abc) silently 404'd.
    expect(calledUrl).toBe('https://api.test/api/v1/listings/abc');
    expect(calledUrl).toMatch(/\/api\/v1\/listings\/abc$/);

    await waitFor(() => expect(addToComparison).toHaveBeenCalledWith(mockListing));
  });

  it('renders the error state when the upstream listing 404s', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => ({}),
    });
    vi.stubGlobal('fetch', fetchMock);
    vi.spyOn(console, 'error').mockImplementation(() => {});

    render(<ComparisonUrlHandler sharedIds={['missing']} />);

    await waitFor(() => expect(screen.getByRole('alert')).toBeInTheDocument());
    expect(addToComparison).not.toHaveBeenCalled();
  });

  it('degrades gracefully: one bad id is dropped, the good ones still load', async () => {
    // Regression bar for the all-or-nothing bug: `Promise.all` used to reject
    // on the first failure, so a single stale/missing id blanked the entire
    // shared comparison. The good listings must survive.
    const goodA = { ...mockListing, id: 'good-a', slug: 'good-a' };
    const goodB = { ...mockListing, id: 'good-b', slug: 'good-b' };

    const fetchMock = vi.fn(async (url: string) => {
      if (url.endsWith('/bad')) {
        return { ok: false, status: 404, json: async () => ({}) };
      }
      const id = url.endsWith('/good-a') ? goodA : goodB;
      return { ok: true, status: 200, json: async () => id };
    });
    vi.stubGlobal('fetch', fetchMock);
    vi.spyOn(console, 'error').mockImplementation(() => {});

    render(<ComparisonUrlHandler sharedIds={['good-a', 'bad', 'good-b']} />);

    // Both good listings are added despite the bad id in the middle.
    await waitFor(() => expect(addToComparison).toHaveBeenCalledWith(goodA));
    await waitFor(() => expect(addToComparison).toHaveBeenCalledWith(goodB));

    // The bad id is dropped — it is never handed to the comparison. Every
    // added listing is one of the two good ones (guard against the harness
    // re-running the effect since the mocked context stays empty).
    for (const [added] of addToComparison.mock.calls) {
      expect(['good-a', 'good-b']).toContain(added.id);
    }

    // The comparison is usable, so the error state must NOT blank it out.
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
});
