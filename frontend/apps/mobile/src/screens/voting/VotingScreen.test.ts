import { parseVoteSummaries } from './VotingScreen';

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
