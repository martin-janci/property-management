/**
 * Article Detail Page - Displays a single article with comments and reactions.
 * Epic 59: News & Media Management
 */

import DOMPurify from 'dompurify';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { useOrganization } from '../../../hooks';
import { getApiClient } from '../../../lib/api';
import { ArticleComments, ArticleReactions } from '../components';
import type { ArticleCommentWithAuthor, NewsArticle, ReactionCounts, ReactionType } from '../types';

interface ArticleDetailPageProps {
  articleId: string;
}

interface ArticleWithAuthor extends NewsArticle {
  authorName: string;
  authorAvatarUrl: string | null;
}

export function ArticleDetailPage({ articleId }: ArticleDetailPageProps) {
  const { t } = useTranslation();
  const { organizationId } = useOrganization();
  const [article, setArticle] = useState<ArticleWithAuthor | null>(null);
  const [reactionCounts, setReactionCounts] = useState<ReactionCounts | null>(null);
  const [comments, setComments] = useState<ArticleCommentWithAuthor[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [userReaction, setUserReaction] = useState<ReactionType | null>(null);

  const loadArticle = useCallback(async () => {
    if (!organizationId || !articleId) return;

    setLoading(true);
    setError(null);

    try {
      // Route through the shared axios client so every request is authenticated
      // (Bearer token via the request interceptor). Raw `fetch()` here sent no
      // Authorization header and 401'd behind <ProtectedRoute> (#2982). Article
      // load must succeed; reactions/comments stay best-effort, so we use
      // `allSettled` and only surface a failure for the article itself.
      const params = { organization_id: organizationId };
      const client = getApiClient();

      const [articleRes, reactionsRes, commentsRes] = await Promise.allSettled([
        client.get(`/news/${articleId}`, { params }),
        client.get(`/news/${articleId}/reactions/counts`, { params }),
        client.get(`/news/${articleId}/comments`, { params }),
      ]);

      if (articleRes.status === 'rejected') {
        const status = (articleRes.reason as { status?: number } | undefined)?.status;
        throw new Error(status === 404 ? 'Article not found' : 'Failed to load article');
      }

      const articleData = articleRes.value.data;

      // Convert snake_case to camelCase
      setArticle({
        id: articleData.id,
        organizationId: articleData.organization_id,
        authorId: articleData.author_id,
        title: articleData.title,
        content: articleData.content,
        excerpt: articleData.excerpt,
        coverImageUrl: articleData.cover_image_url,
        buildingIds: articleData.building_ids || [],
        status: articleData.status,
        publishedAt: articleData.published_at,
        archivedAt: articleData.archived_at,
        pinned: articleData.pinned,
        pinnedAt: articleData.pinned_at,
        pinnedBy: articleData.pinned_by,
        commentsEnabled: articleData.comments_enabled,
        reactionsEnabled: articleData.reactions_enabled,
        viewCount: articleData.view_count,
        reactionCount: articleData.reaction_count,
        commentCount: articleData.comment_count,
        shareCount: articleData.share_count || 0,
        createdAt: articleData.created_at,
        updatedAt: articleData.updated_at,
        authorName: articleData.author_name,
        authorAvatarUrl: articleData.author_avatar_url,
      });

      if (reactionsRes.status === 'fulfilled') {
        setReactionCounts(reactionsRes.value.data);
      }

      if (commentsRes.status === 'fulfilled') {
        const commentsData = commentsRes.value.data;
        setComments(
          commentsData.map((c: Record<string, unknown>) => ({
            id: c.id,
            articleId: c.article_id,
            userId: c.user_id,
            parentId: c.parent_id,
            content: c.content,
            isModerated: c.is_moderated,
            moderatedAt: c.moderated_at,
            moderatedBy: c.moderated_by,
            moderationReason: c.moderation_reason,
            deletedAt: c.deleted_at,
            deletedBy: c.deleted_by,
            likeCount: c.like_count,
            createdAt: c.created_at,
            updatedAt: c.updated_at,
            authorName: c.author_name,
            authorAvatarUrl: c.author_avatar_url,
            replyCount: c.reply_count || 0,
          }))
        );
      }

      // Record view (fire and forget)
      client.post(`/news/${articleId}/view`, { organization_id: organizationId }).catch(() => {
        // Silently ignore view tracking errors
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load article');
    } finally {
      setLoading(false);
    }
  }, [organizationId, articleId]);

  useEffect(() => {
    loadArticle();
  }, [loadArticle]);

  const handleReaction = useCallback(
    async (reactionType: ReactionType) => {
      if (!organizationId || !articleId) return;

      try {
        const response = await getApiClient().post(`/news/${articleId}/reactions`, {
          organization_id: organizationId,
          reaction_type: reactionType,
        });

        const result = response.data;
        setUserReaction(result.added ? reactionType : null);

        // Reload reaction counts
        const countsRes = await getApiClient().get(`/news/${articleId}/reactions/counts`, {
          params: { organization_id: organizationId },
        });
        setReactionCounts(countsRes.data);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to update reaction');
      }
    },
    [organizationId, articleId]
  );

  const handleAddComment = useCallback(
    async (content: string, parentId?: string) => {
      if (!organizationId || !articleId) return;

      try {
        const response = await getApiClient().post(`/news/${articleId}/comments`, {
          organization_id: organizationId,
          content,
          parent_id: parentId,
        });

        const newCommentData = response.data;
        const newComment: ArticleCommentWithAuthor = {
          id: newCommentData.id,
          articleId: newCommentData.article_id,
          userId: newCommentData.user_id,
          parentId: newCommentData.parent_id,
          content: newCommentData.content,
          isModerated: newCommentData.is_moderated,
          moderatedAt: newCommentData.moderated_at,
          moderatedBy: newCommentData.moderated_by,
          moderationReason: newCommentData.moderation_reason,
          deletedAt: newCommentData.deleted_at,
          deletedBy: newCommentData.deleted_by,
          likeCount: newCommentData.like_count,
          createdAt: newCommentData.created_at,
          updatedAt: newCommentData.updated_at,
          authorName: newCommentData.author_name,
          authorAvatarUrl: newCommentData.author_avatar_url,
          replyCount: 0,
        };
        setComments((prev) => [...prev, newComment]);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to post comment');
      }
    },
    [organizationId, articleId]
  );

  const handleEditComment = useCallback(
    async (commentId: string, content: string) => {
      if (!organizationId || !articleId) return;

      try {
        const response = await getApiClient().put(`/news/${articleId}/comments/${commentId}`, {
          organization_id: organizationId,
          content,
        });

        const updatedData = response.data;
        setComments((prev) =>
          prev.map((c) =>
            c.id === commentId
              ? {
                  ...c,
                  content: updatedData.content,
                  updatedAt: updatedData.updated_at,
                }
              : c
          )
        );
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to update comment');
      }
    },
    [organizationId, articleId]
  );

  const handleDeleteComment = useCallback(
    async (commentId: string) => {
      if (!organizationId || !articleId) return;

      try {
        await getApiClient().delete(`/news/${articleId}/comments/${commentId}`, {
          params: { organization_id: organizationId },
        });

        setComments((prev) => prev.filter((c) => c.id !== commentId));
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to delete comment');
      }
    },
    [organizationId, articleId]
  );

  if (!organizationId) {
    return <div className="article-error">Please select an organization</div>;
  }

  if (loading) {
    return <div className="article-loading">Loading article...</div>;
  }

  if (error) {
    return (
      <div className="article-error" role="alert">
        {error}
        <button type="button" onClick={loadArticle}>
          Retry
        </button>
      </div>
    );
  }

  if (!article) {
    return <div className="article-not-found">Article not found</div>;
  }

  return (
    <article className="article-detail-page">
      <header className="article-header">
        <Link to="/news" className="back-link">
          &larr; Back to News
        </Link>

        {article.pinned && <span className="pinned-badge">Pinned</span>}

        <h1>{article.title}</h1>

        <div className="article-meta">
          <span className="author">
            {article.authorAvatarUrl && (
              <img
                src={article.authorAvatarUrl}
                alt={article.authorName}
                className="author-avatar"
              />
            )}
            {article.authorName}
          </span>
          {article.publishedAt && (
            <time dateTime={article.publishedAt}>
              {new Date(article.publishedAt).toLocaleDateString()}
            </time>
          )}
          <span className="views">{article.viewCount} views</span>
        </div>
      </header>

      {article.coverImageUrl && (
        <img src={article.coverImageUrl} alt={article.title} className="article-cover-image" />
      )}

      <div
        className="article-content"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: Content is sanitized with an explicit allowlist (defense-in-depth)
        dangerouslySetInnerHTML={{
          __html: DOMPurify.sanitize(article.content, {
            ALLOWED_TAGS: [
              'a',
              'b',
              'blockquote',
              'br',
              'code',
              'em',
              'figure',
              'figcaption',
              'h2',
              'h3',
              'h4',
              'hr',
              'i',
              'img',
              'li',
              'ol',
              'p',
              'pre',
              'strong',
              'ul',
            ],
            ALLOWED_ATTR: ['href', 'title', 'alt', 'src'],
            ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto):|[^a-z]|[a-z+.-]+(?:[^a-z+.\-:]|$))/i,
          }),
        }}
      />

      {article.reactionsEnabled && reactionCounts && (
        <section className="article-reactions-section" aria-label={t('aria.reactions')}>
          <ArticleReactions
            reactionCounts={reactionCounts}
            userReaction={userReaction}
            onToggleReaction={handleReaction}
          />
        </section>
      )}

      {article.commentsEnabled && (
        <section className="article-comments-section" aria-label={t('aria.comments')}>
          <ArticleComments
            comments={comments}
            onAddComment={handleAddComment}
            onEditComment={handleEditComment}
            onDeleteComment={handleDeleteComment}
            currentUserId=""
          />
        </section>
      )}
    </article>
  );
}

export default ArticleDetailPage;
