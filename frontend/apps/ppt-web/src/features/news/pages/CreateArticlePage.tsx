/**
 * Create Article Page - Create new news articles.
 * Epic 59: News & Media Management
 */

import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useOrganization } from '../../../hooks';
import { getApiClient } from '../../../lib/api';
import type { ArticleStatus, CreateArticleRequest } from '../types';

export function CreateArticlePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { organizationId } = useOrganization();

  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [excerpt, setExcerpt] = useState('');
  const [coverImageUrl, setCoverImageUrl] = useState('');
  const [status, setStatus] = useState<ArticleStatus>('draft');
  const [commentsEnabled, setCommentsEnabled] = useState(true);
  const [reactionsEnabled, setReactionsEnabled] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();

      if (!organizationId) return;
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
        const payload: CreateArticleRequest = {
          title: title.trim(),
          content: content.trim(),
          excerpt: excerpt.trim() || undefined,
          coverImageUrl: coverImageUrl.trim() || undefined,
          status,
          commentsEnabled,
          reactionsEnabled,
        };

        // Route through the shared axios client so the request is authenticated
        // (Bearer token via the request interceptor). A raw `fetch()` here sent
        // no Authorization header and 401'd behind <ProtectedRoute> (#2982).
        const response = await getApiClient().post<{ id: string }>('/news', payload, {
          params: { organization_id: organizationId },
        });

        navigate(`/news/${response.data.id}`);
      } catch (err) {
        setError(err instanceof Error ? err.message : t('news.errors.createFailed'));
      } finally {
        setIsSubmitting(false);
      }
    },
    [
      organizationId,
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
    navigate('/news');
  };

  if (!organizationId) {
    return <div className="news-page-error">{t('common.selectOrganization')}</div>;
  }

  return (
    <div className="create-article-page">
      <header className="article-form-header">
        <h1>{t('news.createArticle')}</h1>
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

export default CreateArticlePage;
