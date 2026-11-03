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
 *   - The two branches that ran a *reduced* hook set on the buggy code — the
 *     missing-id (empty) branch and the loading branch — render their reduced
 *     UI (empty state / spinner) and NOT the full ballot, so a regression that
 *     reintroduces the pre-hook early return fails here on both the render
 *     (crash) and the branch structure.
 *
 * These branch checks assert the component's stable surface (the empty-state
 * copy, the loading-title fallback, and the *absence* of the ballot controls)
 * rather than a full-tree `toMatchSnapshot()`. The old snapshots embedded the
 * shared `screenStyles` values, so an unrelated edit to that shared module
 * regenerated them with no change to this screen's logic — the same
 * styling-churn trap the ranked-ballot suite below already avoids.
 */

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react-native';
import { Alert } from 'react-native';
import { VoteDetailScreen } from './VoteDetailScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useApiMutation: jest.fn(),
}));

// Override the global `react-i18next` mock (src/test/setup.ts) with a *spyable*
// `t` so the eligibility-reason i18n regression test below can assert the
// server string is routed THROUGH `t(key, { defaultValue })`. The behavior is
// identical to the global mock (`defaultValue ?? key`), so every other test in
// this file — which only asserts the resolved defaultValue text — stays green.
// `mock`-prefixed names are allowed inside a hoisted jest.mock factory.
const mockT = jest.fn((key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key);
jest.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: mockT,
    i18n: { language: 'en', changeLanguage: jest.fn() },
  }),
  initReactI18next: { type: '3rdParty', init: jest.fn() },
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
    // The screen renders the i18n fallback (defaultValue), not the raw key:
    // `t('voting.missingId', { defaultValue: 'No vote selected' })`.
    expect(screen.getByText('No vote selected')).toBeTruthy();
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

describe('VoteDetailScreen — reduced-hook branch render', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiMutation.mockReturnValue(IDLE_MUTATION);
  });

  // The missing-id branch is the exact path whose pre-hook early return caused
  // the Rules-of-Hooks crash. Assert it renders the "no vote selected" empty
  // state and NOT the ballot surface — the structural guarantee the fix must
  // keep — without pinning the shared-style tree in a snapshot.
  it('renders the missing-id (empty) branch: the empty state, not the ballot', () => {
    mockUseApiQuery.mockReturnValue(IDLE_QUERY);

    renderScreen({ voteId: undefined });

    expect(screen.getByText('No vote selected')).toBeOnTheScreen();
    // None of the ballot surface renders on this reduced branch.
    expect(screen.queryByText('Submit ballot')).toBeNull();
    expect(screen.queryByLabelText('Move up')).toBeNull();
  });

  // The loading branch runs the *full* hook set (voteId present) and then the
  // spinner render path — the render that the buggy code could never reach
  // consistently once the missing-id render had skipped hooks. Assert the full
  // hook set ran, the loading-title fallback shows, and the ballot does not.
  it('renders the loading branch: the full hook set and loading title, not the ballot', () => {
    mockUseApiQuery.mockReturnValue(LOADING_QUERY);

    renderScreen({ voteId: 'vote-1' });

    // All three data queries plus the cast mutation still run — the hook set is
    // complete even while the screen shows only the spinner.
    expect(mockUseApiQuery).toHaveBeenCalledTimes(3);
    expect(mockUseApiMutation).toHaveBeenCalledTimes(1);

    // The title falls back to the loading copy while `detail` is still fetching
    // (`detail?.title ?? t('common.loading', { defaultValue: 'Loading…' })`),
    // and the ballot is not rendered mid-load.
    expect(screen.getByText('Loading…')).toBeOnTheScreen();
    expect(screen.queryByText('Submit ballot')).toBeNull();
    expect(screen.queryByLabelText('Move up')).toBeNull();
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
 * resolved "Move up" / "Move down" accessibility labels (the component renders
 * the `t(key, { defaultValue })` fallback, not the raw key), and the cast
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
    // The submit button renders the *resolved* label via
    // `t('voting.submit', { defaultValue: 'Submit ballot' })`, so press the
    // visible English text, not the raw i18n key.
    fireEvent.press(screen.getByText('Submit ballot'));
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

    // One move-up and one move-down control per option — the buggy renderer
    // had none. Assert on the *resolved* accessibility label (the
    // `t(key, { defaultValue })` fallback the component renders), not the raw
    // key — a regression that drops the defaultValue idiom (so VoiceOver /
    // TalkBack announce "voting.moveUp") fails here (#3206).
    expect(screen.getAllByLabelText('Move up')).toHaveLength(OPTION_LABELS.length);
    expect(screen.getAllByLabelText('Move down')).toHaveLength(OPTION_LABELS.length);

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

    // Move the top option (Roof repair) down one place → Garden, Roof repair,
    // Elevator. Use the resolved accessibility label the component renders.
    fireEvent.press(screen.getAllByLabelText('Move down')[0]);
    castBallot();

    expect(castAnswers()).toEqual({ 'q-ranked': ['opt-b', 'opt-a', 'opt-c'] });
  });
});

/**
 * Eligibility-reason i18n regression guard
 * (code-review-mobile-rn-vote-eligibility-reason-i18n).
 *
 * The StatusBanner used to render the server-provided `eligibility.reason`
 * string RAW — `<Text>{eligibilityReason}</Text>` — bypassing the i18n layer
 * entirely, so a non-English voter saw the backend's English sentence
 * ("Vote is not currently accepting ballots", repositories/vote.rs) regardless
 * of their locale. The fix routes it through `t(key, { defaultValue })` where
 * the key is derived from the reason, mirroring how the rest of this screen
 * (submit button, move controls, empty states) resolves strings.
 *
 * Proof that this fails before the fix: the global `t` mock resolves
 * `{ defaultValue }` to the raw string, so the *rendered text* is identical
 * before and after — the only observable difference is whether `t` is invoked
 * at all for the reason. The pre-fix code renders the raw string directly and
 * never calls `t` with the reason key, so the `toHaveBeenCalledWith` assertion
 * fails; the fixed code calls `t('voting.eligibilityReason.<code>', …)` and
 * passes. The raw string is still shown (via the defaultValue fallback) so an
 * unmapped reason or untranslated locale degrades gracefully.
 */
describe('VoteDetailScreen — eligibility reason i18n', () => {
  const VOTE_ID = 'vote-1';

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
    question_count: 0,
  };

  // can_vote=false with a reason is exactly the branch that rendered the raw
  // server string. One not-yet-voted unit keeps `alreadyVoted` false so the
  // reason branch (not the "you have voted" branch) is the one exercised.
  function eligibility(reason: string | null) {
    return {
      vote_id: VOTE_ID,
      user_id: 'user-1',
      eligible_units: [
        {
          unit_id: 'unit-1',
          unit_designation: '1A',
          ownership_share: '1/1',
          is_owner: true,
          is_delegated: false,
          delegation_id: null,
          already_voted: false,
        },
      ],
      can_vote: false,
      reason,
    };
  }

  function mockQueries(reason: string | null) {
    mockUseApiQuery.mockImplementation((_key: unknown, url: string) => {
      if (url.endsWith('/questions')) {
        return { data: [], isLoading: false, error: null };
      }
      if (url.endsWith('/eligibility')) {
        return { data: eligibility(reason), isLoading: false, error: null };
      }
      return { data: DETAIL, isLoading: false, error: null };
    });
  }

  beforeEach(() => {
    jest.clearAllMocks();
    mockT.mockImplementation(
      (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key
    );
    mockUseApiMutation.mockReturnValue(IDLE_MUTATION);
  });

  it('routes a known server reason through t() with a derived key and the raw string as defaultValue', () => {
    mockQueries('Vote is not currently accepting ballots');

    renderScreen({ voteId: VOTE_ID });

    // The load-bearing assertion: the reason must go through the i18n layer.
    // Pre-fix (`<Text>{eligibilityReason}</Text>`) never calls t with this key.
    expect(mockT).toHaveBeenCalledWith(
      'voting.eligibilityReason.voteNotAcceptingBallots',
      expect.objectContaining({ defaultValue: 'Vote is not currently accepting ballots' })
    );

    // And the raw sentence is NOT the only thing rendered by its own — it is the
    // defaultValue fallback, so it still shows (graceful degradation) but now via t.
    expect(screen.getByText('Vote is not currently accepting ballots')).toBeTruthy();
  });

  it('derives a slug key for an unmapped reason, still falling back to the raw string', () => {
    mockQueries('Some brand new backend reason');

    renderScreen({ voteId: VOTE_ID });

    expect(mockT).toHaveBeenCalledWith(
      'voting.eligibilityReason.some-brand-new-backend-reason',
      expect.objectContaining({ defaultValue: 'Some brand new backend reason' })
    );
    expect(screen.getByText('Some brand new backend reason')).toBeTruthy();
  });
});
