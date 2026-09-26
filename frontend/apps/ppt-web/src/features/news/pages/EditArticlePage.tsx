/**
 * Edit Article Page - Edit existing news articles.
 * Epic 59: News & Media Management
 */

import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router-dom';
import { useOrganization } from '../../../hooks';
import { getApiClient } from '../../../lib/api';
import type { ArticleStatus, NewsArticle, UpdateArticleRequest } from '../types';

export function EditArticlePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { id } = useParams<{ id: string }>();
  const { organizationId } = useOrganization();

  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [excerpt, setExcerpt] = useState('');
  const [coverImageUrl, setCoverImageUrl] = useState('');
  const [status, setStatus] = useState<ArticleStatus>('draft');
  const [commentsEnabled, setCommentsEnabled] = useState(true);
  const [reactionsEnabled, setReactionsEnabled] = useState(true);
  const [isLoading, setIsLoading] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load article data
  useEffect(() => {
    const loadArticle = async () => {
      if (!organizationId || !id) return;

      setIsLoading(true);
      setError(null);

      try {
        // Authenticated read via the shared axios client (Bearer token from the
        // request interceptor); a raw `fetch()` here 401'd in production (#2982).
        const response = await getApiClient().get(`/news/${id}`, {
          params: { organization_id: organizationId },
        });

        const data = response.data;
        const article: NewsArticle = {
          id: data.id,
          organizationId: data.organization_id,
          authorId: data.author_id,
          title: data.title,
          content: data.content,
          excerpt: data.excerpt,
          coverImageUrl: data.cover_image_url,
          buildingIds: data.building_ids || [],
          status: data.status,
          publishedAt: data.published_at,
          archivedAt: data.archived_at,
          pinned: data.pinned,
          pinnedAt: data.pinned_at,
          pinnedBy: data.pinned_by,
          commentsEnabled: data.comments_enabled,
          reactionsEnabled: data.reactions_enabled,
          viewCount: data.view_count,
          reactionCount: data.reaction_count,
          commentCount: data.comment_count,
          shareCount: data.share_count,
          createdAt: data.created_at,
          updatedAt: data.updated_at,
        };

        setTitle(article.title);
        setContent(article.content);
        setExcerpt(article.excerpt || '');
        setCoverImageUrl(article.coverImageUrl || '');
        setStatus(article.status);
        setCommentsEnabled(article.commentsEnabled);
        setReactionsEnabled(article.reactionsEnabled);
      } catch (err) {
        setError(err instanceof Error ? err.message : t('news.errors.loadFailed'));
      } finally {
        setIsLoading(false);
      }
    };

    loadArticle();
  }, [organizationId, id, t]);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();

      if (!organizationId || !id) return;
      if (!title.trim()) {
        setError(t('news.errors.titleRequired'));
        return;
      }
      if (!content.trim()) {
        setError(t('news.errors.contentRequired'));
        return;
      }

      setIsSubmitting(true);
      setError(null);

      try {
        const payload: UpdateArticleRequest = {
          title: title.trim(),
          content: content.trim(),
          excerpt: excerpt.trim() || undefined,
          coverImageUrl: coverImageUrl.trim() || undefined,
          status,
          commentsEnabled,
          reactionsEnabled,
        };

        await getApiClient().put(`/news/${id}`, payload, {
          params: { organization_id: organizationId },
        });

        navigate(`/news/${id}`);
      } catch (err) {
        setError(err instanceof Error ? err.message : t('news.errors.updateFailed'));
      } finally {
        setIsSubmitting(false);
      }
    },
    [
      organizationId,
      id,
      title,
      content,
      excerpt,
      coverImageUrl,
      status,
      commentsEnabled,
      reactionsEnabled,
      navigate,
      t,
    ]
  );

  const handleCancel = () => {
    navigate(`/news/${id}`);
  };

  if (!organizationId) {
    return <div className="news-page-error">{t('common.selectOrganization')}</div>;
  }

  if (isLoading) {
    return <div className="news-loading">{t('common.loading')}</div>;
  }

  return (
    <div className="edit-article-page">
      <header className="article-form-header">
        <h1>{t('news.editArticle')}</h1>
      </header>

      {error && (
        <div className="article-form-error" role="alert">
          {error}
          <button type="button" onClick={() => setError(null)}>
            {t('common.dismiss')}
          </button>
        </div>
      )}

      <form onSubmit={handleSubmit} className="article-form">
        <div className="form-group">
          <label htmlFor="title">{t('news.title')} *</label>
          <input
            type="text"
            id="title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={t('news.titlePlaceholder')}
            required
            disabled={isSubmitting}
          />
        </div>

        <div className="form-group">
          <label htmlFor="excerpt">{t('news.excerpt')}</label>
          <textarea
            id="excerpt"
            value={excerpt}
            onChange={(e) => setExcerpt(e.target.value)}
            placeholder={t('news.excerptPlaceholder')}
            rows={2}
            disabled={isSubmitting}
          />
        </div>

        <div className="form-group">
          <label htmlFor="content">{t('news.content')} *</label>
          <textarea
            id="content"
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder={t('news.contentPlaceholder')}
            rows={10}
            required
            disabled={isSubmitting}
          />
        </div>

        <div className="form-group">
          <label htmlFor="coverImageUrl">{t('news.coverImage')}</label>
          <input
            type="url"
            id="coverImageUrl"
            value={coverImageUrl}
            onChange={(e) => setCoverImageUrl(e.target.value)}
            placeholder={t('news.coverImagePlaceholder')}
            disabled={isSubmitting}
          />
        </div>

        <div className="form-group">
          <label htmlFor="status">{t('news.status')}</label>
          <select
            id="status"
            value={status}
            onChange={(e) => setStatus(e.target.value as ArticleStatus)}
            disabled={isSubmitting}
          >
            <option value="draft">{t('news.statusDraft')}</option>
            <option value="published">{t('news.statusPublished')}</option>
            <option value="archived">{t('news.statusArchived')}</option>
          </select>
        </div>

        <div className="form-group form-group-inline">
          <label>
            <input
              type="checkbox"
              checked={commentsEnabled}
              onChange={(e) => setCommentsEnabled(e.target.checked)}
              disabled={isSubmitting}
            />
            {t('news.commentsEnabled')}
          </label>
        </div>

        <div className="form-group form-group-inline">
          <label>
            <input
              type="checkbox"
              checked={reactionsEnabled}
              onChange={(e) => setReactionsEnabled(e.target.checked)}
              disabled={isSubmitting}
            />
            {t('news.reactionsEnabled')}
          </label>
        </div>

        <div className="form-actions">
          <button type="button" onClick={handleCancel} disabled={isSubmitting}>
            {t('common.cancel')}
          </button>
          <button type="submit" className="btn-primary" disabled={isSubmitting}>
            {isSubmitting ? t('common.saving') : t('common.save')}
          </button>
        </div>
      </form>
    </div>
  );
}

export default EditArticlePage;
