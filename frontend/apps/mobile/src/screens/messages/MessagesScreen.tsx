/**
 * MessagesScreen (UC-05).
 *
 * Lists message threads for the signed-in user against the api-server's
 * `/api/v1/messages/threads` endpoint.
 */

import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, RefreshControl, ScrollView, Text, TextInput, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { formatDate } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';

/** Shape returned by `GET /api/v1/messages/threads`. The api-server serializes
 *  these structs with `#[serde(rename_all = "camelCase")]` (see
 *  `db::models::messaging::{ThreadWithPreview, ParticipantInfo, MessagePreview}`
 *  and the `messaging` route response types), so the wire format is camelCase. */
interface ParticipantInfo {
  id: string;
  firstName: string;
  lastName: string;
  email: string;
}

interface MessagePreview {
  id: string;
  content: string;
  senderId: string;
  isFromMe: boolean;
  createdAt: string;
}

interface ThreadWithPreview {
  id: string;
  organizationId: string;
  participantIds: string[];
  /** All other participants (everyone except the current user). For a 2-party
   *  thread this is a single entry; for group threads ([BIT-206]) it is the
   *  full list. Matches `ThreadWithPreview.participants` on the api-server. */
  participants: ParticipantInfo[];
  lastMessage: MessagePreview | null;
  unreadCount: number;
  createdAt: string;
  updatedAt: string;
}

interface ThreadListResponse {
  threads: ThreadWithPreview[];
  count: number;
  total: number;
}

function formatRelative(iso: string): string {
  const diffMs = Date.now() - new Date(iso).getTime();
  const minutes = Math.floor(diffMs / 60_000);
  if (minutes < 60) return `${Math.max(minutes, 1)}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d`;
  return formatDate(iso, { month: 'short', day: 'numeric' });
}

function participantName(p: ParticipantInfo): string {
  const full = `${p.firstName ?? ''} ${p.lastName ?? ''}`.trim();
  return full || p.email;
}

/** Title for a thread row, derived from its `participants` list. 2-party →
 *  the other person's name; group ([BIT-206]) → "Name +N"; empty/malformed →
 *  a safe fallback (never throws on an unexpected shape). */
function threadTitle(thread: ThreadWithPreview): string {
  const people = thread.participants ?? [];
  if (people.length === 0) return 'Conversation';
  const [first, ...rest] = people;
  const name = participantName(first);
  return rest.length > 0 ? `${name} +${rest.length}` : name;
}

interface MessagesScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function MessagesScreen({ onNavigate }: MessagesScreenProps) {
  const { t } = useTranslation();
  const [search, setSearch] = useState('');

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<ThreadListResponse>(
    ['messages', 'threads'],
    '/api/v1/messages/threads',
    { staleTime: 30_000 }
  );

  const threads = data?.threads ?? [];

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  const filtered = useMemo(() => {
    const needle = search.trim().toLowerCase();
    if (!needle) return threads;
    return threads.filter((th) =>
      `${threadTitle(th)} ${th.lastMessage?.content ?? ''}`.toLowerCase().includes(needle)
    );
  }, [threads, search]);

  const unreadTotal = threads.reduce((acc, th) => acc + th.unreadCount, 0);

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Messages</Text>
        {unreadTotal > 0 && (
          <Text style={s.headerSubtitle}>
            {unreadTotal} unread message{unreadTotal === 1 ? '' : 's'}
          </Text>
        )}
      </View>

      <View style={s.searchContainer}>
        <TextInput
          style={s.searchInput}
          placeholder="Search messages…"
          value={search}
          onChangeText={setSearch}
        />
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
            <Text style={s.emptyTitle}>{t('messages.loadError')}</Text>
            <Pressable style={s.primaryButton} onPress={() => refetch()}>
              <Text style={s.primaryButtonText}>{t('common.tryAgain')}</Text>
            </Pressable>
          </View>
        ) : filtered.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>📬</Text>
            <Text style={s.emptyTitle}>{t('messages.emptyTitle')}</Text>
            <Text style={s.emptyText}>{t('messages.emptyText')}</Text>
          </View>
        ) : (
          filtered.map((thread) => {
            const lastAt = thread.lastMessage?.createdAt ?? thread.updatedAt;
            return (
              <Pressable
                key={thread.id}
                style={s.card}
                onPress={() => onNavigate?.('ThreadDetail', { threadId: thread.id })}
              >
                <View style={s.cardHeader}>
                  <Text style={s.cardTitle} numberOfLines={1}>
                    {threadTitle(thread)}
                  </Text>
                  <Text style={s.cardMeta}>{formatRelative(lastAt)}</Text>
                </View>
                <Text style={s.cardBody} numberOfLines={2}>
                  {thread.lastMessage?.content ?? 'No messages yet.'}
                </Text>
                {thread.unreadCount > 0 && (
                  <View style={s.cardFooter}>
                    <View style={[s.badge, { backgroundColor: colors.accent }]}>
                      <Text style={[s.badgeText, { color: '#fff' }]}>{thread.unreadCount} new</Text>
                    </View>
                  </View>
                )}
              </Pressable>
            );
          })
        )}

        <Pressable style={s.primaryButton} onPress={() => onNavigate?.('NewMessage')}>
          <Text style={s.primaryButtonText}>+ New message</Text>
        </Pressable>

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}
