/**
 * News Article Card Component (Epic 59, Story 59.1)
 */

import { useTranslation } from 'react-i18next';
import type { ArticleStatus, ArticleSummary } from '../types';

interface NewsArticleCardProps {
  article: ArticleSummary;
  onView?: (id: string) => void;
  onEdit?: (id: string) => void;
  onDelete?: (id: string) => void;
  onPublish?: (id: string) => void;
  onArchive?: (id: string) => void;
  onPin?: (id: string, pinned: boolean) => void;
}

const statusColors: Record<ArticleStatus, string> = {
  draft: 'bg-gray-100 text-gray-800',
  published: 'bg-green-100 text-green-800',
  archived: 'bg-yellow-100 text-yellow-800',
};

export function NewsArticleCard({
  article,
  onView,
  onEdit,
  onDelete,
  onPublish,
  onArchive,
  onPin,
}: NewsArticleCardProps) {
  const { t } = useTranslation();
  const canEdit = article.status === 'draft';
  const canDelete = article.status === 'draft';
  const canPublish = article.status === 'draft';
  const canArchive = article.status === 'published';

  return (
    <article className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            {article.pinned && (
              <span className="text-amber-500" aria-label={t('aria.pinnedArticle')}>
                <svg
                  className="w-4 h-4"
                  fill="currentColor"
                  viewBox="0 0 20 20"
                  aria-hidden="true"
                  role="img"
                >
                  <title>Pinned</title>
                  <path d="M9.828.722a.5.5 0 01.354 0l7 3A.5.5 0 0117.5 4v1.5a.5.5 0 01-.5.5h-1v4.5a.5.5 0 01-.5.5H13v5.5a.5.5 0 01-.5.5h-5a.5.5 0 01-.5-.5V11H4.5a.5.5 0 01-.5-.5V6h-1a.5.5 0 01-.5-.5V4a.5.5 0 01.328-.472l7-3z" />
                </svg>
              </span>
            )}
            <h3 className="text-lg font-semibold text-gray-900">{article.title}</h3>
          </div>

          {article.excerpt && (
            <p className="mt-2 text-sm text-gray-600 line-clamp-2">{article.excerpt}</p>
          )}

          {article.coverImageUrl && (
            <img
              src={article.coverImageUrl}
              alt={`Cover for: ${article.title}`}
              className="mt-3 w-full h-48 object-cover rounded"
            />
          )}

          <div className="mt-3 flex items-center gap-3 flex-wrap">
            <span
              className={`px-2 py-1 text-xs font-medium rounded ${statusColors[article.status]}`}
              aria-label={`Status: ${article.status}`}
            >
              {article.status.charAt(0).toUpperCase() + article.status.slice(1)}
            </span>
            <span className="text-xs text-gray-500 flex items-center gap-1">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <title>Views</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                />
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"
                />
              </svg>
              {article.viewCount}
            </span>
            <span className="text-xs text-gray-500 flex items-center gap-1">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <title>Reactions</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z"
                />
              </svg>
              {article.reactionCount}
            </span>
            <span className="text-xs text-gray-500 flex items-center gap-1">
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <title>Comments</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
                />
              </svg>
              {article.commentCount}
            </span>
          </div>

          {article.publishedAt && (
            <p className="mt-2 text-xs text-gray-400">
              Published: {new Date(article.publishedAt).toLocaleDateString()}
            </p>
          )}
        </div>
      </div>

      <nav
        className="mt-4 flex items-center gap-2 border-t pt-3"
        aria-label={t('aria.articleActions')}
      >
        <button
          type="button"
          onClick={() => onView?.(article.id)}
          className="text-sm text-blue-600 hover:text-blue-800"
          aria-label={`View article: ${article.title}`}
        >
          View
        </button>
        {canEdit && onEdit && (
          <button
            type="button"
            onClick={() => onEdit(article.id)}
            className="text-sm text-gray-600 hover:text-gray-800"
            aria-label={`Edit article: ${article.title}`}
          >
            Edit
          </button>
        )}
        {canPublish && onPublish && (
          <button
            type="button"
            onClick={() => onPublish(article.id)}
            className="text-sm text-green-600 hover:text-green-800"
            aria-label={`Publish article: ${article.title}`}
          >
            Publish
          </button>
        )}
        {canArchive && onArchive && (
          <button
            type="button"
            onClick={() => onArchive(article.id)}
            className="text-sm text-yellow-600 hover:text-yellow-800"
            aria-label={`Archive article: ${article.title}`}
          >
            Archive
          </button>
        )}
        {canDelete && onDelete && (
          <button
            type="button"
            onClick={() => onDelete(article.id)}
            className="text-sm text-red-600 hover:text-red-800"
            aria-label={`Delete article: ${article.title}`}
          >
            Delete
          </button>
        )}
        {onPin && (
          <button
            type="button"
            onClick={() => onPin(article.id, !article.pinned)}
            className="text-sm text-amber-600 hover:text-amber-800"
            aria-label={article.pinned ? 'Unpin article' : 'Pin article'}
          >
            {article.pinned ? 'Unpin' : 'Pin'}
          </button>
        )}
      </nav>
    </article>
  );
}
