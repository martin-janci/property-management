//! Realtor routes (Epic 33: Realtor Tools).
//!
//! D1.2: handlers now use the unified `RequestPrincipal` extractor.

use crate::state::AppState;
use api_core::extractors::RequestPrincipal;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use db::models::{
    CreateRealtorProfile, InquiryMessage, ListingInquiry, RealtorProfile, SendInquiryMessage,
    UpdateRealtorProfile,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Create realtors router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/profile", get(get_my_profile))
        .route("/profile", post(create_profile))
        .route("/profile", put(update_profile))
        .route("/{user_id}/profile", get(get_profile))
        .route("/inquiries", get(list_inquiries))
        .route("/inquiries/{id}/read", post(mark_inquiry_read))
        .route("/inquiries/{id}/respond", post(respond_to_inquiry))
}

/// Realtor profile response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProfileResponse {
    pub profile: RealtorProfile,
}

/// Inquiries list response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct InquiriesResponse {
    pub inquiries: Vec<ListingInquiry>,
    pub total: i64,
}

/// Inquiries query parameters.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct InquiriesQuery {
    pub status: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Get current user's realtor profile.
#[utoipa::path(
    get,
    path = "/api/v1/realtors/profile",
    tag = "Realtors",
    responses(
        (status = 200, description = "Realtor profile", body = ProfileResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Profile not found")
    )
)]
pub async fn get_my_profile(
    State(state): State<AppState>,
    principal: RequestPrincipal,
) -> Result<Json<ProfileResponse>, (axum::http::StatusCode, String)> {
    let profile = state
        .reality_portal_repo
        .get_realtor_profile(principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("get realtor profile", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Profile not found".to_string(),
            )
        })?;

    Ok(Json(ProfileResponse { profile }))
}

/// Get realtor profile by user ID.
#[utoipa::path(
    get,
    path = "/api/v1/realtors/{user_id}/profile",
    tag = "Realtors",
    params(("user_id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "Realtor profile", body = ProfileResponse),
        (status = 404, description = "Profile not found")
    )
)]
pub async fn get_profile(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<ProfileResponse>, (axum::http::StatusCode, String)> {
    let profile = state
        .reality_portal_repo
        .get_realtor_profile(user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("get realtor profile", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Profile not found".to_string(),
            )
        })?;

    Ok(Json(ProfileResponse { profile }))
}

/// Create realtor profile.
#[utoipa::path(
    post,
    path = "/api/v1/realtors/profile",
    tag = "Realtors",
    request_body = CreateRealtorProfile,
    responses(
        (status = 201, description = "Profile created", body = ProfileResponse),
        (status = 401, description = "Unauthorized"),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_profile(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(data): Json<CreateRealtorProfile>,
) -> Result<Json<ProfileResponse>, (axum::http::StatusCode, String)> {
    let profile = state
        .reality_portal_repo
        .upsert_realtor_profile(principal.user_id, data)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("already exists") {
                (
                    axum::http::StatusCode::BAD_REQUEST,
                    "Profile already exists".to_string(),
                )
            } else {
                crate::util::errors::db_error("create realtor profile", e)
            }
        })?;

    Ok(Json(ProfileResponse { profile }))
}

/// Update realtor profile.
#[utoipa::path(
    put,
    path = "/api/v1/realtors/profile",
    tag = "Realtors",
    request_body = UpdateRealtorProfile,
    responses(
        (status = 200, description = "Profile updated", body = ProfileResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Profile not found")
    )
)]
pub async fn update_profile(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Json(data): Json<UpdateRealtorProfile>,
) -> Result<Json<ProfileResponse>, (axum::http::StatusCode, String)> {
    let profile = state
        .reality_portal_repo
        .update_realtor_profile(principal.user_id, data)
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            if error_str.contains("not found") {
                (
                    axum::http::StatusCode::NOT_FOUND,
                    "Profile not found".to_string(),
                )
            } else {
                crate::util::errors::db_error("update realtor profile", e)
            }
        })?;

    Ok(Json(ProfileResponse { profile }))
}

/// List realtor's inquiries.
#[utoipa::path(
    get,
    path = "/api/v1/realtors/inquiries",
    tag = "Realtors",
    params(InquiriesQuery),
    responses(
        (status = 200, description = "List of inquiries", body = InquiriesResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_inquiries(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Query(query): Query<InquiriesQuery>,
) -> Result<Json<InquiriesResponse>, (axum::http::StatusCode, String)> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = crate::util::clamp_offset_i32(query.offset);

    // Run the page query and the matching COUNT in parallel — the count must
    // use the same status filter as the page so clients can compute
    // `total / limit` for pagination. Returning `inquiries.len()` here (the
    // previous behaviour, #774 item 3) silently lied on every non-final page:
    // it reported the page size, not the real total.
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
    .map_err(|e| crate::util::errors::db_error("list realtor inquiries", e))?;

    Ok(Json(InquiriesResponse { inquiries, total }))
}

/// Mark inquiry as read.
#[utoipa::path(
    post,
    path = "/api/v1/realtors/inquiries/{id}/read",
    tag = "Realtors",
    params(("id" = Uuid, Path, description = "Inquiry ID")),
    responses(
        (status = 204, description = "Marked as read"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Inquiry not found")
    )
)]
pub async fn mark_inquiry_read(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // Issue #519 — was IDOR-able. Scoped to the calling realtor so realtor B
    // cannot flip realtor A's inquiries to 'read' (information manipulation).
    let updated = state
        .reality_portal_repo
        .mark_inquiry_read_for_realtor(id, principal.user_id)
        .await
        .map_err(|e| crate::util::errors::db_error("mark inquiry read", e))?;

    if !updated {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Inquiry not found".to_string(),
        ));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Respond to inquiry.
#[utoipa::path(
    post,
    path = "/api/v1/realtors/inquiries/{id}/respond",
    tag = "Realtors",
    params(("id" = Uuid, Path, description = "Inquiry ID")),
    request_body = SendInquiryMessage,
    responses(
        (status = 201, description = "Response sent", body = InquiryMessage),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Inquiry not found")
    )
)]
pub async fn respond_to_inquiry(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(data): Json<SendInquiryMessage>,
) -> Result<Json<InquiryMessage>, (axum::http::StatusCode, String)> {
    let message = state
        .reality_portal_repo
        .respond_to_inquiry(id, principal.user_id, &data.message)
        .await
        .map_err(|e| crate::util::errors::db_error("respond to inquiry", e))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Inquiry not found".to_string(),
            )
        })?;

    Ok(Json(message))
}
