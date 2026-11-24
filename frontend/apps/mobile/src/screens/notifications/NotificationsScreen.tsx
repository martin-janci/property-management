/**
 * NotificationsScreen (UC-01.4).
 *
 * Inbox-style list of all notifications delivered to the user.
 *
 * Wired to the in-app notification inbox on the api-server (Epic 29
 * notification groups), the same REST surface ppt-web's NotificationInboxPage
 * consumes through `@ppt/api-client`:
 *   GET    /api/v1/users/me/notification-preferences/granular/groups
 *   POST   .../groups/read-all
 *   POST   .../groups/{group_id}/read
 * Backend writes in-app notifications into `notification_groups` /
 * `grouped_notifications`; each group is one entity (a fault, a vote, …) with
 * its newest notifications. We read through the mobile `useApi` hooks (absolute
 * URLs + SecureStore bearer token) rather than the web `@ppt/api-client` hooks,
 * which use browser-relative paths that don't resolve on React Native — only
 * the response TYPES are shared from the generated client so a backend field
 * rename surfaces at compile time.
 */

import type { InboxGroupWithNotifications, InboxResponse } from '@ppt/api-client';
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { EmptyState } from '../../components/states';
import { useApiMutation, useApiQuery } from '../../hooks/useApi';
import { formatDate } from '../../i18n/format';
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
  /** The entity the group is about; a tap opens it (see `notificationTarget`). */
  entityId: string;
}

/** Where a tap on a notification leads. Faults, announcements and votes open
 *  the entity itself; messages and meters open their list, because the group's
 *  entity id is not guaranteed to be a thread or meter id. `system` rows have
 *  nowhere to go and return `null` (the row is only marked read). Before this
 *  the tap navigated to a `NotificationDetail` screen that does not exist and
 *  landed on the Dashboard. Exported for unit testing. */
export function notificationTarget(
  notification: Pick<AppNotification, 'category' | 'entityId'>
): { screen: string; params?: Record<string, unknown> } | null {
  const { category, entityId } = notification;
  switch (category) {
    case 'fault':
      return entityId
        ? { screen: 'FaultDetail', params: { faultId: entityId } }
        : { screen: 'Faults' };
    case 'announcement':
      return entityId
        ? { screen: 'AnnouncementDetail', params: { announcementId: entityId } }
        : { screen: 'Announcements' };
    case 'vote':
      return entityId
        ? { screen: 'VoteDetail', params: { voteId: entityId } }
        : { screen: 'Voting' };
    case 'message':
      return { screen: 'Messages' };
    case 'meter':
      return { screen: 'Meters' };
    default:
      return null;
  }
}

const INBOX_BASE_PATH = '/api/v1/users/me/notification-preferences/granular';
/** `GET {INBOX_BASE_PATH}/groups` — include read groups so the inbox is a full
 *  history, matching the previous screen behaviour (it showed read items too). */
const INBOX_LIST_PATH = `${INBOX_BASE_PATH}/groups?includeRead=true&limit=50`;

/** Filter values rendered as chips. The visible label for each is resolved at
 *  render time via `t('notifications.filters.<value>')` so it tracks the active
 *  locale rather than the hardcoded English the chips used to carry. */
const CATEGORY_FILTERS: ReadonlyArray<'all' | NotificationCategory> = [
  'all',
  'fault',
  'announcement',
  'message',
  'vote',
  'meter',
];

/** Map a backend notification-group `entityType` onto the UI category enum.
 *  Entity types the mobile UI has no dedicated filter for (document, finance,
 *  facility, critical, …) fall back to `system`. Exported for unit testing. */
export function toCategory(entityType: string): NotificationCategory {
  switch (entityType) {
    case 'fault':
      return 'fault';
    case 'announcement':
      return 'announcement';
    case 'message':
      return 'message';
    case 'vote':
      return 'vote';
    case 'meter':
      return 'meter';
    default:
      return 'system';
  }
}

/** Flatten one inbox group into the screen's notification row shape. The group
 *  carries the summary (title, latest timestamp, read flag); its newest entry
 *  supplies the body. Exported for unit testing. */
export function toAppNotification(item: InboxGroupWithNotifications): AppNotification {
  const { group, notifications } = item;
  const latest = notifications[0];
  return {
    id: group.id,
    category: toCategory(group.entityType),
    title: group.title,
    body: latest?.body ?? latest?.title ?? '',
    receivedAt: group.latestNotificationAt,
    read: group.isRead,
    entityId: group.entityId ?? '',
  };
}

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

interface NotificationsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function NotificationsScreen({ onNavigate }: NotificationsScreenProps) {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<'all' | NotificationCategory>('all');

  // Relative "time ago" label, localized. Lives inside the component so it can
  // read `t`; absolute dates (>7d) still fall back to the locale-aware
  // `formatDate` helper.
  const relativeTime = useCallback(
    (iso: string): string => {
      const diffMs = Date.now() - new Date(iso).getTime();
      const minutes = Math.floor(diffMs / 60_000);
      if (minutes < 60) {
        return t('notifications.time.minutesAgo', { count: Math.max(minutes, 1) });
      }
      const hours = Math.floor(minutes / 60);
      if (hours < 24) return t('notifications.time.hoursAgo', { count: hours });
      const days = Math.floor(hours / 24);
      if (days < 7) return t('notifications.time.daysAgo', { count: days });
      return formatDate(iso, { month: 'short', day: 'numeric' });
    },
    [t]
  );

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<InboxResponse>(
    ['notifications', 'inbox'],
    INBOX_LIST_PATH,
    { staleTime: 30_000 }
  );

  const markAll = useApiMutation<void, void>(`${INBOX_BASE_PATH}/groups/read-all`, 'POST');
  const markOne = useApiMutation<void, string>(
    (groupId) => `${INBOX_BASE_PATH}/groups/${encodeURIComponent(groupId)}/read`,
    'POST'
  );

  const items = useMemo<AppNotification[]>(
    () => (data?.groups ?? []).map(toAppNotification),
    [data]
  );

  const filtered = useMemo(
    () => (filter === 'all' ? items : items.filter((i) => i.category === filter)),
    [items, filter]
  );

  const unread = data?.totalUnread ?? 0;

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  const markAllRead = useCallback(() => {
    markAll.mutate(undefined, { onSuccess: () => void refetch() });
  }, [markAll, refetch]);

  const openNotification = useCallback(
    (notification: AppNotification) => {
      if (!notification.read) {
        markOne.mutate(notification.id, { onSuccess: () => void refetch() });
      }
      const target = notificationTarget(notification);
      if (target) {
        onNavigate?.(target.screen, target.params);
      }
    },
    [markOne, refetch, onNavigate]
  );

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>{t('notifications.title')}</Text>
        {unread > 0 && (
          <Text style={s.headerSubtitle}>{t('notifications.unreadCount', { count: unread })}</Text>
        )}
      </View>

      <View style={s.filtersWrapper}>
        <ScrollView horizontal showsHorizontalScrollIndicator={false}>
          <View style={s.filtersRow}>
            {CATEGORY_FILTERS.map((value) => (
              <Pressable
                key={value}
                style={[s.filterChip, filter === value && s.filterChipActive]}
                onPress={() => setFilter(value)}
              >
                <Text style={[s.filterText, filter === value && s.filterTextActive]}>
                  {t(`notifications.filters.${value}`)}
                </Text>
              </Pressable>
            ))}
          </View>
        </ScrollView>
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={isFetching} onRefresh={onRefresh} />}
      >
        {isLoading ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>{t('common.loading')}</Text>
          </View>
        ) : error ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>{t('notifications.loadError')}</Text>
            <Pressable onPress={() => void refetch()} style={styles.markAll}>
              <Text style={styles.markAllText}>{t('common.tryAgain')}</Text>
            </Pressable>
          </View>
        ) : (
          <>
            {unread > 0 && (
              <Pressable onPress={markAllRead} disabled={markAll.isPending} style={styles.markAll}>
                <Text style={styles.markAllText}>{t('notifications.markAllRead')}</Text>
              </Pressable>
            )}

            {filtered.length === 0 ? (
              <EmptyState
                icon="🔕"
                title={t('notifications.emptyTitle')}
                description={t('notifications.emptyText')}
              />
            ) : (
              filtered.map((notification) => (
                <Pressable
                  key={notification.id}
                  style={[s.card, !notification.read && styles.unreadCard]}
                  onPress={() => openNotification(notification)}
                >
                  <View style={s.cardHeader}>
                    <Text style={styles.icon}>{categoryIcon(notification.category)}</Text>
                    <Text style={[s.cardTitle, { marginLeft: 8 }]} numberOfLines={1}>
                      {notification.title}
                    </Text>
                    <Text style={s.cardMeta}>{relativeTime(notification.receivedAt)}</Text>
                  </View>
                  <Text style={s.cardBody} numberOfLines={2}>
                    {notification.body}
                  </Text>
                </Pressable>
              ))
            )}
          </>
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
