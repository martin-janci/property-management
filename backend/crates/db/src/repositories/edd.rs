//! Enhanced Due Diligence (EDD) Repository (Epic 100, Story 100.4).
//!
//! Handles AML risk assessments, Enhanced Due Diligence records,
//! EDD document management, and compliance verification.

use crate::models::compliance::{
    AddComplianceNote, AmlAssessmentStatus, AmlRiskAssessment, AmlRiskLevel, ComplianceNote,
    CountryRisk, CountryRiskRating, CreateAmlRiskAssessment, CreateEddDocument,
    CreateEnhancedDueDiligence, DocumentVerificationStatus, EddDocument, EddStatus,
    EnhancedDueDiligence, RiskFactor,
};
use crate::DbPool;
use chrono::{DateTime, Duration, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for EDD and AML compliance operations.
#[derive(Clone)]
pub struct EddRepository {
    pool: DbPool,
}

impl EddRepository {
    /// Create a new EddRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // COUNTRY RISK DATABASE
    // ========================================================================

    /// Get all country risk entries.
    pub async fn list_country_risks(&self) -> Result<Vec<CountryRisk>, SqlxError> {
        let risks = sqlx::query_as::<_, CountryRisk>(
            r#"
            SELECT country_code, country_name, risk_rating, is_sanctioned, fatf_status, notes, updated_at
            FROM country_risks
            ORDER BY country_name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(risks)
    }

    /// Get country risk by country code.
    pub async fn get_country_risk(
        &self,
        country_code: &str,
    ) -> Result<Option<CountryRisk>, SqlxError> {
        let risk = sqlx::query_as::<_, CountryRisk>(
            r#"
            SELECT country_code, country_name, risk_rating, is_sanctioned, fatf_status, notes, updated_at
            FROM country_risks
            WHERE country_code = $1
            "#,
        )
        .bind(country_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(risk)
    }

    /// Update country risk rating.
    pub async fn update_country_risk(
        &self,
        country_code: &str,
        risk_rating: CountryRiskRating,
        is_sanctioned: bool,
        fatf_status: Option<&str>,
        notes: Option<&str>,
    ) -> Result<CountryRisk, SqlxError> {
        let risk = sqlx::query_as::<_, CountryRisk>(
            r#"
            UPDATE country_risks SET
                risk_rating = $2,
                is_sanctioned = $3,
                fatf_status = $4,
                notes = $5,
                updated_at = NOW()
            WHERE country_code = $1
            RETURNING *
            "#,
        )
        .bind(country_code)
        .bind(risk_rating)
        .bind(is_sanctioned)
        .bind(fatf_status)
        .bind(notes)
        .fetch_one(&self.pool)
        .await?;

        Ok(risk)
    }

    // ========================================================================
    // AML RISK ASSESSMENTS
    // ========================================================================

    /// Create a new AML risk assessment.
    pub async fn create_aml_assessment(
        &self,
        data: CreateAmlRiskAssessment,
    ) -> Result<AmlRiskAssessment, SqlxError> {
        // Calculate risk score based on various factors
        let mut risk_score = 0i32;
        let mut risk_factors: Vec<RiskFactor> = Vec::new();

        // Factor 1: Transaction amount (if above threshold - 10,000 EUR)
        let aml_threshold = 1_000_000i64; // 10,000 EUR in cents
        if let Some(amount) = data.transaction_amount_cents {
            if amount >= aml_threshold {
                risk_score += 30;
                risk_factors.push(RiskFactor {
                    factor_type: "high_value_transaction".to_string(),
                    description: format!(
                        "Transaction amount ({} cents) exceeds AML threshold",
                        amount
                    ),
                    weight: 30,
                    mitigated: false,
                });
            }
        }

        // Factor 2: Country risk
        let (country_risk, country_risk_weight) = if let Some(ref code) = data.country_code {
            match self.get_country_risk(code).await? {
                Some(risk) => {
                    let weight = match risk.risk_rating {
                        CountryRiskRating::Sanctioned | CountryRiskRating::High => {
                            risk_factors.push(RiskFactor {
                                factor_type: "high_risk_country".to_string(),
                                description: "Party is from a high-risk or sanctioned jurisdiction"
                                    .to_string(),
                                weight: 40,
                                mitigated: false,
                            });
                            40
                        }
                        CountryRiskRating::Medium => {
                            risk_factors.push(RiskFactor {
                                factor_type: "medium_risk_country".to_string(),
                                description: "Party is from a medium-risk jurisdiction".to_string(),
                                weight: 20,
                                mitigated: false,
                            });
                            20
                        }
                        CountryRiskRating::Low => 0,
                    };
                    (Some(risk.risk_rating), weight)
                }
                None => (None, 0),
            }
        } else {
            (None, 0)
        };
        risk_score += country_risk_weight;

        // Factor 3: Party type
        if data.party_type == "company" {
            risk_score += 10;
            risk_factors.push(RiskFactor {
                factor_type: "corporate_party".to_string(),
                description: "Corporate entities require additional due diligence".to_string(),
                weight: 10,
                mitigated: false,
            });
        }

        // Determine risk level
        let risk_level = match risk_score {
            0..=25 => AmlRiskLevel::Low,
            26..=50 => AmlRiskLevel::Medium,
            51..=75 => AmlRiskLevel::High,
            _ => AmlRiskLevel::Critical,
        };

        // Determine if flagged for review
        let flagged_for_review =
            risk_level == AmlRiskLevel::High || risk_level == AmlRiskLevel::Critical;

        let review_reason = if flagged_for_review {
            Some("Risk score exceeds threshold".to_string())
        } else {
            None
        };

        let status = if flagged_for_review {
            AmlAssessmentStatus::RequiresReview
        } else {
            AmlAssessmentStatus::Completed
        };

        let assessment = sqlx::query_as::<_, AmlRiskAssessment>(
            r#"
            INSERT INTO aml_risk_assessments (
                organization_id, transaction_id, party_id, party_type,
                transaction_amount_cents, currency, risk_score, risk_level,
                status, risk_factors, country_code, country_risk,
                flagged_for_review, review_reason, assessed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW())
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(data.transaction_id)
        .bind(data.party_id)
        .bind(&data.party_type)
        .bind(data.transaction_amount_cents)
        .bind(&data.currency)
        .bind(risk_score)
        .bind(risk_level)
        .bind(status)
        .bind(serde_json::to_value(&risk_factors).ok())
        .bind(&data.country_code)
        .bind(country_risk)
        .bind(flagged_for_review)
        .bind(&review_reason)
        .fetch_one(&self.pool)
        .await?;

        Ok(assessment)
    }

    /// Get AML assessment by ID, scoped to the caller's organization.
    ///
    /// The `org_id` filter is a defense-in-depth tenant-isolation guard: these
    /// handlers run on the raw pool (RLS GUCs are not set), so the
    /// `aml_risk_assessments_tenant_isolation` RLS policy cannot protect a
    /// global-id lookup. Scoping by `organization_id` here ensures a compliance
    /// officer in one org cannot read another org's assessment by guessing the
    /// UUID (cross-org IDOR — see issue #897).
    pub async fn get_aml_assessment(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<AmlRiskAssessment>, SqlxError> {
        let assessment = sqlx::query_as::<_, AmlRiskAssessment>(
            r#"SELECT * FROM aml_risk_assessments WHERE id = $1 AND organization_id = $2"#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(assessment)
    }

    /// List AML assessments for an organization.
    pub async fn list_aml_assessments(
        &self,
        org_id: Uuid,
        status: Option<AmlAssessmentStatus>,
        risk_level: Option<AmlRiskLevel>,
        flagged_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<AmlRiskAssessment>, i64), SqlxError> {
        let (total,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM aml_risk_assessments
            WHERE organization_id = $1
                AND ($2::aml_assessment_status IS NULL OR status = $2)
                AND ($3::aml_risk_level IS NULL OR risk_level = $3)
                AND ($4 = FALSE OR flagged_for_review = TRUE)
            "#,
        )
        .bind(org_id)
        .bind(status)
        .bind(risk_level)
        .bind(flagged_only)
        .fetch_one(&self.pool)
        .await?;

        let assessments = sqlx::query_as::<_, AmlRiskAssessment>(
            r#"
            SELECT * FROM aml_risk_assessments
            WHERE organization_id = $1
                AND ($2::aml_assessment_status IS NULL OR status = $2)
                AND ($3::aml_risk_level IS NULL OR risk_level = $3)
                AND ($4 = FALSE OR flagged_for_review = TRUE)
            ORDER BY created_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(org_id)
        .bind(status)
        .bind(risk_level)
        .bind(flagged_only)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((assessments, total))
    }

    /// Review an AML assessment.
    pub async fn review_aml_assessment(
        &self,
        id: Uuid,
        org_id: Uuid,
        reviewer_id: Uuid,
        decision: AmlAssessmentStatus,
        notes: Option<&str>,
    ) -> Result<AmlRiskAssessment, SqlxError> {
        let assessment = sqlx::query_as::<_, AmlRiskAssessment>(
            r#"
            UPDATE aml_risk_assessments SET
                status = $2,
                assessed_by = $3,
                assessed_at = NOW(),
                assessor_notes = COALESCE($4, assessor_notes),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $5
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(decision)
        .bind(reviewer_id)
        .bind(notes)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(assessment)
    }

    /// Update verification status for an assessment.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_assessment_verification(
        &self,
        id: Uuid,
        id_verified: Option<bool>,
        source_of_funds_documented: Option<bool>,
        pep_check_completed: Option<bool>,
        is_pep: Option<bool>,
        sanctions_check_completed: Option<bool>,
        sanctions_match: Option<bool>,
    ) -> Result<AmlRiskAssessment, SqlxError> {
        let assessment = sqlx::query_as::<_, AmlRiskAssessment>(
            r#"
            UPDATE aml_risk_assessments SET
                id_verified = COALESCE($2, id_verified),
                source_of_funds_documented = COALESCE($3, source_of_funds_documented),
                pep_check_completed = COALESCE($4, pep_check_completed),
                is_pep = COALESCE($5, is_pep),
                sanctions_check_completed = COALESCE($6, sanctions_check_completed),
                sanctions_match = COALESCE($7, sanctions_match),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(id_verified)
        .bind(source_of_funds_documented)
        .bind(pep_check_completed)
        .bind(is_pep)
        .bind(sanctions_check_completed)
        .bind(sanctions_match)
        .fetch_one(&self.pool)
        .await?;

        Ok(assessment)
    }

    // ========================================================================
    // ENHANCED DUE DILIGENCE (EDD) RECORDS
    // ========================================================================

    /// Create a new EDD record.
    pub async fn create_edd(
        &self,
        data: CreateEnhancedDueDiligence,
    ) -> Result<EnhancedDueDiligence, SqlxError> {
        let documents_requested = data
            .documents_requested
            .and_then(|d| serde_json::to_value(d).ok());
        let next_review_date = Utc::now() + Duration::days(365);

        let edd = sqlx::query_as::<_, EnhancedDueDiligence>(
            r#"
            INSERT INTO edd_records (
                aml_assessment_id, organization_id, party_id, status,
                documents_requested, initiated_by, next_review_date
            )
            VALUES ($1, $2, $3, 'in_progress', $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(data.aml_assessment_id)
        .bind(data.organization_id)
        .bind(data.party_id)
        .bind(documents_requested)
        .bind(data.initiated_by)
        .bind(next_review_date.date_naive())
        .fetch_one(&self.pool)
        .await?;

        // Update the AML assessment status
        sqlx::query(
            r#"UPDATE aml_risk_assessments SET status = 'in_progress', updated_at = NOW() WHERE id = $1"#,
        )
        .bind(data.aml_assessment_id)
        .execute(&self.pool)
        .await?;

        Ok(edd)
    }

    /// Get EDD record by ID, scoped to the caller's organization.
    ///
    /// Like [`get_aml_assessment`](Self::get_aml_assessment), this enforces
    /// tenant isolation at the query level because the AML/DSA handlers run on
    /// the raw pool without RLS GUCs. Every EDD sub-resource handler
    /// (documents, notes, completion) gates on this lookup, so scoping it here
    /// closes the whole EDD subtree against cross-org access (issue #897).
    pub async fn get_edd(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<EnhancedDueDiligence>, SqlxError> {
        let edd = sqlx::query_as::<_, EnhancedDueDiligence>(
            r#"SELECT * FROM edd_records WHERE id = $1 AND organization_id = $2"#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(edd)
    }

    /// Get EDD record by AML assessment ID.
    pub async fn get_edd_by_assessment(
        &self,
        aml_assessment_id: Uuid,
    ) -> Result<Option<EnhancedDueDiligence>, SqlxError> {
        let edd = sqlx::query_as::<_, EnhancedDueDiligence>(
            r#"SELECT * FROM edd_records WHERE aml_assessment_id = $1"#,
        )
        .bind(aml_assessment_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(edd)
    }

    /// List pending EDD records for an organization.
    pub async fn list_pending_edd(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<EnhancedDueDiligence>, SqlxError> {
        let edds = sqlx::query_as::<_, EnhancedDueDiligence>(
            r#"
            SELECT * FROM edd_records
            WHERE organization_id = $1
                AND status IN ('required', 'in_progress', 'pending_documents', 'under_review')
            ORDER BY initiated_at ASC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(edds)
    }

    /// List EDD records requiring review (next_review_date in the past).
    pub async fn list_edd_requiring_review(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<EnhancedDueDiligence>, SqlxError> {
        let today = Utc::now().date_naive();
        let edds = sqlx::query_as::<_, EnhancedDueDiligence>(
            r#"
            SELECT * FROM edd_records
            WHERE organization_id = $1
                AND status = 'completed'
                AND next_review_date IS NOT NULL
                AND next_review_date <= $2
            ORDER BY next_review_date ASC
            "#,
        )
        .bind(org_id)
        .bind(today)
        .fetch_all(&self.pool)
        .await?;

        Ok(edds)
    }

    /// Update EDD record.
    pub async fn update_edd(
        &self,
        id: Uuid,
        source_of_wealth: Option<&str>,
        source_of_funds: Option<&str>,
        beneficial_ownership: Option<serde_json::Value>,
        relationship_purpose: Option<&str>,
        expected_transaction_patterns: Option<&str>,
    ) -> Result<EnhancedDueDiligence, SqlxError> {
        let edd = sqlx::query_as::<_, EnhancedDueDiligence>(
            r#"
            UPDATE edd_records SET
                source_of_wealth = COALESCE($2, source_of_wealth),
                source_of_funds = COALESCE($3, source_of_funds),
                beneficial_ownership = COALESCE($4, beneficial_ownership),
                relationship_purpose = COALESCE($5, relationship_purpose),
                expected_transaction_patterns = COALESCE($6, expected_transaction_patterns),
                status = CASE
                    WHEN status = 'required' THEN 'in_progress'::edd_status
                    ELSE status
                END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(source_of_wealth)
        .bind(source_of_funds)
        .bind(beneficial_ownership)
        .bind(relationship_purpose)
        .bind(expected_transaction_patterns)
        .fetch_one(&self.pool)
        .await?;

        Ok(edd)
    }

    /// Update EDD status.
    pub async fn update_edd_status(
        &self,
        id: Uuid,
        status: EddStatus,
    ) -> Result<EnhancedDueDiligence, SqlxError> {
        let edd = sqlx::query_as::<_, EnhancedDueDiligence>(
            r#"
            UPDATE edd_records SET
                status = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        Ok(edd)
    }

    /// Complete EDD process.
    pub async fn complete_edd(
        &self,
        id: Uuid,
        completed_by: Uuid,
        next_review_date: Option<DateTime<Utc>>,
    ) -> Result<EnhancedDueDiligence, SqlxError> {
        let next_review = next_review_date.unwrap_or_else(|| Utc::now() + Duration::days(365));

        let edd = sqlx::query_as::<_, EnhancedDueDiligence>(
            r#"
            UPDATE edd_records SET
                status = 'completed',
                completed_at = NOW(),
                completed_by = $2,
                next_review_date = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(completed_by)
        .bind(next_review.date_naive())
        .fetch_one(&self.pool)
        .await?;

        // Update the AML assessment status
        sqlx::query(
            r#"UPDATE aml_risk_assessments SET status = 'approved', updated_at = NOW() WHERE id = $1"#,
        )
        .bind(edd.aml_assessment_id)
        .execute(&self.pool)
        .await?;

        Ok(edd)
    }

    /// Add compliance note to EDD record.
    pub async fn add_compliance_note(
        &self,
        edd_id: Uuid,
        note: AddComplianceNote,
        added_by: Uuid,
        added_by_name: &str,
    ) -> Result<ComplianceNote, SqlxError> {
        let note_entry = ComplianceNote {
            id: Uuid::new_v4(),
            content: note.content,
            added_by,
            added_by_name: added_by_name.to_string(),
            added_at: Utc::now(),
        };

        // Append note to the compliance_notes JSON array
        sqlx::query(
            r#"
            UPDATE edd_records SET
                compliance_notes = COALESCE(compliance_notes, '[]'::jsonb) || $2::jsonb,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(edd_id)
        .bind(serde_json::to_value(&note_entry).unwrap())
        .execute(&self.pool)
        .await?;

        Ok(note_entry)
    }

    /// Get compliance notes for an EDD record.
    pub async fn get_compliance_notes(
        &self,
        edd_id: Uuid,
    ) -> Result<Vec<ComplianceNote>, SqlxError> {
        let (notes_json,): (Option<serde_json::Value>,) =
            sqlx::query_as(r#"SELECT compliance_notes FROM edd_records WHERE id = $1"#)
                .bind(edd_id)
                .fetch_one(&self.pool)
                .await?;

        let notes: Vec<ComplianceNote> = notes_json
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        Ok(notes)
    }

    // ========================================================================
    // EDD DOCUMENTS
    // ========================================================================

    /// Upload document to EDD record.
    pub async fn upload_edd_document(
        &self,
        data: CreateEddDocument,
    ) -> Result<EddDocument, SqlxError> {
        let doc = sqlx::query_as::<_, EddDocument>(
            r#"
            INSERT INTO edd_documents (
                edd_id, document_type, file_path, original_filename,
                file_size_bytes, mime_type, expiry_date, uploaded_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(data.edd_id)
        .bind(&data.document_type)
        .bind(&data.file_path)
        .bind(&data.original_filename)
        .bind(data.file_size_bytes)
        .bind(&data.mime_type)
        .bind(data.expiry_date.map(|d| d.date_naive()))
        .bind(data.uploaded_by)
        .fetch_one(&self.pool)
        .await?;

        // Update EDD status if it was waiting for documents
        sqlx::query(
            r#"
            UPDATE edd_records SET
                status = CASE
                    WHEN status = 'pending_documents' THEN 'under_review'::edd_status
                    ELSE status
                END,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(data.edd_id)
        .execute(&self.pool)
        .await?;

        Ok(doc)
    }

    /// Get EDD document by ID.
    pub async fn get_edd_document(&self, id: Uuid) -> Result<Option<EddDocument>, SqlxError> {
        let doc = sqlx::query_as::<_, EddDocument>(r#"SELECT * FROM edd_documents WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(doc)
    }

    /// List documents for an EDD record.
    pub async fn list_edd_documents(&self, edd_id: Uuid) -> Result<Vec<EddDocument>, SqlxError> {
        let docs = sqlx::query_as::<_, EddDocument>(
            r#"
            SELECT * FROM edd_documents
            WHERE edd_id = $1
            ORDER BY uploaded_at DESC
            "#,
        )
        .bind(edd_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(docs)
    }

    /// Verify an EDD document.
    pub async fn verify_edd_document(
        &self,
        id: Uuid,
        edd_id: Uuid,
        verified_by: Uuid,
        status: DocumentVerificationStatus,
        rejection_reason: Option<&str>,
    ) -> Result<EddDocument, SqlxError> {
        let doc = sqlx::query_as::<_, EddDocument>(
            r#"
            UPDATE edd_documents SET
                verification_status = $2,
                verified_by = $3,
                verified_at = NOW(),
                rejection_reason = $4
            WHERE id = $1 AND edd_id = $5
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(verified_by)
        .bind(rejection_reason)
        .bind(edd_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(doc)
    }

    /// Check if all required documents are verified for an EDD record.
    pub async fn are_all_documents_verified(&self, edd_id: Uuid) -> Result<bool, SqlxError> {
        let (pending_count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM edd_documents
            WHERE edd_id = $1 AND verification_status != 'verified'
            "#,
        )
        .bind(edd_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(pending_count == 0)
    }

    /// List expiring documents (expiry date within next 30 days).
    pub async fn list_expiring_documents(
        &self,
        org_id: Uuid,
        days_ahead: i32,
    ) -> Result<Vec<EddDocument>, SqlxError> {
        let cutoff = Utc::now() + Duration::days(days_ahead as i64);

        let docs = sqlx::query_as::<_, EddDocument>(
            r#"
            SELECT d.* FROM edd_documents d
            JOIN edd_records e ON e.id = d.edd_id
            WHERE e.organization_id = $1
                AND d.expiry_date IS NOT NULL
                AND d.expiry_date <= $2
                AND d.verification_status = 'verified'
            ORDER BY d.expiry_date ASC
            "#,
        )
        .bind(org_id)
        .bind(cutoff.date_naive())
        .fetch_all(&self.pool)
        .await?;

        Ok(docs)
    }

    // ========================================================================
    // STATISTICS
    // ========================================================================

    /// Get EDD statistics for an organization.
    pub async fn get_edd_statistics(&self, org_id: Uuid) -> Result<EddStatistics, SqlxError> {
        let (total_assessments,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM aml_risk_assessments WHERE organization_id = $1"#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let (flagged_assessments,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM aml_risk_assessments WHERE organization_id = $1 AND flagged_for_review = TRUE"#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let (high_risk_assessments,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM aml_risk_assessments WHERE organization_id = $1 AND risk_level IN ('high', 'critical')"#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let (pending_edd,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM edd_records WHERE organization_id = $1 AND status IN ('required', 'in_progress', 'pending_documents', 'under_review')"#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let (completed_edd,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM edd_records WHERE organization_id = $1 AND status = 'completed'"#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let today = Utc::now().date_naive();
        let (requiring_review,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM edd_records WHERE organization_id = $1 AND status = 'completed' AND next_review_date <= $2"#,
        )
        .bind(org_id)
        .bind(today)
        .fetch_one(&self.pool)
        .await?;

        let (pending_documents,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM edd_documents d
            JOIN edd_records e ON e.id = d.edd_id
            WHERE e.organization_id = $1 AND d.verification_status = 'pending'
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(EddStatistics {
            total_assessments,
            flagged_assessments,
            high_risk_assessments,
            pending_edd,
            completed_edd,
            requiring_review,
            pending_document_verifications: pending_documents,
        })
    }
}

/// EDD statistics summary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EddStatistics {
    /// Total AML assessments
    pub total_assessments: i64,
    /// Assessments flagged for review
    pub flagged_assessments: i64,
    /// High/critical risk assessments
    pub high_risk_assessments: i64,
    /// Pending EDD processes
    pub pending_edd: i64,
    /// Completed EDD processes
    pub completed_edd: i64,
    /// EDD records requiring periodic review
    pub requiring_review: i64,
    /// Documents pending verification
    pub pending_document_verifications: i64,
}
