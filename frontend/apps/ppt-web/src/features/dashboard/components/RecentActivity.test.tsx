/// <reference types="vitest/globals" />
/**
 * RecentActivity — the resident dashboard feed must show only real data
 * (published announcements + the user's own fault reports), never invented
 * placeholder rows, and fall back to an explicit empty state.
 *
 * The fault half is sourced from `GET /api/v1/faults/my` (via `useMyFaults`),
 * which filters by reporter server-side before applying the limit. See #3315:
 * the previous implementation fetched the 20 newest building-wide faults and
 * filtered by `reporter_id` on the client, so a user's own faults vanished
 * from the feed whenever they ranked below 20 newer third-party faults.
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import en from '../../../../messages/en.json';
import { formatRelativeTime, RecentActivity } from './RecentActivity';

const mockUseAnnouncements = vi.fn();
const mockUseMyFaults = vi.fn();
vi.mock('@ppt/api-client', () => ({
  useMyFaults: (...args: unknown[]) => mockUseMyFaults(...args),
}));
vi.mock('../hooks/useResidentAnnouncements', () => ({
  useResidentAnnouncements: (...args: unknown[]) => mockUseAnnouncements(...args),
}));

function query<T>(data: T, overrides: Record<string, unknown> = {}) {
  return { data, isLoading: false, isError: false, ...overrides };
}

function renderFeed() {
  return render(
    <MemoryRouter>
      <RecentActivity />
    </MemoryRouter>
  );
}

const A = en.dashboard.activity;

beforeEach(() => {
  vi.clearAllMocks();
});

describe('RecentActivity', () => {
  it('lists published announcements and the faults the user reported, newest first', () => {
    mockUseAnnouncements.mockReturnValue(
      query({
        items: [{ id: 'an-1', title: 'Water outage', publishedAt: '2026-01-02T10:00:00Z' }],
      })
    );
    // `/faults/my` already scopes to the signed-in reporter — the component
    // renders every row it returns, with no client-side reporter filter.
    mockUseMyFaults.mockReturnValue(
      query({
        faults: [
          {
            id: 'f-1',
            title: 'Broken lift',
            reporter_id: 'user-1',
            created_at: '2026-01-03T10:00:00Z',
          },
        ],
        count: 1,
      })
    );

    renderFeed();

    // Scopes the request to the user's own faults rather than an org-wide list.
    expect(mockUseMyFaults).toHaveBeenCalledWith({ limit: 5 });

    const links = screen.getAllByRole('link');
    expect(links).toHaveLength(2);
    expect(links[0]).toHaveAttribute('href', '/faults/f-1');
    expect(links[0]).toHaveTextContent('You reported a fault: Broken lift');
    expect(links[1]).toHaveAttribute('href', '/announcements/an-1');
    expect(links[1]).toHaveTextContent('New announcement: Water outage');
  });

  it('shows the user’s own fault even when it is far from the newest building-wide fault (#3315)', () => {
    mockUseAnnouncements.mockReturnValue(query({ items: [] }));
    // The server returns the user's own faults regardless of where they rank
    // among the building's newest faults. A month-old self-reported fault that
    // the old client-side filter over a capped 20-row org-wide fetch would have
    // dropped must still surface here.
    mockUseMyFaults.mockReturnValue(
      query({
        faults: [
          {
            id: 'f-old',
            title: 'Leaking radiator',
            reporter_id: 'user-1',
            created_at: '2025-12-01T10:00:00Z',
          },
        ],
        count: 1,
      })
    );

    renderFeed();

    const link = screen.getByRole('link');
    expect(link).toHaveAttribute('href', '/faults/f-old');
    expect(link).toHaveTextContent('You reported a fault: Leaking radiator');
  });

  it('shows an empty state instead of placeholder activity when there is no data', () => {
    mockUseAnnouncements.mockReturnValue(query({ items: [] }));
    mockUseMyFaults.mockReturnValue(query({ faults: [], count: 0 }));

    renderFeed();

    expect(screen.getByText(A.empty)).toBeInTheDocument();
    expect(screen.queryByRole('link')).not.toBeInTheDocument();
  });

  it('shows an error message when both sources fail', () => {
    mockUseAnnouncements.mockReturnValue(query(undefined, { isError: true }));
    mockUseMyFaults.mockReturnValue(query(undefined, { isError: true }));

    renderFeed();

    expect(screen.getByRole('alert')).toHaveTextContent(A.loadFailed);
  });
});

describe('formatRelativeTime', () => {
  const now = Date.parse('2026-01-10T12:00:00Z');

  it('formats in the requested language', () => {
    expect(formatRelativeTime('2026-01-10T10:00:00Z', 'en', now)).toBe('2 hours ago');
    expect(formatRelativeTime('2026-01-10T10:00:00Z', 'sk', now)).toBe('pred 2 hodinami');
  });
});
