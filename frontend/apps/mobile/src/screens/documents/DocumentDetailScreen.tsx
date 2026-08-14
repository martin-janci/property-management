/**
 * DocumentDetailScreen (UC-08.4 + Story 7A.5).
 *
 * Single-document view with metadata, an "open" action that fetches a
 * short-lived presigned URL and hands it off to the platform viewer, and a
 * "Share" button that opens DocumentShareSheet.
 *
 * Data comes from GET /api/v1/documents/{id} (wrapped as `{ document: ... }`).
 * That payload carries no file URL, so the open action calls the presigned
 * /preview endpoint (falling back to /download) — mirroring
 * DocumentPreviewScreen.
 */

import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Alert, Linking, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { apiRequest, useApiQuery } from '../../hooks/useApi';
import { formatDate } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';
import { isPreviewUnsupportedError } from './DocumentPreviewScreen';
import { DocumentShareSheet } from './DocumentShareSheet';

// ─── Types ───────────────────────────────────────────────────────────────────────

/**
 * Shape of GET /api/v1/documents/{id}. The server wraps the document in a
 * `document` key (DocumentDetailResponse) and flattens DocumentWithDetails, so
 * the base Document fields sit alongside the *_name / share_count extras.
 */
interface ApiDocumentDetailResponse {
  document: {
    id: string;
    title: string;
    description?: string | null;
    category: string;
    file_name: string;
    mime_type: string;
    size_bytes: number;
    created_at: string;
    updated_at: string;
    created_by_name?: string;
    folder_name?: string | null;
  };
}

/** Shape of UrlResponse from GET /api/v1/documents/{id}/preview|download. */
interface PresignedUrlResponse {
  url: string;
  expires_at: string;
}

// ─── Helpers ───────────────────────────────────────────────────────────────────

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function getFileIcon(mimeType: string, fileName: string): string {
  const mime = (mimeType ?? '').toLowerCase();
  const name = (fileName ?? '').toLowerCase();
  if (mime.includes('pdf') || name.endsWith('.pdf')) return '📄';
  if (mime.startsWith('image/') || /\.(png|jpe?g|gif|webp)$/.test(name)) return '🖼️';
  if (mime.includes('spreadsheet') || mime.includes('excel') || /\.(xlsx?|csv)$/.test(name))
    return '📊';
  return '🗂️';
}

// ─── Component ───────────────────────────────────────────────────────────────

interface DocumentDetailScreenProps {
  documentId?: string;
  onBack?: () => void;
}

export function DocumentDetailScreen({ documentId, onBack }: DocumentDetailScreenProps) {
  const { t } = useTranslation();
  const [shareSheetVisible, setShareSheetVisible] = useState(false);
  const [opening, setOpening] = useState(false);

  const { data, isLoading, error, refetch } = useApiQuery<ApiDocumentDetailResponse>(
    ['documents', 'detail', documentId ?? ''],
    `/api/v1/documents/${documentId ?? ''}`,
    { staleTime: 60_000, enabled: !!documentId }
  );

  const document = data?.document;

  const handleRetry = useCallback(() => {
    void refetch();
  }, [refetch]);

  /**
   * Fetch a presigned URL (preview, with download fallback) and hand it to the
   * native viewer. The detail payload carries no file URL of its own.
   */
  const handleOpen = useCallback(async () => {
    if (!documentId) return;
    setOpening(true);
    try {
      let presigned: PresignedUrlResponse;
      try {
        presigned = await apiRequest<PresignedUrlResponse>(
          `/api/v1/documents/${documentId}/preview`
        );
      } catch (err) {
        if (!isPreviewUnsupportedError(err)) throw err;
        presigned = await apiRequest<PresignedUrlResponse>(
          `/api/v1/documents/${documentId}/download`
        );
      }

      const supported = await Linking.canOpenURL(presigned.url);
      if (!supported) throw new Error('Cannot open URL');
      await Linking.openURL(presigned.url);
    } catch {
      Alert.alert('Could not open document', 'The file viewer is unavailable on this device.');
    } finally {
      setOpening(false);
    }
  }, [documentId]);

  const title = document?.title ?? '';

  return (
    <View style={s.container}>
      <View style={s.header}>
        {onBack && (
          <Pressable onPress={onBack} style={styles.backLink}>
            <Text style={styles.backLinkText}>← Documents</Text>
          </Pressable>
        )}
        <Text style={s.headerTitle} numberOfLines={2}>
          {title || 'Document'}
        </Text>
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
            <Text style={s.emptyTitle}>{t('documents.detailLoadError')}</Text>
            <Pressable style={[s.primaryButton, styles.retryButton]} onPress={handleRetry}>
              <Text style={s.primaryButtonText}>{t('common.retry')}</Text>
            </Pressable>
          </View>
        )}

        {!isLoading && !error && !document && (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>📄</Text>
            <Text style={s.emptyTitle}>{t('documents.notFound')}</Text>
          </View>
        )}

        {!isLoading && !error && document && (
          <>
            <View style={s.card}>
              <View style={s.cardHeader}>
                <Text style={styles.icon}>
                  {getFileIcon(document.mime_type, document.file_name)}
                </Text>
                <Text style={[s.cardTitle, { marginLeft: 8 }]} numberOfLines={2}>
                  {document.title}
                </Text>
              </View>
              <View style={[s.cardFooter, { marginTop: 16 }]}>
                {typeof document.size_bytes === 'number' && document.size_bytes > 0 && (
                  <Text style={s.cardMeta}>{formatBytes(document.size_bytes)}</Text>
                )}
                <Text style={s.cardMeta}>Updated {formatDate(document.updated_at)}</Text>
              </View>
            </View>

            <View style={s.card}>
              <Text style={styles.sectionLabel}>Category</Text>
              <Text style={styles.sectionValue}>{document.category}</Text>
              <Text style={[styles.sectionLabel, { marginTop: 12 }]}>File</Text>
              <Text style={styles.sectionValue}>{document.file_name}</Text>
              <Text style={[styles.sectionLabel, { marginTop: 12 }]}>Created</Text>
              <Text style={styles.sectionValue}>{formatDate(document.created_at)}</Text>
            </View>

            <Pressable
              style={[s.primaryButton, opening && styles.buttonDisabled]}
              onPress={() => void handleOpen()}
              disabled={opening}
            >
              <Text style={s.primaryButtonText}>{opening ? 'Opening…' : 'Open document'}</Text>
            </Pressable>

            <Pressable style={styles.shareButton} onPress={() => setShareSheetVisible(true)}>
              <Text style={styles.shareButtonText}>🔗 Share</Text>
            </Pressable>

            <DocumentShareSheet
              documentId={document.id}
              documentTitle={document.title}
              visible={shareSheetVisible}
              onClose={() => setShareSheetVisible(false)}
            />
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
  icon: { fontSize: 24 },
  buttonDisabled: { opacity: 0.6 },
  sectionLabel: {
    fontSize: 12,
    color: colors.textMuted,
    fontWeight: '600',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  sectionValue: { fontSize: 16, color: colors.text, marginTop: 4 },
  shareButton: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 14,
    paddingHorizontal: 16,
    borderRadius: 8,
    backgroundColor: colors.surface,
    borderWidth: 1,
    borderColor: colors.accent,
    marginTop: 12,
  },
  shareButtonText: {
    color: colors.accent,
    fontSize: 16,
    fontWeight: '600',
  },
});
