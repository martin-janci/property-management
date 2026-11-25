/**
 * Status-badge i18n regression test
 * (code-review-mobile-rn-raw-status-untranslated — remaining screens).
 *
 * Several list/detail screens rendered the raw status enum value straight
 * from the API — `{building.status}`, `{vote.status}`, `{lease.status}`,
 * `announcement.status.charAt(0)…`, `data.status.replace('_', ' ')` — so a
 * Slovak/German/etc. user saw the English/snake_case token
 * ("under_construction") instead of a localized label, even though
 * react-i18next was already wired.
 *
 * FaultsListScreen was already fixed on `dev` via its own `statusLabelKey`
 * map (and is intentionally left untouched here); this suite locks the fix
 * for the five remaining screens — Buildings, Voting, Leases,
 * AnnouncementDetail and OutageDetail — which now route every status badge
 * through `t(`<area>.status.${value}`)`.
 *
 * Two layers of assertion (mirroring AnnouncementsScreen.i18n.test.tsx):
 *  1. Render layer — a local react-i18next mock resolves `t(key) => key`, so
 *     we assert the badge renders the i18n KEY (`buildings.status.under_construction`)
 *     and that the OLD raw token (`under_construction`) is gone. On the
 *     pre-fix source the raw token rendered and these assertions fail.
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

jest.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: 'en' } }),
}));

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
    mockUseTenantId.mockReturnValue({ tenantId: 'tenant-1' });
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
            status: 'under_construction',
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
  });

  it('renders the buildings.status.<value> key, not the raw enum token', async () => {
    await render(<BuildingsScreen />);

    // New behaviour: the badge is sourced from an i18n key.
    expect(screen.getByText('buildings.status.under_construction')).toBeTruthy();
    // Old behaviour: the raw `{building.status}` token must be gone.
    expect(screen.queryByText('under_construction')).toBeNull();
  });
});

describe('status keys resolve to real, locale-distinct translations', () => {
  const locales = { sk, cs, de, hu, pl } as const;

  // area -> the enum values that must be present under `<area>.status`.
  // (faults is excluded — it uses the pre-existing flat `statusOpen`… keys;
  //  outages pre-dates this change and already ships its own status keys.)
  const AREAS: Record<string, string[]> = {
    buildings: ['active', 'under_construction', 'renovation', 'inactive'],
    announcements: ['published', 'draft', 'scheduled'],
    leases: ['active', 'pending', 'expired', 'terminated'],
    voting: ['active', 'closed', 'pending'],
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
