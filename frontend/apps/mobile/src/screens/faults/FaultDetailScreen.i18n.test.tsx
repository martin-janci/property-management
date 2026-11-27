/**
 * FaultDetailScreen — category-label i18n regression test
 * (code-review-mobile-rn-fault-category-i18n).
 *
 * The fault detail view rendered status and priority through react-i18next
 * (enum→key maps) but dropped the `category` field entirely — it was parsed
 * into the model and never shown, so the one place a fault's category could be
 * surfaced as text carried no localization at all. This suite locks the new
 * contract: the detail screen renders a localized "Category" row sourced from
 * `FAULT_CATEGORY_LABEL_KEY` via `t(key, { defaultValue })`, never a dangling
 * catalog key or a silently-dropped field.
 *
 * Render layer: `src/test/setup.ts` mocks react-i18next so `t(key) => key` but
 * honours an explicit `defaultValue` (`t(key, { defaultValue }) => defaultValue`,
 * matching real i18next for an untranslated key). Because the category value is
 * rendered with a `defaultValue`, the badge resolves to the fallback string
 * under the mock while the row *label* (no defaultValue) resolves to its key —
 * the same split asserted by ArticleDetailScreen.test.tsx. The resource layer
 * (en.json carries real, non-empty values for every mapped key) is covered by
 * FaultDetailScreen.test.ts.
 */

import { render, screen } from '@testing-library/react-native';
import { FaultDetailScreen } from './FaultDetailScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

function mockFault(overrides: Record<string, unknown> = {}) {
  mockUseApiQuery.mockReturnValue({
    data: {
      fault: {
        id: 'f-1',
        title: 'Leaking pipe',
        description: 'Water under the sink',
        location_description: 'Kitchen',
        category: 'plumbing',
        priority: 'high',
        status: 'waiting_parts',
        created_at: '2026-11-20T10:00:00Z',
        reporter_name: 'Jana Nova',
        assigned_to_name: null,
        building_name: 'Block A',
        unit_designation: '3B',
        resolution_notes: null,
        ...overrides,
      },
      timeline: [],
      attachments: [],
    },
    isLoading: false,
    error: null,
  });
}

describe('FaultDetailScreen category i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders a localized Category row routed through i18n, not a dropped field', async () => {
    mockFault();
    await render(<FaultDetailScreen faultId="f-1" />);

    // Row label comes from an i18n key (no defaultValue → mock returns the key).
    expect(screen.getByText('faults.categoryLabel')).toBeTruthy();
    // The value is rendered via t('faults.categoryPlumbing', { defaultValue:
    // 'plumbing' }); the mock honours defaultValue, so the badge resolves to the
    // fallback while a dangling catalog key must NOT appear on screen.
    expect(screen.getByText('plumbing')).toBeTruthy();
    expect(screen.queryByText('faults.categoryPlumbing')).toBeNull();
  });

  it('falls back to the raw server value for an unknown/future category', async () => {
    mockFault({ category: 'telecoms' });
    await render(<FaultDetailScreen faultId="f-1" />);

    // Unmapped category → t('telecoms', { defaultValue: 'telecoms' }) resolves
    // to the raw string instead of a dangling key.
    expect(screen.getByText('telecoms')).toBeTruthy();
  });
});
