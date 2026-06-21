/**
 * RenameFolderSheet (feat-folder-organization-document-mobile)
 *
 * A modal bottom-sheet that lets a manager rename an existing folder.
 * Calls PUT /api/v1/documents/folders/{id} with the updated name (and
 * optionally description), then invalidates the folder-tree query.
 *
 * Usage:
 *   <RenameFolderSheet
 *     visible={showRename}
 *     folderId={folder.id}
 *     currentName={folder.name}
 *     onClose={() => setShowRename(false)}
 *     onRenamed={() => refetchFolders()}
 *   />
 */

import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
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
import { FOLDER_TREE_QUERY_KEY } from '../../hooks/useFolderTree';
import { colors } from '../shared/screenStyles';

// ─── API types ────────────────────────────────────────────────────────────────

interface UpdateFolderRequest {
  name?: string;
  description?: string;
}

// UpdateFolderResponse is not used (we only care about cache invalidation),
// but the mutateAsync call needs a type so it compiles cleanly.
interface UpdateFolderResponse {
  message: string;
}

// ─── Props ──────────────────────────────────────────────────────────────────────

export interface RenameFolderSheetProps {
  visible: boolean;
  folderId: string;
  /** Current folder name – pre-populates the name field. */
  currentName: string;
  onClose: () => void;
  /** Called after the folder is successfully renamed. */
  onRenamed: () => void;
}

// ─── Component ──────────────────────────────────────────────────────────────────

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

  const renameMutation = useApiMutation<UpdateFolderResponse, UpdateFolderRequest>(
    `/api/v1/documents/folders/${folderId}`,
    'PUT'
  );

  const reset = () => {
    setName(currentName);
    setNameError(null);
  };

  const handleClose = () => {
    if (!renameMutation.isPending) {
      reset();
      onClose();
    }
  };

  const handleRename = async () => {
    const trimmedName = name.trim();

    // Client-side validation — mirrors backend (1–255 chars)
    if (!trimmedName) {
      setNameError(t('documents.folder.nameRequired'));
      return;
    }
    if (trimmedName.length > 255) {
      setNameError(t('documents.folder.nameTooLong'));
      return;
    }
    // No-op if nothing changed
    if (trimmedName === currentName) {
      onClose();
      return;
    }
    setNameError(null);

    try {
      await renameMutation.mutateAsync({ name: trimmedName });

      // Invalidate the folder tree so DocumentsScreen refetches
      await queryClient.invalidateQueries({ queryKey: [...FOLDER_TREE_QUERY_KEY] });

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
                {currentName}
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

            {/* Submit */}
            <Pressable
              style={[styles.renameBtn, renameMutation.isPending && styles.renameBtnDisabled]}
              onPress={handleRename}
              disabled={renameMutation.isPending}
            >
              {renameMutation.isPending ? (
                <ActivityIndicator size="small" color={colors.white} />
              ) : (
                <Text style={styles.renameBtnText}>{t('documents.folder.renameButton')}</Text>
              )}
            </Pressable>
          </View>
        </View>
      </KeyboardAvoidingView>
    </Modal>
  );
}

// ─── Styles ────────────────────────────────────────────────────────────────────

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
  renameBtn: {
    marginTop: 24,
    backgroundColor: colors.accent,
    borderRadius: 10,
    paddingVertical: 14,
    alignItems: 'center',
    justifyContent: 'center',
  },
  renameBtnDisabled: {
    backgroundColor: colors.accentDisabled,
  },
  renameBtnText: {
    fontSize: 15,
    fontWeight: '600',
    color: colors.white,
  },
});
