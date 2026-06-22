/**
 * FaultReportsPage tests (Story 4.7, BIT-186).
 * Covers manager-only gating, loading / empty / error states, KPI + chart
 * rendering, drill-down, and CSV export wiring.
 */

/// <reference types="vitest/globals" />
import { fireEvent, render, screen } from '@testing-library/react';
import type { FaultStatistics } from '@ppt/api-client';
import { FaultReportsPage } from './FaultReportsPage';

const noop = () => {};

const baseProps = {
  isManager: true,
  buildings: [{ id: 'b-1', name: 'Maple Court' }],
  filters: {},
  onFiltersChange: noop,
  onDrillDown: noop,
};

const stats: FaultStatistics = {
  total_count: 12,
  open_count: 5,
  closed_count: 7,
  by_status: [
    { status: 'new', count: 3 },
    { status: 'waiting_parts', count: 2 },
  ],
  by_category: [{ category: 'plumbing', count: 8 }],
  by_priority: [{ priority: 'high', count: 4 }],
  average_resolution_time_hours: 36.5,
  average_rating: 4.25,
};

describe('FaultReportsPage', () => {
  it('shows an access notice for non-managers and no charts', () => {
    render(<FaultReportsPage {...baseProps} isManager={false} stats={stats} />);
    expect(screen.getByRole('alert')).toHaveTextContent('do not have permission');
    expect(screen.queryByText('By status')).not.toBeInTheDocument();
  });

  it('renders a loading spinner when isLoading', () => {
    const { container } = render(<FaultReportsPage {...baseProps} isLoading />);
    expect(container.querySelector('[aria-busy="true"]')).toBeInTheDocument();
  });

  it('renders an error state with a retry button', () => {
    const onRetry = vi.fn();
    render(<FaultReportsPage {...baseProps} isError onRetry={onRetry} />);
    expect(screen.getByRole('alert')).toHaveTextContent('Failed to load fault statistics');
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it('renders the empty state when there are no faults', () => {
    render(<FaultReportsPage {...baseProps} stats={{ ...stats, total_count: 0 }} />);
    expect(screen.getByText('No faults match the selected filters.')).toBeInTheDocument();
  });

  it('renders KPIs and breakdown charts when stats are present', () => {
    render(<FaultReportsPage {...baseProps} stats={stats} />);
    expect(screen.getByText('Total faults')).toBeInTheDocument();
    expect(screen.getByText('36.5')).toBeInTheDocument(); // avg resolution hours
    expect(screen.getByText('4.25 ★')).toBeInTheDocument(); // avg rating
    expect(screen.getByLabelText('By status')).toBeInTheDocument();
    expect(screen.getByLabelText('By category')).toBeInTheDocument();
    expect(screen.getByLabelText('By priority')).toBeInTheDocument();
    // snake_case status humanised
    expect(screen.getByText('Waiting parts')).toBeInTheDocument();
  });

  it('calls onDrillDown with the raw bucket key when a bar is clicked', () => {
    const onDrillDown = vi.fn();
    render(<FaultReportsPage {...baseProps} stats={stats} onDrillDown={onDrillDown} />);
    fireEvent.click(screen.getByRole('button', { name: /New: 3.*View these faults/ }));
    expect(onDrillDown).toHaveBeenCalledWith({ status: 'new' });
  });

  it('exports a CSV blob when Export CSV is clicked', () => {
    const createObjectURL = vi.fn(() => 'blob:fake');
    const revokeObjectURL = vi.fn();
    // jsdom lacks these — stub them.
    vi.stubGlobal('URL', { ...URL, createObjectURL, revokeObjectURL });
    const clickSpy = vi
      .spyOn(HTMLAnchorElement.prototype, 'click')
      .mockImplementation(() => {});

    render(<FaultReportsPage {...baseProps} stats={stats} />);
    fireEvent.click(screen.getByRole('button', { name: 'Export CSV' }));
    expect(createObjectURL).toHaveBeenCalledOnce();
    expect(clickSpy).toHaveBeenCalledOnce();

    clickSpy.mockRestore();
    vi.unstubAllGlobals();
  });
});
