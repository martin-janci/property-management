//! News article routes (Epic 59: News & Media Management).

use crate::state::AppState;
use api_core::{AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use common::errors::ErrorResponse;
use db::models::{
    article_status, ArticleCommentWithAuthor, ArticleListQuery, ArticleMedia, ArticleStatistics,
    ArticleSummary, ArticleWithDetails, CreateArticle, CreateArticleComment, CreateArticleMedia,
    NewsArticle, ReactionCounts, UpdateArticle,
};
use db::repositories::NewsArticleRepository;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Maximum allowed title length (characters).
const MAX_TITLE_LENGTH: usize = 255;

/// Maximum allowed content length (characters).
const MAX_CONTENT_LENGTH: usize = 100_000; // Rich text can be larger

/// This value is the single source of truth for comment length validation.
/// Frontend clients should obtain this value via the API / OpenAPI schema
/// instead of duplicating it as a hard-coded constant.
const MAX_COMMENT_LENGTH: usize = 2000;

/// Validation rules for news article comments.
///
/// This struct is intended to be exposed via the API (and OpenAPI schema)
/// so that frontend clients can derive their validation rules from the
/// backend configuration rather than duplicating constants.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CommentValidationRules {
    /// Maximum length in characters allowed for a comment.
    pub max_length: usize,
}

impl CommentValidationRules {
    /// Build validation rules from the backend defaults defined in this module.
    pub fn from_defaults() -> Self {
        Self {
            max_length: MAX_COMMENT_LENGTH,
        }
    }
}

/// Helper to obtain the current comment validation rules.
/// This can be used by route handlers to expose validation limits to clients.
pub fn comment_validation_rules() -> CommentValidationRules {
    CommentValidationRules::from_defaults()
}

// ============================================================================
// Error Helpers
// ============================================================================

type ApiError = (StatusCode, Json<ErrorResponse>);

fn bad_request(message: &str) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::bad_request(message)),
    )
}

fn not_found(message: &str) -> ApiError {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::not_found(message)),
    )
}

fn internal_error(message: &str) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::internal_error(message)),
    )
}

fn forbidden(message: &str) -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse::forbidden(message)),
    )
}

/// Verify the article exists **and belongs to the caller's organization**.
///
/// SECURITY (issue #2314): child-resource handlers (media / reactions /
/// comments / views) are keyed on an article `id` taken from the path but their
/// repository queries are not all org-scoped. `news_articles` only ENABLEs (not
/// FORCEs) RLS and api-server connects with a BYPASSRLS role, so the RLS policy
/// is inert on this path. This guard resolves the parent article through the
/// org-scoped [`NewsArticleRepository::find_by_id`] and answers 404 when the
/// article is not in the caller's org, closing the cross-tenant IDOR uniformly
/// before any child row is read or written.
async fn ensure_article_in_org(
    repo: &NewsArticleRepository,
    id: Uuid,
    organization_id: Uuid,
) -> Result<(), ApiError> {
    repo.find_by_id(id, organization_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to load article: {}", e)))?
        .ok_or_else(|| not_found("Article not found"))?;
    Ok(())
}

// ============================================================================
// Response Types
// ============================================================================

/// Response for article creation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateArticleResponse {
    pub id: Uuid,
    pub message: String,
}

/// Response for article list with pagination.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArticleListResponse {
    pub articles: Vec<ArticleSummary>,
    pub count: usize,
    pub total: i64,
}

/// Response for article details.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArticleDetailResponse {
    pub article: ArticleWithDetails,
    pub media: Vec<ArticleMedia>,
    pub user_reaction: Option<String>,
    pub reaction_counts: ReactionCounts,
}

/// Response for generic article action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArticleActionResponse {
    pub message: String,
    pub article: NewsArticle,
}

/// Response for media list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MediaResponse {
    pub media: Vec<ArticleMedia>,
}

/// Response for statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StatisticsResponse {
    pub statistics: ArticleStatistics,
}

/// Response for reaction toggle.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReactionResponse {
    pub added: bool,
    pub reaction_counts: ReactionCounts,
}

/// Response for comments list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CommentsResponse {
    pub comments: Vec<ArticleCommentWithAuthor>,
    pub count: usize,
}

// ============================================================================
// Request Types
// ============================================================================

/// Request for creating an article.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateArticleRequest {
    pub title: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub cover_image_url: Option<String>,
    #[serde(default)]
    pub building_ids: Vec<Uuid>,
    pub status: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    #[serde(default = "default_true")]
    pub comments_enabled: bool,
    #[serde(default = "default_true")]
    pub reactions_enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Request for updating an article.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateArticleRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub excerpt: Option<String>,
    pub cover_image_url: Option<String>,
    pub building_ids: Option<Vec<Uuid>>,
    pub status: Option<String>,
    pub comments_enabled: Option<bool>,
    pub reactions_enabled: Option<bool>,
}

/// Request for publishing an article.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PublishArticleRequest {
    pub published_at: Option<DateTime<Utc>>,
}

/// Request for pinning/unpinning an article.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PinArticleRequest {
    pub pinned: bool,
}

/// Request for adding media.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddMediaRequest {
    pub media_type: String,
    pub file_key: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub embed_url: Option<String>,
    pub embed_html: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub alt_text: Option<String>,
    pub caption: Option<String>,
    pub display_order: Option<i32>,
}

/// Request for toggling reaction (Story 59.2).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ToggleReactionRequest {
    pub reaction: String,
}

/// Request for creating a comment (Story 59.3).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateCommentRequest {
    pub content: String,
    pub parent_id: Option<Uuid>,
}

/// Request for updating a comment.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateCommentRequest {
    pub content: String,
}

/// Request for moderating a comment.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ModerateCommentRequest {
    pub delete: bool,
    pub reason: Option<String>,
}

/// Request for recording view.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecordViewRequest {
    pub duration_seconds: Option<i32>,
}

// ============================================================================
// Router Setup
// ============================================================================

/// Create the news articles router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Article CRUD
        .route("/", get(list_articles).post(create_article))
        .route(
            "/{id}",
            get(get_article).put(update_article).delete(delete_article),
        )
        .route("/{id}/publish", post(publish_article))
        .route("/{id}/archive", post(archive_article))
        .route("/{id}/restore", post(restore_article))
        .route("/{id}/pin", post(pin_article))
        // Media
        .route("/{id}/media", get(list_media).post(add_media))
        .route("/{id}/media/{media_id}", delete(delete_media))
        // Reactions (Story 59.2)
        .route("/{id}/reactions", post(toggle_reaction))
        .route("/{id}/reactions/counts", get(get_reaction_counts))
        // Comments (Story 59.3)
        .route("/{id}/comments", get(list_comments).post(create_comment))
        .route(
            "/{id}/comments/{comment_id}",
            put(update_comment).delete(delete_comment),
        )
        .route(
            "/{id}/comments/{comment_id}/moderate",
            post(moderate_comment),
        )
        .route(
            "/{id}/comments/{comment_id}/replies",
            get(list_comment_replies),
        )
        // Analytics
        .route("/{id}/view", post(record_view))
        .route("/statistics", get(get_statistics))
}

// ============================================================================
// Article Handlers
// ============================================================================

/// List articles with filters (Story 59.1).
///
/// # Permissions
/// - Managers: Can see all articles (drafts, published, archived)
/// - Residents: Can see published articles for their buildings
#[utoipa::path(
    get,
    path = "/api/v1/news",
    params(ArticleListQuery),
    responses(
        (status = 200, description = "Articles retrieved", body = ArticleListResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "News Articles"
)]
async fn list_articles(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    _user: AuthUser,
    Query(query): Query<ArticleListQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    // Filter articles by tenant_id from tenant context for multi-tenant security
    let articles = repo
        .list(tenant.tenant_id, &query)
        .await
        .map_err(|e| internal_error(&format!("Failed to list articles: {}", e)))?;

    // Use separate count query for accurate pagination totals
    let total = repo
        .count(tenant.tenant_id, &query)
        .await
        .map_err(|e| internal_error(&format!("Failed to count articles: {}", e)))?;

    Ok(Json(ArticleListResponse {
        count: articles.len(),
        total,
        articles,
    }))
}

/// Get article details (Story 59.1).
#[utoipa::path(
    get,
    path = "/api/v1/news/{id}",
    params(
        ("id" = Uuid, Path, description = "Article ID")
    ),
    responses(
        (status = 200, description = "Article details retrieved", body = ArticleDetailResponse),
        (status = 404, description = "Article not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "News Articles"
)]
async fn get_article(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    // SECURITY (issue #2314): scope the lookup to the caller's org. Everything
    // below is keyed on this article `id`, so once the org-scoped read returns
    // 404 for a foreign article, none of the child reads can leak either.
    let article = repo
        .find_by_id_with_details(id, tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get article: {}", e)))?
        .ok_or_else(|| not_found("Article not found"))?;

    let media = repo
        .list_media(id, tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get media: {}", e)))?;

    let user_reaction = repo
        .get_user_reaction(id, user.user_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get user reaction: {}", e)))?;

    let reaction_counts = repo
        .get_reaction_counts(id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get reaction counts: {}", e)))?;

    Ok(Json(ArticleDetailResponse {
        article,
        media,
        user_reaction,
        reaction_counts,
    }))
}

/// Create a new article (Story 59.1).
///
/// # Permissions
/// - Only managers can create articles
#[utoipa::path(
    post,
    path = "/api/v1/news",
    request_body = CreateArticleRequest,
    responses(
        (status = 201, description = "Article created", body = CreateArticleResponse),
        (status = 400, description = "Invalid input"),
        (status = 403, description = "Forbidden - managers only"),
        (status = 500, description = "Internal server error")
    ),
    tag = "News Articles"
)]
async fn create_article(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    user: AuthUser,
    Json(req): Json<CreateArticleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Authorization check - only managers can create articles
    let is_manager = user.role.as_ref().map(|r| r.is_manager()).unwrap_or(false);
    if !is_manager {
        return Err(forbidden("Only managers can create articles"));
    }

    // Validate input
    if req.title.is_empty() || req.title.len() > MAX_TITLE_LENGTH {
        return Err(bad_request(&format!(
            "Title must be between 1 and {} characters",
            MAX_TITLE_LENGTH
        )));
    }

    if req.content.is_empty() || req.content.len() > MAX_CONTENT_LENGTH {
        return Err(bad_request(&format!(
            "Content must be between 1 and {} characters",
            MAX_CONTENT_LENGTH
        )));
    }

    // Validate status if provided
    if let Some(ref status) = req.status {
        if !article_status::ALL.contains(&status.as_str()) {
            return Err(bad_request(&format!(
                "Invalid status. Must be one of: {:?}",
                article_status::ALL
            )));
        }
    }

    let repo = NewsArticleRepository::new(state.db.clone());

    let data = CreateArticle {
        title: req.title,
        content: req.content,
        excerpt: req.excerpt,
        cover_image_url: req.cover_image_url,
        building_ids: req.building_ids,
        status: req.status,
        published_at: req.published_at,
        comments_enabled: req.comments_enabled,
        reactions_enabled: req.reactions_enabled,
    };

    let article = repo
        .create(tenant.tenant_id, user.user_id, data)
        .await
        .map_err(|e| internal_error(&format!("Failed to create article: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(CreateArticleResponse {
            id: article.id,
            message: "Article created successfully".to_string(),
        }),
    ))
}

/// Update an article (Story 59.1).
///
/// # Permissions
/// - Only the author or managers can update articles
#[utoipa::path(
    put,
    path = "/api/v1/news/{id}",
    params(
        ("id" = Uuid, Path, description = "Article ID")
    ),
    request_body = UpdateArticleRequest,
    responses(
        (status = 200, description = "Article updated", body = ArticleActionResponse),
        (status = 400, description = "Invalid input"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Article not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "News Articles"
)]
async fn update_article(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateArticleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    // Authorization check - only author or managers can update.
    // SECURITY (issue #2314): the lookup is org-scoped, so a caller in another
    // org gets 404 here rather than reaching the author/manager check.
    let article = repo
        .find_by_id(id, tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get article: {}", e)))?
        .ok_or_else(|| not_found("Article not found"))?;

    let is_manager = user.role.as_ref().map(|r| r.is_manager()).unwrap_or(false);
    let is_author = article.author_id == user.user_id;
    if !is_manager && !is_author {
        return Err(forbidden(
            "Only the author or managers can update this article",
        ));
    }

    // Validate input
    if let Some(ref title) = req.title {
        if title.is_empty() || title.len() > MAX_TITLE_LENGTH {
            return Err(bad_request(&format!(
                "Title must be between 1 and {} characters",
                MAX_TITLE_LENGTH
            )));
        }
    }

    if let Some(ref content) = req.content {
        if content.is_empty() || content.len() > MAX_CONTENT_LENGTH {
            return Err(bad_request(&format!(
                "Content must be between 1 and {} characters",
                MAX_CONTENT_LENGTH
            )));
        }
    }

    let data = UpdateArticle {
        title: req.title,
        content: req.content,
        excerpt: req.excerpt,
        cover_image_url: req.cover_image_url,
        building_ids: req.building_ids,
        status: req.status,
        comments_enabled: req.comments_enabled,
        reactions_enabled: req.reactions_enabled,
    };

    let article = repo
        .update(id, tenant.tenant_id, data)
        .await
        .map_err(|e| internal_error(&format!("Failed to update article: {}", e)))?
        .ok_or_else(|| not_found("Article not found"))?;

    Ok(Json(ArticleActionResponse {
        message: "Article updated successfully".to_string(),
        article,
    }))
}

/// Publish an article (Story 59.1).
#[utoipa::path(
    post,
    path = "/api/v1/news/{id}/publish",
    params(
        ("id" = Uuid, Path, description = "Article ID")
    ),
    request_body = PublishArticleRequest,
    responses(
        (status = 200, description = "Article published", body = ArticleActionResponse),
        (status = 404, description = "Article not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "News Articles"
)]
async fn publish_article(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PublishArticleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    // Authorization check - only the author or managers can publish.
    // The lookup is org-scoped, so a caller in another org gets 404 here
    // rather than reaching the author/manager check.
    let existing = repo
        .find_by_id(id, tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get article: {}", e)))?
        .ok_or_else(|| not_found("Article not found"))?;

    let is_manager = user.role.as_ref().map(|r| r.is_manager()).unwrap_or(false);
    let is_author = existing.author_id == user.user_id;
    if !is_manager && !is_author {
        return Err(forbidden(
            "Only the author or managers can publish this article",
        ));
    }

    let article = repo
        .publish(id, tenant.tenant_id, req.published_at)
        .await
        .map_err(|e| internal_error(&format!("Failed to publish article: {}", e)))?
        .ok_or_else(|| not_found("Cannot publish article"))?;

    Ok(Json(ArticleActionResponse {
        message: "Article published successfully".to_string(),
        article,
    }))
}

/// Archive an article (Story 59.4).
#[utoipa::path(
    post,
    path = "/api/v1/news/{id}/archive",
    params(
        ("id" = Uuid, Path, description = "Article ID")
    ),
    responses(
        (status = 200, description = "Article archived", body = ArticleActionResponse),
        (status = 404, description = "Article not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "News Articles"
)]
async fn archive_article(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    // Authorization check - only the author or managers can archive.
    // The lookup is org-scoped, so a caller in another org gets 404 here
    // rather than reaching the author/manager check.
    let existing = repo
        .find_by_id(id, tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get article: {}", e)))?
        .ok_or_else(|| not_found("Article not found"))?;

    let is_manager = user.role.as_ref().map(|r| r.is_manager()).unwrap_or(false);
    let is_author = existing.author_id == user.user_id;
    if !is_manager && !is_author {
        return Err(forbidden(
            "Only the author or managers can archive this article",
        ));
    }

    let article = repo
        .archive(id, tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to archive article: {}", e)))?
        .ok_or_else(|| not_found("Article not found"))?;

    Ok(Json(ArticleActionResponse {
        message: "Article archived successfully".to_string(),
        article,
    }))
}

/// Restore an article from archive (Story 59.4).
#[utoipa::path(
    post,
    path = "/api/v1/news/{id}/restore",
    params(
        ("id" = Uuid, Path, description = "Article ID")
    ),
    responses(
        (status = 200, description = "Article restored", body = ArticleActionResponse),
        (status = 404, description = "Article not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "News Articles"
)]
async fn restore_article(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    // Authorization check - only the author or managers can restore.
    // The lookup is org-scoped, so a caller in another org gets 404 here
    // rather than reaching the author/manager check.
    let existing = repo
        .find_by_id(id, tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get article: {}", e)))?
        .ok_or_else(|| not_found("Article not found"))?;

    let is_manager = user.role.as_ref().map(|r| r.is_manager()).unwrap_or(false);
    let is_author = existing.author_id == user.user_id;
    if !is_manager && !is_author {
        return Err(forbidden(
            "Only the author or managers can restore this article",
        ));
    }

    let article = repo
        .restore(id, tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to restore article: {}", e)))?
        .ok_or_else(|| not_found("Cannot restore article"))?;

    Ok(Json(ArticleActionResponse {
        message: "Article restored successfully".to_string(),
        article,
    }))
}

/// Delete an article permanently (Story 59.4).
#[utoipa::path(
    delete,
    path = "/api/v1/news/{id}",
    params(
        ("id" = Uuid, Path, description = "Article ID")
    ),
    responses(
        (status = 200, description = "Article deleted"),
        (status = 404, description = "Article not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "News Articles"
)]
async fn delete_article(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    let deleted = repo
        .delete(id, tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to delete article: {}", e)))?;

    if !deleted {
        return Err(not_found("Article not found"));
    }

    Ok(StatusCode::OK)
}

/// Pin/unpin an article.
#[utoipa::path(
    post,
    path = "/api/v1/news/{id}/pin",
    params(
        ("id" = Uuid, Path, description = "Article ID")
    ),
    request_body = PinArticleRequest,
    responses(
        (status = 200, description = "Article pin status updated", body = ArticleActionResponse),
        (status = 404, description = "Article not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "News Articles"
)]
async fn pin_article(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PinArticleRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    // Authorization check - only the author or managers can pin/unpin.
    // The lookup is org-scoped, so a caller in another org gets 404 here
    // rather than reaching the author/manager check.
    let existing = repo
        .find_by_id(id, tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get article: {}", e)))?
        .ok_or_else(|| not_found("Article not found"))?;

    let is_manager = user.role.as_ref().map(|r| r.is_manager()).unwrap_or(false);
    let is_author = existing.author_id == user.user_id;
    if !is_manager && !is_author {
        return Err(forbidden(
            "Only the author or managers can pin this article",
        ));
    }

    let article = repo
        .set_pinned(id, tenant.tenant_id, req.pinned, Some(user.user_id))
        .await
        .map_err(|e| internal_error(&format!("Failed to pin article: {}", e)))?
        .ok_or_else(|| not_found("Article not found"))?;

    let message = if req.pinned {
        "Article pinned successfully"
    } else {
        "Article unpinned successfully"
    };

    Ok(Json(ArticleActionResponse {
        message: message.to_string(),
        article,
    }))
}

// ============================================================================
// Media Handlers
// ============================================================================

/// List media for an article.
async fn list_media(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    let media = repo
        .list_media(id, tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to list media: {}", e)))?;

    Ok(Json(MediaResponse { media }))
}

/// Add media to an article.
async fn add_media(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AddMediaRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    let data = CreateArticleMedia {
        media_type: req.media_type,
        file_key: req.file_key,
        file_name: req.file_name,
        file_size: req.file_size,
        mime_type: req.mime_type,
        embed_url: req.embed_url,
        embed_html: req.embed_html,
        width: req.width,
        height: req.height,
        alt_text: req.alt_text,
        caption: req.caption,
        display_order: req.display_order,
    };

    // SECURITY (issue #2314): the repo insert is guarded by an EXISTS on the
    // parent article for this org; `None` means the article is not in the
    // caller's org, so answer 404 rather than 500.
    let media = repo
        .add_media(id, tenant.tenant_id, data)
        .await
        .map_err(|e| internal_error(&format!("Failed to add media: {}", e)))?
        .ok_or_else(|| not_found("Article not found"))?;

    Ok((StatusCode::CREATED, Json(media)))
}

/// Delete media from an article.
async fn delete_media(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    _user: AuthUser,
    Path((_id, media_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    // SECURITY (issue #2314): scoped by the parent article's org so a caller
    // cannot delete another org's media item by UUID.
    let deleted = repo
        .delete_media(media_id, tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to delete media: {}", e)))?;

    if !deleted {
        return Err(not_found("Media not found"));
    }

    Ok(StatusCode::OK)
}

// ============================================================================
// Reaction Handlers (Story 59.2)
// ============================================================================

/// Toggle reaction on an article.
async fn toggle_reaction(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ToggleReactionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate reaction type
    const VALID_REACTIONS: &[&str] = &["like", "love", "surprised", "sad", "angry"];
    if !VALID_REACTIONS.contains(&req.reaction.as_str()) {
        return Err(bad_request(&format!(
            "Invalid reaction type. Must be one of: {:?}",
            VALID_REACTIONS
        )));
    }

    let repo = NewsArticleRepository::new(state.db.clone());
    ensure_article_in_org(&repo, id, tenant.tenant_id).await?;

    let added = repo
        .toggle_reaction(id, user.user_id, &req.reaction)
        .await
        .map_err(|e| internal_error(&format!("Failed to toggle reaction: {}", e)))?;

    let reaction_counts = repo
        .get_reaction_counts(id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get reaction counts: {}", e)))?;

    Ok(Json(ReactionResponse {
        added,
        reaction_counts,
    }))
}

/// Get reaction counts for an article.
async fn get_reaction_counts(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());
    ensure_article_in_org(&repo, id, tenant.tenant_id).await?;

    let counts = repo
        .get_reaction_counts(id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get reaction counts: {}", e)))?;

    Ok(Json(counts))
}

// ============================================================================
// Comment Handlers (Story 59.3)
// ============================================================================

/// List top-level comments for an article.
async fn list_comments(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());
    ensure_article_in_org(&repo, id, tenant.tenant_id).await?;

    let comments = repo
        .list_comments(id, None)
        .await
        .map_err(|e| internal_error(&format!("Failed to list comments: {}", e)))?;

    Ok(Json(CommentsResponse {
        count: comments.len(),
        comments,
    }))
}

/// List replies to a specific comment.
async fn list_comment_replies(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    _user: AuthUser,
    Path((id, comment_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());
    ensure_article_in_org(&repo, id, tenant.tenant_id).await?;

    let comments = repo
        .list_comments(id, Some(comment_id))
        .await
        .map_err(|e| internal_error(&format!("Failed to list comment replies: {}", e)))?;

    Ok(Json(CommentsResponse {
        count: comments.len(),
        comments,
    }))
}

/// Create a comment on an article.
async fn create_comment(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate input
    if req.content.is_empty() || req.content.len() > MAX_COMMENT_LENGTH {
        return Err(bad_request(&format!(
            "Comment must be between 1 and {} characters",
            MAX_COMMENT_LENGTH
        )));
    }

    let repo = NewsArticleRepository::new(state.db.clone());
    ensure_article_in_org(&repo, id, tenant.tenant_id).await?;

    let data = CreateArticleComment {
        content: req.content,
        parent_id: req.parent_id,
    };

    let comment = repo
        .add_comment(id, user.user_id, data)
        .await
        .map_err(|e| internal_error(&format!("Failed to create comment: {}", e)))?;

    Ok((StatusCode::CREATED, Json(comment)))
}

/// Update a comment.
async fn update_comment(
    State(state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    user: AuthUser,
    Path((_id, comment_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateCommentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Ownership is verified in the repository query (WHERE user_id = $2)
    // Validate input
    if req.content.is_empty() || req.content.len() > MAX_COMMENT_LENGTH {
        return Err(bad_request(&format!(
            "Comment must be between 1 and {} characters",
            MAX_COMMENT_LENGTH
        )));
    }

    let repo = NewsArticleRepository::new(state.db.clone());

    let comment = repo
        .update_comment(comment_id, user.user_id, &req.content)
        .await
        .map_err(|e| internal_error(&format!("Failed to update comment: {}", e)))?
        .ok_or_else(|| not_found("Comment not found"))?;

    Ok(Json(comment))
}

/// Delete a comment (soft delete).
async fn delete_comment(
    State(state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    user: AuthUser,
    Path((_id, comment_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    let deleted = repo
        .delete_comment(comment_id, user.user_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to delete comment: {}", e)))?;

    if !deleted {
        return Err(not_found("Comment not found"));
    }

    Ok(StatusCode::OK)
}

/// Moderate a comment (manager action).
async fn moderate_comment(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    user: AuthUser,
    Path((_id, comment_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ModerateCommentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Only managers can moderate comments
    let is_manager = user.role.as_ref().map(|r| r.is_manager()).unwrap_or(false);
    if !is_manager {
        return Err(forbidden("Only managers can moderate comments"));
    }

    let repo = NewsArticleRepository::new(state.db.clone());

    // SECURITY (issue #2314): the repo query is org-scoped via an EXISTS on the
    // parent article, so a manager cannot moderate another org's comment by UUID.
    let comment = repo
        .moderate_comment(
            comment_id,
            user.user_id,
            tenant.tenant_id,
            req.delete,
            req.reason,
        )
        .await
        .map_err(|e| internal_error(&format!("Failed to moderate comment: {}", e)))?
        .ok_or_else(|| not_found("Comment not found"))?;

    Ok(Json(comment))
}

// ============================================================================
// Analytics Handlers
// ============================================================================

/// Record a view on an article.
async fn record_view(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RecordViewRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());
    ensure_article_in_org(&repo, id, tenant.tenant_id).await?;

    repo.record_view(id, Some(user.user_id), req.duration_seconds)
        .await
        .map_err(|e| internal_error(&format!("Failed to record view: {}", e)))?;

    Ok(StatusCode::OK)
}

/// Get article statistics.
async fn get_statistics(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    _user: AuthUser,
) -> Result<impl IntoResponse, ApiError> {
    let repo = NewsArticleRepository::new(state.db.clone());

    // SECURITY (issue #2314): stats are scoped to the caller's org; the repo
    // query previously aggregated across ALL organizations.
    let statistics = repo
        .get_statistics(tenant.tenant_id)
        .await
        .map_err(|e| internal_error(&format!("Failed to get statistics: {}", e)))?;

    Ok(Json(StatisticsResponse { statistics }))
}
