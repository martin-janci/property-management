/**
 * AnnouncementDetailScreen — Story 6.2 read-receipt & acknowledgment flow (mobile).
 *
 * On mount:
 *   1. Fetches full announcement detail via GET /api/v1/announcements/{id}
 *   2. Fires a fire-and-forget POST /api/v1/announcements/{id}/read so the
 *      backend persists the per-user read state (idempotent upsert).
 *
 * Retry: TanStack Query default retry (3 attempts, exponential back-off) covers
 * transient network failures for the GET. The mark-read call uses apiRequest
 * directly and silently ignores transient failures — read-receipt is best-effort.
 *
 * Acknowledgment: shown only when `acknowledgment_required` is true.
 * Calls POST /api/v1/announcements/{id}/acknowledge and disables the button
 * on success so the user cannot double-submit.
 */

import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { ActivityIndicator, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { apiRequest, useApiMutation, useApiQuery } from '../../hooks/useApi';
import { formatDate as formatLocaleDate } from '../../i18n/format';
import { colors } from '../shared/screenStyles';

// ─── API shapes ──────────────────────────────────────────────────────────────

interface AnnouncementDetail {
  id: string;
  title: string;
  content: string;
  status: string;
  pinned: boolean;
  acknowledgment_required: boolean;
  comments_enabled: boolean;
  published_at?: string | null;
  created_at: string;
  read_count: number;
  acknowledged_count: number;
  comment_count: number;
  author_name?: string;
  target_type: string;
}

interface AnnouncementAttachment {
  id: string;
  file_name: string;
  file_type: string;
  file_size: number;
}

interface AnnouncementDetailResponse {
  announcement: AnnouncementDetail;
  attachments: AnnouncementAttachment[];
}

// ─── Query key factory ────────────────────────────────────────────────────────

export const announcementDetailKey = (id: string) => ['announcements', 'detail', id] as const;

// ─── Props ────────────────────────────────────────────────────────────────────

interface AnnouncementDetailScreenProps {
  announcementId: string;
  onBack: () => void;
}

// ─── Component ────────────────────────────────────────────────────────────────

export function AnnouncementDetailScreen({
  announcementId,
  onBack,
}: AnnouncementDetailScreenProps) {
  const queryClient = useQueryClient();
  const { t } = useTranslation();
  const markReadFired = useRef(false);

  // Fetch announcement detail
  const { data, isLoading, error } = useApiQuery<AnnouncementDetailResponse>(
    announcementDetailKey(announcementId),
    `/api/v1/announcements/${announcementId}`,
    { staleTime: 60_000 }
  );

  // Acknowledge mutation
  const acknowledge = useApiMutation<void, void>(
    `/api/v1/announcements/${announcementId}/acknowledge`,
    'POST'
  );

  // Fire mark-read on first render.
  // Guard ref prevents double-fire on StrictMode or re-mount.
  useEffect(() => {
    if (!markReadFired.current) {
      markReadFired.current = true;
      // Fire-and-forget — UI is not blocked by read-receipt.
      // Silently ignores failures; the user can still read the announcement.
      apiRequest<void>(`/api/v1/announcements/${announcementId}/read`, { method: 'POST' })
        .then(() => {
          queryClient.invalidateQueries({ queryKey: ['announcements', 'unread-count'] });
          queryClient.invalidateQueries({ queryKey: ['announcements', 'list'] });
        })
        .catch(() => {
          // best-effort — no blocking UX failure on missed read-receipt
        });
    }
  }, [announcementId, queryClient]);

  const handleAcknowledge = () => {
    acknowledge.mutate(undefined, {
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: announcementDetailKey(announcementId) });
        queryClient.invalidateQueries({ queryKey: ['announcements', 'list'] });
      },
    });
  };

  const formatDate = (dateString?: string | null): string => {
    if (!dateString) return '';
    return formatLocaleDate(dateString, {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  if (isLoading) {
    return (
      <View style={styles.center}>
        <ActivityIndicator size="large" color={colors.accent} />
        <Text style={styles.loadingText}>{t('common.loading')}</Text>
      </View>
    );
  }

  if (error || !data) {
    return (
      <View style={styles.container}>
        <Pressable style={styles.backButton} onPress={onBack}>
          <Text style={styles.backButtonText}>{t('announcements.detail.back')}</Text>
        </Pressable>
        <View style={styles.center}>
          <Text style={styles.errorTitle}>{t('announcements.detail.loadError')}</Text>
          <Text style={styles.errorText}>{error?.message ?? t('errors.unknown')}</Text>
        </View>
      </View>
    );
  }

  const { announcement, attachments } = data;
  const ackDone = acknowledge.isSuccess;

  return (
    <View style={styles.container}>
      <Pressable style={styles.backButton} onPress={onBack}>
        <Text style={styles.backButtonText}>{t('announcements.detail.backToList')}</Text>
      </Pressable>

      <ScrollView style={styles.scrollView} contentContainerStyle={styles.scrollContent}>
        {/* Header card */}
        <View style={styles.headerCard}>
          <View style={styles.badgeRow}>
            {announcement.pinned ? (
              <View style={styles.pinnedBadge}>
                <Text style={styles.pinnedBadgeText}>{t('announcements.pinned')}</Text>
              </View>
            ) : null}
            <View style={[styles.statusPill, statusPillStyle(announcement.status)]}>
              <Text style={[styles.statusPillText, statusPillTextStyle(announcement.status)]}>
                {announcement.status.charAt(0).toUpperCase() + announcement.status.slice(1)}
              </Text>
            </View>
          </View>

          <Text style={styles.title}>{announcement.title}</Text>

          <View style={styles.metaRow}>
            {announcement.author_name ? (
              <Text style={styles.metaText}>
                {t('announcements.by', { author: announcement.author_name })}
              </Text>
            ) : null}
            {announcement.published_at ? (
              <Text style={styles.metaText}>
                {t('announcements.detail.publishedAt', {
                  date: formatDate(announcement.published_at),
                })}
              </Text>
            ) : null}
          </View>

          {/* Stats row */}
          <View style={styles.statsRow}>
            <Text style={styles.statItem}>
              <Text style={styles.statNumber}>{announcement.read_count}</Text>{' '}
              {t('announcements.detail.reads')}
            </Text>
            {announcement.acknowledgment_required ? (
              <Text style={styles.statItem}>
                <Text style={styles.statNumber}>{announcement.acknowledged_count}</Text>{' '}
                {t('announcements.detail.acknowledged')}
              </Text>
            ) : null}
            {announcement.comments_enabled ? (
              <Text style={styles.statItem}>
                <Text style={styles.statNumber}>{announcement.comment_count}</Text>{' '}
                {t('announcements.detail.comments')}
              </Text>
            ) : null}
          </View>
        </View>

        {/* Body */}
        <View style={styles.bodyCard}>
          <Text style={styles.bodyText}>{announcement.content}</Text>
        </View>

        {/* Attachments */}
        {attachments.length > 0 ? (
          <View style={styles.card}>
            <Text style={styles.sectionTitle}>{t('announcements.detail.attachments')}</Text>
            {attachments.map((att) => (
              <View key={att.id} style={styles.attachmentRow}>
                <Text style={styles.attachmentName}>{att.file_name}</Text>
                <Text style={styles.attachmentSize}>{formatFileSize(att.file_size)}</Text>
              </View>
            ))}
          </View>
        ) : null}

        {/* Acknowledge button */}
        {announcement.acknowledgment_required ? (
          <View style={styles.ackCard}>
            <Text style={styles.ackLabel}>{t('announcements.detail.ackRequired')}</Text>
            {ackDone ? (
              <View style={styles.ackDoneBanner}>
                <Text style={styles.ackDoneText}>{t('announcements.detail.ackDone')}</Text>
              </View>
            ) : (
              <Pressable
                style={[styles.ackButton, acknowledge.isPending && styles.ackButtonDisabled]}
                onPress={handleAcknowledge}
                disabled={acknowledge.isPending}
              >
                {acknowledge.isPending ? (
                  <ActivityIndicator size="small" color={colors.surface} />
                ) : (
                  <Text style={styles.ackButtonText}>{t('announcements.detail.ackButton')}</Text>
                )}
              </Pressable>
            )}
            {acknowledge.isError ? (
              <Text style={styles.ackError}>
                {t('announcements.detail.ackError', {
                  message: acknowledge.error?.message ?? t('errors.unknown'),
                })}
              </Text>
            ) : null}
          </View>
        ) : null}

        <View style={styles.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

// ─── Status pill helpers ──────────────────────────────────────────────────────

function statusPillStyle(status: string): object {
  switch (status) {
    case 'published':
      return { backgroundColor: colors.successBg };
    case 'draft':
      return { backgroundColor: colors.surfaceMuted };
    case 'scheduled':
      return { backgroundColor: colors.infoBg };
    default:
      return { backgroundColor: colors.surfaceMuted };
  }
}

function statusPillTextStyle(status: string): object {
  switch (status) {
    case 'published':
      return { color: colors.successDark };
    case 'draft':
      return { color: colors.textMuted };
    case 'scheduled':
      return { color: colors.info };
    default:
      return { color: colors.textMuted };
  }
}

// ─── Styles ───────────────────────────────────────────────────────────────────

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background,
  },
  center: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    padding: 24,
  },
  loadingText: {
    marginTop: 12,
    fontSize: 14,
    color: colors.textMuted,
  },
  errorTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: colors.textSecondary,
    marginBottom: 4,
  },
  errorText: {
    fontSize: 14,
    color: colors.textMuted,
    textAlign: 'center',
  },
  backButton: {
    paddingHorizontal: 20,
    paddingTop: 56,
    paddingBottom: 12,
    backgroundColor: colors.surface,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  backButtonText: {
    fontSize: 15,
    color: colors.accent,
    fontWeight: '500',
  },
  scrollView: {
    flex: 1,
  },
  scrollContent: {
    padding: 16,
    gap: 12,
  },
  headerCard: {
    backgroundColor: colors.surface,
    borderRadius: 12,
    padding: 20,
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
  },
  badgeRow: {
    flexDirection: 'row',
    gap: 8,
    marginBottom: 12,
    flexWrap: 'wrap',
  },
  pinnedBadge: {
    paddingHorizontal: 8,
    paddingVertical: 3,
    borderRadius: 4,
    backgroundColor: colors.warningBg,
  },
  pinnedBadgeText: {
    fontSize: 11,
    fontWeight: '700',
    color: colors.warningInk,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  statusPill: {
    paddingHorizontal: 10,
    paddingVertical: 3,
    borderRadius: 4,
  },
  statusPillText: {
    fontSize: 12,
    fontWeight: '600',
  },
  title: {
    fontSize: 20,
    fontWeight: '700',
    color: colors.text,
    marginBottom: 10,
    lineHeight: 28,
  },
  metaRow: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
    marginBottom: 12,
  },
  metaText: {
    fontSize: 13,
    color: colors.textMuted,
  },
  statsRow: {
    flexDirection: 'row',
    gap: 16,
    borderTopWidth: 1,
    borderTopColor: colors.divider,
    paddingTop: 12,
    flexWrap: 'wrap',
  },
  statItem: {
    fontSize: 13,
    color: colors.textMuted,
  },
  statNumber: {
    fontWeight: '700',
    color: colors.text,
  },
  bodyCard: {
    backgroundColor: colors.surface,
    borderRadius: 12,
    padding: 20,
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
  },
  bodyText: {
    fontSize: 15,
    color: colors.text,
    lineHeight: 24,
  },
  card: {
    backgroundColor: colors.surface,
    borderRadius: 12,
    padding: 20,
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
  },
  sectionTitle: {
    fontSize: 13,
    fontWeight: '600',
    color: colors.textMuted,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: 12,
  },
  attachmentRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingVertical: 8,
    borderTopWidth: 1,
    borderTopColor: colors.divider,
  },
  attachmentName: {
    fontSize: 14,
    color: colors.text,
    flex: 1,
    marginRight: 12,
  },
  attachmentSize: {
    fontSize: 12,
    color: colors.textMuted,
  },
  ackCard: {
    backgroundColor: colors.surface,
    borderRadius: 12,
    padding: 20,
    borderWidth: 1,
    borderColor: colors.warning,
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
  },
  ackLabel: {
    fontSize: 14,
    color: colors.textSecondary,
    marginBottom: 16,
    lineHeight: 20,
  },
  ackButton: {
    backgroundColor: colors.accent,
    borderRadius: 8,
    paddingVertical: 14,
    alignItems: 'center',
    justifyContent: 'center',
    minHeight: 48,
  },
  ackButtonDisabled: {
    backgroundColor: colors.accentDisabled,
  },
  ackButtonText: {
    fontSize: 15,
    fontWeight: '600',
    color: colors.surface,
  },
  ackDoneBanner: {
    backgroundColor: colors.successBg,
    borderRadius: 8,
    padding: 14,
    alignItems: 'center',
  },
  ackDoneText: {
    fontSize: 14,
    fontWeight: '500',
    color: colors.successDark,
  },
  ackError: {
    marginTop: 8,
    fontSize: 12,
    color: colors.danger,
  },
  bottomSpacer: {
    height: 32,
  },
});
