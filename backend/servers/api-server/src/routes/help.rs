//! Contextual Help routes (Epic 10B, Story 10B.7).
//!
//! Routes for help articles, FAQ, and tooltips.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::repositories::{FaqEntry, HelpArticle, HelpCategory, Tooltip};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Create help router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Articles
        .route("/articles", get(list_articles))
        .route("/articles/search", get(search_articles))
        .route(
            "/articles/category/{category}",
            get(list_articles_by_category),
        )
        .route(
            "/articles/context/{context_key}",
            get(list_articles_by_context),
        )
        .route("/articles/{slug}", get(get_article))
        .route("/articles/{slug}/feedback", post(submit_article_feedback))
        // Categories
        .route("/categories", get(list_categories))
        .route("/categories/{slug}", get(get_category))
        // FAQ
        .route("/faq", get(list_faq))
        .route("/faq/search", get(search_faq))
        .route("/faq/category/{category}", get(list_faq_by_category))
        // Tooltips
        .route("/tooltips", get(list_tooltips))
        .route("/tooltips/{key}", get(get_tooltip))
        .route("/tooltips/prefix/{prefix}", get(list_tooltips_by_prefix))
}

// ==================== Types ====================

/// Search query parameters.
#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct SearchQuery {
    /// Search query
    pub q: String,
}

/// Article feedback request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ArticleFeedbackRequest {
    /// Whether the article was helpful
    pub is_helpful: bool,
    /// Optional feedback text
    pub feedback: Option<String>,
}

/// Article feedback response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ArticleFeedbackResponse {
    pub message: String,
}

// ==================== Helper Functions ====================

/// Extract optional user from token (for feedback).
fn extract_user_token(
    headers: &axum::http::HeaderMap,
    state: &AppState,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_TOKEN",
                    "Authorization header required",
                )),
            )
        })?;

    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Bearer token required")),
        ));
    }

    let token = &auth_header[7..];
    let claims = state
        .jwt_service
        .validate_access_token(token)
        .map_err(|e| {
            tracing::debug!(error = %e, "Invalid access token");
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "INVALID_TOKEN",
                    "Invalid or expired token",
                )),
            )
        })?;

    let user_id: Uuid = claims.sub.parse().map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_TOKEN", "Invalid token format")),
        )
    })?;

    Ok(user_id)
}

// ==================== Article Endpoints ====================

/// List all published articles.
#[utoipa::path(
    get,
    path = "/api/v1/help/articles",
    responses(
        (status = 200, description = "Articles retrieved", body = Vec<HelpArticle>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn list_articles(
    State(state): State<AppState>,
) -> Result<Json<Vec<HelpArticle>>, (StatusCode, Json<ErrorResponse>)> {
    let articles = state
        .help_repo
        .get_published_articles()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get articles");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get articles",
                )),
            )
        })?;

    Ok(Json(articles))
}

/// Search articles.
#[utoipa::path(
    get,
    path = "/api/v1/help/articles/search",
    params(SearchQuery),
    responses(
        (status = 200, description = "Search results", body = Vec<HelpArticle>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn search_articles(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<HelpArticle>>, (StatusCode, Json<ErrorResponse>)> {
    let articles = state
        .help_repo
        .search_articles(&query.q)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to search articles");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to search articles",
                )),
            )
        })?;

    Ok(Json(articles))
}

/// List articles by category.
#[utoipa::path(
    get,
    path = "/api/v1/help/articles/category/{category}",
    params(
        ("category" = String, Path, description = "Category slug")
    ),
    responses(
        (status = 200, description = "Articles retrieved", body = Vec<HelpArticle>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn list_articles_by_category(
    State(state): State<AppState>,
    Path(category): Path<String>,
) -> Result<Json<Vec<HelpArticle>>, (StatusCode, Json<ErrorResponse>)> {
    let articles = state
        .help_repo
        .get_articles_by_category(&category)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get articles by category");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get articles",
                )),
            )
        })?;

    Ok(Json(articles))
}

/// List articles by context key.
#[utoipa::path(
    get,
    path = "/api/v1/help/articles/context/{context_key}",
    params(
        ("context_key" = String, Path, description = "Context key (e.g., page:dashboard)")
    ),
    responses(
        (status = 200, description = "Articles retrieved", body = Vec<HelpArticle>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn list_articles_by_context(
    State(state): State<AppState>,
    Path(context_key): Path<String>,
) -> Result<Json<Vec<HelpArticle>>, (StatusCode, Json<ErrorResponse>)> {
    let articles = state
        .help_repo
        .get_articles_by_context(&context_key)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get articles by context");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get articles",
                )),
            )
        })?;

    Ok(Json(articles))
}

/// Get article by slug.
#[utoipa::path(
    get,
    path = "/api/v1/help/articles/{slug}",
    params(
        ("slug" = String, Path, description = "Article slug")
    ),
    responses(
        (status = 200, description = "Article retrieved", body = HelpArticle),
        (status = 404, description = "Article not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn get_article(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<HelpArticle>, (StatusCode, Json<ErrorResponse>)> {
    let article = state
        .help_repo
        .get_article_by_slug(&slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get article");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get article",
                )),
            )
        })?;

    let article = article.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("ARTICLE_NOT_FOUND", "Article not found")),
        )
    })?;

    // Increment view count in background
    let slug_clone = slug.clone();
    let repo = state.help_repo.clone();
    tokio::spawn(async move {
        let _ = repo.increment_article_view(&slug_clone).await;
    });

    Ok(Json(article))
}

/// Submit feedback for an article.
#[utoipa::path(
    post,
    path = "/api/v1/help/articles/{slug}/feedback",
    params(
        ("slug" = String, Path, description = "Article slug")
    ),
    request_body = ArticleFeedbackRequest,
    responses(
        (status = 200, description = "Feedback submitted", body = ArticleFeedbackResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Article not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Help"
)]
pub async fn submit_article_feedback(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(slug): Path<String>,
    Json(request): Json<ArticleFeedbackRequest>,
) -> Result<Json<ArticleFeedbackResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = extract_user_token(&headers, &state)?;

    let article = state
        .help_repo
        .get_article_by_slug(&slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get article");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get article",
                )),
            )
        })?;

    let article = article.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("ARTICLE_NOT_FOUND", "Article not found")),
        )
    })?;

    state
        .help_repo
        .record_article_feedback(
            article.id,
            user_id,
            request.is_helpful,
            request.feedback.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to record feedback");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to record feedback",
                )),
            )
        })?;

    Ok(Json(ArticleFeedbackResponse {
        message: "Thank you for your feedback!".to_string(),
    }))
}

// ==================== Category Endpoints ====================

/// List all categories.
#[utoipa::path(
    get,
    path = "/api/v1/help/categories",
    responses(
        (status = 200, description = "Categories retrieved", body = Vec<HelpCategory>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn list_categories(
    State(state): State<AppState>,
) -> Result<Json<Vec<HelpCategory>>, (StatusCode, Json<ErrorResponse>)> {
    let categories = state.help_repo.get_categories().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get categories");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to get categories",
            )),
        )
    })?;

    Ok(Json(categories))
}

/// Get category by slug.
#[utoipa::path(
    get,
    path = "/api/v1/help/categories/{slug}",
    params(
        ("slug" = String, Path, description = "Category slug")
    ),
    responses(
        (status = 200, description = "Category retrieved", body = HelpCategory),
        (status = 404, description = "Category not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn get_category(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<HelpCategory>, (StatusCode, Json<ErrorResponse>)> {
    let category = state
        .help_repo
        .get_category_by_slug(&slug)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get category");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get category",
                )),
            )
        })?;

    let category = category.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "CATEGORY_NOT_FOUND",
                "Category not found",
            )),
        )
    })?;

    Ok(Json(category))
}

// ==================== FAQ Endpoints ====================

/// List all FAQ.
#[utoipa::path(
    get,
    path = "/api/v1/help/faq",
    responses(
        (status = 200, description = "FAQ retrieved", body = Vec<FaqEntry>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn list_faq(
    State(state): State<AppState>,
) -> Result<Json<Vec<FaqEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let faq = state.help_repo.get_faq().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get FAQ");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get FAQ")),
        )
    })?;

    Ok(Json(faq))
}

/// Search FAQ.
#[utoipa::path(
    get,
    path = "/api/v1/help/faq/search",
    params(SearchQuery),
    responses(
        (status = 200, description = "Search results", body = Vec<FaqEntry>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn search_faq(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<FaqEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let faq = state.help_repo.search_faq(&query.q).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to search FAQ");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Failed to search FAQ")),
        )
    })?;

    Ok(Json(faq))
}

/// List FAQ by category.
#[utoipa::path(
    get,
    path = "/api/v1/help/faq/category/{category}",
    params(
        ("category" = String, Path, description = "Category slug")
    ),
    responses(
        (status = 200, description = "FAQ retrieved", body = Vec<FaqEntry>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn list_faq_by_category(
    State(state): State<AppState>,
    Path(category): Path<String>,
) -> Result<Json<Vec<FaqEntry>>, (StatusCode, Json<ErrorResponse>)> {
    let faq = state
        .help_repo
        .get_faq_by_category(&category)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get FAQ by category");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get FAQ")),
            )
        })?;

    Ok(Json(faq))
}

// ==================== Tooltip Endpoints ====================

/// List all tooltips.
#[utoipa::path(
    get,
    path = "/api/v1/help/tooltips",
    responses(
        (status = 200, description = "Tooltips retrieved", body = Vec<Tooltip>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn list_tooltips(
    State(state): State<AppState>,
) -> Result<Json<Vec<Tooltip>>, (StatusCode, Json<ErrorResponse>)> {
    let tooltips = state.help_repo.get_tooltips().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get tooltips");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to get tooltips",
            )),
        )
    })?;

    Ok(Json(tooltips))
}

/// Get tooltip by key.
#[utoipa::path(
    get,
    path = "/api/v1/help/tooltips/{key}",
    params(
        ("key" = String, Path, description = "Tooltip key")
    ),
    responses(
        (status = 200, description = "Tooltip retrieved", body = Tooltip),
        (status = 404, description = "Tooltip not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn get_tooltip(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<Tooltip>, (StatusCode, Json<ErrorResponse>)> {
    let tooltip = state.help_repo.get_tooltip(&key).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get tooltip");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(
                "DATABASE_ERROR",
                "Failed to get tooltip",
            )),
        )
    })?;

    let tooltip = tooltip.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("TOOLTIP_NOT_FOUND", "Tooltip not found")),
        )
    })?;

    Ok(Json(tooltip))
}

/// List tooltips by prefix.
#[utoipa::path(
    get,
    path = "/api/v1/help/tooltips/prefix/{prefix}",
    params(
        ("prefix" = String, Path, description = "Tooltip key prefix")
    ),
    responses(
        (status = 200, description = "Tooltips retrieved", body = Vec<Tooltip>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Help"
)]
pub async fn list_tooltips_by_prefix(
    State(state): State<AppState>,
    Path(prefix): Path<String>,
) -> Result<Json<Vec<Tooltip>>, (StatusCode, Json<ErrorResponse>)> {
    let tooltips = state
        .help_repo
        .get_tooltips_by_prefix(&prefix)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get tooltips by prefix");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to get tooltips",
                )),
            )
        })?;

    Ok(Json(tooltips))
}
