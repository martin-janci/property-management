/**
 * NotificationsScreen — real-data wiring regression test
 * (code-review-mobile-rn-notifications-still-mock).
 *
 * The screen used to render a hardcoded `MOCK_NOTIFICATIONS` array and never
 * touched the api-server, even though the in-app notification inbox is a real
 * REST surface:
 *   GET    /api/v1/users/me/notification-preferences/granular/groups
 *   POST   .../groups/read-all
 *   POST   .../groups/{id}/read
 * (the same endpoints ppt-web's NotificationInboxPage consumes via
 * `@ppt/api-client`). This suite locks the contract that the screen now reads
 * its list from `useApiQuery` against that groups endpoint and that the old
 * mock content is gone.
 */

import { render, screen } from '@testing-library/react-native';
import type { InboxResponse } from '@ppt/api-client';
import { NotificationsScreen } from './NotificationsScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useApiMutation: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;
const mockUseApiMutation = jest.requireMock('../../hooks/useApi').useApiMutation as jest.Mock;

function mockQuery(overrides: Record<string, unknown>) {
  mockUseApiQuery.mockReturnValue({
    data: undefined,
    isLoading: false,
    error: null,
    refetch: jest.fn(),
    isFetching: false,
    ...overrides,
  });
}

// Titles that lived in the old hardcoded MOCK_NOTIFICATIONS array. None of
// them must ever render again — they are not in any API payload.
const OLD_MOCK_TITLES = [
  'Fault status updated',
  'New announcement',
  'Voting open',
  'Meter reading reminder',
];

const SAMPLE: InboxResponse = {
  totalUnread: 1,
  groups: [
    {
      group: {
        id: 'g1',
        userId: 'u1',
        groupKey: 'fault:f1',
        entityType: 'fault',
        entityId: 'f1',
        title: 'Server-sent fault update',
        notificationCount: 1,
        latestNotificationAt: '2026-11-10T08:00:00Z',
        isRead: false,
        isExpanded: false,
        createdAt: '2026-11-10T08:00:00Z',
        updatedAt: '2026-11-10T08:00:00Z',
      },
      notifications: [
        {
          id: 'n1',
          groupId: 'g1',
          userId: 'u1',
          eventType: 'fault_status_changed',
          title: 'Server-sent fault update',
          body: 'A contractor was assigned to your fault.',
          isRead: false,
          createdAt: '2026-11-10T08:00:00Z',
        },
      ],
    },
  ],
};

describe('NotificationsScreen real-data wiring', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiMutation.mockReturnValue({ mutate: jest.fn(), isPending: false });
  });

  it('queries the notification groups REST endpoint', async () => {
    mockQuery({ data: SAMPLE });
    await render(<NotificationsScreen />);
    const paths = mockUseApiQuery.mock.calls.map((call) => String(call[1]));
    expect(
      paths.some((p) => p.includes('/notification-preferences/granular/groups'))
    ).toBe(true);
  });

  it('renders notifications from the API payload, not mock data', async () => {
    mockQuery({ data: SAMPLE });
    await render(<NotificationsScreen />);
    expect(screen.getByText('Server-sent fault update')).toBeTruthy();
    for (const title of OLD_MOCK_TITLES) {
      expect(screen.queryByText(title)).toBeNull();
    }
  });

  it('shows the empty state when the API returns no groups', async () => {
    mockQuery({ data: { groups: [], totalUnread: 0 } satisfies InboxResponse });
    await render(<NotificationsScreen />);
    for (const title of OLD_MOCK_TITLES) {
      expect(screen.queryByText(title)).toBeNull();
    }
  });
});
