//! Listing report routes (UC-23: Report a Listing).
//!
//! Allows users to submit reports about problematic listings and track their status.
//! D1.2: handlers now use the unified `RequestPrincipal` /
//! `OptionalRequestPrincipal` extractors. The submit-report path stays
//! anonymous-friendly via the optional wrapper; the list-my-reports path
//! requires a real principal.

use crate::handlers::inquiries::{InquiriesHandler, InquiryResult};
use crate::routes::inquiries::client_ip_bucket;
use crate::state::AppState;
use api_core::extractors::{OptionalRequestPrincipal, RequestPrincipal};
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Input hygiene constants (H6, H7)
// ---------------------------------------------------------------------------
// Public-anonymous endpoint — without caps an unauthenticated attacker can
// post arbitrary-size strings/arrays that bloat DB rows, log files and the
// request body. 5000 chars mirrors the existing inquiry-response cap.
const MAX_REPORT_DESCRIPTION_LEN: usize = 5000;
const MAX_REPORT_ATTACHMENTS: usize = 10;
const MAX_URL_LEN: usize = 2048;

/// Reject `javascript:`, `data:`, `file:`, etc. URLs that would later be
/// rendered in `<img src=>` / `<a href=>` on the frontend (stored XSS).
/// Inlined locally per H7 brief — once the auth-hardening branch lands a
/// shared `url_validator` module this should be swapped for the import.
fn validate_image_or_link_url(s: &str) -> Result<(), String> {
    if s.chars().count() > MAX_URL_LEN {
        return Err(format!("URL must be at most {} characters", MAX_URL_LEN));
    }
    let parsed = url::Url::parse(s).map_err(|_| "Invalid URL".to_string())?;
    match parsed.scheme() {
        "https" | "http" => Ok(()),
        _ => Err("URL scheme must be http or https".to_string()),
    }
}

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

/// Enforce the per-IP anonymous report-submission throttle. Returns `Err(429)`
/// when the client has exceeded the quota.
///
/// Reuses the same per-client-IP limiter and spoof-resistant bucket key as the
/// anonymous inquiry POSTs (`state.inquiry_rate_limiters` +
/// [`client_ip_bucket`]): the abuse profile is identical (listing-scoped
/// anonymous POST), so a shared budget keeps the blast radius to a single
/// state field. If an operator later needs an independent budget, split off a
/// dedicated `state.report_rate_limiters` slot.
async fn enforce_public_report_rate_limit(
    state: &AppState,
    headers: &HeaderMap,
    listing_id: Uuid,
) -> Result<(), (axum::http::StatusCode, String)> {
    let key = client_ip_bucket(headers, state.inquiry_trusted_proxy_hops);
    let decision = state.inquiry_rate_limiters.check(key).await;
    match InquiriesHandler::rate_limit_result(decision) {
        Some(InquiryResult::RateLimited) => {
            tracing::warn!(%listing_id, "Anonymous report rate limit tripped");
            Err((
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                "Too many requests. Please try again in a minute.".to_string(),
            ))
        }
        _ => Ok(()),
    }
}

/// Submit a listing report (UC-23).
///
/// Auth is optional — anonymous reports allowed (per screen-map
/// `reality/report-listing`). Authenticated reports get a faster SLA because
/// the moderator can follow up; anonymous reports go through the same per-IP
/// throttle enforced at the route layer (`enforce_public_report_rate_limit`,
/// reusing the inquiry limiter) before any validation or DB work.
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
    OptionalRequestPrincipal(principal): OptionalRequestPrincipal,
    headers: HeaderMap,
    Json(data): Json<SubmitReportRequest>,
) -> Result<(axum::http::StatusCode, Json<SubmitReportResponse>), (axum::http::StatusCode, String)>
{
    // Reject anonymous floods at the routing layer BEFORE any validation or DB
    // work — an unauthenticated attacker could otherwise bloat `listing_reports`
    // and the moderation queue unboundedly from a single IP.
    enforce_public_report_rate_limit(&state, &headers, data.listing_id).await?;

    if data.description.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Description is required".to_string(),
        ));
    }
    if data.description.chars().count() > MAX_REPORT_DESCRIPTION_LEN {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!(
                "Description must be at most {} characters",
                MAX_REPORT_DESCRIPTION_LEN
            ),
        ));
    }

    // H6/H7: cap attachments array and validate each URL. Reject the request
    // generically — do NOT echo the offending URL back, that would itself be
    // a reflected XSS vector if the frontend renders error strings as HTML.
    if let Some(ref atts) = data.attachments {
        if atts.len() > MAX_REPORT_ATTACHMENTS {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!("At most {} attachments are allowed", MAX_REPORT_ATTACHMENTS),
            ));
        }
        for url in atts.iter() {
            if validate_image_or_link_url(url).is_err() {
                return Err((
                    axum::http::StatusCode::BAD_REQUEST,
                    "Attachment URL is invalid (must be http(s) and <= 2048 chars)".to_string(),
                ));
            }
        }
    }

    // Optional reporter contact details are persisted verbatim into
    // `listing_reports` and later surfaced to moderators, so they need the same
    // length + format guarantees as the anonymous inquiry contact fields.
    // Reuse `InquiriesHandler::is_valid_email` / `is_valid_phone` (the identical
    // anonymous-contact validators) rather than re-deriving divergent checks
    // here. Both are optional: skip when absent or blank, reject when present
    // but malformed. `is_valid_email` caps length at 254 chars and
    // `is_valid_phone` at 9–15 cleaned digits, covering the length bound too.
    if let Some(ref email) = data.reporter_email {
        let email = email.trim().to_lowercase();
        if !email.is_empty() && !InquiriesHandler::is_valid_email(&email) {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid reporter email format".to_string(),
            ));
        }
    }
    if let Some(ref phone) = data.reporter_phone {
        let phone = phone.trim();
        if !phone.is_empty() && !InquiriesHandler::is_valid_phone(phone) {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid reporter phone number format".to_string(),
            ));
        }
    }

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    // Check listing exists AND is publicly visible.
    //
    // SECURITY (H5, round-9 audit): without the status filter, this endpoint
    // is an unauthenticated existence oracle — an attacker can probe whether
    // a UUID belongs to a draft/archived/pending listing by diffing 404 vs
    // 201. Restricting to `status = 'active'` collapses the response shape
    // to "is there a public listing with this id?", matching what an honest
    // reporter would see in the UI.
    let listing_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM listings WHERE id = $1 AND status = 'active')",
    )
    .bind(data.listing_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("check listing", e))?;

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
    .bind(principal.as_ref().map(|p| p.user_id))
    .bind(&problem_type_str)
    .bind(&data.description)
    .bind(&attachments_json)
    .bind(&data.reporter_email)
    .bind(&data.reporter_phone)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("submit report", e))?;

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
    principal: RequestPrincipal,
    Query(query): Query<MyReportsQuery>,
) -> Result<Json<MyReportsResponse>, (axum::http::StatusCode, String)> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

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
    .bind(principal.user_id)
    .bind(limit)
    .bind(offset)
    .bind(&query.status)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("list reports", e))?;

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

    // Total row count for pagination must reflect ALL matching rows, not just
    // the current page (LIMIT/OFFSET), so run a COUNT with the same filters.
    let total_row = sqlx::query(
        r#"
        SELECT COUNT(*) AS total
        FROM listing_reports
        WHERE reporter_user_id = $1
          AND ($2::text IS NULL OR status = $2)
        "#,
    )
    .bind(principal.user_id)
    .bind(&query.status)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("count reports", e))?;

    let total_count: i64 = total_row.get("total");
    let total = usize::try_from(total_count).unwrap_or(0);

    Ok(Json(MyReportsResponse { reports, total }))
}
