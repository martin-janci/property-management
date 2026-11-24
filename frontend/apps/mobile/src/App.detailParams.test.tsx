/**
 * MainApp detail-screen id wiring.
 *
 * The list screens call `onNavigate('LeaseDetail', { leaseId })`,
 * `('MeterDetail', { meterId })`, `('MeterReading', { meterId })` and
 * `('ThreadDetail', { threadId })`, but the `App.tsx` switch rendered those
 * screens without forwarding the id. Each of them keeps its query disabled and
 * shows "Loading" while the id is missing, so every lease, meter and message
 * thread opened onto a permanent loading state.
 *
 * Drives the real `handleNavigate` and asserts each screen receives its id.
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
    ArticleDetailScreen: () => null,
    BuildingsScreen: () => null,
    DocumentDetailScreen: () => null,
    DocumentPermissionsScreen: () => null,
    DocumentPreviewScreen: () => null,
    DocumentsScreen: () => null,
    DocumentUploadScreen: () => null,
    FaultsListScreen: () => null,
    FormsScreen: () => null,
    LeaseDetailScreen: ({ leaseId }: { leaseId?: string }) => (
      <RNText>{`lease-detail:${leaseId}`}</RNText>
    ),
    LeaseSignatureScreen: () => null,
    LeasesScreen: () => null,
    MessagesScreen: () => null,
    MeterDetailScreen: ({ meterId }: { meterId?: string }) => (
      <RNText>{`meter-detail:${meterId}`}</RNText>
    ),
    MeterReadingScreen: ({ meterId }: { meterId?: string }) => (
      <RNText>{`meter-reading:${meterId}`}</RNText>
    ),
    MetersScreen: () => null,
    NeighborsScreen: () => null,
    // List screens render identifiable text so back-navigation lands are observable.
    NewsScreen: () => <RNText>{'news-list'}</RNText>,
    NotificationsScreen: () => null,
    OutageDetailScreen: () => null,
    OutagesScreen: () => <RNText>{'outages-list'}</RNText>,
    PersonMonthsScreen: () => null,
    ProfileScreen: () => null,
    ReportFaultScreen: () => null,
    ThreadDetailScreen: ({ threadId }: { threadId?: string }) => (
      <RNText>{`thread-detail:${threadId}`}</RNText>
    ),
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

async function navigate(screen: string, params?: Record<string, unknown>) {
  if (!capturedNavigate) {
    throw new Error('onNavigate was never captured — DashboardScreen did not render');
  }
  await act(() => {
    capturedNavigate!(screen, params);
  });
}

describe('MainApp detail screens receive their id param', () => {
  beforeEach(() => {
    capturedNavigate = undefined;
    jest.clearAllMocks();
  });

  it.each([
    ['LeaseDetail', { leaseId: 'l-1' }, 'lease-detail:l-1'],
    ['MeterDetail', { meterId: 'm-1' }, 'meter-detail:m-1'],
    ['MeterReading', { meterId: 'm-2' }, 'meter-reading:m-2'],
    ['ThreadDetail', { threadId: 't-1' }, 'thread-detail:t-1'],
  ])('%s forwards its id', async (screen, params, expected) => {
    const { getByText } = await render(<App />);
    expect(getByText('dashboard')).toBeTruthy();

    await navigate(screen, params);

    expect(getByText(expected)).toBeTruthy();
  });
});
