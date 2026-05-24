//! Document intelligence routes — OCR, classification, search, summarization (Epics 28, 92).

use crate::state::AppState;
use api_core::{extractors::RlsConnection, AuthUser, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use common::errors::ErrorResponse;
use db::models::{
    ClassificationFeedback, DocumentSearchRequest, DocumentSearchResponse,
    GenerateSummaryRequest, document_category,
};
use uuid::Uuid;

use super::core::*;

/// Create documents intelligence router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}/ocr/reprocess", post(reprocess_ocr))
        .route("/search", post(search_documents))
        .route("/{id}/classification", get(get_classification))
        .route("/{id}/classification/feedback", post(submit_classification_feedback))
        .route("/{id}/classification/history", get(get_classification_history))
        .route("/{id}/summarize", post(request_summarization))
        .route("/{id}/ai-summarize", post(ai_summarize_document))
        .route("/intelligence/stats", get(get_intelligence_stats))
}

// Document Intelligence Handlers (Epic 28)
// ============================================================================

/// Reprocess OCR for a document (Story 28.1).
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/ocr/reprocess",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "OCR reprocess queued", body = OcrReprocessResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn reprocess_ocr(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<OcrReprocessResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify document exists and user has access
    let document = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find document: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find document",
                )),
            ));
        }
    };

    // Verify organization
    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    // Queue for OCR
    match state.document_repo.queue_for_ocr(id, Some(1)).await {
        Ok(queue_id) => {
            rls.release().await;
            Ok(Json(OcrReprocessResponse {
                message: "Document queued for OCR processing".to_string(),
                queue_id,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to queue document for OCR: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to queue document for OCR",
                )),
            ))
        }
    }
}

/// Full-text search across documents (Story 28.2).
#[utoipa::path(
    post,
    path = "/api/v1/documents/search",
    request_body = SearchDocumentsRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Search results", body = DocumentSearchResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn search_documents(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Json(req): Json<SearchDocumentsRequest>,
) -> Result<Json<DocumentSearchResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate query
    if req.query.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Search query cannot be empty",
            )),
        ));
    }

    if req.query.len() > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Search query too long (max 500 characters)",
            )),
        ));
    }

    let search_request = DocumentSearchRequest {
        query: req.query,
        organization_id: tenant.tenant_id,
        include_content: true,
        folder_id: req.folder_id,
        category: req.category,
        limit: req.limit,
        offset: req.offset,
    };

    match state.document_repo.full_text_search(search_request).await {
        Ok(response) => {
            rls.release().await;
            Ok(Json(response))
        }
        Err(e) => {
            tracing::error!("Failed to search documents: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to search documents",
                )),
            ))
        }
    }
}

/// Get document classification (Story 28.3).
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/classification",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Classification info", body = ClassificationResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn get_classification(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ClassificationResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Find document with details
    let document = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find document: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find document",
                )),
            ));
        }
    };

    // Verify organization
    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    // Return response based on the document's category
    rls.release().await;
    Ok(Json(ClassificationResponse {
        document_id: id,
        predicted_category: Some(document.category.clone()),
        confidence: None,
        classified_at: None,
        accepted: None,
    }))
}

/// Submit classification feedback (Story 28.3).
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/classification/feedback",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    request_body = ClassificationFeedbackRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Feedback submitted", body = MessageResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn submit_classification_feedback(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<ClassificationFeedbackRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify document exists
    let document = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find document: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find document",
                )),
            ));
        }
    };

    // Verify organization
    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    // Validate category if provided
    if let Some(ref cat) = req.correct_category {
        if !document_category::ALL.contains(&cat.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("BAD_REQUEST", "Invalid category")),
            ));
        }
    }

    let feedback = ClassificationFeedback {
        document_id: id,
        accepted: req.accepted,
        correct_category: req.correct_category,
        feedback_by: auth.user_id,
    };

    match state
        .document_repo
        .submit_classification_feedback(feedback)
        .await
    {
        Ok(()) => {
            rls.release().await;
            Ok(Json(MessageResponse {
                message: "Classification feedback submitted".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to submit classification feedback: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to submit feedback",
                )),
            ))
        }
    }
}

/// Get classification history for a document (Story 28.3).
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/classification/history",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Classification history", body = ClassificationHistoryResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn get_classification_history(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ClassificationHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify document exists and belongs to tenant
    let document = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find document: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find document",
                )),
            ));
        }
    };

    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    match state.document_repo.get_classification_history(id).await {
        Ok(history) => {
            rls.release().await;
            Ok(Json(ClassificationHistoryResponse { history }))
        }
        Err(e) => {
            tracing::error!("Failed to get classification history: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get classification history",
                )),
            ))
        }
    }
}

/// Request document summarization (Story 28.4).
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/summarize",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Summarization queued", body = SummarizationResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn request_summarization(
    State(state): State<AppState>,
    auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<SummarizationResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify document exists
    let document = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find document: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find document",
                )),
            ));
        }
    };

    // Verify organization
    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    let request = GenerateSummaryRequest {
        document_id: id,
        requested_by: auth.user_id,
        priority: Some(1),
    };

    match state.document_repo.queue_for_summarization(request).await {
        Ok(queue_id) => {
            rls.release().await;
            Ok(Json(SummarizationResponse {
                message: "Document queued for summarization".to_string(),
                queue_id,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to queue document for summarization: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to queue document for summarization",
                )),
            ))
        }
    }
}

/// Get document intelligence statistics.
#[utoipa::path(
    get,
    path = "/api/v1/documents/intelligence/stats",
    params(IntelligenceStatsQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Intelligence statistics", body = IntelligenceStatsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn get_intelligence_stats(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Query(query): Query<IntelligenceStatsQuery>,
) -> Result<Json<IntelligenceStatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Parse dates, default to last 30 days
    let end_date = query
        .end_date
        .as_ref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .unwrap_or_else(|| Utc::now().date_naive());

    let start_date = query
        .start_date
        .as_ref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .unwrap_or_else(|| end_date - chrono::Duration::days(30));

    match state
        .document_repo
        .get_intelligence_stats(tenant.tenant_id, start_date, end_date)
        .await
    {
        Ok(stats) => {
            rls.release().await;
            Ok(Json(IntelligenceStatsResponse { stats }))
        }
        Err(e) => {
            tracing::error!("Failed to get intelligence stats: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get statistics",
                )),
            ))
        }
    }
}

/// Simple message response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

// ============================================================================
// Epic 92.3: AI Document Summarization Handler
// ============================================================================

/// AI-powered document summarization (Story 92.3).
///
/// Generates a concise summary of the document using LLM.
/// Supports PDF, DOCX, and TXT documents.
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/ai-summarize",
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    request_body = AiSummarizeRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Document summarized", body = AiSummarizeResponse),
        (status = 404, description = "Document not found", body = ErrorResponse),
        (status = 400, description = "Unsupported document type", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Document Intelligence"
)]
async fn ai_summarize_document(
    State(state): State<AppState>,
    _auth: AuthUser,
    tenant: TenantExtractor,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<AiSummarizeRequest>,
) -> Result<Json<AiSummarizeResponse>, (StatusCode, Json<ErrorResponse>)> {
    use std::time::Instant;

    let start_time = Instant::now();

    // Verify document exists
    let document = match state
        .document_repo
        .find_by_id_rls(&mut **rls.conn(), id)
        .await
    {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to find document: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find document",
                )),
            ));
        }
    };

    // Verify organization
    if document.organization_id != tenant.tenant_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
        ));
    }

    // Check if document type is supported
    let supported_mime_types = [
        "application/pdf",
        "application/msword",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "text/plain",
        "text/markdown",
        "application/rtf",
    ];

    if !supported_mime_types.contains(&document.mime_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "UNSUPPORTED_FORMAT",
                format!(
                    "Document type '{}' is not supported for summarization. Supported: PDF, DOCX, DOC, TXT, MD, RTF",
                    document.mime_type
                ),
            )),
        ));
    }

    // Get document content (extracted text)
    let content = match state.document_repo.get_extracted_text(id).await {
        Ok(Some(text)) if !text.trim().is_empty() => text,
        Ok(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "NO_CONTENT",
                    "Document has no extracted text. Please run OCR first using /{id}/ocr/reprocess",
                )),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to get document content: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to retrieve document content",
                )),
            ));
        }
    };

    // Determine provider and model
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| match provider.as_str() {
        "openai" => "gpt-4o".to_string(),
        "azure_openai" => "gpt-4o".to_string(),
        _ => "claude-3-5-sonnet-20241022".to_string(),
    });

    // Build the summarization prompt
    let language = req.language.as_deref().unwrap_or("en");
    let summary_length = req.summary_length.as_deref().unwrap_or("medium");

    let word_target = match summary_length {
        "short" => "50-100 words",
        "long" => "300-500 words",
        _ => "150-250 words",
    };

    let language_name = match language {
        "sk" => "Slovak",
        "cs" => "Czech",
        "de" => "German",
        _ => "English",
    };

    let system_prompt = format!(
        r#"You are an expert document analyst and summarizer.

Your task is to create a concise summary of the provided document in {} language.

Requirements:
1. Generate a summary of {} (target length)
2. Extract 3-7 key points as bullet points
3. Maintain the document's core message and important details
4. Use clear, professional language
5. Do not add information not present in the original document

Format your response as:
SUMMARY:
[Your summary here]

KEY POINTS:
- [Key point 1]
- [Key point 2]
..."#,
        language_name, word_target
    );

    // For very long documents, truncate
    let content_tokens_estimate = content.len() / 4;
    let max_context = 100_000;

    let content_to_summarize = if content_tokens_estimate > max_context {
        let char_limit = max_context * 4;
        let half_limit = char_limit / 2;
        let start = &content[..half_limit.min(content.len())];
        let end_start = content.len().saturating_sub(half_limit);
        let end = &content[end_start..];

        format!(
            "[Document truncated for length]\n\n{}\n\n[...]\n\n{}",
            start, end
        )
    } else {
        content.clone()
    };

    // Call LLM
    let llm_request = integrations::ChatCompletionRequest {
        model: model.clone(),
        messages: vec![
            integrations::ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            integrations::ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Please summarize the following document:\n\n{}",
                    content_to_summarize
                ),
            },
        ],
        temperature: Some(0.3),
        max_tokens: Some(2000),
    };

    let response = state
        .llm_client
        .chat(&provider, &llm_request)
        .await
        .map_err(|e| {
            tracing::error!("LLM summarization failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "LLM_ERROR",
                    format!("Failed to generate summary: {}", e),
                )),
            )
        })?;

    let response_content = response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    // Parse the response
    let (summary, key_points) = parse_summary_response(&response_content);

    let processing_time_ms = start_time.elapsed().as_millis() as u64;

    // Store the summary in the document
    let key_points_json = serde_json::to_value(&key_points).ok();
    let word_count = summary.split_whitespace().count() as i32;
    let _ = state
        .document_repo
        .update_summary(
            id,
            &summary,
            key_points_json,
            None, // action_items
            None, // topics
            Some(word_count),
            Some(language),
        )
        .await;

    tracing::info!(
        "Document {} summarized (tokens: {}, latency: {}ms)",
        id,
        response.usage.total_tokens,
        processing_time_ms
    );

    rls.release().await;
    Ok(Json(AiSummarizeResponse {
        document_id: id,
        summary: summary.clone(),
        key_points,
        word_count: summary.split_whitespace().count(),
        tokens_used: response.usage.total_tokens,
        processing_time_ms,
        provider,
        model,
    }))
}

/// Parse the summary response from LLM.
fn parse_summary_response(content: &str) -> (String, Vec<String>) {
    let mut summary = String::new();
    let mut key_points = Vec::new();
    let mut in_summary = false;
    let mut in_key_points = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.to_uppercase().starts_with("SUMMARY:") {
            in_summary = true;
            in_key_points = false;
            let after_label = trimmed
                .strip_prefix("SUMMARY:")
                .or_else(|| trimmed.strip_prefix("Summary:"))
                .unwrap_or("");
            if !after_label.trim().is_empty() {
                summary.push_str(after_label.trim());
                summary.push(' ');
            }
            continue;
        }

        if trimmed.to_uppercase().starts_with("KEY POINTS:")
            || trimmed.to_uppercase().starts_with("KEY_POINTS:")
        {
            in_summary = false;
            in_key_points = true;
            continue;
        }

        if in_summary && !trimmed.is_empty() {
            summary.push_str(trimmed);
            summary.push(' ');
        }

        if in_key_points && trimmed.starts_with('-') {
            let point = trimmed.trim_start_matches('-').trim();
            if !point.is_empty() {
                key_points.push(point.to_string());
            }
        }
    }

    // If parsing failed, use the whole content as summary
    if summary.is_empty() {
        summary = content.to_string();
    }

    (summary.trim().to_string(), key_points)
}
