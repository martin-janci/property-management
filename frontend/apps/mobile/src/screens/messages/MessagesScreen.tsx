/**
 * MessagesScreen (UC-05).
 *
 * Lists message threads for the signed-in user. Renders mock data until the
 * mobile app wires `@ppt/api-client/messaging` hooks (see ppt-web for the
 * fully-wired version).
 */

import { useCallback, useMemo, useState } from 'react';
import { Pressable, RefreshControl, ScrollView, Text, TextInput, View } from 'react-native';
import { colors, screenStyles as s } from '../shared/screenStyles';

export interface MessageThread {
  id: string;
  participantName: string;
  lastMessage: string;
  lastMessageAt: string;
  unreadCount: number;
}

const MOCK_THREADS: MessageThread[] = [
  {
    id: 't1',
    participantName: 'Building manager',
    lastMessage: 'The plumber will arrive tomorrow at 9am.',
    lastMessageAt: '2026-04-23T16:42:00Z',
    unreadCount: 2,
  },
  {
    id: 't2',
    participantName: 'Anna Novak (Apt 3B)',
    lastMessage: 'Sure, I can take in your package this afternoon.',
    lastMessageAt: '2026-04-22T11:00:00Z',
    unreadCount: 0,
  },
  {
    id: 't3',
    participantName: 'Cleaning service',
    lastMessage: 'We have rescheduled the lobby cleaning to Friday.',
    lastMessageAt: '2026-04-20T08:15:00Z',
    unreadCount: 1,
  },
];

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

interface MessagesScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function MessagesScreen({ onNavigate }: MessagesScreenProps) {
  const [refreshing, setRefreshing] = useState(false);
  const [search, setSearch] = useState('');

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((r) => setTimeout(r, 600));
    setRefreshing(false);
  }, []);

  const filtered = useMemo(
    () =>
      MOCK_THREADS.filter((t) =>
        search.trim() === ''
          ? true
          : `${t.participantName} ${t.lastMessage}`.toLowerCase().includes(search.toLowerCase())
      ),
    [search]
  );

  const unreadTotal = MOCK_THREADS.reduce((acc, t) => acc + t.unreadCount, 0);

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>Messages</Text>
        {unreadTotal > 0 && (
          <Text style={s.headerSubtitle}>{unreadTotal} unread message{unreadTotal === 1 ? '' : 's'}</Text>
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
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={onRefresh} />}
      >
        {filtered.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>📬</Text>
            <Text style={s.emptyTitle}>No conversations</Text>
            <Text style={s.emptyText}>Start a conversation with your manager or neighbours.</Text>
          </View>
        ) : (
          filtered.map((thread) => (
            <Pressable
              key={thread.id}
              style={s.card}
              onPress={() => onNavigate?.('ThreadDetail', { threadId: thread.id })}
            >
              <View style={s.cardHeader}>
                <Text style={s.cardTitle} numberOfLines={1}>
                  {thread.participantName}
                </Text>
                <Text style={s.cardMeta}>{formatRelative(thread.lastMessageAt)}</Text>
              </View>
              <Text style={s.cardBody} numberOfLines={2}>
                {thread.lastMessage}
              </Text>
              {thread.unreadCount > 0 && (
                <View style={s.cardFooter}>
                  <View style={[s.badge, { backgroundColor: colors.accent }]}>
                    <Text style={[s.badgeText, { color: '#fff' }]}>{thread.unreadCount} new</Text>
                  </View>
                </View>
              )}
            </Pressable>
          ))
        )}

        <Pressable style={s.primaryButton} onPress={() => onNavigate?.('NewMessage')}>
          <Text style={s.primaryButtonText}>+ New message</Text>
        </Pressable>

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}
