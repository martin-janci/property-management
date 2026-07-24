//! Document Intelligence (Epic 28): OCR, full-text search, classification,
//! summarization and intelligence statistics.

use super::DocumentRepository;
use crate::models::{
    ClassificationFeedback, DocumentClassificationHistory, DocumentIntelligenceStats,
    DocumentOcrQueue, DocumentSearchRequest, DocumentSearchResponse, DocumentSearchResult,
    DocumentSummarizationQueue, DocumentSummary, GenerateSummaryRequest,
};
use chrono::{NaiveDate, Utc};
use sqlx::{Error as SqlxError, Executor, Postgres, Row};
use uuid::Uuid;

impl DocumentRepository {
    // ========================================================================
    // Document Intelligence (Epic 28)
    // Note: These methods involve complex operations with queues and stats
    // that often require multiple queries. They are kept as legacy methods
    // for now, but can be migrated to RLS versions as needed.
    // ========================================================================

    // ------------------------------------------------------------------------
    // Story 28.1: OCR Text Extraction
    // ------------------------------------------------------------------------

    /// Queue a document for OCR processing.
    pub async fn queue_for_ocr(
        &self,
        document_id: Uuid,
        priority: Option<i32>,
    ) -> Result<Option<Uuid>, SqlxError> {
        let row = sqlx::query(r#"SELECT queue_document_for_ocr($1, $2) as queue_id"#)
            .bind(document_id)
            .bind(priority.unwrap_or(5))
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("queue_id"))
    }

    /// Queue a document for OCR processing on an RLS-enabled connection.
    ///
    /// RLS-aware counterpart of [`Self::queue_for_ocr`]. The underlying
    /// `queue_document_for_ocr` function reads `documents`, which is under
    /// FORCE ROW LEVEL SECURITY (#754), so it must run on the caller's
    /// org-scoped connection.
    pub async fn queue_for_ocr_rls<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
        priority: Option<i32>,
    ) -> Result<Option<Uuid>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query(r#"SELECT queue_document_for_ocr($1, $2) as queue_id"#)
            .bind(document_id)
            .bind(priority.unwrap_or(5))
            .fetch_one(executor)
            .await?;

        Ok(row.get("queue_id"))
    }

    /// Get pending OCR queue items for processing.
    pub async fn get_pending_ocr_items(
        &self,
        limit: i64,
    ) -> Result<Vec<DocumentOcrQueue>, SqlxError> {
        sqlx::query_as::<_, DocumentOcrQueue>(
            r#"
            SELECT * FROM document_ocr_queue
            WHERE attempts < max_attempts
              AND next_attempt_at <= NOW()
            ORDER BY priority, next_attempt_at
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Update OCR status for a document.
    pub async fn update_ocr_status(
        &self,
        document_id: Uuid,
        status: &str,
        extracted_text: Option<&str>,
        page_count: Option<i32>,
        confidence: Option<f64>,
        error: Option<&str>,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE documents SET
                ocr_status = $2::ocr_status,
                extracted_text = COALESCE($3, extracted_text),
                ocr_page_count = COALESCE($4, ocr_page_count),
                ocr_confidence = COALESCE($5, ocr_confidence),
                ocr_error = $6,
                ocr_processed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(document_id)
        .bind(status)
        .bind(extracted_text)
        .bind(page_count)
        .bind(confidence)
        .bind(error)
        .execute(&self.pool)
        .await?;

        // Remove from queue on success or final failure
        if status == "completed" || status == "failed" || status == "not_applicable" {
            sqlx::query("DELETE FROM document_ocr_queue WHERE document_id = $1")
                .bind(document_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Record OCR attempt (for retry logic).
    pub async fn record_ocr_attempt(
        &self,
        document_id: Uuid,
        error: Option<&str>,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE document_ocr_queue SET
                attempts = attempts + 1,
                last_error = $2,
                next_attempt_at = NOW() + INTERVAL '5 minutes' * power(2, attempts),
                updated_at = NOW()
            WHERE document_id = $1
            "#,
        )
        .bind(document_id)
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Story 28.2: Full-Text Search
    // ------------------------------------------------------------------------

    /// Full-text search across documents.
    pub async fn full_text_search(
        &self,
        request: DocumentSearchRequest,
    ) -> Result<DocumentSearchResponse, SqlxError> {
        // `documents` is under FORCE ROW LEVEL SECURITY (#754). This legacy
        // entrypoint acquires a connection and sets the request's org context
        // so the search stays correct, then delegates to the RLS variant.
        let mut conn = self.pool.acquire().await?;
        crate::tenant_context::set_request_context(
            &mut *conn,
            Some(request.organization_id),
            None,
            false,
        )
        .await?;
        let result = self.full_text_search_rls(&mut conn, request).await;
        let _ = crate::tenant_context::clear_request_context(&mut *conn).await;
        result
    }

    /// Full-text search across documents on an RLS-enabled connection.
    ///
    /// RLS-aware counterpart of [`Self::full_text_search`]. Runs both the
    /// result and count queries on the supplied org-scoped connection, so it
    /// stays correct under FORCE ROW LEVEL SECURITY on `documents` (#754).
    pub async fn full_text_search_rls(
        &self,
        conn: &mut sqlx::PgConnection,
        request: DocumentSearchRequest,
    ) -> Result<DocumentSearchResponse, SqlxError> {
        let limit = request.limit.unwrap_or(20).min(100);
        let offset = request.offset.unwrap_or(0);

        // Parse search query for PostgreSQL full-text search
        let ts_query = format!(
            "{}:*",
            request
                .query
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(":* & ")
        );

        let rows = sqlx::query(
            r#"
            SELECT
                d.id, d.title, d.category::text AS category, d.file_name, d.mime_type, d.size_bytes,
                d.folder_id, d.created_at,
                ts_rank_cd(d.search_vector, to_tsquery('english', $2)) as rank,
                ts_headline('english', COALESCE(d.extracted_text, d.description, ''),
                    to_tsquery('english', $2),
                    'StartSel=<mark>, StopSel=</mark>, MaxWords=50, MinWords=20') as headline,
                CASE
                    WHEN d.title ILIKE '%' || $6 || '%' THEN 'title'
                    WHEN d.extracted_text ILIKE '%' || $6 || '%' THEN 'content'
                    WHEN d.description ILIKE '%' || $6 || '%' THEN 'description'
                    ELSE 'other'
                END as matched_field
            FROM documents d
            WHERE d.organization_id = $1
              AND d.deleted_at IS NULL
              AND d.search_vector @@ to_tsquery('english', $2)
              AND ($3::uuid IS NULL OR d.folder_id = $3)
              AND ($4::text IS NULL OR d.category = $4::document_category)
            ORDER BY rank DESC, d.created_at DESC
            LIMIT $5 OFFSET $7
            "#,
        )
        .bind(request.organization_id)
        .bind(&ts_query)
        .bind(request.folder_id)
        .bind(&request.category)
        .bind(limit)
        .bind(&request.query)
        .bind(offset)
        .fetch_all(&mut *conn)
        .await?;

        // Get total count
        let count_row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM documents d
            WHERE d.organization_id = $1
              AND d.deleted_at IS NULL
              AND d.search_vector @@ to_tsquery('english', $2)
              AND ($3::uuid IS NULL OR d.folder_id = $3)
              AND ($4::text IS NULL OR d.category = $4::document_category)
            "#,
        )
        .bind(request.organization_id)
        .bind(&ts_query)
        .bind(request.folder_id)
        .bind(&request.category)
        .fetch_one(&mut *conn)
        .await?;

        let total: i64 = count_row.get("count");

        let results: Vec<DocumentSearchResult> = rows
            .into_iter()
            .map(|row| {
                let matched_field: String = row.get("matched_field");
                DocumentSearchResult {
                    document: DocumentSummary {
                        id: row.get("id"),
                        title: row.get("title"),
                        category: row.get("category"),
                        file_name: row.get("file_name"),
                        mime_type: row.get("mime_type"),
                        size_bytes: row.get("size_bytes"),
                        folder_id: row.get("folder_id"),
                        created_at: row.get("created_at"),
                    },
                    rank: row.get("rank"),
                    headline: row.get("headline"),
                    matched_fields: vec![matched_field],
                }
            })
            .collect();

        Ok(DocumentSearchResponse {
            results,
            total,
            query: request.query,
        })
    }

    // ------------------------------------------------------------------------
    // Story 28.3: Auto-Classification
    // ------------------------------------------------------------------------

    /// Update document classification.
    pub async fn update_classification(
        &self,
        document_id: Uuid,
        predicted_category: &str,
        confidence: f64,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE documents SET
                ai_predicted_category = $2,
                ai_classification_confidence = $3,
                ai_classification_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(document_id)
        .bind(predicted_category)
        .bind(confidence)
        .execute(&self.pool)
        .await?;

        // Record in history
        sqlx::query(
            r#"
            INSERT INTO document_classification_history
                (document_id, predicted_category, confidence)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(document_id)
        .bind(predicted_category)
        .bind(confidence)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Submit classification feedback.
    pub async fn submit_classification_feedback(
        &self,
        feedback: ClassificationFeedback,
    ) -> Result<(), SqlxError> {
        // Update the document
        sqlx::query(
            r#"
            UPDATE documents SET
                ai_classification_accepted = $2,
                category = COALESCE($3::document_category, category),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(feedback.document_id)
        .bind(feedback.accepted)
        .bind(&feedback.correct_category)
        .execute(&self.pool)
        .await?;

        // Update the latest classification history entry
        sqlx::query(
            r#"
            UPDATE document_classification_history SET
                was_accepted = $2,
                actual_category = $3,
                feedback_by = $4
            WHERE id = (
                SELECT id FROM document_classification_history
                WHERE document_id = $1
                ORDER BY created_at DESC
                LIMIT 1
            )
            "#,
        )
        .bind(feedback.document_id)
        .bind(feedback.accepted)
        .bind(&feedback.correct_category)
        .bind(feedback.feedback_by)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get classification history for a document.
    pub async fn get_classification_history(
        &self,
        document_id: Uuid,
    ) -> Result<Vec<DocumentClassificationHistory>, SqlxError> {
        sqlx::query_as::<_, DocumentClassificationHistory>(
            r#"
            SELECT * FROM document_classification_history
            WHERE document_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await
    }

    // ------------------------------------------------------------------------
    // Story 28.4: Document Summarization
    // ------------------------------------------------------------------------

    /// Queue a document for summarization.
    pub async fn queue_for_summarization(
        &self,
        request: GenerateSummaryRequest,
    ) -> Result<Uuid, SqlxError> {
        let row = sqlx::query(
            r#"
            INSERT INTO document_summarization_queue
                (document_id, priority, requested_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (document_id) DO UPDATE SET
                priority = LEAST(document_summarization_queue.priority, EXCLUDED.priority),
                next_attempt_at = NOW()
            RETURNING id
            "#,
        )
        .bind(request.document_id)
        .bind(request.priority.unwrap_or(5))
        .bind(request.requested_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("id"))
    }

    /// Get pending summarization queue items.
    pub async fn get_pending_summarization_items(
        &self,
        limit: i64,
    ) -> Result<Vec<DocumentSummarizationQueue>, SqlxError> {
        sqlx::query_as::<_, DocumentSummarizationQueue>(
            r#"
            SELECT * FROM document_summarization_queue
            WHERE attempts < max_attempts
              AND next_attempt_at <= NOW()
            ORDER BY priority, next_attempt_at
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Update document summary.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_summary(
        &self,
        document_id: Uuid,
        summary: &str,
        key_points: Option<serde_json::Value>,
        action_items: Option<serde_json::Value>,
        topics: Option<serde_json::Value>,
        word_count: Option<i32>,
        language: Option<&str>,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE documents SET
                ai_summary = $2,
                ai_key_points = COALESCE($3, ai_key_points),
                ai_action_items = COALESCE($4, ai_action_items),
                ai_topics = COALESCE($5, ai_topics),
                word_count = COALESCE($6, word_count),
                language_detected = COALESCE($7, language_detected),
                ai_summary_generated_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(document_id)
        .bind(summary)
        .bind(key_points)
        .bind(action_items)
        .bind(topics)
        .bind(word_count)
        .bind(language)
        .execute(&self.pool)
        .await?;

        // Remove from queue
        sqlx::query("DELETE FROM document_summarization_queue WHERE document_id = $1")
            .bind(document_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ------------------------------------------------------------------------
    // Statistics
    // ------------------------------------------------------------------------

    /// Get or create daily statistics for an organization.
    pub async fn get_or_create_daily_stats(
        &self,
        organization_id: Uuid,
        date: NaiveDate,
    ) -> Result<DocumentIntelligenceStats, SqlxError> {
        // Try to insert first
        let result = sqlx::query_as::<_, DocumentIntelligenceStats>(
            r#"
            INSERT INTO document_intelligence_stats (organization_id, date)
            VALUES ($1, $2)
            ON CONFLICT (organization_id, date) DO NOTHING
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(date)
        .fetch_optional(&self.pool)
        .await?;

        // If insert returned nothing (conflict), fetch existing
        match result {
            Some(stats) => Ok(stats),
            None => {
                sqlx::query_as::<_, DocumentIntelligenceStats>(
                    r#"
                    SELECT * FROM document_intelligence_stats
                    WHERE organization_id = $1 AND date = $2
                    "#,
                )
                .bind(organization_id)
                .bind(date)
                .fetch_one(&self.pool)
                .await
            }
        }
    }

    /// Increment OCR completed count.
    pub async fn increment_ocr_stats(
        &self,
        organization_id: Uuid,
        success: bool,
        pages: i32,
        confidence: Option<f64>,
    ) -> Result<(), SqlxError> {
        let today = Utc::now().date_naive();

        if success {
            sqlx::query(
                r#"
                INSERT INTO document_intelligence_stats
                    (organization_id, date, documents_processed, ocr_completed, total_pages_processed, avg_ocr_confidence)
                VALUES ($1, $2, 1, 1, $3, $4)
                ON CONFLICT (organization_id, date) DO UPDATE SET
                    documents_processed = document_intelligence_stats.documents_processed + 1,
                    ocr_completed = document_intelligence_stats.ocr_completed + 1,
                    total_pages_processed = document_intelligence_stats.total_pages_processed + EXCLUDED.total_pages_processed,
                    avg_ocr_confidence = (
                        COALESCE(document_intelligence_stats.avg_ocr_confidence, 0) * document_intelligence_stats.ocr_completed
                        + COALESCE(EXCLUDED.avg_ocr_confidence, 0)
                    ) / (document_intelligence_stats.ocr_completed + 1),
                    updated_at = NOW()
                "#,
            )
            .bind(organization_id)
            .bind(today)
            .bind(pages)
            .bind(confidence)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                INSERT INTO document_intelligence_stats
                    (organization_id, date, documents_processed, ocr_failed)
                VALUES ($1, $2, 1, 1)
                ON CONFLICT (organization_id, date) DO UPDATE SET
                    documents_processed = document_intelligence_stats.documents_processed + 1,
                    ocr_failed = document_intelligence_stats.ocr_failed + 1,
                    updated_at = NOW()
                "#,
            )
            .bind(organization_id)
            .bind(today)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Increment classification stats.
    pub async fn increment_classification_stats(
        &self,
        organization_id: Uuid,
        accepted: bool,
        confidence: f64,
    ) -> Result<(), SqlxError> {
        let today = Utc::now().date_naive();

        sqlx::query(
            r#"
            INSERT INTO document_intelligence_stats
                (organization_id, date, classifications_completed, classifications_accepted, avg_classification_confidence)
            VALUES ($1, $2, 1, $3::int, $4)
            ON CONFLICT (organization_id, date) DO UPDATE SET
                classifications_completed = document_intelligence_stats.classifications_completed + 1,
                classifications_accepted = document_intelligence_stats.classifications_accepted + EXCLUDED.classifications_accepted,
                avg_classification_confidence = (
                    COALESCE(document_intelligence_stats.avg_classification_confidence, 0) * document_intelligence_stats.classifications_completed
                    + EXCLUDED.avg_classification_confidence
                ) / (document_intelligence_stats.classifications_completed + 1),
                updated_at = NOW()
            "#,
        )
        .bind(organization_id)
        .bind(today)
        .bind(if accepted { 1 } else { 0 })
        .bind(confidence)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Increment summarization stats.
    pub async fn increment_summarization_stats(
        &self,
        organization_id: Uuid,
    ) -> Result<(), SqlxError> {
        let today = Utc::now().date_naive();

        sqlx::query(
            r#"
            INSERT INTO document_intelligence_stats
                (organization_id, date, summaries_generated)
            VALUES ($1, $2, 1)
            ON CONFLICT (organization_id, date) DO UPDATE SET
                summaries_generated = document_intelligence_stats.summaries_generated + 1,
                updated_at = NOW()
            "#,
        )
        .bind(organization_id)
        .bind(today)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get intelligence stats for a date range.
    pub async fn get_intelligence_stats(
        &self,
        organization_id: Uuid,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DocumentIntelligenceStats>, SqlxError> {
        sqlx::query_as::<_, DocumentIntelligenceStats>(
            r#"
            SELECT * FROM document_intelligence_stats
            WHERE organization_id = $1
              AND date >= $2
              AND date <= $3
            ORDER BY date DESC
            "#,
        )
        .bind(organization_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await
    }

    /// Get extracted text for a document (Epic 92).
    pub async fn get_extracted_text(&self, document_id: Uuid) -> Result<Option<String>, SqlxError> {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            r#"
            SELECT extracted_text FROM documents
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|(text,)| text))
    }

    /// Get extracted text for a document on an RLS-enabled connection (Epic 92).
    ///
    /// RLS-aware counterpart of [`Self::get_extracted_text`]. `documents` is
    /// under FORCE ROW LEVEL SECURITY (#754), so the read must run on the
    /// caller's org-scoped connection.
    pub async fn get_extracted_text_rls<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
    ) -> Result<Option<String>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row: Option<(Option<String>,)> = sqlx::query_as(
            r#"
            SELECT extracted_text FROM documents
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(document_id)
        .fetch_optional(executor)
        .await?;

        Ok(row.and_then(|(text,)| text))
    }
}
