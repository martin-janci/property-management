/**
 * Status-badge i18n regression test
 * (code-review-mobile-rn-raw-status-untranslated).
 *
 * Several list/detail screens rendered the raw status enum value straight
 * from the API — `fault.status.replace('_', ' ')`, `{building.status}`,
 * `{vote.status}`, `announcement.status.charAt(0)…`, etc. — so a Slovak or
 * German user saw the English/snake_case token ("in progress", "under_
 * construction") instead of a localized label, even though react-i18next
 * was already wired.
 *
 * The fix routes every status badge through `t(`<area>.status.${value}`)`
 * and adds a nested `status` object per area to all six locale files.
 *
 * Two layers of assertion (mirroring FaultsListScreen.i18n.test.tsx):
 *  1. Render layer — `src/test/setup.ts` mocks react-i18next so `t(key) => key`.
 *     We assert the badge now renders the i18n KEY (`faults.status.in_progress`)
 *     and that the OLD raw token ("in progress") is gone. On the pre-fix source
 *     the raw token would render and these assertions would fail.
 *  2. Resource layer — every locale JSON carries the nested status keys with
 *     locale-distinct values, proving the keys resolve to real translations.
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { FaultsListScreen } from './FaultsListScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

describe('FaultsListScreen status badge i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders the faults.status.<value> key, not the raw enum token', async () => {
    mockUseApiQuery.mockReturnValue({
      data: {
        faults: [
          {
            id: 'f1',
            title: 'Leaking pipe',
            description: 'Water everywhere',
            status: 'in_progress',
            priority: 'high',
            category: 'plumbing',
            location_description: 'Basement',
            created_at: '2026-01-01T10:00:00Z',
            updated_at: '2026-01-01T10:00:00Z',
          },
        ],
        count: 1,
      },
      isLoading: false,
      error: null,
      refetch: jest.fn(),
      isFetching: false,
    });

    await render(<FaultsListScreen />);

    // New behaviour: the badge is sourced from an i18n key.
    expect(screen.getByText('faults.status.in_progress')).toBeTruthy();
    // Old behaviour: the raw `status.replace('_', ' ')` token must be gone.
    expect(screen.queryByText('in progress')).toBeNull();
    expect(screen.queryByText('in_progress')).toBeNull();
  });
});

describe('status keys resolve to real, locale-distinct translations', () => {
  const locales = { sk, cs, de, hu, pl } as const;

  // area -> the enum values that must be present under `<area>.status`.
  const AREAS: Record<string, string[]> = {
    faults: ['open', 'in_progress', 'resolved', 'closed'],
    buildings: ['active', 'under_construction', 'renovation', 'inactive'],
    announcements: ['published', 'draft', 'scheduled'],
    leases: ['active', 'pending', 'expired', 'terminated'],
    voting: ['active', 'closed', 'pending'],
    outages: ['scheduled', 'in_progress', 'resolved'],
  };

  it('en defines every status key for every area', () => {
    for (const [area, values] of Object.entries(AREAS)) {
      const status = (en as Record<string, { status?: Record<string, string> }>)[area]?.status;
      expect(status).toBeDefined();
      for (const value of values) {
        expect(typeof status?.[value]).toBe('string');
        expect((status?.[value] ?? '').length).toBeGreaterThan(0);
      }
    }
  });

  it('every non-English locale defines the keys and localizes them away from English', () => {
    for (const [area, values] of Object.entries(AREAS)) {
      const enStatus = (en as Record<string, { status?: Record<string, string> }>)[area]
        ?.status as Record<string, string>;
      for (const [, locale] of Object.entries(locales)) {
        const status = (locale as Record<string, { status?: Record<string, string> }>)[area]
          ?.status as Record<string, string> | undefined;
        expect(status).toBeDefined();
        for (const value of values) {
          expect(typeof status?.[value]).toBe('string');
          expect((status?.[value] ?? '').length).toBeGreaterThan(0);
        }
        // At least one value must differ from English, proving real translation
        // (not an English copy left behind). Some tokens legitimately coincide
        // across languages (e.g. "Aktiv"), so we check the set, not every key.
        const anyLocalized = values.some((v) => status?.[v] !== enStatus?.[v]);
        expect(anyLocalized).toBe(true);
      }
    }
  });
});
