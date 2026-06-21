/**
 * MoveDocumentSheet (gap-7a-2-mobile-folder-manage-ui)
 *
 * A modal bottom-sheet that lets a user move a document into a folder (or back
 * to root). It renders a flat, scrollable list of folders from the live folder
 * tree — mirroring the web MoveFolderDialog but adapted for touch-first RN
 * interaction (no nested expand/collapse, just a flat list ordered depth-first).
 *
 * API: POST /api/v1/documents/{documentId}/move  { folder_id: string | null }
 *
 * Usage:
 *   <MoveDocumentSheet
 *     visible={showMove}
 *     documentId={doc.id}
 *     documentTitle={doc.name}
 *     currentFolderId={doc.parentId}
 *     folderTree={folderTree}          // ApiFolderTreeNode[] from DocumentsScreen
 *     onClose={() => setShowMove(false)}
 *     onMoved={() => Promise.all([refetch(), refetchFolders()])}
 *   />
 */

import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
  Alert,
  Modal,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { useApiMutation } from '../../hooks/useApi';
import { type ApiFolderTreeNode, FOLDER_TREE_QUERY_KEY } from '../../hooks/useFolderTree';
import { colors } from '../shared/screenStyles';
import { type FlatFolder, flattenTree } from './folderTree';

// ─── API types ────────────────────────────────────────────────────────────────

interface MoveDocumentRequest {
  folder_id: string | null;
}

// `FlatFolder` + `flattenTree` now live in the shared `folderTree` util (single
// source of truth, GH #1589). Re-exported here so existing `./MoveDocumentSheet`
// importers (incl. DocumentFolderOrganization.test.ts) are unaffected.
export { type FlatFolder, flattenTree };

// ─── Props ─────────────────────────────────────────────────────────────────────────

export interface MoveDocumentSheetProps {
  visible: boolean;
  documentId: string;
  documentTitle: string;
  /** Current folder id (null = document is at root). Highlighted in the list. */
  currentFolderId: string | null;
  /** Live folder tree passed from DocumentsScreen (avoids double-fetch). */
  folderTree: ApiFolderTreeNode[];
  onClose: () => void;
  /** Called after the document is successfully moved. */
  onMoved: () => void;
}

// ─── Component ────────────────────────────────────────────────────────────────────

export function MoveDocumentSheet({
  visible,
  documentId,
  documentTitle,
  currentFolderId,
  folderTree,
  onClose,
  onMoved,
}: MoveDocumentSheetProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [selectedId, setSelectedId] = useState<string | null>(currentFolderId);

  // The move endpoint returns the full updated Document, but the result is
  // never read here (we just invalidate the affected queries on success), so
  // the response is typed as `void` rather than a hand-rolled shape.
  const moveMutation = useApiMutation<void, MoveDocumentRequest>(
    `/api/v1/documents/${documentId}/move`,
    'POST'
  );

  const flatFolders = flattenTree(folderTree);
  const hasChanged = selectedId !== currentFolderId;

  const handleClose = () => {
    if (!moveMutation.isPending) {
      setSelectedId(currentFolderId);
      onClose();
    }
  };

  const handleMove = async () => {
    if (!hasChanged) return;

    try {
      await moveMutation.mutateAsync({ folder_id: selectedId });

      // Invalidate both the document list and the folder tree so counts update
      await queryClient.invalidateQueries({ queryKey: ['documents', 'list'] });
      await queryClient.invalidateQueries({ queryKey: FOLDER_TREE_QUERY_KEY });

      setSelectedId(currentFolderId);
      onMoved();
      onClose();
    } catch (err) {
      const message = err instanceof Error ? err.message : t('errors.generic');
      Alert.alert(t('documents.move.errorTitle'), message);
    }
  };

  // Derive selected folder name for the destination preview
  const selectedFolder = flatFolders.find((f) => f.id === selectedId);
  const destinationLabel = selectedFolder ? selectedFolder.name : t('documents.move.rootLabel');

  return (
    <Modal visible={visible} animationType="slide" transparent onRequestClose={handleClose}>
      <View style={styles.outerWrap}>
        <Pressable style={styles.backdrop} onPress={handleClose} />

        <View style={styles.sheet}>
          {/* Handle */}
          <View style={styles.handle} />

          {/* Header */}
          <View style={styles.header}>
            <View style={styles.headerText}>
              <Text style={styles.title}>{t('documents.move.title')}</Text>
              <Text style={styles.subtitle} numberOfLines={1}>
                {documentTitle}
              </Text>
            </View>
            <Pressable
              style={styles.closeBtn}
              onPress={handleClose}
              hitSlop={8}
              disabled={moveMutation.isPending}
            >
              <Text style={styles.closeBtnText}>✕</Text>
            </Pressable>
          </View>

          {/* Destination preview strip */}
          <View style={styles.destStrip}>
            <Text style={styles.destLabel}>{t('documents.move.destination')}</Text>
            <Text style={styles.destValue} numberOfLines={1}>
              {destinationLabel}
            </Text>
          </View>

          {/* Folder list */}
          <ScrollView style={styles.list} showsVerticalScrollIndicator={false}>
            {/* Root / "no folder" option */}
            <Pressable
              style={[styles.folderRow, selectedId === null && styles.folderRowSelected]}
              onPress={() => setSelectedId(null)}
            >
              <Text style={styles.folderIcon}>🏠</Text>
              <Text
                style={[styles.folderName, selectedId === null && styles.folderNameSelected]}
                numberOfLines={1}
              >
                {t('documents.move.rootLabel')}
              </Text>
              {selectedId === null && <Text style={styles.checkmark}>✓</Text>}
            </Pressable>

            {flatFolders.length === 0 ? (
              <View style={styles.emptyState}>
                <Text style={styles.emptyText}>{t('documents.move.noFolders')}</Text>
              </View>
            ) : (
              flatFolders.map((folder) => {
                const isSelected = selectedId === folder.id;
                return (
                  <Pressable
                    key={folder.id}
                    style={[styles.folderRow, isSelected && styles.folderRowSelected]}
                    onPress={() => setSelectedId(folder.id)}
                  >
                    {/* Indent by depth */}
                    {folder.depth > 0 && <View style={{ width: folder.depth * 16 }} />}
                    <Text style={styles.folderIcon}>📁</Text>
                    <Text
                      style={[styles.folderName, isSelected && styles.folderNameSelected]}
                      numberOfLines={1}
                    >
                      {folder.name}
                    </Text>
                    {isSelected && <Text style={styles.checkmark}>✓</Text>}
                  </Pressable>
                );
              })
            )}

            <View style={styles.listSpacer} />
          </ScrollView>

          {/* Footer */}
          <View style={styles.footer}>
            <Pressable
              style={styles.cancelBtn}
              onPress={handleClose}
              disabled={moveMutation.isPending}
            >
              <Text style={styles.cancelBtnText}>{t('common.cancel')}</Text>
            </Pressable>

            <Pressable
              style={[
                styles.moveBtn,
                (!hasChanged || moveMutation.isPending) && styles.moveBtnDisabled,
              ]}
              onPress={handleMove}
              disabled={!hasChanged || moveMutation.isPending}
            >
              {moveMutation.isPending ? (
                <ActivityIndicator size="small" color={colors.white} />
              ) : (
                <Text style={styles.moveBtnText}>{t('documents.move.confirm')}</Text>
              )}
            </Pressable>
          </View>
        </View>
      </View>
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
    maxHeight: '80%',
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
  destStrip: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 20,
    paddingVertical: 10,
    backgroundColor: colors.surfaceMuted,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
    gap: 8,
  },
  destLabel: {
    fontSize: 11,
    fontWeight: '700',
    color: colors.textSubtle,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  destValue: {
    flex: 1,
    fontSize: 13,
    fontWeight: '500',
    color: colors.accent,
  },
  list: {
    flexGrow: 0,
  },
  folderRow: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: 12,
    paddingHorizontal: 20,
    borderBottomWidth: 1,
    borderBottomColor: colors.surfaceMuted,
    gap: 10,
  },
  folderRowSelected: {
    backgroundColor: colors.accentSoft,
  },
  folderIcon: {
    fontSize: 18,
  },
  folderName: {
    flex: 1,
    fontSize: 14,
    color: colors.text,
  },
  folderNameSelected: {
    fontWeight: '600',
    color: colors.accent,
  },
  checkmark: {
    fontSize: 16,
    color: colors.accent,
    fontWeight: '700',
  },
  emptyState: {
    alignItems: 'center',
    paddingVertical: 32,
  },
  emptyText: {
    fontSize: 14,
    color: colors.textMuted,
  },
  listSpacer: {
    height: 8,
  },
  footer: {
    flexDirection: 'row',
    gap: 12,
    padding: 16,
    borderTopWidth: 1,
    borderTopColor: colors.border,
  },
  cancelBtn: {
    flex: 1,
    paddingVertical: 12,
    borderRadius: 10,
    borderWidth: 1,
    borderColor: colors.border,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.surface,
  },
  cancelBtnText: {
    fontSize: 15,
    fontWeight: '500',
    color: colors.textMuted,
  },
  moveBtn: {
    flex: 2,
    paddingVertical: 12,
    borderRadius: 10,
    alignItems: 'center',
    justifyContent: 'center',
    backgroundColor: colors.accent,
  },
  moveBtnDisabled: {
    backgroundColor: colors.accentDisabled,
  },
  moveBtnText: {
    fontSize: 15,
    fontWeight: '600',
    color: colors.white,
  },
});
