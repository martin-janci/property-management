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
import { fireEvent, render, screen } from '@testing-library/react-native';
import { Alert } from 'react-native';
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

/**
 * Ranked-choice ballot rendering (UC-04 / Epic 5).
 *
 * `ranked` questions get their own reorderable renderer (RankedQuestionCard):
 * every option shows its 1-based rank position with move-up / move-down
 * controls, the declared option order is the default ballot, and casting
 * always submits the full ordered option-id list (the backend weights answers
 * by array position — repositories/vote.rs).
 *
 * These tests bind only to the component's stable surface — option text, the
 * `voting.moveUp` / `voting.moveDown` accessibility labels, and the cast
 * payload — so they survive styling / layout churn. The checkbox renderer this
 * path replaced exposed none of those controls and submitted an order-less set.
 */
describe('VoteDetailScreen — ranked ballot', () => {
  const VOTE_ID = 'vote-1';

  const RANKED_QUESTION = {
    id: 'q-ranked',
    vote_id: VOTE_ID,
    question_text: 'Rank these priorities',
    description: null,
    question_type: 'ranked',
    // Declared order === default ballot: Roof repair, Garden, Elevator.
    options: [
      { id: 'opt-a', text: 'Roof repair', order: 0 },
      { id: 'opt-b', text: 'Garden', order: 1 },
      { id: 'opt-c', text: 'Elevator', order: 2 },
    ],
    display_order: 0,
    is_required: true,
  };
  const OPTION_LABELS = RANKED_QUESTION.options.map((o) => o.text);
  const DECLARED_ORDER = RANKED_QUESTION.options.map((o) => o.id);

  const DETAIL = {
    id: VOTE_ID,
    title: 'Budget priorities',
    description: null,
    status: 'active',
    start_at: null,
    end_at: '2026-12-31T00:00:00Z',
    quorum_type: 'simple',
    quorum_percentage: null,
    participation_count: null,
    eligible_count: null,
    building_name: 'Maple Court',
    created_by_name: 'Manager',
    question_count: 1,
  };

  const ELIGIBILITY = {
    vote_id: VOTE_ID,
    user_id: 'user-1',
    eligible_units: [
      {
        unit_id: 'unit-1',
        unit_designation: '1A',
        ownership_share: '1/3',
        is_owner: true,
        is_delegated: false,
        delegation_id: null,
        already_voted: false,
      },
    ],
    can_vote: true,
    reason: null,
  };

  const mutate = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiMutation.mockReturnValue({ mutate, isPending: false });
    // Route each query by its URL suffix, so the order the screen issues them
    // in is irrelevant to the test.
    mockUseApiQuery.mockImplementation((_key: unknown, url: string) => {
      if (url.endsWith('/questions')) {
        return { data: [RANKED_QUESTION], isLoading: false, error: null };
      }
      if (url.endsWith('/eligibility')) {
        return { data: ELIGIBILITY, isLoading: false, error: null };
      }
      return { data: DETAIL, isLoading: false, error: null };
    });
    jest.spyOn(Alert, 'alert').mockImplementation(() => undefined);
  });

  afterEach(() => {
    (Alert.alert as jest.Mock).mockRestore?.();
  });

  // Press "submit", then fire the confirm (`style: 'default'`) button of the
  // confirmation Alert so the cast actually runs. Throws a clear error if submit
  // never reached a confirmation dialog, so a regression that drops the confirm
  // step fails loudly instead of silently skipping the cast assertions.
  function castBallot() {
    fireEvent.press(screen.getByText('voting.submit'));
    const withButtons = [...(Alert.alert as jest.Mock).mock.calls]
      .reverse()
      .find((call) => Array.isArray(call[2]));
    if (!withButtons) {
      throw new Error('submit did not open a confirmation dialog with buttons');
    }
    const confirm = withButtons[2].find((b: { style?: string }) => b.style === 'default');
    if (!confirm) {
      throw new Error('confirmation dialog had no default (confirm) button');
    }
    confirm.onPress?.();
  }

  // The answers map of the single cast the screen performed.
  function castAnswers(): Record<string, string | string[]> {
    expect(mutate).toHaveBeenCalledTimes(1);
    return mutate.mock.calls[0][0].answers;
  }

  it('shows every option with a reorder control and a 1-based rank position', () => {
    renderScreen({ voteId: VOTE_ID });

    for (const label of OPTION_LABELS) {
      expect(screen.getByText(label)).toBeOnTheScreen();
    }

    // One move-up and one move-down control per option, found by the stable
    // accessibility labels the component exposes.
    expect(screen.getAllByLabelText('voting.moveUp')).toHaveLength(OPTION_LABELS.length);
    expect(screen.getAllByLabelText('voting.moveDown')).toHaveLength(OPTION_LABELS.length);

    // Rank badges read 1..N in row order (bare-digit text nodes). Collecting
    // them is robust to stray copy elsewhere on screen, unlike a global
    // getByText('1'), and pins the declared ordering.
    const positions = screen.getAllByText(/^\d+$/).map((node) => String(node.props.children));
    expect(positions).toEqual(['1', '2', '3']);
  });

  it('submits the full declared option order when the voter does not reorder', () => {
    renderScreen({ voteId: VOTE_ID });

    castBallot();

    expect(mutate).toHaveBeenCalledWith(
      expect.objectContaining({ unit_id: 'unit-1', answers: { 'q-ranked': DECLARED_ORDER } }),
      expect.anything()
    );
  });

  it('captures the reordered preference list in the cast answers', () => {
    renderScreen({ voteId: VOTE_ID });

    // Move the top option (Roof repair) down one place → Garden, Roof repair, Elevator.
    fireEvent.press(screen.getAllByLabelText('voting.moveDown')[0]);
    castBallot();

    expect(castAnswers()).toEqual({ 'q-ranked': ['opt-b', 'opt-a', 'opt-c'] });
  });
});
