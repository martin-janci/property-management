import * as Sharing from 'expo-sharing';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Alert,
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

export type DocumentType = 'folder' | 'pdf' | 'image' | 'document' | 'spreadsheet';

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
}

interface DocumentsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

/** Subset of `Document` returned by `GET /api/v1/documents`. */
interface ApiDocument {
  id: string;
  name: string;
  file_path?: string | null;
  size_bytes?: number | null;
  content_type?: string | null;
  uploaded_at?: string | null;
  created_at: string;
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
  };
}

export function DocumentsScreen({ onNavigate: _onNavigate }: DocumentsScreenProps) {
  const { t } = useTranslation();
  const [currentPath, setCurrentPath] = useState<Document[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [downloading, setDownloading] = useState<string | null>(null);

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<ApiDocumentListResponse>(
    ['documents', 'list'],
    '/api/v1/documents',
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

  const handleDocumentPress = async (doc: Document) => {
    if (doc.type === 'folder') {
      navigateToFolder(doc);
    } else {
      // Download/View document
      setDownloading(doc.id);
      try {
        // Simulate download
        await new Promise((resolve) => setTimeout(resolve, 1500));

        // In a real app, this would download from the actual URL
        // and open with the appropriate viewer
        Alert.alert(t('documents.readyTitle'), t('documents.readyMessage', { name: doc.name }), [
          { text: t('common.close'), style: 'cancel' },
          {
            text: t('documents.share'),
            onPress: async () => {
              if (await Sharing.isAvailableAsync()) {
                // Would share the actual downloaded file
                Alert.alert(t('documents.sharing'), t('documents.sharingMessage'));
              }
            },
          },
        ]);
      } catch (_error) {
        Alert.alert(t('common.error'), t('documents.downloadFailed'));
      } finally {
        setDownloading(null);
      }
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
                  style={[styles.documentRow, downloading === doc.id && styles.documentDownloading]}
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
                  </View>
                  {doc.type === 'folder' ? (
                    <Text style={styles.arrowIcon}>›</Text>
                  ) : downloading === doc.id ? (
                    <View style={styles.downloadingIndicator}>
                      <Text style={styles.downloadingText}>...</Text>
                    </View>
                  ) : (
                    <Text style={styles.downloadIcon}>⬇️</Text>
                  )}
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
  documentDownloading: {
    backgroundColor: colors.surfaceMuted,
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
  downloadingIndicator: {
    width: 24,
    alignItems: 'center',
  },
  downloadingText: {
    fontSize: 14,
    color: colors.textMuted,
  },
  bottomSpacer: {
    height: 100,
  },
});
