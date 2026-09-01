//! Favorites routes - save and manage favorite listings (Story 16.2).
//!
//! D1.2: all endpoints in this file now adopt the unified `RequestPrincipal`
//! / `OptionalRequestPrincipal` extractors (Phase 2 identity stack). The
//! `list_favorite_ids` SSR-anonymous path uses the optional wrapper so a
//! missing Authorization header yields an empty list, while a malformed or
//! revoked token still surfaces as 401/403 (do NOT silently demote — see
//! leak #10/#11 defenses on the wrapper).
//!
//! Behavior contract for the auth-required endpoints (list/add/remove/check):
//!
//! * No bearer token / invalid token / unknown user → 401.
//! * Token for a non-platform principal arriving at a tenant-scoped host
//!   without an active membership → 403 (leak #11 cross-tenant rejection).
//! * Otherwise → 2xx with the response payload.
//!
//! We deliberately do NOT consult `principal.effective_org` here: favorites
//! are user-scoped, not org-scoped, and the listing rows are read via the
//! public listings index. The principal is used purely as the
//! authenticated-user identifier.

use crate::state::AppState;
use api_core::extractors::{OptionalRequestPrincipal, RequestPrincipal};
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
    Json, Router,
};
use db::models::{
    AddFavorite, FavoriteAlert, PortalFavorite, PortalFavoriteWithListing, UpdatePortalFavorite,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Create favorites router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_favorites))
        .route("/alerts", get(list_favorite_alerts))
        .route("/alerts/read-all", post(mark_all_favorite_alerts_read))
        .route("/alerts/{alert_id}/read", post(mark_favorite_alert_read))
        // Static `/ids` declared before `/{listing_id}` so axum's matcher
        // prefers the literal segment over the UUID capture.
        .route("/ids", get(list_favorite_ids))
        .route("/{listing_id}", post(add_favorite))
        .route("/{listing_id}", patch(update_favorite))
        .route("/{listing_id}", delete(remove_favorite))
        .route("/{listing_id}/check", get(check_favorite))
}

/// Favorites list response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FavoritesResponse {
    pub favorites: Vec<PortalFavoriteWithListing>,
}

/// Check favorite response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CheckFavoriteResponse {
    pub is_favorited: bool,
}

const ALERTS_DEFAULT_LIMIT: i64 = 100;
const ALERTS_MAX_LIMIT: i64 = 200;

#[derive(Debug, Deserialize)]
pub struct FavoriteAlertsQuery {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FavoriteAlertsResponse {
    pub alerts: Vec<FavoriteAlert>,
    pub unread_count: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MarkAllFavoriteAlertsReadResponse {
    pub marked_read: u64,
}

/// List user's favorites.
///
/// Uses the unified [`RequestPrincipal`] extractor (see file-level docs for
/// the full auth/membership contract). Favorites are user-scoped — the
/// principal is consulted purely as the authenticated-user identifier; we
/// do NOT branch on `principal.effective_org` here.
#[utoipa::path(
    get,
    path = "/api/v1/favorites",
    tag = "Favorites",
    responses(
        (status = 200, description = "List of favorites", body = FavoritesResponse),
        (status = 401, description = "Unauthorized — missing or invalid token"),
        (status = 403, description = "Forbidden — host/principal mismatch (leak #11 defense)")
    )
)]
pub async fn list_favorites(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<FavoritesResponse>, (axum::http::StatusCode, String)> {
    let favorites = state
        .reality_portal_repo
        .get_favorites_with_listings(principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("list favorites", e))?;

    Ok(Json(FavoritesResponse { favorites }))
}

/// Add listing to favorites.
#[utoipa::path(
    post,
    path = "/api/v1/favorites/{listing_id}",
    tag = "Favorites",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    request_body = AddFavorite,
    responses(
        (status = 201, description = "Added to favorites", body = PortalFavorite),
        (status = 400, description = "Max favorites reached"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Listing not found")
    )
)]
pub async fn add_favorite(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(listing_id): Path<Uuid>,
    Json(data): Json<AddFavorite>,
) -> Result<Json<PortalFavorite>, (axum::http::StatusCode, String)> {
    let favorite = state
        .reality_portal_repo
        .add_favorite(principal.user_id, listing_id, data.notes)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("not found") {
                (
                    axum::http::StatusCode::NOT_FOUND,
                    "Listing not found".to_string(),
                )
            } else if error_str.contains("maximum") || error_str.contains("limit") {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    "Maximum favorites limit reached".to_string(),
                )
            } else {
                crate::util::errors::db_error("add favorite", e)
            }
        })?;

    Ok(Json(favorite))
}

/// Update a favorite's settings (notes / price-alert opt-in).
///
/// Completes AC-5 of story 84.3: lets a user subscribe or unsubscribe from
/// price-change alerts on a favorited listing by toggling
/// `price_alert_enabled`. Both fields are `COALESCE`-merged server-side, so a
/// partial body (e.g. just `{"price_alert_enabled": false}`) leaves `notes`
/// untouched. Favorites are user-scoped — the principal is the owning-user
/// identifier only (see file-level docs).
#[utoipa::path(
    patch,
    path = "/api/v1/favorites/{listing_id}",
    tag = "Favorites",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    request_body = UpdatePortalFavorite,
    responses(
        (status = 200, description = "Updated favorite", body = PortalFavorite),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Favorite not found")
    )
)]
pub async fn update_favorite(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(listing_id): Path<Uuid>,
    Json(data): Json<UpdatePortalFavorite>,
) -> Result<Json<PortalFavorite>, (axum::http::StatusCode, String)> {
    let favorite = state
        .reality_portal_repo
        .update_favorite(principal.user_id, listing_id, data)
        .await
        .map_err(|e| {
            // `fetch_one` yields RowNotFound ("no rows") when the user has no
            // such favorite — surface that as 404 rather than a 500.
            if e.to_string().contains("no rows") {
                (
                    axum::http::StatusCode::NOT_FOUND,
                    "Favorite not found".to_string(),
                )
            } else {
                crate::util::errors::db_error("update favorite", e)
            }
        })?;

    Ok(Json(favorite))
}

/// Remove listing from favorites.
#[utoipa::path(
    delete,
    path = "/api/v1/favorites/{listing_id}",
    tag = "Favorites",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 204, description = "Removed from favorites"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Favorite not found")
    )
)]
pub async fn remove_favorite(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(listing_id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    state
        .reality_portal_repo
        .remove_favorite(principal.user_id, listing_id)
        .await
        .map_err(|e| crate::util::errors::db_error("remove favorite", e))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Check if listing is favorited.
#[utoipa::path(
    get,
    path = "/api/v1/favorites/{listing_id}/check",
    tag = "Favorites",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Favorite status", body = CheckFavoriteResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn check_favorite(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<CheckFavoriteResponse>, (axum::http::StatusCode, String)> {
    // Single-row existence query (was previously a full list-then-filter which
    // silently lost favorites past the implicit fetch cap for power users).
    let is_favorited = state
        .reality_portal_repo
        .is_favorite(principal.user_id, listing_id)
        .await
        .map_err(|e| crate::util::errors::db_error("check favorite", e))?;

    Ok(Json(CheckFavoriteResponse { is_favorited }))
}

/// List the authenticated user's favorite alerts (newest first).
#[utoipa::path(
    get,
    path = "/api/v1/favorites/alerts",
    tag = "Favorites",
    responses(
        (status = 200, description = "Favorite alerts", body = FavoriteAlertsResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_favorite_alerts(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(query): Query<FavoriteAlertsQuery>,
) -> Result<Json<FavoriteAlertsResponse>, (axum::http::StatusCode, String)> {
    let limit = query
        .limit
        .unwrap_or(ALERTS_DEFAULT_LIMIT)
        .clamp(1, ALERTS_MAX_LIMIT);
    let offset = crate::util::clamp_offset(query.offset);

    let (alerts, unread_count) = tokio::try_join!(
        state
            .reality_portal_repo
            .get_favorite_alerts(principal.user_id, limit, offset),
        state
            .reality_portal_repo
            .count_pending_favorite_alerts(principal.user_id),
    )
    .map_err(|e| crate::util::errors::db_error("list favorite alerts", e))?;

    Ok(Json(FavoriteAlertsResponse {
        alerts,
        unread_count,
        limit,
        offset,
    }))
}

/// Mark a single favorite alert as read.
#[utoipa::path(
    post,
    path = "/api/v1/favorites/alerts/{alert_id}/read",
    tag = "Favorites",
    params(("alert_id" = Uuid, Path, description = "Favorite alert queue ID")),
    responses(
        (status = 204, description = "Favorite alert marked read"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Alert not found")
    )
)]
pub async fn mark_favorite_alert_read(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(alert_id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    let outcome = state
        .reality_portal_repo
        .mark_favorite_alert_read(alert_id, principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("mark favorite alert read", e))?;

    mark_read_outcome_status(outcome)
}

/// Map a [`db::repositories::FavoriteAlertReadOutcome`] to the HTTP result the
/// `mark_favorite_alert_read` route returns.
///
/// Extracted as a pure, DB-free function so the idempotency contract (#1852
/// finding-4) has a fast regression guard that actually runs on the PR gate
/// (#1999 finding-2) while the DB-backed
/// `mark_favorite_alert_read_is_idempotent` integration test stays quarantined
/// under BIT-351.
///
/// Idempotent: a freshly-flipped alert AND one already read both succeed with
/// 204 — a client polling read-state shouldn't get a surprising 404 for an
/// alert it already marked. Genuinely absent / not-owned still 404s (no
/// existence leak).
fn mark_read_outcome_status(
    outcome: db::repositories::FavoriteAlertReadOutcome,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    use db::repositories::FavoriteAlertReadOutcome;
    match outcome {
        FavoriteAlertReadOutcome::Flipped | FavoriteAlertReadOutcome::AlreadyRead => {
            Ok(axum::http::StatusCode::NO_CONTENT)
        }
        FavoriteAlertReadOutcome::NotFound => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Alert not found".to_string(),
        )),
    }
}

/// Mark all of the authenticated user's pending favorite alerts as read.
#[utoipa::path(
    post,
    path = "/api/v1/favorites/alerts/read-all",
    tag = "Favorites",
    responses(
        (status = 200, description = "Favorite alerts marked read", body = MarkAllFavoriteAlertsReadResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn mark_all_favorite_alerts_read(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<MarkAllFavoriteAlertsReadResponse>, (axum::http::StatusCode, String)> {
    let marked_read = state
        .reality_portal_repo
        .mark_all_favorite_alerts_read(principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("mark all favorite alerts read", e))?;

    Ok(Json(MarkAllFavoriteAlertsReadResponse { marked_read }))
}

/// List the listing IDs that the current user has favorited.
///
/// GET `/api/v1/favorites/ids`. Returns a flat array of listing UUIDs so the
/// reality-web client can hydrate `isFavorite` flags on cards in O(1) without
/// fetching the full favorite payload. Returns an empty array when the user
/// is unauthenticated rather than 401, since this endpoint is also called
/// from anonymous SSR pages and shouldn't surface auth errors there.
#[utoipa::path(
    get,
    path = "/api/v1/favorites/ids",
    tag = "Favorites",
    responses(
        (status = 200, description = "List of favorited listing IDs", body = [Uuid])
    )
)]
// D1.1/D1.2: now uses `OptionalRequestPrincipal`. Behavior:
//   * No Authorization header → empty list (anonymous SSR path stays quiet).
//   * Authorization present but invalid/forbidden → propagated 401/403 by
//     the wrapper. Do NOT silently demote to "anonymous + empty list" — a
//     stolen-but-revoked token must be told "no", not given an indistinguish-
//     able response from a real anonymous request (leak #10/#11 defense).
pub async fn list_favorite_ids(
    State(state): State<AppState>,
    OptionalRequestPrincipal(principal): OptionalRequestPrincipal,
) -> Result<Json<Vec<Uuid>>, (axum::http::StatusCode, String)> {
    let Some(principal) = principal else {
        return Ok(Json(Vec::new()));
    };

    let favorites = state
        .reality_portal_repo
        .get_favorites_with_listings(principal.user_id)
        .await
        .map_err(|e| {
            tracing::error!(user_id = %principal.user_id, error = %e, "Failed to list favorite IDs");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            )
        })?;

    Ok(Json(favorites.into_iter().map(|f| f.listing_id).collect()))
}

#[cfg(test)]
mod tests {
    use super::mark_read_outcome_status;
    use axum::http::StatusCode;
    use db::repositories::FavoriteAlertReadOutcome;

    // #1852 finding-4 / #1999 finding-2: the mark-read route is idempotent.
    // These assert the outcome→status mapping directly, without a database, so
    // the contract is guarded on every PR even while the DB-backed
    // `mark_favorite_alert_read_is_idempotent` test stays quarantined (BIT-351).

    #[test]
    fn flipped_maps_to_204() {
        assert_eq!(
            mark_read_outcome_status(FavoriteAlertReadOutcome::Flipped),
            Ok(StatusCode::NO_CONTENT)
        );
    }

    #[test]
    fn already_read_maps_to_204_not_404() {
        // The core of finding-4: a second mark on an already-read alert is a
        // success, not a surprising 404.
        assert_eq!(
            mark_read_outcome_status(FavoriteAlertReadOutcome::AlreadyRead),
            Ok(StatusCode::NO_CONTENT)
        );
    }

    #[test]
    fn not_found_maps_to_404() {
        let err = mark_read_outcome_status(FavoriteAlertReadOutcome::NotFound)
            .expect_err("NotFound must map to an error status");
        assert_eq!(err.0, StatusCode::NOT_FOUND);
    }
}
