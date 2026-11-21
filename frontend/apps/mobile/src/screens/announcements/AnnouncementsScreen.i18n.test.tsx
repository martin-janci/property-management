/**
 * AnnouncementsScreen — search-placeholder i18n regression test
 * (code-review-mobile-rn-list-search-placeholders-i18n).
 *
 * The announcements search box rendered a hardcoded English placeholder
 * ("Search announcements...") even though an `announcements.searchPlaceholder`
 * key already existed in every locale file — the screen simply wasn't using
 * it. This suite locks the fix: the placeholder now flows through
 * `t('announcements.searchPlaceholder', …)`.
 *
 * The render layer uses a local react-i18next mock that resolves `t(key) => key`
 * (the global `src/test/setup.ts` mock returns the `defaultValue`), so we can
 * assert the placeholder is the i18n KEY rather than the old literal.
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { AnnouncementsScreen } from './AnnouncementsScreen';

jest.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: 'en' } }),
}));

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

describe('AnnouncementsScreen search-placeholder i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiQuery.mockReturnValue({
      data: { announcements: [], count: 0, total: 0 },
      isLoading: false,
      error: null,
      refetch: jest.fn(),
      isFetching: false,
    });
  });

  it('routes the search placeholder through the announcements.searchPlaceholder key', async () => {
    await render(<AnnouncementsScreen />);

    expect(screen.getByPlaceholderText('announcements.searchPlaceholder')).toBeTruthy();
    // The old hardcoded English literal must no longer be rendered.
    expect(screen.queryByPlaceholderText('Search announcements...')).toBeNull();
  });
});

describe('announcements.searchPlaceholder resolves to real, locale-distinct copy', () => {
  it('en carries the expected English copy', () => {
    expect(en.announcements.searchPlaceholder).toBe('Search announcements...');
  });

  it('every locale defines a non-empty, non-English translation', () => {
    for (const locale of [sk, cs, de, hu, pl]) {
      expect(typeof locale.announcements.searchPlaceholder).toBe('string');
      expect(locale.announcements.searchPlaceholder.length).toBeGreaterThan(0);
      expect(locale.announcements.searchPlaceholder).not.toBe(en.announcements.searchPlaceholder);
    }
  });
});
