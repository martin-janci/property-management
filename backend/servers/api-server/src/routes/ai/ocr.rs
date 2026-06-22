//! OCR meter-reading endpoints (Epic 128: OCR Meter Preview).
//!
//! POST /api/v1/ai/ocr/meter-reading — accept a meter image (multipart) and
//!   return an extracted reading value + confidence.
//! POST /api/v1/ai/ocr/correction — accept a correction feedback payload for
//!   model-improvement (stored but not yet forwarded to a training pipeline).
//!
//! The OCR extraction backend is not yet integrated (requires a Vision-capable
//! LLM or dedicated OCR service). Until that is wired, the endpoint returns a
//! 501 response with a clear reason so the frontend can degrade gracefully
//! instead of silently receiving a 404.

use crate::state::AppState;
use axum::{
    http::StatusCode,
    routing::post,
    Json, Router,
};
use axum_extra::extract::Multipart;
use common::errors::ErrorResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Response / request types
// ============================================================================

/// OCR extraction result (mirrors the frontend `OcrResult` shape).
#[derive(Debug, Serialize, ToSchema)]
pub struct OcrProcessResponse {
    pub result: OcrResult,
    pub image_url: String,
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
    pub original_value: f64,
    pub corrected_value: f64,
    pub image_url: String,
    pub bounding_box: Option<serde_json::Value>,
    pub timestamp: String,
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

/// Process a meter image with OCR and return the extracted reading.
///
/// Accepts multipart/form-data with fields:
/// - `image` (binary, required) — JPEG/PNG photo of the meter display
/// - `meter_id` (text, required) — UUID of the meter being read
///
/// Returns 501 until an OCR backend is configured.
#[utoipa::path(
    post,
    path = "/api/v1/ai/ocr/meter-reading",
    request_body(content = String, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "OCR result", body = OcrProcessResponse),
        (status = 501, description = "OCR backend not yet configured", body = ErrorResponse),
    ),
    tag = "AI OCR"
)]
async fn process_meter_reading(
    mut multipart: Multipart,
) -> Result<Json<OcrProcessResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Consume the multipart fields to avoid connection issues.
    while let Ok(Some(_field)) = multipart.next_field().await {}

    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse::new(
            "OCR_NOT_CONFIGURED",
            "OCR meter-reading extraction is not yet configured. \
             Please submit readings manually until this feature is enabled.",
        )),
    ))
}

/// Accept OCR correction feedback for model improvement.
///
/// The payload is accepted (200 OK) and discarded until a training pipeline
/// is connected. This ensures the frontend correction flow does not break
/// when the feature is partially enabled.
#[utoipa::path(
    post,
    path = "/api/v1/ai/ocr/correction",
    request_body = OcrCorrectionRequest,
    responses(
        (status = 200, description = "Correction recorded"),
        (status = 400, description = "Invalid payload", body = ErrorResponse),
    ),
    tag = "AI OCR"
)]
async fn submit_correction(
    Json(_body): Json<OcrCorrectionRequest>,
) -> StatusCode {
    // Accepted and discarded until a training-data sink is wired.
    tracing::debug!("Received OCR correction feedback (sink not yet connected)");
    StatusCode::OK
}
