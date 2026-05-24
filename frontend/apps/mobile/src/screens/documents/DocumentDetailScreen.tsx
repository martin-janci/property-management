/**
 * DocumentDetailScreen (UC-08.4 + Story 7A.5).
 *
 * Single-document view with metadata, an "open" action that hands the URL off
 * to the platform viewer, and a "Share" button that opens DocumentShareSheet.
 */

import { useState } from 'react';
import { Alert, Linking, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { colors, screenStyles as s } from '../shared/screenStyles';
import { DocumentShareSheet } from './DocumentShareSheet';
import type { Document } from './DocumentsScreen';

const MOCK_DOCUMENT: Document = {
  id: 'd1',
  name: 'House Rules 2026.pdf',
  type: 'pdf',
  size: 1_245_678,
  createdAt: '2026-03-12T08:30:00Z',
  updatedAt: '2026-03-12T08:30:00Z',
  parentId: null,
  downloadUrl: 'https://example.com/house-rules-2026.pdf',
};

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

interface DocumentDetailScreenProps {
  documentId?: string;
  document?: Document;
  onBack?: () => void;
}

export function DocumentDetailScreen({
  document = MOCK_DOCUMENT,
  onBack,
}: DocumentDetailScreenProps) {
  const [shareSheetVisible, setShareSheetVisible] = useState(false);

  const handleOpen = async () => {
    if (!document.downloadUrl) {
      Alert.alert('Unavailable', 'No file URL is associated with this document.');
      return;
    }
    try {
      const supported = await Linking.canOpenURL(document.downloadUrl);
      if (!supported) throw new Error('Cannot open URL');
      await Linking.openURL(document.downloadUrl);
    } catch {
      Alert.alert('Could not open document', 'The file viewer is unavailable on this device.');
    }
  };

  return (
    <View style={s.container}>
      <View style={s.header}>
        {onBack && (
          <Pressable onPress={onBack} style={styles.backLink}>
            <Text style={styles.backLinkText}>← Documents</Text>
          </Pressable>
        )}
        <Text style={s.headerTitle}>{document.name}</Text>
      </View>

      <ScrollView style={s.scrollView}>
        <View style={s.card}>
          <View style={s.cardHeader}>
            <Text style={styles.icon}>{document.type === 'pdf' ? '📄' : '🗂️'}</Text>
            <Text style={[s.cardTitle, { marginLeft: 8 }]}>{document.name}</Text>
          </View>
          <View style={[s.cardFooter, { marginTop: 16 }]}>
            {typeof document.size === 'number' && (
              <Text style={s.cardMeta}>{formatBytes(document.size)}</Text>
            )}
            <Text style={s.cardMeta}>
              Updated {new Date(document.updatedAt).toLocaleDateString()}
            </Text>
          </View>
        </View>

        <View style={s.card}>
          <Text style={styles.sectionLabel}>Type</Text>
          <Text style={styles.sectionValue}>{document.type.toUpperCase()}</Text>
          <Text style={[styles.sectionLabel, { marginTop: 12 }]}>Created</Text>
          <Text style={styles.sectionValue}>
            {new Date(document.createdAt).toLocaleDateString()}
          </Text>
        </View>

        <Pressable style={s.primaryButton} onPress={handleOpen}>
          <Text style={s.primaryButtonText}>Open document</Text>
        </Pressable>

        <Pressable style={styles.shareButton} onPress={() => setShareSheetVisible(true)}>
          <Text style={styles.shareButtonText}>Share</Text>
        </Pressable>

        <View style={s.bottomSpacer} />
      </ScrollView>

      <DocumentShareSheet
        documentId={document.id}
        documentTitle={document.name}
        visible={shareSheetVisible}
        onClose={() => setShareSheetVisible(false)}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  backLink: { marginBottom: 8 },
  backLinkText: { color: colors.accent, fontSize: 14 },
  icon: { fontSize: 24 },
  sectionLabel: {
    fontSize: 12,
    color: colors.textMuted,
    fontWeight: '600',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  sectionValue: { fontSize: 16, color: colors.text, marginTop: 4 },
  shareButton: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 14,
    paddingHorizontal: 16,
    borderRadius: 8,
    backgroundColor: colors.surface,
    borderWidth: 1,
    borderColor: colors.accent,
    marginTop: 12,
  },
  shareButtonText: {
    color: colors.accent,
    fontSize: 16,
    fontWeight: '600',
  },
});
