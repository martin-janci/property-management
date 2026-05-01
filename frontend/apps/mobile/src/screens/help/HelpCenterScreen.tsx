/**
 * Help center screen with FAQs and tutorials.
 *
 * Epic 50 - Story 50.2-50.3: Contextual Help, FAQ & Tutorials
 */
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Pressable, ScrollView, StyleSheet, Text, TextInput, View } from 'react-native';

import { helpCenter } from '../../onboarding';
import type { FAQCategory, FAQItem, Tutorial } from '../../onboarding/types';
import { colors } from '../shared/screenStyles';

interface HelpCenterScreenProps {
  onNavigate: (screen: string, params?: Record<string, unknown>) => void;
}

export function HelpCenterScreen({ onNavigate }: HelpCenterScreenProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<FAQCategory | null>(null);
  const [activeTab, setActiveTab] = useState<'faq' | 'tutorials'>('faq');
  const [expandedFAQ, setExpandedFAQ] = useState<string | null>(null);

  const categories = useMemo(() => helpCenter.getFAQCategories(), []);

  const filteredFAQs = useMemo(() => {
    return helpCenter.searchFAQs(searchQuery, selectedCategory ?? undefined);
  }, [searchQuery, selectedCategory]);

  const filteredTutorials = useMemo(() => {
    if (searchQuery) {
      return helpCenter.searchTutorials(searchQuery);
    }
    if (selectedCategory) {
      return helpCenter.getTutorialsByCategory(selectedCategory);
    }
    return helpCenter.getAllTutorials();
  }, [searchQuery, selectedCategory]);

  const handleFAQPress = useCallback((faqId: string) => {
    setExpandedFAQ((prev) => (prev === faqId ? null : faqId));
  }, []);

  const handleVote = useCallback(async (faqId: string, helpful: boolean) => {
    await helpCenter.voteFAQ(faqId, helpful);
  }, []);

  const handleTutorialPress = useCallback(
    (tutorial: Tutorial) => {
      onNavigate('TutorialDetail', { tutorialId: tutorial.id });
    },
    [onNavigate]
  );

  const renderFAQItem = useCallback(
    (faq: FAQItem) => {
      const isExpanded = expandedFAQ === faq.id;
      const userVote = helpCenter.getUserVote(faq.id);

      return (
        <View key={faq.id} style={styles.faqItem}>
          <Pressable style={styles.faqQuestion} onPress={() => handleFAQPress(faq.id)}>
            <Text style={styles.faqQuestionText}>{faq.question}</Text>
            <Text style={styles.expandIcon}>{isExpanded ? '−' : '+'}</Text>
          </Pressable>

          {isExpanded && (
            <View style={styles.faqAnswer}>
              <Text style={styles.faqAnswerText}>{faq.answer}</Text>

              <View style={styles.faqFeedback}>
                <Text style={styles.feedbackLabel}>{t('help.wasHelpful')}</Text>
                <View style={styles.feedbackButtons}>
                  <Pressable
                    style={[styles.feedbackButton, userVote === 'helpful' && styles.feedbackActive]}
                    onPress={() => handleVote(faq.id, true)}
                  >
                    <Text
                      style={[
                        styles.feedbackButtonText,
                        userVote === 'helpful' && styles.feedbackActiveText,
                      ]}
                    >
                      👍 {faq.helpful}
                    </Text>
                  </Pressable>
                  <Pressable
                    style={[
                      styles.feedbackButton,
                      userVote === 'not_helpful' && styles.feedbackActive,
                    ]}
                    onPress={() => handleVote(faq.id, false)}
                  >
                    <Text
                      style={[
                        styles.feedbackButtonText,
                        userVote === 'not_helpful' && styles.feedbackActiveText,
                      ]}
                    >
                      👎 {faq.notHelpful}
                    </Text>
                  </Pressable>
                </View>
              </View>
            </View>
          )}
        </View>
      );
    },
    [expandedFAQ, handleFAQPress, handleVote, t]
  );

  const renderTutorialItem = useCallback(
    (tutorial: Tutorial) => {
      const durationMin = Math.ceil(tutorial.duration / 60);

      return (
        <Pressable
          key={tutorial.id}
          style={styles.tutorialItem}
          onPress={() => handleTutorialPress(tutorial)}
        >
          <View style={styles.tutorialThumbnail}>
            <Text style={styles.playIcon}>▶</Text>
          </View>
          <View style={styles.tutorialInfo}>
            <Text style={styles.tutorialTitle}>{tutorial.title}</Text>
            <Text style={styles.tutorialDescription} numberOfLines={2}>
              {tutorial.description}
            </Text>
            <Text style={styles.tutorialDuration}>{durationMin} min</Text>
          </View>
        </Pressable>
      );
    },
    [handleTutorialPress]
  );

  return (
    <View style={styles.container}>
      <View style={styles.header}>
        <Pressable onPress={() => onNavigate('Dashboard')} style={styles.backButton}>
          <Text style={styles.backButtonText}>← {t('common.back')}</Text>
        </Pressable>
        <Text style={styles.title}>{t('help.title')}</Text>
      </View>

      <View style={styles.searchContainer}>
        <TextInput
          style={styles.searchInput}
          placeholder={t('help.searchPlaceholder')}
          value={searchQuery}
          onChangeText={setSearchQuery}
          placeholderTextColor={colors.textSubtle}
        />
      </View>

      <View style={styles.tabs}>
        <Pressable
          style={[styles.tab, activeTab === 'faq' && styles.tabActive]}
          onPress={() => setActiveTab('faq')}
        >
          <Text style={[styles.tabText, activeTab === 'faq' && styles.tabTextActive]}>
            {t('help.faqTab')}
          </Text>
        </Pressable>
        <Pressable
          style={[styles.tab, activeTab === 'tutorials' && styles.tabActive]}
          onPress={() => setActiveTab('tutorials')}
        >
          <Text style={[styles.tabText, activeTab === 'tutorials' && styles.tabTextActive]}>
            {t('help.tutorialsTab')}
          </Text>
        </Pressable>
      </View>

      <ScrollView
        horizontal
        showsHorizontalScrollIndicator={false}
        style={styles.categoriesContainer}
        contentContainerStyle={styles.categoriesContent}
      >
        <Pressable
          style={[styles.categoryChip, selectedCategory === null && styles.categoryChipActive]}
          onPress={() => setSelectedCategory(null)}
        >
          <Text
            style={[
              styles.categoryChipText,
              selectedCategory === null && styles.categoryChipTextActive,
            ]}
          >
            All
          </Text>
        </Pressable>
        {categories.map((cat) => (
          <Pressable
            key={cat.category}
            style={[
              styles.categoryChip,
              selectedCategory === cat.category && styles.categoryChipActive,
            ]}
            onPress={() => setSelectedCategory(cat.category)}
          >
            <Text
              style={[
                styles.categoryChipText,
                selectedCategory === cat.category && styles.categoryChipTextActive,
              ]}
            >
              {cat.label} ({cat.count})
            </Text>
          </Pressable>
        ))}
      </ScrollView>

      <ScrollView style={styles.content}>
        {activeTab === 'faq' ? (
          <>
            {filteredFAQs.length === 0 ? (
              <View style={styles.emptyState}>
                <Text style={styles.emptyIcon}>🔍</Text>
                <Text style={styles.emptyTitle}>{t('help.noFAQs')}</Text>
                <Text style={styles.emptyText}>{t('help.noFAQsMessage')}</Text>
              </View>
            ) : (
              filteredFAQs.map(renderFAQItem)
            )}
          </>
        ) : (
          <>
            {filteredTutorials.length === 0 ? (
              <View style={styles.emptyState}>
                <Text style={styles.emptyIcon}>🎬</Text>
                <Text style={styles.emptyTitle}>{t('help.noTutorials')}</Text>
                <Text style={styles.emptyText}>{t('help.noTutorialsMessage')}</Text>
              </View>
            ) : (
              filteredTutorials.map(renderTutorialItem)
            )}
          </>
        )}
      </ScrollView>

      <Pressable style={styles.feedbackFab} onPress={() => onNavigate('Feedback')}>
        <Text style={styles.feedbackFabText}>💬 {t('help.sendFeedbackButton')}</Text>
      </Pressable>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background,
  },
  header: {
    backgroundColor: colors.surface,
    paddingTop: 60,
    paddingHorizontal: 16,
    paddingBottom: 16,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  backButton: {
    marginBottom: 8,
  },
  backButtonText: {
    color: colors.accent,
    fontSize: 16,
  },
  title: {
    fontSize: 24,
    fontWeight: 'bold',
    color: colors.text,
  },
  searchContainer: {
    backgroundColor: colors.surface,
    paddingHorizontal: 16,
    paddingVertical: 12,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  searchInput: {
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
    paddingHorizontal: 16,
    paddingVertical: 12,
    fontSize: 16,
    color: colors.text,
  },
  tabs: {
    flexDirection: 'row',
    backgroundColor: colors.surface,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  tab: {
    flex: 1,
    paddingVertical: 14,
    alignItems: 'center',
    borderBottomWidth: 2,
    borderBottomColor: 'transparent',
  },
  tabActive: {
    borderBottomColor: colors.accent,
  },
  tabText: {
    fontSize: 16,
    fontWeight: '500',
    color: colors.textMuted,
  },
  tabTextActive: {
    color: colors.accent,
  },
  categoriesContainer: {
    backgroundColor: colors.surface,
    maxHeight: 56,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  categoriesContent: {
    paddingHorizontal: 12,
    paddingVertical: 10,
    gap: 8,
  },
  categoryChip: {
    paddingHorizontal: 14,
    paddingVertical: 8,
    backgroundColor: colors.surfaceMuted,
    borderRadius: 20,
    marginRight: 8,
  },
  categoryChipActive: {
    backgroundColor: colors.accent,
  },
  categoryChipText: {
    fontSize: 14,
    color: colors.textMuted,
  },
  categoryChipTextActive: {
    color: colors.surface,
  },
  content: {
    flex: 1,
    padding: 16,
  },
  faqItem: {
    backgroundColor: colors.surface,
    borderRadius: 12,
    marginBottom: 12,
    overflow: 'hidden',
  },
  faqQuestion: {
    flexDirection: 'row',
    alignItems: 'center',
    padding: 16,
  },
  faqQuestionText: {
    flex: 1,
    fontSize: 16,
    fontWeight: '500',
    color: colors.text,
  },
  expandIcon: {
    fontSize: 20,
    color: colors.textMuted,
    marginLeft: 12,
  },
  faqAnswer: {
    padding: 16,
    paddingTop: 0,
    borderTopWidth: 1,
    borderTopColor: colors.surfaceMuted,
  },
  faqAnswerText: {
    fontSize: 15,
    color: colors.textMuted,
    lineHeight: 22,
  },
  faqFeedback: {
    flexDirection: 'row',
    alignItems: 'center',
    marginTop: 16,
    paddingTop: 12,
    borderTopWidth: 1,
    borderTopColor: colors.surfaceMuted,
  },
  feedbackLabel: {
    fontSize: 14,
    color: colors.textMuted,
    marginRight: 12,
  },
  feedbackButtons: {
    flexDirection: 'row',
    gap: 8,
  },
  feedbackButton: {
    paddingHorizontal: 12,
    paddingVertical: 6,
    borderRadius: 6,
    backgroundColor: colors.surfaceMuted,
  },
  feedbackActive: {
    backgroundColor: colors.accentSoft,
  },
  feedbackButtonText: {
    fontSize: 14,
    color: colors.textMuted,
  },
  feedbackActiveText: {
    color: colors.accent,
  },
  tutorialItem: {
    flexDirection: 'row',
    backgroundColor: colors.surface,
    borderRadius: 12,
    marginBottom: 12,
    overflow: 'hidden',
  },
  tutorialThumbnail: {
    width: 100,
    height: 80,
    backgroundColor: colors.text,
    alignItems: 'center',
    justifyContent: 'center',
  },
  playIcon: {
    fontSize: 24,
    color: colors.surface,
  },
  tutorialInfo: {
    flex: 1,
    padding: 12,
  },
  tutorialTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 4,
  },
  tutorialDescription: {
    fontSize: 14,
    color: colors.textMuted,
    marginBottom: 8,
  },
  tutorialDuration: {
    fontSize: 12,
    color: colors.textSubtle,
  },
  emptyState: {
    alignItems: 'center',
    padding: 40,
  },
  emptyIcon: {
    fontSize: 48,
    marginBottom: 16,
  },
  emptyTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 8,
  },
  emptyText: {
    fontSize: 14,
    color: colors.textMuted,
    textAlign: 'center',
  },
  feedbackFab: {
    position: 'absolute',
    bottom: 24,
    right: 24,
    backgroundColor: colors.accent,
    paddingHorizontal: 20,
    paddingVertical: 14,
    borderRadius: 28,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.25,
    shadowRadius: 4,
    elevation: 5,
  },
  feedbackFabText: {
    color: colors.surface,
    fontSize: 16,
    fontWeight: '600',
  },
});
