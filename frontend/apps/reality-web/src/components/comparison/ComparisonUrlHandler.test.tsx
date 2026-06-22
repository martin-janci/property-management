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
});
