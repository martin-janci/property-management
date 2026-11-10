/**
 * DocumentPermissionsScreen (gap-7a-3 — Permission-Based Document Access).
 *
 * Shows the access_scope (audience) and publication status of a document so
 * residents and managers can understand who can see it. This is the mobile
 * counterpart of the web's audience-badge column in DocumentsBrowse.
 *
 * The backend enforces PostgreSQL RLS — this screen only *visualises* the
 * permission model, it does not grant or restrict access itself.
 */

import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { formatDate as formatLocaleDate } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';

// ─── types ───────────────────────────────────────────────────────────────────

export type AccessScope = 'organization' | 'building' | 'unit' | 'user' | 'public';
export type DocumentStatus = 'published' | 'draft' | 'archived';

export interface DocumentPermission {
  id: string;
  title: string;
  access_scope?: AccessScope;
  status?: DocumentStatus;
  category?: string;
  created_at: string;
  updated_at: string;
}

interface ApiDocumentDetailResponse {
  id: string;
  title: string;
  access_scope?: AccessScope;
  status?: DocumentStatus;
  category?: string;
  created_at: string;
  updated_at: string;
}

// ─── helpers ─────────────────────────────────────────────────────────────────

interface AudienceMeta {
  label: string;
  description: string;
  color: string;
  bg: string;
  icon: string;
}

function getAudienceMeta(scope: AccessScope | undefined): AudienceMeta {
  switch (scope) {
    case 'organization':
      return {
        label: 'All Residents',
        description: 'Visible to every resident in the organization.',
        color: colors.accent,
        bg: colors.accentSoft,
        icon: '🏘️',
      };
    case 'building':
      return {
        label: 'Building Only',
        description: 'Visible to residents of the specific building.',
        color: '#4f46e5',
        bg: '#ede9fe',
        icon: '🏢',
      };
    case 'unit':
      return {
        label: 'Unit Only',
        description: 'Visible only to occupants of a specific unit.',
        color: colors.warning,
        bg: colors.warningBg,
        icon: '🚪',
      };
    case 'user':
      return {
        label: 'Specific Users',
        description: 'Visible only to explicitly named users.',
        color: colors.warningDark,
        bg: colors.warningBg,
        icon: '👤',
      };
    case 'public':
      return {
        label: 'Public',
        description: 'Visible to anyone, including unauthenticated users.',
        color: colors.success,
        bg: colors.successBg,
        icon: '🌍',
      };
    default:
      return {
        label: 'All Residents',
        description: 'Visible to every resident in the organization.',
        color: colors.accent,
        bg: colors.accentSoft,
        icon: '🏘️',
      };
  }
}

interface StatusMeta {
  label: string;
  color: string;
  bg: string;
}

function getStatusMeta(status: DocumentStatus | undefined): StatusMeta {
  switch (status) {
    case 'draft':
      return { label: 'Draft', color: colors.warningDark, bg: colors.warningBg };
    case 'archived':
      return { label: 'Archived', color: colors.textMuted, bg: colors.surfaceMuted };
    default:
      return { label: 'Published', color: colors.successDark, bg: colors.successBg };
  }
}

function formatDate(iso: string): string {
  return formatLocaleDate(iso, {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  });
}

// ─── component ───────────────────────────────────────────────────────────────

export interface DocumentPermissionsScreenProps {
  documentId: string;
  onBack?: () => void;
}

export function DocumentPermissionsScreen({ documentId, onBack }: DocumentPermissionsScreenProps) {
  const { t } = useTranslation();

  const { data, isLoading, error, refetch } = useApiQuery<ApiDocumentDetailResponse>(
    ['documents', 'detail', documentId],
    `/api/v1/documents/${documentId}`,
    { staleTime: 60_000 }
  );

  const handleRetry = useCallback(() => {
    void refetch();
  }, [refetch]);

  const audienceMeta = getAudienceMeta(data?.access_scope);
  const statusMeta = getStatusMeta(data?.status);

  return (
    <View style={s.container}>
      {/* Header */}
      <View style={s.header}>
        {onBack && (
          <Pressable onPress={onBack} style={styles.backLink}>
            <Text style={styles.backLinkText}>← {t('documents.title')}</Text>
          </Pressable>
        )}
        <Text style={s.headerTitle}>{t('documents.permissions.title')}</Text>
        {data?.title ? (
          <Text style={s.headerSubtitle} numberOfLines={1}>
            {data.title}
          </Text>
        ) : null}
      </View>

      <ScrollView style={s.scrollView}>
        {isLoading && (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>{t('common.loading')}</Text>
          </View>
        )}

        {error && !isLoading && (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>{t('documents.permissions.loadError')}</Text>
            <Pressable style={[s.primaryButton, styles.retryButton]} onPress={handleRetry}>
              <Text style={s.primaryButtonText}>{t('common.retry')}</Text>
            </Pressable>
          </View>
        )}

        {!isLoading && !error && data && (
          <>
            {/* Audience card */}
            <View style={s.card}>
              <Text style={styles.sectionLabel}>{t('documents.permissions.audience')}</Text>
              <View style={[styles.audienceBadge, { backgroundColor: audienceMeta.bg }]}>
                <Text style={styles.audienceIcon}>{audienceMeta.icon}</Text>
                <View style={styles.audienceBody}>
                  <Text style={[styles.audienceLabel, { color: audienceMeta.color }]}>
                    {audienceMeta.label}
                  </Text>
                  <Text style={styles.audienceDesc}>{audienceMeta.description}</Text>
                </View>
              </View>
            </View>

            {/* Status card */}
            <View style={s.card}>
              <Text style={styles.sectionLabel}>{t('documents.permissions.status')}</Text>
              <View style={[styles.statusPill, { backgroundColor: statusMeta.bg }]}>
                <Text style={[styles.statusText, { color: statusMeta.color }]}>
                  {statusMeta.label}
                </Text>
              </View>
              {data.status === 'draft' && (
                <Text style={styles.statusNote}>{t('documents.permissions.draftNote')}</Text>
              )}
              {data.status === 'archived' && (
                <Text style={styles.statusNote}>{t('documents.permissions.archivedNote')}</Text>
              )}
            </View>

            {/* Metadata card */}
            <View style={s.card}>
              <Text style={styles.sectionLabel}>{t('documents.permissions.metadata')}</Text>
              {data.category ? (
                <View style={styles.metaRow}>
                  <Text style={styles.metaKey}>{t('documents.permissions.category')}</Text>
                  <Text style={styles.metaValue}>{data.category}</Text>
                </View>
              ) : null}
              <View style={styles.metaRow}>
                <Text style={styles.metaKey}>{t('documents.permissions.created')}</Text>
                <Text style={styles.metaValue}>{formatDate(data.created_at)}</Text>
              </View>
              <View style={styles.metaRow}>
                <Text style={styles.metaKey}>{t('documents.permissions.updated')}</Text>
                <Text style={styles.metaValue}>{formatDate(data.updated_at)}</Text>
              </View>
            </View>

            {/* RLS explanation card */}
            <View style={[s.card, styles.infoCard]}>
              <Text style={styles.infoTitle}>{t('documents.permissions.rlsTitle')}</Text>
              <Text style={styles.infoBody}>{t('documents.permissions.rlsBody')}</Text>
            </View>
          </>
        )}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  backLink: { marginBottom: 8 },
  backLinkText: { color: colors.accent, fontSize: 14 },
  retryButton: { marginTop: 16 },
  sectionLabel: {
    fontSize: 11,
    fontWeight: '700',
    textTransform: 'uppercase',
    letterSpacing: 0.8,
    color: colors.textSubtle,
    marginBottom: 10,
  },
  audienceBadge: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    borderRadius: 10,
    padding: 14,
    gap: 12,
  },
  audienceIcon: {
    fontSize: 28,
    lineHeight: 32,
  },
  audienceBody: {
    flex: 1,
  },
  audienceLabel: {
    fontSize: 16,
    fontWeight: '700',
    marginBottom: 4,
  },
  audienceDesc: {
    fontSize: 13,
    color: colors.textSecondary,
    lineHeight: 18,
  },
  statusPill: {
    alignSelf: 'flex-start',
    paddingHorizontal: 14,
    paddingVertical: 6,
    borderRadius: 999,
    marginBottom: 8,
  },
  statusText: {
    fontSize: 13,
    fontWeight: '700',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  statusNote: {
    fontSize: 13,
    color: colors.textMuted,
    lineHeight: 18,
  },
  metaRow: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingVertical: 6,
    borderBottomWidth: 1,
    borderBottomColor: colors.divider,
  },
  metaKey: {
    fontSize: 13,
    color: colors.textMuted,
  },
  metaValue: {
    fontSize: 13,
    fontWeight: '500',
    color: colors.text,
  },
  infoCard: {
    backgroundColor: colors.accentSoft,
    borderWidth: 1,
    borderColor: colors.accentDisabled,
  },
  infoTitle: {
    fontSize: 13,
    fontWeight: '700',
    color: colors.accent,
    marginBottom: 6,
  },
  infoBody: {
    fontSize: 13,
    color: colors.textSecondary,
    lineHeight: 18,
  },
});
