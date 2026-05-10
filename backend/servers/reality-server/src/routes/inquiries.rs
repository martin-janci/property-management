//! Inquiries routes - contact and viewing requests.
//!
//! Handles listing inquiries, contact forms, and viewing scheduling.

use crate::extractors::AuthenticatedUser;
use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use db::models::{CreateListingInquiry, ListingInquiry, SendInquiryMessage};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Create inquiries router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Public routes (no auth required)
        .route("/contact/{listing_id}", post(send_contact_message))
        .route("/viewing/{listing_id}", post(request_viewing))
        // Authenticated routes
        .route("/", get(list_my_inquiries))
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
    Json(req): Json<ContactMessageRequest>,
) -> Result<Json<ContactMessageResponse>, (axum::http::StatusCode, String)> {
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
                Err((
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to send message: {}", e),
                ))
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
    Json(req): Json<ViewingRequest>,
) -> Result<Json<ViewingRequestResponse>, (axum::http::StatusCode, String)> {
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
                Err((
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to send viewing request: {}", e),
                ))
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
    auth: AuthenticatedUser,
    Query(query): Query<InquiryListQuery>,
) -> Result<Json<InquiryListResponse>, (axum::http::StatusCode, String)> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    let inquiries = state
        .reality_portal_repo
        .get_realtor_inquiries(auth.user_id, query.status, limit, offset)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list inquiries: {}", e),
            )
        })?;

    let total = inquiries.len() as i64;

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
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<InquiryDetailResponse>, (axum::http::StatusCode, String)> {
    // Get inquiries for this user and find the one with matching ID
    let inquiries = state
        .reality_portal_repo
        .get_realtor_inquiries(auth.user_id, None, 100, 0)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get inquiry: {}", e),
            )
        })?;

    let inquiry = inquiries.into_iter().find(|i| i.id == id).ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            "Inquiry not found".to_string(),
        )
    })?;

    // Mark as read if not already
    let _ = state.reality_portal_repo.mark_inquiry_read(id).await;

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
    _auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    state
        .reality_portal_repo
        .mark_inquiry_read(id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to mark as read: {}", e),
            )
        })?;

    Ok(axum::http::StatusCode::NO_CONTENT)
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
    auth: AuthenticatedUser,
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
        .respond_to_inquiry(id, auth.user_id, &req.message)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to send response: {}", e),
            )
        })?;

    Ok(Json(InquiryMessageResponse {
        id: message.id,
        sender_type: message.sender_type,
        message: message.message,
        created_at: message.created_at.to_rfc3339(),
    }))
}
