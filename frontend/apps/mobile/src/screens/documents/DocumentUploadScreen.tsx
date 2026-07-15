/**
 * DocumentUploadScreen (7A-1 Document Upload with Metadata).
 *
 * Lets a user pick a file from the device (via expo-document-picker or
 * expo-image-picker for photos) then fill in title, category,
 * description, and optional building context before uploading to
 * POST /api/v1/documents/upload.
 *
 * Upload is performed via the mobile-aware `apiRequest` helper
 * (multipart/form-data with bearer token + X-Tenant-ID headers).
 */

import * as DocumentPicker from 'expo-document-picker';
import * as ImagePicker from 'expo-image-picker';
import * as SecureStore from 'expo-secure-store';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
  Alert,
  KeyboardAvoidingView,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { getApiBaseUrl } from '../../config/api';
import { extractTenantId } from '../../utils/jwt';
import { colors } from '../shared/screenStyles';

// ─── Constants ────────────────────────────────────────────────────────────────

const ACCESS_TOKEN_KEY = 'ppt_access_token';

/**
 * Client-side upload guards — kept in sync with the backend's
 * `MAX_FILE_SIZE` / `ALLOWED_MIME_TYPES` on `POST /api/v1/documents/upload`
 * (`backend/crates/db/src/models/document.rs`). Enforcing them here rejects
 * an oversize / disallowed file *before* the multipart body is streamed over
 * cellular data, instead of letting the user find out via the server's 422.
 */
const MAX_FILE_SIZE = 50 * 1024 * 1024; // 50 MiB

const ALLOWED_MIME_TYPES: readonly string[] = [
  // Documents
  'application/pdf',
  'application/msword',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  'application/vnd.ms-excel',
  'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
  'text/plain',
  'text/csv',
  // Images
  'image/png',
  'image/jpeg',
  'image/gif',
  'image/webp',
];

const DOCUMENT_CATEGORIES = [
  'contract',
  'invoice',
  'receipt',
  'report',
  'minutes',
  'policy',
  'notice',
  'maintenance',
  'insurance',
  'legal',
  'financial',
  'correspondence',
  'other',
] as const;
type DocumentCategory = (typeof DOCUMENT_CATEGORIES)[number];

// ─── Types ────────────────────────────────────────────────────────────────────

interface PickedFile {
  uri: string;
  name: string;
  mimeType: string;
  size: number;
}

interface UploadFormErrors {
  title?: string;
  category?: string;
  file?: string;
}

interface DocumentUploadScreenProps {
  /** Called after a successful upload so navigation can return to the list. */
  onSuccess?: () => void;
  /** Called when the user dismisses without uploading. */
  onCancel?: () => void;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/**
 * Thrown by {@link uploadDocumentMultipart} when there is no readable access
 * token. The upload endpoint is auth-protected, so proceeding without a
 * bearer token is a guaranteed 401 — we fail fast *before* streaming the
 * multipart body over the network and let the caller surface a translated
 * "session expired / log in again" message (GitHub #2327).
 */
export class UploadAuthError extends Error {
  constructor() {
    super('AUTH_REQUIRED');
    this.name = 'UploadAuthError';
  }
}

/**
 * Upload the file as multipart/form-data.
 *
 * We cannot reuse the JSON-based `apiRequest` here because the backend
 * expects a real multipart upload (Content-Type: multipart/form-data with
 * a file part).  We roll our own fetch call so we can set Authorization and
 * X-Tenant-ID without accidentally overriding the boundary in the
 * Content-Type header (React Native's FormData sets that automatically when
 * you omit it).
 *
 * Throws {@link UploadAuthError} when no access token can be read — the
 * request is auth-protected, so an unauthenticated POST would only ever 401.
 */
export async function uploadDocumentMultipart(params: {
  file: PickedFile;
  title: string;
  description: string;
  category: string;
}): Promise<{ id: string; message: string }> {
  let token: string | null;
  try {
    token = await SecureStore.getItemAsync(ACCESS_TOKEN_KEY);
  } catch {
    token = null;
  }
  if (!token) {
    // Fail fast: no point streaming the file only to be rejected with a 401.
    throw new UploadAuthError();
  }
  const tenantId = extractTenantId(token);
  const baseUrl = getApiBaseUrl();

  const formData = new FormData();

  // React Native's FormData accepts an object with `uri`, `name`, `type`
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  formData.append('file', {
    uri: params.file.uri,
    name: params.file.name,
    type: params.file.mimeType,
  } as unknown as Blob);

  formData.append('title', params.title.trim());
  if (params.description.trim()) {
    formData.append('description', params.description.trim());
  }
  formData.append('category', params.category);

  const headers: Record<string, string> = {
    Authorization: `Bearer ${token}`,
    ...(tenantId ? { 'X-Tenant-ID': tenantId } : {}),
  };
  // NOTE: Do NOT set Content-Type — fetch + FormData sets the boundary automatically.

  const response = await fetch(`${baseUrl}/api/v1/documents/upload`, {
    method: 'POST',
    headers,
    body: formData,
  });

  if (!response.ok) {
    let message = `HTTP ${response.status}`;
    try {
      const body = await response.json();
      if (body?.message) message = body.message;
    } catch {
      // non-JSON body, keep status message
    }
    throw new Error(message);
  }

  return response.json() as Promise<{ id: string; message: string }>;
}

// ─── Component ────────────────────────────────────────────────────────────────

export function DocumentUploadScreen({ onSuccess, onCancel }: DocumentUploadScreenProps) {
  const { t } = useTranslation();

  const [pickedFile, setPickedFile] = useState<PickedFile | null>(null);
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [category, setCategory] = useState<DocumentCategory | null>(null);
  const [isUploading, setIsUploading] = useState(false);
  const [errors, setErrors] = useState<UploadFormErrors>({});

  // ── File picking ──────────────────────────────────────────────

  const pickDocument = useCallback(async () => {
    try {
      const result = await DocumentPicker.getDocumentAsync({
        type: '*/*',
        copyToCacheDirectory: true,
      });
      if (result.canceled) return;

      const asset = result.assets[0];
      setPickedFile({
        uri: asset.uri,
        name: asset.name,
        mimeType: asset.mimeType ?? 'application/octet-stream',
        size: asset.size ?? 0,
      });
      // Pre-populate title from filename (strip extension)
      const baseName = asset.name.replace(/\.[^/.]+$/, '');
      if (!title) setTitle(baseName);
      setErrors((prev) => ({ ...prev, file: undefined }));
    } catch {
      Alert.alert(t('common.error'), t('documents.upload.pickFailed'));
    }
  }, [title, t]);

  const pickPhoto = useCallback(async () => {
    try {
      const { status } = await ImagePicker.requestMediaLibraryPermissionsAsync();
      if (status !== 'granted') {
        Alert.alert(t('permissions.denied'), t('faults.photoLibraryPermissionDenied'));
        return;
      }
      const result = await ImagePicker.launchImageLibraryAsync({
        mediaTypes: 'images',
        quality: 0.9,
      });
      if (result.canceled) return;

      const asset = result.assets[0];
      const extension = asset.uri.split('.').pop() ?? 'jpg';
      const mimeType = extension === 'png' ? 'image/png' : 'image/jpeg';
      const fileName = `photo_${Date.now()}.${extension}`;
      setPickedFile({
        uri: asset.uri,
        name: fileName,
        mimeType,
        size: asset.fileSize ?? 0,
      });
      if (!title) setTitle(fileName.replace(/\.[^/.]+$/, ''));
      setErrors((prev) => ({ ...prev, file: undefined }));
    } catch {
      Alert.alert(t('common.error'), t('faults.failedToPickImage'));
    }
  }, [title, t]);

  // ── Validation ────────────────────────────────────────────────

  const validate = (): boolean => {
    const next: UploadFormErrors = {};
    if (!pickedFile) {
      next.file = t('documents.upload.fileRequired');
    } else if (!ALLOWED_MIME_TYPES.includes(pickedFile.mimeType)) {
      // Mirror the backend's ALLOWED_MIME_TYPES allow-list. A file the picker
      // couldn't type falls back to application/octet-stream, which is (also)
      // rejected here rather than after a full upload.
      next.file = t('documents.upload.fileTypeNotAllowed');
    } else if (pickedFile.size > MAX_FILE_SIZE) {
      // size === 0 means the picker didn't report a size; we can't enforce the
      // cap client-side in that case, so we let the server's 422 be the
      // backstop rather than blocking a legitimate file.
      next.file = t('documents.upload.fileTooLarge', {
        max: formatBytes(MAX_FILE_SIZE),
      });
    }
    if (!title.trim()) next.title = t('documents.upload.titleRequired');
    if (!category) next.category = t('documents.upload.categoryRequired');
    setErrors(next);
    return Object.keys(next).length === 0;
  };

  // ── Submit ────────────────────────────────────────────────────

  const handleSubmit = async () => {
    if (!validate() || !pickedFile || !category) return;
    setIsUploading(true);

    try {
      await uploadDocumentMultipart({
        file: pickedFile,
        title,
        description,
        category,
      });

      Alert.alert(t('documents.upload.successTitle'), t('documents.upload.successMessage'), [
        { text: t('common.ok'), onPress: onSuccess },
      ]);
    } catch (err) {
      const message =
        err instanceof UploadAuthError
          ? t('documents.upload.sessionExpired')
          : err instanceof Error
            ? err.message
            : t('errors.generic');
      Alert.alert(t('documents.upload.errorTitle'), message);
    } finally {
      setIsUploading(false);
    }
  };

  // ── Render ────────────────────────────────────────────────────

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
    >
      {/* Header */}
      <View style={styles.header}>
        <Pressable style={styles.cancelButton} onPress={onCancel} disabled={isUploading}>
          <Text style={styles.cancelText}>{t('common.cancel')}</Text>
        </Pressable>
        <Text style={styles.headerTitle}>{t('documents.upload.title')}</Text>
        <View style={styles.headerPlaceholder} />
      </View>

      <ScrollView style={styles.scrollView} keyboardShouldPersistTaps="handled">
        {/* ── File picker ── */}
        <View style={styles.formGroup}>
          <Text style={styles.label}>{t('documents.upload.fileLabel')} *</Text>

          {pickedFile ? (
            <View style={styles.filePreview}>
              <Text style={styles.fileIcon}>{getFileIcon(pickedFile.mimeType)}</Text>
              <View style={styles.fileInfo}>
                <Text style={styles.fileName} numberOfLines={2}>
                  {pickedFile.name}
                </Text>
                <Text style={styles.fileSize}>{formatBytes(pickedFile.size)}</Text>
              </View>
              <Pressable
                style={styles.clearFileButton}
                onPress={() => setPickedFile(null)}
                disabled={isUploading}
              >
                <Text style={styles.clearFileText}>✕</Text>
              </Pressable>
            </View>
          ) : (
            <View style={styles.filePickerRow}>
              <Pressable style={styles.filePickerButton} onPress={pickDocument}>
                <Text style={styles.filePickerIcon}>📎</Text>
                <Text style={styles.filePickerText}>{t('documents.upload.pickFile')}</Text>
              </Pressable>
              <Pressable style={styles.filePickerButton} onPress={pickPhoto}>
                <Text style={styles.filePickerIcon}>🖼️</Text>
                <Text style={styles.filePickerText}>{t('documents.upload.pickPhoto')}</Text>
              </Pressable>
            </View>
          )}

          {errors.file && <Text style={styles.errorText}>{errors.file}</Text>}
        </View>

        {/* ── Title ── */}
        <View style={styles.formGroup}>
          <Text style={styles.label}>{t('documents.upload.titleLabel')} *</Text>
          <TextInput
            style={[styles.input, errors.title ? styles.inputError : undefined]}
            placeholder={t('documents.upload.titlePlaceholder')}
            value={title}
            onChangeText={setTitle}
            maxLength={200}
            editable={!isUploading}
          />
          {errors.title && <Text style={styles.errorText}>{errors.title}</Text>}
        </View>

        {/* ── Category ── */}
        <View style={styles.formGroup}>
          <Text style={styles.label}>{t('documents.upload.categoryLabel')} *</Text>
          <View style={styles.categoryGrid}>
            {DOCUMENT_CATEGORIES.map((cat) => (
              <Pressable
                key={cat}
                style={[styles.categoryChip, category === cat && styles.categoryChipSelected]}
                onPress={() => {
                  setCategory(cat);
                  setErrors((prev) => ({ ...prev, category: undefined }));
                }}
                disabled={isUploading}
              >
                <Text
                  style={[
                    styles.categoryChipText,
                    category === cat && styles.categoryChipTextSelected,
                  ]}
                >
                  {t(`documents.upload.categories.${cat}`)}
                </Text>
              </Pressable>
            ))}
          </View>
          {errors.category && <Text style={styles.errorText}>{errors.category}</Text>}
        </View>

        {/* ── Description (optional) ── */}
        <View style={styles.formGroup}>
          <Text style={styles.label}>{t('documents.upload.descriptionLabel')}</Text>
          <TextInput
            style={[styles.input, styles.textArea]}
            placeholder={t('documents.upload.descriptionPlaceholder')}
            value={description}
            onChangeText={setDescription}
            multiline
            numberOfLines={3}
            textAlignVertical="top"
            maxLength={1000}
            editable={!isUploading}
          />
        </View>

        {/* ── Upload progress ── */}
        {isUploading && (
          <View style={styles.progressContainer}>
            <Text style={styles.progressLabel}>{t('documents.upload.uploading')}</Text>
          </View>
        )}

        {/* ── Submit ── */}
        <Pressable
          style={[styles.submitButton, isUploading && styles.submitButtonDisabled]}
          onPress={handleSubmit}
          disabled={isUploading}
        >
          {isUploading ? (
            <ActivityIndicator color={colors.white} />
          ) : (
            <Text style={styles.submitButtonText}>{t('documents.upload.submitButton')}</Text>
          )}
        </Pressable>

        <View style={styles.bottomSpacer} />
      </ScrollView>
    </KeyboardAvoidingView>
  );
}

// ─── Helper: file icon by MIME ─────────────────────────────────────────────────

export function getFileIcon(mimeType: string): string {
  if (mimeType.includes('pdf')) return '📄';
  if (mimeType.startsWith('image/')) return '🖼️';
  if (mimeType.includes('spreadsheet') || mimeType.includes('excel')) return '📊';
  if (mimeType.includes('word') || mimeType.includes('document')) return '📝';
  return '📎';
}

// ─── Styles ───────────────────────────────────────────────────────────────────

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: 20,
    paddingTop: 60,
    backgroundColor: colors.surface,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  cancelButton: {
    padding: 4,
  },
  cancelText: {
    color: colors.textMuted,
    fontSize: 16,
  },
  headerTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: colors.text,
  },
  headerPlaceholder: {
    width: 50,
  },
  scrollView: {
    flex: 1,
    padding: 16,
  },
  formGroup: {
    marginBottom: 20,
  },
  label: {
    fontSize: 14,
    fontWeight: '600',
    color: colors.textSecondary,
    marginBottom: 8,
  },
  input: {
    backgroundColor: colors.surface,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    padding: 12,
    fontSize: 16,
    color: colors.text,
  },
  inputError: {
    borderColor: colors.danger,
  },
  textArea: {
    minHeight: 88,
  },
  errorText: {
    color: colors.danger,
    fontSize: 12,
    marginTop: 4,
  },
  // File picker
  filePickerRow: {
    flexDirection: 'row',
    gap: 12,
  },
  filePickerButton: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 20,
    borderRadius: 8,
    borderWidth: 1,
    borderStyle: 'dashed',
    borderColor: colors.borderStrong,
    backgroundColor: colors.surface,
    gap: 6,
  },
  filePickerIcon: {
    fontSize: 28,
  },
  filePickerText: {
    fontSize: 13,
    color: colors.textMuted,
    fontWeight: '500',
  },
  // File preview
  filePreview: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.accentSoft,
    borderRadius: 8,
    padding: 12,
    gap: 10,
  },
  fileIcon: {
    fontSize: 28,
  },
  fileInfo: {
    flex: 1,
  },
  fileName: {
    fontSize: 14,
    fontWeight: '500',
    color: colors.text,
    marginBottom: 2,
  },
  fileSize: {
    fontSize: 12,
    color: colors.textMuted,
  },
  clearFileButton: {
    padding: 4,
  },
  clearFileText: {
    fontSize: 16,
    color: colors.textMuted,
  },
  // Category chips
  categoryGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
  },
  categoryChip: {
    paddingHorizontal: 12,
    paddingVertical: 8,
    borderRadius: 20,
    backgroundColor: colors.surface,
    borderWidth: 1,
    borderColor: colors.borderStrong,
  },
  categoryChipSelected: {
    backgroundColor: colors.accentSoft,
    borderColor: colors.accent,
  },
  categoryChipText: {
    fontSize: 13,
    color: colors.textSecondary,
  },
  categoryChipTextSelected: {
    color: colors.accent,
    fontWeight: '500',
  },
  // Progress
  progressContainer: {
    marginBottom: 16,
  },
  progressLabel: {
    fontSize: 13,
    color: colors.textMuted,
    marginBottom: 6,
  },
  // Submit
  submitButton: {
    backgroundColor: colors.accent,
    borderRadius: 8,
    padding: 16,
    alignItems: 'center',
    marginTop: 8,
  },
  submitButtonDisabled: {
    backgroundColor: colors.accentDisabled,
  },
  submitButtonText: {
    color: colors.white,
    fontSize: 16,
    fontWeight: '600',
  },
  bottomSpacer: {
    height: 40,
  },
});
