//! Listing report routes (UC-23: Report a Listing).
//!
//! Allows users to submit reports about problematic listings and track their status.

use crate::extractors::{AuthenticatedUser, OptionalAuth};
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Create reports router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(submit_report))
        .route("/me", get(list_my_reports))
}

/// Report problem type.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProblemType {
    IncorrectInformation,
    FraudulentListing,
    AlreadySold,
    PriceManipulation,
    InappropriateContent,
    DuplicateListing,
    Other,
}

impl std::fmt::Display for ProblemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ProblemType::IncorrectInformation => "incorrect_information",
            ProblemType::FraudulentListing => "fraudulent_listing",
            ProblemType::AlreadySold => "already_sold",
            ProblemType::PriceManipulation => "price_manipulation",
            ProblemType::InappropriateContent => "inappropriate_content",
            ProblemType::DuplicateListing => "duplicate_listing",
            ProblemType::Other => "other",
        };
        write!(f, "{}", s)
    }
}

/// Submit a listing report request.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SubmitReportRequest {
    pub listing_id: Uuid,
    pub problem_type: ProblemType,
    pub description: String,
    pub attachments: Option<Vec<String>>,
    pub reporter_email: Option<String>,
    pub reporter_phone: Option<String>,
}

/// A submitted listing report.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListingReport {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub reporter_user_id: Option<Uuid>,
    pub problem_type: String,
    pub description: String,
    pub attachments: Option<Vec<String>>,
    pub reporter_email: Option<String>,
    pub reporter_phone: Option<String>,
    pub status: String,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Submit report response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SubmitReportResponse {
    pub report: ListingReport,
}

/// My reports list response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MyReportsResponse {
    pub reports: Vec<ListingReport>,
    pub total: usize,
}

/// Query parameters for listing my reports.
#[derive(Debug, Deserialize, IntoParams)]
pub struct MyReportsQuery {
    /// Filter by status: received, in_review, resolved, rejected
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Submit a listing report (UC-23).
///
/// Auth is optional — anonymous reports allowed (per screen-map
/// `reality/report-listing`). Authenticated reports get a faster SLA because
/// the moderator can follow up; anonymous reports still go through but should
/// be rate-limited by IP at the moderation layer.
#[utoipa::path(
    post,
    path = "/api/v1/reports",
    tag = "Reports",
    request_body = SubmitReportRequest,
    responses(
        (status = 201, description = "Report submitted", body = SubmitReportResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Listing not found")
    ),
    security((), ("session_token" = []))
)]
pub async fn submit_report(
    State(state): State<AppState>,
    auth: OptionalAuth,
    Json(data): Json<SubmitReportRequest>,
) -> Result<(axum::http::StatusCode, Json<SubmitReportResponse>), (axum::http::StatusCode, String)>
{
    if data.description.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Description is required".to_string(),
        ));
    }

    let mut conn = state.acquire_public_conn().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
    })?;

    // Check listing exists
    let listing_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM listings WHERE id = $1)")
            .bind(data.listing_id)
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

    let problem_type_str = data.problem_type.to_string();
    let attachments_json = data
        .attachments
        .as_ref()
        .map(|a| serde_json::to_value(a).unwrap_or(serde_json::Value::Array(vec![])));

    let row = sqlx::query(
        r#"
        INSERT INTO listing_reports
            (listing_id, reporter_user_id, problem_type, description, attachments,
             reporter_email, reporter_phone, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, 'received')
        RETURNING id, listing_id, reporter_user_id, problem_type, description,
                  attachments, reporter_email, reporter_phone, status, resolution_notes,
                  created_at, updated_at
        "#,
    )
    .bind(data.listing_id)
    .bind(auth.0.as_ref().map(|a| a.user_id))
    .bind(&problem_type_str)
    .bind(&data.description)
    .bind(&attachments_json)
    .bind(&data.reporter_email)
    .bind(&data.reporter_phone)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to submit report: {}", e),
        )
    })?;

    use sqlx::Row;
    let attachments_stored: Option<serde_json::Value> = row.get("attachments");
    let attachments_vec: Option<Vec<String>> =
        attachments_stored.and_then(|v| serde_json::from_value(v).ok());

    let report = ListingReport {
        id: row.get("id"),
        listing_id: row.get("listing_id"),
        reporter_user_id: row.get("reporter_user_id"),
        problem_type: row.get("problem_type"),
        description: row.get("description"),
        attachments: attachments_vec,
        reporter_email: row.get("reporter_email"),
        reporter_phone: row.get("reporter_phone"),
        status: row.get("status"),
        resolution_notes: row.get("resolution_notes"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    };

    Ok((
        axum::http::StatusCode::CREATED,
        Json(SubmitReportResponse { report }),
    ))
}

/// List the current user's submitted reports.
#[utoipa::path(
    get,
    path = "/api/v1/reports/me",
    tag = "Reports",
    params(MyReportsQuery),
    responses(
        (status = 200, description = "User's submitted reports", body = MyReportsResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("session_token" = []))
)]
pub async fn list_my_reports(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<MyReportsQuery>,
) -> Result<Json<MyReportsResponse>, (axum::http::StatusCode, String)> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let mut conn = state.acquire_public_conn().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
    })?;

    let rows = sqlx::query(
        r#"
        SELECT id, listing_id, reporter_user_id, problem_type, description,
               attachments, reporter_email, reporter_phone, status, resolution_notes,
               created_at, updated_at
        FROM listing_reports
        WHERE reporter_user_id = $1
          AND ($4::text IS NULL OR status = $4)
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(auth.user_id)
    .bind(limit)
    .bind(offset)
    .bind(&query.status)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list reports: {}", e),
        )
    })?;

    use sqlx::Row;
    let reports: Vec<ListingReport> = rows
        .into_iter()
        .map(|row| {
            let attachments_stored: Option<serde_json::Value> = row.get("attachments");
            let attachments_vec: Option<Vec<String>> =
                attachments_stored.and_then(|v| serde_json::from_value(v).ok());
            ListingReport {
                id: row.get("id"),
                listing_id: row.get("listing_id"),
                reporter_user_id: row.get("reporter_user_id"),
                problem_type: row.get("problem_type"),
                description: row.get("description"),
                attachments: attachments_vec,
                reporter_email: row.get("reporter_email"),
                reporter_phone: row.get("reporter_phone"),
                status: row.get("status"),
                resolution_notes: row.get("resolution_notes"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        })
        .collect();

    let total = reports.len();
    Ok(Json(MyReportsResponse { reports, total }))
}
