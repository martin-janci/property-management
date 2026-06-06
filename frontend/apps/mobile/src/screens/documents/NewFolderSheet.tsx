/**
 * NewFolderSheet (gap-7a-2-mobile-folder-manage-ui)
 *
 * A modal bottom-sheet that lets a manager create a new folder under the
 * current parent. Calls POST /api/v1/documents/folders and invalidates the
 * folder-tree query on success.
 *
 * Usage:
 *   <NewFolderSheet
 *     visible={showNewFolder}
 *     parentId={currentFolderId}           // null = create at root
 *     parentName={currentFolderName}       // shown in subtitle
 *     onClose={() => setShowNewFolder(false)}
 *     onCreated={() => refetchFolders()}   // tree refresh
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
import { colors } from '../shared/screenStyles';

// ─── API types ────────────────────────────────────────────────────────────────

interface CreateFolderRequest {
  name: string;
  description?: string;
  parent_id?: string | null;
}

interface CreateFolderResponse {
  id: string;
  message: string;
}

// ─── Props ─────────────────────────────────────────────────────────────────────────

export interface NewFolderSheetProps {
  visible: boolean;
  /** Parent folder id. Pass null to create at root level. */
  parentId: string | null;
  /** Human-readable parent folder name (shown in subtitle). Pass null for root. */
  parentName: string | null;
  onClose: () => void;
  /** Called after the folder is successfully created. */
  onCreated: () => void;
}

// ─── Component ────────────────────────────────────────────────────────────────────

export function NewFolderSheet({
  visible,
  parentId,
  parentName,
  onClose,
  onCreated,
}: NewFolderSheetProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [nameError, setNameError] = useState<string | null>(null);

  const createMutation = useApiMutation<CreateFolderResponse, CreateFolderRequest>(
    '/api/v1/documents/folders',
    'POST'
  );

  const reset = () => {
    setName('');
    setDescription('');
    setNameError(null);
  };

  const handleClose = () => {
    if (!createMutation.isPending) {
      reset();
      onClose();
    }
  };

  const handleCreate = async () => {
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
    setNameError(null);

    try {
      await createMutation.mutateAsync({
        name: trimmedName,
        description: description.trim() || undefined,
        parent_id: parentId,
      });

      // Invalidate the folder tree so DocumentsScreen refetches
      await queryClient.invalidateQueries({ queryKey: ['documents', 'folders', 'tree'] });

      reset();
      onCreated();
      onClose();
    } catch (err) {
      const message = err instanceof Error ? err.message : t('errors.generic');
      // Surface server errors (e.g. MAX_DEPTH_EXCEEDED, FORBIDDEN) via Alert
      Alert.alert(t('documents.folder.createError'), message);
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
              <Text style={styles.title}>{t('documents.folder.newTitle')}</Text>
              <Text style={styles.subtitle} numberOfLines={1}>
                {parentName
                  ? t('documents.folder.insideFolder', { name: parentName })
                  : t('documents.folder.atRoot')}
              </Text>
            </View>
            <Pressable
              style={styles.closeBtn}
              onPress={handleClose}
              hitSlop={8}
              disabled={createMutation.isPending}
            >
              <Text style={styles.closeBtnText}>✕</Text>
            </Pressable>
          </View>

          {/* Form */}
          <View style={styles.form}>
            {/* Name field */}
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
              returnKeyType="next"
              editable={!createMutation.isPending}
            />
            {nameError ? <Text style={styles.errorText}>{nameError}</Text> : null}

            {/* Description field (optional) */}
            <Text style={[styles.label, styles.labelSpaced]}>
              {t('documents.folder.descriptionLabel')}
            </Text>
            <TextInput
              style={[styles.input, styles.inputMultiline]}
              placeholder={t('documents.folder.descriptionPlaceholder')}
              value={description}
              onChangeText={setDescription}
              multiline
              numberOfLines={3}
              maxLength={500}
              editable={!createMutation.isPending}
            />

            {/* Submit */}
            <Pressable
              style={[styles.createBtn, createMutation.isPending && styles.createBtnDisabled]}
              onPress={handleCreate}
              disabled={createMutation.isPending}
            >
              {createMutation.isPending ? (
                <ActivityIndicator size="small" color={colors.white} />
              ) : (
                <Text style={styles.createBtnText}>{t('documents.folder.createButton')}</Text>
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
  labelSpaced: {
    marginTop: 16,
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
  inputMultiline: {
    height: 80,
    textAlignVertical: 'top',
  },
  errorText: {
    fontSize: 12,
    color: colors.danger,
    marginTop: 4,
  },
  createBtn: {
    marginTop: 24,
    backgroundColor: colors.accent,
    borderRadius: 10,
    paddingVertical: 14,
    alignItems: 'center',
    justifyContent: 'center',
  },
  createBtnDisabled: {
    backgroundColor: colors.accentDisabled,
  },
  createBtnText: {
    fontSize: 15,
    fontWeight: '600',
    color: colors.white,
  },
});
