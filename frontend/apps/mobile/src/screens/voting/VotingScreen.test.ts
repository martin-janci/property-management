import type { ApiVoteSummary } from './VotingScreen';
import {
  formatVoteDate,
  parseVoteSummaries,
  quorumProgressPercent,
  toUiStatus,
  toUiVote,
} from './VotingScreen';

const validSummary = {
  id: 'vote-1',
  building_id: 'b-1',
  title: 'Roof repair budget',
  status: 'open',
  end_at: '2026-07-01T00:00:00Z',
  quorum_type: 'simple',
};

describe('parseVoteSummaries', () => {
  it('accepts a bare array of summaries', () => {
    expect(parseVoteSummaries([validSummary])).toEqual([validSummary]);
  });

  it('accepts the wrapped `{ votes: [...] }` shape', () => {
    expect(parseVoteSummaries({ votes: [validSummary], total: 1 })).toEqual([validSummary]);
  });

  // Regression: the previous `data as unknown as ApiVoteSummary[]` double-cast
  // let these shapes through and crashed at `.map(toUiVote)` render time.
  it.each([
    ['null', null],
    ['undefined', undefined],
    ['a number', 42],
    ['a string', 'unexpected'],
    ['an object without votes', { error: 'boom' }],
    ['an object whose votes is not an array', { votes: 'nope' }],
  ])('returns [] for an unexpected top-level shape: %s', (_label, input) => {
    expect(parseVoteSummaries(input)).toEqual([]);
  });

  it('drops malformed entries instead of throwing', () => {
    const mixed = [
      validSummary,
      null,
      'garbage',
      { id: 'no-title' }, // missing title/status/end_at
      { ...validSummary, id: 123 }, // wrong field type
    ];
    expect(parseVoteSummaries(mixed)).toEqual([validSummary]);
  });
});

describe('formatVoteDate', () => {
  const iso = '2026-07-01T00:00:00Z';

  // Regression: the screen hardcoded 'en-US', so vote dates never localised.
  // The formatter must honour the locale it is handed (i18n.language at the
  // call site) rather than a fixed locale.
  it('formats using the locale it is passed', () => {
    const enUS = formatVoteDate(iso, 'en-US');
    const de = formatVoteDate(iso, 'de-DE');

    // Both produce a non-empty, parseable string for the same instant…
    expect(enUS).toBeTruthy();
    expect(de).toBeTruthy();
    // …and locale-specific month abbreviations differ between en and de
    // (e.g. "Jul" vs "Juli"), proving the locale argument is actually used.
    expect(de).not.toBe(enUS);
  });

  it.each([
    ['en-US', /2026/],
    ['sk', /2026/],
    ['cs', /2026/],
    ['de', /2026/],
  ])('includes the year for locale %s', (locale, yearMatcher) => {
    expect(formatVoteDate(iso, locale as string)).toMatch(yearMatcher as RegExp);
  });
});

describe('toUiStatus', () => {
  // 'active'/'open' collapse to the UI 'active' state…
  it.each([['active'], ['open']])('maps %s to "active"', (status) => {
    expect(toUiStatus(status)).toBe('active');
  });

  // …'closed'/'cancelled' collapse to the UI 'closed' state…
  it.each([['closed'], ['cancelled']])('maps %s to "closed"', (status) => {
    expect(toUiStatus(status)).toBe('closed');
  });

  // …and every other/unknown status falls through to 'pending' (the safe
  // default) rather than leaking an unmapped backend string into the UI enum.
  it.each([['draft'], ['scheduled'], ['archived'], ['unknown'], ['']])(
    'maps unrecognised status %s to "pending"',
    (status) => {
      expect(toUiStatus(status)).toBe('pending');
    }
  );

  it('returns a value in the VoteStatus union for any input', () => {
    const allowed = new Set(['active', 'closed', 'pending']);
    for (const status of ['active', 'open', 'closed', 'cancelled', 'whatever']) {
      expect(allowed.has(toUiStatus(status))).toBe(true);
    }
  });
});

describe('toUiVote', () => {
  const base: ApiVoteSummary = {
    id: 'vote-1',
    building_id: 'b-1',
    title: 'Roof repair budget',
    status: 'open',
    end_at: '2026-07-01T00:00:00Z',
    quorum_type: 'simple',
    participation_count: 12,
    eligible_count: 40,
    quorum_met: false,
  };

  it('maps the summary onto the UI Vote shape', () => {
    expect(toUiVote(base)).toEqual({
      id: 'vote-1',
      title: 'Roof repair budget',
      description: '',
      status: 'active',
      type: 'simple',
      options: [],
      startsAt: '',
      endsAt: '2026-07-01T00:00:00Z',
      totalVotes: 12,
      requiredQuorum: 40,
      currentQuorum: 12,
      hasVoted: false,
      createdBy: 'Building Management',
    });
  });

  it('routes the status through toUiStatus', () => {
    expect(toUiVote({ ...base, status: 'cancelled' }).status).toBe('closed');
    expect(toUiVote({ ...base, status: 'mystery' }).status).toBe('pending');
  });

  // The list endpoint omits options/results detail, so these are always the
  // documented "summary" placeholders regardless of input.
  it('always returns the summary-only placeholders', () => {
    const ui = toUiVote(base);
    expect(ui.description).toBe('');
    expect(ui.type).toBe('simple');
    expect(ui.options).toEqual([]);
    expect(ui.startsAt).toBe('');
    expect(ui.hasVoted).toBe(false);
    expect(ui.createdBy).toBe('Building Management');
  });

  // participation_count → totalVotes AND currentQuorum (same source value).
  it('uses participation_count for both totalVotes and currentQuorum', () => {
    const ui = toUiVote({ ...base, participation_count: 7 });
    expect(ui.totalVotes).toBe(7);
    expect(ui.currentQuorum).toBe(7);
  });

  // Nullable numeric fields fall back to 0 rather than producing NaN in the
  // quorum-progress width math on the screen.
  it.each([
    ['null', null],
    ['undefined', undefined],
  ])('defaults missing participation_count (%s) to 0', (_label, value) => {
    const ui = toUiVote({ ...base, participation_count: value as number | null | undefined });
    expect(ui.totalVotes).toBe(0);
    expect(ui.currentQuorum).toBe(0);
  });

  it.each([
    ['null', null],
    ['undefined', undefined],
  ])('defaults missing eligible_count (%s) to 0', (_label, value) => {
    const ui = toUiVote({ ...base, eligible_count: value as number | null | undefined });
    expect(ui.requiredQuorum).toBe(0);
  });

  it('preserves end_at as endsAt verbatim', () => {
    expect(toUiVote({ ...base, end_at: '2030-01-02T03:04:05Z' }).endsAt).toBe(
      '2030-01-02T03:04:05Z'
    );
  });
});

describe('quorumProgressPercent', () => {
  // Regression: the screen computed `(current / required) * 100` inline, so a
  // vote with eligible_count null/0 (requiredQuorum === 0) produced
  // `0 / 0 = NaN` (or `n / 0 = Infinity`) → a `width: "NaN%"` style string.
  // The guarded helper must always return a finite number in [0, 100].
  it.each([
    ['zero required, zero current', 0, 0],
    ['zero required, positive current', 5, 0],
    ['negative required', 3, -2],
    ['NaN required', 3, Number.NaN],
    ['NaN current', Number.NaN, 40],
    ['Infinity required', 3, Number.POSITIVE_INFINITY],
    ['Infinity current', Number.POSITIVE_INFINITY, 40],
  ])('never returns NaN/Infinity (%s)', (_label, current, required) => {
    const pct = quorumProgressPercent(current, required);
    expect(Number.isFinite(pct)).toBe(true);
    expect(pct).toBeGreaterThanOrEqual(0);
    expect(pct).toBeLessThanOrEqual(100);
  });

  it('returns 0 when there is no eligible population (required <= 0)', () => {
    expect(quorumProgressPercent(0, 0)).toBe(0);
    expect(quorumProgressPercent(12, 0)).toBe(0);
  });

  it('returns 0 for a non-positive current count', () => {
    expect(quorumProgressPercent(0, 40)).toBe(0);
    expect(quorumProgressPercent(-5, 40)).toBe(0);
  });

  it('computes the ratio as a percentage for a normal case', () => {
    expect(quorumProgressPercent(10, 40)).toBe(25);
    expect(quorumProgressPercent(20, 40)).toBe(50);
  });

  it('clamps to 100 when current meets or exceeds required', () => {
    expect(quorumProgressPercent(40, 40)).toBe(100);
    expect(quorumProgressPercent(80, 40)).toBe(100);
  });

  // End-to-end with the mapper: a summary with a null eligible_count must not
  // feed NaN into the width math.
  it('stays finite when fed a toUiVote result with null eligible_count', () => {
    const ui = toUiVote({
      id: 'v',
      building_id: 'b',
      title: 't',
      status: 'open',
      end_at: '2026-07-01T00:00:00Z',
      quorum_type: 'simple',
      participation_count: 5,
      eligible_count: null,
    });
    const pct = quorumProgressPercent(ui.currentQuorum, ui.requiredQuorum);
    expect(Number.isFinite(pct)).toBe(true);
    expect(pct).toBe(0);
  });
});
