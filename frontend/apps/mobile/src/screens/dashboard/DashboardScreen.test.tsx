/**
 * DashboardScreen — error-state regression test (issue #2282).
 *
 * The dashboard fires four independent `useApiQuery` calls (announcements,
 * fault stats, votes, unread messages) and previously consumed only `.data`.
 * On any failure the stat derivations coalesce to `?? 0` / `?? []`, so the
 * screen silently rendered an all-zero dashboard indistinguishable from a
 * genuinely-empty building. The fix destructures `isError` and renders a
 * distinct, retryable banner (wired to the existing pull-to-refresh
 * `onRefresh`) above the zeroed stats.
 *
 * These tests lock the contract: no banner when everything loads, a banner
 * the moment ANY query errors, and a retry button that refetches all four
 * queries. i18n `t` returns the key in tests (see src/test/setup.ts), so the
 * banner asserts on the `dashboard.loadError` / `common.retry` keys.
 *
 * NOTE: `useDashboardLayout` and `LayoutSections` are mocked so this suite
 * stays focused on DashboardScreen's own four-query contract and error-banner
 * logic. Stats/actions are rendered by registry components which have their
 * own tests in layout.test.tsx.
 */

import { fireEvent, render, screen } from '@testing-library/react-native';
import { DashboardScreen } from './DashboardScreen';

// --- Mocks ---

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

jest.mock('../../contexts/AuthContext', () => ({
  useAuth: () => ({
    user: { firstName: 'Ada', lastName: 'Byron' },
    logout: jest.fn(),
  }),
}));

// Mock useDashboardLayout so the layout hook doesn't trigger additional
// useApiQuery calls through the section registry components.
jest.mock('../../features/layout/useDashboardLayout', () => ({
  useDashboardLayout: jest.fn(() => ({
    layout: {
      screen: 'ppt/dashboard',
      version: 0,
      sections: [],
    },
  })),
}));

// Mock LayoutSections to avoid rendering registry components (which also
// call useApiQuery) so this suite stays focused on DashboardScreen's own
// four-query contract and error-banner logic.
jest.mock('../../features/layout/LayoutSections', () => ({
  LayoutSections: () => null,
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

// --- Fixtures ---

const ANNOUNCEMENTS = {
  announcements: [
    {
      id: 'a-1',
      title: 'Lift maintenance',
      status: 'published',
      pinned: false,
      created_at: '2026-06-20T10:00:00Z',
    },
  ],
};
const FAULT_STATS = { statistics: { open_count: 2, in_progress_count: 1 } };
const VOTES = {
  votes: [{ id: 'v-1', title: 'New bike shed', status: 'active', end_at: '2026-07-20T10:00:00Z' }],
};
const UNREAD = { unreadCount: 5 };

interface QueryOverrides {
  data?: unknown;
  isError?: boolean;
  refetch?: jest.Mock;
}

function queryResult({
  data = undefined,
  isError = false,
  refetch = jest.fn(),
}: QueryOverrides = {}) {
  return {
    data,
    isLoading: false,
    isError,
    error: isError ? new Error('boom') : null,
    isFetching: false,
    refetch,
  };
}

/**
 * Mock the four queries in call order:
 * announcements, faultsStats, votes, unreadMessages.
 */
function mockQueries(results: ReturnType<typeof queryResult>[]) {
  mockUseApiQuery.mockReset();
  for (const r of results) {
    mockUseApiQuery.mockReturnValueOnce(r);
  }
}

const allSuccess = () => [
  queryResult({ data: ANNOUNCEMENTS }),
  queryResult({ data: FAULT_STATS }),
  queryResult({ data: VOTES }),
  queryResult({ data: UNREAD }),
];

// --- Tests ---

describe('DashboardScreen error state (#2282)', () => {
  afterEach(() => jest.clearAllMocks());

  it('renders no error banner and shows announcement data when all queries succeed', () => {
    mockQueries(allSuccess());
    render(<DashboardScreen />);

    expect(screen.queryByText('dashboard.loadError')).toBeNull();
    // The announcements section renders real data from the query
    expect(screen.getByText('Lift maintenance')).toBeTruthy();
  });

  it('shows the error banner when ANY query fails (not a silent all-zero state)', () => {
    // Only the fault-stats query fails; the other three succeed. Without the
    // fix the fault card would read 0 and the screen would look "empty".
    mockQueries([
      queryResult({ data: ANNOUNCEMENTS }),
      queryResult({ isError: true }),
      queryResult({ data: VOTES }),
      queryResult({ data: UNREAD }),
    ]);
    render(<DashboardScreen />);

    expect(screen.getByText('dashboard.loadError')).toBeTruthy();
    expect(screen.getByText('common.retry')).toBeTruthy();
  });

  it('does not distinguish a genuinely-empty building from an error — empty data shows NO banner', () => {
    // All queries succeed but return empty/zeroed payloads: this is a real
    // empty building, so there must be no error banner.
    mockQueries([
      queryResult({ data: { announcements: [] } }),
      queryResult({ data: { statistics: { open_count: 0, in_progress_count: 0 } } }),
      queryResult({ data: { votes: [] } }),
      queryResult({ data: { unreadCount: 0 } }),
    ]);
    render(<DashboardScreen />);

    expect(screen.queryByText('dashboard.loadError')).toBeNull();
  });

  it('retry button refetches all four queries', () => {
    const refetches = [jest.fn(), jest.fn(), jest.fn(), jest.fn()];
    mockQueries([
      queryResult({ isError: true, refetch: refetches[0] }),
      queryResult({ data: FAULT_STATS, refetch: refetches[1] }),
      queryResult({ data: VOTES, refetch: refetches[2] }),
      queryResult({ data: UNREAD, refetch: refetches[3] }),
    ]);
    render(<DashboardScreen />);

    fireEvent.press(screen.getByText('common.retry'));

    for (const refetch of refetches) {
      expect(refetch).toHaveBeenCalledTimes(1);
    }
  });
});
