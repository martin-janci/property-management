/**
 * DocumentPermissionsScreen i18n regression suite
 * (code-review-mobile-rn-i18n-hardcoded-screen-strings).
 *
 * A code-review sweep found that DocumentPermissionsScreen still rendered the
 * audience label + description and the publication-status label as hardcoded
 * English literals ("All Residents", "Visible to every resident in the
 * organization.", "Draft", …) even though react-i18next was already wired on
 * the screen (every other label went through `t()`). A Slovak/German/etc. user
 * saw English for the audience/status copy while the rest of the screen was
 * localized — the strings lived in `getAudienceMeta` / `getStatusMeta`, never
 * passing through `t()`.
 *
 * This suite locks the fix. Two layers (mirroring HardcodedChromeI18n.test):
 *  1. Render layer — `src/test/setup.ts` mocks react-i18next so `t(key) => key`.
 *     We assert the screen renders the i18n KEY and that the OLD hardcoded
 *     literal is gone. On the pre-fix source the literal rendered and the key
 *     did not, so these assertions fail.
 *  2. Resource layer — every locale JSON carries the new keys with non-empty,
 *     locale-distinct values, proving the keys resolve to real translations.
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { type AccessScope, DocumentPermissionsScreen } from './DocumentPermissionsScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useTenantId: jest.fn(() => ({ tenantId: 't1' })),
  apiRequest: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;

function queryResult(overrides: Record<string, unknown>) {
  return {
    data: undefined,
    isLoading: false,
    error: null,
    refetch: jest.fn(),
    isFetching: false,
    ...overrides,
  };
}

function documentPayload(overrides: Record<string, unknown>) {
  return {
    document: {
      id: 'd1',
      title: 'Contract',
      category: 'Legal',
      created_at: '2026-01-01T10:00:00Z',
      updated_at: '2026-01-02T10:00:00Z',
      ...overrides,
    },
  };
}

beforeEach(() => {
  jest.clearAllMocks();
});

describe('DocumentPermissionsScreen audience i18n', () => {
  const cases: Array<{ scope: AccessScope; labelKey: string; descKey: string; literal: string }> = [
    {
      scope: 'organization',
      labelKey: 'documents.permissions.audienceLabels.organization',
      descKey: 'documents.permissions.audienceDescriptions.organization',
      literal: 'All Residents',
    },
    {
      scope: 'building',
      labelKey: 'documents.permissions.audienceLabels.building',
      descKey: 'documents.permissions.audienceDescriptions.building',
      literal: 'Building Only',
    },
    {
      scope: 'unit',
      labelKey: 'documents.permissions.audienceLabels.unit',
      descKey: 'documents.permissions.audienceDescriptions.unit',
      literal: 'Unit Only',
    },
    {
      scope: 'user',
      labelKey: 'documents.permissions.audienceLabels.user',
      descKey: 'documents.permissions.audienceDescriptions.user',
      literal: 'Specific Users',
    },
    {
      scope: 'public',
      labelKey: 'documents.permissions.audienceLabels.public',
      descKey: 'documents.permissions.audienceDescriptions.public',
      literal: 'Public',
    },
  ];

  it.each(cases)(
    'renders audience $scope label + description via i18n keys, not literals',
    async ({ scope, labelKey, descKey, literal }) => {
      mockUseApiQuery.mockReturnValue(
        queryResult({ data: documentPayload({ access_scope: scope, status: 'published' }) })
      );
      await render(<DocumentPermissionsScreen documentId="d1" onBack={jest.fn()} />);

      expect(screen.getByText(labelKey)).toBeTruthy();
      expect(screen.getByText(descKey)).toBeTruthy();
      expect(screen.queryByText(literal)).toBeNull();
    }
  );
});

describe('DocumentPermissionsScreen status i18n', () => {
  const cases: Array<{ status: string; labelKey: string; literal: string }> = [
    {
      status: 'published',
      labelKey: 'documents.permissions.statusLabels.published',
      literal: 'Published',
    },
    { status: 'draft', labelKey: 'documents.permissions.statusLabels.draft', literal: 'Draft' },
    {
      status: 'archived',
      labelKey: 'documents.permissions.statusLabels.archived',
      literal: 'Archived',
    },
  ];

  it.each(cases)(
    'renders $status status label via i18n key, not literal',
    async ({ status, labelKey, literal }) => {
      mockUseApiQuery.mockReturnValue(
        queryResult({ data: documentPayload({ access_scope: 'organization', status }) })
      );
      await render(<DocumentPermissionsScreen documentId="d1" onBack={jest.fn()} />);

      expect(screen.getByText(labelKey)).toBeTruthy();
      expect(screen.queryByText(literal)).toBeNull();
    }
  );
});

describe('permission audience/status keys resolve to real, locale-distinct translations', () => {
  const locales = { sk, cs, de, pl, hu } as const;

  // key path (namespace.key) → expected English copy.
  const EN_EXPECTED: Record<string, string> = {
    'documents.permissions.audienceLabels.organization': 'All Residents',
    'documents.permissions.audienceLabels.building': 'Building Only',
    'documents.permissions.audienceLabels.unit': 'Unit Only',
    'documents.permissions.audienceLabels.user': 'Specific Users',
    'documents.permissions.audienceLabels.public': 'Public',
    'documents.permissions.audienceDescriptions.organization':
      'Visible to every resident in the organization.',
    'documents.permissions.audienceDescriptions.building':
      'Visible to residents of the specific building.',
    'documents.permissions.audienceDescriptions.unit':
      'Visible only to occupants of a specific unit.',
    'documents.permissions.audienceDescriptions.user': 'Visible only to explicitly named users.',
    'documents.permissions.audienceDescriptions.public':
      'Visible to anyone, including unauthenticated users.',
    'documents.permissions.statusLabels.published': 'Published',
    'documents.permissions.statusLabels.draft': 'Draft',
    'documents.permissions.statusLabels.archived': 'Archived',
  };

  function readKey(obj: Record<string, unknown>, path: string): unknown {
    return path.split('.').reduce<unknown>((acc, part) => {
      if (acc && typeof acc === 'object') return (acc as Record<string, unknown>)[part];
      return undefined;
    }, obj);
  }

  it('en carries the expected English copy for every new key', () => {
    for (const [path, value] of Object.entries(EN_EXPECTED)) {
      expect(readKey(en as Record<string, unknown>, path)).toBe(value);
    }
  });

  it('every other locale defines a non-empty, non-English value for each new key', () => {
    for (const [name, locale] of Object.entries(locales)) {
      for (const path of Object.keys(EN_EXPECTED)) {
        const value = readKey(locale as Record<string, unknown>, path);
        expect(typeof value).toBe('string');
        expect((value as string).length).toBeGreaterThan(0);
        // A real translation, not an English copy left behind.
        expect(`${name}:${value}`).not.toBe(`${name}:${EN_EXPECTED[path]}`);
      }
    }
  });
});
