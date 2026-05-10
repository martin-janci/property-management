//! Compare routes (UC-48: Compare Listings).
//!
//! Allows authenticated portal users to maintain a compare list of up to 4 listings.

use crate::extractors::AuthenticatedUser;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Maximum number of listings in a compare list.
pub const MAX_COMPARE_LISTINGS: i64 = 4;

/// Create compare router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_compare_list))
        .route("/:listing_id", post(add_to_compare))
        .route("/:listing_id", delete(remove_from_compare))
}

/// A single entry in the compare list with denormalised listing fields.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CompareEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub listing_id: Uuid,
    pub added_at: DateTime<Utc>,
    pub title: Option<String>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub city: Option<String>,
    pub property_type: Option<String>,
    pub transaction_type: Option<String>,
    /// Floor area in sqm (`size_sqm` in listings table).
    pub size_sqm: Option<f64>,
    pub rooms: Option<i32>,
    pub photo_url: Option<String>,
    pub status: Option<String>,
}

/// Compare list response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CompareListResponse {
    pub entries: Vec<CompareEntry>,
    pub count: usize,
}

/// Response after adding a listing to the compare list.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddCompareResponse {
    pub listing_id: Uuid,
    pub message: String,
}

/// Get the current user's compare list (up to 4 listings with full detail).
#[utoipa::path(
    get,
    path = "/api/v1/compare",
    tag = "Compare",
    responses(
        (status = 200, description = "Compare list", body = CompareListResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("session_token" = []))
)]
pub async fn get_compare_list(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<CompareListResponse>, (axum::http::StatusCode, String)> {
    let mut conn = state.acquire_public_conn().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
    })?;

    let rows = sqlx::query(
        r#"
        SELECT
            cl.id,
            cl.user_id,
            cl.listing_id,
            cl.added_at,
            l.title,
            l.price::float8 AS price,
            l.currency,
            l.city,
            l.property_type,
            l.transaction_type,
            l.size_sqm::float8 AS size_sqm,
            l.rooms,
            (SELECT url FROM listing_photos WHERE listing_id = l.id ORDER BY display_order ASC LIMIT 1) AS photo_url,
            l.status
        FROM compare_lists cl
        LEFT JOIN listings l ON l.id = cl.listing_id
        WHERE cl.user_id = $1
        ORDER BY cl.added_at ASC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch compare list: {}", e),
        )
    })?;

    use sqlx::Row;
    let entries: Vec<CompareEntry> = rows
        .into_iter()
        .map(|r| CompareEntry {
            id: r.get("id"),
            user_id: r.get("user_id"),
            listing_id: r.get("listing_id"),
            added_at: r.get("added_at"),
            title: r.get("title"),
            price: r.get("price"),
            currency: r.get("currency"),
            city: r.get("city"),
            property_type: r.get("property_type"),
            transaction_type: r.get("transaction_type"),
            size_sqm: r.get("size_sqm"),
            rooms: r.get("rooms"),
            photo_url: r.get("photo_url"),
            status: r.get("status"),
        })
        .collect();

    let count = entries.len();
    Ok(Json(CompareListResponse { entries, count }))
}

/// Add a listing to the compare list (max 4).
#[utoipa::path(
    post,
    path = "/api/v1/compare/{listing_id}",
    tag = "Compare",
    params(("listing_id" = Uuid, Path, description = "Listing ID to add")),
    responses(
        (status = 201, description = "Added to compare list", body = AddCompareResponse),
        (status = 400, description = "Compare list is full (max 4) or listing already in list"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Listing not found")
    ),
    security(("session_token" = []))
)]
pub async fn add_to_compare(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(listing_id): Path<Uuid>,
) -> Result<(axum::http::StatusCode, Json<AddCompareResponse>), (axum::http::StatusCode, String)> {
    let mut conn = state.acquire_public_conn().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
    })?;

    // Check current count
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM compare_lists WHERE user_id = $1")
        .bind(auth.user_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count compare entries: {}", e),
            )
        })?;

    if count >= MAX_COMPARE_LISTINGS {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!(
                "Compare list is full (maximum {} listings)",
                MAX_COMPARE_LISTINGS
            ),
        ));
    }

    // Check listing exists
    let listing_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM listings WHERE id = $1)")
            .bind(listing_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to check listing: {}", e),
                )
            })?;

    if !listing_exists {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Listing not found".to_string(),
        ));
    }

    // Check duplicate
    let already_in: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM compare_lists WHERE user_id = $1 AND listing_id = $2)",
    )
    .bind(auth.user_id)
    .bind(listing_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to check compare list: {}", e),
        )
    })?;

    if already_in {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Listing already in compare list".to_string(),
        ));
    }

    sqlx::query("INSERT INTO compare_lists (user_id, listing_id) VALUES ($1, $2)")
        .bind(auth.user_id)
        .bind(listing_id)
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to add to compare list: {}", e),
            )
        })?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(AddCompareResponse {
            listing_id,
            message: "Added to compare list".to_string(),
        }),
    ))
}

/// Remove a listing from the compare list.
#[utoipa::path(
    delete,
    path = "/api/v1/compare/{listing_id}",
    tag = "Compare",
    params(("listing_id" = Uuid, Path, description = "Listing ID to remove")),
    responses(
        (status = 204, description = "Removed from compare list"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not in compare list")
    ),
    security(("session_token" = []))
)]
pub async fn remove_from_compare(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(listing_id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    let mut conn = state.acquire_public_conn().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
    })?;

    let rows_affected =
        sqlx::query("DELETE FROM compare_lists WHERE user_id = $1 AND listing_id = $2")
            .bind(auth.user_id)
            .bind(listing_id)
            .execute(&mut *conn)
            .await
            .map_err(|e| {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to remove from compare list: {}", e),
                )
            })?
            .rows_affected();

    if rows_affected == 0 {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Listing not in compare list".to_string(),
        ));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}
