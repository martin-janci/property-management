//! Inquiries routes - contact and viewing requests.
//!
//! Handles listing inquiries, contact forms, and viewing scheduling.
//! D1.2: handlers now use the unified `RequestPrincipal` extractor.

use crate::handlers::inquiries::{InquiriesHandler, InquiryResult};
use crate::state::AppState;
use api_core::extractors::RequestPrincipal;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use db::models::{CreateListingInquiry, ListingInquiry, SendInquiryMessage};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Input hygiene constants (H6)
// ---------------------------------------------------------------------------
// `respond_to_inquiry` already caps at 5000; mirror the same for the
// public-anonymous initial submit (`send_contact_message`, `request_viewing`)
// so unauthenticated traffic cannot upload arbitrary-size payloads.
const MAX_INQUIRY_MESSAGE_LEN: usize = 5000;
const MAX_INQUIRY_NAME_LEN: usize = 200;
const MAX_INQUIRY_EMAIL_LEN: usize = 320; // RFC 5321 max
const MAX_INQUIRY_PHONE_LEN: usize = 32;
const MAX_INQUIRY_PREFERRED_TIMES: usize = 20;
const MAX_INQUIRY_PREFERRED_TIME_LEN: usize = 200;

/// Apply length caps to a public-submitted inquiry payload. Returns a generic
/// error message that does NOT echo the user-submitted string back (avoids
/// reflected XSS if a frontend ever renders error bodies as HTML).
fn validate_inquiry_lengths(
    name: &str,
    email: &str,
    phone: Option<&str>,
    message: &str,
) -> Result<(), &'static str> {
    if name.len() > MAX_INQUIRY_NAME_LEN {
        return Err("Name is too long");
    }
    if email.len() > MAX_INQUIRY_EMAIL_LEN {
        return Err("Email is too long");
    }
    if let Some(p) = phone {
        if p.len() > MAX_INQUIRY_PHONE_LEN {
            return Err("Phone is too long");
        }
    }
    if message.len() > MAX_INQUIRY_MESSAGE_LEN {
        return Err("Message is too long (max 5000 characters)");
    }
    Ok(())
}

/// Derive a stable per-client bucket key for the anonymous inquiry throttle.
///
/// reality-server sits behind a reverse proxy, so the peer socket is the
/// proxy, not the visitor — we read the forwarded client address from
/// `X-Forwarded-For` (first hop) or `X-Real-IP`. The address is hashed into a
/// `Uuid` (the key type of the reused `TenantRateLimiterSet`) rather than
/// stored verbatim, so no raw IP lives in the limiter map. A missing/garbled
/// header collapses to one shared "unknown" bucket, which is still throttled
/// collectively — fail-safe, never fail-open.
fn client_ip_bucket(headers: &HeaderMap) -> Uuid {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|s| !s.is_empty())
        })
        .unwrap_or("unknown");

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    ip.hash(&mut hasher);
    // High half is a fixed namespace tag so these IP buckets never collide
    // with real org UUIDs (they live in a separate limiter set regardless).
    Uuid::from_u64_pair(0x0000_0000_494e_5150, hasher.finish())
}

/// Enforce the per-IP anonymous inquiry throttle. Returns `Err(429)` when the
/// client has exceeded the quota (see `AppState::inquiry_rate_limiters`),
/// constructing/​matching `InquiryResult::RateLimited` so the abuse control is
/// live rather than dead code.
async fn enforce_inquiry_rate_limit(
    state: &AppState,
    headers: &HeaderMap,
    listing_id: Uuid,
) -> Result<(), (axum::http::StatusCode, String)> {
    let key = client_ip_bucket(headers);
    let decision = state.inquiry_rate_limiters.check(key).await;
    match InquiriesHandler::rate_limit_result(decision) {
        Some(InquiryResult::RateLimited) => {
            tracing::warn!(%listing_id, "Anonymous inquiry rate limit tripped");
            Err((
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                "Too many requests. Please try again in a minute.".to_string(),
            ))
        }
        _ => Ok(()),
    }
}

/// Create inquiries router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Public routes (no auth required)
        .route("/contact/{listing_id}", post(send_contact_message))
        .route("/viewing/{listing_id}", post(request_viewing))
        // Authenticated routes
        .route("/", get(list_my_inquiries))
        // Buyer-axis listing: inquiries the authenticated buyer submitted.
        // Must be registered before `/{id}` so `mine` is not captured as an id.
        .route("/mine", get(list_buyer_inquiries))
        .route("/{id}", get(get_inquiry))
        .route("/{id}/read", put(mark_as_read))
        .route("/{id}/respond", post(respond_to_inquiry))
}

/// Contact message request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ContactMessageRequest {
    /// Sender's name
    pub name: String,
    /// Sender's email
    pub email: String,
    /// Sender's phone (optional)
    pub phone: Option<String>,
    /// Message content
    pub message: String,
    /// Preferred contact method
    pub preferred_contact: Option<String>,
}

/// Contact message response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ContactMessageResponse {
    /// Success message
    pub message: String,
    /// Inquiry ID
    pub inquiry_id: Uuid,
}

/// Viewing request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ViewingRequest {
    /// Requester's name
    pub name: String,
    /// Requester's email
    pub email: String,
    /// Requester's phone
    pub phone: Option<String>,
    /// Preferred viewing times
    pub preferred_times: Option<Vec<String>>,
    /// Additional message
    pub message: Option<String>,
}

/// Viewing request response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ViewingRequestResponse {
    /// Success message
    pub message: String,
    /// Inquiry ID
    pub inquiry_id: Uuid,
}

/// Inquiry list query parameters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct InquiryListQuery {
    /// Filter by status (new, read, responded, closed)
    pub status: Option<String>,
    /// Page number
    pub page: Option<i32>,
    /// Page size
    pub limit: Option<i32>,
}

/// Inquiry list response.
#[derive(Debug, Serialize, ToSchema)]
pub struct InquiryListResponse {
    /// List of inquiries
    pub inquiries: Vec<ListingInquiry>,
    /// Total count
    pub total: i64,
    /// Current page
    pub page: i32,
    /// Page size
    pub limit: i32,
}

/// Inquiry detail response.
#[derive(Debug, Serialize, ToSchema)]
pub struct InquiryDetailResponse {
    /// Inquiry details
    pub inquiry: ListingInquiry,
    /// Conversation messages
    pub messages: Vec<InquiryMessageResponse>,
}

/// Inquiry message in conversation.
#[derive(Debug, Serialize, ToSchema)]
pub struct InquiryMessageResponse {
    /// Message ID
    pub id: Uuid,
    /// Sender type (user, realtor)
    pub sender_type: String,
    /// Message content
    pub message: String,
    /// Sent at
    pub created_at: String,
}

/// Send contact message to listing realtor.
#[utoipa::path(
    post,
    path = "/api/v1/inquiries/contact/{listing_id}",
    tag = "Inquiries",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    request_body = ContactMessageRequest,
    responses(
        (status = 201, description = "Message sent", body = ContactMessageResponse),
        (status = 400, description = "Invalid input"),
        (status = 404, description = "Listing not found")
    )
)]
pub async fn send_contact_message(
    State(state): State<AppState>,
    Path(listing_id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ContactMessageRequest>,
) -> Result<Json<ContactMessageResponse>, (axum::http::StatusCode, String)> {
    // Per-IP throttle FIRST: an anonymous flood must be rejected before any
    // validation, DB connection, listing lookup, row insert, or realtor
    // notification (the spam / DB-exhaustion vector this endpoint exposed).
    enforce_inquiry_rate_limit(&state, &headers, listing_id).await?;

    // H6: length caps before semantic validation — reject oversize bodies
    // before they bloat the request body / DB row.
    if let Err(msg) =
        validate_inquiry_lengths(&req.name, &req.email, req.phone.as_deref(), &req.message)
    {
        return Err((axum::http::StatusCode::BAD_REQUEST, msg.to_string()));
    }

    // Validate contact info
    let validation = crate::handlers::inquiries::InquiriesHandler::validate_contact(
        &req.name,
        &req.email,
        req.phone.as_deref(),
        &req.message,
    );

    if !validation.is_valid {
        let errors: Vec<String> = validation
            .errors
            .iter()
            .map(|e| e.message.clone())
            .collect();
        return Err((axum::http::StatusCode::BAD_REQUEST, errors.join(", ")));
    }

    // Public endpoint: acquire a connection and clear any stale RLS context
    // before looking up the listing's realtor. Detailed errors are logged
    // server-side; clients only see a generic 500 message.
    let mut conn = state.acquire_public_conn().await.map_err(|e| {
        tracing::error!(%listing_id, error = %e, "Failed to acquire db connection");
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
    })?;
    let listing = sqlx::query_as::<_, (Uuid,)>(
        "SELECT created_by FROM listings WHERE id = $1 AND status = 'active'",
    )
    .bind(listing_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| {
        tracing::error!(%listing_id, error = %e, "Failed to query listing");
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
    })?;

    let realtor_id = match listing {
        Some((created_by,)) => created_by,
        None => {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            ));
        }
    };

    let inquiry_data = CreateListingInquiry {
        name: req.name,
        email: req.email,
        phone: req.phone,
        message: req.message,
        inquiry_type: Some("info".to_string()),
        preferred_contact: req.preferred_contact,
        preferred_time: None,
    };

    match state
        .reality_portal_repo
        .create_inquiry(listing_id, realtor_id, None, inquiry_data)
        .await
    {
        Ok(inquiry) => {
            tracing::info!(
                inquiry_id = %inquiry.id,
                listing_id = %listing_id,
                "Contact message sent"
            );

            Ok(Json(ContactMessageResponse {
                message: "Your message has been sent. The realtor will respond soon.".to_string(),
                inquiry_id: inquiry.id,
            }))
        }
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("violates foreign key") {
                Err((
                    axum::http::StatusCode::NOT_FOUND,
                    "Listing not found".to_string(),
                ))
            } else {
                Err(crate::util::errors::db_error("send contact message", e))
            }
        }
    }
}

/// Request a viewing for a listing.
#[utoipa::path(
    post,
    path = "/api/v1/inquiries/viewing/{listing_id}",
    tag = "Inquiries",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    request_body = ViewingRequest,
    responses(
        (status = 201, description = "Viewing requested", body = ViewingRequestResponse),
        (status = 400, description = "Invalid input"),
        (status = 404, description = "Listing not found")
    )
)]
pub async fn request_viewing(
    State(state): State<AppState>,
    Path(listing_id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<ViewingRequest>,
) -> Result<Json<ViewingRequestResponse>, (axum::http::StatusCode, String)> {
    // Per-IP throttle FIRST (same anonymous abuse vector as the contact form).
    enforce_inquiry_rate_limit(&state, &headers, listing_id).await?;

    // H6: cap fields BEFORE we concatenate them into the outgoing message.
    if req.name.len() > MAX_INQUIRY_NAME_LEN {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Name is too long".to_string(),
        ));
    }
    if req.email.len() > MAX_INQUIRY_EMAIL_LEN {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Email is too long".to_string(),
        ));
    }
    if let Some(ref phone) = req.phone {
        if phone.len() > MAX_INQUIRY_PHONE_LEN {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Phone is too long".to_string(),
            ));
        }
    }
    if let Some(ref m) = req.message {
        if m.len() > MAX_INQUIRY_MESSAGE_LEN {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Message is too long (max 5000 characters)".to_string(),
            ));
        }
    }
    if let Some(ref times) = req.preferred_times {
        if times.len() > MAX_INQUIRY_PREFERRED_TIMES {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!(
                    "At most {} preferred times allowed",
                    MAX_INQUIRY_PREFERRED_TIMES
                ),
            ));
        }
        for t in times.iter() {
            if t.len() > MAX_INQUIRY_PREFERRED_TIME_LEN {
                return Err((
                    axum::http::StatusCode::BAD_REQUEST,
                    "Preferred time entry is too long".to_string(),
                ));
            }
        }
    }

    // Build message with preferred times
    let message = if let Some(times) = &req.preferred_times {
        format!(
            "{}\n\nPreferred viewing times:\n{}",
            req.message
                .as_deref()
                .unwrap_or("I would like to schedule a viewing."),
            times.join("\n")
        )
    } else {
        req.message
            .unwrap_or_else(|| "I would like to schedule a viewing.".to_string())
    };

    // Validate contact info
    let validation = crate::handlers::inquiries::InquiriesHandler::validate_contact(
        &req.name,
        &req.email,
        req.phone.as_deref(),
        &message,
    );

    if !validation.is_valid {
        let errors: Vec<String> = validation
            .errors
            .iter()
            .map(|e| e.message.clone())
            .collect();
        return Err((axum::http::StatusCode::BAD_REQUEST, errors.join(", ")));
    }

    // Public endpoint: acquire a connection and clear any stale RLS context
    // before looking up the listing's realtor. Detailed errors are logged
    // server-side; clients only see a generic 500 message.
    let mut conn = state.acquire_public_conn().await.map_err(|e| {
        tracing::error!(%listing_id, error = %e, "Failed to acquire db connection");
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
    })?;
    let listing = sqlx::query_as::<_, (Uuid,)>(
        "SELECT created_by FROM listings WHERE id = $1 AND status = 'active'",
    )
    .bind(listing_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| {
        tracing::error!(%listing_id, error = %e, "Failed to query listing");
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
    })?;

    let realtor_id = match listing {
        Some((created_by,)) => created_by,
        None => {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            ));
        }
    };

    let inquiry_data = CreateListingInquiry {
        name: req.name,
        email: req.email,
        phone: req.phone,
        message,
        inquiry_type: Some("viewing".to_string()),
        preferred_contact: Some("phone".to_string()),
        preferred_time: None,
    };

    match state
        .reality_portal_repo
        .create_inquiry(listing_id, realtor_id, None, inquiry_data)
        .await
    {
        Ok(inquiry) => {
            tracing::info!(
                inquiry_id = %inquiry.id,
                listing_id = %listing_id,
                "Viewing request sent"
            );

            Ok(Json(ViewingRequestResponse {
                message: "Your viewing request has been sent. The realtor will contact you to confirm the time.".to_string(),
                inquiry_id: inquiry.id,
            }))
        }
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("violates foreign key") {
                Err((
                    axum::http::StatusCode::NOT_FOUND,
                    "Listing not found".to_string(),
                ))
            } else {
                Err(crate::util::errors::db_error("send viewing request", e))
            }
        }
    }
}

/// List my inquiries (as a realtor or user).
#[utoipa::path(
    get,
    path = "/api/v1/inquiries",
    tag = "Inquiries",
    params(InquiryListQuery),
    responses(
        (status = 200, description = "Inquiry list", body = InquiryListResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_my_inquiries(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(query): Query<InquiryListQuery>,
) -> Result<Json<InquiryListResponse>, (axum::http::StatusCode, String)> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    // Run the page query and the matching COUNT in parallel — the count
    // must use the same filter as the page so clients can compute
    // `total / limit` for pagination. Returning `inquiries.len()` here
    // (the previous behaviour) silently lied on every non-final page.
    let status_filter = query.status.clone();
    let (inquiries, total) = tokio::try_join!(
        state.reality_portal_repo.get_realtor_inquiries(
            principal.user_id,
            query.status,
            limit,
            offset,
        ),
        state
            .reality_portal_repo
            .count_realtor_inquiries(principal.user_id, status_filter),
    )
    .map_err(|e| crate::util::errors::db_error("list inquiries", e))?;

    Ok(Json(InquiryListResponse {
        inquiries,
        total,
        page,
        limit,
    }))
}

/// List inquiries the authenticated buyer submitted.
///
/// Buyer-axis counterpart to [`list_my_inquiries`] (which is realtor-scoped):
/// returns only inquiries whose `user_id` matches the calling principal, so a
/// buyer sees the inquiries they sent and never another buyer's. `total` is the
/// FULL filtered count (not the current page length) so clients can paginate —
/// the page-length-as-total bug fixed for the realtor axis in PR #919.
#[utoipa::path(
    get,
    path = "/api/v1/inquiries/mine",
    tag = "Inquiries",
    params(InquiryListQuery),
    responses(
        (status = 200, description = "Buyer's inquiry list", body = InquiryListResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_buyer_inquiries(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(query): Query<InquiryListQuery>,
) -> Result<Json<InquiryListResponse>, (axum::http::StatusCode, String)> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    // Page query and matching COUNT run in parallel; both use the same filter
    // so `total / limit` paginates correctly.
    let status_filter = query.status.clone();
    let (inquiries, total) = tokio::try_join!(
        state.reality_portal_repo.get_buyer_inquiries(
            principal.user_id,
            query.status,
            limit,
            offset,
        ),
        state
            .reality_portal_repo
            .count_buyer_inquiries(principal.user_id, status_filter),
    )
    .map_err(|e| crate::util::errors::db_error("list buyer inquiries", e))?;

    Ok(Json(InquiryListResponse {
        inquiries,
        total,
        page,
        limit,
    }))
}

/// Get inquiry details.
#[utoipa::path(
    get,
    path = "/api/v1/inquiries/{id}",
    tag = "Inquiries",
    params(("id" = Uuid, Path, description = "Inquiry ID")),
    responses(
        (status = 200, description = "Inquiry details", body = InquiryDetailResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Inquiry not found")
    )
)]
pub async fn get_inquiry(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<InquiryDetailResponse>, (axum::http::StatusCode, String)> {
    // Single-row lookup scoped to the calling realtor. The previous code
    // fetched the first 100 inquiries and linear-scanned in Rust, which
    // silently 404'd for any inquiry beyond the first page for power users.
    let inquiry = state
        .reality_portal_repo
        .get_inquiry_for_realtor(id, principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("get inquiry", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Inquiry not found".to_string(),
            )
        })?;

    // Mark as read if not already (ownership already verified above)
    let _ = state
        .reality_portal_repo
        .mark_inquiry_read_for_realtor(id, principal.user_id)
        .await;

    Ok(Json(InquiryDetailResponse {
        inquiry,
        messages: vec![], // Would fetch conversation messages
    }))
}

/// Mark inquiry as read.
#[utoipa::path(
    put,
    path = "/api/v1/inquiries/{id}/read",
    tag = "Inquiries",
    params(("id" = Uuid, Path, description = "Inquiry ID")),
    responses(
        (status = 204, description = "Marked as read"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Inquiry not found")
    )
)]
pub async fn mark_as_read(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    let owned = state
        .reality_portal_repo
        .mark_inquiry_read_for_realtor(id, principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("mark inquiry read", e))?;

    if owned {
        Ok(axum::http::StatusCode::NO_CONTENT)
    } else {
        Err((
            axum::http::StatusCode::NOT_FOUND,
            "Inquiry not found".to_string(),
        ))
    }
}

/// Respond to an inquiry.
#[utoipa::path(
    post,
    path = "/api/v1/inquiries/{id}/respond",
    tag = "Inquiries",
    params(("id" = Uuid, Path, description = "Inquiry ID")),
    request_body = SendInquiryMessage,
    responses(
        (status = 200, description = "Response sent", body = InquiryMessageResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Inquiry not found")
    )
)]
pub async fn respond_to_inquiry(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(req): Json<SendInquiryMessage>,
) -> Result<Json<InquiryMessageResponse>, (axum::http::StatusCode, String)> {
    // Validate message
    if req.message.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Message is required".to_string(),
        ));
    }

    if req.message.len() > 5000 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Message must be less than 5000 characters".to_string(),
        ));
    }

    let message = state
        .reality_portal_repo
        .respond_to_inquiry(id, principal.user_id, &req.message)
        .await
        .map_err(|e| crate::util::errors::db_error("respond to inquiry", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Inquiry not found".to_string(),
            )
        })?;

    Ok(Json(InquiryMessageResponse {
        id: message.id,
        sender_type: message.sender_type,
        message: message.message,
        created_at: message.created_at.to_rfc3339(),
    }))
}
