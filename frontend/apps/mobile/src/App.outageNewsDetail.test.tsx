/**
 * MainApp Outage/News detail navigation wiring tests
 * (code-review-mobile-rn-outage-news-detail-dead-nav).
 *
 * `OutagesScreen` cards call `onNavigate('OutageDetail', { outageId })` and
 * `NewsScreen` cards call `onNavigate('ArticleDetail', { articleId })`. Those
 * two screen names had no `case` in the `App.tsx` render switch, so every tap
 * fell through to the `default` branch, silently rendered the Dashboard, and
 * discarded the params (dead navigation).
 *
 * This suite drives the real `handleNavigate` (captured from a rendered screen)
 * to each detail route and asserts the matching detail screen renders with the
 * id param preserved — NOT the Dashboard fallback. A regression that drops
 * either `case` makes the detail screen disappear and these assertions fail.
 *
 * Render-wiring test (mirrors App.authgate.test.tsx): the screens barrel is
 * mocked so no real screen/query mounts.
 */

import { act, render } from '@testing-library/react-native';
import type React from 'react';

// Captured from whichever mocked screen last received the `onNavigate` prop.
// App passes the real `handleNavigate` to every list screen, so calling this
// drives the real navigation state machine.
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

// ── Screens — mock the barrel. DashboardScreen captures `onNavigate`; the two
// detail screens render their id param so param preservation is observable.
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
    ArticleDetailScreen: ({ articleId }: { articleId?: string }) => (
      <RNText>{`article-detail:${articleId}`}</RNText>
    ),
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
    OutageDetailScreen: ({ outageId }: { outageId?: string }) => (
      <RNText>{`outage-detail:${outageId}`}</RNText>
    ),
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

// ── Expo / i18n / query client ──────────────────────────────────────────────
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

function navigate(screen: string, params?: Record<string, unknown>) {
  if (!capturedNavigate) {
    throw new Error('onNavigate was never captured — DashboardScreen did not render');
  }
  act(() => {
    // biome-ignore lint/style/noNonNullAssertion: guarded above
    capturedNavigate!(screen, params);
  });
}

describe('MainApp Outage/News detail navigation wiring', () => {
  beforeEach(() => {
    capturedNavigate = undefined;
    jest.clearAllMocks();
  });

  it('resolves OutageDetail to OutageDetailScreen with the outageId param', () => {
    const { getByText, queryByText } = render(<App />);
    expect(getByText('dashboard')).toBeTruthy();

    navigate('OutageDetail', { outageId: 'o-1' });

    // Route resolved to the detail screen with the param preserved — NOT the
    // Dashboard fallback that the missing `case` used to produce.
    expect(getByText('outage-detail:o-1')).toBeTruthy();
    expect(queryByText('dashboard')).toBeNull();
  });

  it('resolves ArticleDetail to ArticleDetailScreen with the articleId param', () => {
    const { getByText, queryByText } = render(<App />);
    expect(getByText('dashboard')).toBeTruthy();

    navigate('ArticleDetail', { articleId: 'a-1' });

    expect(getByText('article-detail:a-1')).toBeTruthy();
    expect(queryByText('dashboard')).toBeNull();
  });
});
