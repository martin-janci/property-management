import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Pressable,
  RefreshControl,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { type ApiFolderTreeNode, useFolderTree } from '../../hooks/useFolderTree';
import { colors } from '../shared/screenStyles';
import type { AccessScope } from './DocumentPermissionsScreen';
import { MoveDocumentSheet } from './MoveDocumentSheet';
import { NewFolderSheet } from './NewFolderSheet';

export type DocumentType = 'folder' | 'pdf' | 'image' | 'document' | 'spreadsheet';
export type DocumentStatus = 'published' | 'draft' | 'archived';

export interface Document {
  id: string;
  name: string;
  type: DocumentType;
  size?: number;
  createdAt: string;
  updatedAt: string;
  parentId: string | null;
  downloadUrl?: string;
  children?: Document[];
  /** Number of documents directly in a folder (folder rows only, gap-7a-2). */
  documentCount?: number;
  /** RLS-enforced audience scope returned from the server (gap-7a-3). */
  accessScope?: AccessScope;
  status?: DocumentStatus;
}

interface DocumentsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

/** Human-readable label + style for each access_scope value. */
const AUDIENCE_OPTIONS: ReadonlyArray<{
  value: AccessScope;
  label: string;
  color: string;
  bg: string;
  icon: string;
}> = [
  { value: 'organization', label: 'All Residents', color: '#2563eb', bg: '#eff6ff', icon: '🏘️' },
  { value: 'building', label: 'Building', color: '#4f46e5', bg: '#ede9fe', icon: '🏢' },
  { value: 'unit', label: 'Unit', color: '#d97706', bg: '#fef3c7', icon: '🚪' },
  { value: 'user', label: 'Specific Users', color: '#b45309', bg: '#fef3c7', icon: '👤' },
  { value: 'public', label: 'Public', color: '#047857', bg: '#d1fae5', icon: '🌍' },
] as const;

/**
 * Item shape of `GET /api/v1/documents` (`DocumentSummary` on the server).
 *
 * Fields mirror the backend exactly: `title` (not `name`), `file_name`
 * (not `file_path`), `mime_type` (not `content_type`), and `created_at`
 * (the summary has no separate upload timestamp). The summary carries no
 * `access_scope` or `status` — those live on the document detail/permissions
 * endpoints, not the list.
 */
export interface ApiDocument {
  id: string;
  title: string;
  category: string;
  file_name: string;
  mime_type: string;
  size_bytes: number;
  folder_id?: string | null;
  created_at: string;
}

interface ApiDocumentListResponse {
  documents: ApiDocument[];
  count?: number;
  total?: number;
}

// `ApiFolderTreeNode` is owned by the folder-tree hook (the data layer it
// describes). Re-exported here so existing `./DocumentsScreen` importers
// (index barrel, sheets, tests) keep working without a screen→hook coupling.
export type { ApiFolderTreeNode };

/** Lightweight breadcrumb entry: just enough to render + look up children. */
interface FolderCrumb {
  id: string;
  name: string;
}

/**
 * Walk the nested folder tree to the folder identified by `path` (a list of
 * folder ids from root downward) and return its direct subfolders. An empty
 * path returns the root-level folders.
 *
 * Exported for unit-testing (feat-mobile-document-folder-organization).
 */
export function subfoldersAtPath(
  tree: ApiFolderTreeNode[],
  path: ReadonlyArray<FolderCrumb>
): ApiFolderTreeNode[] {
  let level = tree;
  for (const crumb of path) {
    const match = level.find((n) => n.id === crumb.id);
    if (!match) return [];
    level = match.children ?? [];
  }
  return level;
}

/**
 * Map a folder tree node to the screen's unified `Document` row shape.
 *
 * Exported for unit-testing (feat-mobile-document-folder-organization).
 */
export function folderNodeToDocument(node: ApiFolderTreeNode): Document {
  return {
    id: node.id,
    name: node.name,
    type: 'folder',
    createdAt: '',
    updatedAt: '',
    parentId: node.parent_id ?? null,
    // Carry the live document count so the row meta ("N items") is real
    // rather than always 0. `children` here is the on-tree subfolder list,
    // which the row's "items" count intentionally does not use.
    documentCount: node.document_count,
  };
}

export function pickDocumentType(d: ApiDocument): DocumentType {
  const mime = (d.mime_type ?? '').toLowerCase();
  const fileName = (d.file_name ?? d.title).toLowerCase();
  if (mime.includes('pdf') || fileName.endsWith('.pdf')) return 'pdf';
  if (mime.startsWith('image/') || /\.(png|jpe?g|gif|webp)$/.test(fileName)) return 'image';
  if (mime.includes('spreadsheet') || mime.includes('excel') || /\.(xlsx?|csv)$/.test(fileName))
    return 'spreadsheet';
  return 'document';
}

export function toUiDocument(d: ApiDocument): Document {
  return {
    id: d.id,
    name: d.title,
    type: pickDocumentType(d),
    size: d.size_bytes ?? undefined,
    createdAt: d.created_at,
    updatedAt: d.created_at,
    parentId: d.folder_id ?? null,
    downloadUrl: undefined,
    children: undefined,
  };
}

/** Small inline badge showing the access_scope of a document row (gap-7a-3). */
function AudienceScopeBadge({ scope }: { scope: AccessScope }) {
  const opt = AUDIENCE_OPTIONS.find((o) => o.value === scope);
  if (!opt) return null;
  return (
    <View style={[audienceBadgeStyles.pill, { backgroundColor: opt.bg }]}>
      <Text style={audienceBadgeStyles.icon}>{opt.icon}</Text>
      <Text style={[audienceBadgeStyles.label, { color: opt.color }]}>{opt.label}</Text>
    </View>
  );
}

const audienceBadgeStyles = StyleSheet.create({
  pill: {
    flexDirection: 'row',
    alignItems: 'center',
    alignSelf: 'flex-start',
    borderRadius: 999,
    paddingHorizontal: 6,
    paddingVertical: 2,
    marginTop: 4,
    gap: 3,
  },
  icon: { fontSize: 10 },
  label: { fontSize: 10, fontWeight: '600' },
});

export function DocumentsScreen({ onNavigate: _onNavigate }: DocumentsScreenProps) {
  const { t } = useTranslation();
  /** Folder drill-down path from root → current folder (gap-7a-2). */
  const [currentPath, setCurrentPath] = useState<FolderCrumb[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [downloading, _setDownloading] = useState<string | null>(null);
  /** Active access_scope audience filter (gap-7a-3). Undefined = no filter. */
  const [selectedScope, setSelectedScope] = useState<AccessScope | undefined>(undefined);
  /** Controls the NewFolderSheet visibility (gap-7a-2). */
  const [showNewFolder, setShowNewFolder] = useState(false);
  /** Document targeted for a move operation (gap-7a-2). Null = sheet closed. */
  const [moveTarget, setMoveTarget] = useState<Document | null>(null);

  const currentFolder = currentPath.length > 0 ? currentPath[currentPath.length - 1] : null;

  // ── Live folder hierarchy (gap-7a-2) ──────────────────────────────────────
  // Drives the breadcrumb drill-down. Tenant scoping is enforced server-side
  // by the RLS connection behind GET /api/v1/documents/folders/tree.
  const {
    folderTree,
    isLoading: foldersLoading,
    error: foldersError,
    refetch: refetchFolders,
    isFetching: foldersFetching,
  } = useFolderTree();

  // ── Documents in the current folder ───────────────────────────────────────
  // When inside a folder we scope the list by `folder_id`. At the root we omit
  // it (the endpoint then returns every document) and filter client-side to
  // the ones with no parent, so the root view shows only top-level documents.
  const listQuery: string[] = [];
  if (currentFolder) listQuery.push(`folder_id=${currentFolder.id}`);
  if (selectedScope) listQuery.push(`access_scope=${selectedScope}`);
  const queryParams = listQuery.length > 0 ? `?${listQuery.join('&')}` : '';

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<ApiDocumentListResponse>(
    ['documents', 'list', currentFolder?.id ?? 'root', selectedScope],
    `/api/v1/documents${queryParams}`,
    { staleTime: 60_000 }
  );

  const apiDocuments: Document[] = (data?.documents ?? []).map(toUiDocument);
  // At the root the endpoint returns all documents; keep only the ones that
  // live at the root (no folder). Inside a folder the server already scoped
  // the list, so take everything it returned.
  const documents: Document[] = currentFolder
    ? apiDocuments
    : apiDocuments.filter((d) => d.parentId === null);

  const onRefresh = useCallback(async () => {
    await Promise.all([refetch(), refetchFolders()]);
  }, [refetch, refetchFolders]);

  const getCurrentDocuments = (): Document[] => {
    // Subfolders at this level come from the live folder tree; documents come
    // from the (folder-scoped) list query. Folders render first.
    const folders = subfoldersAtPath(folderTree, currentPath).map(folderNodeToDocument);
    return [...folders, ...documents];
  };

  const isLoadingView = isLoading || foldersLoading;
  const combinedError = error ?? foldersError;
  const isRefreshing = isFetching || foldersFetching;

  const getFileIcon = (type: DocumentType): string => {
    switch (type) {
      case 'folder':
        return '📁';
      case 'pdf':
        return '📄';
      case 'image':
        return '🖼️';
      case 'spreadsheet':
        return '📊';
      default:
        return '📃';
    }
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  };

  const navigateToFolder = (folder: Document) => {
    setCurrentPath((prev) => [...prev, { id: folder.id, name: folder.name }]);
  };

  const navigateBack = () => {
    setCurrentPath((prev) => prev.slice(0, -1));
  };

  const navigateToRoot = () => {
    setCurrentPath([]);
  };

  const handleDocumentPress = (doc: Document) => {
    if (doc.type === 'folder') {
      navigateToFolder(doc);
    } else {
      // Navigate to DocumentPreviewScreen which fetches a real presigned URL
      // (Story 7A.4 — replaces the earlier stub download simulation).
      _onNavigate?.('DocumentPreview', { document: doc });
    }
  };

  const filteredDocuments = getCurrentDocuments().filter(
    (doc) => searchQuery === '' || doc.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <View style={styles.container}>
      {/* Header */}
      <View style={styles.header}>
        <View style={styles.headerContent}>
          {currentPath.length > 0 ? (
            <>
              <Pressable style={styles.backButton} onPress={navigateBack}>
                <Text style={styles.backIcon}>←</Text>
              </Pressable>
              <Text style={styles.headerTitle} numberOfLines={1}>
                {currentPath[currentPath.length - 1].name}
              </Text>
              {/* New-folder action available inside a folder too */}
              <Pressable style={styles.newFolderButton} onPress={() => setShowNewFolder(true)}>
                <Text style={styles.newFolderButtonText}>📁+</Text>
              </Pressable>
            </>
          ) : (
            <>
              <Text style={styles.headerTitle}>{t('documents.title')}</Text>
              <View style={styles.headerActions}>
                <Pressable style={styles.newFolderButton} onPress={() => setShowNewFolder(true)}>
                  <Text style={styles.newFolderButtonText}>📁+</Text>
                </Pressable>
                <Pressable
                  style={styles.uploadButton}
                  onPress={() => _onNavigate?.('DocumentUpload')}
                >
                  <Text style={styles.uploadButtonText}>{t('documents.upload.buttonLabel')}</Text>
                </Pressable>
              </View>
            </>
          )}
        </View>
      </View>

      {/* Breadcrumb */}
      {currentPath.length > 0 && (
        <View style={styles.breadcrumb}>
          <Pressable onPress={navigateToRoot}>
            <Text style={styles.breadcrumbLink}>{t('documents.title')}</Text>
          </Pressable>
          {currentPath.map((folder, index) => (
            <View key={folder.id} style={styles.breadcrumbItem}>
              <Text style={styles.breadcrumbSeparator}>/</Text>
              {index === currentPath.length - 1 ? (
                <Text style={styles.breadcrumbCurrent}>{folder.name}</Text>
              ) : (
                <Pressable onPress={() => setCurrentPath(currentPath.slice(0, index + 1))}>
                  <Text style={styles.breadcrumbLink}>{folder.name}</Text>
                </Pressable>
              )}
            </View>
          ))}
        </View>
      )}

      {/* Audience filter (gap-7a-3 — RLS access_scope chips) */}
      <View style={styles.audienceFilterContainer}>
        <ScrollView
          horizontal
          showsHorizontalScrollIndicator={false}
          contentContainerStyle={styles.audienceChipsRow}
        >
          <Pressable
            style={[styles.audienceChip, selectedScope === undefined && styles.audienceChipAll]}
            onPress={() => setSelectedScope(undefined)}
          >
            <Text
              style={[
                styles.audienceChipText,
                selectedScope === undefined && styles.audienceChipTextActive,
              ]}
            >
              {t('documents.filter.all')}
            </Text>
          </Pressable>
          {AUDIENCE_OPTIONS.map((opt) => {
            const isActive = selectedScope === opt.value;
            return (
              <Pressable
                key={opt.value}
                style={[
                  styles.audienceChip,
                  isActive && { backgroundColor: opt.color, borderColor: opt.color },
                ]}
                onPress={() => setSelectedScope(isActive ? undefined : opt.value)}
              >
                <Text style={styles.audienceChipIcon}>{opt.icon}</Text>
                <Text style={[styles.audienceChipText, isActive && styles.audienceChipTextActive]}>
                  {opt.label}
                </Text>
              </Pressable>
            );
          })}
        </ScrollView>
      </View>

      {/* Search */}
      <View style={styles.searchContainer}>
        <TextInput
          style={styles.searchInput}
          placeholder={t('documents.searchPlaceholder')}
          value={searchQuery}
          onChangeText={setSearchQuery}
        />
      </View>

      {/* Documents List */}
      <ScrollView
        style={styles.scrollView}
        refreshControl={
          <RefreshControl
            refreshing={isRefreshing}
            onRefresh={onRefresh}
            tintColor={colors.accent}
          />
        }
      >
        {isLoadingView ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyTitle}>{t('common.loading') ?? 'Loading…'}</Text>
          </View>
        ) : combinedError ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyIcon}>⚠️</Text>
            <Text style={styles.emptyTitle}>{t('documents.loadError') ?? "Couldn't load"}</Text>
            <Text style={styles.emptyText}>{combinedError.message}</Text>
          </View>
        ) : filteredDocuments.length === 0 ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyIcon}>📂</Text>
            <Text style={styles.emptyTitle}>{t('documents.emptyTitle')}</Text>
            <Text style={styles.emptyText}>
              {searchQuery ? t('documents.noMatches') : t('documents.folderEmpty')}
            </Text>
          </View>
        ) : (
          <>
            {/* Folders first, then files */}
            {filteredDocuments
              .sort((a, b) => {
                if (a.type === 'folder' && b.type !== 'folder') return -1;
                if (a.type !== 'folder' && b.type === 'folder') return 1;
                return a.name.localeCompare(b.name);
              })
              .map((doc) => (
                <Pressable
                  key={doc.id}
                  style={styles.documentRow}
                  onPress={() => handleDocumentPress(doc)}
                >
                  <Text style={styles.fileIcon}>{getFileIcon(doc.type)}</Text>
                  <View style={styles.documentInfo}>
                    <Text style={styles.documentName} numberOfLines={1}>
                      {doc.name}
                    </Text>
                    <View style={styles.documentMeta}>
                      {doc.type === 'folder' ? (
                        <Text style={styles.metaText}>
                          {t('documents.folderItemCount', { count: doc.documentCount ?? 0 })}
                        </Text>
                      ) : (
                        <Text style={styles.metaText}>
                          {formatFileSize(doc.size || 0)} • {formatDate(doc.updatedAt)}
                        </Text>
                      )}
                    </View>
                    {/* Audience scope badge (gap-7a-3) */}
                    {doc.accessScope ? <AudienceScopeBadge scope={doc.accessScope} /> : null}
                  </View>
                  <View style={styles.rowActions}>
                    {/* Permissions detail button — documents only (folders
                        have no per-document RLS scope to inspect). */}
                    {doc.type !== 'folder' ? (
                      <>
                        <Pressable
                          style={styles.moveButton}
                          onPress={() => setMoveTarget(doc)}
                          hitSlop={8}
                        >
                          <Text style={styles.moveIcon}>↗</Text>
                        </Pressable>
                        <Pressable
                          style={styles.permissionsButton}
                          onPress={() =>
                            _onNavigate?.('DocumentPermissions', { documentId: doc.id })
                          }
                          hitSlop={8}
                        >
                          <Text style={styles.permissionsIcon}>🔒</Text>
                        </Pressable>
                      </>
                    ) : null}
                    {doc.type === 'folder' ? (
                      <Text style={styles.arrowIcon}>›</Text>
                    ) : downloading === doc.id ? (
                      <View style={styles.downloadingIndicator}>
                        <Text style={styles.downloadingText}>...</Text>
                      </View>
                    ) : (
                      <Text style={styles.downloadIcon}>⬇️</Text>
                    )}
                  </View>
                </Pressable>
              ))}
          </>
        )}

        <View style={styles.bottomSpacer} />
      </ScrollView>

      {/* New-folder sheet (gap-7a-2) */}
      <NewFolderSheet
        visible={showNewFolder}
        parentId={currentFolder?.id ?? null}
        parentName={currentFolder?.name ?? null}
        onClose={() => setShowNewFolder(false)}
        onCreated={() => {
          void refetchFolders();
        }}
      />

      {/* Move-document sheet (gap-7a-2) */}
      {moveTarget !== null && (
        <MoveDocumentSheet
          visible={moveTarget !== null}
          documentId={moveTarget.id}
          documentTitle={moveTarget.name}
          currentFolderId={moveTarget.parentId}
          folderTree={folderTree}
          onClose={() => setMoveTarget(null)}
          onMoved={() => {
            void onRefresh();
          }}
        />
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background,
  },
  header: {
    padding: 20,
    paddingTop: 60,
    backgroundColor: colors.surface,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  headerContent: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  uploadButton: {
    backgroundColor: colors.accent,
    borderRadius: 6,
    paddingHorizontal: 12,
    paddingVertical: 7,
  },
  uploadButtonText: {
    color: colors.white,
    fontSize: 13,
    fontWeight: '600',
  },
  backButton: {
    padding: 4,
    marginRight: 8,
  },
  backIcon: {
    fontSize: 24,
    color: colors.accent,
  },
  headerTitle: {
    fontSize: 24,
    fontWeight: 'bold',
    color: colors.text,
    flex: 1,
  },
  breadcrumb: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 16,
    paddingVertical: 8,
    backgroundColor: colors.surface,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
    flexWrap: 'wrap',
  },
  breadcrumbItem: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  breadcrumbLink: {
    fontSize: 13,
    color: colors.accent,
  },
  breadcrumbSeparator: {
    fontSize: 13,
    color: colors.textSubtle,
    marginHorizontal: 6,
  },
  breadcrumbCurrent: {
    fontSize: 13,
    color: colors.textMuted,
  },
  searchContainer: {
    padding: 16,
    paddingBottom: 8,
    backgroundColor: colors.surface,
  },
  searchInput: {
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
    padding: 12,
    fontSize: 16,
  },
  scrollView: {
    flex: 1,
  },
  emptyState: {
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 60,
  },
  emptyIcon: {
    fontSize: 48,
    marginBottom: 16,
  },
  emptyTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: colors.textSecondary,
    marginBottom: 4,
  },
  emptyText: {
    fontSize: 14,
    color: colors.textMuted,
  },
  documentRow: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.surface,
    paddingVertical: 14,
    paddingHorizontal: 16,
    borderBottomWidth: 1,
    borderBottomColor: colors.surfaceMuted,
  },
  fileIcon: {
    fontSize: 28,
    marginRight: 12,
  },
  documentInfo: {
    flex: 1,
  },
  documentName: {
    fontSize: 15,
    fontWeight: '500',
    color: colors.text,
    marginBottom: 2,
  },
  documentMeta: {
    flexDirection: 'row',
  },
  metaText: {
    fontSize: 12,
    color: colors.textSubtle,
  },
  arrowIcon: {
    fontSize: 24,
    color: colors.textSubtle,
  },
  downloadIcon: {
    fontSize: 18,
  },
  bottomSpacer: {
    height: 100,
  },
  // ── Header actions ──────────────────────────────────────────────────────
  headerActions: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 8,
  },
  newFolderButton: {
    paddingHorizontal: 10,
    paddingVertical: 6,
    borderRadius: 6,
    borderWidth: 1,
    borderColor: colors.border,
    backgroundColor: colors.surface,
  },
  newFolderButtonText: {
    fontSize: 16,
  },
  // ── Move button ──────────────────────────────────────────────────────────
  moveButton: {
    padding: 4,
  },
  moveIcon: {
    fontSize: 16,
    color: colors.textMuted,
  },
  // ── Audience filter strip (gap-7a-3) ──────────────────────────────────────
  audienceFilterContainer: {
    backgroundColor: colors.surface,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
    paddingVertical: 8,
  },
  audienceChipsRow: {
    flexDirection: 'row',
    paddingHorizontal: 12,
    gap: 6,
  },
  audienceChip: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 10,
    paddingVertical: 6,
    borderRadius: 999,
    borderWidth: 1,
    borderColor: colors.border,
    backgroundColor: colors.surface,
    gap: 4,
  },
  audienceChipAll: {
    borderColor: colors.accent,
    backgroundColor: colors.accentSoft,
  },
  audienceChipIcon: {
    fontSize: 13,
  },
  audienceChipText: {
    fontSize: 12,
    fontWeight: '500',
    color: colors.textMuted,
  },
  audienceChipTextActive: {
    color: colors.white,
  },
  // ── Row actions / permissions button ──────────────────────────────────────
  rowActions: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 8,
  },
  permissionsButton: {
    padding: 4,
  },
  permissionsIcon: {
    fontSize: 14,
  },
  downloadingIndicator: {
    width: 24,
    height: 24,
    alignItems: 'center',
    justifyContent: 'center',
  },
  downloadingText: {
    fontSize: 14,
    color: colors.textMuted,
  },
});
