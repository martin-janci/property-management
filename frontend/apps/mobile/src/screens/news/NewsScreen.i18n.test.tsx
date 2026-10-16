/**
 * NewsScreen — header + filter-chip i18n regression test
 * (code-review-mobile-rn-outages-news-header-chip-i18n).
 *
 * The screen rendered its header subtitle and the category filter chips
 * ("Updates from your building and neighbourhood.", "All", "Building",
 * "District", "Partners") as hardcoded English literals even though
 * react-i18next was already wired for the empty/error states. This suite
 * locks the contract: every header/chip string is now sourced from an i18n
 * key via `t()`, not a literal. (The header title already reused `news.title`.)
 *
 * Mirrors FaultsListScreen.i18n.test.tsx: render layer asserts the screen
 * renders the i18n KEYS (setup mocks `t(key) => key`) and that the old
 * literals are gone; resource layer asserts the locale JSON carries them.
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { NewsScreen } from './NewsScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

// The old hardcoded header/chip literals that must no longer be rendered.
const HARDCODED = [
  'Updates from your building and neighbourhood.',
  'All',
  'Building',
  'District',
  'Partners',
];

describe('NewsScreen header + filter-chip i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiQuery.mockReturnValue({
      data: { articles: [] },
      isLoading: false,
      error: null,
      refetch: jest.fn(),
      isFetching: false,
    });
  });

  it('renders header title/subtitle keys, not hardcoded literals', () => {
    render(<NewsScreen />);
    expect(screen.getByText('news.title')).toBeTruthy();
    expect(screen.getByText('news.subtitle')).toBeTruthy();
  });

  it('renders filter-chip keys, not hardcoded literals', () => {
    render(<NewsScreen />);
    expect(screen.getByText('news.filters.all')).toBeTruthy();
    expect(screen.getByText('news.filters.building')).toBeTruthy();
    expect(screen.getByText('news.filters.district')).toBeTruthy();
    expect(screen.getByText('news.filters.partner')).toBeTruthy();
  });

  it('no longer renders any hardcoded English header/chip literal', () => {
    render(<NewsScreen />);
    for (const literal of HARDCODED) {
      expect(screen.queryByText(literal)).toBeNull();
    }
  });
});

describe('news header/filter keys resolve to real, locale-distinct translations', () => {
  it('en carries the expected English copy', () => {
    expect(en.news.subtitle).toBe('Updates from your building and neighbourhood.');
    expect(en.news.filters.all).toBe('All');
    expect(en.news.filters.building).toBe('Building');
    expect(en.news.filters.district).toBe('District');
    expect(en.news.filters.partner).toBe('Partners');
  });

  it('every locale defines the keys and translates the subtitle away from English', () => {
    for (const locale of [sk, cs, de, pl, hu]) {
      expect(typeof locale.news.subtitle).toBe('string');
      expect(locale.news.subtitle.length).toBeGreaterThan(0);
      expect(locale.news.subtitle).not.toBe(en.news.subtitle);
      for (const key of ['all', 'building', 'district', 'partner'] as const) {
        expect(typeof locale.news.filters[key]).toBe('string');
        expect(locale.news.filters[key].length).toBeGreaterThan(0);
      }
    }
  });
});
