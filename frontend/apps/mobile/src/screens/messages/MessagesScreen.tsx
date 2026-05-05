/**
 * MessagesScreen (UC-05).
 *
 * Lists message threads for the signed-in user against the api-server's
 * `/api/v1/messages/threads` endpoint.
 */

import { useCallback, useMemo, useState } from 'react';
import { Pressable, RefreshControl, ScrollView, Text, TextInput, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { colors, screenStyles as s } from '../shared/screenStyles';

/** Shape returned by `GET /api/v1/messages/threads`. The api-server uses
 *  Rust serde defaults (snake_case), so the wire format matches the field
 *  names in `db::models::messaging::ThreadWithPreview` verbatim. */
interface ParticipantInfo {
  id: string;
  first_name: string;
  last_name: string;
  email: string;
}

interface MessagePreview {
  id: string;
  content: string;
  sender_id: string;
  is_from_me: boolean;
  created_at: string;
}

interface ThreadWithPreview {
  id: string;
  organization_id: string;
  participant_ids: string[];
  other_participant: ParticipantInfo;
  last_message: MessagePreview | null;
  unread_count: number;
  created_at: string;
  updated_at: string;
}

interface ThreadListResponse {
  threads: ThreadWithPreview[];
  total: number;
  has_more: boolean;
}

function formatRelative(iso: string): string {
  const diffMs = Date.now() - new Date(iso).getTime();
  const minutes = Math.floor(diffMs / 60_000);
  if (minutes < 60) return `${Math.max(minutes, 1)}m`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d`;
  return new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

function participantName(p: ParticipantInfo): string {
  const full = `${p.first_name ?? ''} ${p.last_name ?? ''}`.trim();
  return full || p.email;
}

interface MessagesScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function MessagesScreen({ onNavigate }: MessagesScreenProps) {
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
    return threads.filter((t) =>
      `${participantName(t.other_participant)} ${t.last_message?.content ?? ''}`
        .toLowerCase()
        .includes(needle)
    );
  }, [threads, search]);

  const unreadTotal = threads.reduce((acc, t) => acc + t.unread_count, 0);

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
            <Text style={s.emptyTitle}>Loading…</Text>
          </View>
        ) : error ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>Couldn't load messages</Text>
            <Text style={s.emptyText}>{error.message}</Text>
            <Pressable style={s.primaryButton} onPress={() => refetch()}>
              <Text style={s.primaryButtonText}>Try again</Text>
            </Pressable>
          </View>
        ) : filtered.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>📬</Text>
            <Text style={s.emptyTitle}>No conversations</Text>
            <Text style={s.emptyText}>Start a conversation with your manager or neighbours.</Text>
          </View>
        ) : (
          filtered.map((thread) => {
            const lastAt = thread.last_message?.created_at ?? thread.updated_at;
            return (
              <Pressable
                key={thread.id}
                style={s.card}
                onPress={() => onNavigate?.('ThreadDetail', { threadId: thread.id })}
              >
                <View style={s.cardHeader}>
                  <Text style={s.cardTitle} numberOfLines={1}>
                    {participantName(thread.other_participant)}
                  </Text>
                  <Text style={s.cardMeta}>{formatRelative(lastAt)}</Text>
                </View>
                <Text style={s.cardBody} numberOfLines={2}>
                  {thread.last_message?.content ?? 'No messages yet.'}
                </Text>
                {thread.unread_count > 0 && (
                  <View style={s.cardFooter}>
                    <View style={[s.badge, { backgroundColor: colors.accent }]}>
                      <Text style={[s.badgeText, { color: '#fff' }]}>
                        {thread.unread_count} new
                      </Text>
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
