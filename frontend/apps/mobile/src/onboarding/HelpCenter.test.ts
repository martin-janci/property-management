/**
 * Unit tests for `HelpCenter` (Epic 50 — Story 50.2/50.3: Contextual Help,
 * FAQ & Tutorials).
 *
 * Covers the three behaviours this ~480-LOC module hinges on and that had zero
 * coverage before:
 *   1. Vote reversal — `voteFAQ` must undo a prior vote's count before applying
 *      the new one, so a user flipping helpful -> not_helpful never double-counts.
 *   2. AsyncStorage persistence — votes are written to `@ppt/faq_votes` and
 *      re-hydrated by `initialize()`, with corrupted/invalid payloads degrading
 *      gracefully to an empty map (never throwing).
 *   3. Search ranking — `searchFAQs` scores question (10) > answer (5) > tag (3)
 *      matches and sorts descending, with optional category filtering.
 *
 * These tests assert the module's ACTUAL behaviour (no refactor): e.g. an empty
 * query matches every FAQ because `String.includes('')` is always true, so the
 * net-helpfulness fallback path is unreachable and results come back in
 * insertion order.
 *
 * Uses a seedable in-memory `@react-native-async-storage/async-storage`
 * replacement (same pattern as resetLocalData.test.ts / the integration suites)
 * so we can assert exactly what is persisted and drive the load path.
 */
import AsyncStorage from '@react-native-async-storage/async-storage';
import { HelpCenter } from './HelpCenter';

const FAQ_VOTES_KEY = '@ppt/faq_votes';

jest.mock('@react-native-async-storage/async-storage', () => {
  const store = new Map<string, string>();
  return {
    __store: store,
    getItem: jest.fn(async (k: string) => store.get(k) ?? null),
    setItem: jest.fn(async (k: string, v: string) => {
      store.set(k, v);
    }),
    removeItem: jest.fn(async (k: string) => {
      store.delete(k);
    }),
    clear: jest.fn(async () => {
      store.clear();
    }),
  };
});

const mockStorage = AsyncStorage as unknown as {
  __store: Map<string, string>;
  getItem: jest.Mock;
  setItem: jest.Mock;
};

const storedVotes = (): Record<string, string> => {
  const raw = mockStorage.__store.get(FAQ_VOTES_KEY);
  return raw ? JSON.parse(raw) : {};
};

beforeEach(() => {
  mockStorage.__store.clear();
  mockStorage.getItem.mockClear();
  mockStorage.setItem.mockClear();
});

describe('HelpCenter — vote reversal', () => {
  it('increments the helpful count and records the user vote', async () => {
    const hc = new HelpCenter();
    // faq-1 seed: helpful 42 / notHelpful 3. searchFAQs returns live references
    // to the stored FAQItem, so snapshot the counts as numbers before voting.
    const beforeHelpful = hc.searchFAQs('', undefined).find((f) => f.id === 'faq-1')?.helpful;

    await hc.voteFAQ('faq-1', true);

    const after = hc.searchFAQs('', undefined).find((f) => f.id === 'faq-1');
    expect(beforeHelpful).toBe(42);
    expect(after?.helpful).toBe(43);
    expect(after?.notHelpful).toBe(3);
    expect(hc.getUserVote('faq-1')).toBe('helpful');
  });

  it('reverses a prior helpful vote when switching to not_helpful (no double count)', async () => {
    const hc = new HelpCenter();

    await hc.voteFAQ('faq-1', true); // helpful 43 / notHelpful 3
    await hc.voteFAQ('faq-1', false); // undo helpful -> 42, add notHelpful -> 4

    const faq = hc.searchFAQs('', undefined).find((f) => f.id === 'faq-1');
    expect(faq?.helpful).toBe(42);
    expect(faq?.notHelpful).toBe(4);
    expect(hc.getUserVote('faq-1')).toBe('not_helpful');
  });

  it('reverses a prior not_helpful vote when switching back to helpful', async () => {
    const hc = new HelpCenter();

    await hc.voteFAQ('faq-1', false); // helpful 42 / notHelpful 4
    await hc.voteFAQ('faq-1', true); // undo notHelpful -> 3, add helpful -> 43

    const faq = hc.searchFAQs('', undefined).find((f) => f.id === 'faq-1');
    expect(faq?.helpful).toBe(43);
    expect(faq?.notHelpful).toBe(3);
    expect(hc.getUserVote('faq-1')).toBe('helpful');
  });

  it('is idempotent for the same vote repeated (removes then re-adds)', async () => {
    const hc = new HelpCenter();

    await hc.voteFAQ('faq-1', true);
    await hc.voteFAQ('faq-1', true);

    const faq = hc.searchFAQs('', undefined).find((f) => f.id === 'faq-1');
    expect(faq?.helpful).toBe(43); // +1 then undo +re-add, net +1
    expect(faq?.notHelpful).toBe(3);
    expect(hc.getUserVote('faq-1')).toBe('helpful');
  });

  it('ignores a vote for an unknown FAQ and does not persist', async () => {
    const hc = new HelpCenter();

    await hc.voteFAQ('faq-does-not-exist', true);

    expect(hc.getUserVote('faq-does-not-exist')).toBeNull();
    expect(mockStorage.setItem).not.toHaveBeenCalled();
  });

  it('getUserVote returns null for an un-voted FAQ', () => {
    const hc = new HelpCenter();
    expect(hc.getUserVote('faq-2')).toBeNull();
  });
});

describe('HelpCenter — AsyncStorage persistence', () => {
  it('persists the user vote map to the FAQ_VOTES_KEY on vote', async () => {
    const hc = new HelpCenter();

    await hc.voteFAQ('faq-1', true);

    expect(mockStorage.setItem).toHaveBeenCalledWith(
      FAQ_VOTES_KEY,
      JSON.stringify({ 'faq-1': 'helpful' })
    );
    expect(storedVotes()).toEqual({ 'faq-1': 'helpful' });
  });

  it('persists the reversed value, not an accumulation', async () => {
    const hc = new HelpCenter();

    await hc.voteFAQ('faq-1', true);
    await hc.voteFAQ('faq-1', false);
    await hc.voteFAQ('faq-4', true);

    expect(storedVotes()).toEqual({ 'faq-1': 'not_helpful', 'faq-4': 'helpful' });
  });

  it('re-hydrates stored votes via initialize()', async () => {
    mockStorage.__store.set(
      FAQ_VOTES_KEY,
      JSON.stringify({ 'faq-1': 'helpful', 'faq-6': 'not_helpful' })
    );

    const hc = new HelpCenter();
    await hc.initialize();

    expect(hc.getUserVote('faq-1')).toBe('helpful');
    expect(hc.getUserVote('faq-6')).toBe('not_helpful');
    expect(hc.getUserVote('faq-2')).toBeNull();
  });

  it('leaves votes empty when storage has no entry', async () => {
    const hc = new HelpCenter();
    await hc.initialize();

    expect(hc.getUserVote('faq-1')).toBeNull();
  });

  it('degrades gracefully on corrupted JSON (resets to empty, no throw)', async () => {
    mockStorage.__store.set(FAQ_VOTES_KEY, '{not valid json');
    const warnSpy = jest.spyOn(console, 'warn').mockImplementation(() => {});

    const hc = new HelpCenter();
    await expect(hc.initialize()).resolves.toBeUndefined();

    expect(hc.getUserVote('faq-1')).toBeNull();
    expect(warnSpy).toHaveBeenCalled();
    warnSpy.mockRestore();
  });

  it('ignores a non-object (array) payload and keeps votes empty', async () => {
    mockStorage.__store.set(FAQ_VOTES_KEY, JSON.stringify(['faq-1']));

    const hc = new HelpCenter();
    await hc.initialize();

    expect(hc.getUserVote('faq-1')).toBeNull();
  });
});

describe('HelpCenter — search ranking', () => {
  it('weights a question match (10) above an answer-only match (5)', () => {
    const hc = new HelpCenter();
    // "change": faq-8 question "How do I change my password?" (10) + answer
    // "...Change Password..." (5) = 15; faq-6 answer "...can be changed..." (5).
    const results = hc.searchFAQs('change');

    expect(results.map((r) => r.id)).toEqual(['faq-8', 'faq-6']);
  });

  it('orders by combined score: answer+tag (8) above answer-only matches (5)', () => {
    const hc = new HelpCenter();
    // "dashboard": faq-1 answer (5) + tag "dashboard" (3) = 8 ranks first;
    // faq-3 and faq-4 each match in their answer only (5), kept in insertion
    // order by the stable sort.
    const results = hc.searchFAQs('dashboard');

    expect(results.map((r) => r.id)).toEqual(['faq-1', 'faq-3', 'faq-4']);
  });

  it('matches on tags (score 3)', () => {
    const hc = new HelpCenter();
    // "alerts" only appears in faq-2's tags.
    const results = hc.searchFAQs('alerts');

    expect(results.map((r) => r.id)).toEqual(['faq-2']);
  });

  it('is case-insensitive and trims the query', () => {
    const hc = new HelpCenter();

    expect(hc.searchFAQs('  PASSWORD  ').map((r) => r.id)).toEqual(['faq-8']);
  });

  it('returns an empty list when nothing matches', () => {
    const hc = new HelpCenter();

    expect(hc.searchFAQs('zzz-no-such-term')).toEqual([]);
  });

  it('filters by category, excluding otherwise-matching FAQs', () => {
    const hc = new HelpCenter();

    // faq-8 ("password") is category "account", so a "security" filter drops it.
    expect(hc.searchFAQs('password', 'account').map((r) => r.id)).toEqual(['faq-8']);
    expect(hc.searchFAQs('password', 'security')).toEqual([]);
  });

  it('returns every FAQ in insertion order for an empty query (includes("") is always true)', () => {
    const hc = new HelpCenter();

    const ids = hc.searchFAQs('').map((r) => r.id);
    expect(ids).toHaveLength(10);
    expect(ids[0]).toBe('faq-1');
    expect(ids).toEqual([
      'faq-1',
      'faq-2',
      'faq-3',
      'faq-4',
      'faq-5',
      'faq-6',
      'faq-7',
      'faq-8',
      'faq-9',
      'faq-10',
    ]);
  });
});
