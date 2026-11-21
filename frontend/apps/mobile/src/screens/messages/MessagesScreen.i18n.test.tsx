/**
 * MessagesScreen — search-placeholder i18n regression test
 * (code-review-mobile-rn-list-search-placeholders-i18n).
 *
 * The thread-list search box rendered a hardcoded English placeholder
 * ("Search messages…") instead of routing through react-i18next, even though
 * the rest of the screen already used `t()`. This suite locks the contract:
 * the placeholder now flows through `t('messages.searchPlaceholder', …)` and
 * the key resolves to real, locale-distinct copy in every locale file.
 *
 * Two layers:
 *  1. Render layer — a local react-i18next mock resolves `t(key) => key`
 *     (unlike the global `src/test/setup.ts` mock, which returns the
 *     `defaultValue`), so we can assert the placeholder is the i18n KEY and
 *     the old hardcoded literal is gone.
 *  2. Resource layer — the locale JSON files carry the new key with
 *     translated, locale-distinct values. (Before the fix, this key did not
 *     exist in any locale, so these assertions fail on `undefined`.)
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { MessagesScreen } from './MessagesScreen';

// Local override: resolve to the raw key (ignoring defaultValue) so the render
// layer can distinguish a routed-through-`t()` placeholder from a hardcoded
// English literal.
jest.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: 'en' } }),
}));

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

describe('MessagesScreen search-placeholder i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiQuery.mockReturnValue({
      data: { threads: [], count: 0, total: 0 },
      isLoading: false,
      error: null,
      refetch: jest.fn(),
      isFetching: false,
    });
  });

  it('routes the search placeholder through the messages.searchPlaceholder key', async () => {
    await render(<MessagesScreen />);

    expect(screen.getByPlaceholderText('messages.searchPlaceholder')).toBeTruthy();
    // The old hardcoded English literal must no longer be rendered.
    expect(screen.queryByPlaceholderText('Search messages…')).toBeNull();
  });
});

describe('messages.searchPlaceholder resolves to real, locale-distinct copy', () => {
  it('en carries the expected English copy', () => {
    expect(en.messages.searchPlaceholder).toBe('Search messages…');
  });

  it('every locale defines a non-empty, non-English translation', () => {
    for (const locale of [sk, cs, de, hu, pl]) {
      expect(typeof locale.messages.searchPlaceholder).toBe('string');
      expect(locale.messages.searchPlaceholder.length).toBeGreaterThan(0);
      expect(locale.messages.searchPlaceholder).not.toBe(en.messages.searchPlaceholder);
    }
  });
});
