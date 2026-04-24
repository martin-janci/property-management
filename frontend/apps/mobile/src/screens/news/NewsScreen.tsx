/**
 * NewsScreen (UC-13).
 *
 * Reads news/articles relevant to the resident: building news, district
 * announcements, partner promotions.
 */

import { useCallback, useMemo, useState } from 'react';
import { Image, Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { colors, screenStyles as s } from '../shared/screenStyles';

export interface NewsArticle {
  id: string;
  title: string;
  excerpt: string;
  category: 'building' | 'district' | 'partner';
  publishedAt: string;
  imageUrl?: string;
  read: boolean;
}

const MOCK_ARTICLES: NewsArticle[] = [
  {
    id: 'a1',
    title: 'Renovation works begin on the south façade',
    excerpt:
      'Scaffolding will be installed on April 28. Expect noise on weekdays between 8am and 5pm.',
    category: 'building',
    publishedAt: '2026-04-23T07:30:00Z',
    read: false,
  },
  {
    id: 'a2',
    title: 'Spring street fair this weekend',
    excerpt: 'The annual spring fair returns to our street on Saturday with live music and food.',
    category: 'district',
    publishedAt: '2026-04-22T15:00:00Z',
    read: false,
  },
  {
    id: 'a3',
    title: 'Discount on smart thermostats for residents',
    excerpt: 'Our partner offers 20% off smart thermostats with installation through May.',
    category: 'partner',
    publishedAt: '2026-04-19T10:00:00Z',
    read: true,
  },
];

interface NewsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function NewsScreen({ onNavigate }: NewsScreenProps) {
  const [refreshing, setRefreshing] = useState(false);
  const [filter, setFilter] = useState<'all' | NewsArticle['category']>('all');

  const onRefresh = useCallback(async () => {
    setRefreshing(true);
    await new Promise((r) => setTimeout(r, 500));
    setRefreshing(false);
  }, []);

  const filtered = useMemo(
    () => (filter === 'all' ? MOCK_ARTICLES : MOCK_ARTICLES.filter((a) => a.category === filter)),
    [filter]
  );

  const filters: ReadonlyArray<{ value: 'all' | NewsArticle['category']; label: string }> = [
    { value: 'all', label: 'All' },
    { value: 'building', label: 'Building' },
    { value: 'district', label: 'District' },
    { value: 'partner', label: 'Partners' },
  ];

  return (
    <View style={s.container}>
      <View style={s.header}>
        <Text style={s.headerTitle}>News</Text>
        <Text style={s.headerSubtitle}>Updates from your building and neighbourhood.</Text>
      </View>

      <View style={s.filtersWrapper}>
        <ScrollView horizontal showsHorizontalScrollIndicator={false}>
          <View style={s.filtersRow}>
            {filters.map((option) => (
              <Pressable
                key={option.value}
                style={[s.filterChip, filter === option.value && s.filterChipActive]}
                onPress={() => setFilter(option.value)}
              >
                <Text style={[s.filterText, filter === option.value && s.filterTextActive]}>
                  {option.label}
                </Text>
              </Pressable>
            ))}
          </View>
        </ScrollView>
      </View>

      <ScrollView
        style={s.scrollView}
        refreshControl={<RefreshControl refreshing={refreshing} onRefresh={onRefresh} />}
      >
        {filtered.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>📰</Text>
            <Text style={s.emptyTitle}>Nothing new</Text>
            <Text style={s.emptyText}>Check back later for fresh updates.</Text>
          </View>
        ) : (
          filtered.map((article) => (
            <Pressable
              key={article.id}
              style={[s.card, !article.read && styles.unreadCard]}
              onPress={() => onNavigate?.('ArticleDetail', { articleId: article.id })}
            >
              {article.imageUrl && (
                <Image source={{ uri: article.imageUrl }} style={styles.cover} />
              )}
              <View style={s.cardHeader}>
                <Text style={s.cardTitle} numberOfLines={2}>
                  {article.title}
                </Text>
                <View style={s.badge}>
                  <Text style={s.badgeText}>{article.category}</Text>
                </View>
              </View>
              <Text style={s.cardBody} numberOfLines={3}>
                {article.excerpt}
              </Text>
              <Text style={[s.cardMeta, { marginTop: 8 }]}>
                {new Date(article.publishedAt).toLocaleDateString()}
              </Text>
            </Pressable>
          ))
        )}
        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  unreadCard: { borderLeftWidth: 3, borderLeftColor: colors.accent },
  cover: { width: '100%', height: 140, borderRadius: 8, marginBottom: 8 },
});
