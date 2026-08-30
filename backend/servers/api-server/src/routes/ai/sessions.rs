//! AI chat sessions (Story 13.1) and sentiment analysis (Story 13.2).

use crate::routes::ai::{AlertsQuery, PaginationQuery};
use crate::routes::pagination::clamp_limit;
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use common::TenantRole;
use db::models::{alert_type, CreateSentimentAlert, UpsertSentimentTrend};
use db::models::{
    CreateChatSession, ProvideFeedback, SendChatMessage, SentimentTrendQuery,
    UpdateSentimentThresholds,
};
use integrations::{ChatCompletionRequest, ChatMessage, ContextChunk, LlmClient, TenantAiConfig};
use std::time::Instant;
use uuid::Uuid;

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
    mut rls: RlsConnection,
    Json(req): Json<CreateChatSession>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: the owning org/user are derived from the RLS-validated request
    // context, never trusted from the request body (prevents IDOR / cross-tenant
    // writes). The connection carries RLS context so the INSERT also satisfies
    // the FORCE-RLS `WITH CHECK` policy.
    let organization_id = rls.tenant_id();
    let user_id = rls.user_id();
    let result = state
        .ai_chat_repo
        .create_session(&mut **rls.conn(), organization_id, user_id, req)
        .await;
    rls.release().await;
    match result {
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
    mut rls: RlsConnection,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let user_id = rls.user_id();
    let result = state
        .ai_chat_repo
        .list_user_sessions(
            &mut **rls.conn(),
            user_id,
            clamp_limit(query.limit, 50),
            query.offset.unwrap_or(0),
        )
        .await;
    rls.release().await;
    match result {
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
    mut rls: RlsConnection,
    Path(session_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY (#2279): AI chat sessions are per-user private within an org.
    // Scope the by-id lookup by BOTH org and owner so a colleague in the same
    // org cannot read another member's private session by supplying its UUID.
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let result = state
        .ai_chat_repo
        .find_session_by_id(&mut **rls.conn(), session_id, org_id, user_id)
        .await;
    rls.release().await;
    match result {
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
    mut rls: RlsConnection,
    Path(session_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY (#2279): scope the destructive delete by BOTH org and owner so
    // a colleague in the same org cannot delete another member's session.
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let result = state
        .ai_chat_repo
        .delete_session(&mut **rls.conn(), session_id, org_id, user_id)
        .await;
    rls.release().await;
    match result {
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
    mut rls: RlsConnection,
    Path(session_id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY (#2279): scope the transcript read by BOTH org and owner so a
    // colleague in the same org cannot read another member's conversation.
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let result = state
        .ai_chat_repo
        .list_session_messages(
            &mut **rls.conn(),
            session_id,
            org_id,
            user_id,
            clamp_limit(query.limit, 100),
            query.offset.unwrap_or(0),
        )
        .await;
    rls.release().await;
    match result {
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
    mut rls: RlsConnection,
    Path(session_id): Path<Uuid>,
    Json(req): Json<SendChatMessage>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let start_time = Instant::now();

    // SECURITY: tenant is now derived from the RLS-validated request context,
    // never from a client-supplied header. Per-tenant features (RAG, sentiment
    // trend bookkeeping, custom system prompt) require a resolved org; a platform
    // principal on the platform host has no `effective_org` and is rejected by
    // the `RlsConnection` extractor itself.
    //
    // The RLS connection is held for the whole handler (including the LLM call)
    // because the AI-chat reads/writes must run on a context-set connection.
    // Early `?` returns leave cleanup to `RlsConnection::drop` (spawns a
    // context-clear task); the happy path calls `rls.release().await` explicitly.
    let tenant_id = rls.tenant_id();
    let user_id = rls.user_id();

    // Verify session exists and belongs to this tenant AND this user.
    // SECURITY (#2279): AI chat sessions are per-user private; scoping by org
    // alone let any colleague post into another member's private session.
    let _session = state
        .ai_chat_repo
        .find_session_by_id(&mut **rls.conn(), session_id, tenant_id, user_id)
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
        .list_session_messages(
            &mut **rls.conn(),
            session_id,
            tenant_id,
            user_id,
            DEFAULT_HISTORY_LIMIT,
            0,
        )
        .await
        .unwrap_or_default();

    // Add user message after fetching history to avoid duplication
    let user_msg = state
        .ai_chat_repo
        .add_message(
            rls.conn(),
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
    let sentiment_analysis = {
        // Check if sentiment analysis is enabled via feature flag
        let sentiment_enabled = std::env::var("SENTIMENT_ANALYSIS_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .to_lowercase();
        let analyze_sentiment = sentiment_enabled != "false" && sentiment_enabled != "0";

        if analyze_sentiment {
            match state.llm_client.analyze_sentiment(&req.content, None).await {
                Ok(result) => {
                    // PAP-80: the sentiment tables are FORCE-RLS, so these
                    // best-effort writes must run on a connection that carries
                    // the org GUC (the raw pool collapses to deny-all). Acquire a
                    // short-lived RLS-context connection scoped to this tenant.
                    // PAP-150: short-lived org-context guard (acquire + set RLS
                    // context + clear-on-release) instead of a handler-side raw
                    // `state.db.acquire()` + manual set/clear of the GUCs.
                    match db::RlsPool::new(state.db.clone())
                        .acquire_with_rls(tenant_id, user_id, false)
                        .await
                    {
                        Ok(mut sguard) => {
                            // Check if alert should be triggered based on organization thresholds
                            if let Ok(thresholds) = state
                                .sentiment_repo
                                .get_thresholds(&mut *sguard.conn(), tenant_id)
                                .await
                            {
                                if thresholds.enabled {
                                    // Story 97.4: Check if sentiment requires attention and create alert
                                    let should_alert = result.requires_attention
                                        || (result.score < 0.0
                                            && result.score.abs() >= thresholds.negative_threshold);

                                    if should_alert {
                                        let alert = CreateSentimentAlert {
                                            organization_id: tenant_id,
                                            building_id: None, // Could be extracted from session context
                                            alert_type: alert_type::SPIKE_NEGATIVE.to_string(),
                                            threshold_breached: thresholds.negative_threshold,
                                            current_sentiment: result.score,
                                            previous_sentiment: None,
                                            sample_message_ids: vec![user_msg.id],
                                        };

                                        if let Err(e) = state
                                            .sentiment_repo
                                            .create_alert(&mut **sguard.conn(), alert)
                                            .await
                                        {
                                            tracing::warn!(
                                                "Failed to create sentiment alert: {}",
                                                e
                                            );
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
                                        organization_id: tenant_id,
                                        building_id: None,
                                        date: today,
                                        avg_sentiment: result.score,
                                        message_count: 1,
                                        negative_count: neg,
                                        neutral_count: neut,
                                        positive_count: pos,
                                    };

                                    if let Err(e) = state
                                        .sentiment_repo
                                        .upsert_trend(&mut **sguard.conn(), trend_data)
                                        .await
                                    {
                                        tracing::warn!("Failed to update sentiment trend: {}", e);
                                    }
                                }
                            }

                            // Clear RLS context + return the connection to the pool.
                            sguard.release().await;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to acquire connection for sentiment write: {}",
                                e
                            );
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
    };

    // Story 97.1: Build tenant-specific system prompt
    // Load tenant AI configuration if available (custom personality, building context).
    // Tenant is always present now (auth required); kept the wrapper for the
    // future per-tenant DB lookup so the option type is the right shape.
    let tenant_config = {
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

    // Story 97.2: Search for relevant documents using RAG with semantic similarity.
    // Scoped to `tenant_id` derived from the verified principal.
    //
    // PAP-108 (PAP-80): `document_embeddings` is FORCE-RLS (migration 00179),
    // so these reads must run on a connection that carries the org GUC — on the
    // raw pool the policy collapsed to deny-all and RAG silently returned no
    // context. Acquire a short-lived RLS-context connection scoped to this
    // tenant (same pattern as the sentiment block above).
    let mut context_chunks: Vec<ContextChunk> = vec![];
    {
        // Check if semantic search is enabled via feature flag
        let semantic_search_enabled = std::env::var("RAG_SEMANTIC_SEARCH_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .to_lowercase();
        let use_semantic_search =
            semantic_search_enabled != "false" && semantic_search_enabled != "0";

        // PAP-150: short-lived org-context guard (acquire + set RLS context +
        // clear-on-release) instead of a handler-side raw `state.db.acquire()`
        // + manual set/clear of the GUCs.
        let mut rag_guard = match db::RlsPool::new(state.db.clone())
            .acquire_with_rls(tenant_id, user_id, false)
            .await
        {
            Ok(guard) => Some(guard),
            Err(e) => {
                tracing::warn!("Failed to acquire connection for RAG search: {}", e);
                None
            }
        };

        if let Some(guard) = rag_guard.as_mut() {
            if use_semantic_search {
                // Try semantic similarity search first (Story 97.2)
                // Generate embedding for the user's query
                match state
                    .llm_client
                    .generate_embedding(&req.content, None)
                    .await
                {
                    Ok(embedding_result) => {
                        // Search documents by embedding similarity.
                        //
                        // Story 84.5 / 103.5: prefer the pgvector-native path
                        // (`search_similar_documents` SQL function, IVFFlat cosine
                        // index) which pushes the top-k similarity search into
                        // Postgres instead of streaming every org row into Rust.
                        // `search_documents_pgvector` transparently falls back to
                        // the application-level cosine scan when the pgvector
                        // extension / function is absent, so this is a safe swap.
                        match state
                            .llm_document_repo
                            .search_documents_pgvector(
                                &mut *guard.conn(),
                                tenant_id,
                                &embedding_result.embedding,
                                5,         // Get top 5 relevant chunks
                                Some(0.6), // Minimum similarity threshold
                                // Provenance (#2201): only compare against rows
                                // embedded with the same model — mixing stub and
                                // OpenAI vectors (both 1536-dim) returns garbage.
                                Some(embedding_result.model.as_str()),
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
                    .search_documents_by_text(&mut **guard.conn(), tenant_id, &req.content, 3)
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
                            tenant_id,
                            e
                        );
                    }
                }
            }
        }

        // Clear RLS context + return the connection to the pool.
        if let Some(mut guard) = rag_guard {
            guard.release().await;
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
            rls.conn(),
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

    // Clear RLS context before returning the connection to the pool (happy path).
    rls.release().await;

    Ok((StatusCode::CREATED, Json(response_json)))
}

async fn provide_feedback(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(message_id): Path<Uuid>,
    Json(req): Json<ProvideFeedback>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let mut feedback = req;
    feedback.message_id = message_id;

    // SECURITY (issue #766 / #816, owner predicate #2317): the feedback author,
    // the owning tenant, AND the owning user are all derived from the
    // RLS-validated request context, never from the body or the path. The repo
    // only writes when the target message's session belongs to the caller's org
    // AND to the caller themselves — otherwise (a) a caller in org B could
    // attach feedback to (and poison the training signal for) org A's chat
    // messages, and (b) a colleague in the same org could attach/overwrite
    // feedback on another member's private message or use the 201-vs-404
    // response as an existence oracle for org-internal message UUIDs.
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let result = state
        .ai_chat_repo
        .add_feedback_for_org(&mut **rls.conn(), user_id, org_id, feedback)
        .await;
    rls.release().await;
    match result {
        Ok(Some(fb)) => Ok((StatusCode::CREATED, Json(serde_json::json!(fb)))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Message not found")),
        )),
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
    mut rls: RlsConnection,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();

    // SECURITY (#2317): `list_escalated_messages` is a cross-user, org-wide read
    // — it returns the full content of EVERY member's escalated AI-chat messages
    // in the org. Under the per-user privacy model #2279/#2289 pinned for
    // sessions, this must not be an implicit right of every authenticated member
    // (a plain resident could otherwise read colleagues' escalated messages
    // without even a session UUID). Escalation review is a manager/admin
    // function, so gate it on org role here. (We use the org-level `TenantRole`
    // gate — the `admin-core` `RequireCapability` extractor is a
    // platform-principal-only mechanism for `/admin/*` and is the wrong tool for
    // an org-scoped tenant route.)
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "INSUFFICIENT_ROLE",
                "Manager role required to review escalated AI messages",
            )),
        ));
    }

    let result = state
        .ai_chat_repo
        .list_escalated_messages(
            &mut **rls.conn(),
            tenant_id,
            clamp_limit(query.limit, 50),
            query.offset.unwrap_or(0),
        )
        .await;
    rls.release().await;
    match result {
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
    mut rls: RlsConnection,
    Query(query): Query<SentimentTrendQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // RLS context (org GUC) is set on rls.conn(); tenant_id is the authoritative org.
    let tenant_id = rls.tenant_id();

    let result = state
        .sentiment_repo
        .list_trends(&mut **rls.conn(), tenant_id, query)
        .await;
    rls.release().await;

    match result {
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
    mut rls: RlsConnection,
    Query(query): Query<AlertsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();

    let result = state
        .sentiment_repo
        .list_alerts(
            &mut **rls.conn(),
            tenant_id,
            query.acknowledged,
            clamp_limit(query.limit, 50),
            query.offset.unwrap_or(0),
        )
        .await;
    rls.release().await;

    match result {
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
    mut rls: RlsConnection,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();

    let result = state
        .sentiment_repo
        .acknowledge_alert(&mut **rls.conn(), alert_id, org_id, user_id)
        .await;
    rls.release().await;

    match result {
        Ok(alert) => Ok(Json(serde_json::json!(alert))),
        Err(sqlx::Error::RowNotFound) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Alert not found")),
        )),
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
    mut rls: RlsConnection,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();

    let result = state
        .sentiment_repo
        .get_thresholds(rls.conn(), tenant_id)
        .await;
    rls.release().await;

    match result {
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
    mut rls: RlsConnection,
    Json(req): Json<UpdateSentimentThresholds>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();

    let result = state
        .sentiment_repo
        .update_thresholds(&mut **rls.conn(), tenant_id, req)
        .await;
    rls.release().await;

    match result {
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
    mut rls: RlsConnection,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let today = chrono::Utc::now().date_naive();
    let thirty_days_ago = today - chrono::Duration::days(30);

    // Run all three reads on the same RLS-context connection, then release once.
    // Errors are captured (not early-returned) so release() always runs.
    let dashboard = async {
        let org_avg = state
            .sentiment_repo
            .get_org_average_sentiment(&mut **rls.conn(), org_id, thirty_days_ago, today)
            .await?;
        let trends = state
            .sentiment_repo
            .list_trends(&mut **rls.conn(), org_id, SentimentTrendQuery::default())
            .await?;
        let alerts = state
            .sentiment_repo
            .list_alerts(&mut **rls.conn(), org_id, Some(false), 5, 0)
            .await?;
        Ok::<_, sqlx::Error>((org_avg, trends, alerts))
    }
    .await;
    rls.release().await;

    let (org_avg, trends, alerts) = dashboard.map_err(|e| {
        tracing::error!("Failed to get dashboard: {}", e);
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
