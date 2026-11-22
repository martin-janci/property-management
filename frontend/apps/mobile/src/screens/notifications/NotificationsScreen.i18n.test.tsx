/**
 * NotificationsScreen — header / filter-chip / state i18n regression test
 * (code-review-mobile-rn-notifications-no-i18n; frontend-i18n burn-down for the
 * 2026-12-01 no-hardcoded-strings lint flip).
 *
 * The screen rendered its header ("Notifications", "{n} unread"), the category
 * filter chips ("All", "Faults", …), the loading / error / empty states
 * ("Loading…", "Couldn't load notifications", "Try again", "Mark all as read",
 * "You're all caught up", "New notifications will appear here.") and the
 * relative "time ago" labels as hardcoded English literals — react-i18next was
 * never wired. On the 6-locale mobile app a Slovak/Czech/German/… user saw
 * English chrome throughout the inbox.
 *
 * This suite locks the contract. Mirrors OutagesScreen.i18n.test.tsx:
 *  1. Render layer — `src/test/setup.ts` mocks react-i18next so `t(key) => key`.
 *     We assert the screen renders the i18n KEYS and that the OLD hardcoded
 *     English literals are gone.
 *  2. Resource layer — the locale JSON files carry translated, locale-distinct
 *     values for those keys.
 */

import type { InboxResponse } from '@ppt/api-client';
import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { NotificationsScreen } from './NotificationsScreen';

jest.mock('../../hooks/useApi', () => ({
  useApiQuery: jest.fn(),
  useApiMutation: jest.fn(),
}));

const mockUseApiQuery = jest.requireMock('../../hooks/useApi').useApiQuery as jest.Mock;
const mockUseApiMutation = jest.requireMock('../../hooks/useApi').useApiMutation as jest.Mock;

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

// The old hardcoded header / chip / state literals that must no longer render.
const HARDCODED = [
  'Notifications',
  'All',
  'Faults',
  'Announcements',
  'Messages',
  'Voting',
  'Meters',
  "Couldn't load notifications",
  'Mark all as read',
  "You're all caught up",
  'New notifications will appear here.',
];

const LOCALES = { sk, cs, de, pl, hu } as const;
const FILTER_KEYS = ['all', 'fault', 'announcement', 'message', 'vote', 'meter'] as const;

describe('NotificationsScreen header + filter-chip i18n', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseApiMutation.mockReturnValue({ mutate: jest.fn(), isPending: false });
    mockQuery({ data: { groups: [], totalUnread: 0 } satisfies InboxResponse });
  });

  it('renders the header title key, not the hardcoded literal', async () => {
    await render(<NotificationsScreen />);
    expect(screen.getByText('notifications.title')).toBeTruthy();
  });

  it('renders filter-chip keys, not hardcoded literals', async () => {
    await render(<NotificationsScreen />);
    for (const key of FILTER_KEYS) {
      expect(screen.getByText(`notifications.filters.${key}`)).toBeTruthy();
    }
  });

  it('renders the empty-state via i18n keys', async () => {
    await render(<NotificationsScreen />);
    expect(screen.getByText('notifications.emptyTitle')).toBeTruthy();
    expect(screen.getByText('notifications.emptyText')).toBeTruthy();
  });

  it('renders the error state via i18n keys', async () => {
    mockQuery({ error: new Error('boom') });
    await render(<NotificationsScreen />);
    expect(screen.getByText('notifications.loadError')).toBeTruthy();
    expect(screen.getByText('common.tryAgain')).toBeTruthy();
  });

  it('no longer renders any hardcoded English header/chip/state literal', async () => {
    await render(<NotificationsScreen />);
    for (const literal of HARDCODED) {
      expect(screen.queryByText(literal)).toBeNull();
    }
  });
});

describe('notifications keys resolve to real, locale-distinct translations', () => {
  it('en carries the expected English copy', () => {
    expect(en.notifications.title).toBe('Notifications');
    expect(en.notifications.emptyTitle).toBe("You're all caught up");
    expect(en.notifications.filters.all).toBe('All');
    expect(en.tabs.more).toBe('More');
  });

  it('every locale defines the notification keys and translates them away from English', () => {
    for (const locale of Object.values(LOCALES)) {
      for (const key of ['title', 'loadError', 'markAllRead', 'emptyTitle', 'emptyText'] as const) {
        expect(typeof locale.notifications[key]).toBe('string');
        expect(locale.notifications[key].length).toBeGreaterThan(0);
        expect(locale.notifications[key]).not.toBe(en.notifications[key]);
      }
      for (const key of FILTER_KEYS) {
        expect(typeof locale.notifications.filters[key]).toBe('string');
        expect(locale.notifications.filters[key].length).toBeGreaterThan(0);
      }
      for (const key of ['minutesAgo', 'hoursAgo', 'daysAgo'] as const) {
        // interpolation token must survive translation
        expect(locale.notifications.time[key]).toContain('{{count}}');
      }
      expect(typeof locale.tabs.more).toBe('string');
      expect(locale.tabs.more.length).toBeGreaterThan(0);
    }
  });

  it('en time + unread templates keep the {{count}} interpolation token', () => {
    expect(en.notifications.unreadCount).toContain('{{count}}');
    expect(en.notifications.time.minutesAgo).toContain('{{count}}');
    expect(en.notifications.time.hoursAgo).toContain('{{count}}');
    expect(en.notifications.time.daysAgo).toContain('{{count}}');
  });
});
