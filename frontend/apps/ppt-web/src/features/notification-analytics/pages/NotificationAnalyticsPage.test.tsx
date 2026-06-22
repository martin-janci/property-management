/**
 * NotificationAnalyticsPage tests (Story 2B-C.3, PM #969 gap 4 / BIT-214).
 * Covers operator-only / forbidden gating, loading / error states, the firing
 * failure-rate alert banner, totals KPIs, and the per-channel table.
 */

/// <reference types="vitest/globals" />
import { fireEvent, render, screen } from '@testing-library/react';
import type { NotificationAnalyticsResponse } from '../types';
import { NotificationAnalyticsPage } from './NotificationAnalyticsPage';

const noop = () => {};

const baseProps = {
  isOperator: true,
  filters: { window: '24h' as const },
  onFiltersChange: noop,
};

const data: NotificationAnalyticsResponse = {
  from: '2026-06-21T18:00:00Z',
  to: '2026-06-22T18:00:00Z',
  channel_filter: null,
  totals: {
    channel: 'all',
    sent: 23,
    delivered: 0,
    failed: 2,
    skipped: 3,
    opened: 0,
    clicked: 0,
    attempted: 25,
    failure_rate: 0.08,
  },
  by_channel: [
    {
      channel: 'email',
      sent: 18,
      delivered: 0,
      failed: 2,
      skipped: 1,
      opened: 0,
      clicked: 0,
      attempted: 20,
      failure_rate: 0.1,
    },
  ],
  alert: { firing: true, threshold: 0.05, failure_rate: 0.08, attempted: 25, min_attempts: 20 },
};

describe('NotificationAnalyticsPage', () => {
  it('shows an access notice for non-operators and no table', () => {
    render(<NotificationAnalyticsPage {...baseProps} isOperator={false} data={data} />);
    expect(screen.getByRole('alert')).toHaveTextContent('do not have permission');
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
  });

  it('shows the forbidden notice when the backend rejects with 403', () => {
    render(<NotificationAnalyticsPage {...baseProps} isForbidden data={undefined} />);
    expect(screen.getByRole('alert')).toHaveTextContent('do not have permission');
  });

  it('renders a loading spinner when isLoading', () => {
    const { container } = render(<NotificationAnalyticsPage {...baseProps} isLoading />);
    expect(container.querySelector('[aria-busy="true"]')).toBeInTheDocument();
  });

  it('renders an error state with a retry button', () => {
    const onRetry = vi.fn();
    render(<NotificationAnalyticsPage {...baseProps} isError onRetry={onRetry} />);
    expect(screen.getByRole('alert')).toHaveTextContent('Failed to load notification analytics');
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it('renders the firing alert banner, totals KPIs, and per-channel rows', () => {
    render(<NotificationAnalyticsPage {...baseProps} data={data} />);
    // Alert banner driven by alert.firing
    expect(screen.getByText('Elevated notification failure rate')).toBeInTheDocument();
    // Totals failure rate rendered as a percentage
    expect(screen.getAllByText('8.0%').length).toBeGreaterThan(0);
    // Per-channel table row ("Email" also appears as a filter <option>, so
    // scope to the table cell to disambiguate).
    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(screen.getByRole('cell', { name: 'Email' })).toBeInTheDocument();
    expect(screen.getByText('10.0%')).toBeInTheDocument();
  });

  it('renders the empty state when there is no channel activity', () => {
    render(
      <NotificationAnalyticsPage
        {...baseProps}
        data={{
          ...data,
          by_channel: [],
          totals: { ...data.totals, attempted: 0, sent: 0, failed: 0, failure_rate: 0 },
          alert: { ...data.alert, firing: false, failure_rate: 0, attempted: 0 },
        }}
      />
    );
    expect(
      screen.getByText('No notification activity in the selected window.')
    ).toBeInTheDocument();
  });
});
