import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
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

export interface AnnouncementAttachment {
  id: string;
  name: string;
  url: string;
  type: 'pdf' | 'image' | 'document';
}

export interface Announcement {
  id: string;
  title: string;
  createdAt: string;
  author: string;
  isRead: boolean;
  isPinned: boolean;
  attachments: AnnouncementAttachment[];
  commentsCount: number;
}

/**
 * API shape of an item in `GET /api/v1/announcements/published`.
 *
 * The published-list endpoint returns `AnnouncementSummary` rows, which are
 * intentionally lightweight: no `content` body, no author, no category. The
 * full body is fetched lazily by `AnnouncementDetailScreen`.
 */
export interface ApiAnnouncementSummary {
  id: string;
  title: string;
  status: string;
  target_type: string;
  published_at?: string | null;
  pinned: boolean;
  comments_enabled: boolean;
  acknowledgment_required: boolean;
}

/** Response shape of `GET /api/v1/announcements/published`. */
interface ApiAnnouncementListResponse {
  announcements?: ApiAnnouncementSummary[];
  count?: number;
  total?: number;
}

/** Coerce a published-list summary into the UI's Announcement shape. */
export function toUiAnnouncement(a: ApiAnnouncementSummary): Announcement {
  return {
    id: a.id,
    title: a.title,
    createdAt: a.published_at ?? new Date().toISOString(),
    author: 'Building Management',
    isRead: false,
    isPinned: a.pinned,
    attachments: [],
    commentsCount: 0,
  };
}

/** Extract the summary list from the published-list response. */
export function extractItems(
  data: ApiAnnouncementListResponse | undefined
): ApiAnnouncementSummary[] {
  if (!data) return [];
  return data.announcements ?? [];
}

/**
 * Derive the pinned sticky-band items from the published list (PR #943).
 *
 * PR #943 dropped the dedicated `?pinned=true` query and now derives pinned
 * items client-side from the same published list, keeping only `isPinned`
 * rows. This is the band shown above the main feed.
 */
export function derivePinnedItems(items: Announcement[]): Announcement[] {
  return items.filter((a) => a.isPinned);
}

/**
 * Build the main (non-pinned) feed from the published list (PR #943).
 *
 * Excludes pinned items (they live in the sticky band), matches the search
 * query against the TITLE only (the published summary has no `content` body
 * since PR #943), and sorts newest-first by `createdAt`.
 */
export function filterMainList(items: Announcement[], searchQuery: string): Announcement[] {
  const q = searchQuery.toLowerCase();
  return items
    .filter((a) => !a.isPinned)
    .filter((a) => q === '' || a.title.toLowerCase().includes(q))
    .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());
}

/**
 * Overlay the locally-tracked read state onto an announcement.
 *
 * Read state is tracked client-side in a `Set<id>` (the published summary
 * carries no per-user read flag). Once an id is in the set the row is read;
 * otherwise the row keeps whatever `isRead` the mapper produced.
 */
export function applyReadState(announcement: Announcement, readIds: Set<string>): Announcement {
  if (announcement.isRead || !readIds.has(announcement.id)) return announcement;
  return { ...announcement, isRead: true };
}

/** Build the read-state-aware UI list from a published-list response. */
export function buildAnnouncements(
  data: ApiAnnouncementListResponse | undefined,
  readIds: Set<string>
): Announcement[] {
  return extractItems(data)
    .map(toUiAnnouncement)
    .map((a) => applyReadState(a, readIds));
}

/** Number of unread rows in a UI list. */
export function countUnread(items: Announcement[]): number {
  return items.filter((a) => !a.isRead).length;
}

/**
 * Format a timestamp as a short relative label for the feed cards.
 *
 * < 1h → "Just now"; < 24h → "Nh ago"; < 7d → "Nd ago"; otherwise an
 * absolute "Mon D" date. `now` is injectable so the output is deterministic
 * under test.
 */
export function formatRelativeDate(dateString: string, now: Date = new Date()): string {
  const date = new Date(dateString);
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
}

interface AnnouncementsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function AnnouncementsScreen({ onNavigate }: AnnouncementsScreenProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');
  const [readIds, setReadIds] = useState<Set<string>>(new Set());

  // Resident-facing published list. The manager-only `/api/v1/announcements`
  // endpoint 403s for residents — the published endpoint is RLS-scoped and
  // readable by any authenticated tenant member.
  const { data, isLoading, error, refetch, isFetching } = useApiQuery<ApiAnnouncementListResponse>(
    ['announcements', 'list'],
    '/api/v1/announcements/published',
    { staleTime: 60_000 }
  );

  // Story 6.4 — pinned items are derived client-side from the published list
  // (the published endpoint already orders `pinned DESC`). A dedicated query
  // is unnecessary and the endpoint does not accept a `pinned` filter.
  const allRaw: Announcement[] = buildAnnouncements(data, readIds);
  const pinnedItems: Announcement[] = derivePinnedItems(allRaw);

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  const markAsRead = (id: string) => {
    setReadIds((prev) => {
      if (prev.has(id)) return prev;
      const next = new Set(prev);
      next.add(id);
      return next;
    });
  };

  // Main list excludes pinned items (they appear in the sticky band above)
  const filteredAnnouncements = filterMainList(allRaw, searchQuery);

  const unreadCount = countUnread(allRaw);

  const handleCardPress = (announcement: Announcement) => {
    markAsRead(announcement.id);
    onNavigate?.('AnnouncementDetail', { announcementId: announcement.id });
  };

  return (
    <View style={styles.container}>
      {/* Header */}
      <View style={styles.header}>
        <View>
          <Text style={styles.headerTitle}>Announcements</Text>
          {unreadCount > 0 && <Text style={styles.unreadBadge}>{unreadCount} unread</Text>}
        </View>
      </View>

      {/* Story 6.4 — Pinned announcements sticky band */}
      {pinnedItems.length > 0 && (
        <View style={styles.pinnedBand}>
          <View style={styles.pinnedBandLabel}>
            <Text style={styles.pinnedBandLabelText}>Pinned</Text>
          </View>
          <ScrollView
            horizontal
            showsHorizontalScrollIndicator={false}
            contentContainerStyle={styles.pinnedBandChips}
          >
            {pinnedItems.map((ann) => (
              <Pressable
                key={ann.id}
                style={styles.pinnedChip}
                onPress={() => handleCardPress(ann)}
              >
                <Text style={styles.pinnedChipText} numberOfLines={1}>
                  {ann.title}
                </Text>
              </Pressable>
            ))}
          </ScrollView>
        </View>
      )}

      {/* Search */}
      <View style={styles.searchContainer}>
        <TextInput
          style={styles.searchInput}
          placeholder="Search announcements..."
          value={searchQuery}
          onChangeText={setSearchQuery}
        />
      </View>

      {/* Announcements List */}
      <ScrollView
        style={styles.scrollView}
        refreshControl={
          <RefreshControl refreshing={isFetching} onRefresh={onRefresh} tintColor={colors.accent} />
        }
      >
        {isLoading ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyTitle}>{t('common.loading')}</Text>
          </View>
        ) : error ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyIcon}>⚠️</Text>
            <Text style={styles.emptyTitle}>{t('announcements.loadError')}</Text>
          </View>
        ) : filteredAnnouncements.length === 0 ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyIcon}>📭</Text>
            <Text style={styles.emptyTitle}>{t('announcements.empty')}</Text>
            <Text style={styles.emptyText}>{t('announcements.emptyMessage')}</Text>
          </View>
        ) : (
          filteredAnnouncements.map((announcement) => (
            <Pressable
              key={announcement.id}
              style={[styles.announcementCard, !announcement.isRead && styles.unreadCard]}
              onPress={() => handleCardPress(announcement)}
            >
              <View style={styles.announcementHeader}>
                <Text style={styles.announcementDate}>
                  {formatRelativeDate(announcement.createdAt)}
                </Text>
              </View>

              <Text style={styles.announcementTitle}>{announcement.title}</Text>

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
  // Story 6.4 — pinned band
  pinnedBand: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.warningBg,
    borderBottomWidth: 1,
    borderBottomColor: colors.warning,
    paddingVertical: 8,
    paddingHorizontal: 16,
    minHeight: 44,
  },
  pinnedBandLabel: {
    flexDirection: 'row',
    alignItems: 'center',
    marginRight: 8,
    flexShrink: 0,
  },
  pinnedBandLabelText: {
    fontSize: 11,
    fontWeight: '700',
    color: colors.warningInk,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  pinnedBandChips: {
    flexDirection: 'row',
    gap: 8,
    alignItems: 'center',
  },
  pinnedChip: {
    paddingHorizontal: 12,
    paddingVertical: 4,
    borderRadius: 999,
    backgroundColor: 'rgba(245, 158, 11, 0.15)',
    borderWidth: 1,
    borderColor: colors.warning,
    maxWidth: 200,
  },
  pinnedChipText: {
    fontSize: 13,
    fontWeight: '500',
    color: colors.warningInk,
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
  announcementHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 8,
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
