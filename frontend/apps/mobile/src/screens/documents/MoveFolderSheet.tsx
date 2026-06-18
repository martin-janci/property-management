/**
 * MoveFolderSheet (feat-folder-organization-document-mobile)
 *
 * A modal bottom-sheet that lets a manager move a folder to a new parent (or
 * back to root). It renders a flat, scrollable list of all OTHER folders from
 * the live tree — the folder being moved and all its descendants are excluded
 * to prevent circular references.
 *
 * API: PUT /api/v1/documents/folders/{folderId}  { parent_id: string | null }
 *
 * Usage:
 *   <MoveFolderSheet
 *     visible={showMoveFolder}
 *     folderId={folder.id}
 *     folderName={folder.name}
 *     currentParentId={folder.parentId}
 *     folderTree={folderTree}
 *     onClose={() => setShowMoveFolder(false)}
 *     onMoved={() => refetchFolders()}
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
import { FOLDER_TREE_QUERY_KEY } from '../../hooks/useFolderTree';
import { colors } from '../shared/screenStyles';
import type { ApiFolderTreeNode } from './DocumentsScreen';
import type { FlatFolder } from './MoveDocumentSheet';

// ─── API types ────────────────────────────────────────────────────────────────

interface UpdateFolderRequest {
  parent_id: string | null;
}

interface UpdateFolderResponse {
  message: string;
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Collect all descendant folder ids of `targetId` within the tree (inclusive
 * of `targetId` itself). Used to exclude the subtree being moved from the
 * destination picker — prevents circular references on the client side before
 * the server validates them.
 *
 * Exported for unit-testing.
 */
export function collectDescendantIds(nodes: ApiFolderTreeNode[], targetId: string): Set<string> {
  const ids = new Set<string>();

  function walk(current: ApiFolderTreeNode) {
    ids.add(current.id);
    for (const child of current.children ?? []) {
      walk(child);
    }
  }

  function findAndWalk(nodes: ApiFolderTreeNode[]): boolean {
    for (const node of nodes) {
      if (node.id === targetId) {
        walk(node);
        return true;
      }
      if (findAndWalk(node.children ?? [])) return true;
    }
    return false;
  }

  findAndWalk(nodes);
  return ids;
}

/**
 * Flatten the full tree into a depth-first list, excluding the subtree rooted
 * at `excludeId` (which is the folder being moved — it cannot be its own
 * parent or a parent of its own ancestor).
 *
 * Exported for unit-testing.
 */
export function flattenTreeExcluding(
  nodes: ApiFolderTreeNode[],
  excludeIds: Set<string>,
  depth = 0
): FlatFolder[] {
  const result: FlatFolder[] = [];
  for (const node of nodes) {
    if (excludeIds.has(node.id)) continue;
    result.push({ id: node.id, name: node.name, depth });
    result.push(...flattenTreeExcluding(node.children ?? [], excludeIds, depth + 1));
  }
  return result;
}

// ─── Props ──────────────────────────────────────────────────────────────────────

export interface MoveFolderSheetProps {
  visible: boolean;
  folderId: string;
  folderName: string;
  /** Current parent folder id (null = folder is at root). */
  currentParentId: string | null;
  /** Live folder tree passed from DocumentsScreen (avoids double-fetch). */
  folderTree: ApiFolderTreeNode[];
  onClose: () => void;
  /** Called after the folder is successfully moved. */
  onMoved: () => void;
}

// ─── Component ──────────────────────────────────────────────────────────────────

export function MoveFolderSheet({
  visible,
  folderId,
  folderName,
  currentParentId,
  folderTree,
  onClose,
  onMoved,
}: MoveFolderSheetProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [selectedParentId, setSelectedParentId] = useState<string | null>(currentParentId);

  const moveMutation = useApiMutation<UpdateFolderResponse, UpdateFolderRequest>(
    `/api/v1/documents/folders/${folderId}`,
    'PUT'
  );

  // Build the set of ids to exclude from the destination list (the folder
  // itself and all its descendants — they cannot become parents of themselves).
  const excludedIds = collectDescendantIds(folderTree, folderId);
  const availableFolders = flattenTreeExcluding(folderTree, excludedIds);

  const hasChanged = selectedParentId !== currentParentId;

  const handleClose = () => {
    if (!moveMutation.isPending) {
      setSelectedParentId(currentParentId);
      onClose();
    }
  };

  const handleMove = async () => {
    if (!hasChanged) return;

    try {
      await moveMutation.mutateAsync({ parent_id: selectedParentId });

      // Invalidate the folder tree so counts and hierarchy update everywhere
      await queryClient.invalidateQueries({ queryKey: [...FOLDER_TREE_QUERY_KEY] });

      setSelectedParentId(currentParentId);
      onMoved();
      onClose();
    } catch (err) {
      const message = err instanceof Error ? err.message : t('errors.generic');
      Alert.alert(t('documents.folderMove.errorTitle'), message);
    }
  };

  // Derive destination label for the preview strip
  const selectedFolder = availableFolders.find((f) => f.id === selectedParentId);
  const destinationLabel = selectedFolder
    ? selectedFolder.name
    : t('documents.folderMove.rootLabel');

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
              <Text style={styles.title}>{t('documents.folderMove.title')}</Text>
              <Text style={styles.subtitle} numberOfLines={1}>
                {folderName}
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
            <Text style={styles.destLabel}>{t('documents.folderMove.destination')}</Text>
            <Text style={styles.destValue} numberOfLines={1}>
              {destinationLabel}
            </Text>
          </View>

          {/* Folder list */}
          <ScrollView style={styles.list} showsVerticalScrollIndicator={false}>
            {/* Root / "no parent" option */}
            <Pressable
              style={[styles.folderRow, selectedParentId === null && styles.folderRowSelected]}
              onPress={() => setSelectedParentId(null)}
            >
              <Text style={styles.folderIcon}>🏠</Text>
              <Text
                style={[styles.folderName, selectedParentId === null && styles.folderNameSelected]}
                numberOfLines={1}
              >
                {t('documents.folderMove.rootLabel')}
              </Text>
              {selectedParentId === null && <Text style={styles.checkmark}>✓</Text>}
            </Pressable>

            {availableFolders.length === 0 ? (
              <View style={styles.emptyState}>
                <Text style={styles.emptyText}>{t('documents.folderMove.noFolders')}</Text>
              </View>
            ) : (
              availableFolders.map((folder) => {
                const isSelected = selectedParentId === folder.id;
                return (
                  <Pressable
                    key={folder.id}
                    style={[styles.folderRow, isSelected && styles.folderRowSelected]}
                    onPress={() => setSelectedParentId(folder.id)}
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
                <Text style={styles.moveBtnText}>{t('documents.folderMove.confirm')}</Text>
              )}
            </Pressable>
          </View>
        </View>
      </View>
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
