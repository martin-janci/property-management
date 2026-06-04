/**
 * FaultsPage state-wiring regression test (gap-79-1).
 * Verifies the loading / error / empty triad threaded from the route's
 * TanStack Query hook (isLoading / isError / onRetry) reaches the UI.
 */

/// <reference types="vitest/globals" />
import { fireEvent, render, screen } from '@testing-library/react';
import { FaultsPage } from './FaultsPage';

const noop = () => {};

const baseProps = {
  faults: [],
  total: 0,
  onNavigateToCreate: noop,
  onNavigateToView: noop,
  onNavigateToEdit: noop,
  onNavigateToTriage: noop,
  onFilterChange: noop,
};

describe('FaultsPage state wiring', () => {
  it('renders the empty state when not loading and no error', () => {
    render(<FaultsPage {...baseProps} isLoading={false} />);
    expect(screen.getByText('No faults found.')).toBeInTheDocument();
  });

  it('renders an inline error state (not the empty state) when isError is true', () => {
    render(<FaultsPage {...baseProps} isError />);
    expect(screen.getByRole('alert')).toHaveTextContent('Failed to load faults');
    expect(screen.queryByText('No faults found.')).not.toBeInTheDocument();
  });

  it('invokes onRetry from the error state retry button', () => {
    const onRetry = vi.fn();
    render(<FaultsPage {...baseProps} isError onRetry={onRetry} />);
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('does not show the empty state while loading', () => {
    render(<FaultsPage {...baseProps} isLoading />);
    expect(screen.queryByText('No faults found.')).not.toBeInTheDocument();
  });
});
