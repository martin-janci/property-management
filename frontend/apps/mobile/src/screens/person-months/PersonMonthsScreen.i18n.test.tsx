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

import { fireEvent, render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import {
  canReport,
  latestSubmissionByMonth,
  PersonMonthsScreen,
  parsePersonMonths,
  visibleMonths,
} from './PersonMonthsScreen';

const mockMutate = jest.fn();
jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useApiMutation: jest.fn(() => ({ mutate: mockMutate, isPending: false })),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;
const mockUseApiMutation = jest.requireMock('../../hooks/useApi').useApiMutation as jest.Mock;

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

const THIS_YEAR = new Date().getFullYear();
const THIS_MONTH = new Date().getMonth() + 1;

function mockQueries(
  units: Record<string, unknown>,
  months: Record<string, unknown>,
  submissions: Record<string, unknown> = { data: [] }
) {
  mockUseApiQuery.mockImplementation((key: readonly unknown[]) => {
    if (key[0] === 'my-units') return query(units);
    if (key[1] === 'submissions') return query(submissions);
    return query(months);
  });
}

describe('PersonMonthsScreen', () => {
  beforeEach(() => jest.clearAllMocks());

  it('requests person-months and submissions for the resident unit and current year', async () => {
    mockQueries({ data: MY_UNITS }, { data: ROWS });
    await render(<PersonMonthsScreen />);
    const paths = mockUseApiQuery.mock.calls.map((c) => c[1]);
    expect(paths).toContain(`/api/v1/buildings/bld-1/units/unit-1/person-months?year=${THIS_YEAR}`);
    expect(paths).toContain(
      `/api/v1/buildings/bld-1/units/unit-1/person-months/submissions?year=${THIS_YEAR}`
    );
    expect(mockUseApiMutation).toHaveBeenCalledWith(
      '/api/v1/buildings/bld-1/units/unit-1/person-months/submissions',
      'POST'
    );
  });

  it('lists every month up to now with a report action', async () => {
    mockQueries({ data: MY_UNITS }, { data: [] });
    await render(<PersonMonthsScreen />);
    expect(screen.getAllByText('personMonths.report')).toHaveLength(THIS_MONTH);
    expect(screen.getAllByText('personMonths.notRecorded')).toHaveLength(THIS_MONTH);
    expect(screen.getByText('personMonths.approvalHint')).toBeTruthy();
  });

  it('shows a pending report next to the recorded value', async () => {
    mockQueries(
      { data: MY_UNITS },
      {
        data: [
          {
            id: 'pm',
            unit_id: 'unit-1',
            year: THIS_YEAR,
            month: THIS_MONTH,
            count: 2,
            source: 'manual',
            notes: null,
          },
        ],
      },
      {
        data: [
          {
            id: 's1',
            year: THIS_YEAR,
            month: THIS_MONTH,
            count: 3,
            status: 'pending',
            submitted_at: '2026-01-01T00:00:00Z',
            decision_note: null,
          },
        ],
      }
    );
    await render(<PersonMonthsScreen />);
    expect(screen.getByText('personMonths.submissionStatus.pending')).toBeTruthy();
    expect(screen.getByText('personMonths.pendingCount')).toBeTruthy();
    expect(screen.getByText('personMonths.personCount')).toBeTruthy();
  });

  it('validates and sends a report for a month', async () => {
    mockQueries({ data: MY_UNITS }, { data: [] });
    await render(<PersonMonthsScreen />);
    await fireEvent.press(screen.getAllByText('personMonths.report')[0]);
    const input = screen.getByLabelText('personMonths.reportLabel');

    await fireEvent.changeText(input, '99');
    await fireEvent.press(screen.getByText('personMonths.send'));
    expect(screen.getByText('personMonths.invalidMessage')).toBeTruthy();
    expect(mockMutate).not.toHaveBeenCalled();

    await fireEvent.changeText(input, '3');
    await fireEvent.press(screen.getByText('personMonths.send'));
    expect(mockMutate).toHaveBeenCalledWith(
      { year: THIS_YEAR, month: THIS_MONTH, count: 3 },
      expect.any(Object)
    );
  });

  it('shows the no-unit state when the resident has no unit', async () => {
    mockQueries({ data: [] }, {});
    await render(<PersonMonthsScreen />);
    expect(screen.getByText('personMonths.noUnit')).toBeTruthy();
    const monthsCall = mockUseApiQuery.mock.calls.find((c) => c[0][0] === 'person-months');
    expect(monthsCall?.[2]).toMatchObject({ enabled: false });
  });

  it('shows the error state', async () => {
    mockQueries({ data: MY_UNITS }, { error: new Error('boom') });
    await render(<PersonMonthsScreen />);
    expect(screen.getByText('personMonths.loadError')).toBeTruthy();
  });
});

describe('helpers', () => {
  it('visibleMonths stops at the current month and is empty for future years', () => {
    const today = new Date(2026, 2, 15); // March 2026
    expect(visibleMonths(2026, today)).toEqual([3, 2, 1]);
    expect(visibleMonths(2025, today)).toHaveLength(12);
    expect(visibleMonths(2027, today)).toEqual([]);
  });

  it('canReport allows the current month and the 24 before it', () => {
    const today = new Date(2026, 2, 15);
    expect(canReport(2026, 3, today)).toBe(true);
    expect(canReport(2024, 3, today)).toBe(true);
    expect(canReport(2024, 2, today)).toBe(false);
    expect(canReport(2026, 4, today)).toBe(false);
  });

  it('latestSubmissionByMonth keeps the newest per month', () => {
    const map = latestSubmissionByMonth([
      { id: 'a', month: 1, count: 1, status: 'rejected', submitted_at: '2026-01-01' },
      { id: 'b', month: 1, count: 2, status: 'pending', submitted_at: '2026-02-01' },
    ]);
    expect(map.get(1)?.id).toBe('b');
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
    'notRecorded',
    'pendingCount',
    'report',
    'reportLabel',
    'send',
    'approvalHint',
    'submitFailed',
    'invalidMessage',
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
