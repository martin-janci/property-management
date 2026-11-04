/**
 * News List Page - Displays all news articles with filtering and search.
 * Epic 59: News & Media Management
 */

import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../../../contexts';
import { useOrganization } from '../../../hooks';
import { getApiClient } from '../../../lib/api';
import { isManagerRole } from '../../../routes/shared';
import { NewsArticleCard } from '../components';
import type { ArticleStatus, ArticleSummary } from '../types';

export function NewsListPage() {
  const { t } = useTranslation();
  const { organizationId } = useOrganization();
  const navigate = useNavigate();
  const { user } = useAuth();
  const isManager = isManagerRole(user?.role);
  const [articles, setArticles] = useState<ArticleSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Filters
  const [statusFilter, setStatusFilter] = useState<ArticleStatus | ''>('');
  const [searchQuery, setSearchQuery] = useState('');
  const [showPinnedOnly, setShowPinnedOnly] = useState(false);

  const loadArticles = useCallback(async () => {
    if (!organizationId) return;

    setLoading(true);
    setError(null);

    try {
      // Route through the shared axios client (`getApiClient()`) so the request
      // carries `Authorization: Bearer <token>` via the request interceptor.
      // A raw `fetch('/api/v1/news')` here sent NO auth header and 401'd behind
      // <ProtectedRoute> in production (#2982). Paths are relative to the
      // client baseURL (`/api/v1`).
      const params: Record<string, string> = { organization_id: organizationId };
      if (statusFilter) params.status = statusFilter;
      if (showPinnedOnly) params.pinned_only = 'true';

      const response = await getApiClient().get('/news', { params });

      // The API returns `{ articles, count, total }`; tolerate a bare array too.
      const data = response.data;
      const items = Array.isArray(data) ? data : (data?.articles ?? []);
      // Convert snake_case to camelCase
      setArticles(
        items.map((a: Record<string, unknown>) => ({
          id: a.id,
          title: a.title,
          excerpt: a.excerpt,
          coverImageUrl: a.cover_image_url,
          authorId: a.author_id,
          status: a.status,
          publishedAt: a.published_at,
          pinned: a.pinned,
          viewCount: a.view_count,
          reactionCount: a.reaction_count,
          commentCount: a.comment_count,
          createdAt: a.created_at,
        }))
      );
    } catch (err) {
      setError(err instanceof Error ? err.message : t('news.errors.loadListFailed'));
    } finally {
      setLoading(false);
    }
  }, [organizationId, statusFilter, showPinnedOnly, t]);

  useEffect(() => {
    loadArticles();
  }, [loadArticles]);

  const handleDelete = useCallback(
    async (id: string) => {
      if (!organizationId) return;
      if (!confirm(t('news.confirmDelete'))) return;

      try {
        await getApiClient().delete(`/news/${id}`, {
          params: { organization_id: organizationId },
        });

        setArticles((prev) => prev.filter((a) => a.id !== id));
      } catch (err) {
        setError(err instanceof Error ? err.message : t('news.errors.deleteFailed'));
      }
    },
    [organizationId, t]
  );

  const handlePublish = useCallback(
    async (id: string) => {
      if (!organizationId) return;

      try {
        await getApiClient().post(`/news/${id}/publish`, {
          organization_id: organizationId,
        });

        loadArticles();
      } catch (err) {
        setError(err instanceof Error ? err.message : t('news.errors.publishFailed'));
      }
    },
    [organizationId, loadArticles, t]
  );

  const handleArchive = useCallback(
    async (id: string) => {
      if (!organizationId) return;

      try {
        await getApiClient().post(`/news/${id}/archive`, {
          organization_id: organizationId,
        });

        loadArticles();
      } catch (err) {
        setError(err instanceof Error ? err.message : t('news.errors.archiveFailed'));
      }
    },
    [organizationId, loadArticles, t]
  );

  const handlePin = useCallback(
    async (id: string, pinned: boolean) => {
      if (!organizationId) return;

      try {
        await getApiClient().post(`/news/${id}/pin`, {
          organization_id: organizationId,
          pinned,
        });

        loadArticles();
      } catch (err) {
        setError(err instanceof Error ? err.message : t('news.errors.pinFailed'));
      }
    },
    [organizationId, loadArticles, t]
  );

  // Filter articles by search query
  const filteredArticles = articles.filter((article) => {
    if (!searchQuery) return true;
    const query = searchQuery.toLowerCase();
    return (
      article.title.toLowerCase().includes(query) ||
      (article.excerpt?.toLowerCase().includes(query) ?? false)
    );
  });

  // Sort: pinned first, then by date
  const sortedArticles = [...filteredArticles].sort((a, b) => {
    if (a.pinned && !b.pinned) return -1;
    if (!a.pinned && b.pinned) return 1;
    return new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime();
  });

  if (!organizationId) {
    return <div className="news-page-error">{t('common.selectOrganization')}</div>;
  }

  return (
    <div className="news-list-page">
      <header className="news-header">
        <h1>{t('news.listTitle')}</h1>
        {isManager && (
          <Link to="/news/new" className="btn btn-primary">
            {t('news.createArticle')}
          </Link>
        )}
      </header>

      {error && (
        <div className="news-error" role="alert">
          {error}
          <button type="button" onClick={() => setError(null)}>
            {t('common.dismiss')}
          </button>
        </div>
      )}

      <div className="news-filters">
        <input
          type="search"
          placeholder={t('news.searchPlaceholder')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="search-input"
          aria-label={t('aria.searchArticles')}
        />

        <select
          value={statusFilter}
          onChange={(e) => setStatusFilter(e.target.value as ArticleStatus | '')}
          className="status-filter"
          aria-label={t('aria.filterByStatus')}
        >
          <option value="">{t('news.filters.allStatuses')}</option>
          <option value="draft">{t('news.filters.drafts')}</option>
          <option value="published">{t('news.statusPublished')}</option>
          <option value="archived">{t('news.statusArchived')}</option>
        </select>

        <label className="pinned-filter">
          <input
            type="checkbox"
            checked={showPinnedOnly}
            onChange={(e) => setShowPinnedOnly(e.target.checked)}
          />
          {t('news.filters.pinnedOnly')}
        </label>
      </div>

      {loading ? (
        <div className="news-loading">{t('news.loadingArticles')}</div>
      ) : sortedArticles.length === 0 ? (
        <div className="news-empty">
          {searchQuery || statusFilter || showPinnedOnly
            ? t('news.emptyFiltered')
            : t('news.empty')}
        </div>
      ) : (
        <div className="news-grid">
          {sortedArticles.map((article) => (
            <NewsArticleCard
              key={article.id}
              article={article}
              onView={(id) => {
                navigate(`/news/${id}`);
              }}
              // Writing news is manager-only on the server (403 otherwise),
              // so residents get read-only cards.
              onEdit={
                isManager
                  ? (id) => {
                      navigate(`/news/${id}/edit`);
                    }
                  : undefined
              }
              onPublish={isManager ? handlePublish : undefined}
              onArchive={isManager ? handleArchive : undefined}
              onDelete={isManager ? handleDelete : undefined}
              onPin={isManager ? handlePin : undefined}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export default NewsListPage;
