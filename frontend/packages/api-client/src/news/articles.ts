/**
 * News Articles API client for Epic 59.
 */

import { authenticatedFetch } from '../lib/fetch';
import type {
  ArticleListQuery,
  ArticleMedia,
  ArticleStatistics,
  ArticleSummary,
  ArticleWithDetails,
  CreateArticleRequest,
  CreateMediaRequest,
  CreateNewsCommentRequest,
  NewsArticle,
  NewsCommentWithAuthor,
  ReactionCounts,
  ReactionType,
  ToggleReactionRequest,
  UpdateArticleRequest,
} from './types';

const API_BASE = '/api/v1/news';

/**
 * List news articles with optional filters.
 */
export async function listArticles(query: ArticleListQuery): Promise<ArticleSummary[]> {
  const params = new URLSearchParams();
  params.append('organization_id', query.organization_id);

  if (query.status) params.append('status', query.status);
  if (query.building_id) params.append('building_id', query.building_id);
  if (query.pinned_only) params.append('pinned_only', 'true');
  if (query.limit) params.append('limit', String(query.limit));
  if (query.offset) params.append('offset', String(query.offset));

  const response = await authenticatedFetch(`${API_BASE}?${params.toString()}`);

  if (!response.ok) {
    throw new Error('Unable to load articles. Please try again later.');
  }

  return response.json();
}

/**
 * Get a single article by ID with author details.
 */
export async function getArticle(id: string, organizationId: string): Promise<ArticleWithDetails> {
  const params = new URLSearchParams();
  params.append('organization_id', organizationId);

  const response = await authenticatedFetch(`${API_BASE}/${id}?${params.toString()}`);

  if (!response.ok) {
    if (response.status === 404) {
      throw new Error('Article not found');
    }
    throw new Error('Unable to load article. Please try again later.');
  }

  return response.json();
}

/**
 * Create a new article.
 */
export async function createArticle(
  organizationId: string,
  data: CreateArticleRequest
): Promise<NewsArticle> {
  const response = await authenticatedFetch(`${API_BASE}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      organization_id: organizationId,
      ...data,
    }),
  });

  if (!response.ok) {
    throw new Error('Unable to create article. Please check your input and try again.');
  }

  return response.json();
}

/**
 * Update an existing article.
 */
export async function updateArticle(
  id: string,
  organizationId: string,
  data: UpdateArticleRequest
): Promise<NewsArticle> {
  const response = await authenticatedFetch(`${API_BASE}/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      organization_id: organizationId,
      ...data,
    }),
  });

  if (!response.ok) {
    if (response.status === 404) {
      throw new Error('Article not found');
    }
    throw new Error('Unable to update article. Please check your input and try again.');
  }

  return response.json();
}

/**
 * Delete an article.
 */
export async function deleteArticle(id: string, organizationId: string): Promise<void> {
  const params = new URLSearchParams();
  params.append('organization_id', organizationId);

  const response = await authenticatedFetch(`${API_BASE}/${id}?${params.toString()}`, {
    method: 'DELETE',
  });

  if (!response.ok) {
    if (response.status === 404) {
      throw new Error('Article not found');
    }
    throw new Error('Unable to delete article. Please try again later.');
  }
}

/**
 * Publish an article.
 */
export async function publishArticle(id: string, organizationId: string): Promise<NewsArticle> {
  const response = await authenticatedFetch(`${API_BASE}/${id}/publish`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ organization_id: organizationId }),
  });

  if (!response.ok) {
    throw new Error('Unable to publish article. Please try again later.');
  }

  return response.json();
}

/**
 * Archive an article.
 */
export async function archiveArticle(id: string, organizationId: string): Promise<NewsArticle> {
  const response = await authenticatedFetch(`${API_BASE}/${id}/archive`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ organization_id: organizationId }),
  });

  if (!response.ok) {
    throw new Error('Unable to archive article. Please try again later.');
  }

  return response.json();
}

/**
 * Restore an archived article.
 */
export async function restoreArticle(id: string, organizationId: string): Promise<NewsArticle> {
  const response = await authenticatedFetch(`${API_BASE}/${id}/restore`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ organization_id: organizationId }),
  });

  if (!response.ok) {
    throw new Error('Unable to restore article. Please try again later.');
  }

  return response.json();
}

/**
 * Pin or unpin an article.
 */
export async function pinArticle(
  id: string,
  organizationId: string,
  pinned: boolean
): Promise<NewsArticle> {
  const response = await authenticatedFetch(`${API_BASE}/${id}/pin`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ organization_id: organizationId, pinned }),
  });

  if (!response.ok) {
    throw new Error('Unable to update pin status. Please try again later.');
  }

  return response.json();
}

/**
 * Record a view for an article.
 */
export async function recordView(id: string, organizationId: string): Promise<void> {
  const response = await authenticatedFetch(`${API_BASE}/${id}/view`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ organization_id: organizationId }),
  });

  // Silently ignore view tracking errors
  if (!response.ok) {
    console.warn('Failed to record article view');
  }
}

/**
 * Get article statistics.
 */
export async function getStatistics(organizationId: string): Promise<ArticleStatistics> {
  const params = new URLSearchParams();
  params.append('organization_id', organizationId);

  const response = await authenticatedFetch(`${API_BASE}/statistics?${params.toString()}`);

  if (!response.ok) {
    throw new Error('Unable to load statistics. Please try again later.');
  }

  return response.json();
}

// ==================== Media ====================

/**
 * List media for an article.
 */
export async function listMedia(
  articleId: string,
  organizationId: string
): Promise<ArticleMedia[]> {
  const params = new URLSearchParams();
  params.append('organization_id', organizationId);

  const response = await authenticatedFetch(`${API_BASE}/${articleId}/media?${params.toString()}`);

  if (!response.ok) {
    throw new Error('Unable to load media. Please try again later.');
  }

  return response.json();
}

/**
 * Add media to an article.
 */
export async function addMedia(
  articleId: string,
  organizationId: string,
  data: CreateMediaRequest
): Promise<ArticleMedia> {
  const response = await authenticatedFetch(`${API_BASE}/${articleId}/media`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      organization_id: organizationId,
      ...data,
    }),
  });

  if (!response.ok) {
    throw new Error('Unable to add media. Please try again later.');
  }

  return response.json();
}

/**
 * Delete media from an article.
 */
export async function deleteMedia(
  articleId: string,
  mediaId: string,
  organizationId: string
): Promise<void> {
  const params = new URLSearchParams();
  params.append('organization_id', organizationId);

  const response = await authenticatedFetch(
    `${API_BASE}/${articleId}/media/${mediaId}?${params.toString()}`,
    {
      method: 'DELETE',
    }
  );

  if (!response.ok) {
    throw new Error('Unable to delete media. Please try again later.');
  }
}

// ==================== Reactions ====================

/**
 * Toggle a reaction on an article.
 */
export async function toggleReaction(
  articleId: string,
  organizationId: string,
  reactionType: ReactionType
): Promise<{ added: boolean; reaction_type: ReactionType }> {
  const response = await authenticatedFetch(`${API_BASE}/${articleId}/reactions`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      organization_id: organizationId,
      reaction_type: reactionType,
    } as ToggleReactionRequest & { organization_id: string }),
  });

  if (!response.ok) {
    throw new Error('Unable to update reaction. Please try again later.');
  }

  return response.json();
}

/**
 * Get reaction counts for an article.
 */
export async function getReactionCounts(
  articleId: string,
  organizationId: string
): Promise<ReactionCounts> {
  const params = new URLSearchParams();
  params.append('organization_id', organizationId);

  const response = await authenticatedFetch(
    `${API_BASE}/${articleId}/reactions/counts?${params.toString()}`
  );

  if (!response.ok) {
    throw new Error('Unable to load reactions. Please try again later.');
  }

  return response.json();
}

// ==================== Comments ====================

/**
 * List comments for an article.
 */
export async function listComments(
  articleId: string,
  organizationId: string
): Promise<NewsCommentWithAuthor[]> {
  const params = new URLSearchParams();
  params.append('organization_id', organizationId);

  const response = await authenticatedFetch(
    `${API_BASE}/${articleId}/comments?${params.toString()}`
  );

  if (!response.ok) {
    throw new Error('Unable to load comments. Please try again later.');
  }

  return response.json();
}

/**
 * Create a comment on an article.
 */
export async function createComment(
  articleId: string,
  organizationId: string,
  data: CreateNewsCommentRequest
): Promise<NewsCommentWithAuthor> {
  const response = await authenticatedFetch(`${API_BASE}/${articleId}/comments`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      organization_id: organizationId,
      ...data,
    }),
  });

  if (!response.ok) {
    throw new Error('Unable to post comment. Please try again later.');
  }

  return response.json();
}

/**
 * Update a comment.
 */
export async function updateComment(
  articleId: string,
  commentId: string,
  organizationId: string,
  content: string
): Promise<NewsCommentWithAuthor> {
  const response = await authenticatedFetch(`${API_BASE}/${articleId}/comments/${commentId}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      organization_id: organizationId,
      content,
    }),
  });

  if (!response.ok) {
    throw new Error('Unable to update comment. Please try again later.');
  }

  return response.json();
}

/**
 * Delete a comment.
 */
export async function deleteComment(
  articleId: string,
  commentId: string,
  organizationId: string
): Promise<void> {
  const params = new URLSearchParams();
  params.append('organization_id', organizationId);

  const response = await authenticatedFetch(
    `${API_BASE}/${articleId}/comments/${commentId}?${params.toString()}`,
    {
      method: 'DELETE',
    }
  );

  if (!response.ok) {
    throw new Error('Unable to delete comment. Please try again later.');
  }
}

/**
 * Get replies to a comment.
 */
export async function listReplies(
  articleId: string,
  commentId: string,
  organizationId: string
): Promise<NewsCommentWithAuthor[]> {
  const params = new URLSearchParams();
  params.append('organization_id', organizationId);

  const response = await authenticatedFetch(
    `${API_BASE}/${articleId}/comments/${commentId}/replies?${params.toString()}`
  );

  if (!response.ok) {
    throw new Error('Unable to load replies. Please try again later.');
  }

  return response.json();
}
