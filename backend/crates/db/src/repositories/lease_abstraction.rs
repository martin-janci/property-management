//! Epic 133: AI Lease Abstraction & Document Intelligence repository.
//! Provides database operations for lease document processing and extraction.

use crate::models::lease_abstraction::*;
use crate::DbPool;
use common::errors::AppError;
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone)]
pub struct LeaseAbstractionRepository {
    pool: DbPool,
}

impl LeaseAbstractionRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ======================== Lease Documents (Story 133.1) ========================

    pub async fn create_document(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        req: CreateLeaseDocument,
    ) -> Result<LeaseDocument, AppError> {
        let document = sqlx::query_as::<_, LeaseDocument>(
            r#"
            INSERT INTO lease_documents (
                organization_id, uploaded_by, file_name, file_size_bytes,
                mime_type, storage_path, unit_id, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending')
            RETURNING id, organization_id, uploaded_by, file_name, file_size_bytes,
                      mime_type, storage_path, status, unit_id, processing_started_at,
                      processing_completed_at, error_message, page_count, created_at, updated_at
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(&req.file_name)
        .bind(req.file_size_bytes)
        .bind(&req.mime_type)
        .bind(&req.storage_path)
        .bind(req.unit_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(document)
    }

    pub async fn get_document(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<LeaseDocument>, AppError> {
        let document = sqlx::query_as::<_, LeaseDocument>(
            r#"
            SELECT id, organization_id, uploaded_by, file_name, file_size_bytes,
                   mime_type, storage_path, status, unit_id, processing_started_at,
                   processing_completed_at, error_message, page_count, created_at, updated_at
            FROM lease_documents
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(document)
    }

    pub async fn list_documents(
        &self,
        org_id: Uuid,
        query: LeaseDocumentQuery,
    ) -> Result<Vec<LeaseDocument>, AppError> {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.page.unwrap_or(0) * limit;

        let documents = sqlx::query_as::<_, LeaseDocument>(
            r#"
            SELECT id, organization_id, uploaded_by, file_name, file_size_bytes,
                   mime_type, storage_path, status, unit_id, processing_started_at,
                   processing_completed_at, error_message, page_count, created_at, updated_at
            FROM lease_documents
            WHERE organization_id = $1
              AND ($2::varchar IS NULL OR status = $2)
              AND ($3::uuid IS NULL OR unit_id = $3)
              AND ($4::timestamptz IS NULL OR created_at >= $4)
              AND ($5::timestamptz IS NULL OR created_at <= $5)
            ORDER BY created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(org_id)
        .bind(&query.status)
        .bind(query.unit_id)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(documents)
    }

    pub async fn update_document_status(
        &self,
        id: Uuid,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<(), AppError> {
        let now = if status == document_status::PROCESSING {
            "processing_started_at = NOW(),"
        } else if status == document_status::COMPLETED
            || status == document_status::FAILED
            || status == document_status::REVIEW_REQUIRED
        {
            "processing_completed_at = NOW(),"
        } else {
            ""
        };

        let query = format!(
            r#"
            UPDATE lease_documents
            SET status = $2,
                {}
                error_message = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
            now
        );

        sqlx::query(sqlx::AssertSqlSafe(query))
            .bind(id)
            .bind(status)
            .bind(error_message)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn set_document_page_count(&self, id: Uuid, page_count: i32) -> Result<(), AppError> {
        sqlx::query("UPDATE lease_documents SET page_count = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(page_count)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn delete_document(&self, id: Uuid, org_id: Uuid) -> Result<bool, AppError> {
        let result =
            sqlx::query("DELETE FROM lease_documents WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    // ======================== Lease Extractions (Story 133.2) ========================

    pub async fn create_extraction(
        &self,
        req: CreateLeaseExtraction,
    ) -> Result<LeaseExtraction, AppError> {
        // Calculate extracted and flagged fields
        let confidence_threshold = Decimal::new(80, 0);
        let mut fields_extracted = 0;
        let mut fields_flagged = 0;

        // Check each field
        let fields_and_confidences = [
            req.tenant_name.is_some(),
            req.landlord_name.is_some(),
            req.property_address.is_some(),
            req.lease_start_date.is_some(),
            req.lease_end_date.is_some(),
            req.monthly_rent.is_some(),
            req.security_deposit.is_some(),
            req.payment_due_day.is_some(),
        ];

        let confidences = [
            req.tenant_name_confidence,
            req.landlord_name_confidence,
            req.property_address_confidence,
            req.lease_start_date_confidence,
            req.lease_end_date_confidence,
            req.monthly_rent_confidence,
            req.security_deposit_confidence,
            req.payment_due_day_confidence,
        ];

        for (i, has_value) in fields_and_confidences.iter().enumerate() {
            if *has_value {
                fields_extracted += 1;
                if let Some(conf) = confidences[i] {
                    if conf < confidence_threshold {
                        fields_flagged += 1;
                    }
                }
            }
        }

        // Calculate overall confidence
        let overall_confidence = if fields_extracted > 0 {
            let sum: Decimal = confidences.iter().filter_map(|c| *c).sum();
            let count = confidences.iter().filter(|c| c.is_some()).count();
            if count > 0 {
                Some(sum / Decimal::from(count as i32))
            } else {
                None
            }
        } else {
            None
        };

        // Determine review status - all extractions need review regardless of confidence
        let review_status = review_status::PENDING;

        // Get next version
        let version: i32 = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT MAX(version) FROM lease_extractions WHERE document_id = $1",
        )
        .bind(req.document_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .unwrap_or(0)
            + 1;

        let extraction = sqlx::query_as::<_, LeaseExtraction>(
            r#"
            INSERT INTO lease_extractions (
                document_id, version,
                tenant_name, tenant_name_confidence, tenant_name_location,
                landlord_name, landlord_name_confidence, landlord_name_location,
                property_address, property_address_confidence, property_address_location,
                lease_start_date, lease_start_date_confidence, lease_start_date_location,
                lease_end_date, lease_end_date_confidence, lease_end_date_location,
                monthly_rent, monthly_rent_confidence, monthly_rent_location, rent_currency,
                security_deposit, security_deposit_confidence, security_deposit_location,
                payment_due_day, payment_due_day_confidence, payment_due_day_location,
                special_clauses, overall_confidence, fields_extracted, fields_flagged,
                model_used, extraction_duration_ms, review_status
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34
            )
            RETURNING *
            "#,
        )
        .bind(req.document_id)
        .bind(version)
        .bind(&req.tenant_name)
        .bind(req.tenant_name_confidence)
        .bind(&req.tenant_name_location)
        .bind(&req.landlord_name)
        .bind(req.landlord_name_confidence)
        .bind(&req.landlord_name_location)
        .bind(&req.property_address)
        .bind(req.property_address_confidence)
        .bind(&req.property_address_location)
        .bind(req.lease_start_date)
        .bind(req.lease_start_date_confidence)
        .bind(&req.lease_start_date_location)
        .bind(req.lease_end_date)
        .bind(req.lease_end_date_confidence)
        .bind(&req.lease_end_date_location)
        .bind(req.monthly_rent)
        .bind(req.monthly_rent_confidence)
        .bind(&req.monthly_rent_location)
        .bind(&req.rent_currency)
        .bind(req.security_deposit)
        .bind(req.security_deposit_confidence)
        .bind(&req.security_deposit_location)
        .bind(req.payment_due_day)
        .bind(req.payment_due_day_confidence)
        .bind(&req.payment_due_day_location)
        .bind(&req.special_clauses)
        .bind(overall_confidence)
        .bind(fields_extracted)
        .bind(fields_flagged)
        .bind(&req.model_used)
        .bind(req.extraction_duration_ms)
        .bind(review_status)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Update document status
        let doc_status = if fields_flagged > 0 {
            document_status::REVIEW_REQUIRED
        } else {
            document_status::COMPLETED
        };
        self.update_document_status(req.document_id, doc_status, None)
            .await?;

        Ok(extraction)
    }

    pub async fn get_extraction(&self, id: Uuid) -> Result<Option<LeaseExtraction>, AppError> {
        let extraction =
            sqlx::query_as::<_, LeaseExtraction>("SELECT * FROM lease_extractions WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(extraction)
    }

    pub async fn get_extraction_by_document(
        &self,
        document_id: Uuid,
    ) -> Result<Option<LeaseExtraction>, AppError> {
        let extraction = sqlx::query_as::<_, LeaseExtraction>(
            r#"
            SELECT * FROM lease_extractions
            WHERE document_id = $1
            ORDER BY version DESC
            LIMIT 1
            "#,
        )
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(extraction)
    }

    pub async fn approve_extraction(&self, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE lease_extractions
            SET review_status = 'approved', reviewed_by = $2, reviewed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn reject_extraction(&self, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE lease_extractions
            SET review_status = 'rejected', reviewed_by = $2, reviewed_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    // ======================== Extraction Corrections (Story 133.3) ========================

    pub async fn add_correction(
        &self,
        extraction_id: Uuid,
        user_id: Uuid,
        req: CreateExtractionCorrection,
    ) -> Result<ExtractionCorrection, AppError> {
        let correction = sqlx::query_as::<_, ExtractionCorrection>(
            r#"
            INSERT INTO extraction_corrections (
                extraction_id, corrected_by, field_name, original_value,
                corrected_value, correction_reason
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(extraction_id)
        .bind(user_id)
        .bind(&req.field_name)
        .bind(&req.original_value)
        .bind(&req.corrected_value)
        .bind(&req.correction_reason)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(correction)
    }

    pub async fn get_corrections(
        &self,
        extraction_id: Uuid,
    ) -> Result<Vec<ExtractionCorrection>, AppError> {
        let corrections = sqlx::query_as::<_, ExtractionCorrection>(
            r#"
            SELECT * FROM extraction_corrections
            WHERE extraction_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(extraction_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(corrections)
    }

    // ======================== Lease Imports (Story 133.4) ========================

    pub async fn create_import(&self, extraction_id: Uuid) -> Result<LeaseImport, AppError> {
        let import = sqlx::query_as::<_, LeaseImport>(
            r#"
            INSERT INTO lease_imports (extraction_id, status)
            VALUES ($1, 'pending')
            RETURNING *
            "#,
        )
        .bind(extraction_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(import)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_import_status(
        &self,
        id: Uuid,
        status: &str,
        lease_id: Option<Uuid>,
        user_id: Option<Uuid>,
        fields_imported: Option<serde_json::Value>,
        validation_errors: Option<serde_json::Value>,
        validation_warnings: Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE lease_imports
            SET status = $2,
                lease_id = COALESCE($3, lease_id),
                imported_by = COALESCE($4, imported_by),
                imported_at = CASE WHEN $2 = 'imported' THEN NOW() ELSE imported_at END,
                fields_imported = COALESCE($5, fields_imported),
                validation_errors = COALESCE($6, validation_errors),
                validation_warnings = COALESCE($7, validation_warnings)
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(lease_id)
        .bind(user_id)
        .bind(fields_imported)
        .bind(validation_errors)
        .bind(validation_warnings)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn get_import_by_extraction(
        &self,
        extraction_id: Uuid,
    ) -> Result<Option<LeaseImport>, AppError> {
        let import = sqlx::query_as::<_, LeaseImport>(
            "SELECT * FROM lease_imports WHERE extraction_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(extraction_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(import)
    }

    // ======================== Additional Methods for Routes ========================

    /// Get extraction with organization validation
    pub async fn get_extraction_for_org(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<LeaseExtraction>, AppError> {
        let extraction = sqlx::query_as::<_, LeaseExtraction>(
            r#"
            SELECT e.*
            FROM lease_extractions e
            JOIN lease_documents d ON e.document_id = d.id
            WHERE e.id = $1 AND d.organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(extraction)
    }

    /// List all extractions for a document
    pub async fn list_extractions(
        &self,
        document_id: Uuid,
    ) -> Result<Vec<LeaseExtraction>, AppError> {
        let extractions = sqlx::query_as::<_, LeaseExtraction>(
            r#"
            SELECT * FROM lease_extractions
            WHERE document_id = $1
            ORDER BY version DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(extractions)
    }

    /// Get extraction with fields expanded for UI
    #[allow(clippy::vec_init_then_push)]
    pub async fn get_extraction_with_fields(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<ExtractionWithFields>, AppError> {
        let extraction = self.get_extraction_for_org(id, org_id).await?;

        if let Some(ext) = extraction {
            // Build fields list
            let confidence_threshold = Decimal::new(80, 0);
            let mut fields = vec![];

            // Tenant name
            fields.push(ExtractedField {
                name: "tenant_name".to_string(),
                value: ext.tenant_name.clone(),
                confidence: ext.tenant_name_confidence,
                location: ext.tenant_name_location.clone(),
                needs_review: ext
                    .tenant_name_confidence
                    .map(|c| c < confidence_threshold)
                    .unwrap_or(false),
            });

            // Landlord name
            fields.push(ExtractedField {
                name: "landlord_name".to_string(),
                value: ext.landlord_name.clone(),
                confidence: ext.landlord_name_confidence,
                location: ext.landlord_name_location.clone(),
                needs_review: ext
                    .landlord_name_confidence
                    .map(|c| c < confidence_threshold)
                    .unwrap_or(false),
            });

            // Property address
            fields.push(ExtractedField {
                name: "property_address".to_string(),
                value: ext.property_address.clone(),
                confidence: ext.property_address_confidence,
                location: ext.property_address_location.clone(),
                needs_review: ext
                    .property_address_confidence
                    .map(|c| c < confidence_threshold)
                    .unwrap_or(false),
            });

            // Lease start date
            fields.push(ExtractedField {
                name: "lease_start_date".to_string(),
                value: ext.lease_start_date.map(|d| d.to_string()),
                confidence: ext.lease_start_date_confidence,
                location: ext.lease_start_date_location.clone(),
                needs_review: ext
                    .lease_start_date_confidence
                    .map(|c| c < confidence_threshold)
                    .unwrap_or(false),
            });

            // Lease end date
            fields.push(ExtractedField {
                name: "lease_end_date".to_string(),
                value: ext.lease_end_date.map(|d| d.to_string()),
                confidence: ext.lease_end_date_confidence,
                location: ext.lease_end_date_location.clone(),
                needs_review: ext
                    .lease_end_date_confidence
                    .map(|c| c < confidence_threshold)
                    .unwrap_or(false),
            });

            // Monthly rent
            fields.push(ExtractedField {
                name: "monthly_rent".to_string(),
                value: ext.monthly_rent.map(|r| r.to_string()),
                confidence: ext.monthly_rent_confidence,
                location: ext.monthly_rent_location.clone(),
                needs_review: ext
                    .monthly_rent_confidence
                    .map(|c| c < confidence_threshold)
                    .unwrap_or(false),
            });

            // Security deposit
            fields.push(ExtractedField {
                name: "security_deposit".to_string(),
                value: ext.security_deposit.map(|d| d.to_string()),
                confidence: ext.security_deposit_confidence,
                location: ext.security_deposit_location.clone(),
                needs_review: ext
                    .security_deposit_confidence
                    .map(|c| c < confidence_threshold)
                    .unwrap_or(false),
            });

            // Payment due day
            fields.push(ExtractedField {
                name: "payment_due_day".to_string(),
                value: ext.payment_due_day.map(|d| d.to_string()),
                confidence: ext.payment_due_day_confidence,
                location: ext.payment_due_day_location.clone(),
                needs_review: ext
                    .payment_due_day_confidence
                    .map(|c| c < confidence_threshold)
                    .unwrap_or(false),
            });

            // Get document summary
            let doc_row = sqlx::query_as::<
                _,
                (
                    Uuid,
                    String,
                    String,
                    Option<i32>,
                    chrono::DateTime<chrono::Utc>,
                ),
            >(
                r#"
                SELECT d.id, d.file_name, d.status, d.page_count, d.created_at
                FROM lease_documents d
                WHERE d.id = $1
                "#,
            )
            .bind(ext.document_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            let document = if let Some((id, file_name, status, page_count, created_at)) = doc_row {
                LeaseDocumentSummary {
                    id,
                    file_name,
                    status,
                    page_count,
                    created_at,
                    has_extraction: true,
                }
            } else {
                return Ok(None);
            };

            Ok(Some(ExtractionWithFields {
                extraction: ext,
                fields,
                document,
            }))
        } else {
            Ok(None)
        }
    }

    /// Approve extraction and return updated record
    pub async fn approve_extraction_returning(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<LeaseExtraction>, AppError> {
        self.approve_extraction(id, user_id).await?;
        self.get_extraction(id).await
    }

    /// Reject extraction and return updated record
    pub async fn reject_extraction_returning(
        &self,
        id: Uuid,
        user_id: Uuid,
        _reason: &str,
    ) -> Result<Option<LeaseExtraction>, AppError> {
        self.reject_extraction(id, user_id).await?;
        self.get_extraction(id).await
    }

    /// List corrections for an extraction
    pub async fn list_corrections(
        &self,
        extraction_id: Uuid,
    ) -> Result<Vec<ExtractionCorrection>, AppError> {
        self.get_corrections(extraction_id).await
    }

    /// Validate an import before executing
    pub async fn validate_import(
        &self,
        _extraction_id: Uuid,
        extraction: &LeaseExtraction,
    ) -> Result<ImportValidationResult, AppError> {
        let mut errors = vec![];
        let mut warnings = vec![];

        // Required field validation
        if extraction.tenant_name.is_none() {
            errors.push(ValidationIssue {
                field: "tenant_name".to_string(),
                message: "Tenant name is required".to_string(),
                severity: "error".to_string(),
            });
        }

        if extraction.lease_start_date.is_none() {
            errors.push(ValidationIssue {
                field: "lease_start_date".to_string(),
                message: "Lease start date is required".to_string(),
                severity: "error".to_string(),
            });
        }

        if extraction.monthly_rent.is_none() {
            errors.push(ValidationIssue {
                field: "monthly_rent".to_string(),
                message: "Monthly rent is required".to_string(),
                severity: "error".to_string(),
            });
        }

        // Warnings for low confidence
        let confidence_threshold = Decimal::new(80, 0);
        if let Some(conf) = extraction.overall_confidence {
            if conf < confidence_threshold {
                warnings.push(ValidationIssue {
                    field: "overall".to_string(),
                    message: format!("Overall extraction confidence is low ({:.0}%)", conf),
                    severity: "warning".to_string(),
                });
            }
        }

        Ok(ImportValidationResult {
            can_import: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// Import extraction to create a lease
    pub async fn import_to_lease(
        &self,
        extraction_id: Uuid,
        _unit_id: Uuid,
        user_id: Uuid,
        _overrides: serde_json::Value,
    ) -> Result<ImportResult, AppError> {
        // Create import record
        let import = self.create_import(extraction_id).await?;

        // In production, this would create an actual lease record
        // For now, just update the import status
        let fields_imported = json!(["tenant_name", "monthly_rent", "lease_start_date"]);
        self.update_import_status(
            import.id,
            import_status::IMPORTED,
            None, // Would be the lease_id
            Some(user_id),
            Some(fields_imported.clone()),
            None,
            None,
        )
        .await?;

        Ok(ImportResult {
            success: true,
            lease_id: None, // Would be populated with actual lease ID
            fields_imported: vec![
                "tenant_name".to_string(),
                "monthly_rent".to_string(),
                "lease_start_date".to_string(),
            ],
            errors: vec![],
        })
    }

    /// List imports for an organization
    pub async fn list_imports(&self, org_id: Uuid) -> Result<Vec<LeaseImport>, AppError> {
        let imports = sqlx::query_as::<_, LeaseImport>(
            r#"
            SELECT i.*
            FROM lease_imports i
            JOIN lease_extractions e ON i.extraction_id = e.id
            JOIN lease_documents d ON e.document_id = d.id
            WHERE d.organization_id = $1
            ORDER BY i.created_at DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(imports)
    }

    /// Get import by ID with organization validation
    pub async fn get_import(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<LeaseImport>, AppError> {
        let import = sqlx::query_as::<_, LeaseImport>(
            r#"
            SELECT i.*
            FROM lease_imports i
            JOIN lease_extractions e ON i.extraction_id = e.id
            JOIN lease_documents d ON e.document_id = d.id
            WHERE i.id = $1 AND d.organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(import)
    }

    // ======================== Statistics ========================

    pub async fn get_processing_stats(&self, org_id: Uuid) -> Result<serde_json::Value, AppError> {
        let stats = sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status = 'pending') as pending,
                COUNT(*) FILTER (WHERE status = 'processing') as processing,
                COUNT(*) FILTER (WHERE status = 'completed') as completed,
                COUNT(*) FILTER (WHERE status = 'failed') as failed,
                COUNT(*) FILTER (WHERE status = 'review_required') as review_required
            FROM lease_documents
            WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(json!({
            "pending": stats.0,
            "processing": stats.1,
            "completed": stats.2,
            "failed": stats.3,
            "review_required": stats.4,
            "total": stats.0 + stats.1 + stats.2 + stats.3 + stats.4
        }))
    }
}
