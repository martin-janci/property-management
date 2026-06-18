/**
 * FaultsPage state-wiring regression test (gap-79-1).
 * Verifies the loading / error / empty triad threaded from the route's
 * TanStack Query hook (isLoading / isError / onRetry) reaches the UI.
 * Also covers the new stats summary bar and delete action wiring.
 */

/// <reference types="vitest/globals" />
import { fireEvent, render, screen } from '@testing-library/react';
import type { FaultSummary } from '../components/FaultCard';
import { FaultsPage } from './FaultsPage';

const noop = () => {};

const baseProps = {
  faults: [] as FaultSummary[],
  total: 0,
  onNavigateToCreate: noop,
  onNavigateToView: noop,
  onNavigateToEdit: noop,
  onNavigateToTriage: noop,
  onFilterChange: noop,
};

const newFault: FaultSummary = {
  id: 'f-1',
  buildingId: 'b-1',
  title: 'Leaking pipe',
  category: 'plumbing',
  priority: 'high',
  status: 'new',
  createdAt: '2026-06-01T00:00:00Z',
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

  it('renders the statistics summary bar when stats prop is provided', () => {
    render(<FaultsPage {...baseProps} stats={{ total_count: 15, open_count: 10 }} />);
    const statsBar = screen.getByLabelText('Fault statistics');
    expect(statsBar).toBeInTheDocument();
    expect(statsBar).toHaveTextContent('15'); // total
    expect(statsBar).toHaveTextContent('10'); // open
    expect(statsBar).toHaveTextContent('5'); // closed = 15 - 10
  });

  it('does not render the statistics bar when stats prop is omitted', () => {
    render(<FaultsPage {...baseProps} />);
    expect(screen.queryByLabelText('Fault statistics')).not.toBeInTheDocument();
  });

  it('invokes onDelete when Delete button is clicked on a new-status fault', () => {
    const onDelete = vi.fn();
    render(<FaultsPage {...baseProps} faults={[newFault]} total={1} onDelete={onDelete} />);
    fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    expect(onDelete).toHaveBeenCalledTimes(1);
    expect(onDelete).toHaveBeenCalledWith('f-1');
  });
});
