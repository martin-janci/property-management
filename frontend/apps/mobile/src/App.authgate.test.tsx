/**
 * MainApp auth-gate wiring tests (AC-5, GH #1411).
 *
 * The pure `resolveAuthGate` decision is covered in authGate.test.ts.
 * This suite covers the complementary gap: that MainApp actually wires the
 * three gate outcomes to the right rendered subtrees.
 *
 * A regression that swaps the 'auth' / 'protected' branches, or drops the
 * 'loading' short-circuit, would still pass authGate.test.ts but will fail
 * here.
 */

import { render } from '@testing-library/react-native';
import type React from 'react';

// ── Auth context ──────────────────────────────────────────────────────────────
const mockUseAuth = jest.fn();
jest.mock('./contexts', () => ({
  AuthProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  useAuth: () => mockUseAuth(),
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

// ── Screens — mock the entire screens barrel; each is a no-op component.
// AuthFlow and DashboardScreen render distinct testIDs so the tests can tell
// the auth vs protected gate branches apart.
jest.mock('./screens', () => ({
  AuthFlow: () => {
    const { View } = require('react-native');
    return <View testID="mock-auth-flow" />;
  },
  TwoFactorScreen: () => null,
  DashboardScreen: () => {
    const { View } = require('react-native');
    return <View testID="mock-dashboard" />;
  },
  AnnouncementsScreen: () => null,
  AnnouncementDetailScreen: () => null,
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
  OutagesScreen: () => null,
  PersonMonthsScreen: () => null,
  ProfileScreen: () => null,
  ReportFaultScreen: () => null,
  ThreadDetailScreen: () => null,
  VoteDetailScreen: () => null,
  VotingScreen: () => null,
  WidgetSettingsScreen: () => null,
}));

// ── Expo / i18n ───────────────────────────────────────────────────────────────
jest.mock('expo-status-bar', () => ({ StatusBar: () => null }));
jest.mock('expo-constants', () => ({
  default: { expoConfig: { extra: { apiUrl: 'http://localhost:8080' } } },
}));
jest.mock('@tanstack/react-query', () => ({
  QueryClient: jest.fn(() => ({})),
  QueryClientProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

// ── Subject under test ────────────────────────────────────────────────────────
// Import after mocks are wired.
// eslint-disable-next-line import/first
import App from './App';

// ─────────────────────────────────────────────────────────────────────────────

async function renderApp() {
  return await render(<App />);
}

describe('MainApp auth-gate wiring (AC-5)', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders the loading view while session is being restored', async () => {
    mockUseAuth.mockReturnValue({ isLoading: true, isAuthenticated: false });
    const { getByText } = await renderApp();
    // The loading branch renders t('common.loading') — the i18n mock returns the key.
    expect(getByText('common.loading')).toBeTruthy();
  });

  it('renders AuthFlow for an unauthenticated session (not the dashboard)', async () => {
    mockUseAuth.mockReturnValue({ isLoading: false, isAuthenticated: false });
    const { queryByTestId } = await renderApp();
    // Auth branch: AuthFlow mock is rendered; DashboardScreen is not.
    expect(queryByTestId('mock-auth-flow')).toBeTruthy();
    expect(queryByTestId('mock-dashboard')).toBeNull();
    // And the loading text should not be present.
    expect(queryByTestId('loading-view')).toBeNull();
  });

  it('renders the dashboard (protected) for an authenticated session, not AuthFlow', async () => {
    mockUseAuth.mockReturnValue({ isLoading: false, isAuthenticated: true });
    const { queryByTestId } = await renderApp();
    expect(queryByTestId('mock-dashboard')).toBeTruthy();
    expect(queryByTestId('mock-auth-flow')).toBeNull();
  });

  it('never shows protected content while still loading (even if isAuthenticated=true)', async () => {
    // Simulates the race condition: token restored from storage but loading flag still true.
    mockUseAuth.mockReturnValue({ isLoading: true, isAuthenticated: true });
    const { getByText, queryByTestId } = await renderApp();
    expect(getByText('common.loading')).toBeTruthy();
    expect(queryByTestId('mock-dashboard')).toBeNull();
    expect(queryByTestId('mock-auth-flow')).toBeNull();
  });
});
