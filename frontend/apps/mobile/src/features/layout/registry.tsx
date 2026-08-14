import type React from 'react';
import { createContext, useContext } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { formatDate as formatLocaleDate } from '../../i18n/format';
import { colors } from '../../screens/shared/screenStyles';
import type { ResolvedScreen } from './types';

// NavigateContext threads `onNavigate` to section components without prop drilling
export const NavigateContext = createContext<((screen: string) => void) | undefined>(undefined);

// --- Internal types (verbatim from DashboardScreen) ---
interface DashboardStats {
  pendingFaults: number;
  unreadAnnouncements: number;
  activeVotes: number;
  unreadMessages: number;
  upcomingPayments: number;
}

interface PendingAction {
  id: string;
  type: 'vote' | 'payment' | 'reading' | 'fault';
  title: string;
  dueDate?: string;
}

interface ApiVoteListResponse {
  votes?: Array<{ id: string; title: string; status: string; end_at: string }>;
}

interface ApiFaultStatistics {
  statistics?: { open_count?: number | null; in_progress_count?: number | null };
}

interface ApiAnnouncementListResponse {
  announcements: Array<{
    id: string;
    title: string;
    status: string;
    pinned: boolean;
    published_at?: string | null;
    created_at: string;
  }>;
}

interface ApiUnreadMessages {
  unreadCount?: number;
  unread_count?: number;
  count?: number;
}

// --- Section component props ---
export interface SectionComponentProps {
  mode?: string;
  props?: Record<string, unknown>;
}

// --- DashboardStatsSection ---
export function DashboardStatsSection({ mode: _mode, props: _props }: SectionComponentProps) {
  const { t } = useTranslation();
  const onNavigate = useContext(NavigateContext);

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

  const announcements = (announcementsQuery.data?.announcements ?? []).slice(0, 3);

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

  return (
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
  );
}

// --- ActionQueueSection ---
export function ActionQueueSection({ mode: _mode, props: _props }: SectionComponentProps) {
  const { t } = useTranslation();

  const votesQuery = useApiQuery<ApiVoteListResponse>(['dashboard', 'votes'], '/api/v1/voting', {
    staleTime: 60_000,
  });

  const pendingActions: PendingAction[] = (votesQuery.data?.votes ?? [])
    .filter((v) => v.status === 'active')
    .slice(0, 3)
    .map((v) => ({
      id: v.id,
      type: 'vote',
      title: v.title,
      dueDate: v.end_at,
    }));

  const formatDate = (dateString: string): string =>
    formatLocaleDate(dateString, { month: 'short', day: 'numeric' });

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

  if (pendingActions.length === 0) return null;

  return (
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
  );
}

// --- Registry ---
export type SectionRegistry = Record<
  string,
  { component: React.ComponentType<SectionComponentProps> }
>;

export const dashboardRegistry: SectionRegistry = {
  'dashboard-stats.v1': { component: DashboardStatsSection },
  'action-queue.v1': { component: ActionQueueSection },
};

// --- Default layout ---
export const DEFAULT_DASHBOARD_LAYOUT: ResolvedScreen = {
  screen: 'ppt/dashboard',
  version: 0,
  sections: [
    { type: 'dashboard-stats.v1', presentation: 'visible' },
    { type: 'action-queue.v1', presentation: 'visible' },
  ],
};

// --- Styles ---
const styles = StyleSheet.create({
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
  sectionTitle: { fontSize: 18, fontWeight: '600', color: colors.text, marginBottom: 12 },
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
});
