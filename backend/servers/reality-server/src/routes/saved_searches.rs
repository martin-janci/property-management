//! Saved searches routes (Story 16.3).
//!
//! D1.2: handlers now use the unified `RequestPrincipal` extractor.

use crate::state::AppState;
use api_core::extractors::RequestPrincipal;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use db::models::{
    CreatePortalSavedSearch, PortalSavedSearch, PublicListingSummary, SavedSearchAlert,
    UpdatePortalSavedSearch,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Create saved searches router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_saved_searches))
        .route("/", post(create_saved_search))
        // Alert delivery (#983) — registered before the `/{id}` params; matchit
        // prioritises the static `alerts` segment, but keep them adjacent for clarity.
        .route("/alerts", get(list_search_alerts))
        .route("/alerts/read-all", post(mark_all_alerts_read))
        .route("/alerts/{alert_id}/read", post(mark_alert_read))
        .route("/{id}", get(get_saved_search))
        .route("/{id}", put(update_saved_search))
        .route("/{id}", delete(delete_saved_search))
        .route("/{id}/run", post(run_saved_search))
}

/// Saved searches list response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SavedSearchesResponse {
    pub searches: Vec<PortalSavedSearch>,
}

/// Run saved search response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RunSavedSearchResponse {
    pub count: i64,
    pub listings: Vec<PublicListingSummary>,
}

/// List user's saved searches.
#[utoipa::path(
    get,
    path = "/api/v1/saved-searches",
    tag = "SavedSearches",
    responses(
        (status = 200, description = "List of saved searches", body = SavedSearchesResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_saved_searches(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<SavedSearchesResponse>, (axum::http::StatusCode, String)> {
    let searches = state
        .reality_portal_repo
        .get_saved_searches(principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("list saved searches", e))?;

    Ok(Json(SavedSearchesResponse { searches }))
}

/// Create a saved search.
#[utoipa::path(
    post,
    path = "/api/v1/saved-searches",
    tag = "SavedSearches",
    request_body = CreatePortalSavedSearch,
    responses(
        (status = 201, description = "Saved search created", body = PortalSavedSearch),
        (status = 400, description = "Max searches reached"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_saved_search(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(data): Json<CreatePortalSavedSearch>,
) -> Result<Json<PortalSavedSearch>, (axum::http::StatusCode, String)> {
    let search = state
        .reality_portal_repo
        .create_saved_search(principal.user_id, data)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("maximum") || error_str.contains("limit") {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    "Maximum saved searches limit reached".to_string(),
                )
            } else {
                crate::util::errors::db_error("create saved search", e)
            }
        })?;

    Ok(Json(search))
}

/// Get a saved search by ID.
#[utoipa::path(
    get,
    path = "/api/v1/saved-searches/{id}",
    tag = "SavedSearches",
    params(("id" = Uuid, Path, description = "Saved search ID")),
    responses(
        (status = 200, description = "Saved search", body = PortalSavedSearch),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Saved search not found")
    )
)]
pub async fn get_saved_search(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<PortalSavedSearch>, (axum::http::StatusCode, String)> {
    // Single-row lookup scoped to the owning user. The previous code fetched
    // the entire saved-search collection and linear-scanned, which is both
    // wasteful and breaks at scale (functional bug for power users).
    let search = state
        .reality_portal_repo
        .get_saved_search_for_user(id, principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("get saved search", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Saved search not found".to_string(),
            )
        })?;

    Ok(Json(search))
}

/// Update a saved search.
#[utoipa::path(
    put,
    path = "/api/v1/saved-searches/{id}",
    tag = "SavedSearches",
    params(("id" = Uuid, Path, description = "Saved search ID")),
    request_body = UpdatePortalSavedSearch,
    responses(
        (status = 200, description = "Saved search updated", body = PortalSavedSearch),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Saved search not found")
    )
)]
pub async fn update_saved_search(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdatePortalSavedSearch>,
) -> Result<Json<PortalSavedSearch>, (axum::http::StatusCode, String)> {
    let search = state
        .reality_portal_repo
        .update_saved_search(id, principal.user_id, data)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("not found") || error_str.contains("RowNotFound") {
                (
                    axum::http::StatusCode::NOT_FOUND,
                    "Saved search not found".to_string(),
                )
            } else {
                crate::util::errors::db_error("update saved search", e)
            }
        })?;

    Ok(Json(search))
}

/// Delete a saved search.
#[utoipa::path(
    delete,
    path = "/api/v1/saved-searches/{id}",
    tag = "SavedSearches",
    params(("id" = Uuid, Path, description = "Saved search ID")),
    responses(
        (status = 204, description = "Saved search deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Saved search not found")
    )
)]
pub async fn delete_saved_search(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    state
        .reality_portal_repo
        .delete_saved_search(id, principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("delete saved search", e))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Run a saved search to get matching listings.
#[utoipa::path(
    post,
    path = "/api/v1/saved-searches/{id}/run",
    tag = "SavedSearches",
    params(("id" = Uuid, Path, description = "Saved search ID")),
    responses(
        (status = 200, description = "Search results", body = RunSavedSearchResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Saved search not found")
    )
)]
pub async fn run_saved_search(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<RunSavedSearchResponse>, (axum::http::StatusCode, String)> {
    // Single-row lookup scoped to the owning user (same fix as get_saved_search).
    let search = state
        .reality_portal_repo
        .get_saved_search_for_user(id, principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("get saved search", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Saved search not found".to_string(),
            )
        })?;

    // Parse the stored criteria JSON to a search query
    let query: db::models::PublicListingQuery = serde_json::from_value(search.criteria.clone())
        .map_err(|e| crate::util::errors::db_error("parse saved search criteria", e))?;

    // Execute the search using the portal repository. `count` must reflect
    // the total matching rows across all pages, not just the size of the
    // page we're returning — otherwise the client cannot tell whether
    // more pages exist. Run page + count concurrently against the same
    // query criteria.
    let (results, count) = tokio::try_join!(
        state.portal_repo.search_listings(&query),
        state.portal_repo.count_listings(&query),
    )
    .map_err(|e| crate::util::errors::db_error("run saved search", e))?;

    Ok(Json(RunSavedSearchResponse {
        count,
        listings: results,
    }))
}

// ============================================================================
// Saved-search alert delivery (Story 16.3, issue #983)
//
// The background `SavedSearchAlertWorker` matches alert-enabled saved searches
// against newly published listings and enqueues `search_alert_queue` rows. These
// endpoints are the in-app delivery surface: the user lists their alerts and acks
// them. (reality-server has no email transport; in-app is the delivery channel.)
// ============================================================================

/// Default and maximum page size for the alerts list (#1627).
const ALERTS_DEFAULT_LIMIT: i64 = 100;
const ALERTS_MAX_LIMIT: i64 = 200;

/// Pagination query for the saved-search alerts list.
#[derive(Debug, Deserialize)]
pub struct SearchAlertsQuery {
    /// Page size (default 100, capped at 200).
    #[serde(default)]
    pub limit: Option<i64>,
    /// Number of newest alerts to skip (default 0).
    #[serde(default)]
    pub offset: Option<i64>,
}

/// Saved-search alerts list response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchAlertsResponse {
    pub alerts: Vec<SavedSearchAlert>,
    /// Number of still-undelivered (`pending`) alerts — drives an unread badge.
    /// This is the total across all pages, so it can legitimately exceed the
    /// number of `alerts` rows returned for the current page.
    pub unread_count: i64,
    /// Page size applied (after clamping).
    pub limit: i64,
    /// Offset applied.
    pub offset: i64,
}

/// Mark-all-read response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MarkAllAlertsReadResponse {
    pub marked_read: u64,
}

/// List the authenticated user's saved-search alerts (newest first).
#[utoipa::path(
    get,
    path = "/api/v1/saved-searches/alerts",
    tag = "SavedSearches",
    responses(
        (status = 200, description = "Saved-search alerts", body = SearchAlertsResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_search_alerts(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(query): Query<SearchAlertsQuery>,
) -> Result<Json<SearchAlertsResponse>, (axum::http::StatusCode, String)> {
    let limit = query
        .limit
        .unwrap_or(ALERTS_DEFAULT_LIMIT)
        .clamp(1, ALERTS_MAX_LIMIT);
    let offset = crate::util::clamp_offset(query.offset);

    let (alerts, unread_count) = tokio::try_join!(
        state
            .reality_portal_repo
            .get_search_alerts(principal.user_id, limit, offset),
        state
            .reality_portal_repo
            .count_pending_search_alerts(principal.user_id),
    )
    .map_err(|e| crate::util::errors::db_error("list search alerts", e))?;

    Ok(Json(SearchAlertsResponse {
        alerts,
        unread_count,
        limit,
        offset,
    }))
}

/// Mark a single saved-search alert as read (delivered).
#[utoipa::path(
    post,
    path = "/api/v1/saved-searches/alerts/{alert_id}/read",
    tag = "SavedSearches",
    params(("alert_id" = Uuid, Path, description = "Alert queue ID")),
    responses(
        (status = 204, description = "Alert marked read"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Alert not found")
    )
)]
pub async fn mark_alert_read(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(alert_id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    let marked = state
        .reality_portal_repo
        .mark_search_alert_read(alert_id, principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("mark alert read", e))?;

    if marked {
        Ok(axum::http::StatusCode::NO_CONTENT)
    } else {
        Err((
            axum::http::StatusCode::NOT_FOUND,
            "Alert not found".to_string(),
        ))
    }
}

/// Mark all of the authenticated user's pending alerts as read.
#[utoipa::path(
    post,
    path = "/api/v1/saved-searches/alerts/read-all",
    tag = "SavedSearches",
    responses(
        (status = 200, description = "Pending alerts marked read", body = MarkAllAlertsReadResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn mark_all_alerts_read(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<MarkAllAlertsReadResponse>, (axum::http::StatusCode, String)> {
    let marked_read = state
        .reality_portal_repo
        .mark_all_search_alerts_read(principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("mark all alerts read", e))?;

    Ok(Json(MarkAllAlertsReadResponse { marked_read }))
}
