import { formatVoteDate, parseVoteSummaries } from './VotingScreen';

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
