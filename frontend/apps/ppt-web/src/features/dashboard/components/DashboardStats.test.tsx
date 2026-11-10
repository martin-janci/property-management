/// <reference types="vitest/globals" />
/**
 * Manager dashboard quick stats come from live counts (round-3 audit #16),
 * not the former hard-coded "3 / 2 / 1 / 5" with fixed trend arrows.
 */
import { render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

const state = vi.hoisted(() => ({
  faultStats: undefined as unknown,
  votes: undefined as unknown,
  unread: undefined as unknown,
}));

vi.mock('@ppt/api-client', () => ({
  useFaultStatistics: () => ({ data: state.faultStats }),
  useVotes: (query: unknown) => {
    expect(query).toEqual({ status: 'active' });
    return { data: state.votes };
  },
}));
vi.mock('../../messaging/hooks/useMessaging', () => ({
  useUnreadCount: () => ({ data: state.unread }),
}));

import { DashboardStats } from './DashboardStats';

function tileValue(label: string): string | null | undefined {
  const tile = screen.getByText(label).closest('.quick-stat') as HTMLElement;
  return within(tile).getByText(/./, { selector: '.quick-stat__value' }).textContent;
}

describe('DashboardStats', () => {
  it('shows the counts the API returns', () => {
    state.faultStats = {
      total_count: 9,
      open_count: 6,
      closed_count: 3,
      by_status: [
        { status: 'in_progress', count: 4 },
        { status: 'new', count: 7 },
      ],
      by_category: [],
      by_priority: [],
      average_resolution_time_hours: null,
      average_rating: null,
    };
    state.votes = [{ id: 'v1' }, { id: 'v2' }];
    state.unread = { unreadCount: 11 };

    render(<DashboardStats mode="view" props={{}} />);

    expect(tileValue('Pending faults')).toBe('7');
    expect(tileValue('Active votes')).toBe('2');
    expect(tileValue('Unread messages')).toBe('11');
  });

  it('shows a dash instead of a number while data is missing', () => {
    state.faultStats = undefined;
    state.votes = undefined;
    state.unread = undefined;

    render(<DashboardStats mode="view" props={{}} />);

    expect(tileValue('Pending faults')).toBe('–');
    expect(tileValue('Active votes')).toBe('–');
    expect(tileValue('Unread messages')).toBe('–');
  });

  it('does not render the pending-approvals tile, which has no data source', () => {
    render(<DashboardStats mode="view" props={{}} />);
    expect(screen.queryByText('Pending approvals')).not.toBeInTheDocument();
  });
});
