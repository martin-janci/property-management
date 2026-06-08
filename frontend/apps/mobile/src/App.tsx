import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import Constants from 'expo-constants';
import { StatusBar } from 'expo-status-bar';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { OfflineBanner, SyncProgressToast, SyncStatusBadge } from './components/sync';
import { AuthProvider, useAuth } from './contexts';
import { useDeepLinkRouting, useOfflineSupport, usePushNotifications } from './hooks';
import { colors } from './screens/shared/screenStyles';
import type { Screen } from './services/deepLinkRouting';
import './i18n'; // Initialize i18n
import {
  AnnouncementDetailScreen,
  AnnouncementsScreen,
  AuthFlow,
  BuildingsScreen,
  DashboardScreen,
  DocumentDetailScreen,
  DocumentPermissionsScreen,
  DocumentPreviewScreen,
  DocumentsScreen,
  DocumentUploadScreen,
  FaultsListScreen,
  FormsScreen,
  LeaseDetailScreen,
  LeaseSignatureScreen,
  LeasesScreen,
  MessagesScreen,
  MeterDetailScreen,
  MeterReadingScreen,
  MetersScreen,
  NeighborsScreen,
  NewsScreen,
  NotificationsScreen,
  OutagesScreen,
  PersonMonthsScreen,
  ProfileScreen,
  ReportFaultScreen,
  ThreadDetailScreen,
  TwoFactorScreen,
  VoteDetailScreen,
  VotingScreen,
} from './screens';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5 minutes
      gcTime: 1000 * 60 * 30, // 30 minutes (formerly cacheTime)
      retry: 3,
    },
  },
});

const API_BASE_URL = (Constants.expoConfig?.extra?.apiUrl as string) || 'http://localhost:8080';

function MainApp() {
  const { t } = useTranslation();
  const { isAuthenticated, isLoading } = useAuth();
  const { isConnected, queuedActionsCount, isSyncing, syncProgress, processQueue } =
    useOfflineSupport();
  const { registerForPushNotifications } = usePushNotifications();
  const [showSyncToast, setShowSyncToast] = useState(false);

  // Register this device's push token with the backend once authenticated.
  // Without this the registration hook was never invoked, so no device ever
  // received pushes. Errors are surfaced inside the hook and ignored here —
  // a missing push token must not block the app. Keyed on `isAuthenticated`
  // only: `registerForPushNotifications` is re-created each render, so adding
  // it to the deps would re-register on every render.
  // biome-ignore lint/correctness/useExhaustiveDependencies: register once per auth transition; the callback is intentionally excluded
  useEffect(() => {
    if (isAuthenticated) {
      void registerForPushNotifications();
    }
  }, [isAuthenticated]);

  // Show sync toast when sync progress starts
  const handleRetrySync = useCallback(() => {
    processQueue();
  }, [processQueue]);

  // Show toast when sync is in progress
  const isSyncingWithProgress = isSyncing && syncProgress !== null;
  const [currentScreen, setCurrentScreen] = useState<Screen>('Dashboard');
  const [screenParams, setScreenParams] = useState<Record<string, unknown> | undefined>(undefined);

  const handleNavigate = useCallback((screen: string, params?: Record<string, unknown>) => {
    setCurrentScreen(screen as Screen);
    setScreenParams(params);
  }, []);

  // gap-85-3: route incoming deep links (custom scheme + universal/App Links).
  // Wiring lives in `useDeepLinkRouting`: it handles the cold-start initial
  // URL, runtime `url` events, and the auth gate that queues auth-required
  // links until the user is authenticated.
  useDeepLinkRouting(handleNavigate, isAuthenticated);

  if (isLoading) {
    return (
      <View style={styles.loadingContainer}>
        <Text style={styles.loadingText}>{t('common.loading')}</Text>
      </View>
    );
  }

  if (!isAuthenticated) {
    return <AuthFlow />;
  }

  const renderScreen = () => {
    switch (currentScreen) {
      case 'Dashboard':
        return <DashboardScreen onNavigate={handleNavigate} />;
      case 'Faults':
        return <FaultsListScreen onNavigate={handleNavigate} />;
      case 'ReportFault':
        return (
          <ReportFaultScreen
            onSuccess={() => handleNavigate('Faults')}
            onCancel={() => handleNavigate('Faults')}
          />
        );
      case 'Announcements':
        return <AnnouncementsScreen onNavigate={handleNavigate} />;
      case 'AnnouncementDetail': {
        const announcementId = screenParams?.announcementId as string | undefined;
        if (!announcementId) {
          handleNavigate('Announcements');
          return null;
        }
        return (
          <AnnouncementDetailScreen
            announcementId={announcementId}
            onBack={() => handleNavigate('Announcements')}
          />
        );
      }
      case 'Voting':
        return <VotingScreen onNavigate={handleNavigate} />;
      case 'VoteDetail':
        return (
          <VoteDetailScreen
            voteId={screenParams?.voteId as string | undefined}
            onBack={() => handleNavigate('Voting')}
          />
        );
      case 'Documents':
        return <DocumentsScreen onNavigate={handleNavigate} />;
      case 'DocumentDetail':
        return (
          <DocumentDetailScreen
            documentId={screenParams?.documentId as string | undefined}
            onBack={() => handleNavigate('Documents')}
          />
        );
      case 'DocumentPreview':
        return screenParams?.document ? (
          <DocumentPreviewScreen
            document={
              screenParams.document as Parameters<typeof DocumentPreviewScreen>[0]['document']
            }
            onBack={() => handleNavigate('Documents')}
          />
        ) : (
          <DocumentsScreen onNavigate={handleNavigate} />
        );
      case 'DocumentUpload':
        return (
          <DocumentUploadScreen
            onSuccess={() => handleNavigate('Documents')}
            onCancel={() => handleNavigate('Documents')}
          />
        );
      case 'DocumentPermissions':
        return (
          <DocumentPermissionsScreen
            documentId={(screenParams?.documentId as string | undefined) ?? ''}
            onBack={() => handleNavigate('Documents')}
          />
        );
      case 'MeterReading':
        return (
          <MeterReadingScreen
            onSuccess={() => handleNavigate('Meters')}
            onCancel={() => handleNavigate('Meters')}
          />
        );
      case 'Meters':
        return <MetersScreen onNavigate={handleNavigate} />;
      case 'MeterDetail':
        return (
          <MeterDetailScreen onBack={() => handleNavigate('Meters')} onNavigate={handleNavigate} />
        );
      case 'Profile':
        return <ProfileScreen onTwoFactorPress={() => handleNavigate('TwoFactor')} />;
      case 'TwoFactor':
        return <TwoFactorScreen onDonePress={() => handleNavigate('Profile')} />;
      case 'Messages':
        return <MessagesScreen onNavigate={handleNavigate} />;
      case 'ThreadDetail':
        return <ThreadDetailScreen onBack={() => handleNavigate('Messages')} />;
      case 'Neighbors':
        return <NeighborsScreen onNavigate={handleNavigate} />;
      case 'Notifications':
        return <NotificationsScreen onNavigate={handleNavigate} />;
      case 'PersonMonths':
        return <PersonMonthsScreen onNavigate={handleNavigate} />;
      case 'Outages':
        return <OutagesScreen onNavigate={handleNavigate} />;
      case 'News':
        return <NewsScreen onNavigate={handleNavigate} />;
      case 'Forms':
        return <FormsScreen onNavigate={handleNavigate} />;
      case 'Buildings':
        return <BuildingsScreen onNavigate={handleNavigate} />;
      case 'Leases':
        return <LeasesScreen onNavigate={handleNavigate} />;
      case 'LeaseDetail':
        return (
          <LeaseDetailScreen onBack={() => handleNavigate('Leases')} onNavigate={handleNavigate} />
        );
      case 'LeaseSignature':
        return (
          <LeaseSignatureScreen
            esignatureId={screenParams?.esignatureId as string | undefined}
            onBack={() => handleNavigate('LeaseDetail')}
          />
        );
      case 'More':
        return <MoreMenu onNavigate={handleNavigate} />;
      default:
        return <DashboardScreen onNavigate={handleNavigate} />;
    }
  };

  return (
    <View style={styles.container}>
      {/* Offline banner with sync status */}
      <OfflineBanner
        isConnected={isConnected}
        queuedActionsCount={queuedActionsCount}
        isSyncing={isSyncing}
      />

      {/* Main content */}
      <View style={styles.content}>{renderScreen()}</View>

      {/* Bottom navigation */}
      <View style={styles.bottomNav}>
        <NavButton
          icon="🏠"
          label={t('tabs.home')}
          isActive={currentScreen === 'Dashboard'}
          onPress={() => handleNavigate('Dashboard')}
          syncBadge={
            queuedActionsCount > 0 ? (
              <SyncStatusBadge count={queuedActionsCount} size="small" isSyncing={isSyncing} />
            ) : undefined
          }
        />
        <NavButton
          icon="🔧"
          label={t('tabs.faults')}
          isActive={currentScreen === 'Faults' || currentScreen === 'ReportFault'}
          onPress={() => handleNavigate('Faults')}
        />
        <NavButton
          icon="📢"
          label={t('tabs.news')}
          isActive={currentScreen === 'Announcements'}
          onPress={() => handleNavigate('Announcements')}
        />
        <NavButton
          icon="🗳️"
          label={t('tabs.vote')}
          isActive={currentScreen === 'Voting' || currentScreen === 'VoteDetail'}
          onPress={() => handleNavigate('Voting')}
        />
        <NavButton
          icon="📄"
          label={t('tabs.docs')}
          isActive={
            currentScreen === 'Documents' ||
            currentScreen === 'DocumentDetail' ||
            currentScreen === 'DocumentPreview' ||
            currentScreen === 'DocumentPermissions'
          }
          onPress={() => handleNavigate('Documents')}
        />
        <NavButton
          icon="⋯"
          label="More"
          isActive={
            currentScreen === 'More' ||
            currentScreen === 'Messages' ||
            currentScreen === 'ThreadDetail' ||
            currentScreen === 'Neighbors' ||
            currentScreen === 'Notifications' ||
            currentScreen === 'PersonMonths' ||
            currentScreen === 'Outages' ||
            currentScreen === 'News' ||
            currentScreen === 'Forms' ||
            currentScreen === 'Buildings' ||
            currentScreen === 'Leases' ||
            currentScreen === 'LeaseDetail' ||
            currentScreen === 'LeaseSignature' ||
            currentScreen === 'Meters' ||
            currentScreen === 'MeterDetail' ||
            currentScreen === 'Profile' ||
            currentScreen === 'TwoFactor'
          }
          onPress={() => handleNavigate('More')}
        />
      </View>

      {/* Sync progress toast */}
      <SyncProgressToast
        visible={isSyncingWithProgress || showSyncToast}
        progress={
          syncProgress?.total
            ? Math.round(((syncProgress?.current || 0) / syncProgress.total) * 100)
            : 0
        }
        total={syncProgress?.total || 0}
        current={syncProgress?.current || 0}
        failed={syncProgress?.failed || 0}
        isComplete={syncProgress?.isComplete || false}
        onDismiss={() => setShowSyncToast(false)}
        onRetry={handleRetrySync}
      />

      <StatusBar style="auto" />
    </View>
  );
}

interface MoreMenuProps {
  onNavigate: (screen: string) => void;
}

const MORE_ITEMS: ReadonlyArray<{ icon: string; label: string; screen: string }> = [
  { icon: '🔔', label: 'Notifications', screen: 'Notifications' },
  { icon: '💬', label: 'Messages', screen: 'Messages' },
  { icon: '🏘️', label: 'Neighbours', screen: 'Neighbors' },
  { icon: '📏', label: 'Meters', screen: 'Meters' },
  { icon: '👥', label: 'Person-months', screen: 'PersonMonths' },
  { icon: '⚡', label: 'Outages', screen: 'Outages' },
  { icon: '📰', label: 'News', screen: 'News' },
  { icon: '📝', label: 'Forms', screen: 'Forms' },
  { icon: '🏢', label: 'Buildings', screen: 'Buildings' },
  { icon: '📑', label: 'Leases', screen: 'Leases' },
  { icon: '👤', label: 'Profile', screen: 'Profile' },
];

function MoreMenu({ onNavigate }: MoreMenuProps) {
  return (
    <View style={moreStyles.container}>
      <View style={moreStyles.header}>
        <Text style={moreStyles.title}>More</Text>
        <Text style={moreStyles.subtitle}>Everything else available in the app.</Text>
      </View>
      {MORE_ITEMS.map((item) => (
        <Pressable key={item.screen} style={moreStyles.row} onPress={() => onNavigate(item.screen)}>
          <Text style={moreStyles.rowIcon}>{item.icon}</Text>
          <Text style={moreStyles.rowLabel}>{item.label}</Text>
          <Text style={moreStyles.rowChevron}>›</Text>
        </Pressable>
      ))}
    </View>
  );
}

const moreStyles = StyleSheet.create({
  container: { flex: 1, backgroundColor: colors.surfaceMuted },
  header: {
    padding: 20,
    paddingTop: 60,
    backgroundColor: colors.white,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  title: { fontSize: 24, fontWeight: 'bold', color: colors.text },
  subtitle: { fontSize: 13, color: colors.textMuted, marginTop: 4 },
  row: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: 16,
    paddingHorizontal: 20,
    backgroundColor: colors.white,
    borderBottomWidth: 1,
    borderBottomColor: colors.surfaceMuted,
  },
  rowIcon: { fontSize: 20, width: 32 },
  rowLabel: { flex: 1, fontSize: 16, color: colors.text },
  rowChevron: { fontSize: 20, color: colors.textSubtle },
});

interface NavButtonProps {
  icon: string;
  label: string;
  isActive: boolean;
  onPress: () => void;
  badge?: number;
  syncBadge?: React.ReactNode;
}

function NavButton({ icon, label, isActive, onPress, badge, syncBadge }: NavButtonProps) {
  return (
    <Pressable style={styles.navButton} onPress={onPress}>
      <View style={styles.navIconContainer}>
        <Text style={styles.navIcon}>{icon}</Text>
        {badge && badge > 0 && (
          <View style={styles.badge}>
            <Text style={styles.badgeText}>{badge > 99 ? '99+' : badge}</Text>
          </View>
        )}
        {syncBadge && <View style={styles.syncBadgeContainer}>{syncBadge}</View>}
      </View>
      <Text style={[styles.navLabel, isActive && styles.navLabelActive]}>{label}</Text>
    </Pressable>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AuthProvider apiBaseUrl={API_BASE_URL}>
        <MainApp />
      </AuthProvider>
    </QueryClientProvider>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.surfaceMuted,
  },
  loadingContainer: {
    flex: 1,
    backgroundColor: colors.white,
    alignItems: 'center',
    justifyContent: 'center',
  },
  loadingText: {
    fontSize: 16,
    color: colors.textMuted,
  },
  content: {
    flex: 1,
  },
  bottomNav: {
    flexDirection: 'row',
    backgroundColor: colors.white,
    borderTopWidth: 1,
    borderTopColor: colors.border,
    paddingBottom: 24,
    paddingTop: 8,
  },
  navButton: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 8,
  },
  navIconContainer: {
    position: 'relative',
  },
  navIcon: {
    fontSize: 24,
  },
  badge: {
    position: 'absolute',
    top: -4,
    right: -10,
    backgroundColor: colors.danger,
    borderRadius: 10,
    minWidth: 18,
    height: 18,
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: 4,
  },
  badgeText: {
    color: colors.white,
    fontSize: 10,
    fontWeight: 'bold',
  },
  navLabel: {
    fontSize: 11,
    color: colors.textMuted,
    marginTop: 4,
  },
  navLabelActive: {
    color: colors.accent,
    fontWeight: '600',
  },
  syncBadgeContainer: {
    position: 'absolute',
    top: -4,
    right: -12,
  },
});
