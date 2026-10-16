/**
 * ArticleDetailScreen (UC-13).
 *
 * Full view for a single news article. Reached from `NewsScreen` when a card
 * is tapped — `onNavigate('ArticleDetail', { articleId })`. Before this screen
 * existed the tap fell through the `App.tsx` switch default and bounced the
 * user back to the Dashboard with the `articleId` param discarded.
 *
 * Loads `GET /api/v1/news/{id}` for the article body. The id is optional at
 * the prop boundary (App hands it from untyped navigation params); a missing
 * id renders a back-only empty state rather than firing a `/news/undefined`
 * request — the query is gated on `enabled: !!articleId`.
 */

import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
  Image,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { useApiQuery } from '../../hooks/useApi';
import { formatDate } from '../../i18n/format';
import { colors, screenStyles as s } from '../shared/screenStyles';

interface ApiNewsArticleDetail {
  id: string;
  title: string;
  summary?: string | null;
  excerpt?: string | null;
  body?: string | null;
  category?: string | null;
  published_at?: string | null;
  created_at: string;
  image_url?: string | null;
}

export const articleDetailKey = (id: string) => ['news', 'detail', id] as const;

interface ArticleDetailScreenProps {
  articleId?: string;
  onBack?: () => void;
}

export function ArticleDetailScreen({ articleId, onBack }: ArticleDetailScreenProps) {
  const { t } = useTranslation();

  const { data, isLoading, error } = useApiQuery<ApiNewsArticleDetail>(
    articleDetailKey(articleId ?? ''),
    `/api/v1/news/${articleId}`,
    { staleTime: 60_000, enabled: !!articleId }
  );

  const back = (
    <Pressable style={styles.backButton} onPress={onBack} disabled={!onBack}>
      <Text style={styles.backButtonText}>‹ News</Text>
    </Pressable>
  );

  // `articleId` is missing only if navigation was wired up wrong — render a
  // back-only empty state instead of fetching `/news/undefined`.
  if (!articleId) {
    return (
      <View style={s.container}>
        {back}
        <View style={s.emptyState}>
          <Text style={s.emptyTitle}>{t('news.emptyTitle')}</Text>
        </View>
      </View>
    );
  }

  if (isLoading) {
    return (
      <View style={s.container}>
        {back}
        <View style={s.emptyState}>
          <ActivityIndicator size="large" color={colors.accent} />
          <Text style={s.emptyText}>{t('common.loading')}</Text>
        </View>
      </View>
    );
  }

  if (error || !data) {
    return (
      <View style={s.container}>
        {back}
        <View style={s.emptyState}>
          <Text style={s.emptyIcon}>⚠️</Text>
          <Text style={s.emptyTitle}>{t('news.loadError')}</Text>
        </View>
      </View>
    );
  }

  const body = data.body ?? data.excerpt ?? data.summary ?? '';

  return (
    <View style={s.container}>
      {back}
      <ScrollView style={s.scrollView}>
        <View style={s.card}>
          {data.image_url ? <Image source={{ uri: data.image_url }} style={styles.cover} /> : null}
          <View style={s.cardHeader}>
            <Text style={s.cardTitle} numberOfLines={3}>
              {data.title}
            </Text>
            {data.category ? (
              <View style={s.badge}>
                <Text style={s.badgeText}>{data.category}</Text>
              </View>
            ) : null}
          </View>
          <Text style={[s.cardMeta, { marginBottom: 8 }]}>
            {formatDate(data.published_at ?? data.created_at)}
          </Text>
          {body ? <Text style={s.cardBody}>{body}</Text> : null}
        </View>
        <View style={s.bottomSpacer} />
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  backButton: {
    paddingHorizontal: 20,
    paddingTop: 56,
    paddingBottom: 12,
    backgroundColor: colors.surface,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  backButtonText: {
    fontSize: 15,
    color: colors.accent,
    fontWeight: '500',
  },
  cover: { width: '100%', height: 160, borderRadius: 8, marginBottom: 12 },
});
