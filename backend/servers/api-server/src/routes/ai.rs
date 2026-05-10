//! AI routes (Epic 13: AI Assistant & Automation).
//!
//! Handles AI chat, sentiment analysis, equipment/maintenance, and workflows.
//! Epic 91: Wired to actual LLM providers (OpenAI/Anthropic).

use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use common::{errors::ErrorResponse, TenantContext};
use db::models::{alert_type, CreateSentimentAlert, UpsertSentimentTrend};
use db::models::{
    CreateChatSession, CreateEquipment, CreateMaintenance, CreateWorkflow, CreateWorkflowAction,
    EquipmentQuery, ExecutionQuery, ProvideFeedback, SendChatMessage, SentimentTrendQuery,
    TriggerWorkflow, UpdateEquipment, UpdateMaintenance, UpdateSentimentThresholds, UpdateWorkflow,
    WorkflowQuery,
};
use integrations::{ChatCompletionRequest, ChatMessage, ContextChunk, LlmClient, TenantAiConfig};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract tenant context from request headers.
fn extract_tenant_context(
    headers: &HeaderMap,
) -> Result<TenantContext, (StatusCode, Json<ErrorResponse>)> {
    let tenant_header = headers
        .get("X-Tenant-Context")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new(
                    "MISSING_CONTEXT",
                    "Tenant context required",
                )),
            )
        })?;

    serde_json::from_str(tenant_header).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_CONTEXT",
                "Invalid tenant context format",
            )),
        )
    })
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

// ============================================================================
// Query Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Default, utoipa::IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Default, utoipa::IntoParams)]
pub struct AlertsQuery {
    pub acknowledged: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// AI Chat Router (Story 13.1)
// ============================================================================

pub fn ai_chat_router() -> Router<AppState> {
    Router::new()
        .route("/sessions", post(create_session))
        .route("/sessions", get(list_sessions))
        .route("/sessions/{session_id}", get(get_session))
        .route("/sessions/{session_id}", delete(delete_session))
        .route("/sessions/{session_id}/messages", get(list_messages))
        .route("/sessions/{session_id}/messages", post(send_message))
        .route("/messages/{message_id}/feedback", post(provide_feedback))
        .route("/escalated", get(list_escalated))
}

#[utoipa::path(
    post,
    path = "/api/v1/ai/chat/sessions",
    request_body = CreateChatSession,
    responses(
        (status = 201, description = "Session created"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "AI Chat"
)]
async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateChatSession>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    match state.ai_chat_repo.create_session(req).await {
        Ok(session) => Ok((StatusCode::CREATED, Json(serde_json::json!(session)))),
        Err(e) => {
            tracing::error!("Failed to create session: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create session",
                )),
            ))
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/ai/chat/sessions",
    params(PaginationQuery),
    responses(
        (status = 200, description = "Sessions list"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "AI Chat"
)]
async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .ai_chat_repo
        .list_user_sessions(
            tenant.user_id,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
    {
        Ok(sessions) => Ok(Json(serde_json::json!({ "sessions": sessions }))),
        Err(e) => {
            tracing::error!("Failed to list sessions: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list sessions",
                )),
            ))
        }
    }
}

async fn get_session(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.ai_chat_repo.find_session_by_id(session_id).await {
        Ok(Some(session)) => Ok(Json(serde_json::json!(session))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Session not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get session: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get session",
                )),
            ))
        }
    }
}

async fn delete_session(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.ai_chat_repo.delete_session(session_id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Session not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to delete session: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete session",
                )),
            ))
        }
    }
}

async fn list_messages(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .ai_chat_repo
        .list_session_messages(
            session_id,
            query.limit.unwrap_or(100),
            query.offset.unwrap_or(0),
        )
        .await
    {
        Ok(messages) => Ok(Json(serde_json::json!({ "messages": messages }))),
        Err(e) => {
            tracing::error!("Failed to list messages: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list messages",
                )),
            ))
        }
    }
}

/// Default system prompt for the AI assistant (Story 97.1).
const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a helpful AI assistant for a property management system. You help users with:
- Building and property management questions
- Fault reporting and maintenance inquiries
- Voting and decision-making processes
- Document and announcement management
- Resident and owner questions

Be concise, professional, and helpful. If you're unsure about something or the question requires human expertise, acknowledge this and suggest escalation to building management.

If you don't have enough context to answer a question confidently, say so and ask for clarification."#;

/// Default maximum messages to include in conversation history for context.
const DEFAULT_HISTORY_LIMIT: i64 = 20;

/// Maximum response tokens to reserve when truncating context.
const MAX_RESPONSE_TOKENS: u32 = 2048;

async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(session_id): Path<Uuid>,
    Json(req): Json<SendChatMessage>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let start_time = Instant::now();

    // Get tenant context for RAG document search
    let tenant = extract_tenant_context(&headers).ok();

    // Verify session exists
    let _session = state
        .ai_chat_repo
        .find_session_by_id(session_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to find session",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Session not found")),
            )
        })?;

    // Get conversation history for context BEFORE adding current message
    // Story 97.1: Load more messages for context, will be truncated to fit token limits
    let history = state
        .ai_chat_repo
        .list_session_messages(session_id, DEFAULT_HISTORY_LIMIT, 0)
        .await
        .unwrap_or_default();

    // Add user message after fetching history to avoid duplication
    let user_msg = state
        .ai_chat_repo
        .add_message(
            session_id,
            "user",
            &req.content,
            None,
            vec![],
            false,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to add user message: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to send message",
                )),
            )
        })?;

    // Story 97.4: Analyze sentiment of user message and trigger alerts if needed
    let sentiment_analysis = if let Some(ref tenant_ctx) = tenant {
        // Check if sentiment analysis is enabled via feature flag
        let sentiment_enabled = std::env::var("SENTIMENT_ANALYSIS_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .to_lowercase();
        let analyze_sentiment = sentiment_enabled != "false" && sentiment_enabled != "0";

        if analyze_sentiment {
            match state.llm_client.analyze_sentiment(&req.content, None).await {
                Ok(result) => {
                    // Check if alert should be triggered based on organization thresholds
                    if let Ok(thresholds) = state
                        .sentiment_repo
                        .get_thresholds(tenant_ctx.tenant_id)
                        .await
                    {
                        if thresholds.enabled {
                            // Story 97.4: Check if sentiment requires attention and create alert
                            let should_alert = result.requires_attention
                                || (result.score < 0.0
                                    && result.score.abs() >= thresholds.negative_threshold);

                            if should_alert {
                                let alert = CreateSentimentAlert {
                                    organization_id: tenant_ctx.tenant_id,
                                    building_id: None, // Could be extracted from session context
                                    alert_type: alert_type::SPIKE_NEGATIVE.to_string(),
                                    threshold_breached: thresholds.negative_threshold,
                                    current_sentiment: result.score,
                                    previous_sentiment: None,
                                    sample_message_ids: vec![user_msg.id],
                                };

                                if let Err(e) = state.sentiment_repo.create_alert(alert).await {
                                    tracing::warn!("Failed to create sentiment alert: {}", e);
                                } else {
                                    tracing::info!(
                                        "Created sentiment alert for message {} (score: {:.2})",
                                        user_msg.id,
                                        result.score
                                    );
                                }
                            }

                            // Update daily sentiment trend
                            let today = chrono::Utc::now().date_naive();
                            let (neg, neut, pos) = match result.label.as_str() {
                                "negative" => (1, 0, 0),
                                "neutral" => (0, 1, 0),
                                "positive" => (0, 0, 1),
                                _ => (0, 0, 0),
                            };

                            let trend_data = UpsertSentimentTrend {
                                organization_id: tenant_ctx.tenant_id,
                                building_id: None,
                                date: today,
                                avg_sentiment: result.score,
                                message_count: 1,
                                negative_count: neg,
                                neutral_count: neut,
                                positive_count: pos,
                            };

                            if let Err(e) = state.sentiment_repo.upsert_trend(trend_data).await {
                                tracing::warn!("Failed to update sentiment trend: {}", e);
                            }
                        }
                    }

                    Some(result)
                }
                Err(e) => {
                    tracing::warn!("Failed to analyze sentiment: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Story 97.1: Build tenant-specific system prompt
    // Load tenant AI configuration if available (custom personality, building context)
    let tenant_config = if tenant.is_some() {
        // Try to load tenant AI config from environment or database
        // For now, build from environment variables as a simple implementation
        // In production, this would query a tenant_ai_config table
        let personality = std::env::var("AI_PERSONALITY").ok();
        let building_context = std::env::var("AI_BUILDING_CONTEXT").ok();
        let custom_instructions: Vec<String> = std::env::var("AI_CUSTOM_INSTRUCTIONS")
            .ok()
            .map(|s| s.split(';').map(|i| i.trim().to_string()).collect())
            .unwrap_or_default();

        Some(TenantAiConfig {
            personality,
            building_context,
            custom_instructions,
            preferred_language: Some(
                std::env::var("AI_LANGUAGE").unwrap_or_else(|_| "en".to_string()),
            ),
            escalation_topics: vec![],
        })
    } else {
        None
    };

    // Determine language for response
    let language = tenant_config
        .as_ref()
        .and_then(|c| c.preferred_language.clone())
        .unwrap_or_else(|| "en".to_string());

    // Build the system prompt with tenant-specific configuration
    let system_prompt =
        LlmClient::build_system_prompt(DEFAULT_SYSTEM_PROMPT, tenant_config.as_ref(), &language);

    // Build messages for LLM
    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "system".to_string(),
        content: system_prompt,
    }];

    // Add conversation history (fetched before adding current message, so no duplicates)
    for msg in history.iter() {
        messages.push(ChatMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
        });
    }

    // Story 97.2: Search for relevant documents using RAG with semantic similarity
    let mut context_chunks: Vec<ContextChunk> = vec![];
    if let Some(ref tenant_ctx) = tenant {
        // Check if semantic search is enabled via feature flag
        let semantic_search_enabled = std::env::var("RAG_SEMANTIC_SEARCH_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .to_lowercase();
        let use_semantic_search =
            semantic_search_enabled != "false" && semantic_search_enabled != "0";

        if use_semantic_search {
            // Try semantic similarity search first (Story 97.2)
            // Generate embedding for the user's query
            match state
                .llm_client
                .generate_embedding(&req.content, None)
                .await
            {
                Ok(embedding_result) => {
                    // Search documents by embedding similarity
                    match state
                        .llm_document_repo
                        .search_documents_by_embedding(
                            tenant_ctx.tenant_id,
                            &embedding_result.embedding,
                            5,         // Get top 5 relevant chunks
                            Some(0.6), // Minimum similarity threshold
                        )
                        .await
                    {
                        Ok(docs_with_scores) => {
                            for (doc, similarity) in docs_with_scores {
                                context_chunks.push(ContextChunk {
                                    source_id: doc.document_id,
                                    source_title: doc
                                        .metadata
                                        .get("title")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Document")
                                        .to_string(),
                                    text: doc.chunk_text.clone(),
                                    relevance_score: similarity,
                                });
                            }
                            tracing::debug!(
                                "RAG semantic search found {} relevant chunks",
                                context_chunks.len()
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Semantic search failed, falling back to text search: {}",
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to generate query embedding, falling back to text search: {}",
                        e
                    );
                }
            }
        }

        // Fallback to text search if semantic search didn't find results or is disabled
        if context_chunks.is_empty() {
            match state
                .llm_document_repo
                .search_documents_by_text(tenant_ctx.tenant_id, &req.content, 3)
                .await
            {
                Ok(docs) => {
                    for doc in docs {
                        context_chunks.push(ContextChunk {
                            source_id: doc.document_id,
                            source_title: doc
                                .metadata
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Document")
                                .to_string(),
                            text: doc.chunk_text.clone(),
                            relevance_score: 0.5, // Lower score for text match fallback
                        });
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to search documents for RAG context (tenant: {}): {}",
                        tenant_ctx.tenant_id,
                        e
                    );
                }
            }
        }
    }

    // Add context chunks to user message if available
    let user_content = if !context_chunks.is_empty() {
        let context_text: Vec<String> = context_chunks
            .iter()
            .map(|c| format!("[Source: {}]\n{}", c.source_title, c.text))
            .collect();
        format!(
            "Relevant building documents:\n{}\n\nUser question: {}",
            context_text.join("\n---\n"),
            req.content
        )
    } else {
        req.content.clone()
    };

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: user_content,
    });

    // Feature flag: allow disabling LLM chat without changing route wiring.
    // Environment variable `LLM_CHAT_ENABLED` acts as the `llm.chat_enabled` flag.
    // Any value equal to "false" or "0" (case-insensitive) disables the LLM call.
    let llm_chat_enabled = std::env::var("LLM_CHAT_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .to_lowercase();
    let llm_chat_enabled = llm_chat_enabled != "false" && llm_chat_enabled != "0";

    // Determine provider and model (default to claude-3-5-haiku-20241022 for cost efficiency)
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| match provider.as_str() {
        "openai" => "gpt-4o-mini".to_string(),
        "azure_openai" => "gpt-4o-mini".to_string(),
        _ => "claude-3-5-haiku-20241022".to_string(),
    });

    // Latency will be populated in either branch below.
    let latency_ms: i32;

    // Handle response; in the disabled case we avoid calling the LLM client entirely.
    let (response_content, confidence, escalated, escalation_reason, tokens_used) =
        if llm_chat_enabled {
            // Story 97.1: Truncate messages to fit within model's context window
            // This preserves the system prompt and most recent messages
            let truncated_messages =
                LlmClient::truncate_messages_to_fit(&messages, &model, MAX_RESPONSE_TOKENS);

            let original_count = messages.len();
            let truncated_count = truncated_messages.len();
            if truncated_count < original_count {
                tracing::info!(
                    "Truncated conversation history from {} to {} messages to fit token limit",
                    original_count,
                    truncated_count
                );
            }

            // Build LLM request with truncated messages
            let llm_request = ChatCompletionRequest {
                model: model.clone(),
                messages: truncated_messages,
                temperature: Some(0.7),
                max_tokens: Some(1024),
            };

            // Measure only LLM call time, not DB operations
            let llm_start_time = Instant::now();
            let llm_result = state.llm_client.chat(&provider, &llm_request).await;
            let elapsed_ms = llm_start_time.elapsed().as_millis();
            latency_ms = std::cmp::min(elapsed_ms, i32::MAX as u128) as i32;

            match llm_result {
                Ok(response) => {
                    let content = response
                        .choices
                        .first()
                        .map(|c| c.message.content.clone())
                        .unwrap_or_else(|| {
                            "I'm sorry, I couldn't generate a response.".to_string()
                        });

                    // Simple escalation detection
                    let needs_escalation = content.to_lowercase().contains("contact management")
                        || content.to_lowercase().contains("escalat")
                        || content.to_lowercase().contains("human assistance")
                        || content.to_lowercase().contains("cannot answer");

                    let escalation_reason = if needs_escalation {
                        Some("Response indicates need for human assistance")
                    } else {
                        None
                    };

                    // Confidence based on whether we had context and if escalation was suggested
                    let confidence = if needs_escalation {
                        0.5
                    } else if !context_chunks.is_empty() {
                        0.9
                    } else {
                        0.75
                    };

                    (
                        content,
                        confidence,
                        needs_escalation,
                        escalation_reason,
                        Some(response.usage.total_tokens),
                    )
                }
                Err(e) => {
                    tracing::error!("LLM call failed: {}", e);
                    (
                        "I'm sorry, I'm having trouble processing your request right now. Please try again later or contact building management for assistance."
                            .to_string(),
                        0.3,
                        true,
                        Some("LLM service unavailable"),
                        None,
                    )
                }
            }
        } else {
            // LLM chat is disabled via feature flag; do not call the LLM provider.
            let elapsed_ms = start_time.elapsed().as_millis();
            latency_ms = std::cmp::min(elapsed_ms, i32::MAX as u128) as i32;
            tracing::info!(
                "LLM chat disabled via feature flag LLM_CHAT_ENABLED; returning fallback response"
            );

            (
                "I'm sorry, I'm currently unavailable because AI chat is disabled. Please contact building management for assistance."
                    .to_string(),
                0.0,
                true,
                Some("LLM chat feature disabled"),
                None,
            )
        };

    // Build sources from context chunks
    let sources: Vec<serde_json::Value> = context_chunks
        .iter()
        .map(|c| {
            serde_json::json!({
                "source_id": c.source_id,
                "title": c.source_title,
                "relevance_score": c.relevance_score
            })
        })
        .collect();

    // Add assistant message
    let assistant_msg = state
        .ai_chat_repo
        .add_message(
            session_id,
            "assistant",
            &response_content,
            Some(confidence),
            sources,
            escalated,
            escalation_reason,
            tokens_used,
            Some(latency_ms),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to add assistant message: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to process message",
                )),
            )
        })?;

    // Story 97.4: Include sentiment analysis in response if available
    let response_json = if let Some(sentiment) = sentiment_analysis {
        serde_json::json!({
            "user_message": user_msg,
            "assistant_message": assistant_msg,
            "provider": provider,
            "model": model,
            "latency_ms": latency_ms,
            "sentiment": {
                "score": sentiment.score,
                "label": sentiment.label,
                "confidence": sentiment.confidence,
                "requires_attention": sentiment.requires_attention
            }
        })
    } else {
        serde_json::json!({
            "user_message": user_msg,
            "assistant_message": assistant_msg,
            "provider": provider,
            "model": model,
            "latency_ms": latency_ms
        })
    };

    Ok((StatusCode::CREATED, Json(response_json)))
}

async fn provide_feedback(
    State(state): State<AppState>,
    Path(message_id): Path<Uuid>,
    Json(req): Json<ProvideFeedback>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let mut feedback = req;
    feedback.message_id = message_id;

    match state.ai_chat_repo.add_feedback(feedback).await {
        Ok(fb) => Ok((StatusCode::CREATED, Json(serde_json::json!(fb)))),
        Err(e) => {
            tracing::error!("Failed to add feedback: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to add feedback",
                )),
            ))
        }
    }
}

async fn list_escalated(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .ai_chat_repo
        .list_escalated_messages(
            tenant.tenant_id,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
    {
        Ok(messages) => Ok(Json(serde_json::json!({ "messages": messages }))),
        Err(e) => {
            tracing::error!("Failed to list escalated: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list escalated",
                )),
            ))
        }
    }
}

// ============================================================================
// Sentiment Router (Story 13.2)
// ============================================================================

pub fn sentiment_router() -> Router<AppState> {
    Router::new()
        .route("/trends", get(get_trends))
        .route("/alerts", get(list_alerts))
        .route("/alerts/{alert_id}/acknowledge", post(acknowledge_alert))
        .route("/thresholds", get(get_thresholds))
        .route("/thresholds", put(update_thresholds))
        .route("/dashboard", get(get_dashboard))
}

async fn get_trends(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SentimentTrendQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .sentiment_repo
        .list_trends(tenant.tenant_id, query)
        .await
    {
        Ok(trends) => Ok(Json(serde_json::json!({ "trends": trends }))),
        Err(e) => {
            tracing::error!("Failed to get trends: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get trends")),
            ))
        }
    }
}

async fn list_alerts(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AlertsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .sentiment_repo
        .list_alerts(
            tenant.tenant_id,
            query.acknowledged,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
    {
        Ok(alerts) => Ok(Json(serde_json::json!({ "alerts": alerts }))),
        Err(e) => {
            tracing::error!("Failed to list alerts: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list alerts",
                )),
            ))
        }
    }
}

async fn acknowledge_alert(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .sentiment_repo
        .acknowledge_alert(alert_id, tenant.user_id)
        .await
    {
        Ok(alert) => Ok(Json(serde_json::json!(alert))),
        Err(e) => {
            tracing::error!("Failed to acknowledge alert: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to acknowledge",
                )),
            ))
        }
    }
}

async fn get_thresholds(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state.sentiment_repo.get_thresholds(tenant.tenant_id).await {
        Ok(thresholds) => Ok(Json(serde_json::json!(thresholds))),
        Err(e) => {
            tracing::error!("Failed to get thresholds: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get thresholds",
                )),
            ))
        }
    }
}

async fn update_thresholds(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateSentimentThresholds>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .sentiment_repo
        .update_thresholds(tenant.tenant_id, req)
        .await
    {
        Ok(thresholds) => Ok(Json(serde_json::json!(thresholds))),
        Err(e) => {
            tracing::error!("Failed to update thresholds: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to update")),
            ))
        }
    }
}

async fn get_dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;
    let org_id = tenant.tenant_id;
    let today = chrono::Utc::now().date_naive();
    let thirty_days_ago = today - chrono::Duration::days(30);

    let org_avg = state
        .sentiment_repo
        .get_org_average_sentiment(org_id, thirty_days_ago, today)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get org average sentiment: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get dashboard",
                )),
            )
        })?;

    let trends = state
        .sentiment_repo
        .list_trends(org_id, SentimentTrendQuery::default())
        .await
        .map_err(|e| {
            tracing::error!("Failed to get trends: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get dashboard",
                )),
            )
        })?;

    let alerts = state
        .sentiment_repo
        .list_alerts(org_id, Some(false), 5, 0)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get alerts: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get dashboard",
                )),
            )
        })?;

    Ok(Json(serde_json::json!({
        "organization_avg": org_avg,
        "trends": trends,
        "recent_alerts": alerts
    })))
}

// ============================================================================
// Equipment Router (Story 13.3)
// ============================================================================

pub fn equipment_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_equipment))
        .route("/", get(list_equipment))
        .route("/{id}", get(get_equipment))
        .route("/{id}", put(update_equipment))
        .route("/{id}", delete(delete_equipment))
        .route("/{id}/maintenance", get(list_maintenance))
        .route("/{id}/maintenance", post(create_maintenance))
        .route("/maintenance/{id}", put(update_maintenance))
        .route("/predictions", get(list_predictions))
        .route(
            "/predictions/{id}/acknowledge",
            post(acknowledge_prediction),
        )
        .route("/needing-maintenance", get(list_needing_maintenance))
}

async fn create_equipment(
    State(state): State<AppState>,
    Json(req): Json<CreateEquipment>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    match state.equipment_repo.create(req).await {
        Ok(equipment) => Ok((StatusCode::CREATED, Json(serde_json::json!(equipment)))),
        Err(e) => {
            tracing::error!("Failed to create equipment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to create")),
            ))
        }
    }
}

async fn list_equipment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<EquipmentQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state.equipment_repo.list(tenant.tenant_id, query).await {
        Ok(equipment) => Ok(Json(serde_json::json!({ "equipment": equipment }))),
        Err(e) => {
            tracing::error!("Failed to list equipment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

async fn get_equipment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.equipment_repo.find_by_id(id).await {
        Ok(Some(equipment)) => Ok(Json(serde_json::json!(equipment))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Equipment not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get equipment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get")),
            ))
        }
    }
}

async fn update_equipment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEquipment>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.equipment_repo.update(id, req).await {
        Ok(equipment) => Ok(Json(serde_json::json!(equipment))),
        Err(e) => {
            tracing::error!("Failed to update equipment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to update")),
            ))
        }
    }
}

async fn delete_equipment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.equipment_repo.delete(id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Equipment not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to delete equipment: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to delete")),
            ))
        }
    }
}

async fn list_maintenance(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .equipment_repo
        .list_maintenance(id, query.limit.unwrap_or(50), query.offset.unwrap_or(0))
        .await
    {
        Ok(records) => Ok(Json(serde_json::json!({ "maintenance": records }))),
        Err(e) => {
            tracing::error!("Failed to list maintenance: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

async fn create_maintenance(
    State(state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(req): Json<CreateMaintenance>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    match state.equipment_repo.create_maintenance(req).await {
        Ok(record) => Ok((StatusCode::CREATED, Json(serde_json::json!(record)))),
        Err(e) => {
            tracing::error!("Failed to create maintenance: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to create")),
            ))
        }
    }
}

async fn update_maintenance(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMaintenance>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.equipment_repo.update_maintenance(id, req).await {
        Ok(record) => Ok(Json(serde_json::json!(record))),
        Err(e) => {
            tracing::error!("Failed to update maintenance: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to update")),
            ))
        }
    }
}

async fn list_predictions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .equipment_repo
        .list_high_risk_predictions(tenant.tenant_id, 50.0, query.limit.unwrap_or(20))
        .await
    {
        Ok(predictions) => Ok(Json(serde_json::json!({ "predictions": predictions }))),
        Err(e) => {
            tracing::error!("Failed to list predictions: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AcknowledgePredictionRequest {
    pub action_taken: Option<String>,
}

async fn acknowledge_prediction(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<AcknowledgePredictionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .equipment_repo
        .acknowledge_prediction(id, tenant.user_id, req.action_taken.as_deref())
        .await
    {
        Ok(prediction) => Ok(Json(serde_json::json!(prediction))),
        Err(e) => {
            tracing::error!("Failed to acknowledge: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to acknowledge",
                )),
            ))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default, utoipa::IntoParams)]
pub struct MaintenanceDueQuery {
    pub days_ahead: Option<i32>,
    pub limit: Option<i64>,
}

async fn list_needing_maintenance(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<MaintenanceDueQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .equipment_repo
        .list_needing_maintenance(
            tenant.tenant_id,
            query.days_ahead.unwrap_or(30),
            query.limit.unwrap_or(20),
        )
        .await
    {
        Ok(equipment) => Ok(Json(serde_json::json!({ "equipment": equipment }))),
        Err(e) => {
            tracing::error!("Failed to list: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

// ============================================================================
// Workflow Router (Story 13.6 & 13.7)
// ============================================================================

pub fn workflow_router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_workflow))
        .route("/", get(list_workflows))
        .route("/{id}", get(get_workflow))
        .route("/{id}", put(update_workflow))
        .route("/{id}", delete(delete_workflow))
        .route("/{id}/actions", get(list_actions))
        .route("/{id}/actions", post(add_action))
        .route("/actions/{action_id}", delete(delete_action))
        .route("/{id}/trigger", post(trigger_workflow))
        .route("/executions", get(list_executions))
        .route("/executions/{id}", get(get_execution))
        .route("/executions/{id}/steps", get(list_execution_steps))
        // Story 94.2: Event handling endpoint
        .route("/events", post(handle_workflow_event))
        // Story 94.4: Templates marketplace
        .route("/templates", get(list_workflow_templates))
        .route("/templates/builtin", get(list_builtin_templates))
        .route("/templates/{id}", get(get_workflow_template))
        .route("/templates/{id}/import", post(import_workflow_template))
}

async fn create_workflow(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkflow>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    match state.workflow_repo.create(req).await {
        Ok(workflow) => Ok((StatusCode::CREATED, Json(serde_json::json!(workflow)))),
        Err(e) => {
            tracing::error!("Failed to create workflow: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to create")),
            ))
        }
    }
}

async fn list_workflows(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<WorkflowQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state.workflow_repo.list(tenant.tenant_id, query).await {
        Ok(workflows) => Ok(Json(serde_json::json!({ "workflows": workflows }))),
        Err(e) => {
            tracing::error!("Failed to list workflows: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

async fn get_workflow(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.workflow_repo.find_by_id(id).await {
        Ok(Some(workflow)) => {
            let actions = state.workflow_repo.list_actions(id).await.map_err(|e| {
                tracing::error!("Failed to list actions: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get")),
                )
            })?;
            Ok(Json(serde_json::json!({
                "workflow": workflow,
                "actions": actions
            })))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Workflow not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get workflow: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get")),
            ))
        }
    }
}

async fn update_workflow(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateWorkflow>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.workflow_repo.update(id, req).await {
        Ok(workflow) => Ok(Json(serde_json::json!(workflow))),
        Err(e) => {
            tracing::error!("Failed to update workflow: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to update")),
            ))
        }
    }
}

async fn delete_workflow(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.workflow_repo.delete(id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Workflow not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to delete workflow: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to delete")),
            ))
        }
    }
}

async fn list_actions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.workflow_repo.list_actions(id).await {
        Ok(actions) => Ok(Json(serde_json::json!({ "actions": actions }))),
        Err(e) => {
            tracing::error!("Failed to list actions: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

async fn add_action(
    State(state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(req): Json<CreateWorkflowAction>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    match state.workflow_repo.add_action(req).await {
        Ok(action) => Ok((StatusCode::CREATED, Json(serde_json::json!(action)))),
        Err(e) => {
            tracing::error!("Failed to add action: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to add")),
            ))
        }
    }
}

async fn delete_action(
    State(state): State<AppState>,
    Path(action_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.workflow_repo.delete_action(action_id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Action not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to delete action: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to delete")),
            ))
        }
    }
}

async fn trigger_workflow(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TriggerWorkflow>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    use crate::services::WorkflowExecutor;

    // Create the workflow executor using the repository
    let executor = WorkflowExecutor::new(state.workflow_repo.clone());

    // Execute the workflow asynchronously (Epic 94)
    match executor
        .execute_workflow(id, req.trigger_event, req.context)
        .await
    {
        Ok(execution_id) => {
            tracing::info!(
                workflow_id = %id,
                execution_id = %execution_id,
                "Workflow triggered successfully"
            );
            Ok((
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "execution_id": execution_id,
                    "workflow_id": id,
                    "status": "running"
                })),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to trigger workflow: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            ))
        }
    }
}

async fn list_executions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ExecutionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .workflow_repo
        .list_executions(tenant.tenant_id, query)
        .await
    {
        Ok(executions) => Ok(Json(serde_json::json!({ "executions": executions }))),
        Err(e) => {
            tracing::error!("Failed to list executions: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

async fn get_execution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.workflow_repo.find_execution_by_id(id).await {
        Ok(Some(execution)) => {
            let steps = state
                .workflow_repo
                .list_execution_steps(id)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to list steps: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get")),
                    )
                })?;
            Ok(Json(serde_json::json!({
                "execution": execution,
                "steps": steps
            })))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Execution not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get execution: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get")),
            ))
        }
    }
}

async fn list_execution_steps(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.workflow_repo.list_execution_steps(id).await {
        Ok(steps) => Ok(Json(serde_json::json!({ "steps": steps }))),
        Err(e) => {
            tracing::error!("Failed to list steps: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

// ============================================================================
// Story 94.2: Workflow Event Handling
// ============================================================================

use db::models::{get_builtin_templates, template_category, ImportTemplateRequest};

/// Request to trigger workflows by event.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowEventRequest {
    /// Event type (matches trigger_type constants)
    pub event_type: String,
    /// Event data/payload
    pub data: serde_json::Value,
    /// Optional building ID
    pub building_id: Option<Uuid>,
}

/// Handle workflow trigger events (Story 94.2).
async fn handle_workflow_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<WorkflowEventRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    use crate::services::{WorkflowEvent, WorkflowExecutor};

    let tenant = extract_tenant_context(&headers)?;

    // Create the workflow executor
    let executor = WorkflowExecutor::new(state.workflow_repo.clone());

    // Create the event
    let mut event = WorkflowEvent::new(&req.event_type, tenant.tenant_id, req.data);
    if let Some(building_id) = req.building_id {
        event = event.with_building(building_id);
    }
    // Set the user who triggered the event
    event = event.with_user(tenant.user_id);

    // Handle the event - this finds matching workflows and executes them
    match executor.handle_event(event).await {
        Ok(execution_ids) => {
            tracing::info!(
                event_type = %req.event_type,
                triggered_count = execution_ids.len(),
                "Workflow event processed"
            );
            Ok((
                StatusCode::OK,
                Json(serde_json::json!({
                    "triggered_workflows": execution_ids.len(),
                    "execution_ids": execution_ids
                })),
            ))
        }
        Err(e) => {
            tracing::error!("Failed to handle workflow event: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", e.to_string())),
            ))
        }
    }
}

// ============================================================================
// Story 94.4: Workflow Templates Marketplace
// ============================================================================

/// Query parameters for template search.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TemplateQuery {
    pub category: Option<String>,
    pub trigger_type: Option<String>,
    pub search: Option<String>,
    pub featured: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// List available workflow templates.
async fn list_workflow_templates(
    Query(query): Query<TemplateQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // For now, return built-in templates
    // In production, this would query from workflow_templates table
    let builtin = get_builtin_templates();

    let templates: Vec<serde_json::Value> = builtin
        .iter()
        .enumerate()
        .filter(|(_, (template, _))| {
            // Apply category filter
            if let Some(ref cat) = query.category {
                if template.category != *cat {
                    return false;
                }
            }
            // Apply trigger type filter
            if let Some(ref trigger) = query.trigger_type {
                if template.trigger_type != *trigger {
                    return false;
                }
            }
            // Apply search filter
            if let Some(ref search) = query.search {
                let search_lower = search.to_lowercase();
                if !template.name.to_lowercase().contains(&search_lower)
                    && !template
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&search_lower))
                        .unwrap_or(false)
                {
                    return false;
                }
            }
            true
        })
        .map(|(idx, (template, actions))| {
            serde_json::json!({
                "id": format!("builtin-{}", idx),
                "name": template.name,
                "description": template.description,
                "category": template.category,
                "trigger_type": template.trigger_type,
                "scope": template.scope,
                "tags": template.tags,
                "icon": template.icon,
                "action_count": actions.len(),
                "featured": false,
                "use_count": 0
            })
        })
        .collect();

    // Apply pagination
    let limit = query.limit.unwrap_or(50) as usize;
    let offset = query.offset.unwrap_or(0) as usize;
    let paginated: Vec<_> = templates.into_iter().skip(offset).take(limit).collect();

    Ok(Json(serde_json::json!({
        "templates": paginated,
        "categories": template_category::ALL
    })))
}

/// Get a specific workflow template.
async fn get_workflow_template(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Handle built-in templates
    if id.starts_with("builtin-") {
        let idx: usize = id
            .strip_prefix("builtin-")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new("INVALID_ID", "Invalid template ID")),
                )
            })?;

        let builtin = get_builtin_templates();
        if let Some((template, actions)) = builtin.get(idx) {
            return Ok(Json(serde_json::json!({
                "id": id,
                "template": {
                    "name": template.name,
                    "description": template.description,
                    "category": template.category,
                    "trigger_type": template.trigger_type,
                    "trigger_config": template.trigger_config,
                    "conditions": template.conditions,
                    "scope": template.scope,
                    "tags": template.tags,
                    "icon": template.icon
                },
                "actions": actions.iter().map(|a| serde_json::json!({
                    "action_order": a.action_order,
                    "action_type": a.action_type,
                    "action_config": a.action_config,
                    "description": a.description,
                    "on_failure": a.on_failure,
                    "retry_count": a.retry_count,
                    "retry_delay_seconds": a.retry_delay_seconds
                })).collect::<Vec<_>>(),
                "variables": []
            })));
        }
    }

    // Template not found
    Err((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
    ))
}

/// Import a workflow template.
async fn import_workflow_template(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ImportTemplateRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    // Get the template
    let (template, actions) = if id.starts_with("builtin-") {
        let idx: usize = id
            .strip_prefix("builtin-")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new("INVALID_ID", "Invalid template ID")),
                )
            })?;

        let builtin = get_builtin_templates();
        builtin.get(idx).cloned().ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
            )
        })?
    } else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
        ));
    };

    // Create the workflow from template
    let workflow_name = req
        .name
        .unwrap_or_else(|| format!("{} (from template)", template.name));

    let create_workflow = CreateWorkflow {
        organization_id: tenant.tenant_id,
        name: workflow_name.clone(),
        description: template.description,
        trigger_type: template.trigger_type,
        trigger_config: template.trigger_config,
        conditions: template.conditions,
        created_by: tenant.user_id,
    };

    // Create the workflow
    let workflow = state
        .workflow_repo
        .create(create_workflow)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create workflow from template: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to import template",
                )),
            )
        })?;

    // Add actions
    for action in actions {
        let create_action = CreateWorkflowAction {
            workflow_id: workflow.id,
            action_order: action.action_order,
            action_type: action.action_type,
            action_config: action.action_config,
            on_failure: action.on_failure,
            retry_count: action.retry_count,
            retry_delay_seconds: action.retry_delay_seconds,
        };

        if let Err(e) = state.workflow_repo.add_action(create_action).await {
            tracing::warn!("Failed to add action to imported workflow: {}", e);
        }
    }

    // Enable if requested
    if req.enabled.unwrap_or(false) {
        let _ = state
            .workflow_repo
            .update(
                workflow.id,
                UpdateWorkflow {
                    name: None,
                    description: None,
                    trigger_type: None,
                    trigger_config: None,
                    conditions: None,
                    enabled: Some(true),
                },
            )
            .await;
    }

    tracing::info!(
        workflow_id = %workflow.id,
        template_id = %id,
        workflow_name = %workflow_name,
        "Workflow imported from template"
    );

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "workflow_id": workflow.id,
            "name": workflow_name,
            "enabled": req.enabled.unwrap_or(false),
            "imported_from": id
        })),
    ))
}

/// List all built-in templates.
async fn list_builtin_templates(
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let builtin = get_builtin_templates();

    let templates: Vec<serde_json::Value> = builtin
        .iter()
        .enumerate()
        .map(|(idx, (template, actions))| {
            serde_json::json!({
                "id": format!("builtin-{}", idx),
                "name": template.name,
                "description": template.description,
                "category": template.category,
                "trigger_type": template.trigger_type,
                "tags": template.tags,
                "icon": template.icon,
                "action_count": actions.len()
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "templates": templates,
        "categories": template_category::ALL
    })))
}

// ============================================================================
// Epic 64: Advanced AI & LLM Capabilities
// ============================================================================

// Story 64.1: LLM-Powered Lease Agreement Generation
// Story 64.2: AI Property Listing Description Generator
// Story 64.3: Conversational AI Tenant Support (Enhanced RAG)
// Story 64.4: AI Photo Enhancement for Listings
// Story 64.5: Voice Assistant Integration

use db::models::{
    EnhancePhotoRequest, EnhancedChatRequest, GenerateLeaseRequest,
    GenerateListingDescriptionRequest, LinkVoiceDeviceRequest, UpdateEscalationConfig,
};

/// Router for LLM document generation (Epic 64).
pub fn llm_router() -> Router<AppState> {
    Router::new()
        // Lease generation (Story 64.1)
        .route("/lease/generate", post(generate_lease))
        .route("/lease/templates", get(list_lease_templates))
        .route("/lease/templates/{id}", get(get_lease_template))
        // Listing descriptions (Story 64.2)
        .route("/listing/description", post(generate_listing_description))
        .route(
            "/listing/descriptions/{listing_id}",
            get(list_listing_descriptions),
        )
        .route(
            "/listing/descriptions/{id}/publish",
            post(publish_description),
        )
        // Enhanced chat (Story 64.3)
        .route("/chat/enhanced", post(enhanced_chat))
        .route("/chat/escalation-config", get(get_escalation_config))
        .route("/chat/escalation-config", put(update_escalation_config))
        // Photo enhancement (Story 64.4)
        .route("/photos/enhance", post(enhance_photo))
        .route("/photos/enhance/batch", post(batch_enhance_photos))
        .route("/photos/{id}", get(get_photo_enhancement))
        // Voice assistant (Story 64.5)
        .route("/voice/devices", get(list_voice_devices))
        .route("/voice/devices", post(link_voice_device))
        .route("/voice/devices/{id}", delete(unlink_voice_device))
        .route("/voice/commands/{device_id}", get(list_voice_commands))
        // Statistics
        .route("/statistics", get(get_ai_statistics))
        .route("/requests", get(list_generation_requests))
        .route("/requests/{id}", get(get_generation_request))
}

// ============================================================================
// Story 64.1 + Epic 92.1: Lease Generation Endpoints
// ============================================================================

#[utoipa::path(
    post,
    path = "/api/v1/ai/llm/lease/generate",
    request_body = GenerateLeaseRequest,
    responses(
        (status = 201, description = "Lease agreement generated"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "AI LLM"
)]
async fn generate_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<GenerateLeaseRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;
    let start_time = Instant::now();

    // Check feature flag for document generation
    let doc_gen_enabled = std::env::var("LLM_DOCUMENT_GENERATION")
        .unwrap_or_else(|_| "true".to_string())
        .to_lowercase();
    let doc_gen_enabled = doc_gen_enabled != "false" && doc_gen_enabled != "0";

    // Determine provider and model
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| match provider.as_str() {
        "openai" => "gpt-4o".to_string(),
        "azure_openai" => "gpt-4o".to_string(),
        _ => "claude-3-5-sonnet-20241022".to_string(),
    });

    // Create a generation request record
    let input_data = serde_json::to_value(&req).unwrap_or_default();
    let request = state
        .llm_document_repo
        .create_generation_request(
            tenant.tenant_id,
            tenant.user_id,
            "lease_generation",
            &provider,
            &model,
            input_data.clone(),
            req.template_id,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to create generation request: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create request",
                )),
            )
        })?;

    if !doc_gen_enabled {
        tracing::info!("LLM document generation disabled via feature flag");
        return Ok((
            StatusCode::CREATED,
            Json(serde_json::json!({
                "request_id": request.id,
                "status": "pending",
                "message": "Lease generation is currently disabled. Request queued for later processing."
            })),
        ));
    }

    // Epic 92.1: Generate lease using LLM
    let jurisdiction = req.jurisdiction.as_deref().unwrap_or("SK");
    let lease_input = integrations::LeaseGenerationInput {
        unit_id: req.unit_id,
        landlord_name: req.landlord_name.clone(),
        landlord_address: req.landlord_address.clone(),
        tenant_name: req.tenant_name.clone(),
        tenant_email: req.tenant_email.clone(),
        tenant_phone: req.tenant_phone.clone(),
        start_date: req.start_date.clone(),
        end_date: req.end_date.clone(),
        monthly_rent: req.monthly_rent,
        security_deposit: req.security_deposit,
        currency: req.currency.clone(),
        additional_terms: req.additional_terms.clone(),
        include_pet_clause: req.include_pet_clause,
        include_parking: req.include_parking,
        jurisdiction: Some(jurisdiction.to_string()),
    };

    let system_prompt = build_lease_system_prompt(jurisdiction, &req.language);

    let result = state
        .llm_client
        .generate_lease(
            &provider,
            &model,
            &system_prompt,
            &lease_input,
            &req.language,
        )
        .await;

    let latency_ms = start_time.elapsed().as_millis() as i32;

    match result {
        Ok(lease_result) => {
            // Update the generation request with the result
            let result_json = serde_json::to_value(&lease_result).unwrap_or_default();
            let _ = state
                .llm_document_repo
                .update_generation_status(
                    request.id,
                    "completed",
                    Some(result_json.clone()),
                    None,
                    Some(lease_result.tokens_used),
                    None,
                    Some(latency_ms),
                )
                .await;

            tracing::info!(
                "Lease generated successfully for unit {} (tokens: {}, latency: {}ms)",
                req.unit_id,
                lease_result.tokens_used,
                latency_ms
            );

            Ok((
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "request_id": request.id,
                    "status": "completed",
                    "document_html": lease_result.document_html,
                    "document_text": lease_result.document_text,
                    "clauses": lease_result.clauses,
                    "warnings": lease_result.warnings,
                    "compliance_notes": lease_result.compliance_notes,
                    "tokens_used": lease_result.tokens_used,
                    "latency_ms": latency_ms,
                    "provider": provider,
                    "model": model
                })),
            ))
        }
        Err(e) => {
            tracing::error!("Lease generation failed: {}", e);
            let error_msg = format!("{}", e);

            // Update the generation request with the error
            let _ = state
                .llm_document_repo
                .update_generation_status(
                    request.id,
                    "failed",
                    None,
                    Some(&error_msg),
                    None,
                    None,
                    Some(latency_ms),
                )
                .await;

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "GENERATION_FAILED",
                    format!("Failed to generate lease: {}", e),
                )),
            ))
        }
    }
}

/// Build the system prompt for lease generation based on jurisdiction.
fn build_lease_system_prompt(jurisdiction: &str, language: &str) -> String {
    let legal_framework = match jurisdiction {
        "SK" => "Slovak Civil Code (Obciansky zakonnik) and Act No. 116/1990 Coll. on Rental and Sub-rental of Non-residential Premises",
        "CZ" => "Czech Civil Code (Obcansky zakonik) No. 89/2012 Coll., specifically sections 2235-2301",
        "DE" => "German Civil Code (BGB), Mietrecht (sections 535-580a)",
        _ => "applicable local tenancy laws",
    };

    let language_name = match language {
        "sk" => "Slovak",
        "cs" => "Czech",
        "de" => "German",
        _ => "English",
    };

    format!(
        r#"You are an expert legal document assistant specializing in residential lease agreements for Central European jurisdictions.

Your task is to generate a comprehensive lease agreement that complies with {}.

Requirements:
1. Generate the document in {} language
2. Include all mandatory clauses required by law
3. Use clear, professional legal language appropriate for the jurisdiction
4. Include proper formatting with numbered sections
5. Add placeholders for signatures and dates
6. Include any required notices or disclosures

The generated lease should be ready for review and signing, with proper legal structure and comprehensive terms covering:
- Parties and property identification
- Lease term and renewal conditions
- Rent amount, payment terms, and late fees
- Security deposit terms and conditions for return
- Maintenance responsibilities
- Rules for property use
- Termination conditions
- Dispute resolution procedures

Respond with well-structured content that can be converted to a professional document."#,
        legal_framework, language_name
    )
}

async fn list_lease_templates(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .llm_document_repo
        .list_prompt_templates(Some(tenant.tenant_id), Some("lease_generation"))
        .await
    {
        Ok(templates) => Ok(Json(serde_json::json!({ "templates": templates }))),
        Err(e) => {
            tracing::error!("Failed to list templates: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list templates",
                )),
            ))
        }
    }
}

async fn get_lease_template(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.llm_document_repo.find_prompt_template(id).await {
        Ok(Some(template)) => Ok(Json(serde_json::json!(template))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Template not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get template: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get template",
                )),
            ))
        }
    }
}

// ============================================================================
// Story 64.2 + Epic 92.2: Listing Description Endpoints
// ============================================================================

#[utoipa::path(
    post,
    path = "/api/v1/ai/llm/listing/description",
    request_body = GenerateListingDescriptionRequest,
    responses(
        (status = 201, description = "Description generated"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "AI LLM"
)]
async fn generate_listing_description(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<GenerateListingDescriptionRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;
    let start_time = Instant::now();

    // Check feature flag for document generation
    let doc_gen_enabled = std::env::var("LLM_DOCUMENT_GENERATION")
        .unwrap_or_else(|_| "true".to_string())
        .to_lowercase();
    let doc_gen_enabled = doc_gen_enabled != "false" && doc_gen_enabled != "0";

    // Determine provider and model
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| match provider.as_str() {
        "openai" => "gpt-4o".to_string(),
        "azure_openai" => "gpt-4o".to_string(),
        _ => "claude-3-5-sonnet-20241022".to_string(),
    });

    // Create a generation request
    let input_data = serde_json::to_value(&req).unwrap_or_default();
    let request = state
        .llm_document_repo
        .create_generation_request(
            tenant.tenant_id,
            tenant.user_id,
            "listing_description",
            &provider,
            &model,
            input_data.clone(),
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to create generation request: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create request",
                )),
            )
        })?;

    // Epic 92.2: Generate listing description using LLM
    let style_str = req.style.as_ref().and_then(|s| s.tone.as_deref());
    let system_prompt = build_listing_system_prompt(style_str);

    let listing_input = integrations::ListingDescriptionInput {
        property_type: req.property_type.clone(),
        transaction_type: req.transaction_type.clone(),
        size_sqm: req.size_sqm,
        rooms: req.rooms,
        bathrooms: req.bathrooms,
        floor: req.floor,
        total_floors: req.total_floors,
        features: req.features.clone(),
        city: req.location.city.clone(),
        district: req.location.district.clone(),
        nearby_amenities: req.location.nearby_amenities.clone(),
        price: req.price,
        currency: req.currency.clone(),
        language: req.language.clone(),
        style: req.style.as_ref().and_then(|s| s.tone.clone()),
        max_length: req.max_length,
    };

    // Generate description (or use placeholder if disabled)
    let (description_text, tokens_used) = if doc_gen_enabled {
        match state
            .llm_client
            .generate_listing_description(&provider, &model, &system_prompt, &listing_input)
            .await
        {
            Ok(result) => (result.description, result.tokens_used),
            Err(e) => {
                tracing::warn!("LLM listing generation failed, using fallback: {}", e);
                // Fallback to placeholder
                let placeholder = format!(
                    "Beautiful {} {} in {} with {} rooms. This property offers {} sqm of living space with modern amenities.",
                    req.property_type,
                    if req.transaction_type == "sale" { "for sale" } else { "for rent" },
                    req.location.city,
                    req.rooms.unwrap_or(0),
                    req.size_sqm.unwrap_or(0.0)
                );
                (placeholder, 0)
            }
        }
    } else {
        let placeholder = format!(
            "Beautiful {} {} in {} with {} rooms. This property offers {} sqm of living space with modern amenities.",
            req.property_type,
            if req.transaction_type == "sale" { "for sale" } else { "for rent" },
            req.location.city,
            req.rooms.unwrap_or(0),
            req.size_sqm.unwrap_or(0.0)
        );
        (placeholder, 0)
    };

    let latency_ms = start_time.elapsed().as_millis() as i32;

    // Store the generated description
    let description = state
        .llm_document_repo
        .create_listing_description(
            tenant.tenant_id,
            req.listing_id,
            tenant.user_id,
            &req.language,
            &description_text,
            input_data,
            Some(tokens_used.into()),
            request.id,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to store description: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to store description",
                )),
            )
        })?;

    // Update the generation request status
    let _ = state
        .llm_document_repo
        .update_generation_status(
            request.id,
            "completed",
            Some(serde_json::json!({ "description": description_text })),
            None,
            Some(tokens_used),
            None,
            Some(latency_ms),
        )
        .await;

    tracing::info!(
        "Listing description generated for listing {:?} in {} (tokens: {}, latency: {}ms)",
        req.listing_id,
        req.language,
        tokens_used,
        latency_ms
    );

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": description.id,
            "description": description_text,
            "request_id": request.id,
            "language": req.language,
            "tokens_used": tokens_used,
            "latency_ms": latency_ms,
            "provider": provider,
            "model": model
        })),
    ))
}

/// Build the system prompt for listing description generation.
fn build_listing_system_prompt(style: Option<&str>) -> String {
    let style_instruction = match style {
        Some("luxury") => "Use elegant, sophisticated language that appeals to high-end buyers. Emphasize exclusivity, premium finishes, and prestigious location.",
        Some("casual") => "Use friendly, approachable language. Focus on livability and community aspects.",
        Some("investment") => "Focus on investment potential, rental yields, and market position. Include relevant statistics.",
        _ => "Use professional, engaging language that highlights the property's best features while remaining factual.",
    };

    format!(
        r#"You are an expert real estate copywriter specializing in property listings for Central European markets.

Your task is to generate compelling property listing descriptions that attract potential buyers or renters.

Style Guidelines:
{}

Requirements:
1. Generate a main description (150-300 words unless specified otherwise)
2. Create 3-5 key highlights as bullet points
3. Suggest an attention-grabbing title
4. Provide SEO-friendly keywords
5. Highlight unique selling points
6. Include location benefits
7. Use appropriate language for the target market

The description should:
- Be engaging and persuasive
- Be accurate and not misleading
- Follow real estate advertising best practices
- Be culturally appropriate for the Central European market

Format your response with clear sections for each component."#,
        style_instruction
    )
}

async fn list_listing_descriptions(
    State(state): State<AppState>,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .llm_document_repo
        .list_listing_descriptions(listing_id)
        .await
    {
        Ok(descriptions) => Ok(Json(serde_json::json!({ "descriptions": descriptions }))),
        Err(e) => {
            tracing::error!("Failed to list descriptions: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

async fn publish_description(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.llm_document_repo.publish_description(id).await {
        Ok(Some(desc)) => Ok(Json(serde_json::json!(desc))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Description not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to publish description: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to publish")),
            ))
        }
    }
}

// ============================================================================
// Story 64.3: Enhanced Chat (RAG) Endpoints
// ============================================================================

#[utoipa::path(
    post,
    path = "/api/v1/ai/llm/chat/enhanced",
    request_body = EnhancedChatRequest,
    responses(
        (status = 200, description = "Chat response with context"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "AI LLM"
)]
async fn enhanced_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<EnhancedChatRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    // Get escalation config
    let config = state
        .llm_document_repo
        .get_escalation_config(tenant.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get escalation config: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get config")),
            )
        })?;

    // Placeholder response - real implementation would:
    // 1. Search document embeddings for relevant context
    // 2. Call LLM with context
    // 3. Check confidence against threshold
    let confidence = 0.85; // Placeholder
    let escalated = confidence < config.confidence_threshold;

    let response = serde_json::json!({
        "message_id": Uuid::new_v4(),
        "response": format!("I understand you're asking about: {}. Let me help you with that.", req.message),
        "confidence": confidence,
        "sources": [],
        "escalated": escalated,
        "escalation_reason": if escalated { Some("Low confidence in response") } else { None },
        "language_detected": req.language,
        "tokens_used": 150
    });

    Ok(Json(response))
}

async fn get_escalation_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .llm_document_repo
        .get_escalation_config(tenant.tenant_id)
        .await
    {
        Ok(config) => Ok(Json(serde_json::json!(config))),
        Err(e) => {
            tracing::error!("Failed to get config: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get config")),
            ))
        }
    }
}

async fn update_escalation_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateEscalationConfig>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .llm_document_repo
        .update_escalation_config(tenant.tenant_id, req)
        .await
    {
        Ok(config) => Ok(Json(serde_json::json!(config))),
        Err(e) => {
            tracing::error!("Failed to update config: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update config",
                )),
            ))
        }
    }
}

// ============================================================================
// Story 64.4: Photo Enhancement Endpoints
// ============================================================================

#[utoipa::path(
    post,
    path = "/api/v1/ai/llm/photos/enhance",
    request_body = EnhancePhotoRequest,
    responses(
        (status = 201, description = "Photo enhancement started"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "AI LLM"
)]
async fn enhance_photo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<EnhancePhotoRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    let metadata = serde_json::to_value(&req.options).unwrap_or_default();

    let enhancement = state
        .llm_document_repo
        .create_photo_enhancement(
            tenant.tenant_id,
            req.listing_id,
            tenant.user_id,
            &req.photo_url,
            &req.enhancement_type,
            metadata,
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to create enhancement: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create enhancement",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": enhancement.id,
            "status": enhancement.status,
            "is_ai_enhanced": true,
            "message": "Photo enhancement started. Check status for completion."
        })),
    ))
}

async fn batch_enhance_photos(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<db::models::BatchEnhancePhotosRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    let mut enhancements = Vec::new();
    for photo_url in &req.photo_urls {
        let enhancement = state
            .llm_document_repo
            .create_photo_enhancement(
                tenant.tenant_id,
                req.listing_id,
                tenant.user_id,
                photo_url,
                &req.enhancement_type,
                serde_json::json!({}),
            )
            .await
            .map_err(|e| {
                tracing::error!("Failed to create enhancement: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "INTERNAL_ERROR",
                        "Failed to create enhancement",
                    )),
                )
            })?;
        enhancements.push(serde_json::json!({
            "id": enhancement.id,
            "status": enhancement.status,
            "original_url": photo_url
        }));
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "batch_id": Uuid::new_v4(),
            "total_photos": req.photo_urls.len(),
            "enhancements": enhancements
        })),
    ))
}

async fn get_photo_enhancement(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.llm_document_repo.find_photo_enhancement(id).await {
        Ok(Some(enhancement)) => Ok(Json(serde_json::json!(enhancement))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Enhancement not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get enhancement: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get")),
            ))
        }
    }
}

// ============================================================================
// Story 64.5: Voice Assistant Endpoints
// ============================================================================

async fn list_voice_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .llm_document_repo
        .list_user_voice_devices(tenant.user_id)
        .await
    {
        Ok(devices) => Ok(Json(serde_json::json!({ "devices": devices }))),
        Err(e) => {
            tracing::error!("Failed to list devices: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/ai/llm/voice/devices",
    request_body = LinkVoiceDeviceRequest,
    responses(
        (status = 201, description = "Voice device linked"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "AI Voice"
)]
async fn link_voice_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LinkVoiceDeviceRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    // Generate a unique device ID: platform prefix + UUID for debugging and uniqueness.
    // Format: "google_assistant_550e8400-e29b-41d4-a716-446655440000"
    // While longer than a plain UUID, the platform prefix aids in debugging and log analysis.
    let device_id = format!("{}_{}", req.platform, Uuid::new_v4());

    // Story 98.1: Implement OAuth token exchange using auth_code from request.
    // Exchange the authorization code for access and refresh tokens from the voice platform.
    let (access_token_encrypted, refresh_token_encrypted, token_expires_at) =
        exchange_voice_oauth_tokens(&req.platform, &req.auth_code).await?;

    let device = state
        .llm_document_repo
        .create_voice_device(
            tenant.tenant_id,
            tenant.user_id,
            req.unit_id,
            &req.platform,
            &device_id,
            req.device_name.as_deref(),
            access_token_encrypted.as_deref(),
            refresh_token_encrypted.as_deref(),
            token_expires_at,
            serde_json::json!(["check_balance", "report_fault", "check_announcements"]),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to link device: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to link device",
                )),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "device_id": device.id,
            "platform": device.platform,
            "device_name": device.device_name,
            "capabilities": ["check_balance", "report_fault", "check_announcements"],
            "linked_at": device.linked_at,
            "oauth_linked": access_token_encrypted.is_some()
        })),
    ))
}

/// Exchange OAuth authorization code for tokens from voice platform.
/// Story 98.1: Voice Device OAuth Token Exchange
async fn exchange_voice_oauth_tokens(
    platform: &str,
    auth_code: &str,
) -> Result<
    (
        Option<String>,
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
    ),
    (StatusCode, Json<ErrorResponse>),
> {
    use integrations::{encrypt_if_available, IntegrationCrypto, VoiceOAuthManager, VoicePlatform};

    // Parse the platform
    let voice_platform: VoicePlatform = platform.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_PLATFORM",
                "Unsupported voice platform",
            )),
        )
    })?;

    // Get OAuth manager from environment
    let oauth_manager = VoiceOAuthManager::from_env();

    // Check if platform is configured
    if !oauth_manager.has_platform(voice_platform) {
        // Platform not configured - store device without OAuth tokens
        // This allows development/testing without OAuth credentials
        tracing::warn!(
            "Voice OAuth not configured for platform {}, storing device without tokens",
            platform
        );
        return Ok((None, None, None));
    }

    // Get redirect URI from environment (platform-specific)
    let redirect_uri = match voice_platform {
        VoicePlatform::Alexa => std::env::var("ALEXA_REDIRECT_URI").unwrap_or_else(|_| {
            "https://ppt.three-two-bit.com/api/v1/webhooks/voice/oauth/callback".to_string()
        }),
        VoicePlatform::GoogleAssistant => std::env::var("GOOGLE_VOICE_REDIRECT_URI")
            .unwrap_or_else(|_| {
                "https://ppt.three-two-bit.com/api/v1/webhooks/voice/oauth/callback".to_string()
            }),
    };

    // Exchange the authorization code for tokens
    let tokens = oauth_manager
        .exchange_code(voice_platform, auth_code, &redirect_uri)
        .await
        .map_err(|e| {
            tracing::error!("OAuth token exchange failed for {}: {}", platform, e);
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "OAUTH_EXCHANGE_FAILED",
                    format!("Failed to exchange authorization code: {}", e),
                )),
            )
        })?;

    // Encrypt tokens for storage
    let crypto = IntegrationCrypto::try_from_env();
    let access_encrypted = encrypt_if_available(crypto.as_ref(), &tokens.access_token);
    let refresh_encrypted = tokens
        .refresh_token
        .as_ref()
        .map(|rt| encrypt_if_available(crypto.as_ref(), rt));

    tracing::info!(
        "Successfully exchanged OAuth tokens for voice platform {}",
        platform
    );

    Ok((Some(access_encrypted), refresh_encrypted, tokens.expires_at))
}

async fn unlink_voice_device(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.llm_document_repo.deactivate_voice_device(id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Device not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to unlink device: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to unlink")),
            ))
        }
    }
}

async fn list_voice_commands(
    State(state): State<AppState>,
    Path(device_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state
        .llm_document_repo
        .list_voice_commands(
            device_id,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
    {
        Ok(commands) => Ok(Json(serde_json::json!({ "commands": commands }))),
        Err(e) => {
            tracing::error!("Failed to list commands: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

// ============================================================================
// Statistics and Requests Endpoints
// ============================================================================

async fn get_ai_statistics(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .llm_document_repo
        .get_usage_statistics(tenant.tenant_id, None, None)
        .await
    {
        Ok(stats) => Ok(Json(serde_json::json!(stats))),
        Err(e) => {
            tracing::error!("Failed to get statistics: {}", e);
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

#[derive(Debug, Serialize, Deserialize, Default, utoipa::IntoParams)]
pub struct GenerationRequestsQuery {
    pub request_type: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

async fn list_generation_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<GenerationRequestsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = extract_tenant_context(&headers)?;

    match state
        .llm_document_repo
        .list_generation_requests(
            tenant.tenant_id,
            query.request_type.as_deref(),
            query.status.as_deref(),
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
    {
        Ok(requests) => Ok(Json(serde_json::json!({ "requests": requests }))),
        Err(e) => {
            tracing::error!("Failed to list requests: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list")),
            ))
        }
    }
}

async fn get_generation_request(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.llm_document_repo.find_generation_request(id).await {
        Ok(Some(request)) => Ok(Json(serde_json::json!(request))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Request not found")),
        )),
        Err(e) => {
            tracing::error!("Failed to get request: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get")),
            ))
        }
    }
}
