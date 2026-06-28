/**
 * MessagesScreen — wire-shape regression test (issue #1780).
 *
 * The api-server serializes its messaging response structs
 * (`db::models::messaging::{ThreadWithPreview, ParticipantInfo, MessagePreview}`)
 * with `#[serde(rename_all = "camelCase")]`, so the JSON the screen receives is
 * camelCase. The screen's TS interfaces were previously snake_case, which meant
 * `lastMessage` and `unreadCount` read `undefined` — the UI silently showed the
 * "No messages yet." fallback and no unread badge. The thread title comes from
 * the `participants` ARRAY (`ThreadWithPreview.participants`); there is no
 * singular `otherParticipant` field on the wire.
 *
 * This test feeds a camelCase `ThreadListResponse` fixture (matching the actual
 * wire — a `participants` array, not `otherParticipant`) and asserts the screen
 * reads each nested camelCase field for both a 2-party and a group thread, so
 * frontend.yml now gates the contract.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react-native';
import { MessagesScreen } from './MessagesScreen';

// --- Mocks ---

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

// --- Fixture: a camelCase ThreadListResponse exactly as the api-server emits it ---

const CAMEL_CASE_THREADS = {
  threads: [
    {
      id: 'thread-1',
      organizationId: 'org-7f3c',
      participantIds: ['user-me', 'user-jane'],
      // `participants` is an ARRAY (the api-server emits the full list of other
      // participants — single entry for a 2-party thread, see BIT-206). This is
      // the real wire field; there is no `otherParticipant`.
      participants: [
        {
          id: 'user-jane',
          firstName: 'Jane',
          lastName: 'Doe',
          email: 'jane.doe@example.com',
        },
      ],
      lastMessage: {
        id: 'msg-1',
        content: 'hi',
        senderId: 'user-jane',
        isFromMe: false,
        createdAt: '2026-06-20T10:00:00Z',
      },
      unreadCount: 3,
      createdAt: '2026-06-01T09:00:00Z',
      updatedAt: '2026-06-20T10:00:00Z',
    },
    {
      // Group thread ([BIT-206]): multiple `participants` → "Name +N" title.
      id: 'thread-2',
      organizationId: 'org-7f3c',
      participantIds: ['user-me', 'user-jane', 'user-bob'],
      participants: [
        { id: 'user-jane', firstName: 'Jane', lastName: 'Doe', email: 'jane.doe@example.com' },
        { id: 'user-bob', firstName: 'Bob', lastName: 'Roe', email: 'bob.roe@example.com' },
      ],
      lastMessage: {
        id: 'msg-2',
        content: 'group hello',
        senderId: 'user-bob',
        isFromMe: false,
        createdAt: '2026-06-21T10:00:00Z',
      },
      unreadCount: 0,
      createdAt: '2026-06-02T09:00:00Z',
      updatedAt: '2026-06-21T10:00:00Z',
    },
  ],
  count: 2,
  total: 2,
};

function renderScreen() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <MessagesScreen />
    </QueryClientProvider>
  );
}

// --- Tests ---

describe('MessagesScreen camelCase wire contract (#1780)', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiQuery.mockReturnValue({
      data: CAMEL_CASE_THREADS,
      isLoading: false,
      error: null,
      refetch: jest.fn(),
      isFetching: false,
    });
  });

  it("reads participants[0].firstName/lastName — renders 'Jane Doe'", () => {
    renderScreen();
    // Proves `participants[0].firstName` + `.lastName` are read off the camelCase
    // array. With a wrong/singular field these would be `undefined` (or throw).
    expect(screen.getByText('Jane Doe')).toBeTruthy();
  });

  it("renders a group thread title from the participants array — 'Jane Doe +1'", () => {
    renderScreen();
    // Locks the contract to the real ARRAY shape: a 2-other-participant thread
    // must title as "Name +N", not crash on a missing singular field.
    expect(screen.getByText('Jane Doe +1')).toBeTruthy();
  });

  it("reads lastMessage.content — renders 'hi' and not the empty fallback", () => {
    renderScreen();
    // Proves `lastMessage.content` is read off the camelCase preview object.
    expect(screen.getByText('hi')).toBeTruthy();
    expect(screen.queryByText('No messages yet.')).toBeNull();
  });

  it("reads unreadCount — renders the '3 new' unread badge", () => {
    renderScreen();
    // Proves `unreadCount` is read; with snake_case it was `undefined`, the
    // `> 0` guard was false, and the badge never rendered.
    expect(screen.getByText('3 new')).toBeTruthy();
  });
});
