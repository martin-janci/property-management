/**
 * FaultsListScreen — empty/error/loading i18n regression test
 * (code-review-mobile-rn-hardcoded-empty-error-i18n).
 *
 * The list screens (~18 of them) rendered their empty/error/loading state
 * copy as hardcoded English literals ("Loading…", "Couldn't load faults",
 * "No faults found", …) even though react-i18next was already wired. This
 * suite locks the contract for a representative screen: every state string is
 * now sourced from an i18n key via `t()`, not a literal.
 *
 * Two layers of assertion:
 *  1. Render layer — `src/test/setup.ts` mocks react-i18next so `t(key) => key`
 *     (the repo-wide convention, mirrored by DashboardScreen.test.tsx). We
 *     therefore assert the screen renders the i18n KEYS and, crucially, that
 *     the OLD hardcoded English literals are gone. If any string were still
 *     hardcoded, the "not present" assertions would fail.
 *  2. Resource layer — the locale JSON files actually carry translated,
 *     locale-distinct values for those keys, proving the keys resolve to real
 *     translations rather than dangling references.
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import sk from '../../locales/sk.json';
import { FaultsListScreen } from './FaultsListScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

function mockQuery(overrides: Record<string, unknown>) {
  mockUseApiQuery.mockReturnValue({
    data: undefined,
    isLoading: false,
    error: null,
    refetch: jest.fn(),
    isFetching: false,
    ...overrides,
  });
}

// The old hardcoded literals that must no longer be rendered.
const HARDCODED = [
  'Loading…',
  "Couldn't load faults",
  'No faults found',
  'All issues have been addressed',
];

describe('FaultsListScreen empty/error/loading i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('loading state renders the common.loading key, not a hardcoded literal', () => {
    mockQuery({ isLoading: true });
    render(<FaultsListScreen />);
    expect(screen.getByText('common.loading')).toBeTruthy();
    expect(screen.queryByText('Loading…')).toBeNull();
  });

  it('error state renders the faults.loadError key, not the hardcoded literal', () => {
    mockQuery({ error: new Error('boom') });
    render(<FaultsListScreen />);
    expect(screen.getByText('faults.loadError')).toBeTruthy();
    expect(screen.queryByText("Couldn't load faults")).toBeNull();
  });

  it('empty state renders the faults.emptyTitle/emptyText keys, not hardcoded literals', () => {
    mockQuery({ data: { faults: [] } });
    render(<FaultsListScreen />);
    expect(screen.getByText('faults.emptyTitle')).toBeTruthy();
    expect(screen.getByText('faults.emptyText')).toBeTruthy();
    for (const literal of HARDCODED) {
      expect(screen.queryByText(literal)).toBeNull();
    }
  });
});

describe('faults state keys resolve to real, locale-distinct translations', () => {
  it('en carries the expected English copy', () => {
    expect(en.common.loading).toBe('Loading...');
    expect(en.faults.loadError).toBe("Couldn't load faults");
    expect(en.faults.emptyTitle).toBe('No faults found');
    expect(en.faults.emptyText).toBe('All issues have been addressed');
  });

  it('every locale defines the keys and translates them away from English', () => {
    for (const locale of [sk, cs, de]) {
      expect(typeof locale.faults.loadError).toBe('string');
      expect(locale.faults.loadError.length).toBeGreaterThan(0);
      // A real translation, not an English copy left behind.
      expect(locale.faults.loadError).not.toBe(en.faults.loadError);
      expect(locale.faults.emptyTitle).not.toBe(en.faults.emptyTitle);
    }
  });
});
