/**
 * NotificationsScreen (UC-01.4).
 *
 * Inbox-style list of all notifications delivered to the user.
 *
 * NOTE: still backed by mock data. The api-server currently exposes only a
 * WebSocket sync endpoint for the notification inbox (`ws_notifications` at
 * `GET /api/v1/users/me/notifications/ws`) plus preference/critical-notification
 * routes — there is no REST list endpoint to page the inbox. Wiring this screen
 * needs a backend `GET /api/v1/users/me/notifications` (REST list) first, so it
 * is intentionally left on mock data (tracked as a backend follow-up).
 */

import { useCallback, useMemo, useState } from 'react';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { EmptyState } from '../../components/states';
import { resolveLocale } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';

export type NotificationCategory =
  | 'fault'
  | 'announcement'
  | 'message'
  | 'vote'
  | 'meter'
  | 'system';

export interface AppNotification {
  id: string;
  category: NotificationCategory;
  title: string;
  body: string;
  receivedAt: string;
  read: boolean;
}

const MOCK_NOTIFICATIONS: AppNotification[] = [
  {
    id: 'nf1',
    category: 'fault',
    title: 'Fault status updated',
    body: 'Your reported fault "Leaking pipe in lobby" was assigned to a contractor.',
    receivedAt: '2026-04-24T08:42:00Z',
    read: false,
  },
  {
    id: 'nf2',
    category: 'announcement',
    title: 'New announcement',
    body: 'Annual building meeting scheduled for May 12.',
    receivedAt: '2026-04-23T14:00:00Z',
    read: false,
  },
  {
    id: 'nf3',
    category: 'vote',
    title: 'Voting open',
    body: '"Replace lobby carpet" — voting closes in 3 days.',
    receivedAt: '2026-04-22T09:30:00Z',
    read: true,
  },
  {
    id: 'nf4',
    category: 'meter',
    title: 'Meter reading reminder',
    body: 'Submit your April water-meter reading by April 30.',
    receivedAt: '2026-04-21T07:00:00Z',
    read: true,
  },
];

const CATEGORY_FILTERS: ReadonlyArray<{ value: 'all' | NotificationCategory; label: string }> = [
  { value: 'all', label: 'All' },
  { value: 'fault', label: 'Faults' },
  { value: 'announcement', label: 'Announcements' },
  { value: 'message', label: 'Messages' },
  { value: 'vote', label: 'Voting' },
  { value: 'meter', label: 'Meters' },
];

function categoryIcon(category: NotificationCategory): string {
  switch (category) {
    case 'fault':
      return '🛠️';
    case 'announcement':
      return '📢';
    case 'message':
      return '💬';
    case 'vote':
      return '🗳️';
    case 'meter':
      return '📏';
    default:
      return '🔔';
  }
}

function formatRelative(iso: string): string {
  const diffMs = Date.now() - new Date(iso).getTime();
  const minutes = Math.floor(diffMs / 60_000);
  if (minutes < 60) return `${Math.max(minutes, 1)}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;
  return new Date(iso).toLocaleDateString(resolveLocale(), { month: 'short', day: 'numeric' });
}

interface NotificationsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function NotificationsScreen({ onNavigate }: NotificationsScreenProps) {
  const [refreshing, setRefreshing] = useState(false);
  const [filter, setFilter] = useState<'all' | NotificationCategory>('all');
  const [items, setItems] = useState<AppNotification[]>(MOCK_NOTIFICATIONS);

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((r) => setTimeout(r, 600));
    setRefreshing(false);
  }, []);

  const filtered = useMemo(
    () => (filter === 'all' ? items : items.filter((i) => i.category === filter)),
    [items, filter]
  );

  const markAllRead = () => {
    setItems((prev) => prev.map((i) => ({ ...i, read: true })));
  };

  const unread = items.filter((i) => !i.read).length;

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Notifications</Text>
        {unread > 0 && <Text style={s.headerSubtitle}>{unread} unread</Text>}
      </View>

      <View style={s.filtersWrapper}>
        <ScrollView horizontal showsHorizontalScrollIndicator={false}>
          <View style={s.filtersRow}>
            {CATEGORY_FILTERS.map((option) => (
              <Pressable
                key={option.value}
                style={[s.filterChip, filter === option.value && s.filterChipActive]}
                onPress={() => setFilter(option.value)}
              >
                <Text style={[s.filterText, filter === option.value && s.filterTextActive]}>
                  {option.label}
                </Text>
              </Pressable>
            ))}
          </View>
        </ScrollView>
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={onRefresh} />}
      >
        {unread > 0 && (
          <Pressable onPress={markAllRead} style={styles.markAll}>
            <Text style={styles.markAllText}>Mark all as read</Text>
          </Pressable>
        )}

        {filtered.length === 0 ? (
          <EmptyState
            icon="🔕"
            title="You're all caught up"
            description="New notifications will appear here."
          />
        ) : (
          filtered.map((notification) => (
            <Pressable
              key={notification.id}
              style={[s.card, !notification.read && styles.unreadCard]}
              onPress={() => onNavigate?.('NotificationDetail', { id: notification.id })}
            >
              <View style={s.cardHeader}>
                <Text style={styles.icon}>{categoryIcon(notification.category)}</Text>
                <Text style={[s.cardTitle, { marginLeft: 8 }]} numberOfLines={1}>
                  {notification.title}
                </Text>
                <Text style={s.cardMeta}>{formatRelative(notification.receivedAt)}</Text>
              </View>
              <Text style={s.cardBody} numberOfLines={2}>
                {notification.body}
              </Text>
            </Pressable>
          ))
        )}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  markAll: { alignSelf: 'flex-end', padding: 8, marginBottom: 8 },
  markAllText: { color: colors.accent, fontSize: 14, fontWeight: '500' },
  unreadCard: { borderLeftWidth: 3, borderLeftColor: colors.accent },
  icon: { fontSize: 18 },
});
