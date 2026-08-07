//! Epic 64: Advanced AI & LLM Capabilities.
//!
//! - Story 64.1: LLM-Powered Lease Agreement Generation
//! - Story 64.2: AI Property Listing Description Generator
//! - Story 64.3: Conversational AI Tenant Support (Enhanced RAG)
//! - Story 64.4: AI Photo Enhancement for Listings
//! - Story 64.5: Voice Assistant Integration (handlers in [`super::voice`])

use crate::routes::ai::voice::{
    link_voice_device, list_voice_commands, list_voice_devices, unlink_voice_device,
};
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    EnhancePhotoRequest, EnhancedChatRequest, GenerateLeaseRequest,
    GenerateListingDescriptionRequest, UpdateEscalationConfig,
};
use db::RlsPool;
use integrations::{EmbeddingProvider, StubEmbeddingProvider};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use uuid::Uuid;

/// Client-facing message returned when lease generation fails. The underlying
/// LLM-provider error is logged server-side in [`generate_lease`] but is never
/// forwarded to the client, to avoid leaking upstream provider detail.
const LEASE_GENERATION_FAILED_MESSAGE: &str = "Lease generation failed";

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
        // RAG indexing / embedding-write flow (Story 84.5 / 103.5)
        .route("/rag/index", post(index_document))
        // RAG legacy JSONB → pgvector back-fill (Story 84.5 / 103.5)
        .route("/rag/migrate", post(migrate_embeddings))
        // Statistics
        .route("/statistics", get(get_ai_statistics))
        .route("/requests", get(list_generation_requests))
        .route("/requests/{id}", get(get_generation_request))
}

// ============================================================================
// Story 84.5 / 103.5: pgvector RAG embedding-write flow
// ============================================================================

/// Request to index a document's text chunks into the RAG store.
#[derive(Debug, Deserialize)]
struct IndexDocumentRequest {
    /// Logical document identifier the chunks belong to. Embeddings are stored
    /// org-scoped (RLS) and keyed by `(document_id, chunk_index)`.
    document_id: Uuid,
    /// Pre-split text chunks to embed and store. Empty / whitespace-only
    /// entries are dropped.
    chunks: Vec<String>,
    /// Optional human-readable title, folded into each chunk's metadata as
    /// `title` (used by the retrieval path for `source_title`).
    #[serde(default)]
    title: Option<String>,
    /// Optional extra metadata merged into every chunk's metadata object.
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

/// Result of an indexing run.
#[derive(Debug, Serialize)]
struct IndexDocumentResponse {
    document_id: Uuid,
    /// Number of chunks embedded and upserted.
    chunks_indexed: usize,
    /// IDs of the upserted `document_embeddings` rows, in chunk order.
    embedding_ids: Vec<Uuid>,
    /// Embedding provider used: `"openai"` when a key is configured, otherwise
    /// the deterministic offline stub.
    provider: String,
}

/// Upper bound on chunks per request — guards against unbounded embedding cost
/// and connection hold time.
const MAX_INDEX_CHUNKS: usize = 512;

/// Whether an embedding batch is well-formed for indexing: the provider must
/// return exactly one vector per submitted chunk.
///
/// The [`EmbeddingProvider::embed_batch`] contract is one vector per input, but
/// the production OpenAI backend fulfils it via a remote batch call whose `data`
/// array length is not guaranteed by the type system. If the provider returns
/// *fewer* vectors than chunks (a truncated / partial batch response), the write
/// loop's `chunks.iter().zip(embeddings.iter())` silently drops the surplus
/// chunks — only a prefix of the document is indexed while the caller still gets
/// a 201 success, corrupting later similarity retrieval. Verify the counts match
/// and fail closed instead.
fn embedding_batch_is_complete(chunk_count: usize, embedding_count: usize) -> bool {
    chunk_count == embedding_count
}

/// Index a document into the pgvector RAG store (Story 84.5 / 103.5).
///
/// This is the missing application-side embedding-write flow: it generates an
/// embedding per chunk via the [`EmbeddingProvider`] abstraction and upserts it
/// through `LlmDocumentRepository::upsert_embedding` (pgvector
/// `upsert_document_embedding` when the extension is present, JSONB fallback
/// otherwise). Retrieval (`/chat/enhanced`, `ai_chat_router`) then reads these
/// rows via the pgvector `search_similar_documents` path.
///
/// Provider selection: the live OpenAI embedding backend when an API key is
/// configured; otherwise a deterministic [`StubEmbeddingProvider`] so indexing
/// still succeeds in CI / air-gapped deployments instead of hard-failing.
#[utoipa::path(
    post,
    path = "/api/v1/ai/llm/rag/index",
    request_body = serde_json::Value,
    responses(
        (status = 201, description = "Document chunks embedded and stored"),
        (status = 400, description = "No usable chunks / too many chunks", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Referenced document does not exist / not org-visible", body = ErrorResponse),
        (status = 502, description = "Embedding provider failed", body = ErrorResponse),
        (status = 503, description = "Stub embeddings refused in production", body = ErrorResponse),
    ),
    tag = "AI LLM"
)]
async fn index_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<IndexDocumentRequest>,
) -> Result<(StatusCode, Json<IndexDocumentResponse>), (StatusCode, Json<ErrorResponse>)> {
    // RLS context (org GUC) is set on rls.conn(); tenant_id/user_id are the
    // authoritative principal (copied out so we can release the pooled
    // connection before the slow embed phase — see below).
    let tenant_id = rls.tenant_id();
    let user_id = rls.user_id();

    // Normalise chunks: trim + drop empties.
    let chunks: Vec<String> = req
        .chunks
        .iter()
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect();

    if chunks.is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_INPUT",
                "chunks must contain at least one non-empty entry",
            )),
        ));
    }
    if chunks.len() > MAX_INDEX_CHUNKS {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_INPUT",
                "too many chunks in a single request",
            )),
        ));
    }

    // fk-error-shape (#2201): validate the referenced document exists and is
    // org-visible BEFORE embedding. Embeddings are FK-bound to `documents`
    // (migration 00081); pre-checking here turns a missing / cross-org id into
    // a 404 instead of a generic 500 FK violation surfaced *after* the (paid)
    // embedding round-trip. Runs on the request-scoped RLS connection we
    // already hold.
    let doc_exists = state
        .llm_document_repo
        .document_exists_for_org(&mut **rls.conn(), req.document_id, tenant_id)
        .await;

    // connection-holding (#2201): release the pooled RLS connection BEFORE the
    // (slow, network-bound) embed phase so a single indexing request can't pin a
    // pool connection across many OpenAI round-trips and starve concurrent RLS
    // work (head-of-line blocking). A short-lived org-context connection is
    // re-acquired only for the write loop, mirroring `generate_lease`.
    rls.release().await;

    match doc_exists {
        Ok(true) => {}
        Ok(false) => {
            // 404 (not 403) mirrors the rest of this module: do not confirm the
            // existence of another org's document to a caller who cannot see it.
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
            ));
        }
        Err(e) => {
            tracing::error!("RAG document existence check failed: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to validate document",
                )),
            ));
        }
    }

    // Provider selection (Story 84.5): live OpenAI when configured, else the
    // deterministic offline stub so indexing works without a key.
    let (provider, provider_name): (Box<dyn EmbeddingProvider>, &str) =
        if state.llm_client.has_openai_key() {
            (Box::new(state.llm_client.clone()), "openai")
        } else {
            (Box::new(StubEmbeddingProvider::new()), "stub-deterministic")
        };

    // provider-provenance (#2201): the stub produces 1536-dim vectors that are
    // dimension-identical to OpenAI's but semantically meaningless. Warn loudly
    // whenever we index with it, and refuse outright in production unless
    // explicitly opted in — otherwise a prod deployment silently writes noise
    // vectors that later corrupt cosine retrieval.
    if provider_name != "openai" {
        let is_production = std::env::var("RUST_ENV").as_deref() == Ok("production");
        let allow_stub = std::env::var("ALLOW_STUB_EMBEDDINGS")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        if is_production && !allow_stub {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse::new(
                    "EMBEDDING_UNAVAILABLE",
                    "Embedding provider not configured; refusing to index stub vectors in production",
                )),
            ));
        }
        tracing::warn!(
            organization_id = %tenant_id,
            document_id = %req.document_id,
            "RAG indexing via deterministic stub provider — vectors are NOT semantically \
             meaningful and must not be mixed with a real embedding space at retrieval time"
        );
    }

    // Generate embeddings with NO DB connection held (see connection-holding
    // note above).
    let embeddings = match provider.embed_batch(&chunks).await {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("RAG embedding generation failed: {}", e);
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::new(
                    "EMBEDDING_FAILED",
                    "Failed to generate embeddings",
                )),
            ));
        }
    };

    // partial-batch guard: the write loop below zips chunks with embeddings, so a
    // provider that returns fewer vectors than chunks would silently index only a
    // prefix of the document and still report success. Fail closed on any
    // count mismatch rather than persisting a partially-embedded document.
    if !embedding_batch_is_complete(chunks.len(), embeddings.len()) {
        tracing::error!(
            organization_id = %tenant_id,
            document_id = %req.document_id,
            chunks = chunks.len(),
            embeddings = embeddings.len(),
            "RAG embedding provider returned a mismatched vector count — refusing partial index"
        );
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse::new(
                "EMBEDDING_FAILED",
                "Embedding provider returned an incomplete result",
            )),
        ));
    }

    // Base metadata: caller-provided object (if any) plus an optional title.
    // Per-vector provenance (`embedding_model`) is folded in by the repository's
    // `upsert_embedding`.
    let mut base_meta = req
        .metadata
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}));
    if let (Some(title), Some(obj)) = (req.title.as_ref(), base_meta.as_object_mut()) {
        obj.entry("title")
            .or_insert_with(|| serde_json::Value::String(title.clone()));
    }

    // Re-acquire a short-lived org-context connection for the write loop only
    // (PAP-150: no handler-side raw `state.db`).
    let mut guard = match RlsPool::new(state.db.clone())
        .acquire_with_rls(tenant_id, user_id, false)
        .await
    {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("Failed to acquire db connection for RAG write: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to store embeddings",
                )),
            ));
        }
    };

    // Upsert each chunk (pgvector-aware, JSONB fallback) on the write connection.
    let mut embedding_ids = Vec::with_capacity(chunks.len());
    for (idx, (chunk, emb)) in chunks.iter().zip(embeddings.iter()).enumerate() {
        match state
            .llm_document_repo
            .upsert_embedding(
                guard.conn(),
                tenant_id,
                req.document_id,
                idx as i32,
                chunk,
                &emb.embedding,
                &emb.model,
                base_meta.clone(),
            )
            .await
        {
            Ok(id) => embedding_ids.push(id),
            Err(e) => {
                guard.release().await;
                // fk-error-shape (#2201): a 23503 FK violation here means the
                // document vanished between the pre-check and the write (race)
                // — report it as 404, not a generic 500.
                if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23503") {
                    tracing::warn!("RAG embedding upsert hit FK violation: {}", e);
                    return Err((
                        StatusCode::NOT_FOUND,
                        Json(ErrorResponse::new("NOT_FOUND", "Document not found")),
                    ));
                }
                tracing::error!("RAG embedding upsert failed: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "INTERNAL_ERROR",
                        "Failed to store embeddings",
                    )),
                ));
            }
        }
    }
    guard.release().await;

    tracing::info!(
        "RAG indexed {} chunk(s) for document {} (org {}) via {}",
        embedding_ids.len(),
        req.document_id,
        tenant_id,
        provider_name
    );

    Ok((
        StatusCode::CREATED,
        Json(IndexDocumentResponse {
            document_id: req.document_id,
            chunks_indexed: embedding_ids.len(),
            embedding_ids,
            provider: provider_name.to_string(),
        }),
    ))
}

/// Result of a pgvector back-fill migration run.
#[derive(Debug, Serialize)]
struct MigrateEmbeddingsResponse {
    /// Number of legacy JSONB embeddings converted into the pgvector
    /// `embedding_vector` column by this run.
    migrated: i64,
}

/// Back-fill legacy JSONB embeddings into the pgvector column (Story 84.5 / 103.5).
///
/// Wires `LlmDocumentRepository::migrate_embeddings_to_pgvector` — previously
/// reachable only from tests — to an operator-triggerable HTTP route. Legacy
/// `document_embeddings` rows written before the pgvector column existed store
/// their vector as JSONB and carry no `embedding_model` provenance. This
/// endpoint runs the idempotent `migrate_jsonb_to_vector()` back-fill — only
/// rows with `embedding_vector IS NULL` and a 1536-dim JSONB array are
/// converted — and reports how many rows moved.
///
/// As of migration 00215 (#2300) the back-fill also stamps assumed
/// `metadata.embedding_model` provenance (`text-embedding-3-small`) on any
/// converted row that lacks it, so the retrieval filter
/// (`filter_by_embedding_model`) can isolate the row's embedding space instead
/// of silently mixing it into a provenance-filtered similarity search.
///
/// Platform-admin only: the run needs the super-admin RLS bypass so it can
/// convert legacy rows across every organization in one pass rather than only
/// the caller's tenant. Non-admins get 403; when pgvector is not installed the
/// migration function is absent and the call is a no-op returning 0.
#[utoipa::path(
    post,
    path = "/api/v1/ai/llm/rag/migrate",
    responses(
        (status = 200, description = "Migration run complete; body reports rows migrated"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Caller is not a platform administrator", body = ErrorResponse),
        (status = 500, description = "Migration failed", body = ErrorResponse),
    ),
    tag = "AI LLM"
)]
async fn migrate_embeddings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<(StatusCode, Json<MigrateEmbeddingsResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Global maintenance op: converting legacy rows across ALL organizations
    // requires the super-admin RLS bypass, so gate strictly to platform/super
    // admins. A normal org_admin's RLS-scoped connection would only see (and
    // migrate) its own tenant's rows — and must not be allowed to trigger a
    // cross-tenant back-fill anyway.
    if !rls.is_super_admin() {
        let tenant_id = rls.tenant_id();
        let user_id = rls.user_id();
        rls.release().await;
        tracing::warn!(
            organization_id = %tenant_id,
            user_id = %user_id,
            "Rejected non-admin attempt to trigger pgvector embedding migration"
        );
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only platform administrators can migrate embeddings",
            )),
        ));
    }

    let result = state
        .llm_document_repo
        .migrate_embeddings_to_pgvector(rls.conn())
        .await;
    rls.release().await;

    match result {
        Ok(migrated) => {
            tracing::info!(migrated, "pgvector embedding migration complete");
            Ok((StatusCode::OK, Json(MigrateEmbeddingsResponse { migrated })))
        }
        Err(e) => {
            tracing::error!("pgvector embedding migration failed: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to migrate embeddings",
                )),
            ))
        }
    }
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
    mut rls: RlsConnection,
    Json(req): Json<GenerateLeaseRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    // RLS context (org GUC) is set on rls.conn(); tenant_id is the authoritative org.
    let tenant_id = rls.tenant_id();
    let user_id = rls.user_id();
    let start_time = Instant::now();

    // SECURITY (issue #766 / #816): the lease is generated for a specific
    // `unit_id`. Validate that the unit belongs to the caller's tenant BEFORE
    // creating the generation request or burning any LLM tokens — otherwise a
    // caller could generate a lease (and have it cost-attributed to their org)
    // against another tenant's unit, leaking the unit's identity/context.
    //
    // Run the pre-LLM DB work on the RLS connection, then release it BEFORE
    // the (slow) LLM call so we don't pin a pool connection for seconds.
    let pre_llm = async {
        let unit_ok = state
            .llm_document_repo
            .unit_belongs_to_org(&mut **rls.conn(), req.unit_id, tenant_id)
            .await?;
        Ok::<_, sqlx::Error>(unit_ok)
    }
    .await;

    let unit_ok = match pre_llm {
        Ok(ok) => ok,
        Err(e) => {
            rls.release().await;
            tracing::error!("Failed to validate unit ownership: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to validate unit",
                )),
            ));
        }
    };
    if !unit_ok {
        rls.release().await;
        // 404 (not 403) mirrors the rest of this module: do not confirm the
        // existence of another tenant's unit to a caller who cannot see it.
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Unit not found")),
        ));
    }

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
            &mut **rls.conn(),
            tenant_id,
            user_id,
            "lease_generation",
            &provider,
            &model,
            input_data.clone(),
            req.template_id,
        )
        .await;
    // Release the request-scoped RLS connection before the (slow) LLM
    // round-trip so we don't pin a pooled connection across it. The post-LLM
    // status update re-acquires a short-lived org-context connection via
    // RlsPool::acquire_with_rls (PAP-150: no handler-side raw `state.db`).
    rls.release().await;
    let request = request.map_err(|e| {
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
            // Update the generation request with the result on a short-lived
            // org-context connection (best-effort; status row is non-critical).
            let result_json = serde_json::to_value(&lease_result).unwrap_or_default();
            if let Ok(mut guard) = RlsPool::new(state.db.clone())
                .acquire_with_rls(tenant_id, user_id, false)
                .await
            {
                let _ = state
                    .llm_document_repo
                    .update_generation_status(
                        &mut **guard.conn(),
                        request.id,
                        "completed",
                        Some(result_json.clone()),
                        None,
                        Some(lease_result.tokens_used),
                        None,
                        Some(latency_ms),
                    )
                    .await;
                guard.release().await;
            }

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

            // Update the generation request with the error on a short-lived
            // org-context connection (best-effort).
            if let Ok(mut guard) = RlsPool::new(state.db.clone())
                .acquire_with_rls(tenant_id, user_id, false)
                .await
            {
                let _ = state
                    .llm_document_repo
                    .update_generation_status(
                        &mut **guard.conn(),
                        request.id,
                        "failed",
                        None,
                        Some(&error_msg),
                        None,
                        None,
                        Some(latency_ms),
                    )
                    .await;
                guard.release().await;
            }

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "GENERATION_FAILED",
                    // Do not leak the upstream LLM-provider error to the client;
                    // the detail is already logged server-side above.
                    LEASE_GENERATION_FAILED_MESSAGE,
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
    mut rls: RlsConnection,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();

    let result = state
        .llm_document_repo
        .list_prompt_templates(&mut **rls.conn(), Some(tenant_id), Some("lease_generation"))
        .await;
    rls.release().await;

    match result {
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
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY (issue #766 / #816): scope the read to the caller's org so a
    // tenant cannot read another org's prompt template by guessing its id.
    // System templates (org_id NULL) remain readable by everyone.
    let org_id = rls.tenant_id();
    let result = state
        .llm_document_repo
        .find_prompt_template_for_org(&mut **rls.conn(), id, org_id)
        .await;
    rls.release().await;

    match result {
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
    mut rls: RlsConnection,
    Json(req): Json<GenerateListingDescriptionRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let user_id = rls.user_id();
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
            &mut **rls.conn(),
            tenant_id,
            user_id,
            "listing_description",
            &provider,
            &model,
            input_data.clone(),
            None,
        )
        .await;
    // Release before the LLM round-trip; the post-LLM writes run on the pool
    // (generated_listing_descriptions / llm_generation_requests are not
    // RLS-bound) with the org id taken from the verified principal above.
    rls.release().await;
    let request = request.map_err(|e| {
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

    // Persist on a short-lived org-context connection (the request-scoped RLS
    // conn was released before the LLM round-trip above). PAP-150: no
    // handler-side raw `state.db`.
    let mut gen_guard = RlsPool::new(state.db.clone())
        .acquire_with_rls(tenant_id, user_id, false)
        .await
        .map_err(|e| {
            tracing::error!("Failed to acquire db connection: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to store description",
                )),
            )
        })?;

    // Store the generated description
    let description = match state
        .llm_document_repo
        .create_listing_description(
            &mut **gen_guard.conn(),
            tenant_id,
            req.listing_id,
            user_id,
            &req.language,
            &description_text,
            input_data,
            Some(tokens_used.into()),
            request.id,
        )
        .await
    {
        Ok(d) => d,
        Err(e) => {
            gen_guard.release().await;
            tracing::error!("Failed to store description: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to store description",
                )),
            ));
        }
    };

    // Update the generation request status (best-effort).
    let _ = state
        .llm_document_repo
        .update_generation_status(
            &mut **gen_guard.conn(),
            request.id,
            "completed",
            Some(serde_json::json!({ "description": description_text })),
            None,
            Some(tokens_used),
            None,
            Some(latency_ms),
        )
        .await;
    gen_guard.release().await;

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
    mut rls: RlsConnection,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY (issue #766 / #816): scope the read to the caller's org so a
    // tenant cannot read another org's generated listing descriptions by
    // enumerating a listing id.
    let org_id = rls.tenant_id();
    let result = state
        .llm_document_repo
        .list_listing_descriptions_for_org(&mut **rls.conn(), listing_id, org_id)
        .await;
    rls.release().await;

    match result {
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
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY (issue #766 / #816): scope the mutate to the caller's org so a
    // tenant cannot publish another org's generated listing description.
    let org_id = rls.tenant_id();
    let result = state
        .llm_document_repo
        .publish_description_for_org(&mut **rls.conn(), id, org_id)
        .await;
    rls.release().await;

    match result {
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
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 501, description = "Enhanced (RAG) chat not implemented", body = ErrorResponse),
    ),
    tag = "AI LLM"
)]
async fn enhanced_chat(
    mut rls: RlsConnection,
    Json(_req): Json<EnhancedChatRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Fail closed (code review — code-review-api-handlers-enhanced-chat-stub).
    //
    // The RAG pipeline this endpoint promises — (1) search document embeddings
    // for relevant context, (2) call the LLM with that context, (3) score the
    // response confidence against the escalation threshold — is NOT implemented.
    //
    // The previous placeholder returned a 200 success payload built entirely
    // from fabricated data: a canned echo `response`, a hardcoded
    // `confidence: 0.85`, `tokens_used: 150`, an empty `sources: []`, and an
    // `escalated` flag derived from that fake confidence. Clients — and any
    // escalation automation reading those metrics — could act on invented data.
    // Until the real embedding + LLM pipeline lands, return 501 so no caller
    // mistakes a stub for a real answer.
    //
    // `RlsConnection` is still extracted so unauthenticated requests are
    // rejected with 401 before reaching here; release it immediately as we do
    // no database work.
    rls.release().await;

    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse::new(
            "ENHANCED_CHAT_NOT_IMPLEMENTED",
            "Enhanced (RAG) chat is not yet implemented: the document-embedding \
             search and LLM pipeline are not wired up, so no chat response, \
             confidence score, or escalation decision can be produced.",
        )),
    ))
}

async fn get_escalation_config(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();

    let result = state
        .llm_document_repo
        .get_escalation_config(rls.conn(), tenant_id)
        .await;
    rls.release().await;

    match result {
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
    mut rls: RlsConnection,
    Json(req): Json<UpdateEscalationConfig>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();

    let result = state
        .llm_document_repo
        .update_escalation_config(&mut **rls.conn(), tenant_id, req)
        .await;
    rls.release().await;

    match result {
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
    mut rls: RlsConnection,
    Json(req): Json<EnhancePhotoRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let user_id = rls.user_id();

    let metadata = serde_json::to_value(&req.options).unwrap_or_default();

    let enhancement = state
        .llm_document_repo
        .create_photo_enhancement(
            &mut **rls.conn(),
            tenant_id,
            req.listing_id,
            user_id,
            &req.photo_url,
            &req.enhancement_type,
            metadata,
        )
        .await;
    rls.release().await;
    let enhancement = enhancement.map_err(|e| {
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
    mut rls: RlsConnection,
    Json(req): Json<db::models::BatchEnhancePhotosRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();
    let user_id = rls.user_id();

    // Run every per-photo INSERT on the same RLS-context connection, then
    // release once. Errors are captured (not early-returned) so release()
    // always runs.
    let result = async {
        let mut enhancements = Vec::new();
        for photo_url in &req.photo_urls {
            let enhancement = state
                .llm_document_repo
                .create_photo_enhancement(
                    &mut **rls.conn(),
                    tenant_id,
                    req.listing_id,
                    user_id,
                    photo_url,
                    &req.enhancement_type,
                    serde_json::json!({}),
                )
                .await?;
            enhancements.push(serde_json::json!({
                "id": enhancement.id,
                "status": enhancement.status,
                "original_url": photo_url
            }));
        }
        Ok::<_, sqlx::Error>(enhancements)
    }
    .await;
    rls.release().await;

    let enhancements = result.map_err(|e| {
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
            "batch_id": Uuid::new_v4(),
            "total_photos": req.photo_urls.len(),
            "enhancements": enhancements
        })),
    ))
}

async fn get_photo_enhancement(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY (issue #766 / #816): scope the read to the caller's org so a
    // tenant cannot read another org's photo enhancement by guessing its id.
    let org_id = rls.tenant_id();
    let result = state
        .llm_document_repo
        .find_photo_enhancement_for_org(&mut **rls.conn(), id, org_id)
        .await;
    rls.release().await;

    match result {
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
// Statistics and Requests Endpoints
// ============================================================================

async fn get_ai_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();

    let result = state
        .llm_document_repo
        .get_usage_statistics(rls.conn(), tenant_id, None, None)
        .await;
    rls.release().await;

    match result {
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
    mut rls: RlsConnection,
    axum::extract::Query(query): axum::extract::Query<GenerationRequestsQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = rls.tenant_id();

    let result = state
        .llm_document_repo
        .list_generation_requests(
            &mut **rls.conn(),
            tenant_id,
            query.request_type.as_deref(),
            query.status.as_deref(),
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await;
    rls.release().await;

    match result {
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
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY (issue #766 / #816): scope the read to the caller's org so a
    // tenant cannot read another org's generation request (prompts, results,
    // cost data) by guessing its id.
    let org_id = rls.tenant_id();
    let result = state
        .llm_document_repo
        .find_generation_request_for_org(&mut **rls.conn(), id, org_id)
        .await;
    rls.release().await;

    match result {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for the AI upstream-error leak finding: the client-facing
    /// body for a failed lease generation must carry only the fixed generic
    /// message and must never contain the raw upstream LLM-provider error
    /// (which is logged server-side in `generate_lease`).
    #[test]
    fn lease_generation_error_does_not_leak_upstream_detail() {
        // A representative raw provider error that must not reach the client.
        let upstream_detail = "provider 503: model overloaded (upstream request id abc123)";

        // Mirror the client body built by `generate_lease`'s error branch.
        let response = ErrorResponse::new("GENERATION_FAILED", LEASE_GENERATION_FAILED_MESSAGE);
        let body = serde_json::to_string(&response).expect("serialize ErrorResponse");

        assert!(
            !body.contains(upstream_detail),
            "client error body leaked the upstream provider detail: {body}"
        );
        assert!(
            !body.contains("Failed to generate lease:"),
            "client error body still interpolates the upstream error: {body}"
        );
        assert_eq!(response.message, LEASE_GENERATION_FAILED_MESSAGE);
    }

    /// Regression guard for the RAG partial-index gap: a provider that returns
    /// fewer embedding vectors than submitted chunks must be rejected, because
    /// the `index_document` write loop zips chunks with embeddings and would
    /// otherwise silently persist only a prefix of the document.
    #[test]
    fn embedding_batch_mismatch_is_rejected_before_write() {
        let chunks = ["a", "b", "c"];
        // Provider dropped one vector (truncated / partial batch response).
        let embeddings = [vec![0.0f32], vec![0.0f32]];

        // Demonstrate the silent-truncation hazard the guard exists to prevent:
        // zip stops at the shorter side, so only 2 of the 3 chunks would be
        // written while the caller still gets a success response.
        let written = chunks.iter().zip(embeddings.iter()).count();
        assert_eq!(
            written, 2,
            "zip truncates to the shorter side — 1 chunk would be silently dropped"
        );

        // The guard catches the mismatch...
        assert!(
            !embedding_batch_is_complete(chunks.len(), embeddings.len()),
            "mismatched vector count must be treated as incomplete"
        );
        // ...and accepts the well-formed 1-vector-per-chunk case.
        assert!(
            embedding_batch_is_complete(chunks.len(), chunks.len()),
            "one vector per chunk is a complete batch"
        );
        assert!(
            embedding_batch_is_complete(0, 0),
            "an empty batch is trivially complete"
        );
    }
}
