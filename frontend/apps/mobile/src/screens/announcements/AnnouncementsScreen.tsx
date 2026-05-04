import { useCallback, useState } from 'react';
import {
  Pressable,
  RefreshControl,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { colors } from '../shared/screenStyles';

export type AnnouncementCategory = 'general' | 'urgent' | 'maintenance' | 'event' | 'financial';

export interface AnnouncementAttachment {
  id: string;
  name: string;
  url: string;
  type: 'pdf' | 'image' | 'document';
}

export interface Announcement {
  id: string;
  title: string;
  content: string;
  category: AnnouncementCategory;
  createdAt: string;
  author: string;
  isRead: boolean;
  isPinned: boolean;
  attachments: AnnouncementAttachment[];
  commentsCount: number;
}

/** API shape returned by `GET /api/v1/announcements`. The response holds
 *  the full Announcement model, but only some fields map onto our UI; the
 *  rest are filled with sensible defaults. */
interface ApiAnnouncement {
  id: string;
  title: string;
  content: string;
  status: string;
  pinned: boolean;
  published_at?: string | null;
  created_at: string;
  updated_at: string;
}

interface ApiAnnouncementListResponse {
  announcements: ApiAnnouncement[];
  total?: number;
}

/** Coerce the api-server response into the UI's Announcement shape. The
 *  server doesn't (yet) expose category/author/attachments/comment-count
 *  on the list endpoint, so we default them. */
function toUiAnnouncement(a: ApiAnnouncement): Announcement {
  return {
    id: a.id,
    title: a.title,
    content: a.content,
    category: 'general',
    createdAt: a.published_at ?? a.created_at,
    author: 'Building Management',
    isRead: true,
    isPinned: a.pinned,
    attachments: [],
    commentsCount: 0,
  };
}

interface AnnouncementsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function AnnouncementsScreen({ onNavigate }: AnnouncementsScreenProps) {
  const [filter, setFilter] = useState<'all' | AnnouncementCategory>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [readIds, setReadIds] = useState<Set<string>>(new Set());

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<ApiAnnouncementListResponse>(
    ['announcements', 'list'],
    '/api/v1/announcements?status=published',
    { staleTime: 60_000 }
  );

  const announcements: Announcement[] = (data?.announcements ?? [])
    .map(toUiAnnouncement)
    .map((a) => ({ ...a, isRead: readIds.has(a.id) ? true : a.isRead }));

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  const getCategoryColor = (category: AnnouncementCategory): string => {
    switch (category) {
      case 'urgent':
        return colors.danger;
      case 'maintenance':
        return colors.warning;
      case 'event':
        return colors.eventCategoryInk;
      case 'financial':
        return colors.success;
      default:
        return colors.textMuted;
    }
  };

  const getCategoryIcon = (category: AnnouncementCategory): string => {
    switch (category) {
      case 'urgent':
        return '🚨';
      case 'maintenance':
        return '🔧';
      case 'event':
        return '📅';
      case 'financial':
        return '💰';
      default:
        return '📢';
    }
  };

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffHours < 24) {
      if (diffHours < 1) return 'Just now';
      return `${diffHours}h ago`;
    }
    if (diffDays < 7) {
      return `${diffDays}d ago`;
    }
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  };

  const markAsRead = (id: string) => {
    setReadIds((prev) => {
      if (prev.has(id)) return prev;
      const next = new Set(prev);
      next.add(id);
      return next;
    });
  };

  const filteredAnnouncements = announcements
    .filter((a) => (filter === 'all' ? true : a.category === filter))
    .filter(
      (a) =>
        searchQuery === '' ||
        a.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        a.content.toLowerCase().includes(searchQuery.toLowerCase())
    )
    .sort((a, b) => {
      if (a.isPinned !== b.isPinned) return a.isPinned ? -1 : 1;
      return new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime();
    });

  const unreadCount = announcements.filter((a) => !a.isRead).length;

  return (
    <View style={styles.container}>
      {/* Header */}
      <View style={styles.header}>
        <View>
          <Text style={styles.headerTitle}>Announcements</Text>
          {unreadCount > 0 && <Text style={styles.unreadBadge}>{unreadCount} unread</Text>}
        </View>
      </View>

      {/* Search */}
      <View style={styles.searchContainer}>
        <TextInput
          style={styles.searchInput}
          placeholder="Search announcements..."
          value={searchQuery}
          onChangeText={setSearchQuery}
        />
      </View>

      {/* Filters */}
      <ScrollView horizontal showsHorizontalScrollIndicator={false} style={styles.filtersContainer}>
        <View style={styles.filters}>
          {(['all', 'urgent', 'maintenance', 'event', 'financial', 'general'] as const).map(
            (cat) => (
              <Pressable
                key={cat}
                style={[styles.filterButton, filter === cat && styles.filterButtonActive]}
                onPress={() => setFilter(cat)}
              >
                {cat !== 'all' && <Text style={styles.filterIcon}>{getCategoryIcon(cat)}</Text>}
                <Text style={[styles.filterText, filter === cat && styles.filterTextActive]}>
                  {cat.charAt(0).toUpperCase() + cat.slice(1)}
                </Text>
              </Pressable>
            )
          )}
        </View>
      </ScrollView>

      {/* Announcements List */}
      <ScrollView
        style={styles.scrollView}
        refreshControl={
          <RefreshControl refreshing={isFetching} onRefresh={onRefresh} tintColor={colors.accent} />
        }
      >
        {isLoading ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyTitle}>Loading…</Text>
          </View>
        ) : error ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyIcon}>⚠️</Text>
            <Text style={styles.emptyTitle}>Couldn't load announcements</Text>
            <Text style={styles.emptyText}>{error.message}</Text>
          </View>
        ) : filteredAnnouncements.length === 0 ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyIcon}>📭</Text>
            <Text style={styles.emptyTitle}>No announcements</Text>
            <Text style={styles.emptyText}>Check back later for updates</Text>
          </View>
        ) : (
          filteredAnnouncements.map((announcement) => (
            <Pressable
              key={announcement.id}
              style={[styles.announcementCard, !announcement.isRead && styles.unreadCard]}
              onPress={() => {
                markAsRead(announcement.id);
                onNavigate?.('AnnouncementDetail', { announcementId: announcement.id });
              }}
            >
              {announcement.isPinned && (
                <View style={styles.pinnedBadge}>
                  <Text style={styles.pinnedText}>📌 Pinned</Text>
                </View>
              )}

              <View style={styles.announcementHeader}>
                <View
                  style={[
                    styles.categoryBadge,
                    { backgroundColor: getCategoryColor(announcement.category) },
                  ]}
                >
                  <Text style={styles.categoryIcon}>{getCategoryIcon(announcement.category)}</Text>
                  <Text style={styles.categoryText}>{announcement.category}</Text>
                </View>
                <Text style={styles.announcementDate}>{formatDate(announcement.createdAt)}</Text>
              </View>

              <Text style={styles.announcementTitle}>{announcement.title}</Text>
              <Text style={styles.announcementPreview} numberOfLines={2}>
                {announcement.content}
              </Text>

              <View style={styles.announcementFooter}>
                <Text style={styles.authorText}>By {announcement.author}</Text>
                <View style={styles.footerRight}>
                  {announcement.attachments.length > 0 && (
                    <Text style={styles.attachmentBadge}>📎 {announcement.attachments.length}</Text>
                  )}
                  {announcement.commentsCount > 0 && (
                    <Text style={styles.commentsBadge}>💬 {announcement.commentsCount}</Text>
                  )}
                </View>
              </View>

              {!announcement.isRead && <View style={styles.unreadDot} />}
            </Pressable>
          ))
        )}

        <View style={styles.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background,
  },
  header: {
    padding: 20,
    paddingTop: 60,
    backgroundColor: colors.surface,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  headerTitle: {
    fontSize: 24,
    fontWeight: 'bold',
    color: colors.text,
  },
  unreadBadge: {
    fontSize: 13,
    color: colors.accent,
    marginTop: 4,
  },
  searchContainer: {
    padding: 16,
    paddingBottom: 8,
    backgroundColor: colors.surface,
  },
  searchInput: {
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
    padding: 12,
    fontSize: 16,
  },
  filtersContainer: {
    backgroundColor: colors.surface,
    paddingBottom: 16,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  filters: {
    flexDirection: 'row',
    paddingHorizontal: 16,
    gap: 8,
  },
  filterButton: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 14,
    paddingVertical: 8,
    borderRadius: 20,
    backgroundColor: colors.surfaceMuted,
    gap: 4,
  },
  filterButtonActive: {
    backgroundColor: colors.accent,
  },
  filterIcon: {
    fontSize: 14,
  },
  filterText: {
    fontSize: 14,
    color: colors.textMuted,
    fontWeight: '500',
  },
  filterTextActive: {
    color: colors.surface,
  },
  scrollView: {
    flex: 1,
    padding: 16,
  },
  emptyState: {
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 60,
  },
  emptyIcon: {
    fontSize: 48,
    marginBottom: 16,
  },
  emptyTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: colors.textSecondary,
    marginBottom: 4,
  },
  emptyText: {
    fontSize: 14,
    color: colors.textMuted,
  },
  announcementCard: {
    backgroundColor: colors.surface,
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
    position: 'relative',
  },
  unreadCard: {
    borderLeftWidth: 3,
    borderLeftColor: colors.accent,
  },
  pinnedBadge: {
    marginBottom: 8,
  },
  pinnedText: {
    fontSize: 12,
    color: colors.warning,
    fontWeight: '600',
  },
  announcementHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 8,
  },
  categoryBadge: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 8,
    paddingVertical: 4,
    borderRadius: 4,
    gap: 4,
  },
  categoryIcon: {
    fontSize: 12,
  },
  categoryText: {
    fontSize: 11,
    fontWeight: '600',
    color: colors.surface,
    textTransform: 'uppercase',
  },
  announcementDate: {
    fontSize: 12,
    color: colors.textSubtle,
  },
  announcementTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 6,
  },
  announcementPreview: {
    fontSize: 14,
    color: colors.textMuted,
    lineHeight: 20,
    marginBottom: 12,
  },
  announcementFooter: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingTop: 12,
    borderTopWidth: 1,
    borderTopColor: colors.surfaceMuted,
  },
  authorText: {
    fontSize: 12,
    color: colors.textSubtle,
  },
  footerRight: {
    flexDirection: 'row',
    gap: 12,
  },
  attachmentBadge: {
    fontSize: 12,
    color: colors.textMuted,
  },
  commentsBadge: {
    fontSize: 12,
    color: colors.textMuted,
  },
  unreadDot: {
    position: 'absolute',
    top: 16,
    right: 16,
    width: 8,
    height: 8,
    borderRadius: 4,
    backgroundColor: colors.accent,
  },
  bottomSpacer: {
    height: 100,
  },
});
