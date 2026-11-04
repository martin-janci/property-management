/**
 * AnnouncementsPage state-wiring regression test (gap-79-1).
 * Verifies the loading / error / empty triad threaded from the route's
 * TanStack Query hook (isLoading / isError / onRetry) reaches the UI.
 */

/// <reference types="vitest/globals" />
import { fireEvent, render, screen } from '@testing-library/react';
import { AnnouncementsPage } from './AnnouncementsPage';

const noop = () => {};

const baseProps = {
  announcements: [],
  total: 0,
  onNavigateToCreate: noop,
  onNavigateToView: noop,
  onNavigateToEdit: noop,
  onDelete: noop,
  onPublish: noop,
  onArchive: noop,
  onPin: noop,
  onFilterChange: noop,
};

describe('AnnouncementsPage state wiring', () => {
  it('renders the empty state when not loading and no error', () => {
    render(<AnnouncementsPage {...baseProps} isLoading={false} />);
    expect(screen.getByText('No announcements found.')).toBeInTheDocument();
  });

  it('renders an inline error state (not the empty state) when isError is true', () => {
    render(<AnnouncementsPage {...baseProps} isError />);
    expect(screen.getByRole('alert')).toHaveTextContent('Failed to load announcements');
    expect(screen.queryByText('No announcements found.')).not.toBeInTheDocument();
  });

  it('invokes onRetry from the error state retry button', () => {
    const onRetry = vi.fn();
    render(<AnnouncementsPage {...baseProps} isError onRetry={onRetry} />);
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('does not show the empty state while loading', () => {
    render(<AnnouncementsPage {...baseProps} isLoading />);
    expect(screen.queryByText('No announcements found.')).not.toBeInTheDocument();
  });
  it('renders a read-only list for residents (canManage=false)', () => {
    render(
      <AnnouncementsPage
        {...baseProps}
        canManage={false}
        total={1}
        announcements={[
          {
            id: 'a1',
            title: 'Water outage',
            status: 'published',
            targetType: 'building',
            publishedAt: '2026-04-10T08:00:00Z',
            pinned: true,
            commentsEnabled: false,
            acknowledgmentRequired: false,
          },
        ]}
      />
    );
    // Title appears in the list (and in the pinned band when provided).
    expect(screen.getAllByText('Water outage').length).toBeGreaterThan(0);
    expect(screen.queryAllByRole('combobox')).toHaveLength(0);
    for (const name of ['Create Announcement', 'Archive', 'Unpin', 'Edit', 'Delete']) {
      expect(screen.queryByRole('button', { name })).not.toBeInTheDocument();
    }
    expect(screen.getByRole('button', { name: 'View' })).toBeInTheDocument();
  });

  it('hides the "create first" call-to-action from residents', () => {
    render(<AnnouncementsPage {...baseProps} canManage={false} isLoading={false} />);
    expect(screen.getByText('No announcements found.')).toBeInTheDocument();
    expect(screen.queryAllByRole('button')).toHaveLength(0);
  });
});
