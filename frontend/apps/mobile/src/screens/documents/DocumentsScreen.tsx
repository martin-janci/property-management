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
import { colors } from '../shared/screenStyles';
import type { AccessScope } from './DocumentPermissionsScreen';

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

/** Subset of `Document` returned by `GET /api/v1/documents`. */
interface ApiDocument {
  id: string;
  name: string;
  file_path?: string | null;
  size_bytes?: number | null;
  content_type?: string | null;
  uploaded_at?: string | null;
  created_at: string;
  /** RLS-enforced audience scope (gap-7a-3). */
  access_scope?: AccessScope;
  /** Publication status: absent means published (backward compat). */
  status?: DocumentStatus;
}

interface ApiDocumentListResponse {
  documents: ApiDocument[];
  total?: number;
}

function pickDocumentType(d: ApiDocument): DocumentType {
  const ct = (d.content_type ?? '').toLowerCase();
  const name = d.name.toLowerCase();
  if (ct.includes('pdf') || name.endsWith('.pdf')) return 'pdf';
  if (ct.startsWith('image/') || /\.(png|jpe?g|gif|webp)$/.test(name)) return 'image';
  if (ct.includes('spreadsheet') || ct.includes('excel') || /\.(xlsx?|csv)$/.test(name))
    return 'spreadsheet';
  return 'document';
}

function toUiDocument(d: ApiDocument): Document {
  return {
    id: d.id,
    name: d.name,
    type: pickDocumentType(d),
    size: d.size_bytes ?? undefined,
    createdAt: d.created_at,
    updatedAt: d.uploaded_at ?? d.created_at,
    parentId: null,
    downloadUrl: d.file_path ?? undefined,
    children: undefined,
    accessScope: d.access_scope,
    status: d.status,
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
  const [currentPath, setCurrentPath] = useState<Document[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [downloading, setDownloading] = useState<string | null>(null);
  /** Active access_scope audience filter (gap-7a-3). Undefined = no filter. */
  const [selectedScope, setSelectedScope] = useState<AccessScope | undefined>(undefined);

  // Build query params: include access_scope when a filter is active.
  const queryParams = selectedScope ? `?access_scope=${selectedScope}` : '';

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<ApiDocumentListResponse>(
    ['documents', 'list', selectedScope],
    `/api/v1/documents${queryParams}`,
    { staleTime: 60_000 }
  );

  // The api-server returns a flat list; the screen still supports a
  // folder breadcrumb but at this layer all documents live at the root.
  const documents: Document[] = (data?.documents ?? []).map(toUiDocument);

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  const getCurrentDocuments = (): Document[] => {
    if (currentPath.length === 0) {
      return documents;
    }
    const current = currentPath[currentPath.length - 1];
    return current.children || [];
  };

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
    setCurrentPath((prev) => [...prev, folder]);
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
            </>
          ) : (
            <>
              <Text style={styles.headerTitle}>{t('documents.title')}</Text>
              <Pressable
                style={styles.uploadButton}
                onPress={() => _onNavigate?.('DocumentUpload')}
              >
                <Text style={styles.uploadButtonText}>{t('documents.upload.buttonLabel')}</Text>
              </Pressable>
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
          <RefreshControl refreshing={isFetching} onRefresh={onRefresh} tintColor={colors.accent} />
        }
      >
        {isLoading ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyTitle}>{t('common.loading') ?? 'Loading…'}</Text>
          </View>
        ) : error ? (
          <View style={styles.emptyState}>
            <Text style={styles.emptyIcon}>⚠️</Text>
            <Text style={styles.emptyTitle}>{t('documents.loadError') ?? "Couldn't load"}</Text>
            <Text style={styles.emptyText}>{error.message}</Text>
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
                        <Text style={styles.metaText}>{doc.children?.length || 0} items</Text>
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
                    {/* Permissions detail button */}
                    <Pressable
                      style={styles.permissionsButton}
                      onPress={() => _onNavigate?.('DocumentPermissions', { documentId: doc.id })}
                      hitSlop={8}
                    >
                      <Text style={styles.permissionsIcon}>🔒</Text>
                    </Pressable>
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
