/**
 * MessagesScreen — wire-shape regression test (issue #1780).
 *
 * The api-server serializes its messaging response structs
 * (`db::models::messaging::{ThreadWithPreview, ParticipantInfo, MessagePreview}`)
 * with `#[serde(rename_all = "camelCase")]`, so the JSON the screen receives is
 * camelCase. The screen's TS interfaces were previously snake_case, which meant
 * `otherParticipant`, `lastMessage`, and `unreadCount` all read `undefined` —
 * the UI silently showed empty names, the "No messages yet." fallback, and no
 * unread badge against a perfectly valid response.
 *
 * This test feeds a camelCase `ThreadListResponse` fixture (matching the actual
 * wire) and asserts the screen reads each nested camelCase field. It PASSES on
 * the camelCase code shipped by #1780 and would FAIL on the pre-fix snake_case
 * interfaces (the assertions below would hit `undefined`), so frontend.yml now
 * gates the contract.
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
      otherParticipant: {
        id: 'user-jane',
        firstName: 'Jane',
        lastName: 'Doe',
        email: 'jane.doe@example.com',
      },
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
  ],
  count: 1,
  total: 1,
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

  it("reads otherParticipant.firstName/lastName — renders 'Jane Doe'", () => {
    renderScreen();
    // Proves `otherParticipant.firstName` + `.lastName` are read. With the old
    // snake_case interface these were `undefined` and the name fell back to email.
    expect(screen.getByText('Jane Doe')).toBeTruthy();
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
