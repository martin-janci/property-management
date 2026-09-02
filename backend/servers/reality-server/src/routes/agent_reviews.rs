//! Agent review routes (UC-49, UC-51: Realtor Reviews).
//!
//! Allows portal users to review realtors. The "verified buyer" flag is set
//! server-side based on whether the reviewer has a prior responded inquiry with
//! that realtor.
//! D1.2: handlers now use the unified `RequestPrincipal` extractor.

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

/// Maximum length (in bytes) of a review `body`.
///
/// Without a cap, an authenticated user can post an arbitrary-size string that
/// bloats DB rows, log files and the request body (multi-MiB blobs observed).
/// 5000 mirrors the existing report-description / inquiry-message caps in this
/// server so free-text limits stay consistent.
const MAX_REVIEW_BODY_LEN: usize = 5000;

/// Guard: a realtor must not review their own profile.
///
/// `reviewer_user_id` is the authenticated principal posting the review;
/// `realtor_user_id` is the user account that owns the realtor profile being
/// reviewed. When they are the same account the request is a self-review — an
/// agent grading their own reviews — which is rejected with `403 Forbidden`.
/// Without this gate the `subject != reviewer` invariant was unenforced.
fn reject_self_review(
    reviewer_user_id: Uuid,
    realtor_user_id: Uuid,
) -> Result<(), (axum::http::StatusCode, String)> {
    if reviewer_user_id == realtor_user_id {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            "You cannot review your own realtor profile".to_string(),
        ));
    }
    Ok(())
}

/// Create agent reviews router.
/// Mounted at /api/v1/realtors/:id/reviews via main.rs.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_reviews))
        .route("/", post(create_review))
}

/// A single realtor review.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RealtorReview {
    pub id: Uuid,
    pub realtor_id: Uuid,
    pub reviewer_user_id: Uuid,
    pub reviewer_name: String,
    pub rating: i32,
    pub body: Option<String>,
    pub verified_buyer: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create review request.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateReviewRequest {
    /// Rating from 1 (worst) to 5 (best).
    pub rating: i32,
    pub body: Option<String>,
}

/// Paginated reviews response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReviewsResponse {
    pub reviews: Vec<RealtorReview>,
    pub total: i64,
    pub avg_rating: Option<f64>,
}

/// Reviews query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ReviewsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// List reviews for a realtor.
#[utoipa::path(
    get,
    path = "/api/v1/realtors/{id}/reviews",
    tag = "AgentReviews",
    params(
        ("id" = Uuid, Path, description = "Realtor profile ID"),
        ReviewsQuery
    ),
    responses(
        (status = 200, description = "List of reviews", body = ReviewsResponse),
        (status = 404, description = "Realtor not found")
    )
)]
pub async fn list_reviews(
    State(state): State<AppState>,
    Path(realtor_id): Path<Uuid>,
    Query(query): Query<ReviewsQuery>,
) -> Result<Json<ReviewsResponse>, (axum::http::StatusCode, String)> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = crate::util::clamp_offset(query.offset);

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    // Verify realtor exists
    let realtor_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM realtor_profiles WHERE id = $1)")
            .bind(realtor_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| crate::util::errors::db_error("check realtor", e))?;

    if !realtor_exists {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Realtor not found".to_string(),
        ));
    }

    let rows = sqlx::query(
        r#"
        SELECT
            rr.id,
            rr.realtor_id,
            rr.reviewer_user_id,
            COALESCE(pu.name, 'Anonymous') AS reviewer_name,
            rr.rating,
            rr.body,
            rr.verified_buyer,
            rr.created_at,
            rr.updated_at
        FROM realtor_reviews rr
        LEFT JOIN users pu ON pu.id = rr.reviewer_user_id AND pu.principal_kind = 'public'
        WHERE rr.realtor_id = $1
        ORDER BY rr.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(realtor_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("list reviews", e))?;

    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM realtor_reviews WHERE realtor_id = $1")
            .bind(realtor_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| crate::util::errors::db_error("count reviews", e))?;

    let avg_rating: Option<f64> =
        sqlx::query_scalar("SELECT AVG(rating::float8) FROM realtor_reviews WHERE realtor_id = $1")
            .bind(realtor_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(|e| crate::util::errors::db_error("compute avg rating", e))?;

    let reviews: Vec<RealtorReview> = rows
        .into_iter()
        .map(|row| RealtorReview {
            id: row.get("id"),
            realtor_id: row.get("realtor_id"),
            reviewer_user_id: row.get("reviewer_user_id"),
            reviewer_name: row
                .try_get::<String, _>("reviewer_name")
                .unwrap_or_else(|_| "Anonymous".to_string()),
            rating: row.get("rating"),
            body: row.get("body"),
            verified_buyer: row.get("verified_buyer"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect();

    Ok(Json(ReviewsResponse {
        reviews,
        total,
        avg_rating,
    }))
}

/// Create a review for a realtor.
///
/// `verified_buyer` is set server-side: true when the authenticated user has
/// at least one responded inquiry for a listing owned by this realtor.
#[utoipa::path(
    post,
    path = "/api/v1/realtors/{id}/reviews",
    tag = "AgentReviews",
    params(("id" = Uuid, Path, description = "Realtor profile ID")),
    request_body = CreateReviewRequest,
    responses(
        (status = 201, description = "Review created", body = RealtorReview),
        (status = 400, description = "Invalid rating or already reviewed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Cannot review your own realtor profile"),
        (status = 404, description = "Realtor not found")
    ),
    security(("session_token" = []))
)]
pub async fn create_review(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(realtor_id): Path<Uuid>,
    Json(data): Json<CreateReviewRequest>,
) -> Result<(axum::http::StatusCode, Json<RealtorReview>), (axum::http::StatusCode, String)> {
    if data.rating < 1 || data.rating > 5 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Rating must be between 1 and 5".to_string(),
        ));
    }

    if let Some(body) = data.body.as_deref() {
        if body.chars().count() > MAX_REVIEW_BODY_LEN {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!("Review body must be at most {MAX_REVIEW_BODY_LEN} characters"),
            ));
        }
    }

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    // Verify realtor exists and fetch the owning user account so we can reject
    // self-reviews (an agent reviewing their own profile).
    let realtor_user_id: Option<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM realtor_profiles WHERE id = $1")
            .bind(realtor_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| crate::util::errors::db_error("check realtor", e))?;

    let realtor_user_id = match realtor_user_id {
        Some(uid) => uid,
        None => {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "Realtor not found".to_string(),
            ))
        }
    };

    // Reject self-reviews: the reviewer must not be the profile's own account.
    reject_self_review(principal.user_id, realtor_user_id)?;

    // Determine verified_buyer: check for a prior responded inquiry with this realtor.
    // listing_inquiries.realtor_id references portal_users.id (the realtor's user_id).
    // We match against realtor_profiles.user_id to find the portal user for the realtor.
    let verified_buyer: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM listing_inquiries li
            JOIN realtor_profiles rp ON rp.user_id = li.realtor_id
            WHERE li.user_id = $1 AND rp.id = $2 AND li.responded_at IS NOT NULL
        )
        "#,
    )
    .bind(principal.user_id)
    .bind(realtor_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("check verified buyer status", e))?;

    // Phase 6: reads from `users` (portal_users dropped in migration 00148).
    let reviewer_name: String =
        sqlx::query_scalar("SELECT name FROM users WHERE id = $1 AND principal_kind = 'public'")
            .bind(principal.user_id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(|e| crate::util::errors::db_error("get reviewer name", e))?
            .flatten()
            .unwrap_or_else(|| "Anonymous".to_string());

    // Insert review; unique constraint prevents duplicates
    let row = sqlx::query(
        r#"
        INSERT INTO realtor_reviews
            (realtor_id, reviewer_user_id, rating, body, verified_buyer)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (realtor_id, reviewer_user_id) DO NOTHING
        RETURNING id, realtor_id, reviewer_user_id, rating, body, verified_buyer,
                  created_at, updated_at
        "#,
    )
    .bind(realtor_id)
    .bind(principal.user_id)
    .bind(data.rating)
    .bind(&data.body)
    .bind(verified_buyer)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("create review", e))?;

    match row {
        Some(r) => {
            let review = RealtorReview {
                id: r.get("id"),
                realtor_id: r.get("realtor_id"),
                reviewer_user_id: r.get("reviewer_user_id"),
                reviewer_name,
                rating: r.get("rating"),
                body: r.get("body"),
                verified_buyer: r.get("verified_buyer"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            };
            Ok((axum::http::StatusCode::CREATED, Json(review)))
        }
        None => Err((
            axum::http::StatusCode::BAD_REQUEST,
            "You have already reviewed this realtor".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    //! Guard the self-review invariant without a database. `create_review`
    //! delegates the `subject != reviewer` decision to [`reject_self_review`],
    //! so pinning that helper covers the rejection contract on every PR.
    use super::reject_self_review;
    use axum::http::StatusCode;
    use uuid::Uuid;

    #[test]
    fn self_review_is_rejected_with_403() {
        let account = Uuid::new_v4();
        // Reviewer and realtor profile owner are the same account => self-review.
        let err = reject_self_review(account, account)
            .expect_err("a realtor reviewing their own profile must be rejected");
        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    #[test]
    fn distinct_accounts_pass() {
        let reviewer = Uuid::new_v4();
        let realtor = Uuid::new_v4();
        assert!(
            reject_self_review(reviewer, realtor).is_ok(),
            "reviewing a different realtor's profile must be allowed"
        );
    }
}
