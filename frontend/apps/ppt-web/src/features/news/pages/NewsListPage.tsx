/**
 * News List Page - Displays all news articles with filtering and search.
 * Epic 59: News & Media Management
 */

import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link, useNavigate } from 'react-router-dom';
import { useOrganization } from '../../../hooks';
import { NewsArticleCard } from '../components';
import type { ArticleStatus, ArticleSummary } from '../types';

export function NewsListPage() {
  const { t } = useTranslation();
  const { organizationId } = useOrganization();
  const navigate = useNavigate();
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
      const params = new URLSearchParams({ organization_id: organizationId });
      if (statusFilter) params.append('status', statusFilter);
      if (showPinnedOnly) params.append('pinned_only', 'true');

      const response = await fetch(`/api/v1/news?${params}`);

      if (!response.ok) {
        throw new Error('Failed to load articles');
      }

      const data = await response.json();
      // Convert snake_case to camelCase
      setArticles(
        data.map((a: Record<string, unknown>) => ({
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
      setError(err instanceof Error ? err.message : 'Failed to load articles');
    } finally {
      setLoading(false);
    }
  }, [organizationId, statusFilter, showPinnedOnly]);

  useEffect(() => {
    loadArticles();
  }, [loadArticles]);

  const handleDelete = useCallback(
    async (id: string) => {
      if (!organizationId) return;
      if (!confirm('Are you sure you want to delete this article?')) return;

      try {
        const params = new URLSearchParams({ organization_id: organizationId });
        const response = await fetch(`/api/v1/news/${id}?${params}`, {
          method: 'DELETE',
        });

        if (!response.ok) {
          throw new Error('Failed to delete article');
        }

        setArticles((prev) => prev.filter((a) => a.id !== id));
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to delete article');
      }
    },
    [organizationId]
  );

  const handlePublish = useCallback(
    async (id: string) => {
      if (!organizationId) return;

      try {
        const response = await fetch(`/api/v1/news/${id}/publish`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ organization_id: organizationId }),
        });

        if (!response.ok) {
          throw new Error('Failed to publish article');
        }

        loadArticles();
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to publish article');
      }
    },
    [organizationId, loadArticles]
  );

  const handleArchive = useCallback(
    async (id: string) => {
      if (!organizationId) return;

      try {
        const response = await fetch(`/api/v1/news/${id}/archive`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ organization_id: organizationId }),
        });

        if (!response.ok) {
          throw new Error('Failed to archive article');
        }

        loadArticles();
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to archive article');
      }
    },
    [organizationId, loadArticles]
  );

  const handlePin = useCallback(
    async (id: string, pinned: boolean) => {
      if (!organizationId) return;

      try {
        const response = await fetch(`/api/v1/news/${id}/pin`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ organization_id: organizationId, pinned }),
        });

        if (!response.ok) {
          throw new Error('Failed to update pin status');
        }

        loadArticles();
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to update pin status');
      }
    },
    [organizationId, loadArticles]
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
    return <div className="news-page-error">Please select an organization</div>;
  }

  return (
    <div className="news-list-page">
      <header className="news-header">
        <h1>News & Announcements</h1>
        <Link to="/news/new" className="btn btn-primary">
          Create Article
        </Link>
      </header>

      {error && (
        <div className="news-error" role="alert">
          {error}
          <button type="button" onClick={() => setError(null)}>
            Dismiss
          </button>
        </div>
      )}

      <div className="news-filters">
        <input
          type="search"
          placeholder="Search articles..."
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
          <option value="">All Statuses</option>
          <option value="draft">Drafts</option>
          <option value="published">Published</option>
          <option value="archived">Archived</option>
        </select>

        <label className="pinned-filter">
          <input
            type="checkbox"
            checked={showPinnedOnly}
            onChange={(e) => setShowPinnedOnly(e.target.checked)}
          />
          Pinned only
        </label>
      </div>

      {loading ? (
        <div className="news-loading">Loading articles...</div>
      ) : sortedArticles.length === 0 ? (
        <div className="news-empty">
          {searchQuery || statusFilter || showPinnedOnly
            ? 'No articles match your filters.'
            : 'No articles yet. Create your first article!'}
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
              onEdit={(id) => {
                navigate(`/news/${id}/edit`);
              }}
              onPublish={handlePublish}
              onArchive={handleArchive}
              onDelete={handleDelete}
              onPin={handlePin}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export default NewsListPage;
