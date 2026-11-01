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
 * Ranked-choice ballot regression
 * (code-review-mobile-rn-vote-detail-ranked-no-order).
 *
 * The screen used to group `ranked` questions with `multiple_choice` into one
 * `isMulti` checkbox renderer (QuestionCard). That meant a ranked ballot showed
 * no rank position and captured nothing until the voter tapped an option, so a
 * ranked question submitted a bare, order-less option set (or nothing at all)
 * and could not express preference order — even though the backend weights
 * ranked answers by array position (repositories/vote.rs).
 *
 * The fix gives `ranked` its own reorderable renderer that:
 *   - shows every option with its 1-based rank position,
 *   - offers move-up / move-down controls to reorder,
 *   - defaults to the declared option order, and
 *   - always submits the full ordered option-id list.
 *
 * On the buggy (pre-fix) code every assertion below fails: there are no
 * move controls, and casting submits an empty answer set for the untouched
 * ranked question.
 */
describe('VoteDetailScreen — ranked ballot', () => {
  const DETAIL = {
    id: 'vote-1',
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

  const RANKED_QUESTION = {
    id: 'q-ranked',
    vote_id: 'vote-1',
    question_text: 'Rank these priorities',
    description: null,
    question_type: 'ranked',
    options: [
      { id: 'opt-a', text: 'Roof repair', order: 0 },
      { id: 'opt-b', text: 'Garden', order: 1 },
      { id: 'opt-c', text: 'Elevator', order: 2 },
    ],
    display_order: 0,
    is_required: true,
  };

  const ELIGIBILITY = {
    vote_id: 'vote-1',
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
    mockUseApiQuery.mockImplementation((_key: unknown, url: string) => {
      if (url.endsWith('/questions')) {
        return { data: [RANKED_QUESTION], isLoading: false, error: null };
      }
      if (url.endsWith('/eligibility')) {
        return { data: ELIGIBILITY, isLoading: false, error: null };
      }
      return { data: DETAIL, isLoading: false, error: null };
    });
    jest.spyOn(Alert, 'alert').mockImplementation(() => {});
  });

  afterEach(() => {
    (Alert.alert as jest.Mock).mockRestore?.();
  });

  // Fire the "default"-styled button of the most recent Alert.alert with
  // buttons — that's the vote-confirmation action that triggers the cast.
  function confirmSubmit() {
    const calls = (Alert.alert as jest.Mock).mock.calls;
    for (let i = calls.length - 1; i >= 0; i--) {
      const buttons = calls[i][2];
      if (Array.isArray(buttons)) {
        const confirm = buttons.find((b: { style?: string }) => b.style === 'default');
        confirm?.onPress?.();
        return;
      }
    }
    throw new Error('no Alert with buttons was shown — submit did not reach confirmation');
  }

  it('renders each ranked option with its position and reorder controls', () => {
    renderScreen({ voteId: 'vote-1' });

    expect(screen.getByText('Roof repair')).toBeTruthy();
    expect(screen.getByText('Garden')).toBeTruthy();
    expect(screen.getByText('Elevator')).toBeTruthy();

    // Rank badges 1/2/3 — absent on the buggy checkbox renderer.
    expect(screen.getByText('1')).toBeTruthy();
    expect(screen.getByText('2')).toBeTruthy();
    expect(screen.getByText('3')).toBeTruthy();

    // One move-up and one move-down control per option — the buggy renderer
    // had none, so these queries throw there. Assert on the *resolved*
    // accessibility label (the `t(key, { defaultValue })` fallback), not the
    // raw key — a regression that drops the defaultValue idiom (so VoiceOver /
    // TalkBack announce "voting.moveUp") fails here (#3206).
    expect(screen.getAllByLabelText('Move up')).toHaveLength(3);
    expect(screen.getAllByLabelText('Move down')).toHaveLength(3);
  });

  it('submits the full declared option order when the voter does not reorder', () => {
    renderScreen({ voteId: 'vote-1' });

    fireEvent.press(screen.getByText('Submit ballot'));
    confirmSubmit();

    expect(mutate).toHaveBeenCalledTimes(1);
    expect(mutate).toHaveBeenCalledWith(
      expect.objectContaining({
        unit_id: 'unit-1',
        answers: { 'q-ranked': ['opt-a', 'opt-b', 'opt-c'] },
      }),
      expect.anything()
    );
  });

  it('captures the reordered preference list in the cast answers', () => {
    renderScreen({ voteId: 'vote-1' });

    // Push the top option (Roof repair) down one position → B, A, C.
    fireEvent.press(screen.getAllByLabelText('Move down')[0]);

    fireEvent.press(screen.getByText('Submit ballot'));
    confirmSubmit();

    expect(mutate).toHaveBeenCalledWith(
      expect.objectContaining({
        answers: { 'q-ranked': ['opt-b', 'opt-a', 'opt-c'] },
      }),
      expect.anything()
    );
  });
});
