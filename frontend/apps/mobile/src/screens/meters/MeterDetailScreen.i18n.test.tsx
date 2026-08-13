/**
 * MeterDetailScreen — hardcoded-string i18n regression test
 * (code-review-mobile-rn-meterdetail-i18n).
 *
 * MeterDetailScreen rendered several UI strings as hardcoded English literals
 * ("← Meters", "Latest reading", "Reading history", "Submit new reading", the
 * "Used since previous reading" summary, and the "Meter" fallback label) even
 * though react-i18next was already wired for its loading/error/empty states.
 * This suite locks the contract: every one of those strings now flows through
 * an i18n key via `t()`, and the keys resolve to real, locale-distinct copy.
 *
 * Two layers, mirroring FaultsListScreen.i18n.test.tsx:
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
import { MeterDetailScreen } from './MeterDetailScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

// Force the app locale away from the runtime default so the regression test
// below can prove the rendered dates are formatted with `resolveLocale()`
// rather than a bare, device-locale `toLocaleDateString()` (issue #2752).
jest.mock('../../i18n/format', () => ({
  resolveLocale: jest.fn(() => 'de'),
}));

const mockResolveLocale = jest.requireMock('../../i18n/format').resolveLocale as jest.Mock;

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

const responseWithReadings = {
  meter: { id: 'm-1', meter_number: 'CW-001', unit_of_measure: 'm³' },
  recent_readings: [
    { id: 'r2', reading: '142.4', reading_date: '2026-03-31' },
    { id: 'r1', reading: '141.2', reading_date: '2026-02-28' },
  ],
};

// The old hardcoded literals that must no longer be rendered.
const HARDCODED = ['← Meters', 'Latest reading', 'Reading history', 'Submit new reading'];

describe('MeterDetailScreen hardcoded-string i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders the meters.* keys, not the hardcoded English literals', () => {
    mockQuery({ data: responseWithReadings });
    render(<MeterDetailScreen meterId="m-1" onBack={() => {}} onNavigate={() => {}} />);

    expect(screen.getByText('meters.backToMeters', { exact: false })).toBeTruthy();
    expect(screen.getByText('meters.latestReading')).toBeTruthy();
    expect(screen.getByText('meters.usedSincePrevious')).toBeTruthy();
    expect(screen.getByText('meters.readingHistory')).toBeTruthy();
    expect(screen.getByText('meters.submitNewReading')).toBeTruthy();

    for (const literal of HARDCODED) {
      expect(screen.queryByText(literal)).toBeNull();
    }
  });

  it('falls back to the meters.meterFallback key when no label or meter number is present', () => {
    mockQuery({ data: { meter: { id: 'm-1' }, recent_readings: [] } });
    render(<MeterDetailScreen meterId="m-1" />);
    expect(screen.getByText('meters.meterFallback')).toBeTruthy();
    expect(screen.queryByText('Meter')).toBeNull();
  });
});

describe('meter detail keys resolve to real, locale-distinct translations', () => {
  it('en carries the expected English copy', () => {
    expect(en.meters.meterFallback).toBe('Meter');
    expect(en.meters.backToMeters).toBe('Meters');
    expect(en.meters.latestReading).toBe('Latest reading');
    expect(en.meters.readingHistory).toBe('Reading history');
    expect(en.meters.submitNewReading).toBe('Submit new reading');
    expect(en.meters.usedSincePrevious).toContain('{{amount}}');
    expect(en.meters.usedSincePrevious).toContain('{{unit}}');
  });

  it('every locale defines the keys and preserves the interpolation placeholders', () => {
    for (const locale of [sk, cs, de, hu, pl]) {
      for (const key of [
        'backToMeters',
        'latestReading',
        'readingHistory',
        'submitNewReading',
        'meterFallback',
        'usedSincePrevious',
      ] as const) {
        expect(typeof locale.meters[key]).toBe('string');
        expect(locale.meters[key].length).toBeGreaterThan(0);
      }
      // Real translations, not English copies left behind.
      expect(locale.meters.latestReading).not.toBe(en.meters.latestReading);
      expect(locale.meters.submitNewReading).not.toBe(en.meters.submitNewReading);
      // Interpolation placeholders survive translation.
      expect(locale.meters.usedSincePrevious).toContain('{{amount}}');
      expect(locale.meters.usedSincePrevious).toContain('{{unit}}');
    }
  });
});

describe('reading dates follow the in-app locale, not the device locale (#2752)', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('formats the latest-reading and history dates via resolveLocale()', () => {
    mockQuery({ data: responseWithReadings });
    render(<MeterDetailScreen meterId="m-1" onBack={() => {}} onNavigate={() => {}} />);

    // The screen must route every reading date through the app locale helper.
    expect(mockResolveLocale).toHaveBeenCalled();

    // With resolveLocale mocked to 'de', the newest reading (2026-03-31) and
    // the older one (2026-02-28) render in German day.month.year order — the
    // format a bare, device-locale `toLocaleDateString()` would NOT produce.
    const de1 = new Date('2026-03-31').toLocaleDateString('de'); // "31.3.2026"
    const de2 = new Date('2026-02-28').toLocaleDateString('de'); // "28.2.2026"
    // Latest-reading card + history card both show the newest date.
    expect(screen.getAllByText(de1).length).toBeGreaterThanOrEqual(1);
    // History card shows the older date.
    expect(screen.getByText(de2)).toBeTruthy();

    // Guard the regression: the pre-fix bare-`en-US` rendering must be gone.
    const enUs1 = new Date('2026-03-31').toLocaleDateString('en-US'); // "3/31/2026"
    expect(screen.queryByText(enUs1)).toBeNull();
  });
});
