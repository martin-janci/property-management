/**
 * ThreadDetailScreen — send-failure regression test.
 *
 * Guards the code-review finding (code-review-mobile-rn-thread-send-silent-fail):
 * the send-message mutation used to have `onSuccess` only, so a failed POST was
 * swallowed — the Send button reset and the drafted text vanished with no error
 * shown. This suite asserts that a failed send:
 *   1. surfaces the failure inline (error banner + retry affordance), and
 *   2. does NOT silently discard the drafted message (it stays in the composer).
 *
 * The pure-helper tests (parse/title/toUiMessages) live in the sibling
 * `ThreadDetailScreen.test.ts`; this file renders the component.
 *
 * The `useApiMutation` mock is a synchronous, state-driven stand-in: `mutate`
 * flips real React state (so the `isError` banner re-renders) and invokes the
 * caller's `onSuccess` / `onError` — exactly the contract the screen relies on,
 * without the flakiness of a real async TanStack mutation under jest-expo.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react-native';
import { ThreadDetailScreen } from './ThreadDetailScreen';

const mockMutationFn = jest.fn();
// Toggled per test to make the next `mutate` succeed or fail.
let mockSendShouldFail = false;
const SEND_ERROR = 'Network unreachable';

jest.mock('../../hooks/useApi', () => {
  const React = require('react');
  return {
    useApiQuery: jest.fn(),
    useApiMutation: () => {
      const [state, setState] = React.useState({
        isPending: false,
        isError: false,
        error: undefined as Error | undefined,
      });
      const mutate = (
        vars: unknown,
        opts?: { onSuccess?: () => void; onError?: (e: Error) => void }
      ) => {
        mockMutationFn(vars);
        if (mockSendShouldFail) {
          const err = new Error(SEND_ERROR);
          setState({ isPending: false, isError: true, error: err });
          opts?.onError?.(err);
        } else {
          setState({ isPending: false, isError: false, error: undefined });
          opts?.onSuccess?.();
        }
      };
      return { mutate, ...state };
    },
  };
});

jest.mock('../../contexts/AuthContext', () => ({
  useAuth: () => ({ user: { id: 'u-1' } }),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

function renderScreen() {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={queryClient}>
      <ThreadDetailScreen threadId="thread-1" />
    </QueryClientProvider>
  );
}

describe('ThreadDetailScreen — send outcomes', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockSendShouldFail = false;
    // An empty, loaded thread so the composer renders.
    mockUseApiQuery.mockReturnValue({
      data: { participants: [], messages: [] },
      isLoading: false,
      error: null,
      refetch: jest.fn(),
      isFetching: false,
    });
  });

  it('surfaces an error and keeps the draft when the send mutation rejects', () => {
    mockSendShouldFail = true;
    renderScreen();

    fireEvent.changeText(
      screen.getByPlaceholderText('Write a message…'),
      'Please fix the elevator'
    );
    fireEvent.press(screen.getByText('Send'));

    // The send was actually attempted (not a silent no-op).
    expect(mockMutationFn).toHaveBeenCalledTimes(1);
    expect(mockMutationFn).toHaveBeenCalledWith({ content: 'Please fix the elevator' });

    // The failure is surfaced inline (reused QueryErrorBanner) with a retry.
    // The banner shows a user-facing message, never the raw backend error.
    expect(screen.getByText(/Couldn't send/)).toBeTruthy();
    expect(screen.queryByText(new RegExp(SEND_ERROR))).toBeNull();
    expect(screen.getByText('common.retry')).toBeTruthy(); // i18n key in tests

    // The drafted message is NOT silently discarded — it stays in the composer
    // so the user can retry without retyping.
    expect(screen.getByDisplayValue('Please fix the elevator')).toBeTruthy();
  });

  it('clears the draft and shows no error banner on a successful send', () => {
    renderScreen();

    fireEvent.changeText(screen.getByPlaceholderText('Write a message…'), 'Thanks!');
    fireEvent.press(screen.getByText('Send'));

    expect(mockMutationFn).toHaveBeenCalledTimes(1);
    expect(screen.queryByDisplayValue('Thanks!')).toBeNull();
    expect(screen.queryByText(/Couldn't send/)).toBeNull();
  });
});
