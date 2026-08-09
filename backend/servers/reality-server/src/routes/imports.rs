//! Property Import routes (Epic 34: Property Import).
//!
//! GH #1300: handlers use `PortalPrincipal`, not `RequestPrincipal`. Reality
//! agencies are `principal_kind = 'public'` with no `organization_members` row
//! and reach the portal on a `PlatformHost`, so `RequestPrincipal` 403s them.
//! `PortalPrincipal` authenticates the JWT + re-derives kind without requiring
//! a tenant/membership; IDOR stays closed via `user_id` repo keying (PR #1297).

use crate::state::AppState;
use api_core::extractors::PortalPrincipal;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use db::{
    models::{
        CreateFeedSubscription, CreatePortalImportJob, PortalImportJob, PortalImportJobWithStats,
        RealityFeedSubscription, UpdateFeedSubscription, UpdatePortalImportJob,
    },
    SqlxError,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Create imports router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Import jobs
        .route("/jobs", get(list_import_jobs))
        .route("/jobs", post(create_import_job))
        .route("/jobs/{id}", get(get_import_job))
        .route("/jobs/{id}", put(update_import_job))
        .route("/jobs/{id}/start", post(start_import_job))
        .route("/jobs/{id}/cancel", post(cancel_import_job))
        // Feed subscriptions
        .route("/feeds", get(list_feeds))
        .route("/feeds", post(create_feed))
        .route("/feeds/{id}", get(get_feed))
        .route("/feeds/{id}", put(update_feed))
        .route("/feeds/{id}/sync", post(sync_feed))
}

/// Resolve the agency the calling portal user belongs to (#1584).
///
/// Feed subscriptions are agency-scoped: they belong to the realtor's agency and
/// are shared across the agency's members, not keyed on the individual user.
/// Returns 403 when the caller is a member of no agency (they cannot own feeds).
/// Previously the feed handlers passed `principal.user_id` straight into the
/// `agency_id`-keyed repo methods, which (a) only worked because tests registered
/// agency UUIDs as users and (b) hid an agency's feeds from its other members.
async fn resolve_agency(
    state: &AppState,
    user_id: Uuid,
) -> Result<Uuid, (axum::http::StatusCode, String)> {
    state
        .reality_portal_repo
        .get_active_agency_for_user(user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("resolve agency", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::FORBIDDEN,
                "You are not a member of any agency".to_string(),
            )
        })
}

/// Import jobs list response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ImportJobsResponse {
    pub jobs: Vec<PortalImportJobWithStats>,
    pub total: i64,
}

/// Single import job response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ImportJobResponse {
    pub job: PortalImportJob,
}

/// Import jobs query parameters.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ImportJobsQuery {
    pub status: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Feed subscriptions list response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FeedsResponse {
    pub feeds: Vec<RealityFeedSubscription>,
    pub total: i64,
}

/// Single feed response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FeedResponse {
    pub feed: RealityFeedSubscription,
}

/// List import jobs for the current user/agency.
#[utoipa::path(
    get,
    path = "/api/v1/imports/jobs",
    tag = "Imports",
    params(ImportJobsQuery),
    responses(
        (status = 200, description = "List of import jobs", body = ImportJobsResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_import_jobs(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Query(query): Query<ImportJobsQuery>,
) -> Result<Json<ImportJobsResponse>, (axum::http::StatusCode, String)> {
    let jobs = state
        .reality_portal_repo
        .list_import_jobs(
            principal.user_id,
            query.status,
            query.limit.unwrap_or(20),
            query.offset.unwrap_or(0),
        )
        .await
        .map_err(|e| crate::util::errors::db_error("list import jobs", e))?;

    let total = jobs.len() as i64;

    Ok(Json(ImportJobsResponse { jobs, total }))
}

/// Create a new import job.
#[utoipa::path(
    post,
    path = "/api/v1/imports/jobs",
    tag = "Imports",
    request_body = CreatePortalImportJob,
    responses(
        (status = 201, description = "Import job created", body = ImportJobResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_import_job(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Json(data): Json<CreatePortalImportJob>,
) -> Result<Json<ImportJobResponse>, (axum::http::StatusCode, String)> {
    let job = state
        .reality_portal_repo
        .create_import_job(principal.user_id, data)
        .await
        .map_err(|e| crate::util::errors::db_error("create import job", e))?;

    Ok(Json(ImportJobResponse { job }))
}

/// Get import job by ID.
#[utoipa::path(
    get,
    path = "/api/v1/imports/jobs/{id}",
    tag = "Imports",
    params(("id" = Uuid, Path, description = "Import job ID")),
    responses(
        (status = 200, description = "Import job details", body = ImportJobResponse),
        (status = 404, description = "Job not found")
    )
)]
pub async fn get_import_job(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<ImportJobResponse>, (axum::http::StatusCode, String)> {
    let job = state
        .reality_portal_repo
        .get_import_job(id, principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("get import job", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Import job not found".to_string(),
            )
        })?;

    Ok(Json(ImportJobResponse { job }))
}

/// Update import job.
#[utoipa::path(
    put,
    path = "/api/v1/imports/jobs/{id}",
    tag = "Imports",
    params(("id" = Uuid, Path, description = "Import job ID")),
    request_body = UpdatePortalImportJob,
    responses(
        (status = 200, description = "Job updated", body = ImportJobResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found")
    )
)]
pub async fn update_import_job(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdatePortalImportJob>,
) -> Result<Json<ImportJobResponse>, (axum::http::StatusCode, String)> {
    let job = state
        .reality_portal_repo
        .update_import_job(id, principal.user_id, data)
        .await
        .map_err(|e| match e {
            SqlxError::RowNotFound => (
                axum::http::StatusCode::NOT_FOUND,
                "Import job not found".to_string(),
            ),
            other => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update import job: {}", other),
            ),
        })?;

    Ok(Json(ImportJobResponse { job }))
}

/// Start processing an import job.
#[utoipa::path(
    post,
    path = "/api/v1/imports/jobs/{id}/start",
    tag = "Imports",
    params(("id" = Uuid, Path, description = "Import job ID")),
    responses(
        (status = 200, description = "Job started", body = ImportJobResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
        (status = 409, description = "Job already running or completed")
    )
)]
pub async fn start_import_job(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<ImportJobResponse>, (axum::http::StatusCode, String)> {
    let job = state
        .reality_portal_repo
        .start_import_job(id, principal.user_id)
        .await
        .map_err(|e| match e {
            SqlxError::RowNotFound => (
                axum::http::StatusCode::NOT_FOUND,
                "Import job not found".to_string(),
            ),
            other => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to start import job: {}", other),
            ),
        })?;

    Ok(Json(ImportJobResponse { job }))
}

/// Cancel an import job.
#[utoipa::path(
    post,
    path = "/api/v1/imports/jobs/{id}/cancel",
    tag = "Imports",
    params(("id" = Uuid, Path, description = "Import job ID")),
    responses(
        (status = 200, description = "Job cancelled", body = ImportJobResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found")
    )
)]
pub async fn cancel_import_job(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<ImportJobResponse>, (axum::http::StatusCode, String)> {
    let job = state
        .reality_portal_repo
        .cancel_import_job(id, principal.user_id)
        .await
        .map_err(|e| match e {
            SqlxError::RowNotFound => (
                axum::http::StatusCode::NOT_FOUND,
                "Import job not found".to_string(),
            ),
            other => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to cancel import job: {}", other),
            ),
        })?;

    Ok(Json(ImportJobResponse { job }))
}

/// List feed subscriptions.
#[utoipa::path(
    get,
    path = "/api/v1/imports/feeds",
    tag = "Imports",
    responses(
        (status = 200, description = "List of feeds", body = FeedsResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_feeds(
    State(state): State<AppState>,
    principal: PortalPrincipal,
) -> Result<Json<FeedsResponse>, (axum::http::StatusCode, String)> {
    let agency_id = resolve_agency(&state, principal.user_id).await?;
    let feeds = state
        .reality_portal_repo
        .list_feed_subscriptions(agency_id)
        .await
        .map_err(|e| crate::util::errors::db_error("list feed subscriptions", e))?;

    let total = feeds.len() as i64;

    Ok(Json(FeedsResponse { feeds, total }))
}

/// Create a new feed subscription.
#[utoipa::path(
    post,
    path = "/api/v1/imports/feeds",
    tag = "Imports",
    request_body = CreateFeedSubscription,
    responses(
        (status = 201, description = "Feed created", body = FeedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_feed(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Json(data): Json<CreateFeedSubscription>,
) -> Result<Json<FeedResponse>, (axum::http::StatusCode, String)> {
    let agency_id = resolve_agency(&state, principal.user_id).await?;
    let feed = state
        .reality_portal_repo
        .create_feed_subscription(agency_id, data)
        .await
        .map_err(|e| crate::util::errors::db_error("create feed subscription", e))?;

    Ok(Json(FeedResponse { feed }))
}

/// Get feed subscription by ID.
#[utoipa::path(
    get,
    path = "/api/v1/imports/feeds/{id}",
    tag = "Imports",
    params(("id" = Uuid, Path, description = "Feed ID")),
    responses(
        (status = 200, description = "Feed details", body = FeedResponse),
        (status = 404, description = "Feed not found")
    )
)]
pub async fn get_feed(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<FeedResponse>, (axum::http::StatusCode, String)> {
    let agency_id = resolve_agency(&state, principal.user_id).await?;
    let feed = state
        .reality_portal_repo
        .get_feed_subscription(id, agency_id)
        .await
        .map_err(|e| crate::util::errors::db_error("get feed subscription", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Feed not found".to_string(),
            )
        })?;

    Ok(Json(FeedResponse { feed }))
}

/// Update feed subscription.
#[utoipa::path(
    put,
    path = "/api/v1/imports/feeds/{id}",
    tag = "Imports",
    params(("id" = Uuid, Path, description = "Feed ID")),
    request_body = UpdateFeedSubscription,
    responses(
        (status = 200, description = "Feed updated", body = FeedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Feed not found")
    )
)]
pub async fn update_feed(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateFeedSubscription>,
) -> Result<Json<FeedResponse>, (axum::http::StatusCode, String)> {
    let agency_id = resolve_agency(&state, principal.user_id).await?;
    let feed = state
        .reality_portal_repo
        .update_feed_subscription(id, agency_id, data)
        .await
        .map_err(|e| match e {
            SqlxError::RowNotFound => (
                axum::http::StatusCode::NOT_FOUND,
                "Feed not found".to_string(),
            ),
            other => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update feed: {}", other),
            ),
        })?;

    Ok(Json(FeedResponse { feed }))
}

/// Trigger immediate sync for a feed.
#[utoipa::path(
    post,
    path = "/api/v1/imports/feeds/{id}/sync",
    tag = "Imports",
    params(("id" = Uuid, Path, description = "Feed ID")),
    responses(
        (status = 200, description = "Sync triggered", body = FeedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Feed not found")
    )
)]
pub async fn sync_feed(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<FeedResponse>, (axum::http::StatusCode, String)> {
    let agency_id = resolve_agency(&state, principal.user_id).await?;
    let feed = state
        .reality_portal_repo
        .trigger_feed_sync(id, agency_id)
        .await
        .map_err(|e| match e {
            SqlxError::RowNotFound => (
                axum::http::StatusCode::NOT_FOUND,
                "Feed not found".to_string(),
            ),
            other => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to trigger feed sync: {}", other),
            ),
        })?;

    Ok(Json(FeedResponse { feed }))
}
