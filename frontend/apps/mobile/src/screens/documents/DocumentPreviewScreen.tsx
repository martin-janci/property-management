/**
 * DocumentPreviewScreen — Story 7A.4 (Document Download & Preview, mobile slice).
 *
 * Calls GET /api/v1/documents/{id}/preview to obtain a short-lived presigned
 * URL (1 h) then hands it to the platform's native viewer via Linking.openURL
 * so the OS can render PDFs, images, office docs, etc. without bundling a
 * heavyweight in-app renderer (contrast: web uses react-pdf — PR #446).
 *
 * If the backend returns PREVIEW_NOT_SUPPORTED (400) we fall back to the
 * download URL (GET /api/v1/documents/{id}/download) so the user can at
 * least save the file.
 *
 * Sharing is delegated to expo-sharing after the file is downloaded locally
 * via expo-file-system. The share sheet is only shown when Sharing.isAvailable.
 */

import * as FileSystem from 'expo-file-system/legacy';
import * as Sharing from 'expo-sharing';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
  Alert,
  Linking,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { apiRequest } from '../../hooks/useApi';
import { formatDate, formatTime } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';
import type { Document } from './DocumentsScreen';

// ─── Types ────────────────────────────────────────────────────────────────────

/** Shape of UrlResponse from GET /api/v1/documents/{id}/preview|download */
interface PresignedUrlResponse {
  url: string;
  expires_at: string;
}

type PreviewState =
  | { status: 'idle' }
  | { status: 'loading' }
  | { status: 'ready'; url: string; expiresAt: Date }
  | { status: 'fallback'; url: string; expiresAt: Date } // download URL used as fallback
  | { status: 'error'; message: string };

interface DocumentPreviewScreenProps {
  /** Full document object as passed from DocumentsScreen. */
  document: Document;
  onBack?: () => void;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Human-readable file size. Exported so the size label shown next to a
 * downloaded/previewed document can be unit-tested at the boundaries
 * (B / KB / MB) without rendering the RN tree.
 */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Decide whether a failed /preview request means "this file type can't be
 * previewed, fall back to /download" (true) versus a genuine error to surface
 * (false). The backend signals an unsupported preview with PREVIEW_NOT_SUPPORTED
 * / HTTP 400; anything else (auth, network, 5xx) is a hard failure.
 *
 * Exported so the fallback decision can be unit-tested independently of the
 * RN render tree, and reused by DocumentDetailScreen's open action.
 */
export function isPreviewUnsupportedError(err: unknown): boolean {
  const message = err instanceof Error ? err.message : '';
  return (
    message.includes('PREVIEW_NOT_SUPPORTED') ||
    message.includes('not supported') ||
    message.includes('HTTP 400')
  );
}

function getFileIcon(type: Document['type']): string {
  switch (type) {
    case 'pdf':
      return '📄';
    case 'image':
      return '🖼️';
    case 'spreadsheet':
      return '📊';
    default:
      return '🗂️';
  }
}

/**
 * Download a remote file to the device cache directory and return the local
 * URI. Used before invoking expo-sharing (which requires a local URI).
 *
 * Exported so the cache-reuse behaviour (skip the network round-trip when a
 * session-local copy already exists) can be unit-tested with a mocked
 * expo-file-system — this is the load-bearing part of the download path that
 * avoids re-fetching the presigned blob on every share.
 */
export async function downloadToCache(url: string, filename: string): Promise<string> {
  const dest = `${FileSystem.cacheDirectory ?? ''}${filename}`;
  const info = await FileSystem.getInfoAsync(dest);
  // Re-use cached copy if it's already there (session-local cache).
  if (info.exists) return dest;

  const { uri } = await FileSystem.downloadAsync(url, dest);
  return uri;
}

// ─── Component ────────────────────────────────────────────────────────────────

export function DocumentPreviewScreen({ document, onBack }: DocumentPreviewScreenProps) {
  const { t } = useTranslation();
  const [state, setState] = useState<PreviewState>({ status: 'idle' });
  const [isSharing, setIsSharing] = useState(false);

  // ── Fetch presigned URL on mount ──────────────────────────────────────────

  const fetchPreviewUrl = useCallback(async () => {
    setState({ status: 'loading' });
    try {
      const data = await apiRequest<PresignedUrlResponse>(
        `/api/v1/documents/${document.id}/preview`
      );
      setState({
        status: 'ready',
        url: data.url,
        expiresAt: new Date(data.expires_at),
      });
    } catch (err) {
      // PREVIEW_NOT_SUPPORTED (400) → fall back to download endpoint
      if (isPreviewUnsupportedError(err)) {
        try {
          const fallback = await apiRequest<PresignedUrlResponse>(
            `/api/v1/documents/${document.id}/download`
          );
          setState({
            status: 'fallback',
            url: fallback.url,
            expiresAt: new Date(fallback.expires_at),
          });
        } catch (fallbackErr) {
          const fallbackMessage =
            fallbackErr instanceof Error ? fallbackErr.message : t('errors.generic');
          setState({ status: 'error', message: fallbackMessage });
        }
      } else {
        setState({
          status: 'error',
          message: err instanceof Error ? err.message : t('errors.generic'),
        });
      }
    }
  }, [document.id, t]);

  useEffect(() => {
    void fetchPreviewUrl();
  }, [fetchPreviewUrl]);

  // ── Open in native viewer ─────────────────────────────────────────────────

  const handleOpen = useCallback(async () => {
    if (state.status !== 'ready' && state.status !== 'fallback') return;
    try {
      const canOpen = await Linking.canOpenURL(state.url);
      if (!canOpen) {
        Alert.alert(
          t('documents.preview.cannotOpenTitle'),
          t('documents.preview.cannotOpenMessage')
        );
        return;
      }
      await Linking.openURL(state.url);
    } catch {
      Alert.alert(t('common.error'), t('documents.preview.openFailed'));
    }
  }, [state, t]);

  // ── Share via expo-sharing ────────────────────────────────────────────────

  const handleShare = useCallback(async () => {
    if (state.status !== 'ready' && state.status !== 'fallback') return;
    const sharingAvailable = await Sharing.isAvailableAsync();
    if (!sharingAvailable) {
      Alert.alert(t('documents.preview.sharingUnavailableTitle'), '');
      return;
    }

    setIsSharing(true);
    try {
      const localUri = await downloadToCache(state.url, document.name);
      await Sharing.shareAsync(localUri, {
        mimeType: mimeTypeFromDocType(document.type),
        dialogTitle: document.name,
        UTI: utiFromDocType(document.type), // iOS only
      });
    } catch (_err) {
      Alert.alert(t('common.error'), t('documents.preview.shareFailed'));
    } finally {
      setIsSharing(false);
    }
  }, [state, document, t]);

  // ── Render ────────────────────────────────────────────────────────────────

  const isReady = state.status === 'ready' || state.status === 'fallback';
  const isFallback = state.status === 'fallback';

  return (
    <View style={s.container}>
      {/* Header */}
      <View style={s.header}>
        {onBack && (
          <Pressable onPress={onBack} style={styles.backLink}>
            <Text style={styles.backLinkText}>← {t('documents.title')}</Text>
          </Pressable>
        )}
        <Text style={s.headerTitle} numberOfLines={2}>
          {document.name}
        </Text>
      </View>

      <ScrollView style={s.scrollView}>
        {/* Document metadata card */}
        <View style={s.card}>
          <View style={s.cardHeader}>
            <Text style={styles.fileIcon}>{getFileIcon(document.type)}</Text>
            <Text style={[s.cardTitle, { marginLeft: 8 }]} numberOfLines={2}>
              {document.name}
            </Text>
          </View>
          <View style={[s.cardFooter, { marginTop: 16 }]}>
            {typeof document.size === 'number' && document.size > 0 && (
              <Text style={s.cardMeta}>{formatBytes(document.size)}</Text>
            )}
            <Text style={s.cardMeta}>
              {t('documents.preview.updated')} {formatDate(document.updatedAt)}
            </Text>
          </View>
        </View>

        {/* Status / URL card */}
        {state.status === 'loading' && (
          <View style={[s.card, styles.loadingCard]}>
            <ActivityIndicator color={colors.accent} size="small" />
            <Text style={styles.loadingText}>{t('documents.preview.fetching')}</Text>
          </View>
        )}

        {state.status === 'error' && (
          <View style={[s.card, styles.errorCard]}>
            <Text style={styles.errorIcon}>⚠️</Text>
            <Text style={styles.errorTitle}>{t('documents.preview.errorTitle')}</Text>
            <Text style={styles.errorMessage}>{state.message}</Text>
            <Pressable style={styles.retryButton} onPress={() => void fetchPreviewUrl()}>
              <Text style={styles.retryButtonText}>{t('common.retry')}</Text>
            </Pressable>
          </View>
        )}

        {isReady && (
          <>
            {isFallback && (
              <View style={styles.fallbackBanner}>
                <Text style={styles.fallbackText}>{t('documents.preview.fallbackNotice')}</Text>
              </View>
            )}

            {/* Expiry notice */}
            <View style={[s.card, styles.urlCard]}>
              <Text style={styles.urlLabel}>{t('documents.preview.urlReady')}</Text>
              <Text style={styles.urlExpiry}>
                {t('documents.preview.expiresAt')}{' '}
                {formatTime(state.expiresAt, {
                  hour: '2-digit',
                  minute: '2-digit',
                })}
              </Text>
            </View>

            {/* Open action */}
            <Pressable style={s.primaryButton} onPress={() => void handleOpen()}>
              <Text style={s.primaryButtonText}>
                {isFallback
                  ? t('documents.preview.downloadAction')
                  : t('documents.preview.openAction')}
              </Text>
            </Pressable>

            {/* Share action */}
            <Pressable
              style={[styles.shareButton, isSharing && styles.shareButtonDisabled]}
              onPress={() => void handleShare()}
              disabled={isSharing}
            >
              {isSharing ? (
                <ActivityIndicator color={colors.accent} size="small" />
              ) : (
                <Text style={styles.shareButtonText}>🔗 {t('documents.share')}</Text>
              )}
            </Pressable>
          </>
        )}

        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

// ─── MIME / UTI helpers ───────────────────────────────────────────────────────

/**
 * Map the app's coarse document type to a MIME type for expo-sharing.
 * Exported for unit testing — a wrong MIME type silently breaks the
 * Android share intent target resolution.
 */
export function mimeTypeFromDocType(type: Document['type']): string {
  switch (type) {
    case 'pdf':
      return 'application/pdf';
    case 'image':
      return 'image/*';
    case 'spreadsheet':
      return 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet';
    default:
      return 'application/octet-stream';
  }
}

/** iOS UTI used by expo-sharing. Ignored on Android. Exported for unit testing. */
export function utiFromDocType(type: Document['type']): string | undefined {
  if (!Platform.OS) return undefined; // safety guard for test env
  switch (type) {
    case 'pdf':
      return 'com.adobe.pdf';
    case 'image':
      return 'public.image';
    default:
      return 'public.data';
  }
}

// ─── Styles ───────────────────────────────────────────────────────────────────

const styles = StyleSheet.create({
  backLink: { marginBottom: 8 },
  backLinkText: { color: colors.accent, fontSize: 14 },
  fileIcon: { fontSize: 28 },

  // Loading state
  loadingCard: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 12,
    paddingVertical: 20,
  },
  loadingText: {
    fontSize: 14,
    color: colors.textMuted,
  },

  // Error state
  errorCard: {
    alignItems: 'center',
    paddingVertical: 24,
    gap: 8,
  },
  errorIcon: { fontSize: 32 },
  errorTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: colors.text,
  },
  errorMessage: {
    fontSize: 13,
    color: colors.textMuted,
    textAlign: 'center',
    paddingHorizontal: 8,
  },
  retryButton: {
    marginTop: 8,
    paddingVertical: 10,
    paddingHorizontal: 20,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.accent,
  },
  retryButtonText: {
    color: colors.accent,
    fontWeight: '600',
  },

  // Fallback banner
  fallbackBanner: {
    backgroundColor: colors.warningBg,
    borderRadius: 8,
    paddingVertical: 10,
    paddingHorizontal: 14,
    marginBottom: 8,
  },
  fallbackText: {
    fontSize: 13,
    color: colors.warningDark,
  },

  // URL ready card
  urlCard: {
    backgroundColor: colors.successBg,
    borderRadius: 8,
    paddingVertical: 12,
  },
  urlLabel: {
    fontSize: 14,
    fontWeight: '600',
    color: colors.successDark,
    marginBottom: 2,
  },
  urlExpiry: {
    fontSize: 12,
    color: colors.textMuted,
  },

  // Share button
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
    gap: 6,
  },
  shareButtonDisabled: {
    borderColor: colors.accentDisabled,
  },
  shareButtonText: {
    color: colors.accent,
    fontSize: 16,
    fontWeight: '600',
  },
});
