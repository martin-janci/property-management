//! Journal/News article routes (UC-13: Reality Portal News).
//!
//! Public article list + detail, comment list and (auth-required) comment creation.
//! D1.2: comment creation now uses the unified `RequestPrincipal` extractor.

use crate::state::AppState;
use api_core::extractors::RequestPrincipal;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Input hygiene constants (H6, M5)
// ---------------------------------------------------------------------------
// Comments are public-write (after auth) but a 100MB body still bloats the
// DB. Mirror the 5000-char cap used elsewhere in this server.
const MAX_COMMENT_BODY_LEN: usize = 5000;
// Default page size for unauthenticated comment listing. Without a cap a
// popular article DoSs both the API request and the SSR page that renders
// it.
const DEFAULT_COMMENTS_LIMIT: i64 = 50;
const MAX_COMMENTS_LIMIT: i64 = 200;

/// Create articles router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_articles))
        .route("/{slug}", get(get_article))
        .route("/{slug}/comments", get(list_comments))
        .route("/{slug}/comments", post(create_comment))
}

/// Article summary (for list view).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArticleSummary {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub excerpt: Option<String>,
    pub category: String,
    pub author_name: String,
    pub cover_image_url: Option<String>,
    pub comment_count: i64,
    pub published_at: DateTime<Utc>,
}

/// Related article (for detail view).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RelatedArticle {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub cover_image_url: Option<String>,
    pub published_at: DateTime<Utc>,
}

/// Article detail.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArticleDetail {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub body: String,
    pub excerpt: Option<String>,
    pub category: String,
    pub author_name: String,
    pub author_avatar_url: Option<String>,
    pub cover_image_url: Option<String>,
    pub tags: Option<Vec<String>>,
    pub comment_count: i64,
    pub published_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub related_articles: Vec<RelatedArticle>,
}

/// Article list response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArticlesListResponse {
    pub articles: Vec<ArticleSummary>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

/// Article detail response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArticleDetailResponse {
    pub article: ArticleDetail,
}

/// Article comment.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArticleComment {
    pub id: Uuid,
    pub article_id: Uuid,
    pub author_user_id: Uuid,
    pub author_name: String,
    pub author_avatar_url: Option<String>,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

/// Comments list response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommentsResponse {
    pub comments: Vec<ArticleComment>,
    pub total: i64,
}

/// Create comment request.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateCommentRequest {
    pub body: String,
}

/// Articles list query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ArticlesQuery {
    pub category: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Comments list query parameters (M5: paginate).
#[derive(Debug, Deserialize, IntoParams)]
pub struct CommentsQuery {
    /// Maximum comments to return. Defaults to 50, capped at 200.
    pub limit: Option<i64>,
    /// Number of comments to skip. Defaults to 0.
    pub offset: Option<i64>,
}

/// List articles with optional filters.
#[utoipa::path(
    get,
    path = "/api/v1/articles",
    tag = "Articles",
    params(ArticlesQuery),
    responses(
        (status = 200, description = "Article list", body = ArticlesListResponse)
    )
)]
pub async fn list_articles(
    State(state): State<AppState>,
    Query(query): Query<ArticlesQuery>,
) -> Result<Json<ArticlesListResponse>, (axum::http::StatusCode, String)> {
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;
    let search_pattern = query.search.as_ref().map(|s| format!("%{}%", s));

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    let rows = sqlx::query(
        r#"
        SELECT
            a.id,
            a.slug,
            a.title,
            a.excerpt,
            a.category,
            COALESCE(pu.name, 'Editorial Team') AS author_name,
            a.cover_image_url,
            COUNT(ac.id) AS comment_count,
            a.published_at
        FROM reality_articles a
        LEFT JOIN users pu ON pu.id = a.author_user_id AND pu.principal_kind = 'public'
        LEFT JOIN reality_article_comments ac ON ac.article_id = a.id
        WHERE a.published_at <= NOW()
          AND ($1::text IS NULL OR a.category = $1)
          AND ($2::text IS NULL OR (a.title ILIKE $2 OR a.excerpt ILIKE $2))
        GROUP BY a.id, pu.name
        ORDER BY a.published_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(&query.category)
    .bind(&search_pattern)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("list articles", e))?;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM reality_articles
        WHERE published_at <= NOW()
          AND ($1::text IS NULL OR category = $1)
          AND ($2::text IS NULL OR (title ILIKE $2 OR excerpt ILIKE $2))
        "#,
    )
    .bind(&query.category)
    .bind(&search_pattern)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("count articles", e))?;

    let articles: Vec<ArticleSummary> = rows
        .into_iter()
        .map(|r| ArticleSummary {
            id: r.get("id"),
            slug: r.get("slug"),
            title: r.get("title"),
            excerpt: r.get("excerpt"),
            category: r.get("category"),
            author_name: r
                .try_get::<String, _>("author_name")
                .unwrap_or_else(|_| "Editorial Team".to_string()),
            cover_image_url: r.get("cover_image_url"),
            comment_count: r.get("comment_count"),
            published_at: r.get("published_at"),
        })
        .collect();

    Ok(Json(ArticlesListResponse {
        articles,
        total,
        page,
        per_page,
    }))
}

/// Get article detail by slug.
#[utoipa::path(
    get,
    path = "/api/v1/articles/{slug}",
    tag = "Articles",
    params(("slug" = String, Path, description = "Article URL slug")),
    responses(
        (status = 200, description = "Article detail", body = ArticleDetailResponse),
        (status = 404, description = "Article not found")
    )
)]
pub async fn get_article(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<ArticleDetailResponse>, (axum::http::StatusCode, String)> {
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    let row = sqlx::query(
        r#"
        SELECT
            a.id, a.slug, a.title, a.body, a.excerpt, a.category,
            COALESCE(pu.name, 'Editorial Team') AS author_name,
            pu.profile_image_url AS author_avatar_url,
            a.cover_image_url, a.tags,
            COUNT(ac.id) AS comment_count,
            a.published_at, a.updated_at
        FROM reality_articles a
        LEFT JOIN users pu ON pu.id = a.author_user_id AND pu.principal_kind = 'public'
        LEFT JOIN reality_article_comments ac ON ac.article_id = a.id
        WHERE a.slug = $1 AND a.published_at <= NOW()
        GROUP BY a.id, pu.name, pu.profile_image_url
        "#,
    )
    .bind(&slug)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("get article", e))?
    .ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            "Article not found".to_string(),
        )
    })?;

    let category: String = row.get("category");

    // Fetch related articles (same category, different slug, latest 3)
    let related_rows = sqlx::query(
        r#"
        SELECT id, slug, title, cover_image_url, published_at
        FROM reality_articles
        WHERE category = $1 AND slug != $2 AND published_at <= NOW()
        ORDER BY published_at DESC
        LIMIT 3
        "#,
    )
    .bind(&category)
    .bind(&slug)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("fetch related articles", e))?;

    let related_articles: Vec<RelatedArticle> = related_rows
        .into_iter()
        .map(|r| RelatedArticle {
            id: r.get("id"),
            slug: r.get("slug"),
            title: r.get("title"),
            cover_image_url: r.get("cover_image_url"),
            published_at: r.get("published_at"),
        })
        .collect();

    let tags_json: Option<serde_json::Value> = row.get("tags");
    let tags: Option<Vec<String>> = tags_json.and_then(|v| serde_json::from_value(v).ok());

    let article = ArticleDetail {
        id: row.get("id"),
        slug: row.get("slug"),
        title: row.get("title"),
        body: row.get("body"),
        excerpt: row.get("excerpt"),
        category,
        author_name: row
            .try_get::<String, _>("author_name")
            .unwrap_or_else(|_| "Editorial Team".to_string()),
        author_avatar_url: row.get("author_avatar_url"),
        cover_image_url: row.get("cover_image_url"),
        tags,
        comment_count: row.get("comment_count"),
        published_at: row.get("published_at"),
        updated_at: row.get("updated_at"),
        related_articles,
    };

    Ok(Json(ArticleDetailResponse { article }))
}

/// List comments for an article (paginated — M5).
#[utoipa::path(
    get,
    path = "/api/v1/articles/{slug}/comments",
    tag = "Articles",
    params(
        ("slug" = String, Path, description = "Article URL slug"),
        CommentsQuery,
    ),
    responses(
        (status = 200, description = "Comment list", body = CommentsResponse),
        (status = 404, description = "Article not found")
    )
)]
pub async fn list_comments(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<CommentsQuery>,
) -> Result<Json<CommentsResponse>, (axum::http::StatusCode, String)> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_COMMENTS_LIMIT)
        .clamp(1, MAX_COMMENTS_LIMIT);
    let offset = query.offset.unwrap_or(0).max(0);

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    // Resolve article ID
    let article_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM reality_articles WHERE slug = $1 AND published_at <= NOW()",
    )
    .bind(&slug)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("database error", e))?;

    let article_id = article_id.ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            "Article not found".to_string(),
        )
    })?;

    // COUNT(*) OVER () returns the unfiltered total alongside each page row
    // so we get the real total without a second round-trip.
    let rows = sqlx::query(
        r#"
        SELECT
            ac.id, ac.article_id, ac.author_user_id,
            COALESCE(pu.name, 'Anonymous') AS author_name,
            pu.profile_image_url AS author_avatar_url,
            ac.body, ac.created_at,
            COUNT(*) OVER () AS total_count
        FROM reality_article_comments ac
        LEFT JOIN users pu ON pu.id = ac.author_user_id AND pu.principal_kind = 'public'
        WHERE ac.article_id = $1
        ORDER BY ac.created_at ASC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(article_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("list comments", e))?;

    let total: i64 = rows
        .first()
        .map(|r| r.get::<i64, _>("total_count"))
        .unwrap_or(0);

    let comments: Vec<ArticleComment> = rows
        .into_iter()
        .map(|r| ArticleComment {
            id: r.get("id"),
            article_id: r.get("article_id"),
            author_user_id: r.get("author_user_id"),
            author_name: r
                .try_get::<String, _>("author_name")
                .unwrap_or_else(|_| "Anonymous".to_string()),
            author_avatar_url: r.get("author_avatar_url"),
            body: r.get("body"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(CommentsResponse { comments, total }))
}

/// Create a comment on an article (requires authentication).
#[utoipa::path(
    post,
    path = "/api/v1/articles/{slug}/comments",
    tag = "Articles",
    params(("slug" = String, Path, description = "Article URL slug")),
    request_body = CreateCommentRequest,
    responses(
        (status = 201, description = "Comment created", body = ArticleComment),
        (status = 400, description = "Empty comment body"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Article not found")
    ),
    security(("session_token" = []))
)]
pub async fn create_comment(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(slug): Path<String>,
    Json(data): Json<CreateCommentRequest>,
) -> Result<(axum::http::StatusCode, Json<ArticleComment>), (axum::http::StatusCode, String)> {
    if data.body.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Comment body cannot be empty".to_string(),
        ));
    }
    if data.body.chars().count() > MAX_COMMENT_BODY_LEN {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!(
                "Comment body must be at most {} characters",
                MAX_COMMENT_BODY_LEN
            ),
        ));
    }

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    let article_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM reality_articles WHERE slug = $1 AND published_at <= NOW()",
    )
    .bind(&slug)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("database error", e))?;

    let article_id = article_id.ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            "Article not found".to_string(),
        )
    })?;

    // Get author info
    // Phase 6: reads from `users` (portal_users dropped in migration 00148).
    let author_name: String =
        sqlx::query_scalar("SELECT name FROM users WHERE id = $1 AND principal_kind = 'public'")
            .bind(principal.user_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| crate::util::errors::db_error("get author name", e))?
            .flatten()
            .unwrap_or_else(|| "Anonymous".to_string());

    let author_avatar_url: Option<String> = sqlx::query_scalar(
        "SELECT profile_image_url FROM users WHERE id = $1 AND principal_kind = 'public'",
    )
    .bind(principal.user_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("get avatar", e))?
    .flatten();

    let row = sqlx::query(
        r#"
        INSERT INTO reality_article_comments (article_id, author_user_id, body)
        VALUES ($1, $2, $3)
        RETURNING id, article_id, author_user_id, body, created_at
        "#,
    )
    .bind(article_id)
    .bind(principal.user_id)
    .bind(&data.body)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("create comment", e))?;

    let comment = ArticleComment {
        id: row.get("id"),
        article_id: row.get("article_id"),
        author_user_id: row.get("author_user_id"),
        author_name,
        author_avatar_url,
        body: row.get("body"),
        created_at: row.get("created_at"),
    };

    Ok((axum::http::StatusCode::CREATED, Json(comment)))
}
