/**
 * OutagesScreen — header + filter-chip i18n regression test
 * (code-review-mobile-rn-outages-news-header-chip-i18n).
 *
 * The screen rendered its header title/subtitle and the status filter chips
 * ("Outages", "Planned and active utility interruptions.", "All",
 * "Scheduled", "Active", "Resolved") as hardcoded English literals even
 * though react-i18next was already wired for the empty/error states. This
 * suite locks the contract: every header/chip string is now sourced from an
 * i18n key via `t()`, not a literal.
 *
 * Mirrors FaultsListScreen.i18n.test.tsx:
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
import { OutagesScreen } from './OutagesScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

// The old hardcoded header/chip literals that must no longer be rendered.
const HARDCODED = [
  'Outages',
  'Planned and active utility interruptions.',
  'All',
  'Scheduled',
  'Active',
  'Resolved',
];

describe('OutagesScreen header + filter-chip i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiQuery.mockReturnValue({
      data: { outages: [] },
      isLoading: false,
      error: null,
      refetch: jest.fn(),
      isFetching: false,
    });
  });

  it('renders header title/subtitle keys, not hardcoded literals', () => {
    render(<OutagesScreen />);
    expect(screen.getByText('outages.title')).toBeTruthy();
    expect(screen.getByText('outages.subtitle')).toBeTruthy();
  });

  it('renders filter-chip keys, not hardcoded literals', () => {
    render(<OutagesScreen />);
    expect(screen.getByText('outages.filters.all')).toBeTruthy();
    expect(screen.getByText('outages.filters.scheduled')).toBeTruthy();
    expect(screen.getByText('outages.filters.active')).toBeTruthy();
    expect(screen.getByText('outages.filters.resolved')).toBeTruthy();
  });

  it('no longer renders any hardcoded English header/chip literal', () => {
    render(<OutagesScreen />);
    for (const literal of HARDCODED) {
      expect(screen.queryByText(literal)).toBeNull();
    }
  });
});

describe('outages header/filter keys resolve to real, locale-distinct translations', () => {
  it('en carries the expected English copy', () => {
    expect(en.outages.title).toBe('Outages');
    expect(en.outages.subtitle).toBe('Planned and active utility interruptions.');
    expect(en.outages.filters.all).toBe('All');
    expect(en.outages.filters.scheduled).toBe('Scheduled');
    expect(en.outages.filters.active).toBe('Active');
    expect(en.outages.filters.resolved).toBe('Resolved');
  });

  it('every locale defines the keys and translates them away from English', () => {
    for (const locale of [sk, cs, de, pl, hu]) {
      for (const key of ['title', 'subtitle'] as const) {
        expect(typeof locale.outages[key]).toBe('string');
        expect(locale.outages[key].length).toBeGreaterThan(0);
        expect(locale.outages[key]).not.toBe(en.outages[key]);
      }
      for (const key of ['all', 'scheduled', 'active', 'resolved'] as const) {
        expect(typeof locale.outages.filters[key]).toBe('string');
        expect(locale.outages.filters[key].length).toBeGreaterThan(0);
      }
    }
  });
});
