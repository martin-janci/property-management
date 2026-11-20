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

  it('loading state renders the common.loading key, not a hardcoded literal', async () => {
    mockQuery({ isLoading: true });
    await render(<FaultsListScreen />);
    expect(screen.getByText('common.loading')).toBeTruthy();
    expect(screen.queryByText('Loading…')).toBeNull();
  });

  it('error state renders the faults.loadError key, not the hardcoded literal', async () => {
    mockQuery({ error: new Error('boom') });
    await render(<FaultsListScreen />);
    expect(screen.getByText('faults.loadError')).toBeTruthy();
    expect(screen.queryByText("Couldn't load faults")).toBeNull();
  });

  it('empty state renders the faults.emptyTitle/emptyText keys, not hardcoded literals', async () => {
    mockQuery({ data: { faults: [] } });
    await render(<FaultsListScreen />);
    expect(screen.getByText('faults.emptyTitle')).toBeTruthy();
    expect(screen.getByText('faults.emptyText')).toBeTruthy();
    for (const literal of HARDCODED) {
      expect(screen.queryByText(literal)).toBeNull();
    }
  });
});

describe('FaultsListScreen chrome + card labels i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  // The header title, "+ Report" button, filter chips, status/priority badges
  // and the "Assigned to:" label were all hardcoded English (or raw enum
  // values). They are now sourced from i18n keys via `t()`.
  const CHROME_HARDCODED = ['Faults', '+ Report', 'All', 'Open', 'Resolved', 'Assigned to:'];

  it('renders header, add-button and filter chips from i18n keys, not literals', async () => {
    mockQuery({ data: { faults: [] } });
    await render(<FaultsListScreen />);

    expect(screen.getByText('tabs.faults')).toBeTruthy();
    expect(screen.getByText('faults.addButton')).toBeTruthy();
    expect(screen.getByText('faults.filterAll')).toBeTruthy();
    expect(screen.getByText('faults.filterOpen')).toBeTruthy();
    expect(screen.getByText('faults.filterResolved')).toBeTruthy();

    for (const literal of CHROME_HARDCODED) {
      expect(screen.queryByText(literal)).toBeNull();
    }
  });

  it('renders a fault card status/priority/assignee via i18n keys, not raw enums', async () => {
    mockQuery({
      data: {
        faults: [
          {
            id: 'f1',
            title: 'Leaky faucet',
            description: 'Water everywhere',
            status: 'in_progress',
            priority: 'high',
            category: 'plumbing',
            location_description: 'Unit 301',
            created_at: '2026-01-01T10:00:00Z',
            updated_at: '2026-01-02T10:00:00Z',
            reporter_name: 'Jane',
            assignee_name: 'Bob the plumber',
          },
        ],
        count: 1,
      },
    });
    await render(<FaultsListScreen />);

    // Status + priority badges come from the enum→key maps.
    expect(screen.getByText('faults.statusInProgress')).toBeTruthy();
    expect(screen.getByText('faults.priorityHigh')).toBeTruthy();
    // "Assigned to:" label is localized; the assignee value stays verbatim.
    expect(screen.getByText('faults.assignedTo')).toBeTruthy();
    expect(screen.getByText('Bob the plumber')).toBeTruthy();

    // Raw enum / hardcoded forms must be gone.
    expect(screen.queryByText('in progress')).toBeNull();
    expect(screen.queryByText('high')).toBeNull();
    expect(screen.queryByText('Assigned to:')).toBeNull();
  });
});

describe('fault chrome keys resolve to real, locale-distinct translations', () => {
  it('en carries the expected English copy for the new keys', () => {
    expect(en.faults.addButton).toBe('+ Report');
    expect(en.faults.filterAll).toBe('All');
    expect(en.faults.filterOpen).toBe('Open');
    expect(en.faults.filterResolved).toBe('Resolved');
    expect(en.faults.assignedTo).toBe('Assigned to:');
  });

  it('every locale defines the new keys and localizes the "Assigned to" label', () => {
    for (const locale of [sk, cs, de]) {
      for (const key of [
        'addButton',
        'filterAll',
        'filterOpen',
        'filterResolved',
        'assignedTo',
      ] as const) {
        expect(typeof locale.faults[key]).toBe('string');
        expect(locale.faults[key].length).toBeGreaterThan(0);
      }
      expect(locale.faults.assignedTo).not.toBe(en.faults.assignedTo);
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
