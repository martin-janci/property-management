import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import Constants from 'expo-constants';
import { StatusBar } from 'expo-status-bar';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { OfflineBanner, SyncProgressToast, SyncStatusBadge } from './components/sync';
import { AuthProvider, useAuth } from './contexts';
import { useOfflineSupport } from './hooks';
import './i18n'; // Initialize i18n
import {
  AnnouncementsScreen,
  AuthFlow,
  DashboardScreen,
  DocumentsScreen,
  FaultsListScreen,
  MeterReadingScreen,
  ProfileScreen,
  ReportFaultScreen,
  TwoFactorScreen,
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

type Screen =
  | 'Dashboard'
  | 'Faults'
  | 'ReportFault'
  | 'Announcements'
  | 'Voting'
  | 'Documents'
  | 'Messages'
  | 'Settings'
  | 'MeterReading'
  | 'Profile'
  | 'TwoFactor';

function MainApp() {
  const { t } = useTranslation();
  const { isAuthenticated, isLoading } = useAuth();
  const { isConnected, queuedActionsCount, isSyncing, syncProgress, processQueue } =
    useOfflineSupport();
  const [showSyncToast, setShowSyncToast] = useState(false);

  // Show sync toast when sync progress starts
  const handleRetrySync = useCallback(() => {
    processQueue();
  }, [processQueue]);

  // Show toast when sync is in progress
  const isSyncingWithProgress = isSyncing && syncProgress !== null;
  const [currentScreen, setCurrentScreen] = useState<Screen>('Dashboard');

  const handleNavigate = useCallback((screen: string) => {
    setCurrentScreen(screen as Screen);
  }, []);

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
      case 'Voting':
        return <VotingScreen onNavigate={handleNavigate} />;
      case 'Documents':
        return <DocumentsScreen onNavigate={handleNavigate} />;
      case 'MeterReading':
        return (
          <MeterReadingScreen
            onSuccess={() => handleNavigate('Dashboard')}
            onCancel={() => handleNavigate('Dashboard')}
          />
        );
      case 'Profile':
        return (
          <ProfileScreen
            onTwoFactorPress={() => handleNavigate('TwoFactor')}
          />
        );
      case 'TwoFactor':
        return <TwoFactorScreen onDonePress={() => handleNavigate('Profile')} />;
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
          isActive={currentScreen === 'Voting'}
          onPress={() => handleNavigate('Voting')}
        />
        <NavButton
          icon="📄"
          label={t('tabs.docs')}
          isActive={currentScreen === 'Documents'}
          onPress={() => handleNavigate('Documents')}
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
    backgroundColor: '#f5f5f5',
  },
  loadingContainer: {
    flex: 1,
    backgroundColor: '#fff',
    alignItems: 'center',
    justifyContent: 'center',
  },
  loadingText: {
    fontSize: 16,
    color: '#6b7280',
  },
  content: {
    flex: 1,
  },
  bottomNav: {
    flexDirection: 'row',
    backgroundColor: '#fff',
    borderTopWidth: 1,
    borderTopColor: '#e5e7eb',
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
    backgroundColor: '#ef4444',
    borderRadius: 10,
    minWidth: 18,
    height: 18,
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: 4,
  },
  badgeText: {
    color: '#fff',
    fontSize: 10,
    fontWeight: 'bold',
  },
  navLabel: {
    fontSize: 11,
    color: '#6b7280',
    marginTop: 4,
  },
  navLabelActive: {
    color: '#2563eb',
    fontWeight: '600',
  },
  syncBadgeContainer: {
    position: 'absolute',
    top: -4,
    right: -12,
  },
});
