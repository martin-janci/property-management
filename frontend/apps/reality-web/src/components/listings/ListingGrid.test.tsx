/**
 * ListingGrid Component Tests
 *
 * Regression coverage for the state surfaces (loading / error / empty / results).
 * The error branch guards against the bug where an API failure fell through to
 * the empty ("no results") state, hiding the failure from the user.
 */

import type { ListingSummary } from '@ppt/reality-api-client';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ListingGrid } from './ListingGrid';

const mockListing: ListingSummary = {
  id: 'listing-1',
  slug: 'beautiful-apartment-bratislava',
  title: 'Beautiful Apartment in Bratislava',
  price: 150000,
  currency: 'EUR',
  transactionType: 'sale',
  propertyType: 'apartment',
  status: 'active',
  area: 75,
  rooms: 3,
  floor: 2,
  address: {
    city: 'Bratislava',
    district: 'Old Town',
    country: 'Slovakia',
  },
  isFavorite: false,
  isFeatured: false,
  createdAt: '2024-01-01T00:00:00Z',
  updatedAt: '2024-01-01T00:00:00Z',
};

describe('ListingGrid', () => {
  it('renders listing cards when results are present', () => {
    render(<ListingGrid listings={[mockListing]} viewMode="grid" />);
    expect(screen.getByText('Beautiful Apartment in Bratislava')).toBeInTheDocument();
  });

  it('renders the empty state when there are no results and no error', () => {
    render(<ListingGrid listings={[]} viewMode="grid" />);
    // EmptyState uses role="status"; the mocked translator echoes the key.
    expect(screen.getByRole('status')).toBeInTheDocument();
    expect(screen.getByText('emptyTitle')).toBeInTheDocument();
  });

  it('renders a distinct error state (not the empty state) on API error', () => {
    render(<ListingGrid listings={[]} viewMode="grid" isError />);

    // ErrorState uses role="alert" and the error copy — NOT the empty copy.
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText('errorTitle')).toBeInTheDocument();
    expect(screen.getByText('errorDescription')).toBeInTheDocument();

    // Regression guard: the empty ("no results") state must not be shown.
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(screen.queryByText('emptyTitle')).not.toBeInTheDocument();
  });

  it('shows a retry action that invokes onRetry when errored', () => {
    const onRetry = vi.fn();
    render(<ListingGrid listings={[]} viewMode="grid" isError onRetry={onRetry} />);

    const retryButton = screen.getByRole('button', { name: 'retry' });
    fireEvent.click(retryButton);
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('prioritizes the loading state over the error state', () => {
    render(<ListingGrid listings={[]} viewMode="grid" isLoading isError />);
    // Skeleton has neither the alert nor status role.
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.queryByText('errorTitle')).not.toBeInTheDocument();
  });
});
