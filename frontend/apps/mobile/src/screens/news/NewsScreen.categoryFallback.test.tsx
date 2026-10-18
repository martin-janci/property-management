/**
 * NewsScreen — category-badge i18n fallback regression test (issue #3116).
 *
 * Follow-up to PR #3110: `ArticleDetailScreen` renders the category badge
 * defensively — `t(\`news.categories.${cat}\`, { defaultValue: cat })` — so a
 * category whose translation key is absent from the active locale bundle
 * degrades to the raw backend value instead of leaking the bare i18n key
 * (`news.categories.<x>`) to the user. The sibling list screen `NewsScreen`
 * rendered the same badge WITHOUT that fallback, so the two screens behaved
 * inconsistently for an untranslated/new category.
 *
 * Unlike `NewsScreen.i18n.test.tsx` (which relies on the global
 * `t:(key)=>key` stub in `src/test/setup.ts` and therefore can only assert on
 * i18n KEYS), this suite installs a file-local react-i18next mock whose `t`
 * honours `defaultValue` — i.e. it simulates a locale bundle that is missing
 * the category key and falls back to the supplied default. That is the one
 * behaviour the global stub cannot express, and the only way to pin the
 * fallback: without the fix the badge shows `news.categories.building`; with
 * it, the raw `building`.
 */

import { render, screen } from '@testing-library/react-native';

// File-local override of the global `t:(key)=>key` mock: this `t` resolves no
// keys (simulating a bundle with the category translation missing) and falls
// back to `defaultValue` when one is provided — mirroring real i18next.
jest.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: { defaultValue?: string }) => opts?.defaultValue ?? key,
    i18n: { language: 'en', changeLanguage: jest.fn() },
  }),
  initReactI18next: { type: '3rdParty', init: jest.fn() },
}));

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

import { NewsScreen } from './NewsScreen';

describe('NewsScreen category-badge i18n fallback (#3116)', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiQuery.mockReturnValue({
      data: {
        articles: [
          {
            id: 'a1',
            title: 'Lift maintenance',
            summary: 'The lift will be serviced.',
            category: 'building',
            created_at: '2026-01-10T00:00:00Z',
            read: false,
          },
        ],
      },
      isLoading: false,
      error: null,
      refetch: jest.fn(),
      isFetching: false,
    });
  });

  it('degrades the badge to the raw category when its translation key is missing', () => {
    render(<NewsScreen />);
    // With the defaultValue fallback the badge shows the raw category…
    expect(screen.getByText('building')).toBeTruthy();
    // …and never leaks the bare i18n key to the user.
    expect(screen.queryByText('news.categories.building')).toBeNull();
  });
});
