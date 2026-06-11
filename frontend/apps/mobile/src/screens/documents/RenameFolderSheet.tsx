/**
 * RenameFolderSheet (feat-folder-organization-mobile-slice)
 *
 * A modal bottom-sheet that lets a manager rename an existing folder (and
 * optionally edit its description). Calls PATCH /api/v1/documents/folders/{id}
 * and invalidates the folder-tree query on success — mirroring the web
 * folder-edit flow (`updateFolder` in @ppt/api-client) but adapted for the
 * touch-first RN bottom-sheet pattern already used by NewFolderSheet.
 *
 * Usage:
 *   <RenameFolderSheet
 *     visible={renameTarget !== null}
 *     folderId={renameTarget.id}
 *     currentName={renameTarget.name}
 *     onClose={() => setRenameTarget(null)}
 *     onRenamed={() => refetchFolders()}   // tree refresh
 *   />
 */

import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
  Alert,
  KeyboardAvoidingView,
  Modal,
  Platform,
  Pressable,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { useApiMutation } from '../../hooks/useApi';
import { colors } from '../shared/screenStyles';

// ─── API types ────────────────────────────────────────────────────────────────

interface UpdateFolderRequest {
  name?: string;
  description?: string;
}

interface UpdateFolderResponse {
  message: string;
}

// ─── Pure helpers (exported for unit-testing) ─────────────────────────────────

export type FolderNameValidation = { ok: true } | { ok: false; reason: 'empty' | 'tooLong' };

/**
 * Validate a candidate folder name against the same 1–255 char rule the
 * backend enforces. Pure — covered directly by unit tests.
 */
export function validateFolderName(name: string): FolderNameValidation {
  const trimmed = name.trim();
  if (!trimmed) return { ok: false, reason: 'empty' };
  if (trimmed.length > 255) return { ok: false, reason: 'tooLong' };
  return { ok: true };
}

/**
 * Build the minimal PATCH body for a rename. Only includes `name` when it
 * actually changed from the original, and only includes `description` when a
 * non-empty value is provided — so a no-op rename sends an empty patch and an
 * untouched description is never clobbered. Pure — covered by unit tests.
 */
export function buildRenamePayload(nextName: string, originalName: string): UpdateFolderRequest {
  const trimmed = nextName.trim();
  const payload: UpdateFolderRequest = {};
  if (trimmed && trimmed !== originalName) payload.name = trimmed;
  return payload;
}

// ─── Props ─────────────────────────────────────────────────────────────────────────

export interface RenameFolderSheetProps {
  visible: boolean;
  /** Id of the folder being renamed. */
  folderId: string;
  /** Current folder name (pre-fills the input). */
  currentName: string;
  onClose: () => void;
  /** Called after the folder is successfully renamed. */
  onRenamed: () => void;
}

// ─── Component ────────────────────────────────────────────────────────────────────

export function RenameFolderSheet({
  visible,
  folderId,
  currentName,
  onClose,
  onRenamed,
}: RenameFolderSheetProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [name, setName] = useState(currentName);
  const [nameError, setNameError] = useState<string | null>(null);

  // Keep the input in sync when the sheet is re-targeted at a different folder.
  useEffect(() => {
    if (visible) {
      setName(currentName);
      setNameError(null);
    }
  }, [visible, currentName]);

  const renameMutation = useApiMutation<UpdateFolderResponse, UpdateFolderRequest>(
    `/api/v1/documents/folders/${folderId}`,
    'PATCH'
  );

  const handleClose = () => {
    if (!renameMutation.isPending) {
      setNameError(null);
      onClose();
    }
  };

  const handleRename = async () => {
    const validation = validateFolderName(name);
    if (!validation.ok) {
      setNameError(
        validation.reason === 'empty'
          ? t('documents.folder.nameRequired')
          : t('documents.folder.nameTooLong')
      );
      return;
    }
    setNameError(null);

    const payload = buildRenamePayload(name, currentName);

    // No-op rename — nothing changed, just dismiss.
    if (payload.name === undefined) {
      onClose();
      return;
    }

    try {
      await renameMutation.mutateAsync(payload);

      // Invalidate the folder tree so DocumentsScreen refetches the new name.
      await queryClient.invalidateQueries({ queryKey: ['documents', 'folders', 'tree'] });

      onRenamed();
      onClose();
    } catch (err) {
      const message = err instanceof Error ? err.message : t('errors.generic');
      Alert.alert(t('documents.folder.renameError'), message);
    }
  };

  return (
    <Modal visible={visible} animationType="slide" transparent onRequestClose={handleClose}>
      <KeyboardAvoidingView
        style={styles.outerWrap}
        behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
      >
        <Pressable style={styles.backdrop} onPress={handleClose} />

        <View style={styles.sheet}>
          {/* Handle */}
          <View style={styles.handle} />

          {/* Header */}
          <View style={styles.header}>
            <View style={styles.headerText}>
              <Text style={styles.title}>{t('documents.folder.renameTitle')}</Text>
              <Text style={styles.subtitle} numberOfLines={1}>
                {t('documents.folder.insideFolder', { name: currentName })}
              </Text>
            </View>
            <Pressable
              style={styles.closeBtn}
              onPress={handleClose}
              hitSlop={8}
              disabled={renameMutation.isPending}
            >
              <Text style={styles.closeBtnText}>✕</Text>
            </Pressable>
          </View>

          {/* Form */}
          <View style={styles.form}>
            <Text style={styles.label}>{t('documents.folder.nameLabel')}</Text>
            <TextInput
              style={[styles.input, nameError ? styles.inputError : null]}
              placeholder={t('documents.folder.namePlaceholder')}
              value={name}
              onChangeText={(v) => {
                setName(v);
                if (nameError) setNameError(null);
              }}
              maxLength={255}
              autoFocus
              returnKeyType="done"
              onSubmitEditing={handleRename}
              editable={!renameMutation.isPending}
            />
            {nameError ? <Text style={styles.errorText}>{nameError}</Text> : null}

            <Pressable
              style={[styles.saveBtn, renameMutation.isPending && styles.saveBtnDisabled]}
              onPress={handleRename}
              disabled={renameMutation.isPending}
            >
              {renameMutation.isPending ? (
                <ActivityIndicator size="small" color={colors.white} />
              ) : (
                <Text style={styles.saveBtnText}>{t('documents.folder.renameButton')}</Text>
              )}
            </Pressable>
          </View>
        </View>
      </KeyboardAvoidingView>
    </Modal>
  );
}

// ─── Styles ─────────────────────────────────────────────────────────────────────────

const styles = StyleSheet.create({
  outerWrap: {
    flex: 1,
    justifyContent: 'flex-end',
  },
  backdrop: {
    ...StyleSheet.absoluteFill,
    backgroundColor: colors.bgOverlay,
  },
  sheet: {
    backgroundColor: colors.surface,
    borderTopLeftRadius: 20,
    borderTopRightRadius: 20,
    paddingBottom: Platform.OS === 'ios' ? 36 : 24,
  },
  handle: {
    alignSelf: 'center',
    width: 40,
    height: 4,
    borderRadius: 2,
    backgroundColor: colors.border,
    marginTop: 12,
    marginBottom: 4,
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 20,
    paddingVertical: 14,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  headerText: {
    flex: 1,
  },
  title: {
    fontSize: 17,
    fontWeight: '600',
    color: colors.text,
  },
  subtitle: {
    fontSize: 12,
    color: colors.textMuted,
    marginTop: 2,
  },
  closeBtn: {
    width: 28,
    height: 28,
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: 14,
    backgroundColor: colors.surfaceMuted,
  },
  closeBtnText: {
    fontSize: 14,
    color: colors.textMuted,
  },
  form: {
    padding: 20,
  },
  label: {
    fontSize: 13,
    fontWeight: '600',
    color: colors.textSecondary,
    marginBottom: 6,
  },
  input: {
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
    paddingHorizontal: 12,
    paddingVertical: 10,
    fontSize: 15,
    color: colors.text,
    borderWidth: 1,
    borderColor: colors.border,
  },
  inputError: {
    borderColor: colors.danger,
  },
  errorText: {
    fontSize: 12,
    color: colors.danger,
    marginTop: 4,
  },
  saveBtn: {
    marginTop: 24,
    backgroundColor: colors.accent,
    borderRadius: 10,
    paddingVertical: 14,
    alignItems: 'center',
    justifyContent: 'center',
  },
  saveBtnDisabled: {
    backgroundColor: colors.accentDisabled,
  },
  saveBtnText: {
    fontSize: 15,
    fontWeight: '600',
    color: colors.white,
  },
});
