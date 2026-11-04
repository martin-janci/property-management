/// <reference types="vitest/globals" />
/**
 * RecentActivity — the resident dashboard feed must show only real data
 * (published announcements + the user's own fault reports), never invented
 * placeholder rows, and fall back to an explicit empty state.
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import en from '../../../../messages/en.json';
import { formatRelativeTime, RecentActivity } from './RecentActivity';

const mockUseAnnouncements = vi.fn();
const mockUseFaults = vi.fn();
vi.mock('@ppt/api-client', () => ({
  useAnnouncements: (...args: unknown[]) => mockUseAnnouncements(...args),
  useFaults: (...args: unknown[]) => mockUseFaults(...args),
}));

vi.mock('../../../contexts', () => ({
  useAuth: () => ({ user: { id: 'user-1', email: 'resident@example.com' } }),
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
  it('lists published announcements and only the faults the user reported, newest first', () => {
    mockUseAnnouncements.mockReturnValue(
      query({
        items: [{ id: 'an-1', title: 'Water outage', publishedAt: '2026-01-02T10:00:00Z' }],
      })
    );
    mockUseFaults.mockReturnValue(
      query({
        faults: [
          {
            id: 'f-1',
            title: 'Broken lift',
            reporter_id: 'user-1',
            created_at: '2026-01-03T10:00:00Z',
          },
          {
            id: 'f-2',
            title: 'Someone else',
            reporter_id: 'user-2',
            created_at: '2026-01-04T10:00:00Z',
          },
        ],
        count: 2,
      })
    );

    renderFeed();

    const links = screen.getAllByRole('link');
    expect(links).toHaveLength(2);
    expect(links[0]).toHaveAttribute('href', '/faults/f-1');
    expect(links[0]).toHaveTextContent('You reported a fault: Broken lift');
    expect(links[1]).toHaveAttribute('href', '/announcements/an-1');
    expect(links[1]).toHaveTextContent('New announcement: Water outage');
    expect(screen.queryByText(/Someone else/)).not.toBeInTheDocument();
  });

  it('shows an empty state instead of placeholder activity when there is no data', () => {
    mockUseAnnouncements.mockReturnValue(query({ items: [] }));
    mockUseFaults.mockReturnValue(query({ faults: [], count: 0 }));

    renderFeed();

    expect(screen.getByText(A.empty)).toBeInTheDocument();
    expect(screen.queryByRole('link')).not.toBeInTheDocument();
  });

  it('shows an error message when both sources fail', () => {
    mockUseAnnouncements.mockReturnValue(query(undefined, { isError: true }));
    mockUseFaults.mockReturnValue(query(undefined, { isError: true }));

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
