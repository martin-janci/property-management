/**
 * ThreadDetailScreen — hardcoded-string i18n regression test
 * (code-review-mobile-rn-thread-detail-hardcoded-en).
 *
 * ThreadDetailScreen rendered several UI strings as hardcoded English literals
 * (the "← Messages" back link, the "Write a message…" composer placeholder, the
 * "Send" button, the "Syncing…" indicator, the "Conversation" title fallback,
 * and the "You" / "Participant" author labels) even though react-i18next was
 * already wired for its loading/error/empty states. This suite locks the
 * contract: every one of those strings now flows through an i18n key via `t()`,
 * and the keys resolve to real, locale-distinct copy.
 *
 * Two layers, mirroring MeterDetailScreen.i18n.test.tsx:
 *  1. Render layer — `src/test/setup.ts` mocks react-i18next so `t(key) => key`.
 *     We assert the screen renders the i18n KEYS and that the OLD hardcoded
 *     English literals are gone.
 *  2. Resource layer — the locale JSON files carry translated, locale-distinct
 *     values for those keys.
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { ThreadDetailScreen } from './ThreadDetailScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useApiMutation: () => ({ mutate: jest.fn(), isPending: false, isError: false, error: undefined }),
}));

jest.mock('../../contexts/AuthContext', () => ({
  useAuth: () => ({ user: { id: 'u-1' } }),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

// A participant-less thread (so the title falls back) with one message from
// another, name-less sender (so the author label falls back), and a background
// refetch in flight (so the "Syncing…" indicator renders).
const threadDetail = {
  participants: [],
  messages: [{ id: 'm1', content: 'Hi', createdAt: '2026-04-22T08:30:00Z', sender: { id: 'u-2' } }],
};

// The old hardcoded literals that must no longer be rendered.
const HARDCODED = [
  '← Messages',
  'Write a message…',
  'Send',
  'Syncing…',
  'Conversation',
  'Participant',
];

describe('ThreadDetailScreen hardcoded-string i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiQuery.mockReturnValue({
      data: threadDetail,
      isLoading: false,
      error: null,
      refetch: jest.fn(),
      isFetching: true,
    });
  });

  it('renders the messages.* keys, not the hardcoded English literals', () => {
    render(<ThreadDetailScreen threadId="t-1" onBack={() => {}} />);

    expect(screen.getByText('messages.backToList', { exact: false })).toBeTruthy();
    expect(screen.getByText('messages.threadTitleFallback')).toBeTruthy();
    expect(screen.getByPlaceholderText('messages.composerPlaceholder')).toBeTruthy();
    expect(screen.getByText('messages.send')).toBeTruthy();
    expect(screen.getByText('messages.syncing')).toBeTruthy();
    expect(screen.getByText('messages.authorFallback')).toBeTruthy();

    for (const literal of HARDCODED) {
      expect(screen.queryByText(literal)).toBeNull();
    }
    expect(screen.queryByPlaceholderText('Write a message…')).toBeNull();
  });
});

describe('thread detail keys resolve to real, locale-distinct translations', () => {
  const keys = [
    'backToList',
    'threadTitleFallback',
    'composerPlaceholder',
    'syncing',
    'authorYou',
    'authorFallback',
    'send',
  ] as const;

  it('en carries the expected English copy', () => {
    expect(en.messages.backToList).toBe('Messages');
    expect(en.messages.threadTitleFallback).toBe('Conversation');
    expect(en.messages.composerPlaceholder).toBe('Write a message…');
    expect(en.messages.syncing).toBe('Syncing…');
    expect(en.messages.authorYou).toBe('You');
    expect(en.messages.authorFallback).toBe('Participant');
    expect(en.messages.send).toBe('Send');
  });

  it('every locale defines the keys with non-empty, non-English copy', () => {
    for (const locale of [sk, cs, de, hu, pl]) {
      for (const key of keys) {
        expect(typeof locale.messages[key]).toBe('string');
        expect(locale.messages[key].length).toBeGreaterThan(0);
      }
      // Real translations, not English copies left behind.
      expect(locale.messages.composerPlaceholder).not.toBe(en.messages.composerPlaceholder);
      expect(locale.messages.send).not.toBe(en.messages.send);
    }
  });
});
