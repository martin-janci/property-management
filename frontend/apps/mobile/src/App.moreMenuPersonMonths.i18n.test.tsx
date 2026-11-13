/**
 * MoreMenu "Person-months" label i18n regression test
 * (code-review-mobile-rn-person-months-no-i18n).
 *
 * The bottom-tab bar labels already go through `t('tabs.*')`, but the overflow
 * "More" menu (`MORE_ITEMS` in App.tsx) rendered every row label — including
 * "Person-months" — as a hardcoded English literal. This suite locks the fix
 * for the person-months row: its label is now sourced via `t()` from the
 * already-localised `personMonths.title` key (the PersonMonthsScreen header
 * copy, which is exactly the nav label and is translated in every locale).
 *
 * Render harness mirrors App.outageNewsDetail.test.tsx: the screens barrel and
 * app-shell deps are mocked, DashboardScreen captures the real `handleNavigate`,
 * and we drive it to the "More" route so the real MoreMenu renders. The global
 * react-i18next mock (src/test/setup.ts) returns the key verbatim (`t(k) => k`),
 * so the row renders the KEY when wired through i18n and the raw literal when not.
 */

import { act, render } from '@testing-library/react-native';
import type React from 'react';
import cs from './locales/cs.json';
import de from './locales/de.json';
import en from './locales/en.json';
import hu from './locales/hu.json';
import pl from './locales/pl.json';
import sk from './locales/sk.json';

let capturedNavigate: ((screen: string, params?: Record<string, unknown>) => void) | undefined;
const capture = (onNavigate?: (screen: string, params?: Record<string, unknown>) => void) => {
  capturedNavigate = onNavigate;
};

// ── Auth context — always authenticated so the protected switch renders ──────
jest.mock('./contexts', () => ({
  AuthProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useAuth: () => ({ isLoading: false, isAuthenticated: true }),
}));

// ── Offline / push / deep-link hooks ─────────────────────────────────────────
jest.mock('./hooks', () => ({
  useOfflineSupport: () => ({
    isConnected: true,
    queuedActionsCount: 0,
    isSyncing: false,
    syncProgress: null,
    processQueue: jest.fn(),
    permanentFailures: [],
    clearPermanentFailures: jest.fn(),
  }),
  usePushNotifications: () => ({ registerForPushNotifications: jest.fn() }),
  useDeepLinkRouting: jest.fn(),
}));

// ── Sync UI components ────────────────────────────────────────────────────────
jest.mock('./components/sync', () => ({
  OfflineBanner: () => null,
  SyncProgressToast: () => null,
  SyncStatusBadge: () => null,
}));

// ── Screens — mock the barrel. DashboardScreen captures `onNavigate`. ─────────
jest.mock('./screens', () => {
  const { Text: RNText } = require('react-native');
  return {
    AuthFlow: () => null,
    TwoFactorScreen: () => null,
    DashboardScreen: ({
      onNavigate,
    }: {
      onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
    }) => {
      capture(onNavigate);
      return <RNText>dashboard</RNText>;
    },
    AnnouncementsScreen: () => null,
    AnnouncementDetailScreen: () => null,
    ArticleDetailScreen: () => null,
    BuildingsScreen: () => null,
    DocumentDetailScreen: () => null,
    DocumentPermissionsScreen: () => null,
    DocumentPreviewScreen: () => null,
    DocumentsScreen: () => null,
    DocumentUploadScreen: () => null,
    FaultsListScreen: () => null,
    FormsScreen: () => null,
    LeaseDetailScreen: () => null,
    LeaseSignatureScreen: () => null,
    LeasesScreen: () => null,
    MessagesScreen: () => null,
    MeterDetailScreen: () => null,
    MeterReadingScreen: () => null,
    MetersScreen: () => null,
    NeighborsScreen: () => null,
    NewsScreen: () => null,
    NotificationsScreen: () => null,
    OutageDetailScreen: () => null,
    OutagesScreen: () => null,
    PersonMonthsScreen: () => null,
    ProfileScreen: () => null,
    ReportFaultScreen: () => null,
    ThreadDetailScreen: () => null,
    VoteDetailScreen: () => null,
    VotingScreen: () => null,
    WidgetSettingsScreen: () => null,
  };
});

// ── Expo / query client ──────────────────────────────────────────────────────
jest.mock('expo-status-bar', () => ({ StatusBar: () => null }));
jest.mock('expo-constants', () => ({
  default: { expoConfig: { extra: { apiUrl: 'http://localhost:8080' } } },
}));
jest.mock('@tanstack/react-query', () => ({
  QueryClient: jest.fn(() => ({})),
  QueryClientProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// Import after mocks are wired.
// eslint-disable-next-line import/first
import App from './App';

async function navigate(screen: string) {
  if (!capturedNavigate) {
    throw new Error('onNavigate was never captured — DashboardScreen did not render');
  }
  await act(() => {
    capturedNavigate!(screen);
  });
}

describe('MoreMenu person-months label i18n', () => {
  beforeEach(() => {
    capturedNavigate = undefined;
    jest.clearAllMocks();
  });

  it('renders the person-months row via the personMonths.title key, not a hardcoded literal', async () => {
    const { getByText, queryByText } = await render(<App />);
    expect(getByText('dashboard')).toBeTruthy();

    await navigate('More');

    // t(key) => key under the global mock, so the i18n-wired row renders the KEY.
    expect(getByText('personMonths.title')).toBeTruthy();
    // The old hardcoded English literal must no longer be rendered.
    expect(queryByText('Person-months')).toBeNull();
  });
});

describe('personMonths.title (the reused nav label) resolves in every locale', () => {
  it('en carries the expected English label', () => {
    expect(en.personMonths.title).toBe('Person-months');
  });

  it('every locale translates the label away from the raw English literal', () => {
    for (const locale of [sk, cs, de, pl, hu]) {
      const value = locale.personMonths.title;
      expect(typeof value).toBe('string');
      expect(value.length).toBeGreaterThan(0);
      expect(value).not.toBe('Person-months');
    }
  });
});
