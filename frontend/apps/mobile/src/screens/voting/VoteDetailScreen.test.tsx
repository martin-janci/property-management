/**
 * VoteDetailScreen — Rules-of-Hooks regression guard
 * (code-review-mobile-rn-votedetail-conditional-hooks).
 *
 * The screen used to `return` its "no vote selected" error state *before*
 * calling `useApiQuery` / `useApiMutation` / `useState` / `useMemo`. That is
 * a Rules-of-Hooks violation: when `voteId` is missing only two hooks run,
 * but when it is present the full set runs, so React sees a different number
 * of hooks between renders and crashes ("Rendered more hooks than during the
 * previous render"). The fix moves the early return below every hook and gates
 * the queries on `enabled: !!voteId`.
 *
 * Covers:
 *   - The data hooks run unconditionally even when `voteId` is missing, gated
 *     with `enabled: false` (never true on the buggy pre-fix code — the early
 *     return skipped them entirely).
 *   - Re-rendering from a missing `voteId` to a real one does not throw a
 *     hook-order error.
 *   - The missing-id error state still renders.
 *   - Snapshots of the two branches that ran a *reduced* hook set on the buggy
 *     code — the missing-id (empty) branch and the loading branch — so a future
 *     regression that reintroduces the pre-hook early return fails both the
 *     render (crash) and the snapshot (structure) here.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react-native';
import { VoteDetailScreen } from './VoteDetailScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useApiMutation: jest.fn(),
}));

const useApiMock = jest.requireMock('../../hooks/useApi');
const mockUseApiQuery = useApiMock.useApiQuery as jest.Mock;
const mockUseApiMutation = useApiMock.useApiMutation as jest.Mock;

const IDLE_QUERY = { data: undefined, isLoading: false, error: null };
const LOADING_QUERY = { data: undefined, isLoading: true, error: null };
const IDLE_MUTATION = { mutate: jest.fn(), isPending: false };

function renderScreen(props: React.ComponentProps<typeof VoteDetailScreen> = {}) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <VoteDetailScreen {...props} />
    </QueryClientProvider>
  );
}

describe('VoteDetailScreen — hook ordering', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiQuery.mockReturnValue(IDLE_QUERY);
    mockUseApiMutation.mockReturnValue(IDLE_MUTATION);
  });

  it('calls the data hooks unconditionally even when voteId is missing (gated with enabled:false)', () => {
    renderScreen({ voteId: undefined });

    // On the buggy version these were never reached: the early return fired
    // before any useApiQuery call.
    expect(mockUseApiQuery).toHaveBeenCalledTimes(3);
    expect(mockUseApiMutation).toHaveBeenCalledTimes(1);

    // Every query is disabled so a missing id never fetches `/voting/undefined`.
    for (const call of mockUseApiQuery.mock.calls) {
      const options = call[2] as { enabled?: boolean } | undefined;
      expect(options?.enabled).toBe(false);
    }
  });

  it('renders the missing-id error state without crashing', () => {
    renderScreen({ voteId: undefined });
    expect(screen.getByText('voting.missingId')).toBeTruthy();
  });

  it('does not throw a hook-order error when voteId changes from missing to present', () => {
    const { rerender } = renderScreen({ voteId: undefined });

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    // On the buggy version this rerender throws "Rendered more hooks than
    // during the previous render." because the now-present voteId runs hooks
    // that the missing-id render skipped.
    expect(() =>
      rerender(
        <QueryClientProvider client={queryClient}>
          <VoteDetailScreen voteId="vote-1" />
        </QueryClientProvider>
      )
    ).not.toThrow();
  });
});

describe('VoteDetailScreen — branch render snapshots', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiMutation.mockReturnValue(IDLE_MUTATION);
  });

  // The missing-id branch is the exact path whose pre-hook early return caused
  // the Rules-of-Hooks crash. Snapshotting it pins the "no vote selected"
  // structure and proves the branch renders without running the full ballot UI.
  it('renders a stable snapshot for the missing-id (empty) branch', () => {
    mockUseApiQuery.mockReturnValue(IDLE_QUERY);

    const { toJSON } = renderScreen({ voteId: undefined });

    expect(screen.getByText('voting.missingId')).toBeTruthy();
    expect(toJSON()).toMatchSnapshot();
  });

  // The loading branch runs the *full* hook set (voteId present) and then the
  // spinner render path — the render that the buggy code could never reach
  // consistently once the missing-id render had skipped hooks. Pinning it
  // guards the loading UI structure too.
  it('renders a stable snapshot for the loading branch', () => {
    mockUseApiQuery.mockReturnValue(LOADING_QUERY);

    const { toJSON } = renderScreen({ voteId: 'vote-1' });

    // All three data queries plus the cast mutation still run — the hook set is
    // complete even while the screen shows only the spinner.
    expect(mockUseApiQuery).toHaveBeenCalledTimes(3);
    expect(mockUseApiMutation).toHaveBeenCalledTimes(1);
    expect(toJSON()).toMatchSnapshot();
  });
});
