/**
 * PersonMonthsScreen: read-only, API-backed view (UC-10).
 *
 * Locks the contract:
 * - building/unit ids come from `GET /api/v1/users/me/units` and feed the
 *   `GET /api/v1/buildings/{b}/units/{u}/person-months?year=` request;
 * - rows render newest month first with the localised person count;
 * - there are no edit or bulk-entry controls (writes are manager-only);
 * - loading / error / no-unit / empty states use i18n keys, and every locale
 *   carries them.
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { PersonMonthsScreen, parsePersonMonths } from './PersonMonthsScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

const MY_UNITS = [
  {
    association: { residentId: 'r1', residentType: 'owner' },
    unit: { id: 'unit-1', buildingId: 'bld-1', designation: '4B' },
    building: { id: 'bld-1', street: 'Main 1', city: 'Bratislava', postalCode: '81101' },
  },
];

const ROWS = [
  { id: 'pm-1', unit_id: 'unit-1', year: 2026, month: 1, count: 2, source: 'manual', notes: null },
  { id: 'pm-3', unit_id: 'unit-1', year: 2026, month: 3, count: 3, source: 'manual', notes: null },
];

function query(overrides: Record<string, unknown> = {}) {
  return {
    data: undefined,
    isLoading: false,
    isFetching: false,
    error: null,
    refetch: jest.fn(),
    ...overrides,
  };
}

function mockQueries(units: Record<string, unknown>, months: Record<string, unknown>) {
  mockUseApiQuery.mockImplementation((key: readonly unknown[]) =>
    key[0] === 'my-units' ? query(units) : query(months)
  );
}

describe('PersonMonthsScreen', () => {
  beforeEach(() => jest.clearAllMocks());

  it('requests the unit person-months for the resident unit and current year', async () => {
    mockQueries({ data: MY_UNITS }, { data: ROWS });
    await render(<PersonMonthsScreen />);
    const year = new Date().getFullYear();
    const monthsCall = mockUseApiQuery.mock.calls.find((c) => c[0][0] === 'person-months');
    expect(monthsCall?.[1]).toBe(`/api/v1/buildings/bld-1/units/unit-1/person-months?year=${year}`);
    expect(monthsCall?.[2]).toMatchObject({ enabled: true });
  });

  it('renders rows and header via i18n keys, without edit or bulk controls', async () => {
    mockQueries({ data: MY_UNITS }, { data: ROWS });
    await render(<PersonMonthsScreen />);
    expect(screen.getByText('personMonths.title')).toBeTruthy();
    expect(screen.getByText('personMonths.subtitle')).toBeTruthy();
    expect(screen.getAllByText('personMonths.personCount')).toHaveLength(2);
    expect(screen.getByText('personMonths.contactManager')).toBeTruthy();
    expect(screen.queryByText('common.edit')).toBeNull();
    expect(screen.queryByText('personMonths.bulkEntry')).toBeNull();
  });

  it('shows the no-unit state when the resident has no unit', async () => {
    mockQueries({ data: [] }, {});
    await render(<PersonMonthsScreen />);
    expect(screen.getByText('personMonths.noUnit')).toBeTruthy();
    const monthsCall = mockUseApiQuery.mock.calls.find((c) => c[0][0] === 'person-months');
    expect(monthsCall?.[2]).toMatchObject({ enabled: false });
  });

  it('shows the empty state', async () => {
    mockQueries({ data: MY_UNITS }, { data: [] });
    await render(<PersonMonthsScreen />);
    expect(screen.getByText('personMonths.empty')).toBeTruthy();
  });

  it('shows the error state', async () => {
    mockQueries({ data: MY_UNITS }, { error: new Error('boom') });
    await render(<PersonMonthsScreen />);
    expect(screen.getByText('personMonths.loadError')).toBeTruthy();
  });
});

describe('parsePersonMonths', () => {
  it('drops malformed rows and sorts newest month first', () => {
    const parsed = parsePersonMonths([...ROWS, { id: 7 }, null]);
    expect(parsed.map((r) => r.id)).toEqual(['pm-3', 'pm-1']);
  });

  it('returns [] for non-array payloads', () => {
    expect(parsePersonMonths({ items: ROWS })).toEqual([]);
  });
});

describe('personMonths keys resolve in every locale', () => {
  const KEYS = [
    'title',
    'subtitle',
    'loadError',
    'noUnit',
    'empty',
    'contactManager',
    'previousYear',
    'nextYear',
  ];

  it('en carries the expected English copy', () => {
    expect(en.personMonths.title).toBe('Person-months');
    expect(en.personMonths.personCount_one).toContain('{{count}}');
    expect(en.personMonths.personCount_other).toContain('{{count}}');
  });

  it('every locale defines the keys and translates the subtitle away from English', () => {
    for (const locale of [sk, cs, de, pl, hu]) {
      for (const key of KEYS) {
        const value = (locale.personMonths as unknown as Record<string, string>)[key];
        expect(typeof value).toBe('string');
        expect(value.length).toBeGreaterThan(0);
      }
      expect(locale.personMonths.subtitle).not.toBe(en.personMonths.subtitle);
      expect(locale.personMonths.personCount_other).toContain('{{count}}');
    }
  });
});
