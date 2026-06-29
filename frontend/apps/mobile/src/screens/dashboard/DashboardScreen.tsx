import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useAuth } from '../../contexts/AuthContext';
import { useApiQuery } from '../../hooks/useApi';
import { colors } from '../shared/screenStyles';

// Dashboard card types
interface DashboardStats {
  pendingFaults: number;
  unreadAnnouncements: number;
  activeVotes: number;
  unreadMessages: number;
  upcomingPayments: number;
}

interface Announcement {
  id: string;
  title: string;
  createdAt: string;
  category: 'general' | 'urgent' | 'maintenance' | 'event';
}

interface PendingAction {
  id: string;
  type: 'vote' | 'payment' | 'reading' | 'fault';
  title: string;
  dueDate?: string;
}

interface ApiAnnouncement {
  id: string;
  title: string;
  status: string;
  pinned: boolean;
  published_at?: string | null;
  created_at: string;
}

interface ApiAnnouncementListResponse {
  announcements: ApiAnnouncement[];
}

interface ApiFaultStatistics {
  statistics?: { open_count?: number | null; in_progress_count?: number | null };
}

interface ApiVoteListResponse {
  votes?: Array<{ id: string; title: string; status: string; end_at: string }>;
}

interface ApiUnreadMessages {
  // api-server serializes `UnreadMessagesResponse` with camelCase, so the wire
  // field is `unreadCount` (BIT-239 wire-contract flip). The snake_case/`count`
  // keys are kept only as defensive fallbacks for any stale cache.
  unreadCount?: number;
  unread_count?: number;
  count?: number;
}

interface DashboardScreenProps {
  onNavigate?: (screen: string) => void;
}

export function DashboardScreen({ onNavigate }: DashboardScreenProps) {
  const { t } = useTranslation();
  const { user, logout } = useAuth();

  // Each card pulls its own count so a single slow/down endpoint doesn't
  // stall the whole dashboard. `staleTime` is short here because the
  // counts are user-facing badges that should feel fresh.
  const announcementsQuery = useApiQuery<ApiAnnouncementListResponse>(
    ['dashboard', 'announcements'],
    '/api/v1/announcements?status=published&limit=3',
    { staleTime: 60_000 }
  );
  const faultsStatsQuery = useApiQuery<ApiFaultStatistics>(
    ['dashboard', 'faults-stats'],
    '/api/v1/faults/statistics',
    { staleTime: 60_000 }
  );
  const votesQuery = useApiQuery<ApiVoteListResponse>(['dashboard', 'votes'], '/api/v1/voting', {
    staleTime: 60_000,
  });
  const unreadMessagesQuery = useApiQuery<ApiUnreadMessages>(
    ['dashboard', 'unread-messages'],
    '/api/v1/messages/unread-count',
    { staleTime: 30_000 }
  );

  const announcements: Announcement[] = (announcementsQuery.data?.announcements ?? [])
    .slice(0, 3)
    .map((a) => ({
      id: a.id,
      title: a.title,
      createdAt: a.published_at ?? a.created_at,
      // The list endpoint doesn't expose category yet — default to general
      // so the card colour stays neutral.
      category: 'general',
    }));

  const stats: DashboardStats = {
    pendingFaults:
      (faultsStatsQuery.data?.statistics?.open_count ?? 0) +
      (faultsStatsQuery.data?.statistics?.in_progress_count ?? 0),
    unreadAnnouncements: announcements.length,
    activeVotes: (votesQuery.data?.votes ?? []).filter((v) => v.status === 'active').length,
    unreadMessages:
      unreadMessagesQuery.data?.unreadCount ??
      unreadMessagesQuery.data?.unread_count ??
      unreadMessagesQuery.data?.count ??
      0,
    upcomingPayments: 0,
  };

  // The api-server has no aggregate "pending resident actions" endpoint
  // yet. Once it exists (one feed combining vote eligibility, due
  // payments, scheduled meter reading windows, …) this card will read
  // from there. For now show whatever signals we already have so the
  // card isn't silently empty.
  const pendingActions: PendingAction[] = (votesQuery.data?.votes ?? [])
    .filter((v) => v.status === 'active')
    .slice(0, 3)
    .map((v) => ({
      id: v.id,
      type: 'vote',
      title: v.title,
      dueDate: v.end_at,
    }));

  const isFetching =
    announcementsQuery.isFetching ||
    faultsStatsQuery.isFetching ||
    votesQuery.isFetching ||
    unreadMessagesQuery.isFetching;

  const onRefresh = useCallback(async () => {
    await Promise.all([
      announcementsQuery.refetch(),
      faultsStatsQuery.refetch(),
      votesQuery.refetch(),
      unreadMessagesQuery.refetch(),
    ]);
  }, [announcementsQuery, faultsStatsQuery, votesQuery, unreadMessagesQuery]);

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  };

  const getCategoryColor = (category: Announcement['category']): string => {
    switch (category) {
      case 'urgent':
        return colors.danger;
      case 'maintenance':
        return colors.warning;
      case 'event':
        return colors.eventCategoryInk;
      default:
        return colors.textMuted;
    }
  };

  const getActionIcon = (type: PendingAction['type']): string => {
    switch (type) {
      case 'vote':
        return '🗳️';
      case 'payment':
        return '💳';
      case 'reading':
        return '📊';
      case 'fault':
        return '🔧';
      default:
        return '📋';
    }
  };

  return (
    <View style={styles.container}>
      {/* Header */}
      <View style={styles.header}>
        <View>
          <Text style={styles.greeting}>{t('dashboard.welcomeBack')}</Text>
          <Text style={styles.userName}>
            {user?.firstName} {user?.lastName}
          </Text>
        </View>
        <Pressable style={styles.logoutButton} onPress={logout}>
          <Text style={styles.logoutText}>{t('auth.logout')}</Text>
        </Pressable>
      </View>

      <ScrollView
        style={styles.scrollView}
        refreshControl={
          <RefreshControl refreshing={isFetching} onRefresh={onRefresh} tintColor={colors.accent} />
        }
      >
        {/* Stats Grid */}
        <View style={styles.statsGrid}>
          <Pressable style={styles.statCard} onPress={() => onNavigate?.('Faults')}>
            <Text style={styles.statNumber}>{stats.pendingFaults}</Text>
            <Text style={styles.statLabel}>{t('dashboard.openFaults')}</Text>
          </Pressable>
          <Pressable style={styles.statCard} onPress={() => onNavigate?.('Announcements')}>
            <Text style={styles.statNumber}>{stats.unreadAnnouncements}</Text>
            <Text style={styles.statLabel}>{t('dashboard.newAnnouncements')}</Text>
          </Pressable>
          <Pressable style={styles.statCard} onPress={() => onNavigate?.('Voting')}>
            <Text style={styles.statNumber}>{stats.activeVotes}</Text>
            <Text style={styles.statLabel}>{t('dashboard.activeVotes')}</Text>
          </Pressable>
          <Pressable style={styles.statCard} onPress={() => onNavigate?.('Messages')}>
            <Text style={styles.statNumber}>{stats.unreadMessages}</Text>
            <Text style={styles.statLabel}>{t('dashboard.unreadMessages')}</Text>
          </Pressable>
        </View>

        {/* Pending Actions */}
        {pendingActions.length > 0 && (
          <View style={styles.section}>
            <Text style={styles.sectionTitle}>{t('dashboard.pendingActions')}</Text>
            <View style={styles.actionsList}>
              {pendingActions.map((action) => (
                <Pressable key={action.id} style={styles.actionCard}>
                  <Text style={styles.actionIcon}>{getActionIcon(action.type)}</Text>
                  <View style={styles.actionContent}>
                    <Text style={styles.actionTitle}>{action.title}</Text>
                    {action.dueDate && (
                      <Text style={styles.actionDue}>
                        {t('dashboard.due', { date: formatDate(action.dueDate) })}
                      </Text>
                    )}
                  </View>
                  <Text style={styles.actionArrow}>›</Text>
                </Pressable>
              ))}
            </View>
          </View>
        )}

        {/* Recent Announcements */}
        <View style={styles.section}>
          <View style={styles.sectionHeader}>
            <Text style={styles.sectionTitle}>{t('dashboard.recentAnnouncements')}</Text>
            <Pressable onPress={() => onNavigate?.('Announcements')}>
              <Text style={styles.seeAllText}>{t('dashboard.seeAll')}</Text>
            </Pressable>
          </View>
          <View style={styles.announcementsList}>
            {announcements.map((announcement) => (
              <Pressable key={announcement.id} style={styles.announcementCard}>
                <View style={styles.announcementHeader}>
                  <View
                    style={[
                      styles.categoryBadge,
                      { backgroundColor: getCategoryColor(announcement.category) },
                    ]}
                  >
                    <Text style={styles.categoryText}>{announcement.category}</Text>
                  </View>
                  <Text style={styles.announcementDate}>{formatDate(announcement.createdAt)}</Text>
                </View>
                <Text style={styles.announcementTitle}>{announcement.title}</Text>
              </Pressable>
            ))}
          </View>
        </View>

        {/* Quick Actions */}
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>{t('dashboard.quickActions')}</Text>
          <View style={styles.quickActionsGrid}>
            <Pressable style={styles.quickAction} onPress={() => onNavigate?.('ReportFault')}>
              <Text style={styles.quickActionIcon}>🔧</Text>
              <Text style={styles.quickActionLabel}>{t('dashboard.reportFault')}</Text>
            </Pressable>
            <Pressable style={styles.quickAction} onPress={() => onNavigate?.('Documents')}>
              <Text style={styles.quickActionIcon}>📄</Text>
              <Text style={styles.quickActionLabel}>{t('dashboard.documents')}</Text>
            </Pressable>
            <Pressable style={styles.quickAction} onPress={() => onNavigate?.('MeterReading')}>
              <Text style={styles.quickActionIcon}>📊</Text>
              <Text style={styles.quickActionLabel}>{t('dashboard.meterReading')}</Text>
            </Pressable>
            <Pressable style={styles.quickAction} onPress={() => onNavigate?.('Payments')}>
              <Text style={styles.quickActionIcon}>💳</Text>
              <Text style={styles.quickActionLabel}>{t('dashboard.payments')}</Text>
            </Pressable>
          </View>
        </View>

        {/* Bottom spacing */}
        <View style={styles.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: colors.background },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: 20,
    paddingTop: 60,
    backgroundColor: colors.accent,
  },
  greeting: { fontSize: 14, color: 'rgba(255, 255, 255, 0.8)' },
  userName: { fontSize: 20, fontWeight: '600', color: colors.surface },
  logoutButton: { padding: 8 },
  logoutText: { color: 'rgba(255, 255, 255, 0.9)', fontSize: 14 },
  scrollView: { flex: 1 },
  statsGrid: { flexDirection: 'row', flexWrap: 'wrap', padding: 16, gap: 12 },
  statCard: {
    flex: 1,
    minWidth: '45%',
    backgroundColor: colors.surface,
    borderRadius: 12,
    padding: 16,
    alignItems: 'center',
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
  },
  statNumber: { fontSize: 28, fontWeight: 'bold', color: colors.accent },
  statLabel: { fontSize: 12, color: colors.textMuted, marginTop: 4, textAlign: 'center' },
  section: { padding: 16 },
  sectionHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  sectionTitle: { fontSize: 18, fontWeight: '600', color: colors.text, marginBottom: 12 },
  seeAllText: { color: colors.accent, fontSize: 14 },
  actionsList: { gap: 8 },
  actionCard: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.surface,
    borderRadius: 12,
    padding: 16,
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
  },
  actionIcon: { fontSize: 24, marginRight: 12 },
  actionContent: { flex: 1 },
  actionTitle: { fontSize: 15, fontWeight: '500', color: colors.text },
  actionDue: { fontSize: 13, color: colors.warning, marginTop: 2 },
  actionArrow: { fontSize: 24, color: colors.textSubtle },
  announcementsList: { gap: 8 },
  announcementCard: {
    backgroundColor: colors.surface,
    borderRadius: 12,
    padding: 16,
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
  },
  announcementHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 8,
  },
  categoryBadge: { paddingHorizontal: 8, paddingVertical: 4, borderRadius: 4 },
  categoryText: {
    fontSize: 11,
    fontWeight: '600',
    color: colors.surface,
    textTransform: 'uppercase',
  },
  announcementDate: { fontSize: 12, color: colors.textSubtle },
  announcementTitle: { fontSize: 15, color: colors.text, lineHeight: 20 },
  quickActionsGrid: { flexDirection: 'row', flexWrap: 'wrap', gap: 12 },
  quickAction: {
    flex: 1,
    minWidth: '45%',
    backgroundColor: colors.surface,
    borderRadius: 12,
    padding: 20,
    alignItems: 'center',
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
  },
  quickActionIcon: { fontSize: 32, marginBottom: 8 },
  quickActionLabel: { fontSize: 13, color: colors.textSecondary, fontWeight: '500' },
  bottomSpacer: { height: 100 },
});
