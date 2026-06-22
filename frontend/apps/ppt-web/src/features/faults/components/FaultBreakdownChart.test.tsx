/**
 * FaultBreakdownChart tests (Story 4.7, BIT-186).
 * Covers accessible labels, empty state, and drill-down clicks.
 */

/// <reference types="vitest/globals" />
import { fireEvent, render, screen } from '@testing-library/react';
import { FaultBreakdownChart } from './FaultBreakdownChart';

const data = [
  { key: 'new', label: 'New', count: 3 },
  { key: 'closed', label: 'Closed', count: 1 },
];

describe('FaultBreakdownChart', () => {
  it('renders an empty message when there is no data', () => {
    render(<FaultBreakdownChart title="By status" data={[]} emptyLabel="No data" />);
    expect(screen.getByText('No data')).toBeInTheDocument();
  });

  it('renders accessible buttons with share percentages when onSelect is set', () => {
    const onSelect = vi.fn();
    render(<FaultBreakdownChart title="By status" data={data} onSelect={onSelect} />);
    const btn = screen.getByRole('button', { name: /New: 3 \(75%\).*View these faults/ });
    fireEvent.click(btn);
    expect(onSelect).toHaveBeenCalledWith('new');
  });

  it('renders non-interactive rows (no buttons) when onSelect is omitted', () => {
    render(<FaultBreakdownChart title="By status" data={data} />);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
    expect(screen.getByLabelText('New: 3 (75%)')).toBeInTheDocument();
  });
});
