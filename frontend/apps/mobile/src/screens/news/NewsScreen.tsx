/**
 * NewsScreen (UC-13).
 *
 * Reads news/articles relevant to the resident: building news, district
 * announcements, partner promotions.
 */

import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Image, Pressable, RefreshControl, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { formatDate } from '../../i18n/format';
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

interface ApiNewsArticle {
  id: string;
  title: string;
  summary?: string | null;
  excerpt?: string | null;
  body?: string | null;
  category?: string | null;
  published_at?: string | null;
  created_at: string;
  image_url?: string | null;
  read?: boolean;
}

interface ApiNewsListResponse {
  articles: ApiNewsArticle[];
  total?: number;
}

function toUiArticle(a: ApiNewsArticle): NewsArticle {
  const cat = (a.category ?? 'building') as NewsArticle['category'];
  return {
    id: a.id,
    title: a.title,
    excerpt: a.excerpt ?? a.summary ?? a.body?.slice(0, 200) ?? '',
    category: cat === 'building' || cat === 'district' || cat === 'partner' ? cat : 'building',
    publishedAt: a.published_at ?? a.created_at,
    imageUrl: a.image_url ?? undefined,
    read: a.read ?? false,
  };
}

interface NewsScreenProps {
  onNavigate?: (screen: string, params?: Record<string, unknown>) => void;
}

export function NewsScreen({ onNavigate }: NewsScreenProps) {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<'all' | NewsArticle['category']>('all');

  const { data, isLoading, error, refetch, isFetching } = useApiQuery<ApiNewsListResponse>(
    ['news', 'list'],
    '/api/v1/news',
    { staleTime: 60_000 }
  );

  const articles: NewsArticle[] = (data?.articles ?? []).map(toUiArticle);

  const onRefresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  const filtered = useMemo(
    () => (filter === 'all' ? articles : articles.filter((a) => a.category === filter)),
    [articles, filter]
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
        refreshControl={<RefreshControl refreshing={isFetching} onRefresh={onRefresh} />}
      >
        {isLoading ? (
          <View style={s.emptyState}>
            <Text style={s.emptyTitle}>{t('common.loading')}</Text>
          </View>
        ) : error ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>⚠️</Text>
            <Text style={s.emptyTitle}>{t('news.loadError')}</Text>
          </View>
        ) : filtered.length === 0 ? (
          <View style={s.emptyState}>
            <Text style={s.emptyIcon}>📰</Text>
            <Text style={s.emptyTitle}>{t('news.emptyTitle')}</Text>
            <Text style={s.emptyText}>{t('news.emptyText')}</Text>
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
              <Text style={[s.cardMeta, { marginTop: 8 }]}>{formatDate(article.publishedAt)}</Text>
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
