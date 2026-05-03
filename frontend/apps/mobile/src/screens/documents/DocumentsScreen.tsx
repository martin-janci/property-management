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

// Mock data - hierarchical document structure
const mockDocuments: Document[] = [
  {
    id: '1',
    name: 'Building Rules',
    type: 'folder',
    createdAt: '2025-01-01T00:00:00Z',
    updatedAt: '2025-12-20T00:00:00Z',
    parentId: null,
    children: [
      {
        id: '1-1',
        name: 'House Rules 2025.pdf',
        type: 'pdf',
        size: 245000,
        createdAt: '2025-01-01T00:00:00Z',
        updatedAt: '2025-01-01T00:00:00Z',
        parentId: '1',
        downloadUrl: '#',
      },
      {
        id: '1-2',
        name: 'Parking Regulations.pdf',
        type: 'pdf',
        size: 128000,
        createdAt: '2025-03-15T00:00:00Z',
        updatedAt: '2025-03-15T00:00:00Z',
        parentId: '1',
        downloadUrl: '#',
      },
    ],
  },
  {
    id: '2',
    name: 'Financial Reports',
    type: 'folder',
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2025-12-15T00:00:00Z',
    parentId: null,
    children: [
      {
        id: '2-1',
        name: 'Annual Report 2024.pdf',
        type: 'pdf',
        size: 1250000,
        createdAt: '2025-01-15T00:00:00Z',
        updatedAt: '2025-01-15T00:00:00Z',
        parentId: '2',
        downloadUrl: '#',
      },
      {
        id: '2-2',
        name: 'Budget 2025.xlsx',
        type: 'spreadsheet',
        size: 85000,
        createdAt: '2024-12-01T00:00:00Z',
        updatedAt: '2024-12-01T00:00:00Z',
        parentId: '2',
        downloadUrl: '#',
      },
      {
        id: '2-3',
        name: 'Q4 2024 Statement.pdf',
        type: 'pdf',
        size: 320000,
        createdAt: '2025-01-05T00:00:00Z',
        updatedAt: '2025-01-05T00:00:00Z',
        parentId: '2',
        downloadUrl: '#',
      },
    ],
  },
  {
    id: '3',
    name: 'Meeting Minutes',
    type: 'folder',
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2025-11-20T00:00:00Z',
    parentId: null,
    children: [
      {
        id: '3-1',
        name: 'November 2025 Meeting.pdf',
        type: 'pdf',
        size: 180000,
        createdAt: '2025-11-20T00:00:00Z',
        updatedAt: '2025-11-20T00:00:00Z',
        parentId: '3',
        downloadUrl: '#',
      },
      {
        id: '3-2',
        name: 'October 2025 Meeting.pdf',
        type: 'pdf',
        size: 165000,
        createdAt: '2025-10-18T00:00:00Z',
        updatedAt: '2025-10-18T00:00:00Z',
        parentId: '3',
        downloadUrl: '#',
      },
    ],
  },
  {
    id: '4',
    name: 'Maintenance Manual.pdf',
    type: 'pdf',
    size: 3500000,
    createdAt: '2024-06-01T00:00:00Z',
    updatedAt: '2024-06-01T00:00:00Z',
    parentId: null,
    downloadUrl: '#',
  },
  {
    id: '5',
    name: 'Contact List.pdf',
    type: 'pdf',
    size: 45000,
    createdAt: '2025-12-01T00:00:00Z',
    updatedAt: '2025-12-01T00:00:00Z',
    parentId: null,
    downloadUrl: '#',
  },
];

interface DocumentsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function DocumentsScreen({ onNavigate: _onNavigate }: DocumentsScreenProps) {
  const { t } = useTranslation();
  const [refreshing, setRefreshing] = useState(false);
  const [documents] = useState<Document[]>(mockDocuments);
  const [currentPath, setCurrentPath] = useState<Document[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [downloading, setDownloading] = useState<string | null>(null);

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((resolve) => setTimeout(resolve, 1000));
    setRefreshing(false);
  }, []);

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
            <Text style={styles.headerTitle}>{t('documents.title')}</Text>
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
          <RefreshControl refreshing={refreshing} onRefresh={onRefresh} tintColor={colors.accent} />
        }
      >
        {filteredDocuments.length === 0 ? (
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
