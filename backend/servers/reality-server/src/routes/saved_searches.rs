//! Saved searches routes (Story 16.3).
// TODO(N1-followup): migrate AuthenticatedUser → RequestPrincipal (Phase 2 unified identity).

use crate::extractors::AuthenticatedUser;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use db::models::{
    CreatePortalSavedSearch, PortalSavedSearch, PublicListingSummary, UpdatePortalSavedSearch,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Create saved searches router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_saved_searches))
        .route("/", post(create_saved_search))
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
    auth: AuthenticatedUser,
) -> Result<Json<SavedSearchesResponse>, (axum::http::StatusCode, String)> {
    let searches = state
        .reality_portal_repo
        .get_saved_searches(auth.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list saved searches: {}", e),
            )
        })?;

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
    auth: AuthenticatedUser,
    Json(data): Json<CreatePortalSavedSearch>,
) -> Result<Json<PortalSavedSearch>, (axum::http::StatusCode, String)> {
    let search = state
        .reality_portal_repo
        .create_saved_search(auth.user_id, data)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("maximum") || error_str.contains("limit") {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    "Maximum saved searches limit reached".to_string(),
                )
            } else {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to create saved search: {}", e),
                )
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
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PortalSavedSearch>, (axum::http::StatusCode, String)> {
    // Get all saved searches for user and filter by id (since repository doesn't have get_by_id)
    let searches = state
        .reality_portal_repo
        .get_saved_searches(auth.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get saved search: {}", e),
            )
        })?;

    let search = searches.into_iter().find(|s| s.id == id).ok_or_else(|| {
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
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdatePortalSavedSearch>,
) -> Result<Json<PortalSavedSearch>, (axum::http::StatusCode, String)> {
    let search = state
        .reality_portal_repo
        .update_saved_search(id, auth.user_id, data)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("not found") || error_str.contains("RowNotFound") {
                (
                    axum::http::StatusCode::NOT_FOUND,
                    "Saved search not found".to_string(),
                )
            } else {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to update saved search: {}", e),
                )
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
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    state
        .reality_portal_repo
        .delete_saved_search(id, auth.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete saved search: {}", e),
            )
        })?;

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
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RunSavedSearchResponse>, (axum::http::StatusCode, String)> {
    // First get the saved search to verify ownership and get criteria
    let searches = state
        .reality_portal_repo
        .get_saved_searches(auth.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get saved search: {}", e),
            )
        })?;

    let search = searches.into_iter().find(|s| s.id == id).ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            "Saved search not found".to_string(),
        )
    })?;

    // Parse the stored criteria JSON to a search query
    let query: db::models::PublicListingQuery = serde_json::from_value(search.criteria.clone())
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to parse saved search criteria: {}", e),
            )
        })?;

    // Execute the search using the portal repository
    let results = state
        .portal_repo
        .search_listings(&query)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to run saved search: {}", e),
            )
        })?;

    Ok(Json(RunSavedSearchResponse {
        count: results.len() as i64,
        listings: results,
    }))
}
