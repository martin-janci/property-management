/**
 * Screen chrome/label i18n regression suite
 * (code-review-mobile-rn-hardcoded-i18n-screens).
 *
 * A code-review sweep found that several React Native screens still rendered
 * user-facing copy as hardcoded English literals — screen titles/subtitles,
 * back-links, stat labels, section headings and small interpolated bits
 * ("Updated <date>", "Due <date>", "{n} unread", "By <author>") — even though
 * react-i18next was already wired on each screen. A Slovak/German/etc. user saw
 * English where the rest of the screen was localized.
 *
 * This suite locks the fix. Two layers (mirroring FaultsListScreen.i18n.test):
 *  1. Render layer — `src/test/setup.ts` mocks react-i18next so `t(key) => key`
 *     (honouring an explicit `defaultValue`). We assert each screen renders the
 *     i18n KEY and that the OLD hardcoded literal is gone. On the pre-fix source
 *     the literal rendered and the key did not, so these assertions fail.
 *  2. Resource layer — every locale JSON carries the new keys with non-empty,
 *     locale-distinct values, proving the keys resolve to real translations.
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../locales/cs.json';
import de from '../locales/de.json';
import en from '../locales/en.json';
import hu from '../locales/hu.json';
import pl from '../locales/pl.json';
import sk from '../locales/sk.json';

import { AnnouncementsScreen } from './announcements/AnnouncementsScreen';
import { BuildingsScreen } from './buildings/BuildingsScreen';
import { DocumentDetailScreen } from './documents/DocumentDetailScreen';
import { FormsScreen } from './forms/FormsScreen';
import { LeaseDetailScreen } from './leases/LeaseDetailScreen';
import { LeasesScreen } from './leases/LeasesScreen';
import { MessagesScreen } from './messages/MessagesScreen';
import { MetersScreen } from './meters/MetersScreen';

jest.mock('../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useTenantId: jest.fn(() => ({ tenantId: 't1' })),
  apiRequest: jest.fn(),
}));

jest.mock('./meters/useMyMeters', () => ({
  useMyMeters: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../hooks/useApi').useApiQuery as jest.Mock;
const mockUseMyMeters = jest.requireMock('./meters/useMyMeters').useMyMeters as jest.Mock;

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

beforeEach(() => {
  jest.clearAllMocks();
  mockUseApiQuery.mockReturnValue(queryResult({}));
  mockUseMyMeters.mockReturnValue(queryResult({ isLoading: true }));
});

describe('BuildingsScreen chrome i18n', () => {
  it('renders header + stat labels via i18n keys, not literals', async () => {
    mockUseApiQuery.mockReturnValue(
      queryResult({
        data: {
          buildings: [
            {
              id: 'b1',
              name: 'Main',
              street: 'Main St 1',
              city: 'Bratislava',
              postal_code: '81101',
              total_floors: 4,
              unit_count: 12,
              status: 'active',
            },
          ],
        },
      })
    );
    await render(<BuildingsScreen />);

    expect(screen.getByText('buildings.title')).toBeTruthy();
    expect(screen.getByText('buildings.subtitle')).toBeTruthy();
    expect(screen.getByText('buildings.units')).toBeTruthy();
    expect(screen.getByText('buildings.floors')).toBeTruthy();

    expect(screen.queryByText('Buildings')).toBeNull();
    expect(screen.queryByText('Properties associated with your account.')).toBeNull();
    expect(screen.queryByText('Units')).toBeNull();
    expect(screen.queryByText('Floors')).toBeNull();
  });
});

describe('MetersScreen chrome i18n', () => {
  it('renders header + subtitle via i18n keys, not literals', async () => {
    await render(<MetersScreen />);
    expect(screen.getByText('meters.listTitle')).toBeTruthy();
    expect(screen.getByText('meters.listSubtitle')).toBeTruthy();
    expect(screen.queryByText('Meters')).toBeNull();
    expect(screen.queryByText('Meters registered to your unit.')).toBeNull();
  });
});

describe('MessagesScreen chrome i18n', () => {
  it('renders header + thread unread badge via i18n keys, not literals', async () => {
    mockUseApiQuery.mockReturnValue(
      queryResult({
        data: {
          threads: [
            {
              id: 'th1',
              participants: [],
              unreadCount: 3,
              updatedAt: '2026-01-02T10:00:00Z',
              lastMessage: { content: 'Hi', createdAt: '2026-01-02T10:00:00Z' },
            },
          ],
        },
      })
    );
    await render(<MessagesScreen />);
    expect(screen.getByText('messages.title')).toBeTruthy();
    expect(screen.getByText('messages.threadUnread')).toBeTruthy();
    expect(screen.queryByText('Messages')).toBeNull();
    expect(screen.queryByText('3 new')).toBeNull();
  });
});

describe('AnnouncementsScreen chrome i18n', () => {
  it('renders title, pinned band + author line via i18n keys, not literals', async () => {
    mockUseApiQuery.mockReturnValue(
      queryResult({
        data: {
          announcements: [
            {
              id: 'a1',
              title: 'Pinned one',
              status: 'published',
              target_type: 'all',
              published_at: '2026-01-02T10:00:00Z',
              pinned: true,
              comments_enabled: false,
              acknowledgment_required: false,
            },
            {
              id: 'a2',
              title: 'Feed one',
              status: 'published',
              target_type: 'all',
              published_at: '2026-01-01T10:00:00Z',
              pinned: false,
              comments_enabled: false,
              acknowledgment_required: false,
            },
          ],
        },
      })
    );
    await render(<AnnouncementsScreen />);
    expect(screen.getByText('announcements.title')).toBeTruthy();
    expect(screen.getByText('announcements.pinned')).toBeTruthy();
    expect(screen.getAllByText('announcements.byLine').length).toBeGreaterThan(0);

    expect(screen.queryByText('Announcements')).toBeNull();
    expect(screen.queryByText('Pinned')).toBeNull();
  });
});

describe('LeasesScreen chrome i18n', () => {
  it('renders header + subtitle via i18n keys, not literals', async () => {
    await render(<LeasesScreen />);
    expect(screen.getByText('leases.title')).toBeTruthy();
    expect(screen.getByText('leases.subtitle')).toBeTruthy();
    expect(screen.queryByText('Leases')).toBeNull();
    expect(screen.queryByText('Your active and past rental agreements.')).toBeNull();
  });
});

describe('LeaseDetailScreen chrome i18n', () => {
  it('renders back-link, metrics + sections via i18n keys, not literals', async () => {
    mockUseApiQuery.mockReturnValue(
      queryResult({
        data: {
          lease: {
            id: 'l1',
            monthly_rent: '500',
            start_date: '2026-01-01',
            end_date: '2026-12-31',
            status: 'active',
            landlord_name: 'ACME Realty',
          },
          unit_name: 'Unit 1',
          building_name: 'Building A',
          upcoming_payments: [{ id: 'p1', amount: '500', due_date: '2026-02-01' }],
        },
      })
    );
    await render(<LeaseDetailScreen leaseId="l1" onBack={jest.fn()} />);

    expect(screen.getByText('leases.backToList')).toBeTruthy();
    expect(screen.getByText('leases.monthlyRent')).toBeTruthy();
    // asRole + paidTo render as adjacent children of one <Text> ("As …, paid
    // to …"), so the text node content is the concatenation of both keys.
    expect(screen.getByText(/leases\.asRole/)).toBeTruthy();
    expect(screen.getByText(/leases\.paidTo/)).toBeTruthy();
    expect(screen.getByText('leases.upcomingPayments')).toBeTruthy();
    expect(screen.getByText('leases.due')).toBeTruthy();
    expect(screen.getByText('leases.documentsSection')).toBeTruthy();
    // Rendered as "📄 {key}" in one <Text>, so match on substring.
    expect(screen.getByText(/leases\.leaseAgreement/)).toBeTruthy();
    expect(screen.getByText('leases.signedPdfHint')).toBeTruthy();

    expect(screen.queryByText('← Leases')).toBeNull();
    expect(screen.queryByText('Monthly rent')).toBeNull();
    expect(screen.queryByText('Upcoming payments')).toBeNull();
    expect(screen.queryByText('Signed PDF — open to review or share.')).toBeNull();
  });
});

describe('FormsScreen chrome i18n', () => {
  it('renders header, required badge + due label via i18n keys, not literals', async () => {
    mockUseApiQuery.mockReturnValue(
      queryResult({
        data: {
          forms: [
            {
              id: 'f1',
              title: 'Annual review',
              description: 'Please sign',
              require_signatures: true,
              submission_deadline: '2026-03-01',
            },
          ],
        },
      })
    );
    await render(<FormsScreen />);
    expect(screen.getByText('forms.title')).toBeTruthy();
    expect(screen.getByText('forms.required')).toBeTruthy();
    expect(screen.getByText('forms.due')).toBeTruthy();

    expect(screen.queryByText('Forms')).toBeNull();
    expect(screen.queryByText('Required')).toBeNull();
  });
});

describe('DocumentDetailScreen chrome i18n', () => {
  it('renders back-link, field labels + updated meta via i18n keys, not literals', async () => {
    mockUseApiQuery.mockReturnValue(
      queryResult({
        data: {
          document: {
            id: 'd1',
            title: 'Contract',
            category: 'Legal',
            file_name: 'contract.pdf',
            mime_type: 'application/pdf',
            size_bytes: 2048,
            created_at: '2026-01-01T10:00:00Z',
            updated_at: '2026-01-02T10:00:00Z',
          },
        },
      })
    );
    await render(<DocumentDetailScreen documentId="d1" onBack={jest.fn()} />);

    expect(screen.getByText('documents.backToList')).toBeTruthy();
    expect(screen.getByText('documents.detailCategory')).toBeTruthy();
    expect(screen.getByText('documents.detailFile')).toBeTruthy();
    expect(screen.getByText('documents.detailCreated')).toBeTruthy();
    expect(screen.getByText('documents.updatedAt')).toBeTruthy();

    expect(screen.queryByText('← Documents')).toBeNull();
    expect(screen.queryByText('Category')).toBeNull();
    expect(screen.queryByText('File')).toBeNull();
    expect(screen.queryByText('Created')).toBeNull();
  });
});

describe('new chrome keys resolve to real, locale-distinct translations', () => {
  const locales = { sk, cs, de, pl, hu } as const;

  // key path (namespace.key) → expected English copy.
  const EN_EXPECTED: Record<string, string> = {
    'buildings.title': 'Buildings',
    'buildings.subtitle': 'Properties associated with your account.',
    'buildings.units': 'Units',
    'meters.listTitle': 'Meters',
    'meters.listSubtitle': 'Meters registered to your unit.',
    'messages.title': 'Messages',
    'messages.threadUnread': '{{count}} new',
    'announcements.unreadBadge': '{{count}} unread',
    'announcements.byLine': 'By {{author}}',
    'leases.title': 'Leases',
    'leases.subtitle': 'Your active and past rental agreements.',
    'leases.detailTitle': 'Lease',
    'leases.monthlyRent': 'Monthly rent',
    'leases.upcomingPayments': 'Upcoming payments',
    'leases.documentsSection': 'Documents',
    'leases.signedPdfHint': 'Signed PDF — open to review or share.',
    'leases.backToList': '← Leases',
    'leases.due': 'Due {{date}}',
    'leases.asRole': 'As {{role}}',
    'leases.paidTo': ', paid to {{name}}',
    'leases.leaseAgreement': 'Lease agreement',
    'forms.title': 'Forms',
    'forms.required': 'Required',
    'forms.due': 'Due {{date}}',
    'documents.detailCategory': 'Category',
    'documents.detailFile': 'File',
    'documents.detailCreated': 'Created',
    'documents.backToList': '← Documents',
    'documents.updatedAt': 'Updated {{date}}',
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
        // A real translation, not an English copy left behind. Interpolation
        // placeholders ({{count}}/{{date}}/…) are preserved, so compare the
        // whole string — the non-placeholder words must differ from English.
        expect(`${name}:${value}`).not.toBe(`${name}:${EN_EXPECTED[path]}`);
      }
    }
  });
});
