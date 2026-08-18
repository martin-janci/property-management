//! LLM Document repository (Epic 64: Advanced AI & LLM Capabilities).
//!
//! # RLS Integration (PAP-108 / PAP-80 / PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on `document_embeddings` (the RAG store this
//! repo reads and writes). Under `FORCE` the api-server's owner connection is
//! no longer exempt, so a query issued on a connection without
//! `app.current_org_id` set collapses to deny-all (own-org reads return empty,
//! writes fail the policy `WITH CHECK`) — which silently broke RAG context
//! retrieval on `dev`.
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
//! pool**, so there is no way to issue a query that bypasses RLS. This mirrors
//! the `work_order.rs` / `vendor.rs` / `sentiment.rs` precedent.
//!
//! The non-RLS tables this repo touches (`llm_generation_requests`,
//! `voice_assistant_devices`, …) keep their application-level `organization_id`
//! / `user_id` guards; callers without a tenant principal (the voice-platform
//! OAuth refresh webhook) may run those methods on a plain pool connection.

use crate::models::llm_document::{
    generation_status, AiEscalationConfig, AiUsageStatistics, DocumentEmbedding,
    GeneratedListingDescription, LlmGenerationRequest, LlmPromptTemplate, PhotoEnhancement,
    ProviderStats, RagStatistics, RequestTypeStats, UpdateEscalationConfig, VoiceAssistantDevice,
    VoiceCommandHistory,
};
use crate::repositories::rag_metrics;
use chrono::{DateTime, Utc};
use sqlx::{Error as SqlxError, Executor, PgConnection, PgPool, Postgres};
use uuid::Uuid;

/// Repository for LLM document generation operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Clone)]
pub struct LlmDocumentRepository;

impl LlmDocumentRepository {
    /// Create a new LlmDocumentRepository.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the caller).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    // =========================================================================
    // LLM Generation Requests
    // =========================================================================

    /// Create a new LLM generation request.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_generation_request<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        user_id: Uuid,
        request_type: &str,
        provider: &str,
        model: &str,
        input_data: serde_json::Value,
        prompt_template_id: Option<Uuid>,
    ) -> Result<LlmGenerationRequest, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LlmGenerationRequest>(
            r#"
            INSERT INTO llm_generation_requests (
                organization_id, user_id, request_type, provider, model,
                input_data, prompt_template_id, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(request_type)
        .bind(provider)
        .bind(model)
        .bind(&input_data)
        .bind(prompt_template_id)
        .bind(generation_status::PENDING)
        .fetch_one(executor)
        .await
    }

    /// Find a generation request by ID — tenant-scoped (issue #766 / #816).
    ///
    /// `org_id` must originate from the verified request principal. Returns
    /// `None` for both "not found" and "belongs to another tenant" so a caller
    /// in org B cannot read org A's generation request (an IDOR information
    /// leak: prompts, results, cost data).
    pub async fn find_generation_request_for_org<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<LlmGenerationRequest>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LlmGenerationRequest>(
            "SELECT * FROM llm_generation_requests WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Verify that `unit_id` belongs to `org_id` (issue #766 / #816).
    ///
    /// Lease generation must not be invoked against another tenant's unit.
    /// Units link to an organization via their building
    /// (`units.building_id -> buildings.organization_id`), so this checks the
    /// join rather than a direct column. Returns `true` only when a unit with
    /// that id exists under the caller's org.
    pub async fn unit_belongs_to_org<'e, E>(
        &self,
        executor: E,
        unit_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let exists: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT TRUE
            FROM units u
            JOIN buildings b ON b.id = u.building_id
            WHERE u.id = $1 AND b.organization_id = $2
            "#,
        )
        .bind(unit_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await?;
        Ok(exists.unwrap_or(false))
    }

    /// Check that a `documents` row exists and is visible to `org_id` (#2201).
    ///
    /// The RAG embedding-write flow keys `document_embeddings` by a
    /// caller-supplied `document_id` with a FK onto `documents(id)`
    /// (migration 00081). Callers use this to pre-validate the reference
    /// *before* generating (paid) embeddings, so a missing / cross-org id is
    /// rejected as a 404 instead of surfacing as a generic 500 FK violation
    /// after the embed cost has already been paid.
    ///
    /// The `organization_id` predicate is defensive-in-depth: on a FORCE-RLS
    /// org-context connection the row is already invisible cross-org, but the
    /// explicit filter mirrors [`Self::unit_belongs_to_org`] and holds even on
    /// a global-read connection.
    pub async fn document_exists_for_org<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let exists: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT TRUE
            FROM documents
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(document_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await?;
        Ok(exists.unwrap_or(false))
    }

    /// Update generation request status.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_generation_status<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        status: &str,
        result: Option<serde_json::Value>,
        error_message: Option<&str>,
        tokens_used: Option<i32>,
        cost_cents: Option<i32>,
        latency_ms: Option<i32>,
    ) -> Result<Option<LlmGenerationRequest>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let completed_at =
            if status == generation_status::COMPLETED || status == generation_status::FAILED {
                Some(Utc::now())
            } else {
                None
            };

        sqlx::query_as::<_, LlmGenerationRequest>(
            r#"
            UPDATE llm_generation_requests SET
                status = $2,
                result = COALESCE($3, result),
                error_message = $4,
                tokens_used = COALESCE($5, tokens_used),
                cost_cents = COALESCE($6, cost_cents),
                latency_ms = COALESCE($7, latency_ms),
                completed_at = COALESCE($8, completed_at)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(&result)
        .bind(error_message)
        .bind(tokens_used)
        .bind(cost_cents)
        .bind(latency_ms)
        .bind(completed_at)
        .fetch_optional(executor)
        .await
    }

    /// List generation requests for an organization.
    pub async fn list_generation_requests<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        request_type: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LlmGenerationRequest>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LlmGenerationRequest>(
            r#"
            SELECT * FROM llm_generation_requests
            WHERE organization_id = $1
              AND ($2::text IS NULL OR request_type = $2)
              AND ($3::text IS NULL OR status = $3)
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(organization_id)
        .bind(request_type)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    // =========================================================================
    // Prompt Templates
    // =========================================================================

    /// Create a prompt template.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_prompt_template<'e, E>(
        &self,
        executor: E,
        organization_id: Option<Uuid>,
        name: &str,
        description: Option<&str>,
        request_type: &str,
        system_prompt: &str,
        user_prompt_template: &str,
        variables: serde_json::Value,
        provider: &str,
        model: &str,
        temperature: Option<f32>,
        max_tokens: Option<i32>,
    ) -> Result<LlmPromptTemplate, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LlmPromptTemplate>(
            r#"
            INSERT INTO llm_prompt_templates (
                organization_id, name, description, request_type,
                system_prompt, user_prompt_template, variables,
                provider, model, temperature, max_tokens, is_system
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(name)
        .bind(description)
        .bind(request_type)
        .bind(system_prompt)
        .bind(user_prompt_template)
        .bind(&variables)
        .bind(provider)
        .bind(model)
        .bind(temperature)
        .bind(max_tokens)
        .bind(organization_id.is_none()) // is_system = true if no org
        .fetch_one(executor)
        .await
    }

    /// Find a prompt template by ID — tenant-scoped (issue #766 / #816).
    ///
    /// `org_id` must originate from the verified request principal. A template
    /// is visible only when it belongs to the caller's org OR it is a system
    /// (`is_system = TRUE`, `organization_id IS NULL`) template shared across
    /// all tenants. Returns `None` for "not found" and for "another tenant's
    /// org-specific template" so org B cannot read org A's prompt templates.
    pub async fn find_prompt_template_for_org<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<LlmPromptTemplate>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LlmPromptTemplate>(
            r#"
            SELECT * FROM llm_prompt_templates
            WHERE id = $1 AND (organization_id = $2 OR is_system = TRUE)
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Find the default template for a request type.
    pub async fn find_default_template<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        request_type: &str,
    ) -> Result<Option<LlmPromptTemplate>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // First try org-specific, then system default
        sqlx::query_as::<_, LlmPromptTemplate>(
            r#"
            SELECT * FROM llm_prompt_templates
            WHERE request_type = $2
              AND is_active = TRUE
              AND (organization_id = $1 OR is_system = TRUE)
            ORDER BY
                CASE WHEN organization_id = $1 THEN 0 ELSE 1 END,
                version DESC
            LIMIT 1
            "#,
        )
        .bind(organization_id)
        .bind(request_type)
        .fetch_optional(executor)
        .await
    }

    /// List prompt templates.
    pub async fn list_prompt_templates<'e, E>(
        &self,
        executor: E,
        organization_id: Option<Uuid>,
        request_type: Option<&str>,
    ) -> Result<Vec<LlmPromptTemplate>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LlmPromptTemplate>(
            r#"
            SELECT * FROM llm_prompt_templates
            WHERE ($1::uuid IS NULL OR organization_id = $1 OR is_system = TRUE)
              AND ($2::text IS NULL OR request_type = $2)
              AND is_active = TRUE
            ORDER BY is_system DESC, name
            "#,
        )
        .bind(organization_id)
        .bind(request_type)
        .fetch_all(executor)
        .await
    }

    /// Update a prompt template.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_prompt_template<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        system_prompt: Option<&str>,
        user_prompt_template: Option<&str>,
        variables: Option<serde_json::Value>,
        provider: Option<&str>,
        model: Option<&str>,
        temperature: Option<f32>,
        max_tokens: Option<i32>,
        is_active: Option<bool>,
    ) -> Result<Option<LlmPromptTemplate>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, LlmPromptTemplate>(
            r#"
            UPDATE llm_prompt_templates SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                system_prompt = COALESCE($4, system_prompt),
                user_prompt_template = COALESCE($5, user_prompt_template),
                variables = COALESCE($6, variables),
                provider = COALESCE($7, provider),
                model = COALESCE($8, model),
                temperature = COALESCE($9, temperature),
                max_tokens = COALESCE($10, max_tokens),
                is_active = COALESCE($11, is_active),
                version = version + 1,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(system_prompt)
        .bind(user_prompt_template)
        .bind(&variables)
        .bind(provider)
        .bind(model)
        .bind(temperature)
        .bind(max_tokens)
        .bind(is_active)
        .fetch_optional(executor)
        .await
    }

    // =========================================================================
    // Generated Listing Descriptions
    // =========================================================================

    /// Create a generated listing description.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_listing_description<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        listing_id: Option<Uuid>,
        user_id: Uuid,
        language: &str,
        original_description: &str,
        property_details: serde_json::Value,
        photo_analysis: Option<serde_json::Value>,
        generation_request_id: Uuid,
    ) -> Result<GeneratedListingDescription, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, GeneratedListingDescription>(
            r#"
            INSERT INTO generated_listing_descriptions (
                organization_id, listing_id, user_id, language,
                original_description, property_details, photo_analysis,
                generation_request_id, generated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(listing_id)
        .bind(user_id)
        .bind(language)
        .bind(original_description)
        .bind(&property_details)
        .bind(&photo_analysis)
        .bind(generation_request_id)
        .fetch_one(executor)
        .await
    }

    /// List descriptions for a listing — tenant-scoped (issue #766 / #816).
    ///
    /// `org_id` must originate from the verified request principal. The
    /// `organization_id = $2` guard ensures a caller in org B cannot read the
    /// generated listing descriptions owned by org A by enumerating a
    /// `listing_id`. Returns an empty vec for both "no descriptions" and
    /// "listing belongs to another tenant", so existence is never leaked.
    pub async fn list_listing_descriptions_for_org<'e, E>(
        &self,
        executor: E,
        listing_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<GeneratedListingDescription>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, GeneratedListingDescription>(
            r#"
            SELECT * FROM generated_listing_descriptions
            WHERE listing_id = $1 AND organization_id = $2
            ORDER BY generated_at DESC
            "#,
        )
        .bind(listing_id)
        .bind(org_id)
        .fetch_all(executor)
        .await
    }

    /// Update edited description.
    pub async fn update_edited_description<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        edited_description: &str,
        edited_by: Uuid,
    ) -> Result<Option<GeneratedListingDescription>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, GeneratedListingDescription>(
            r#"
            UPDATE generated_listing_descriptions SET
                edited_description = $2,
                edited_at = NOW(),
                edited_by = $3
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(edited_description)
        .bind(edited_by)
        .fetch_optional(executor)
        .await
    }

    /// Mark description as published — tenant-scoped (issue #766 / #816).
    ///
    /// `org_id` must originate from the verified request principal. The
    /// `organization_id = $2` guard ensures a caller in org B cannot publish
    /// (mutate) a generated listing description owned by org A. Returns `None`
    /// for both "not found" and "belongs to another tenant".
    pub async fn publish_description_for_org<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<GeneratedListingDescription>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, GeneratedListingDescription>(
            r#"
            UPDATE generated_listing_descriptions
            SET is_published = TRUE
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    // =========================================================================
    // Document Embeddings (RAG)
    // =========================================================================

    /// Create a document embedding.
    ///
    /// `document_embeddings` is FORCE-RLS (migration 00179): the executor's
    /// connection MUST carry the org GUC or the INSERT fails the policy
    /// `WITH CHECK`.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_embedding<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        document_id: Uuid,
        chunk_index: i32,
        chunk_text: &str,
        embedding: Option<Vec<f32>>,
        metadata: serde_json::Value,
    ) -> Result<DocumentEmbedding, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, DocumentEmbedding>(
            r#"
            INSERT INTO document_embeddings (
                organization_id, document_id, chunk_index, chunk_text, embedding, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(document_id)
        .bind(chunk_index)
        .bind(chunk_text)
        // Story 103.5: For production RAG systems, use pgvector extension.
        // The embedding is stored as JSONB for compatibility, but vector operations
        // should use the embedding_vector column when pgvector is enabled.
        // See: migration 00079_create_pgvector.sql
        .bind(
            embedding
                .as_ref()
                .and_then(|e| serde_json::to_value(e).ok()),
        )
        .bind(&metadata)
        .fetch_one(executor)
        .await
    }

    /// Find embeddings for a document.
    pub async fn find_document_embeddings<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
    ) -> Result<Vec<DocumentEmbedding>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, DocumentEmbedding>(
            "SELECT * FROM document_embeddings WHERE document_id = $1 ORDER BY chunk_index",
        )
        .bind(document_id)
        .fetch_all(executor)
        .await
    }

    /// Delete embeddings for a document.
    pub async fn delete_document_embeddings<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
    ) -> Result<u64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM document_embeddings WHERE document_id = $1")
            .bind(document_id)
            .execute(executor)
            .await?;
        Ok(result.rows_affected())
    }

    /// Search documents by text (simple text search, not semantic/vector similarity).
    ///
    /// This is a fallback method when pgvector is not available.
    /// For production RAG capability, use `search_documents_by_embedding` instead.
    pub async fn search_documents_by_text<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        search_text: &str,
        limit: i32,
    ) -> Result<Vec<DocumentEmbedding>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, DocumentEmbedding>(
            r#"
            SELECT * FROM document_embeddings
            WHERE organization_id = $1
              AND chunk_text ILIKE '%' || $2 || '%'
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(organization_id)
        .bind(search_text)
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    // =========================================================================
    // Story 97.2: RAG Implementation - Semantic Similarity Search
    // =========================================================================

    /// Update embedding vector for an existing document chunk.
    /// Used after generating embeddings via LLM client.
    pub async fn update_embedding_vector<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        embedding: Vec<f32>,
    ) -> Result<Option<DocumentEmbedding>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Store embedding as JSONB for now (works without pgvector extension)
        // In production with pgvector enabled, this would use vector type directly
        let embedding_json = serde_json::to_value(&embedding).unwrap_or_default();

        sqlx::query_as::<_, DocumentEmbedding>(
            r#"
            UPDATE document_embeddings SET
                embedding = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&embedding_json)
        .fetch_optional(executor)
        .await
    }

    /// Search documents by embedding vector using cosine similarity.
    ///
    /// Story 97.2: Implements semantic similarity search for RAG.
    /// When pgvector extension is enabled, this uses native vector operations.
    /// Otherwise, falls back to application-level cosine similarity calculation.
    ///
    /// Returns document chunks ordered by relevance (highest similarity first).
    ///
    /// Multi-statement: takes `&mut PgConnection` so every query runs on the
    /// same RLS-context connection.
    pub async fn search_documents_by_embedding(
        &self,
        conn: &mut PgConnection,
        organization_id: Uuid,
        query_embedding: &[f32],
        limit: i32,
        min_similarity: Option<f64>,
    ) -> Result<Vec<(DocumentEmbedding, f64)>, SqlxError> {
        let min_sim = min_similarity.unwrap_or(0.5);

        // First, try to use pgvector if available (native cosine similarity)
        // Check if pgvector extension exists
        let pgvector_available: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname = 'vector')",
        )
        .fetch_optional(&mut *conn)
        .await?;

        if pgvector_available == Some(true) {
            // Use pgvector's native cosine similarity (<=> operator)
            // Note: Requires embedding column to be of type vector(1536)
            let results: Vec<DocumentEmbedding> = sqlx::query_as::<_, DocumentEmbedding>(
                r#"
                SELECT de.*
                FROM document_embeddings de
                WHERE de.organization_id = $1
                  AND de.embedding IS NOT NULL
                ORDER BY de.embedding::vector <=> $2::vector
                LIMIT $3
                "#,
            )
            .bind(organization_id)
            .bind(serde_json::to_value(query_embedding).unwrap_or_default())
            .bind(limit)
            .fetch_all(&mut *conn)
            .await?;

            // Calculate similarity scores for the results
            let results_with_scores: Vec<(DocumentEmbedding, f64)> = results
                .into_iter()
                .filter_map(|doc| {
                    let doc_embedding: Option<Vec<f32>> = doc.embedding.as_ref().and_then(|e| {
                        serde_json::from_value(serde_json::Value::Array(
                            e.iter().map(|v| serde_json::Value::from(*v)).collect(),
                        ))
                        .ok()
                    });

                    if let Some(ref emb) = doc_embedding {
                        let similarity = cosine_similarity(query_embedding, emb);
                        if similarity >= min_sim {
                            return Some((doc, similarity));
                        }
                    }
                    None
                })
                .collect();

            return Ok(results_with_scores);
        }

        // Fallback: Load all embeddings and compute similarity in application
        // This is less efficient but works without pgvector
        let all_docs = sqlx::query_as::<_, DocumentEmbedding>(
            r#"
            SELECT * FROM document_embeddings
            WHERE organization_id = $1
              AND embedding IS NOT NULL
            "#,
        )
        .bind(organization_id)
        .fetch_all(&mut *conn)
        .await?;

        // Calculate cosine similarity for each document
        let mut scored_docs: Vec<(DocumentEmbedding, f64)> = all_docs
            .into_iter()
            .filter_map(|doc| {
                let doc_embedding: Option<Vec<f32>> = doc.embedding.as_ref().and_then(|e| {
                    serde_json::from_value(serde_json::Value::Array(
                        e.iter().map(|v| serde_json::Value::from(*v)).collect(),
                    ))
                    .ok()
                });

                if let Some(ref emb) = doc_embedding {
                    let similarity = cosine_similarity(query_embedding, emb);
                    if similarity >= min_sim {
                        return Some((doc, similarity));
                    }
                }
                None
            })
            .collect();

        // Sort by similarity (highest first)
        scored_docs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top N results
        Ok(scored_docs.into_iter().take(limit as usize).collect())
    }

    /// Bulk create embeddings for a document (for chunked text).
    /// Story 97.2: Efficient batch embedding creation for RAG indexing.
    ///
    /// Multi-statement: takes `&mut PgConnection` so every chunk INSERT runs
    /// on the same RLS-context connection.
    pub async fn create_embeddings_batch(
        &self,
        conn: &mut PgConnection,
        organization_id: Uuid,
        document_id: Uuid,
        chunks: Vec<(String, serde_json::Value)>, // (chunk_text, metadata)
    ) -> Result<Vec<DocumentEmbedding>, SqlxError> {
        let mut results = Vec::new();

        for (index, (chunk_text, metadata)) in chunks.into_iter().enumerate() {
            let embedding = self
                .create_embedding(
                    &mut *conn,
                    organization_id,
                    document_id,
                    index as i32,
                    &chunk_text,
                    None, // Embedding will be generated later
                    metadata,
                )
                .await?;
            results.push(embedding);
        }

        Ok(results)
    }

    /// Get documents that need embedding generation (embedding is null).
    /// Story 97.2: Used by background job to process pending embeddings.
    pub async fn get_pending_embeddings<'e, E>(
        &self,
        executor: E,
        organization_id: Option<Uuid>,
        limit: i32,
    ) -> Result<Vec<DocumentEmbedding>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, DocumentEmbedding>(
            r#"
            SELECT * FROM document_embeddings
            WHERE embedding IS NULL
              AND ($1::uuid IS NULL OR organization_id = $1)
            ORDER BY created_at ASC
            LIMIT $2
            "#,
        )
        .bind(organization_id)
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    /// Count documents with and without embeddings for an organization.
    /// Story 97.2: Used to track RAG indexing progress.
    /// Story 103.5: Now also tracks pgvector migration status.
    pub async fn count_embedding_status<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<(i64, i64), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let counts: (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE embedding IS NOT NULL OR embedding_vector IS NOT NULL) as with_embedding,
                COUNT(*) FILTER (WHERE embedding IS NULL AND embedding_vector IS NULL) as without_embedding
            FROM document_embeddings
            WHERE organization_id = $1
            "#,
        )
        .bind(organization_id)
        .fetch_one(executor)
        .await?;

        Ok(counts)
    }

    // =========================================================================
    // Story 103.5: pgvector RAG Operations
    // =========================================================================

    /// Search documents using pgvector native similarity search (Story 103.5).
    ///
    /// This method uses the SQL function created in migration 00079_create_pgvector.sql.
    /// Falls back to application-level search if pgvector is not available.
    ///
    /// Multi-statement: takes `&mut PgConnection` so every query runs on the
    /// same RLS-context connection.
    ///
    /// `model_filter` (#2201): when `Some(model)`, rows whose stored
    /// `metadata.embedding_model` is *known to differ* from `model` are dropped
    /// before returning — cosine similarity is only meaningful within a single
    /// embedding space, and stub vectors are 1536-dim exactly like OpenAI's, so
    /// mixing them silently returns garbage. Legacy rows that predate provenance
    /// tagging (no `embedding_model` key) are kept so this stays backward
    /// compatible. When a filter is set we over-fetch and truncate back to
    /// `limit` so the filtered result still fills the top-k where possible.
    pub async fn search_documents_pgvector(
        &self,
        conn: &mut PgConnection,
        organization_id: Uuid,
        query_embedding: &[f32],
        limit: i32,
        min_similarity: Option<f64>,
        model_filter: Option<&str>,
    ) -> Result<Vec<(DocumentEmbedding, f64)>, SqlxError> {
        // Story 84.5: observe retrieval quality (latency, top-k relevance,
        // empty-result rate). Timed from here so the histogram covers the whole
        // retrieval — including the pgvector-availability probe and the
        // provenance post-filter — not just the SQL round-trip.
        let started = std::time::Instant::now();
        let min_sim = min_similarity.unwrap_or(0.5);
        // Over-fetch when a provenance filter is active so post-filtering still
        // returns up to `limit` compatible rows (capped to avoid unbounded scans).
        let fetch_limit = if model_filter.is_some() {
            over_fetch_limit(limit, 4, 200)
        } else {
            limit
        };

        // Check if pgvector extension and function exist
        let pgvector_available: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_proc WHERE proname = 'search_similar_documents')",
        )
        .fetch_optional(&mut *conn)
        .await?;

        if pgvector_available == Some(true) {
            // Use the pgvector SQL function for efficient search
            let embedding_str = format!(
                "[{}]",
                query_embedding
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );

            let results: Vec<(Uuid, Uuid, i32, String, serde_json::Value, f64)> = sqlx::query_as(
                r#"
                SELECT id, document_id, chunk_index, chunk_text, metadata, similarity
                FROM search_similar_documents($1, $2::vector, $3, $4)
                "#,
            )
            .bind(organization_id)
            .bind(&embedding_str)
            .bind(fetch_limit)
            .bind(min_sim)
            .fetch_all(&mut *conn)
            .await?;

            // Convert to DocumentEmbedding format
            let mut embeddings = Vec::new();
            for (id, document_id, chunk_index, chunk_text, metadata, similarity) in results {
                let emb = DocumentEmbedding {
                    id,
                    organization_id,
                    document_id,
                    chunk_index,
                    chunk_text,
                    embedding: None,
                    metadata,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                embeddings.push((emb, similarity));
            }

            let out = Self::filter_by_embedding_model(embeddings, model_filter, limit);
            Self::observe_retrieval(rag_metrics::BACKEND_PGVECTOR, started.elapsed(), &out);
            return Ok(out);
        }

        // Fallback to the existing application-level search
        let fallback = self
            .search_documents_by_embedding(
                conn,
                organization_id,
                query_embedding,
                fetch_limit,
                min_similarity,
            )
            .await?;
        let out = Self::filter_by_embedding_model(fallback, model_filter, limit);
        Self::observe_retrieval(rag_metrics::BACKEND_FALLBACK, started.elapsed(), &out);
        Ok(out)
    }

    /// Emit RAG retrieval-quality metrics for one search (Story 84.5).
    fn observe_retrieval(
        backend: &'static str,
        elapsed: std::time::Duration,
        results: &[(DocumentEmbedding, f64)],
    ) {
        let scores: Vec<f64> = results.iter().map(|(_, score)| *score).collect();
        rag_metrics::record_retrieval(backend, elapsed, &scores);
    }

    /// Drop rows whose stored `metadata.embedding_model` is known to differ from
    /// `model_filter`, then truncate to `limit` (#2201). Rows with no
    /// `embedding_model` provenance are retained for backward compatibility.
    fn filter_by_embedding_model(
        mut rows: Vec<(DocumentEmbedding, f64)>,
        model_filter: Option<&str>,
        limit: i32,
    ) -> Vec<(DocumentEmbedding, f64)> {
        if let Some(model) = model_filter {
            rows.retain(|(emb, _)| {
                match emb.metadata.get("embedding_model").and_then(|v| v.as_str()) {
                    Some(stored) => stored == model,
                    None => true,
                }
            });
        }
        if limit >= 0 {
            rows.truncate(limit as usize);
        }
        rows
    }

    /// Get RAG statistics for an organization using the v_rag_statistics view (Story 103.5).
    ///
    /// Multi-statement: takes `&mut PgConnection` so every query runs on the
    /// same RLS-context connection.
    #[allow(clippy::type_complexity)]
    pub async fn get_rag_statistics(
        &self,
        conn: &mut PgConnection,
        organization_id: Uuid,
    ) -> Result<RagStatistics, SqlxError> {
        // Check if the view exists
        let view_exists: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM information_schema.views WHERE table_name = 'v_rag_statistics')",
        )
        .fetch_optional(&mut *conn)
        .await?;

        if view_exists == Some(true) {
            // Query the v_rag_statistics view
            let row: Option<(i64, i64, i64, i64, Option<i32>, Option<DateTime<Utc>>)> =
                sqlx::query_as(
                    r#"
                SELECT indexed_documents, total_chunks, chunks_with_vector,
                       chunks_pending_migration, avg_chunk_length, last_updated
                FROM v_rag_statistics
                WHERE organization_id = $1
                "#,
                )
                .bind(organization_id)
                .fetch_optional(&mut *conn)
                .await?;

            if let Some((
                indexed_documents,
                total_chunks,
                chunks_with_vector,
                chunks_pending_migration,
                avg_chunk_length,
                last_updated,
            )) = row
            {
                return Ok(RagStatistics {
                    indexed_documents,
                    total_chunks,
                    chunks_with_vector,
                    chunks_pending_migration,
                    avg_chunk_length: avg_chunk_length.unwrap_or(0),
                    last_updated,
                });
            }
        }

        // Return empty statistics if view doesn't exist or no data
        Ok(RagStatistics {
            indexed_documents: 0,
            total_chunks: 0,
            chunks_with_vector: 0,
            chunks_pending_migration: 0,
            avg_chunk_length: 0,
            last_updated: None,
        })
    }

    /// Upsert a document-embedding chunk, org-scoped (Story 84.5).
    ///
    /// Fast path: calls the `upsert_document_embedding` SQL function shipped by
    /// migration 00081 (present only when the pgvector extension is installed),
    /// which stores the vector in the native `embedding_vector` column.
    ///
    /// Fallback (stock PostgreSQL, e.g. CI): mirrors the SQL function's
    /// select-then-update/insert semantics against the JSONB `embedding`
    /// column, additionally scoped to `organization_id` so a chunk can never
    /// be rewritten across org boundaries even on a privileged connection.
    ///
    /// `document_embeddings` is FORCE-RLS: the connection must carry the org
    /// GUC (or global-read context) or both paths return zero rows / fail the
    /// policy `WITH CHECK`.
    ///
    /// Returns the id of the inserted or updated row.
    ///
    /// Multi-statement: takes `&mut PgConnection` so every query runs on the
    /// same RLS-context connection.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_embedding(
        &self,
        conn: &mut PgConnection,
        organization_id: Uuid,
        document_id: Uuid,
        chunk_index: i32,
        chunk_text: &str,
        embedding: &[f32],
        model: &str,
        metadata: serde_json::Value,
    ) -> Result<Uuid, SqlxError> {
        // Provenance (#2201): fold the embedding provider/model into metadata so
        // retrieval can avoid comparing incompatible vector spaces of the same
        // dimension (stub vs OpenAI are both 1536-dim). Stored under
        // `embedding_model`; a caller-supplied value is overwritten so the row
        // always reflects the model that actually produced the vector.
        let metadata = {
            let mut metadata = metadata;
            if !metadata.is_object() {
                metadata = serde_json::json!({});
            }
            if let Some(obj) = metadata.as_object_mut() {
                obj.insert(
                    "embedding_model".to_string(),
                    serde_json::Value::String(model.to_string()),
                );
            }
            metadata
        };

        // Check whether migration 00081's pgvector upsert function exists.
        let function_exists: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_proc WHERE proname = 'upsert_document_embedding')",
        )
        .fetch_optional(&mut *conn)
        .await?;

        if function_exists == Some(true) {
            let embedding_str = format!(
                "[{}]",
                embedding
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            );

            let id: Uuid = sqlx::query_scalar(
                "SELECT upsert_document_embedding($1, $2, $3, $4, $5::vector, $6)",
            )
            .bind(organization_id)
            .bind(document_id)
            .bind(chunk_index)
            .bind(chunk_text)
            .bind(&embedding_str)
            .bind(&metadata)
            .fetch_one(&mut *conn)
            .await?;

            return Ok(id);
        }

        // Fallback: JSONB upsert with the same (document_id, chunk_index)
        // identity, org-scoped.
        let embedding_json = serde_json::to_value(embedding).unwrap_or_default();

        let existing: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id FROM document_embeddings
            WHERE organization_id = $1 AND document_id = $2 AND chunk_index = $3
            "#,
        )
        .bind(organization_id)
        .bind(document_id)
        .bind(chunk_index)
        .fetch_optional(&mut *conn)
        .await?;

        if let Some(id) = existing {
            sqlx::query(
                r#"
                UPDATE document_embeddings SET
                    chunk_text = $2,
                    embedding = $3,
                    metadata = $4,
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(id)
            .bind(chunk_text)
            .bind(&embedding_json)
            .bind(&metadata)
            .execute(&mut *conn)
            .await?;
            return Ok(id);
        }

        let id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO document_embeddings (
                organization_id, document_id, chunk_index, chunk_text, embedding, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
        )
        .bind(organization_id)
        .bind(document_id)
        .bind(chunk_index)
        .bind(chunk_text)
        .bind(&embedding_json)
        .bind(&metadata)
        .fetch_one(&mut *conn)
        .await?;

        Ok(id)
    }

    /// Migrate pending JSONB embeddings to pgvector format (Story 103.5).
    ///
    /// Returns the number of embeddings migrated.
    ///
    /// Multi-statement: takes `&mut PgConnection` so every query runs on the
    /// same RLS-context connection.
    pub async fn migrate_embeddings_to_pgvector(
        &self,
        conn: &mut PgConnection,
    ) -> Result<i64, SqlxError> {
        // Check if the migration function exists
        let function_exists: Option<bool> = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_proc WHERE proname = 'migrate_jsonb_to_vector')",
        )
        .fetch_optional(&mut *conn)
        .await?;

        if function_exists == Some(true) {
            // Get count before migration
            let before_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM document_embeddings WHERE embedding_vector IS NOT NULL",
            )
            .fetch_one(&mut *conn)
            .await?;

            // Run migration function
            sqlx::query("SELECT migrate_jsonb_to_vector()")
                .execute(&mut *conn)
                .await?;

            // Get count after migration
            let after_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM document_embeddings WHERE embedding_vector IS NOT NULL",
            )
            .fetch_one(&mut *conn)
            .await?;

            return Ok(after_count - before_count);
        }

        Ok(0)
    }

    // =========================================================================
    // Escalation Configuration
    // =========================================================================

    /// Get or create escalation config for an organization.
    ///
    /// Multi-statement (SELECT then conditional INSERT): takes
    /// `&mut PgConnection` so both run on the same RLS-context connection.
    pub async fn get_escalation_config(
        &self,
        conn: &mut PgConnection,
        organization_id: Uuid,
    ) -> Result<AiEscalationConfig, SqlxError> {
        // Try to find existing config
        let existing = sqlx::query_as::<_, AiEscalationConfig>(
            "SELECT * FROM ai_escalation_configs WHERE organization_id = $1",
        )
        .bind(organization_id)
        .fetch_optional(&mut *conn)
        .await?;

        if let Some(config) = existing {
            return Ok(config);
        }

        // Create default config with 80% threshold
        sqlx::query_as::<_, AiEscalationConfig>(
            r#"
            INSERT INTO ai_escalation_configs (
                organization_id, confidence_threshold, auto_escalate_topics
            )
            VALUES ($1, 0.80, '[]')
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .fetch_one(&mut *conn)
        .await
    }

    /// Update escalation config.
    pub async fn update_escalation_config<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        update: UpdateEscalationConfig,
    ) -> Result<AiEscalationConfig, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let topics_json = update
            .auto_escalate_topics
            .map(|t| serde_json::to_value(&t).unwrap_or_default());

        sqlx::query_as::<_, AiEscalationConfig>(
            r#"
            UPDATE ai_escalation_configs SET
                confidence_threshold = COALESCE($2, confidence_threshold),
                escalation_email = COALESCE($3, escalation_email),
                escalation_webhook_url = COALESCE($4, escalation_webhook_url),
                auto_escalate_topics = COALESCE($5, auto_escalate_topics),
                updated_at = NOW()
            WHERE organization_id = $1
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(update.confidence_threshold)
        .bind(&update.escalation_email)
        .bind(&update.escalation_webhook_url)
        .bind(&topics_json)
        .fetch_one(executor)
        .await
    }

    // =========================================================================
    // Photo Enhancement
    // =========================================================================

    /// Create a photo enhancement record.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_photo_enhancement<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        listing_id: Option<Uuid>,
        user_id: Uuid,
        original_photo_url: &str,
        enhancement_type: &str,
        metadata: serde_json::Value,
    ) -> Result<PhotoEnhancement, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, PhotoEnhancement>(
            r#"
            INSERT INTO photo_enhancements (
                organization_id, listing_id, user_id, original_photo_url,
                enhancement_type, status, metadata
            )
            VALUES ($1, $2, $3, $4, $5, 'pending', $6)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(listing_id)
        .bind(user_id)
        .bind(original_photo_url)
        .bind(enhancement_type)
        .bind(&metadata)
        .fetch_one(executor)
        .await
    }

    /// Find photo enhancement by ID — tenant-scoped (issue #766 / #816).
    ///
    /// `org_id` must originate from the verified request principal. Returns
    /// `None` for both "not found" and "belongs to another tenant" so a caller
    /// in org B cannot read org A's photo enhancement record.
    pub async fn find_photo_enhancement_for_org<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<PhotoEnhancement>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, PhotoEnhancement>(
            "SELECT * FROM photo_enhancements WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// Update photo enhancement status and result.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_photo_enhancement<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        status: &str,
        enhanced_photo_url: Option<&str>,
        thumbnail_url: Option<&str>,
        error_message: Option<&str>,
        processing_time_ms: Option<i32>,
        cost_cents: Option<i32>,
    ) -> Result<Option<PhotoEnhancement>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let completed_at = if status == "completed" || status == "failed" {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query_as::<_, PhotoEnhancement>(
            r#"
            UPDATE photo_enhancements SET
                status = $2,
                enhanced_photo_url = COALESCE($3, enhanced_photo_url),
                thumbnail_url = COALESCE($4, thumbnail_url),
                error_message = $5,
                processing_time_ms = COALESCE($6, processing_time_ms),
                cost_cents = COALESCE($7, cost_cents),
                completed_at = COALESCE($8, completed_at)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(enhanced_photo_url)
        .bind(thumbnail_url)
        .bind(error_message)
        .bind(processing_time_ms)
        .bind(cost_cents)
        .bind(completed_at)
        .fetch_optional(executor)
        .await
    }

    /// List photo enhancements for a listing.
    pub async fn list_photo_enhancements<'e, E>(
        &self,
        executor: E,
        listing_id: Uuid,
    ) -> Result<Vec<PhotoEnhancement>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, PhotoEnhancement>(
            "SELECT * FROM photo_enhancements WHERE listing_id = $1 ORDER BY created_at DESC",
        )
        .bind(listing_id)
        .fetch_all(executor)
        .await
    }

    // =========================================================================
    // Voice Assistant Devices
    // =========================================================================

    /// Create a voice assistant device link.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_voice_device<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        user_id: Uuid,
        unit_id: Option<Uuid>,
        platform: &str,
        device_id: &str,
        device_name: Option<&str>,
        access_token_encrypted: Option<&str>,
        refresh_token_encrypted: Option<&str>,
        token_expires_at: Option<DateTime<Utc>>,
        capabilities: serde_json::Value,
        // Keyed HMAC-SHA256 of the access token (#2662): indexed lookup key so
        // voice-webhook auth avoids the O(N) decrypt-and-scan. NULL is accepted
        // and simply defers the device to the linear-scan fallback.
        access_token_hash: Option<&[u8]>,
    ) -> Result<VoiceAssistantDevice, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, VoiceAssistantDevice>(
            r#"
            INSERT INTO voice_assistant_devices (
                organization_id, user_id, unit_id, platform, device_id,
                device_name, access_token_encrypted, refresh_token_encrypted,
                token_expires_at, capabilities, access_token_hash, linked_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(unit_id)
        .bind(platform)
        .bind(device_id)
        .bind(device_name)
        .bind(access_token_encrypted)
        .bind(refresh_token_encrypted)
        .bind(token_expires_at)
        .bind(&capabilities)
        .bind(access_token_hash)
        .fetch_one(executor)
        .await
    }

    /// Find voice device by ID.
    pub async fn find_voice_device<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<VoiceAssistantDevice>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, VoiceAssistantDevice>(
            "SELECT * FROM voice_assistant_devices WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(executor)
        .await
    }

    /// Find voice device by external device ID.
    pub async fn find_voice_device_by_device_id<'e, E>(
        &self,
        executor: E,
        platform: &str,
        device_id: &str,
    ) -> Result<Option<VoiceAssistantDevice>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, VoiceAssistantDevice>(
            "SELECT * FROM voice_assistant_devices WHERE platform = $1 AND device_id = $2 AND is_active = TRUE",
        )
        .bind(platform)
        .bind(device_id)
        .fetch_optional(executor)
        .await
    }

    /// List voice devices for a user.
    pub async fn list_user_voice_devices<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
    ) -> Result<Vec<VoiceAssistantDevice>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, VoiceAssistantDevice>(
            "SELECT * FROM voice_assistant_devices WHERE user_id = $1 AND is_active = TRUE ORDER BY linked_at DESC",
        )
        .bind(user_id)
        .fetch_all(executor)
        .await
    }

    /// Update voice device last used timestamp.
    pub async fn update_voice_device_last_used<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<(), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query("UPDATE voice_assistant_devices SET last_used_at = NOW(), updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(executor)
            .await?;
        Ok(())
    }

    /// Deactivate a voice device.
    ///
    /// The `user_id` parameter scopes the update to devices owned by the
    /// caller, preventing cross-tenant IDOR writes.  Returns `Ok(false)` when
    /// no row is updated — either the `id` does not exist or the device is not
    /// owned by `user_id`.  The handler maps both cases to `404 Not Found`,
    /// giving an attacker no information about whether the target id exists.
    pub async fn deactivate_voice_device<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            "UPDATE voice_assistant_devices SET is_active = FALSE, updated_at = NOW() WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    // =========================================================================
    // Voice Command History
    // =========================================================================

    /// Create a voice command history entry.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_voice_command<'e, E>(
        &self,
        executor: E,
        device_id: Uuid,
        user_id: Uuid,
        command_text: &str,
        intent_detected: Option<&str>,
        response_text: &str,
        action_taken: Option<&str>,
        success: bool,
        error_message: Option<&str>,
        processing_time_ms: i32,
    ) -> Result<VoiceCommandHistory, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, VoiceCommandHistory>(
            r#"
            INSERT INTO voice_command_history (
                device_id, user_id, command_text, intent_detected,
                response_text, action_taken, success, error_message, processing_time_ms
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(device_id)
        .bind(user_id)
        .bind(command_text)
        .bind(intent_detected)
        .bind(response_text)
        .bind(action_taken)
        .bind(success)
        .bind(error_message)
        .bind(processing_time_ms)
        .fetch_one(executor)
        .await
    }

    /// Return true iff the given device belongs to the given user.
    ///
    /// Issue #483: used by `list_voice_commands` to return 404 (not 200
    /// with empty list) on a cross-user probe — matches the disclosure
    /// posture of the unlink handler.
    pub async fn user_owns_voice_device<'e, E>(
        &self,
        executor: E,
        device_id: Uuid,
        user_id: Uuid,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM voice_assistant_devices WHERE id = $1 AND user_id = $2")
                .bind(device_id)
                .bind(user_id)
                .fetch_optional(executor)
                .await?;
        Ok(row.is_some())
    }

    /// List voice command history for a device.
    ///
    /// `user_id` scopes the query to commands for devices owned by the caller,
    /// preventing cross-tenant IDOR reads.  Returns an empty list (not 404) when
    /// the device does not exist or is not owned by `user_id`, matching the same
    /// conservative disclosure posture used by the deactivate path.
    pub async fn list_voice_commands<'e, E>(
        &self,
        executor: E,
        device_id: Uuid,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<VoiceCommandHistory>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, VoiceCommandHistory>(
            r#"
            SELECT vch.* FROM voice_command_history vch
            JOIN voice_assistant_devices vad ON vad.id = vch.device_id
            WHERE vch.device_id = $1
              AND vad.user_id = $2
            ORDER BY vch.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(device_id)
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    // =========================================================================
    // Story 93.1: Voice Assistant OAuth Token Management
    // =========================================================================

    /// Update OAuth tokens for a voice device.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_voice_device_tokens<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        access_token_encrypted: &str,
        refresh_token_encrypted: Option<&str>,
        token_expires_at: Option<DateTime<Utc>>,
        // Keyed HMAC-SHA256 of the new access token (#2662): kept in lock-step
        // with `access_token_encrypted` so the indexed lookup stays correct
        // after a token refresh.
        access_token_hash: Option<&[u8]>,
    ) -> Result<Option<VoiceAssistantDevice>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, VoiceAssistantDevice>(
            r#"
            UPDATE voice_assistant_devices SET
                access_token_encrypted = $2,
                refresh_token_encrypted = COALESCE($3, refresh_token_encrypted),
                token_expires_at = $4,
                access_token_hash = $5,
                updated_at = NOW()
            WHERE id = $1 AND is_active = TRUE
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(access_token_encrypted)
        .bind(refresh_token_encrypted)
        .bind(token_expires_at)
        .bind(access_token_hash)
        .fetch_optional(executor)
        .await
    }

    /// Find voice devices with expiring tokens that need refresh.
    pub async fn find_devices_needing_token_refresh<'e, E>(
        &self,
        executor: E,
        expiry_threshold: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<VoiceAssistantDevice>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, VoiceAssistantDevice>(
            r#"
            SELECT * FROM voice_assistant_devices
            WHERE is_active = TRUE
              AND access_token_encrypted IS NOT NULL
              AND refresh_token_encrypted IS NOT NULL
              AND token_expires_at IS NOT NULL
              AND token_expires_at <= $1
            ORDER BY token_expires_at ASC
            LIMIT $2
            "#,
        )
        .bind(expiry_threshold)
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    /// Clear tokens for a voice device (on revocation or error).
    pub async fn clear_voice_device_tokens<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            UPDATE voice_assistant_devices SET
                access_token_encrypted = NULL,
                refresh_token_encrypted = NULL,
                token_expires_at = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Find voice device by user ID and platform.
    pub async fn find_voice_device_by_user_and_platform<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
        platform: &str,
    ) -> Result<Option<VoiceAssistantDevice>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, VoiceAssistantDevice>(
            r#"
            SELECT * FROM voice_assistant_devices
            WHERE user_id = $1
              AND platform = $2
              AND is_active = TRUE
            ORDER BY linked_at DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .bind(platform)
        .fetch_optional(executor)
        .await
    }

    /// Find the active voice device for an `(organization, user, platform)`
    /// tuple.
    ///
    /// This is the lookup key the OAuth-exchange (account-linking) path upserts
    /// on: a re-link must rotate the tokens on the *existing* device row for the
    /// tuple rather than insert an independent row. Without it, each re-link
    /// minted a fresh `device_id` and a new row, leaving the previous row active
    /// with its own still-usable stored token (stale-token accumulation).
    /// Scoping to `organization_id` as well as `user_id` keeps devices a user
    /// linked under different tenants distinct.
    pub async fn find_active_voice_device_by_org_user_and_platform<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        user_id: Uuid,
        platform: &str,
    ) -> Result<Option<VoiceAssistantDevice>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, VoiceAssistantDevice>(
            r#"
            SELECT * FROM voice_assistant_devices
            WHERE organization_id = $1
              AND user_id = $2
              AND platform = $3
              AND is_active = TRUE
            ORDER BY linked_at DESC
            LIMIT 1
            "#,
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(platform)
        .fetch_optional(executor)
        .await
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    /// Get AI usage statistics for an organization.
    ///
    /// Multi-statement (totals + by-type + by-provider): takes
    /// `&mut PgConnection` so every query runs on the same RLS-context
    /// connection.
    pub async fn get_usage_statistics(
        &self,
        conn: &mut PgConnection,
        organization_id: Uuid,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<AiUsageStatistics, SqlxError> {
        let start = start_date.unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(Utc::now);

        // Get totals
        let totals: (i64, i64, i64, i64, i64, f64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*),
                COUNT(*) FILTER (WHERE status = 'completed'),
                COUNT(*) FILTER (WHERE status = 'failed'),
                COALESCE(SUM(tokens_used), 0),
                COALESCE(SUM(cost_cents), 0),
                COALESCE(AVG(latency_ms), 0)::float8
            FROM llm_generation_requests
            WHERE organization_id = $1
              AND created_at >= $2
              AND created_at <= $3
            "#,
        )
        .bind(organization_id)
        .bind(start)
        .bind(end)
        .fetch_one(&mut *conn)
        .await?;

        // Get by request type
        let by_type: Vec<(String, i64, i64, i64)> = sqlx::query_as(
            r#"
            SELECT
                request_type,
                COUNT(*),
                COALESCE(SUM(tokens_used), 0),
                COALESCE(SUM(cost_cents), 0)
            FROM llm_generation_requests
            WHERE organization_id = $1
              AND created_at >= $2
              AND created_at <= $3
            GROUP BY request_type
            "#,
        )
        .bind(organization_id)
        .bind(start)
        .bind(end)
        .fetch_all(&mut *conn)
        .await?;

        // Get by provider
        let by_provider: Vec<(String, i64, i64, i64, f64)> = sqlx::query_as(
            r#"
            SELECT
                provider,
                COUNT(*),
                COALESCE(SUM(tokens_used), 0),
                COALESCE(SUM(cost_cents), 0),
                COALESCE(AVG(latency_ms), 0)::float8
            FROM llm_generation_requests
            WHERE organization_id = $1
              AND created_at >= $2
              AND created_at <= $3
            GROUP BY provider
            "#,
        )
        .bind(organization_id)
        .bind(start)
        .bind(end)
        .fetch_all(&mut *conn)
        .await?;

        Ok(AiUsageStatistics {
            total_generations: totals.0,
            successful_generations: totals.1,
            failed_generations: totals.2,
            total_tokens_used: totals.3,
            total_cost_cents: totals.4,
            average_latency_ms: totals.5,
            by_request_type: by_type
                .into_iter()
                .map(|(request_type, count, tokens, cost)| RequestTypeStats {
                    request_type,
                    count,
                    tokens_used: tokens,
                    cost_cents: cost,
                })
                .collect(),
            by_provider: by_provider
                .into_iter()
                .map(|(provider, count, tokens, cost, latency)| ProviderStats {
                    provider,
                    count,
                    tokens_used: tokens,
                    cost_cents: cost,
                    average_latency_ms: latency,
                })
                .collect(),
        })
    }
}

// =============================================================================
// Story 97.2: RAG Helper Functions
// =============================================================================

/// Widen a top-k `limit` into an over-fetch window for provenance-filtered
/// pgvector search, then cap it to bound the scan.
///
/// `search_documents_pgvector` over-fetches when a `model_filter` is active so
/// that dropping provenance-mismatched rows still leaves up to `limit`
/// compatible results. The window is `limit * multiplier`, capped at `cap`,
/// but never smaller than `limit` itself.
///
/// Written as `.min(cap).max(limit)` on purpose — NOT `clamp(limit, cap)`:
/// `i32::clamp(min, max)` panics when `min > max`, which happens whenever
/// `limit > cap`. This ordering is panic-free for every `i32` input, and
/// `saturating_mul` guards the multiply against overflow. See #2237.
fn over_fetch_limit(limit: i32, multiplier: i32, cap: i32) -> i32 {
    limit.saturating_mul(multiplier).min(cap).max(limit)
}

/// Calculate cosine similarity between two vectors.
/// Returns a value between -1.0 (opposite) and 1.0 (identical).
/// Used for semantic similarity search in RAG.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum();
    let magnitude_a: f64 = a
        .iter()
        .map(|x| (*x as f64) * (*x as f64))
        .sum::<f64>()
        .sqrt();
    let magnitude_b: f64 = b
        .iter()
        .map(|x| (*x as f64) * (*x as f64))
        .sum::<f64>()
        .sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let similarity = cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_opposite_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let similarity = cosine_similarity(&a, &b);
        assert!((similarity + 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let similarity = cosine_similarity(&a, &b);
        assert!(similarity.abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_empty_vectors() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_cosine_similarity_similar_vectors() {
        // Two similar but not identical vectors
        let a = vec![0.1, 0.2, 0.3, 0.4];
        let b = vec![0.15, 0.25, 0.35, 0.45];
        let similarity = cosine_similarity(&a, &b);
        // Should be very similar (close to 1.0)
        assert!(similarity > 0.99);
    }

    // -------------------------------------------------------------------------
    // #2237: regression guard for the pgvector over-fetch clamp-panic.
    //
    // The provenance-filter over-fetch used to be `clamp(limit, 200)`, which
    // panics (`min > max`) whenever `limit > 200`. `search_documents_pgvector`
    // is reached from RAG retrieval paths that don't pre-clamp `limit`, so a
    // caller-supplied `limit > 200` would crash the request. These pin the
    // boundary that used to panic and stop a refactor reintroducing `clamp`.
    // -------------------------------------------------------------------------

    #[test]
    fn over_fetch_limit_widens_below_cap() {
        // Headroom below the cap: limit * multiplier, untouched by min/max.
        assert_eq!(over_fetch_limit(10, 4, 200), 40);
    }

    #[test]
    fn over_fetch_limit_caps_at_ceiling() {
        // limit * multiplier exceeds the cap → capped, still >= limit.
        assert_eq!(over_fetch_limit(60, 4, 200), 200);
    }

    #[test]
    fn over_fetch_limit_never_panics_when_limit_exceeds_cap() {
        // The historical panic: limit > cap. `clamp(limit, cap)` would panic
        // here (min=limit > max=cap); the `.min().max()` ordering returns
        // `limit` unharmed.
        assert_eq!(over_fetch_limit(250, 4, 200), 250);
    }

    #[test]
    fn over_fetch_limit_handles_multiply_overflow() {
        // saturating_mul must not panic/wrap on a huge limit; result is still
        // clamped up to at least `limit`.
        assert_eq!(over_fetch_limit(i32::MAX, 4, 200), i32::MAX);
    }

    #[test]
    fn over_fetch_limit_matches_the_call_site_arguments() {
        // Guards the exact (multiplier=4, cap=200) contract used by
        // search_documents_pgvector for a typical top-k of 10.
        assert_eq!(over_fetch_limit(10, 4, 200), 40);
    }
}
