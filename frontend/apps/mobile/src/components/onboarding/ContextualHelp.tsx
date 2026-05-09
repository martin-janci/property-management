/**
 * Contextual help component.
 *
 * Epic 50 - Story 50.2: Contextual Help
 */
import { useCallback, useMemo, useState } from 'react';
import { Animated, Modal, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';

import { helpCenter, tourManager } from '../../onboarding';
import { colors } from '../../screens/shared/screenStyles';

interface ContextualHelpProps {
  screenId: string;
  visible: boolean;
  onClose: () => void;
  onNavigateToFAQ?: (faqId: string) => void;
  onNavigateToTutorial?: (tutorialId: string) => void;
}

export function ContextualHelp({
  screenId,
  visible,
  onClose,
  onNavigateToFAQ,
  onNavigateToTutorial,
}: ContextualHelpProps) {
  const [showFullContent, setShowFullContent] = useState(false);

  const helpContent = useMemo(() => helpCenter.getHelpForScreen(screenId), [screenId]);

  const relatedFAQs = useMemo(() => {
    if (!helpContent?.relatedFAQs) return [];
    return helpContent.relatedFAQs
      .map((id) => helpCenter.searchFAQs('').find((f) => f.id === id))
      .filter(Boolean);
  }, [helpContent]);

  const relatedTutorials = useMemo(() => {
    if (!helpContent?.relatedTutorials) return [];
    return helpContent.relatedTutorials.map((id) => helpCenter.getTutorial(id)).filter(Boolean);
  }, [helpContent]);

  const handleDismiss = useCallback(async () => {
    await tourManager.dismissHelp(screenId);
    onClose();
  }, [screenId, onClose]);

  if (!visible || !helpContent) {
    return null;
  }

  return (
    <Modal visible={visible} transparent animationType="slide">
      <View style={styles.container}>
        <Pressable style={styles.backdrop} onPress={onClose} />

        <Animated.View style={styles.panel}>
          <View style={styles.handle} />

          <View style={styles.header}>
            <Text style={styles.title}>{helpContent.title}</Text>
            <Pressable onPress={onClose} style={styles.closeButton}>
              <Text style={styles.closeText}>✕</Text>
            </Pressable>
          </View>

          <ScrollView style={styles.content} showsVerticalScrollIndicator={false}>
            <Text style={styles.description}>{helpContent.shortDescription}</Text>

            {showFullContent ? (
              <View style={styles.fullContent}>
                <Text style={styles.fullContentText}>{helpContent.fullContent}</Text>
                <Pressable onPress={() => setShowFullContent(false)}>
                  <Text style={styles.showLessText}>Show less</Text>
                </Pressable>
              </View>
            ) : (
              <Pressable onPress={() => setShowFullContent(true)}>
                <Text style={styles.showMoreText}>Learn more →</Text>
              </Pressable>
            )}

            {relatedFAQs.length > 0 && (
              <View style={styles.section}>
                <Text style={styles.sectionTitle}>Related FAQs</Text>
                {relatedFAQs.map(
                  (faq) =>
                    faq && (
                      <Pressable
                        key={faq.id}
                        style={styles.faqItem}
                        onPress={() => onNavigateToFAQ?.(faq.id)}
                      >
                        <Text style={styles.faqQuestion}>{faq.question}</Text>
                        <Text style={styles.chevron}>›</Text>
                      </Pressable>
                    )
                )}
              </View>
            )}

            {relatedTutorials.length > 0 && (
              <View style={styles.section}>
                <Text style={styles.sectionTitle}>Tutorials</Text>
                {relatedTutorials.map(
                  (tutorial) =>
                    tutorial && (
                      <Pressable
                        key={tutorial.id}
                        style={styles.tutorialItem}
                        onPress={() => onNavigateToTutorial?.(tutorial.id)}
                      >
                        <View style={styles.playIcon}>
                          <Text style={styles.playText}>▶</Text>
                        </View>
                        <View style={styles.tutorialInfo}>
                          <Text style={styles.tutorialTitle}>{tutorial.title}</Text>
                          <Text style={styles.tutorialDuration}>
                            {Math.ceil(tutorial.duration / 60)} min
                          </Text>
                        </View>
                      </Pressable>
                    )
                )}
              </View>
            )}
          </ScrollView>

          <View style={styles.footer}>
            <Pressable style={styles.dismissButton} onPress={handleDismiss}>
              <Text style={styles.dismissText}>Don't show again</Text>
            </Pressable>
            <Pressable style={styles.helpCenterButton} onPress={onClose}>
              <Text style={styles.helpCenterText}>Help Center</Text>
            </Pressable>
          </View>
        </Animated.View>
      </View>
    </Modal>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    justifyContent: 'flex-end',
  },
  backdrop: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: 'rgba(0, 0, 0, 0.4)',
  },
  panel: {
    backgroundColor: colors.white,
    borderTopLeftRadius: 24,
    borderTopRightRadius: 24,
    maxHeight: '70%',
    shadowColor: colors.shadow,
    shadowOffset: { width: 0, height: -4 },
    shadowOpacity: 0.1,
    shadowRadius: 12,
    elevation: 20,
  },
  handle: {
    width: 40,
    height: 4,
    backgroundColor: colors.borderStrong,
    borderRadius: 2,
    alignSelf: 'center',
    marginTop: 12,
    marginBottom: 8,
  },
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingHorizontal: 20,
    paddingVertical: 12,
    borderBottomWidth: 1,
    borderBottomColor: colors.surfaceMuted,
  },
  title: {
    fontSize: 20,
    fontWeight: 'bold',
    color: colors.text,
  },
  closeButton: {
    width: 32,
    height: 32,
    borderRadius: 16,
    backgroundColor: colors.surfaceMuted,
    alignItems: 'center',
    justifyContent: 'center',
  },
  closeText: {
    fontSize: 16,
    color: colors.textMuted,
  },
  content: {
    padding: 20,
  },
  description: {
    fontSize: 16,
    color: colors.textMuted,
    lineHeight: 24,
    marginBottom: 12,
  },
  showMoreText: {
    fontSize: 16,
    color: colors.accent,
    fontWeight: '500',
  },
  fullContent: {
    backgroundColor: colors.background,
    borderRadius: 12,
    padding: 16,
    marginTop: 12,
  },
  fullContentText: {
    fontSize: 15,
    color: colors.textMuted,
    lineHeight: 22,
    marginBottom: 12,
  },
  showLessText: {
    fontSize: 14,
    color: colors.accent,
    fontWeight: '500',
  },
  section: {
    marginTop: 24,
  },
  sectionTitle: {
    fontSize: 14,
    fontWeight: '600',
    color: colors.textSubtle,
    textTransform: 'uppercase',
    letterSpacing: 0.5,
    marginBottom: 12,
  },
  faqItem: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.surfaceMuted,
    borderRadius: 12,
    padding: 16,
    marginBottom: 8,
  },
  faqQuestion: {
    flex: 1,
    fontSize: 15,
    color: colors.text,
  },
  chevron: {
    fontSize: 20,
    color: colors.textSubtle,
    marginLeft: 8,
  },
  tutorialItem: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.surfaceMuted,
    borderRadius: 12,
    padding: 12,
    marginBottom: 8,
  },
  playIcon: {
    width: 40,
    height: 40,
    borderRadius: 20,
    backgroundColor: colors.accent,
    alignItems: 'center',
    justifyContent: 'center',
    marginRight: 12,
  },
  playText: {
    color: colors.white,
    fontSize: 14,
  },
  tutorialInfo: {
    flex: 1,
  },
  tutorialTitle: {
    fontSize: 15,
    fontWeight: '500',
    color: colors.text,
    marginBottom: 2,
  },
  tutorialDuration: {
    fontSize: 13,
    color: colors.textSubtle,
  },
  footer: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: 16,
    borderTopWidth: 1,
    borderTopColor: colors.surfaceMuted,
    paddingBottom: 32,
  },
  dismissButton: {
    paddingVertical: 10,
    paddingHorizontal: 16,
  },
  dismissText: {
    fontSize: 14,
    color: colors.textSubtle,
  },
  helpCenterButton: {
    backgroundColor: colors.accent,
    paddingVertical: 12,
    paddingHorizontal: 20,
    borderRadius: 8,
  },
  helpCenterText: {
    fontSize: 15,
    fontWeight: '600',
    color: colors.white,
  },
});
