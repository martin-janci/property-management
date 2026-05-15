//! Favorites routes - save and manage favorite listings (Story 16.2).
//!
//! N1 PROOF POINT: `GET /api/v1/favorites` is the canary endpoint that adopts
//! the unified `RequestPrincipal` extractor (Phase 2 identity stack) on
//! reality-server. The other endpoints in this file still use the legacy
//! `AuthenticatedUser` extractor — see the `// TODO(N1-followup):` markers.
//! Migrating one endpoint proves the wiring (state impl, JWT shape,
//! membership lookup against PlatformHost) without doing a sprint of work
//! in this PR.

use crate::extractors::{AuthenticatedUser, OptionalAuth};
use crate::state::AppState;
use api_core::extractors::RequestPrincipal;
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use db::models::{AddFavorite, PortalFavorite, PortalFavoriteWithListing};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Create favorites router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_favorites))
        // Static `/ids` declared before `/{listing_id}` so axum's matcher
        // prefers the literal segment over the UUID capture.
        .route("/ids", get(list_favorite_ids))
        .route("/{listing_id}", post(add_favorite))
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

/// List user's favorites.
///
/// N1 PROOF POINT: this endpoint extracts [`RequestPrincipal`] instead of
/// the legacy [`AuthenticatedUser`]. Behavior contract:
///
/// * No bearer token / invalid token / unknown user → 401 (the extractor
///   itself emits these — `Missing Authorization header`,
///   `Invalid or expired token`, `Unknown principal`).
/// * Token for a public principal whose host-resolved org has no membership
///   for them → 403. Reality-server's host resolution typically yields the
///   `PlatformHost` variant, in which case the extractor allows public
///   principals through with `effective_org = None` (the public portal does
///   not require an org membership).
/// * Token for a non-platform principal arriving at a tenant-scoped host
///   without an active membership → 403 ("no active membership in this
///   organization") — the desired cross-tenant rejection from leak #11.
/// * Otherwise → 200 with the favorites payload.
///
/// We deliberately do NOT consult `principal.effective_org` here: favorites
/// are user-scoped, not org-scoped, and the listing rows are read via the
/// public listings index. The principal is used purely as the
/// authenticated-user identifier.
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
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list favorites: {}", e),
            )
        })?;

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
// TODO(N1-followup): migrate to RequestPrincipal (Phase 2 unified identity).
pub async fn add_favorite(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(listing_id): Path<Uuid>,
    Json(data): Json<AddFavorite>,
) -> Result<Json<PortalFavorite>, (axum::http::StatusCode, String)> {
    let favorite = state
        .reality_portal_repo
        .add_favorite(auth.user_id, listing_id, data.notes)
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
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to add favorite: {}", e),
                )
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
// TODO(N1-followup): migrate to RequestPrincipal (Phase 2 unified identity).
pub async fn remove_favorite(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(listing_id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    state
        .reality_portal_repo
        .remove_favorite(auth.user_id, listing_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to remove favorite: {}", e),
            )
        })?;

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
// TODO(N1-followup): migrate to RequestPrincipal (Phase 2 unified identity).
pub async fn check_favorite(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<CheckFavoriteResponse>, (axum::http::StatusCode, String)> {
    // Check if the user has this listing in favorites by getting favorites and checking
    let favorites = state
        .reality_portal_repo
        .get_favorites_with_listings(auth.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to check favorite: {}", e),
            )
        })?;

    let is_favorited = favorites.iter().any(|f| f.listing_id == listing_id);

    Ok(Json(CheckFavoriteResponse { is_favorited }))
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
// TODO(N1-followup): migrate to an Optional<RequestPrincipal> wrapper once
// the extractor grows an `OptionalRequestPrincipal` form. Today the legacy
// OptionalAuth path is kept for the SSR anonymous case.
pub async fn list_favorite_ids(
    State(state): State<AppState>,
    OptionalAuth(auth): OptionalAuth,
) -> Result<Json<Vec<Uuid>>, (axum::http::StatusCode, String)> {
    let Some(auth) = auth else {
        return Ok(Json(Vec::new()));
    };

    let favorites = state
        .reality_portal_repo
        .get_favorites_with_listings(auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(user_id = %auth.user_id, error = %e, "Failed to list favorite IDs");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            )
        })?;

    Ok(Json(favorites.into_iter().map(|f| f.listing_id).collect()))
}
