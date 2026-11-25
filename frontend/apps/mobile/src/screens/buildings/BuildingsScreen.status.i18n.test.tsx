/**
 * Status-badge i18n regression test
 * (code-review-mobile-rn-raw-status-untranslated, stall #3357 re-land).
 *
 * Several list/detail screens rendered the raw status enum value straight
 * from the API — `{building.status}`, `{lease.status}`, `{vote.status}`,
 * `announcement.status.charAt(0)…`, `data.status.replace('_', ' ')` — so a
 * Slovak/German/etc. user saw the English or `snake_case` token instead of a
 * localized label, even though react-i18next was already wired.
 *
 * The fix routes each badge through `t(`<area>.status.${value}`)` and adds a
 * nested `status` object per area to all six locale files. (The faults badge
 * is deliberately NOT covered here: `dev` already localizes it via the flat
 * `faults.status*` keys + `statusLabelKey` map, so this re-land leaves the
 * faults convention untouched and only lands the other five screens.)
 *
 * Two layers of assertion (mirroring FaultsListScreen.i18n.test.tsx):
 *  1. Render layer — `src/test/setup.ts` mocks react-i18next so `t(key) => key`.
 *     We render BuildingsScreen (representative) and assert the badge now
 *     renders the i18n KEY (`buildings.status.active`) and that the OLD raw
 *     token (`active`) is gone. On the pre-fix source the raw token rendered
 *     and both assertions would fail.
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
import { BuildingsScreen } from './BuildingsScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useTenantId: jest.fn(),
}));

const api = jest.requireMock('../../hooks/useApi');
const mockUseApiQuery = api.useApiQuery as jest.Mock;
const mockUseTenantId = api.useTenantId as jest.Mock;

describe('BuildingsScreen status badge i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseTenantId.mockReturnValue({ tenantId: 'org-1' });
  });

  it('renders the buildings.status.<value> key, not the raw enum token', async () => {
    mockUseApiQuery.mockReturnValue({
      data: {
        buildings: [
          {
            id: 'b1',
            name: 'Riverside Court',
            street: 'Main St 1',
            city: 'Bratislava',
            postal_code: '81101',
            total_floors: 4,
            status: 'active',
            unit_count: 12,
          },
        ],
        total: 1,
      },
      isLoading: false,
      error: null,
      refetch: jest.fn(),
      isFetching: false,
    });

    await render(<BuildingsScreen />);

    // New behaviour: the badge is sourced from an i18n key.
    expect(screen.getByText('buildings.status.active')).toBeTruthy();
    // Old behaviour: the raw status token must be gone.
    expect(screen.queryByText('active')).toBeNull();
  });
});

describe('status keys resolve to real, locale-distinct translations', () => {
  const locales = { sk, cs, de, hu, pl } as const;

  // area -> the enum values that must be present under `<area>.status`.
  // Covers only the five screens this change lands (faults is owned by dev's
  // flat convention and is intentionally excluded).
  const AREAS: Record<string, string[]> = {
    buildings: ['active', 'under_construction', 'renovation', 'inactive'],
    announcements: ['published', 'draft', 'scheduled'],
    leases: ['active', 'pending', 'expired', 'terminated'],
    voting: ['active', 'closed', 'pending'],
    outages: ['scheduled', 'in_progress', 'resolved'],
  };

  it('en defines every status key for every area', () => {
    for (const [area, values] of Object.entries(AREAS)) {
      const status = (en as unknown as Record<string, { status?: Record<string, string> }>)[area]
        ?.status;
      expect(status).toBeDefined();
      for (const value of values) {
        expect(typeof status?.[value]).toBe('string');
        expect((status?.[value] ?? '').length).toBeGreaterThan(0);
      }
    }
  });

  it('every non-English locale defines the keys and localizes them away from English', () => {
    for (const [area, values] of Object.entries(AREAS)) {
      const enStatus = (en as unknown as Record<string, { status?: Record<string, string> }>)[area]
        ?.status as Record<string, string>;
      for (const [, locale] of Object.entries(locales)) {
        const status = (locale as unknown as Record<string, { status?: Record<string, string> }>)[
          area
        ]?.status as Record<string, string> | undefined;
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
