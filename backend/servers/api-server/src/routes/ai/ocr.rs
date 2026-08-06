//! OCR meter-reading endpoints (Epic 128: OCR Meter Preview).
//!
//! POST /api/v1/ai/ocr/meter-reading — accept a meter image (multipart) and
//!   persist a photo-backed meter reading; returns extracted_value + confidence.
//! POST /api/v1/ai/ocr/correction — persist OCR correction feedback for
//!   model-improvement; previously a no-op, now stored in ocr_meter_corrections.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use axum_extra::extract::Multipart;
use common::errors::ErrorResponse;
use db::models::meter::CreateOcrCorrection;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Response / request types
// ============================================================================

/// OCR extraction result (mirrors the frontend `OcrResult` shape).
#[derive(Debug, Serialize, ToSchema)]
pub struct OcrProcessResponse {
    pub result: OcrResult,
    pub image_url: String,
    pub reading_id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OcrResult {
    pub extracted_value: f64,
    pub confidence: f64,
    pub bounding_box: BoundingBox,
    pub raw_text: String,
    pub processing_time_ms: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// OCR correction feedback payload.
#[derive(Debug, Deserialize, ToSchema)]
pub struct OcrCorrectionRequest {
    pub organization_id: Uuid,
    pub original_value: f64,
    pub corrected_value: f64,
    pub image_url: String,
    pub bounding_box: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meter_reading_id: Option<Uuid>,
}

/// Response after persisting an OCR correction.
#[derive(Debug, Serialize, ToSchema)]
pub struct OcrCorrectionResponse {
    pub id: Uuid,
    pub message: String,
}

// ============================================================================
// Router
// ============================================================================

pub fn ocr_router() -> Router<AppState> {
    Router::new()
        .route("/meter-reading", post(process_meter_reading))
        .route("/correction", post(submit_correction))
}

// ============================================================================
// Handlers
// ============================================================================

fn internal_err(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("DB_ERROR", msg)),
    )
}

fn bad_request(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::new("BAD_REQUEST", msg)),
    )
}

fn not_found(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", msg)),
    )
}

/// Process a meter image and persist a photo-backed reading.
///
/// Accepts multipart/form-data with fields:
/// - `meter_id` (text, required) — UUID of the meter being read
/// - `reading_value` (text, required) — numeric value as confirmed by the user
/// - `image` (binary, optional) — JPEG/PNG photo of the meter display
///
/// The OCR extraction pipeline is not yet integrated; the `reading_value`
/// supplied by the caller is stored as-is with `source = photo`. The response
/// shape is kept compatible with the future OCR pipeline so the frontend does
/// not need to change when real extraction is added.
#[utoipa::path(
    post,
    path = "/api/v1/ai/ocr/meter-reading",
    request_body(content = String, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Meter reading created", body = OcrProcessResponse),
        (status = 400, description = "Missing or invalid fields", body = ErrorResponse),
        (status = 404, description = "Meter not found", body = ErrorResponse),
    ),
    tag = "AI OCR"
)]
async fn process_meter_reading(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<OcrProcessResponse>), (StatusCode, Json<ErrorResponse>)> {
    let mut meter_id: Option<Uuid> = None;
    let mut reading_value: Option<Decimal> = None;
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut image_content_type: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("meter_id") => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| bad_request("Invalid meter_id field"))?;
                meter_id = Some(
                    Uuid::parse_str(&text)
                        .map_err(|_| bad_request("meter_id must be a valid UUID"))?,
                );
            }
            Some("reading_value") => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| bad_request("Invalid reading_value field"))?;
                reading_value =
                    Some(Decimal::from_str(&text).map_err(|_| {
                        bad_request("reading_value must be a valid decimal number")
                    })?);
            }
            Some("image") => {
                image_content_type = field.content_type().map(|s| s.to_owned());
                image_bytes = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|_| bad_request("Failed to read image bytes"))?
                        .to_vec(),
                );
            }
            _ => {
                // consume unknown fields
                let _ = field.bytes().await;
            }
        }
    }

    let meter_id = meter_id.ok_or_else(|| bad_request("meter_id is required"))?;
    let reading = reading_value.ok_or_else(|| bad_request("reading_value is required"))?;

    // Verify meter exists and caller is an org member.
    let meter = state
        .meter_repo
        .get_meter(meter_id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Meter lookup failed");
            internal_err("Failed to verify meter access")
        })?
        .ok_or_else(|| not_found("Meter not found"))?;

    let is_member = state
        .org_member_repo
        .is_member(meter.organization_id, auth.user_id)
        .await
        .map_err(|_| internal_err("Database error"))?;
    if !is_member {
        return Err(not_found("Meter not found"));
    }

    // Upload image to storage when available, otherwise use a placeholder.
    let photo_url = if let (Some(bytes), Some(ref storage)) = (image_bytes, &state.storage_service)
    {
        let mime = image_content_type.as_deref().unwrap_or("image/jpeg");
        let key = format!("ocr/meter-readings/{}/{}.jpg", meter_id, Uuid::new_v4());
        storage.upload(&key, bytes, mime).await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to upload OCR image");
            internal_err("Failed to store meter image")
        })?
    } else {
        format!("pending-upload/{}/{}", meter_id, Uuid::new_v4())
    };

    let meter_reading = state
        .meter_repo
        .submit_photo_reading(
            auth.user_id,
            meter_id,
            reading,
            photo_url.clone(),
            Some(reading),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to create photo meter reading");
            internal_err("Failed to persist meter reading")
        })?;

    let extracted = reading.to_string().parse::<f64>().unwrap_or(0.0);

    Ok((
        StatusCode::CREATED,
        Json(OcrProcessResponse {
            result: OcrResult {
                extracted_value: extracted,
                // Confidence is 0 until a real OCR pipeline is wired; the
                // value here is caller-supplied, not machine-extracted.
                confidence: 0.0,
                bounding_box: BoundingBox {
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 0.0,
                },
                raw_text: extracted.to_string(),
                processing_time_ms: 0,
            },
            image_url: photo_url,
            reading_id: meter_reading.id,
        }),
    ))
}

/// Persist OCR correction feedback for model improvement.
#[utoipa::path(
    post,
    path = "/api/v1/ai/ocr/correction",
    request_body = OcrCorrectionRequest,
    responses(
        (status = 201, description = "Correction recorded", body = OcrCorrectionResponse),
        (status = 400, description = "Invalid payload", body = ErrorResponse),
    ),
    tag = "AI OCR"
)]
async fn submit_correction(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<OcrCorrectionRequest>,
) -> Result<(StatusCode, Json<OcrCorrectionResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Do not trust `body.organization_id` alone for authorization: it is
    // client-supplied, and a caller who is a member of *some* org could
    // point `meter_reading_id` at a different org's reading while still
    // passing `is_member` for their own org (IDOR). When a target reading
    // is referenced, derive the owning organization from that reading's
    // meter and require the payload's organization_id to agree with it,
    // rejecting any mismatch as cross-tenant access. With no target
    // reading there is no other-tenant record to leak, so the caller's
    // claimed org membership is the whole scope.
    let scoped_organization_id = if let Some(meter_reading_id) = body.meter_reading_id {
        let reading = state
            .meter_repo
            .get_reading(meter_reading_id)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Meter reading lookup failed");
                internal_err("Failed to verify meter reading access")
            })?
            .ok_or_else(|| not_found("Meter reading not found"))?;

        let meter = state
            .meter_repo
            .get_meter(reading.meter_id)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Meter lookup failed");
                internal_err("Failed to verify meter reading access")
            })?
            .ok_or_else(|| not_found("Meter reading not found"))?;

        if body.organization_id != meter.organization_id {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "FORBIDDEN",
                    "organization_id does not match the meter reading's organisation",
                )),
            ));
        }

        meter.organization_id
    } else {
        body.organization_id
    };

    let is_member = state
        .org_member_repo
        .is_member(scoped_organization_id, auth.user_id)
        .await
        .map_err(|_| internal_err("Database error"))?;
    if !is_member {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Not a member of this organisation",
            )),
        ));
    }

    let orig = Decimal::from_str(&body.original_value.to_string())
        .map_err(|_| bad_request("original_value is invalid"))?;
    let corr = Decimal::from_str(&body.corrected_value.to_string())
        .map_err(|_| bad_request("corrected_value is invalid"))?;

    let record = state
        .meter_repo
        .create_ocr_correction(
            auth.user_id,
            CreateOcrCorrection {
                organization_id: scoped_organization_id,
                meter_reading_id: body.meter_reading_id,
                original_value: orig,
                corrected_value: corr,
                image_url: body.image_url,
                bounding_box: body.bounding_box,
            },
        )
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to persist OCR correction");
            internal_err("Failed to save correction")
        })?;

    Ok((
        StatusCode::CREATED,
        Json(OcrCorrectionResponse {
            id: record.id,
            message: "Correction recorded".to_string(),
        }),
    ))
}
