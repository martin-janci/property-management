//! News article repository (Epic 59: News & Media Management).

use crate::models::news_article::{
    article_status, ArticleComment, ArticleListQuery, ArticleMedia, ArticleStatistics,
    ArticleSummary, ArticleView, ArticleWithDetails, ArticleWithDetailsRow, CommentWithAuthor,
    CommentWithAuthorRow, CreateArticle, CreateArticleComment, CreateArticleMedia, NewsArticle,
    ReactionCounts, UpdateArticle,
};
use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for news article operations.
#[derive(Clone)]
pub struct NewsArticleRepository {
    pool: DbPool,
}

impl NewsArticleRepository {
    /// Create a new NewsArticleRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        organization_id: Uuid,
        author_id: Uuid,
        data: CreateArticle,
    ) -> Result<NewsArticle, SqlxError> {
        let building_ids_json = serde_json::to_value(&data.building_ids).unwrap_or_default();
        let status = data.status.as_deref().unwrap_or(article_status::DRAFT);

        let article = sqlx::query_as::<_, NewsArticle>(
            r#"
            INSERT INTO news_articles (
                organization_id, author_id, title, content, excerpt,
                cover_image_url, building_ids, status, published_at,
                comments_enabled, reactions_enabled
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::article_status, $9, $10, $11)
            RETURNING id, organization_id, author_id, title, content, excerpt, cover_image_url, building_ids, status::TEXT AS status, published_at, archived_at, pinned, pinned_at, pinned_by, comments_enabled, reactions_enabled, view_count, reaction_count, comment_count, share_count, created_at, updated_at
            "#,
        )
        .bind(organization_id)
        .bind(author_id)
        .bind(&data.title)
        .bind(&data.content)
        .bind(&data.excerpt)
        .bind(&data.cover_image_url)
        .bind(&building_ids_json)
        .bind(status)
        .bind(data.published_at)
        .bind(data.comments_enabled)
        .bind(data.reactions_enabled)
        .fetch_one(&self.pool)
        .await?;

        Ok(article)
    }

    /// Find an article by ID, scoped to the caller's organization.
    ///
    /// SECURITY (issue #2314): the `organization_id` predicate is mandatory —
    /// `news_articles` only ENABLEs (not FORCEs) RLS and api-server connects with
    /// a BYPASSRLS role, so the tenant-isolation policy is inert on this path.
    /// The explicit predicate is therefore the only thing preventing a
    /// cross-tenant IDOR. Do not drop it.
    pub async fn find_by_id(
        &self,
        id: Uuid,
        organization_id: Uuid,
    ) -> Result<Option<NewsArticle>, SqlxError> {
        sqlx::query_as::<_, NewsArticle>(
            "SELECT id, organization_id, author_id, title, content, excerpt, cover_image_url, building_ids, status::TEXT AS status, published_at, archived_at, pinned, pinned_at, pinned_by, comments_enabled, reactions_enabled, view_count, reaction_count, comment_count, share_count, created_at, updated_at FROM news_articles WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(organization_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Find an article by ID with full details including author information.
    ///
    /// This uses a JOIN query to fetch the article along with author name and avatar
    /// from the users table.
    ///
    /// SECURITY (issue #2314): scoped to `organization_id`; see [`Self::find_by_id`].
    pub async fn find_by_id_with_details(
        &self,
        id: Uuid,
        organization_id: Uuid,
    ) -> Result<Option<ArticleWithDetails>, SqlxError> {
        let row = sqlx::query_as::<_, ArticleWithDetailsRow>(
            r#"
            SELECT
                a.id, a.organization_id, a.author_id, a.title, a.content,
                a.excerpt, a.cover_image_url, a.building_ids, a.status::TEXT AS status,
                a.published_at, a.archived_at, a.pinned, a.pinned_at, a.pinned_by,
                a.comments_enabled, a.reactions_enabled, a.view_count,
                a.reaction_count, a.comment_count, a.share_count,
                a.created_at, a.updated_at,
                COALESCE(u.name, 'Unknown') as author_name,
                u.profile_image_url as author_avatar_url
            FROM news_articles a
            LEFT JOIN users u ON a.author_id = u.id
            WHERE a.id = $1 AND a.organization_id = $2
            "#,
        )
        .bind(id)
        .bind(organization_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    /// List articles matching the query filters.
    ///
    /// Filters articles by organization_id for multi-tenant security.
    /// Additional filters include status, building_id, and pagination.
    pub async fn list(
        &self,
        organization_id: Uuid,
        query: &ArticleListQuery,
    ) -> Result<Vec<ArticleSummary>, SqlxError> {
        let limit = query.limit.unwrap_or(20).min(100);
        let offset = query.offset.unwrap_or(0);

        let articles = sqlx::query_as::<_, ArticleSummary>(
            r#"
            SELECT id, title, excerpt, cover_image_url, author_id, status::TEXT AS status,
                   published_at, pinned, view_count, reaction_count, comment_count, created_at
            FROM news_articles
            WHERE organization_id = $1
              AND ($2::text IS NULL OR status = $2::article_status)
              AND ($3::uuid IS NULL OR building_ids @> to_jsonb($3::uuid))
              AND ($4::bool IS NULL OR $4 = FALSE OR pinned = TRUE)
            ORDER BY pinned DESC, published_at DESC NULLS LAST, created_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(organization_id)
        .bind(query.status.as_deref())
        .bind(query.building_id)
        .bind(query.pinned_only)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(articles)
    }

    /// Count the total number of articles matching the query filters.
    ///
    /// This is used for accurate pagination totals, separate from the paginated list query.
    /// Filters by organization_id for multi-tenant security.
    pub async fn count(
        &self,
        organization_id: Uuid,
        query: &ArticleListQuery,
    ) -> Result<i64, SqlxError> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM news_articles
            WHERE organization_id = $1
              AND ($2::text IS NULL OR status = $2::article_status)
              AND ($3::uuid IS NULL OR building_ids @> to_jsonb($3::uuid))
              AND ($4::bool IS NULL OR $4 = FALSE OR pinned = TRUE)
            "#,
        )
        .bind(organization_id)
        .bind(query.status.as_deref())
        .bind(query.building_id)
        .bind(query.pinned_only)
        .fetch_one(&self.pool)
        .await?;
        Ok(result.0)
    }

    /// Update an article with the provided data.
    ///
    /// Uses COALESCE pattern for safe partial updates - only non-null parameters
    /// update their corresponding fields while keeping existing values for nulls.
    /// This approach is SQL-injection safe as all field names are static.
    /// Update an article, scoped to the caller's organization (issue #2314).
    pub async fn update(
        &self,
        id: Uuid,
        organization_id: Uuid,
        data: UpdateArticle,
    ) -> Result<Option<NewsArticle>, SqlxError> {
        // Convert building_ids to JSON if present
        let building_ids_json = data
            .building_ids
            .map(|ids| serde_json::to_value(&ids).unwrap_or_default());

        // Use COALESCE pattern - each field only updates if parameter is not null
        // This is SQL-injection safe as all field names are compile-time constants
        sqlx::query_as::<_, NewsArticle>(
            r#"
            UPDATE news_articles SET
                title = COALESCE($3, title),
                content = COALESCE($4, content),
                excerpt = COALESCE($5, excerpt),
                cover_image_url = COALESCE($6, cover_image_url),
                building_ids = COALESCE($7, building_ids),
                status = COALESCE($8::article_status, status),
                comments_enabled = COALESCE($9, comments_enabled),
                reactions_enabled = COALESCE($10, reactions_enabled),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING id, organization_id, author_id, title, content, excerpt, cover_image_url, building_ids, status::TEXT AS status, published_at, archived_at, pinned, pinned_at, pinned_by, comments_enabled, reactions_enabled, view_count, reaction_count, comment_count, share_count, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(organization_id)
        .bind(data.title)
        .bind(data.content)
        .bind(data.excerpt)
        .bind(data.cover_image_url)
        .bind(building_ids_json)
        .bind(data.status)
        .bind(data.comments_enabled)
        .bind(data.reactions_enabled)
        .fetch_optional(&self.pool)
        .await
    }

    /// Publish an article, scoped to the caller's organization (issue #2314).
    pub async fn publish(
        &self,
        id: Uuid,
        organization_id: Uuid,
        published_at: Option<DateTime<Utc>>,
    ) -> Result<Option<NewsArticle>, SqlxError> {
        sqlx::query_as::<_, NewsArticle>(
            "UPDATE news_articles SET status = $3::article_status, published_at = COALESCE($4, NOW()) WHERE id = $1 AND organization_id = $2 RETURNING id, organization_id, author_id, title, content, excerpt, cover_image_url, building_ids, status::TEXT AS status, published_at, archived_at, pinned, pinned_at, pinned_by, comments_enabled, reactions_enabled, view_count, reaction_count, comment_count, share_count, created_at, updated_at",
        )
        .bind(id)
        .bind(organization_id)
        .bind(article_status::PUBLISHED)
        .bind(published_at)
        .fetch_optional(&self.pool)
        .await
    }

    /// Archive an article, scoped to the caller's organization (issue #2314).
    pub async fn archive(
        &self,
        id: Uuid,
        organization_id: Uuid,
    ) -> Result<Option<NewsArticle>, SqlxError> {
        sqlx::query_as::<_, NewsArticle>(
            "UPDATE news_articles SET status = $3::article_status, archived_at = NOW() WHERE id = $1 AND organization_id = $2 RETURNING id, organization_id, author_id, title, content, excerpt, cover_image_url, building_ids, status::TEXT AS status, published_at, archived_at, pinned, pinned_at, pinned_by, comments_enabled, reactions_enabled, view_count, reaction_count, comment_count, share_count, created_at, updated_at",
        )
        .bind(id)
        .bind(organization_id)
        .bind(article_status::ARCHIVED)
        .fetch_optional(&self.pool)
        .await
    }

    /// Restore an archived article, scoped to the caller's organization (issue #2314).
    pub async fn restore(
        &self,
        id: Uuid,
        organization_id: Uuid,
    ) -> Result<Option<NewsArticle>, SqlxError> {
        sqlx::query_as::<_, NewsArticle>(
            "UPDATE news_articles SET status = $3::article_status, archived_at = NULL WHERE id = $1 AND organization_id = $2 RETURNING id, organization_id, author_id, title, content, excerpt, cover_image_url, building_ids, status::TEXT AS status, published_at, archived_at, pinned, pinned_at, pinned_by, comments_enabled, reactions_enabled, view_count, reaction_count, comment_count, share_count, created_at, updated_at",
        )
        .bind(id)
        .bind(organization_id)
        .bind(article_status::DRAFT)
        .fetch_optional(&self.pool)
        .await
    }

    /// Permanently delete an article, scoped to the caller's organization.
    ///
    /// SECURITY (issue #2314): the `organization_id` predicate prevents a
    /// cross-tenant caller from destroying another org's article by UUID — the
    /// original unscoped `WHERE id = $1` was a cross-tenant data-loss IDOR.
    pub async fn delete(&self, id: Uuid, organization_id: Uuid) -> Result<bool, SqlxError> {
        let result =
            sqlx::query("DELETE FROM news_articles WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(organization_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Pin/unpin an article, scoped to the caller's organization (issue #2314).
    pub async fn set_pinned(
        &self,
        id: Uuid,
        organization_id: Uuid,
        pinned: bool,
        pinned_by: Option<Uuid>,
    ) -> Result<Option<NewsArticle>, SqlxError> {
        if pinned {
            sqlx::query_as::<_, NewsArticle>(
                "UPDATE news_articles SET pinned = TRUE, pinned_at = NOW(), pinned_by = $3 WHERE id = $1 AND organization_id = $2 RETURNING id, organization_id, author_id, title, content, excerpt, cover_image_url, building_ids, status::TEXT AS status, published_at, archived_at, pinned, pinned_at, pinned_by, comments_enabled, reactions_enabled, view_count, reaction_count, comment_count, share_count, created_at, updated_at",
            )
            .bind(id)
            .bind(organization_id)
            .bind(pinned_by)
            .fetch_optional(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, NewsArticle>(
                "UPDATE news_articles SET pinned = FALSE, pinned_at = NULL, pinned_by = NULL WHERE id = $1 AND organization_id = $2 RETURNING id, organization_id, author_id, title, content, excerpt, cover_image_url, building_ids, status::TEXT AS status, published_at, archived_at, pinned, pinned_at, pinned_by, comments_enabled, reactions_enabled, view_count, reaction_count, comment_count, share_count, created_at, updated_at",
            )
            .bind(id)
            .bind(organization_id)
            .fetch_optional(&self.pool)
            .await
        }
    }

    /// Add media to an article, scoped to the caller's organization.
    ///
    /// SECURITY (issue #2314): `article_media` carries no `organization_id`
    /// column, so the insert is guarded by an `INSERT ... SELECT ... WHERE
    /// EXISTS` on the parent `news_articles` row for the caller's org. Returns
    /// `Ok(None)` when the article does not belong to the caller's org, so the
    /// handler can answer 404 rather than attaching media to a foreign article.
    pub async fn add_media(
        &self,
        article_id: Uuid,
        organization_id: Uuid,
        data: CreateArticleMedia,
    ) -> Result<Option<ArticleMedia>, SqlxError> {
        let display_order = data.display_order.unwrap_or(0);
        sqlx::query_as::<_, ArticleMedia>(
            r#"
            INSERT INTO article_media (
                article_id, media_type, file_key, file_name, file_size,
                mime_type, embed_url, embed_html, width, height,
                alt_text, caption, display_order
            )
            SELECT $1, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
            WHERE EXISTS (
                SELECT 1 FROM news_articles a
                WHERE a.id = $1 AND a.organization_id = $2
            )
            RETURNING *
            "#,
        )
        .bind(article_id)
        .bind(organization_id)
        .bind(&data.media_type)
        .bind(&data.file_key)
        .bind(&data.file_name)
        .bind(data.file_size)
        .bind(&data.mime_type)
        .bind(&data.embed_url)
        .bind(&data.embed_html)
        .bind(data.width)
        .bind(data.height)
        .bind(&data.alt_text)
        .bind(&data.caption)
        .bind(display_order)
        .fetch_optional(&self.pool)
        .await
    }

    /// List media for an article, scoped to the caller's organization (issue #2314).
    pub async fn list_media(
        &self,
        article_id: Uuid,
        organization_id: Uuid,
    ) -> Result<Vec<ArticleMedia>, SqlxError> {
        sqlx::query_as::<_, ArticleMedia>(
            r#"
            SELECT m.* FROM article_media m
            WHERE m.article_id = $1
              AND EXISTS (
                SELECT 1 FROM news_articles a
                WHERE a.id = m.article_id AND a.organization_id = $2
              )
            ORDER BY m.display_order
            "#,
        )
        .bind(article_id)
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Delete a media item, scoped to the caller's organization (issue #2314).
    ///
    /// The `EXISTS` guard on the parent article prevents a caller from deleting
    /// another org's media by its UUID even though `article_media` has no
    /// `organization_id` column of its own.
    pub async fn delete_media(
        &self,
        media_id: Uuid,
        organization_id: Uuid,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM article_media m
            WHERE m.id = $1
              AND EXISTS (
                SELECT 1 FROM news_articles a
                WHERE a.id = m.article_id AND a.organization_id = $2
              )
            "#,
        )
        .bind(media_id)
        .bind(organization_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Toggle a reaction on an article.
    ///
    /// Uses UPSERT pattern to handle race conditions:
    /// - If user has no reaction, adds the new reaction
    /// - If user has the same reaction, removes it (toggle off)
    /// - If user has a different reaction, updates to the new reaction
    pub async fn toggle_reaction(
        &self,
        article_id: Uuid,
        user_id: Uuid,
        reaction: &str,
    ) -> Result<bool, SqlxError> {
        // First, check if user already has a reaction on this article
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT reaction::text FROM article_reactions WHERE article_id = $1 AND user_id = $2",
        )
        .bind(article_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        match existing {
            Some((existing_reaction,)) if existing_reaction == reaction => {
                // Same reaction - toggle off (remove it)
                sqlx::query("DELETE FROM article_reactions WHERE article_id = $1 AND user_id = $2")
                    .bind(article_id)
                    .bind(user_id)
                    .execute(&self.pool)
                    .await?;
                Ok(false)
            }
            Some(_) => {
                // Different reaction - update to new reaction
                sqlx::query(
                    "UPDATE article_reactions SET reaction = $3::reaction_type WHERE article_id = $1 AND user_id = $2",
                )
                .bind(article_id)
                .bind(user_id)
                .bind(reaction)
                .execute(&self.pool)
                .await?;
                Ok(true)
            }
            None => {
                // No existing reaction - add new one
                sqlx::query(
                    "INSERT INTO article_reactions (article_id, user_id, reaction) VALUES ($1, $2, $3::reaction_type)",
                )
                .bind(article_id)
                .bind(user_id)
                .bind(reaction)
                .execute(&self.pool)
                .await?;
                Ok(true)
            }
        }
    }

    /// Get reaction counts for an article.
    pub async fn get_reaction_counts(&self, article_id: Uuid) -> Result<ReactionCounts, SqlxError> {
        let counts = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT reaction, COUNT(*) as count
            FROM article_reactions
            WHERE article_id = $1
            GROUP BY reaction
            "#,
        )
        .bind(article_id)
        .fetch_all(&self.pool)
        .await?;

        let mut reaction_counts = ReactionCounts::default();

        for (reaction, count) in counts {
            match reaction.as_str() {
                "like" => reaction_counts.like = count as i32,
                "love" => reaction_counts.love = count as i32,
                "surprised" => reaction_counts.surprised = count as i32,
                "sad" => reaction_counts.sad = count as i32,
                "angry" => reaction_counts.angry = count as i32,
                _ => {}
            }
        }

        reaction_counts.total = reaction_counts.like
            + reaction_counts.love
            + reaction_counts.surprised
            + reaction_counts.sad
            + reaction_counts.angry;

        Ok(reaction_counts)
    }

    pub async fn get_user_reaction(
        &self,
        article_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<String>, SqlxError> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT reaction::text FROM article_reactions WHERE article_id = $1 AND user_id = $2",
        )
        .bind(article_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(result.map(|(r,)| r))
    }

    pub async fn add_comment(
        &self,
        article_id: Uuid,
        user_id: Uuid,
        data: CreateArticleComment,
    ) -> Result<ArticleComment, SqlxError> {
        sqlx::query_as::<_, ArticleComment>(
            "INSERT INTO article_comments (article_id, user_id, parent_id, content) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(article_id)
        .bind(user_id)
        .bind(data.parent_id)
        .bind(&data.content)
        .fetch_one(&self.pool)
        .await
    }

    /// List comments for an article with author information.
    pub async fn list_comments(
        &self,
        article_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> Result<Vec<CommentWithAuthor>, SqlxError> {
        let rows = sqlx::query_as::<_, CommentWithAuthorRow>(
            r#"
            SELECT
                c.*,
                u.name as author_name,
                u.profile_image_url as author_avatar_url,
                (SELECT COUNT(*) FROM article_comments WHERE parent_id = c.id AND deleted_at IS NULL) as reply_count
            FROM article_comments c
            LEFT JOIN users u ON c.user_id = u.id
            WHERE c.article_id = $1
                AND c.parent_id IS NOT DISTINCT FROM $2
                AND c.deleted_at IS NULL
            ORDER BY c.created_at DESC
            "#,
        )
        .bind(article_id)
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn update_comment(
        &self,
        comment_id: Uuid,
        user_id: Uuid,
        content: &str,
    ) -> Result<Option<ArticleComment>, SqlxError> {
        sqlx::query_as::<_, ArticleComment>(
            "UPDATE article_comments SET content = $3 WHERE id = $1 AND user_id = $2 RETURNING *",
        )
        .bind(comment_id)
        .bind(user_id)
        .bind(content)
        .fetch_optional(&self.pool)
        .await
    }

    /// Soft delete a comment. Only the comment owner can delete their own comments.
    ///
    /// # Arguments
    ///
    /// * `comment_id` - The ID of the comment to delete.
    /// * `user_id` - The ID of the user requesting deletion. This serves dual purposes:
    ///   - Ownership verification: the WHERE clause ensures only the comment owner can delete
    ///   - Audit trail: recorded in the `deleted_by` field for accountability
    ///
    /// # Returns
    ///
    /// `true` if the comment was deleted, `false` if no matching comment was found
    /// (either doesn't exist or user is not the owner).
    pub async fn delete_comment(&self, comment_id: Uuid, user_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            "UPDATE article_comments
             SET deleted_at = NOW(), deleted_by = $2
             WHERE id = $1 AND user_id = $2",
        )
        .bind(comment_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Moderate a comment, scoped to the caller's organization (issue #2314).
    ///
    /// A manager may only moderate comments on articles belonging to their own
    /// org. The `EXISTS` guard on the parent article closes the cross-tenant
    /// vector (a manager in org A moderating/deleting an org-B comment by UUID).
    pub async fn moderate_comment(
        &self,
        comment_id: Uuid,
        moderator_id: Uuid,
        organization_id: Uuid,
        delete: bool,
        reason: Option<String>,
    ) -> Result<Option<ArticleComment>, SqlxError> {
        if delete {
            sqlx::query_as::<_, ArticleComment>(
                r#"
                UPDATE article_comments c
                SET is_moderated = TRUE, moderated_by = $2, moderation_reason = $3, deleted_at = NOW()
                WHERE c.id = $1
                  AND EXISTS (
                    SELECT 1 FROM news_articles a
                    WHERE a.id = c.article_id AND a.organization_id = $4
                  )
                RETURNING c.*
                "#,
            )
            .bind(comment_id)
            .bind(moderator_id)
            .bind(&reason)
            .bind(organization_id)
            .fetch_optional(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, ArticleComment>(
                r#"
                UPDATE article_comments c
                SET is_moderated = TRUE, moderated_by = $2, moderation_reason = $3
                WHERE c.id = $1
                  AND EXISTS (
                    SELECT 1 FROM news_articles a
                    WHERE a.id = c.article_id AND a.organization_id = $4
                  )
                RETURNING c.*
                "#,
            )
            .bind(comment_id)
            .bind(moderator_id)
            .bind(&reason)
            .bind(organization_id)
            .fetch_optional(&self.pool)
            .await
        }
    }

    pub async fn record_view(
        &self,
        article_id: Uuid,
        user_id: Option<Uuid>,
        duration_seconds: Option<i32>,
    ) -> Result<ArticleView, SqlxError> {
        sqlx::query_as::<_, ArticleView>(
            "INSERT INTO article_views (article_id, user_id, duration_seconds) VALUES ($1, $2, $3) ON CONFLICT (article_id, user_id) DO UPDATE SET viewed_at = NOW(), duration_seconds = $3 RETURNING *",
        )
        .bind(article_id)
        .bind(user_id)
        .bind(duration_seconds)
        .fetch_one(&self.pool)
        .await
    }

    /// Aggregate article statistics, scoped to the caller's organization.
    ///
    /// SECURITY (issue #2314): the previous query aggregated across ALL orgs
    /// (no `WHERE`), leaking cross-tenant counts to any caller. The
    /// `organization_id` predicate restricts the aggregate to the caller's org.
    pub async fn get_statistics(
        &self,
        organization_id: Uuid,
    ) -> Result<ArticleStatistics, SqlxError> {
        sqlx::query_as::<_, ArticleStatistics>(
            r#"
            SELECT
                COUNT(*) as total_articles,
                COUNT(*) FILTER (WHERE status = 'published') as published_articles,
                COUNT(*) FILTER (WHERE status = 'draft') as draft_articles,
                COUNT(*) FILTER (WHERE status = 'archived') as archived_articles,
                COALESCE(SUM(view_count), 0) as total_views,
                COALESCE(SUM(reaction_count), 0) as total_reactions,
                COALESCE(SUM(comment_count), 0) as total_comments
            FROM news_articles
            WHERE organization_id = $1
            "#,
        )
        .bind(organization_id)
        .fetch_one(&self.pool)
        .await
    }
}
