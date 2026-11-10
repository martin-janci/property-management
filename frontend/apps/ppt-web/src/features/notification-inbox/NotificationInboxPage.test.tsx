/// <reference types="vitest/globals" />
/**
 * In-app notification inbox (gap audit R6 P1-8): lists groups, marks read,
 * removes, and toggles read items.
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string; count?: number }) =>
      (opts?.defaultValue ?? key).replace('{{count}}', String(opts?.count ?? '')),
    i18n: { language: 'en' },
  }),
}));

const state = vi.hoisted(() => ({
  query: undefined as unknown,
  markRead: vi.fn(),
  markAll: vi.fn(),
  remove: vi.fn(),
  data: undefined as unknown,
}));

vi.mock('@ppt/api-client', () => ({
  useNotificationInbox: (query: unknown) => {
    state.query = query;
    return { data: state.data, isLoading: false, error: null, refetch: vi.fn() };
  },
  useMarkInboxGroupRead: () => ({ mutate: state.markRead, isPending: false }),
  useMarkAllInboxRead: () => ({ mutate: state.markAll, isPending: false }),
  useDeleteInboxGroup: () => ({ mutate: state.remove, isPending: false }),
}));

import { NotificationInboxPage } from './NotificationInboxPage';

function group(id: string, isRead: boolean) {
  return {
    group: {
      id,
      userId: 'u',
      groupKey: id,
      entityType: 'notification',
      entityId: id,
      title: `Group ${id}`,
      notificationCount: 1,
      latestNotificationAt: '2026-11-09T10:00:00Z',
      isRead,
      isExpanded: false,
      createdAt: '2026-11-09T10:00:00Z',
      updatedAt: '2026-11-09T10:00:00Z',
    },
    notifications: [
      {
        id: `${id}-n`,
        groupId: id,
        userId: 'u',
        eventType: 'fault.created',
        title: `Group ${id}`,
        body: `Body ${id}`,
        isRead,
        createdAt: '2026-11-09T10:00:00Z',
      },
    ],
  };
}

beforeEach(() => {
  state.markRead.mockReset();
  state.markAll.mockReset();
  state.remove.mockReset();
});

it('shows groups and marks one read', async () => {
  state.data = { groups: [group('a', false), group('b', true)], totalUnread: 1 };
  render(<NotificationInboxPage />);

  expect(screen.getByText('Group a')).toBeInTheDocument();
  expect(screen.getByText('Body b')).toBeInTheDocument();
  // Only the unread group offers "Mark as read".
  const markButtons = screen.getAllByRole('button', { name: 'Mark as read' });
  expect(markButtons).toHaveLength(1);
  await userEvent.click(markButtons[0]);
  expect(state.markRead).toHaveBeenCalledWith('a');

  await userEvent.click(screen.getByRole('button', { name: 'Mark all as read' }));
  expect(state.markAll).toHaveBeenCalled();

  await userEvent.click(screen.getAllByRole('button', { name: 'Remove' })[1]);
  expect(state.remove).toHaveBeenCalledWith('b');
});

it('shows the caught-up state and requests read items on toggle', async () => {
  state.data = { groups: [], totalUnread: 0 };
  render(<NotificationInboxPage />);
  expect(screen.getByText('You are all caught up.')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Mark all as read' })).toBeDisabled();

  await userEvent.click(screen.getByRole('checkbox'));
  expect(state.query).toEqual(expect.objectContaining({ includeRead: true }));
});
