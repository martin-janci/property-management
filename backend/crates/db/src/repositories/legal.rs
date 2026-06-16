//! Legal document and compliance repository (Epic 25).
//!
//! # RLS Integration (PAP-80 / PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on all eight legal/compliance tables
//! (`legal_documents`, `legal_document_versions`, `compliance_requirements`,
//! `compliance_verifications`, `legal_notices`, `legal_notice_recipients`,
//! `compliance_templates`, `compliance_audit_trail`). Under `FORCE` the
//! api-server's owner connection is no longer exempt, so a query issued on a
//! connection without `app.current_org_id` set collapses to deny-all (own-org
//! reads return empty, writes fail the policy `WITH CHECK`).
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
//! pool**, so there is no way to issue a query that bypasses RLS. Single-
//! statement methods take a generic `Executor`; methods that run more than one
//! statement (or call an `*_in_org` guard before their query) take
//! `&mut PgConnection` and reborrow it per query. This mirrors the
//! `work_order.rs` / `budget.rs` / `document.rs` precedent.
//!
//! The explicit `organization_id = $n` filters are retained as defence in
//! depth: the handler passes `rls.tenant_id()` as the authoritative org, which
//! is the same tenant the connection's RLS context was set to, so the SQL
//! filter and the policy can never disagree.

use crate::models::{
    AcknowledgeNotice, ApplyTemplate, ComplianceAuditTrail, ComplianceCategoryCount,
    ComplianceQuery, ComplianceRequirement, ComplianceRequirementWithDetails, ComplianceStatistics,
    ComplianceTemplate, ComplianceVerification, CreateAuditTrailEntry, CreateComplianceRequirement,
    CreateComplianceTemplate, CreateComplianceVerification, CreateLegalDocument,
    CreateLegalDocumentVersion, CreateLegalNotice, LegalDocument, LegalDocumentQuery,
    LegalDocumentSummary, LegalDocumentVersion, LegalNotice, LegalNoticeQuery,
    LegalNoticeRecipient, NoticeAcknowledgmentStats, NoticeStatistics, NoticeTypeCount,
    NoticeWithRecipients, UpcomingVerification, UpdateComplianceRequirement,
    UpdateComplianceTemplate, UpdateLegalDocument, UpdateLegalNotice,
};
use chrono::{Months, NaiveDate};
use sqlx::{Executor, PgConnection, PgPool, Postgres};
use uuid::Uuid;

/// Repository for legal document and compliance operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Clone)]
pub struct LegalRepository;

impl LegalRepository {
    /// Create a new LegalRepository.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    // ==================== Legal Documents CRUD ====================

    /// Create a new legal document.
    pub async fn create_document<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateLegalDocument,
    ) -> Result<LegalDocument, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Calculate retention expiry date if retention period is provided
        // Using proper month arithmetic for accuracy
        let retention_expires_at = data.retention_period_months.map(|months| {
            chrono::Utc::now()
                .date_naive()
                .checked_add_months(Months::new(months as u32))
                .unwrap_or_else(|| chrono::Utc::now().date_naive())
        });

        sqlx::query_as(
            r#"
            INSERT INTO legal_documents
                (organization_id, building_id, document_type, title, description, parties,
                 effective_date, expiry_date, file_path, file_name, file_size, mime_type,
                 is_confidential, retention_period_months, retention_expires_at, tags, metadata,
                 created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(&data.document_type)
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.parties.map(sqlx::types::Json))
        .bind(data.effective_date)
        .bind(data.expiry_date)
        .bind(&data.file_path)
        .bind(&data.file_name)
        .bind(data.file_size)
        .bind(&data.mime_type)
        .bind(data.is_confidential.unwrap_or(false))
        .bind(data.retention_period_months)
        .bind(retention_expires_at)
        .bind(&data.tags)
        .bind(data.metadata.map(sqlx::types::Json))
        .bind(user_id)
        .fetch_one(executor)
        .await
    }

    /// Find a legal document by ID, scoped to an organization.
    ///
    /// A foreign-org `id` returns `None` (→ HTTP 404), preventing cross-tenant
    /// reads (IDOR, #829) — now enforced by RLS as well as the explicit filter.
    pub async fn find_document_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<LegalDocument>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM legal_documents WHERE id = $1 AND organization_id = $2")
            .bind(id)
            .bind(org_id)
            .fetch_optional(executor)
            .await
    }

    /// List legal documents for an organization.
    pub async fn list_documents<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: LegalDocumentQuery,
    ) -> Result<Vec<LegalDocument>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let search_pattern = query.search.as_ref().map(|s| format!("%{}%", s));

        sqlx::query_as(
            r#"
            SELECT * FROM legal_documents
            WHERE organization_id = $1
            AND ($2::uuid IS NULL OR building_id = $2)
            AND ($3::text IS NULL OR document_type = $3)
            AND ($4::boolean IS NULL OR is_confidential = $4)
            AND ($5::integer IS NULL OR expiry_date <= CURRENT_DATE + $5::integer)
            AND ($6::text IS NULL OR $6 = ANY(tags))
            AND ($7::text IS NULL OR title ILIKE $7 OR description ILIKE $7)
            ORDER BY created_at DESC
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(&query.document_type)
        .bind(query.is_confidential)
        .bind(query.expiring_days)
        .bind(&query.tag)
        .bind(&search_pattern)
        .bind(query.limit.unwrap_or(50))
        .bind(query.offset.unwrap_or(0))
        .fetch_all(executor)
        .await
    }

    /// List legal documents with version counts.
    pub async fn list_documents_with_summary<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: LegalDocumentQuery,
    ) -> Result<Vec<LegalDocumentSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let search_pattern = query.search.as_ref().map(|s| format!("%{}%", s));

        sqlx::query_as(
            r#"
            SELECT
                d.id, d.organization_id, d.building_id, d.document_type, d.title,
                d.effective_date, d.expiry_date, d.is_confidential, d.created_at,
                COUNT(v.id) as version_count
            FROM legal_documents d
            LEFT JOIN legal_document_versions v ON v.document_id = d.id
            WHERE d.organization_id = $1
            AND ($2::uuid IS NULL OR d.building_id = $2)
            AND ($3::text IS NULL OR d.document_type = $3)
            AND ($4::boolean IS NULL OR d.is_confidential = $4)
            AND ($5::integer IS NULL OR d.expiry_date <= CURRENT_DATE + $5::integer)
            AND ($6::text IS NULL OR $6 = ANY(d.tags))
            AND ($7::text IS NULL OR d.title ILIKE $7 OR d.description ILIKE $7)
            GROUP BY d.id
            ORDER BY d.created_at DESC
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(&query.document_type)
        .bind(query.is_confidential)
        .bind(query.expiring_days)
        .bind(&query.tag)
        .bind(&search_pattern)
        .bind(query.limit.unwrap_or(50))
        .bind(query.offset.unwrap_or(0))
        .fetch_all(executor)
        .await
    }

    /// Update a legal document, scoped to an organization.
    ///
    /// Returns `None` when no row in `org_id` matches `id` (→ HTTP 404),
    /// so a cross-tenant `id` cannot be mutated (IDOR, #829).
    pub async fn update_document<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        data: UpdateLegalDocument,
    ) -> Result<Option<LegalDocument>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE legal_documents SET
                building_id = COALESCE($3, building_id),
                document_type = COALESCE($4, document_type),
                title = COALESCE($5, title),
                description = COALESCE($6, description),
                parties = COALESCE($7, parties),
                effective_date = COALESCE($8, effective_date),
                expiry_date = COALESCE($9, expiry_date),
                file_path = COALESCE($10, file_path),
                file_name = COALESCE($11, file_name),
                file_size = COALESCE($12, file_size),
                mime_type = COALESCE($13, mime_type),
                is_confidential = COALESCE($14, is_confidential),
                retention_period_months = COALESCE($15, retention_period_months),
                retention_expires_at = COALESCE($16, retention_expires_at),
                tags = COALESCE($17, tags),
                metadata = COALESCE($18, metadata),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(data.building_id)
        .bind(&data.document_type)
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.parties.map(sqlx::types::Json))
        .bind(data.effective_date)
        .bind(data.expiry_date)
        .bind(&data.file_path)
        .bind(&data.file_name)
        .bind(data.file_size)
        .bind(&data.mime_type)
        .bind(data.is_confidential)
        .bind(data.retention_period_months)
        .bind(data.retention_expires_at)
        .bind(&data.tags)
        .bind(data.metadata.map(sqlx::types::Json))
        .fetch_optional(executor)
        .await
    }

    /// Delete a legal document, scoped to an organization.
    pub async fn delete_document<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("DELETE FROM legal_documents WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(executor)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    // ==================== Document Versions ====================

    /// Check that a document belongs to an organization.
    ///
    /// Used to org-scope child-resource (version) operations so a foreign-org
    /// `document_id` cannot be read or written through (IDOR, #829).
    async fn document_in_org<'e, E>(
        &self,
        executor: E,
        document_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM legal_documents WHERE id = $1 AND organization_id = $2",
        )
        .bind(document_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await?;
        Ok(exists.is_some())
    }

    /// Add a new version to a document, scoped to an organization.
    ///
    /// Returns `None` when the parent document is not in `org_id` (→ HTTP 404).
    pub async fn add_document_version(
        &self,
        conn: &mut PgConnection,
        document_id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateLegalDocumentVersion,
    ) -> Result<Option<LegalDocumentVersion>, sqlx::Error> {
        if !self
            .document_in_org(&mut *conn, document_id, org_id)
            .await?
        {
            return Ok(None);
        }
        let version: LegalDocumentVersion = sqlx::query_as(
            r#"
            INSERT INTO legal_document_versions
                (document_id, version_number, file_path, file_name, file_size, mime_type,
                 change_notes, created_by)
            VALUES (
                $1,
                (SELECT COALESCE(MAX(version_number), 0) + 1 FROM legal_document_versions WHERE document_id = $1),
                $2, $3, $4, $5, $6, $7
            )
            RETURNING *
            "#,
        )
        .bind(document_id)
        .bind(&data.file_path)
        .bind(&data.file_name)
        .bind(data.file_size)
        .bind(&data.mime_type)
        .bind(&data.change_notes)
        .bind(user_id)
        .fetch_one(&mut *conn)
        .await?;
        Ok(Some(version))
    }

    /// List versions for a document, scoped to an organization.
    ///
    /// Returns `None` when the parent document is not in `org_id` (→ HTTP 404).
    pub async fn list_document_versions(
        &self,
        conn: &mut PgConnection,
        document_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Vec<LegalDocumentVersion>>, sqlx::Error> {
        if !self
            .document_in_org(&mut *conn, document_id, org_id)
            .await?
        {
            return Ok(None);
        }
        let versions = sqlx::query_as(
            r#"
            SELECT * FROM legal_document_versions
            WHERE document_id = $1
            ORDER BY version_number DESC
            "#,
        )
        .bind(document_id)
        .fetch_all(&mut *conn)
        .await?;
        Ok(Some(versions))
    }

    /// Get a specific version, scoped to an organization.
    pub async fn get_document_version(
        &self,
        conn: &mut PgConnection,
        document_id: Uuid,
        org_id: Uuid,
        version_number: i32,
    ) -> Result<Option<LegalDocumentVersion>, sqlx::Error> {
        if !self
            .document_in_org(&mut *conn, document_id, org_id)
            .await?
        {
            return Ok(None);
        }
        sqlx::query_as(
            r#"
            SELECT * FROM legal_document_versions
            WHERE document_id = $1 AND version_number = $2
            "#,
        )
        .bind(document_id)
        .bind(version_number)
        .fetch_optional(&mut *conn)
        .await
    }

    // ==================== Compliance Requirements CRUD ====================

    /// Create a compliance requirement.
    pub async fn create_requirement<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        data: CreateComplianceRequirement,
    ) -> Result<ComplianceRequirement, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO compliance_requirements
                (organization_id, building_id, name, description, category, regulation_reference,
                 frequency, next_due_date, is_mandatory, responsible_party, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.category)
        .bind(&data.regulation_reference)
        .bind(data.frequency.unwrap_or_else(|| "annually".to_string()))
        .bind(data.next_due_date)
        .bind(data.is_mandatory.unwrap_or(true))
        .bind(&data.responsible_party)
        .bind(&data.notes)
        .fetch_one(executor)
        .await
    }

    /// Find a compliance requirement by ID, scoped to an organization.
    ///
    /// A foreign-org `id` returns `None` (→ HTTP 404) (IDOR, #829).
    pub async fn find_requirement_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<ComplianceRequirement>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            "SELECT * FROM compliance_requirements WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// List compliance requirements.
    pub async fn list_requirements<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: ComplianceQuery,
    ) -> Result<Vec<ComplianceRequirement>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM compliance_requirements
            WHERE organization_id = $1
            AND ($2::uuid IS NULL OR building_id = $2)
            AND ($3::text IS NULL OR category = $3)
            AND ($4::text IS NULL OR status = $4)
            AND ($5::boolean IS NULL OR is_mandatory = $5)
            AND ($6::date IS NULL OR next_due_date <= $6)
            AND ($7::boolean IS NOT TRUE OR (next_due_date < CURRENT_DATE AND status = 'pending'))
            ORDER BY next_due_date ASC NULLS LAST, name ASC
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(&query.category)
        .bind(&query.status)
        .bind(query.is_mandatory)
        .bind(query.due_before)
        .bind(query.overdue_only)
        .bind(query.limit.unwrap_or(50))
        .bind(query.offset.unwrap_or(0))
        .fetch_all(executor)
        .await
    }

    /// List requirements with verification details.
    pub async fn list_requirements_with_details<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: ComplianceQuery,
    ) -> Result<Vec<ComplianceRequirementWithDetails>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                r.id, r.organization_id, r.building_id, r.name, r.description, r.category,
                r.frequency, r.status, r.is_mandatory, r.next_due_date, r.last_verified_at,
                COUNT(v.id) as verification_count
            FROM compliance_requirements r
            LEFT JOIN compliance_verifications v ON v.requirement_id = r.id
            WHERE r.organization_id = $1
            AND ($2::uuid IS NULL OR r.building_id = $2)
            AND ($3::text IS NULL OR r.category = $3)
            AND ($4::text IS NULL OR r.status = $4)
            AND ($5::boolean IS NULL OR r.is_mandatory = $5)
            AND ($6::date IS NULL OR r.next_due_date <= $6)
            AND ($7::boolean IS NOT TRUE OR (r.next_due_date < CURRENT_DATE AND r.status = 'pending'))
            GROUP BY r.id
            ORDER BY r.next_due_date ASC NULLS LAST, r.name ASC
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(&query.category)
        .bind(&query.status)
        .bind(query.is_mandatory)
        .bind(query.due_before)
        .bind(query.overdue_only)
        .bind(query.limit.unwrap_or(50))
        .bind(query.offset.unwrap_or(0))
        .fetch_all(executor)
        .await
    }

    /// Update a compliance requirement, scoped to an organization.
    ///
    /// Returns `None` when no row in `org_id` matches `id` (→ HTTP 404) (IDOR, #829).
    pub async fn update_requirement<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        data: UpdateComplianceRequirement,
    ) -> Result<Option<ComplianceRequirement>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE compliance_requirements SET
                building_id = COALESCE($3, building_id),
                name = COALESCE($4, name),
                description = COALESCE($5, description),
                category = COALESCE($6, category),
                regulation_reference = COALESCE($7, regulation_reference),
                frequency = COALESCE($8, frequency),
                next_due_date = COALESCE($9, next_due_date),
                status = COALESCE($10, status),
                is_mandatory = COALESCE($11, is_mandatory),
                responsible_party = COALESCE($12, responsible_party),
                notes = COALESCE($13, notes),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(data.building_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.category)
        .bind(&data.regulation_reference)
        .bind(&data.frequency)
        .bind(data.next_due_date)
        .bind(&data.status)
        .bind(data.is_mandatory)
        .bind(&data.responsible_party)
        .bind(&data.notes)
        .fetch_optional(executor)
        .await
    }

    /// Delete a compliance requirement, scoped to an organization.
    pub async fn delete_requirement<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            "DELETE FROM compliance_requirements WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    // ==================== Compliance Verifications ====================

    /// Check that a compliance requirement belongs to an organization.
    async fn requirement_in_org<'e, E>(
        &self,
        executor: E,
        requirement_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM compliance_requirements WHERE id = $1 AND organization_id = $2",
        )
        .bind(requirement_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await?;
        Ok(exists.is_some())
    }

    /// Record a compliance verification, scoped to an organization.
    ///
    /// Returns `None` when the requirement is not in `org_id` (→ HTTP 404).
    pub async fn create_verification(
        &self,
        conn: &mut PgConnection,
        requirement_id: Uuid,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateComplianceVerification,
    ) -> Result<Option<ComplianceVerification>, sqlx::Error> {
        if !self
            .requirement_in_org(&mut *conn, requirement_id, org_id)
            .await?
        {
            return Ok(None);
        }
        // Record the verification
        let verification: ComplianceVerification = sqlx::query_as(
            r#"
            INSERT INTO compliance_verifications
                (requirement_id, verified_by, status, notes, evidence_document_id,
                 inspector_name, certificate_number, valid_until, issues_found, corrective_actions)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(requirement_id)
        .bind(user_id)
        .bind(&data.status)
        .bind(&data.notes)
        .bind(data.evidence_document_id)
        .bind(&data.inspector_name)
        .bind(&data.certificate_number)
        .bind(data.valid_until)
        .bind(&data.issues_found)
        .bind(&data.corrective_actions)
        .fetch_one(&mut *conn)
        .await?;

        // Update the requirement status and verification dates
        sqlx::query(
            r#"
            UPDATE compliance_requirements SET
                status = $2,
                last_verified_at = NOW(),
                last_verified_by = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(requirement_id)
        .bind(&data.status)
        .bind(user_id)
        .execute(&mut *conn)
        .await?;

        Ok(Some(verification))
    }

    /// List verifications for a requirement, scoped to an organization.
    ///
    /// Returns `None` when the requirement is not in `org_id` (→ HTTP 404).
    pub async fn list_verifications(
        &self,
        conn: &mut PgConnection,
        requirement_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Vec<ComplianceVerification>>, sqlx::Error> {
        if !self
            .requirement_in_org(&mut *conn, requirement_id, org_id)
            .await?
        {
            return Ok(None);
        }
        let verifications = sqlx::query_as(
            r#"
            SELECT * FROM compliance_verifications
            WHERE requirement_id = $1
            ORDER BY verified_at DESC
            "#,
        )
        .bind(requirement_id)
        .fetch_all(&mut *conn)
        .await?;
        Ok(Some(verifications))
    }

    /// Get compliance statistics.
    pub async fn get_compliance_statistics(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
    ) -> Result<ComplianceStatistics, sqlx::Error> {
        let counts: (i64, i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE status = 'compliant') as compliant,
                COUNT(*) FILTER (WHERE status = 'non_compliant') as non_compliant,
                COUNT(*) FILTER (WHERE status = 'pending') as pending,
                COUNT(*) FILTER (WHERE next_due_date < CURRENT_DATE AND status = 'pending') as overdue
            FROM compliance_requirements
            WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(&mut *conn)
        .await?;

        let by_category: Vec<ComplianceCategoryCount> = sqlx::query_as(
            r#"
            SELECT category, COUNT(*) as count
            FROM compliance_requirements
            WHERE organization_id = $1
            GROUP BY category
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&mut *conn)
        .await?;

        let upcoming_verifications: Vec<UpcomingVerification> = sqlx::query_as(
            r#"
            SELECT
                id, name, category, next_due_date, building_id,
                (next_due_date - CURRENT_DATE)::integer as days_until_due
            FROM compliance_requirements
            WHERE organization_id = $1
            AND next_due_date IS NOT NULL
            AND next_due_date >= CURRENT_DATE
            ORDER BY next_due_date ASC
            LIMIT 10
            "#,
        )
        .bind(org_id)
        .fetch_all(&mut *conn)
        .await?;

        Ok(ComplianceStatistics {
            total_requirements: counts.0,
            compliant_count: counts.1,
            non_compliant_count: counts.2,
            pending_count: counts.3,
            overdue_count: counts.4,
            by_category,
            upcoming_verifications,
        })
    }

    // ==================== Legal Notices ====================

    /// Create a legal notice.
    pub async fn create_notice(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateLegalNotice,
    ) -> Result<LegalNotice, sqlx::Error> {
        let notice: LegalNotice = sqlx::query_as(
            r#"
            INSERT INTO legal_notices
                (organization_id, building_id, notice_type, subject, content, priority,
                 delivery_method, requires_acknowledgment, acknowledgment_deadline, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(&data.notice_type)
        .bind(&data.subject)
        .bind(&data.content)
        .bind(data.priority.unwrap_or_else(|| "normal".to_string()))
        .bind(data.delivery_method.unwrap_or_else(|| "email".to_string()))
        .bind(data.requires_acknowledgment.unwrap_or(false))
        .bind(data.acknowledgment_deadline)
        .bind(user_id)
        .fetch_one(&mut *conn)
        .await?;

        // Add recipients
        for recipient in data.recipient_ids {
            sqlx::query(
                r#"
                INSERT INTO legal_notice_recipients
                    (notice_id, recipient_type, recipient_id, recipient_name, recipient_email)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(notice.id)
            .bind(&recipient.recipient_type)
            .bind(recipient.recipient_id)
            .bind(&recipient.recipient_name)
            .bind(&recipient.recipient_email)
            .execute(&mut *conn)
            .await?;
        }

        Ok(notice)
    }

    /// Find a legal notice by ID, scoped to an organization.
    ///
    /// A foreign-org `id` returns `None` (→ HTTP 404) (IDOR, #829).
    pub async fn find_notice_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<LegalNotice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as("SELECT * FROM legal_notices WHERE id = $1 AND organization_id = $2")
            .bind(id)
            .bind(org_id)
            .fetch_optional(executor)
            .await
    }

    /// Check that a legal notice belongs to an organization.
    async fn notice_in_org<'e, E>(
        &self,
        executor: E,
        notice_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let exists: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM legal_notices WHERE id = $1 AND organization_id = $2",
        )
        .bind(notice_id)
        .bind(org_id)
        .fetch_optional(executor)
        .await?;
        Ok(exists.is_some())
    }

    /// List legal notices.
    pub async fn list_notices<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: LegalNoticeQuery,
    ) -> Result<Vec<LegalNotice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM legal_notices
            WHERE organization_id = $1
            AND ($2::uuid IS NULL OR building_id = $2)
            AND ($3::text IS NULL OR notice_type = $3)
            AND ($4::text IS NULL OR priority = $4)
            AND ($5::boolean IS NULL OR (sent_at IS NOT NULL) = $5)
            AND ($6::boolean IS NULL OR requires_acknowledgment = $6)
            ORDER BY created_at DESC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(&query.notice_type)
        .bind(&query.priority)
        .bind(query.sent)
        .bind(query.requires_acknowledgment)
        .bind(query.limit.unwrap_or(50))
        .bind(query.offset.unwrap_or(0))
        .fetch_all(executor)
        .await
    }

    /// List notices with recipient summary.
    pub async fn list_notices_with_recipients<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: LegalNoticeQuery,
    ) -> Result<Vec<NoticeWithRecipients>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT
                n.id, n.organization_id, n.building_id, n.notice_type, n.subject, n.priority,
                n.sent_at, n.requires_acknowledgment,
                COUNT(r.id) as total_recipients,
                COUNT(r.id) FILTER (WHERE r.delivery_status = 'delivered') as delivered_count,
                COUNT(r.id) FILTER (WHERE r.acknowledged_at IS NOT NULL) as acknowledged_count
            FROM legal_notices n
            LEFT JOIN legal_notice_recipients r ON r.notice_id = n.id
            WHERE n.organization_id = $1
            AND ($2::uuid IS NULL OR n.building_id = $2)
            AND ($3::text IS NULL OR n.notice_type = $3)
            AND ($4::text IS NULL OR n.priority = $4)
            AND ($5::boolean IS NULL OR (n.sent_at IS NOT NULL) = $5)
            AND ($6::boolean IS NULL OR n.requires_acknowledgment = $6)
            GROUP BY n.id
            ORDER BY n.created_at DESC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(&query.notice_type)
        .bind(&query.priority)
        .bind(query.sent)
        .bind(query.requires_acknowledgment)
        .bind(query.limit.unwrap_or(50))
        .bind(query.offset.unwrap_or(0))
        .fetch_all(executor)
        .await
    }

    /// Update a legal notice, scoped to an organization.
    ///
    /// Returns `None` when no row in `org_id` matches `id` (→ HTTP 404) (IDOR, #829).
    pub async fn update_notice<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        data: UpdateLegalNotice,
    ) -> Result<Option<LegalNotice>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE legal_notices SET
                subject = COALESCE($3, subject),
                content = COALESCE($4, content),
                priority = COALESCE($5, priority),
                delivery_method = COALESCE($6, delivery_method),
                requires_acknowledgment = COALESCE($7, requires_acknowledgment),
                acknowledgment_deadline = COALESCE($8, acknowledgment_deadline),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&data.subject)
        .bind(&data.content)
        .bind(&data.priority)
        .bind(&data.delivery_method)
        .bind(data.requires_acknowledgment)
        .bind(data.acknowledgment_deadline)
        .fetch_optional(executor)
        .await
    }

    /// Delete a legal notice, scoped to an organization.
    pub async fn delete_notice<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("DELETE FROM legal_notices WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(org_id)
                .execute(executor)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Send a notice (mark as sent and update delivery status).
    ///
    /// Note: This is a synchronous database operation that marks all recipients as 'sent'.
    /// In production, actual email/mail delivery should be handled by a background job
    /// that can retry failures and update individual recipient statuses accordingly.
    pub async fn send_notice(
        &self,
        conn: &mut PgConnection,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<LegalNotice>, sqlx::Error> {
        // Update notice sent_at (org-scoped — foreign-org notice → None → 404).
        let notice: Option<LegalNotice> = sqlx::query_as(
            r#"
            UPDATE legal_notices SET sent_at = NOW(), updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&mut *conn)
        .await?;

        let Some(notice) = notice else {
            return Ok(None);
        };

        // Update recipients status to sent.
        // In a real system, this should be updated asynchronously after actual delivery.
        sqlx::query(
            r#"
            UPDATE legal_notice_recipients SET
                delivery_status = 'sent',
                delivered_at = NOW()
            WHERE notice_id = $1
            "#,
        )
        .bind(id)
        .execute(&mut *conn)
        .await?;

        Ok(Some(notice))
    }

    // ==================== Notice Recipients ====================

    /// List recipients for a notice, scoped to an organization.
    ///
    /// Returns `None` when the parent notice is not in `org_id` (→ HTTP 404).
    pub async fn list_notice_recipients(
        &self,
        conn: &mut PgConnection,
        notice_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Vec<LegalNoticeRecipient>>, sqlx::Error> {
        if !self.notice_in_org(&mut *conn, notice_id, org_id).await? {
            return Ok(None);
        }
        let recipients = sqlx::query_as(
            r#"
            SELECT * FROM legal_notice_recipients
            WHERE notice_id = $1
            ORDER BY recipient_name ASC
            "#,
        )
        .bind(notice_id)
        .fetch_all(&mut *conn)
        .await?;
        Ok(Some(recipients))
    }

    /// Acknowledge a notice by recipient record ID, scoped to an organization.
    /// Uses the unique (notice_id, recipient_id) constraint to identify the recipient.
    ///
    /// Returns `None` when the parent notice is not in `org_id` (→ HTTP 404).
    pub async fn acknowledge_notice(
        &self,
        conn: &mut PgConnection,
        notice_id: Uuid,
        recipient_id: Uuid,
        org_id: Uuid,
        data: AcknowledgeNotice,
    ) -> Result<Option<LegalNoticeRecipient>, sqlx::Error> {
        if !self.notice_in_org(&mut *conn, notice_id, org_id).await? {
            return Ok(None);
        }
        // The unique constraint on (notice_id, recipient_id) ensures this update
        // will only affect exactly one row.
        sqlx::query_as(
            r#"
            UPDATE legal_notice_recipients SET
                acknowledged_at = NOW(),
                acknowledgment_method = $3
            WHERE notice_id = $1 AND recipient_id = $2
            RETURNING *
            "#,
        )
        .bind(notice_id)
        .bind(recipient_id)
        .bind(
            data.acknowledgment_method
                .unwrap_or_else(|| "manual".to_string()),
        )
        .fetch_optional(&mut *conn)
        .await
    }

    /// Get notice statistics.
    pub async fn get_notice_statistics(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
    ) -> Result<NoticeStatistics, sqlx::Error> {
        let counts: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE sent_at IS NOT NULL) as sent,
                COUNT(*) FILTER (WHERE sent_at IS NULL) as pending
            FROM legal_notices
            WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(&mut *conn)
        .await?;

        let by_type: Vec<NoticeTypeCount> = sqlx::query_as(
            r#"
            SELECT notice_type, COUNT(*) as count
            FROM legal_notices
            WHERE organization_id = $1
            GROUP BY notice_type
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&mut *conn)
        .await?;

        // Count at recipient level for consistency (all counts are recipient-based)
        let ack_counts: (i64, i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(r.id) FILTER (WHERE n.requires_acknowledgment = TRUE) as total_requiring,
                COUNT(r.id) FILTER (WHERE n.requires_acknowledgment = TRUE AND r.acknowledged_at IS NOT NULL) as acknowledged,
                COUNT(r.id) FILTER (WHERE n.requires_acknowledgment = TRUE AND r.acknowledged_at IS NULL) as pending,
                COUNT(r.id) FILTER (WHERE n.requires_acknowledgment = TRUE AND r.acknowledged_at IS NULL AND n.acknowledgment_deadline < NOW()) as overdue
            FROM legal_notices n
            INNER JOIN legal_notice_recipients r ON r.notice_id = n.id
            WHERE n.organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(&mut *conn)
        .await?;

        Ok(NoticeStatistics {
            total_notices: counts.0,
            sent_count: counts.1,
            pending_count: counts.2,
            by_type,
            acknowledgment_stats: NoticeAcknowledgmentStats {
                total_requiring: ack_counts.0,
                acknowledged: ack_counts.1,
                pending: ack_counts.2,
                overdue: ack_counts.3,
            },
        })
    }

    // ==================== Compliance Templates ====================

    /// Create a compliance template.
    pub async fn create_template<'e, E>(
        &self,
        executor: E,
        org_id: Option<Uuid>,
        data: CreateComplianceTemplate,
    ) -> Result<ComplianceTemplate, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO compliance_templates
                (organization_id, name, category, description, checklist_items, frequency)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(&data.name)
        .bind(&data.category)
        .bind(&data.description)
        .bind(data.checklist_items.map(sqlx::types::Json))
        .bind(data.frequency.unwrap_or_else(|| "annually".to_string()))
        .fetch_one(executor)
        .await
    }

    /// Find a template by ID, visible to an organization.
    ///
    /// Resolves the caller's own org templates and shared system templates
    /// (`organization_id IS NULL`); a *different* org's private template
    /// returns `None` (→ HTTP 404) (IDOR, #829). The RLS policy on
    /// `compliance_templates` mirrors this `org = current OR org IS NULL` rule,
    /// so shared templates remain visible under context.
    pub async fn find_template_by_id<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<ComplianceTemplate>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM compliance_templates
            WHERE id = $1 AND (organization_id = $2 OR organization_id IS NULL)
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(executor)
        .await
    }

    /// List templates (organization-specific + system templates).
    pub async fn list_templates<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        category: Option<String>,
    ) -> Result<Vec<ComplianceTemplate>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM compliance_templates
            WHERE (organization_id = $1 OR organization_id IS NULL)
            AND ($2::text IS NULL OR category = $2)
            ORDER BY is_system DESC, name ASC
            "#,
        )
        .bind(org_id)
        .bind(&category)
        .fetch_all(executor)
        .await
    }

    /// Update a template, scoped to an organization.
    ///
    /// Only the owning org's own (non-system) templates can be updated; a
    /// foreign-org or system template returns `None` (→ HTTP 404) (IDOR, #829).
    pub async fn update_template<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        data: UpdateComplianceTemplate,
    ) -> Result<Option<ComplianceTemplate>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            UPDATE compliance_templates SET
                name = COALESCE($3, name),
                category = COALESCE($4, category),
                description = COALESCE($5, description),
                checklist_items = COALESCE($6, checklist_items),
                frequency = COALESCE($7, frequency),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2 AND is_system = FALSE
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&data.name)
        .bind(&data.category)
        .bind(&data.description)
        .bind(data.checklist_items.map(sqlx::types::Json))
        .bind(&data.frequency)
        .fetch_optional(executor)
        .await
    }

    /// Delete a template, scoped to an organization.
    ///
    /// Only the owning org's own (non-system) templates can be deleted.
    pub async fn delete_template<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            "DELETE FROM compliance_templates WHERE id = $1 AND organization_id = $2 AND is_system = FALSE",
        )
        .bind(id)
        .bind(org_id)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Apply a template to create requirements.
    pub async fn apply_template(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        data: ApplyTemplate,
    ) -> Result<Vec<ComplianceRequirement>, sqlx::Error> {
        let template = self
            .find_template_by_id(&mut *conn, data.template_id, org_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        // Calculate next due date based on frequency
        let next_due_date = calculate_next_due_date(&template.frequency);

        let requirement: ComplianceRequirement = sqlx::query_as(
            r#"
            INSERT INTO compliance_requirements
                (organization_id, building_id, name, description, category, frequency,
                 next_due_date, is_mandatory)
            VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.building_id)
        .bind(&template.name)
        .bind(&template.description)
        .bind(&template.category)
        .bind(&template.frequency)
        .bind(next_due_date)
        .fetch_one(&mut *conn)
        .await?;

        Ok(vec![requirement])
    }

    // ==================== Compliance Audit Trail ====================

    /// Create an audit trail entry.
    pub async fn create_audit_entry<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateAuditTrailEntry,
    ) -> Result<ComplianceAuditTrail, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            INSERT INTO compliance_audit_trail
                (organization_id, requirement_id, document_id, notice_id, action, action_by,
                 old_values, new_values, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.requirement_id)
        .bind(data.document_id)
        .bind(data.notice_id)
        .bind(&data.action)
        .bind(user_id)
        .bind(data.old_values.map(sqlx::types::Json))
        .bind(data.new_values.map(sqlx::types::Json))
        .bind(&data.notes)
        .fetch_one(executor)
        .await
    }

    /// List audit trail entries.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_audit_trail<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        requirement_id: Option<Uuid>,
        document_id: Option<Uuid>,
        notice_id: Option<Uuid>,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ComplianceAuditTrail>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as(
            r#"
            SELECT * FROM compliance_audit_trail
            WHERE organization_id = $1
            AND ($2::uuid IS NULL OR requirement_id = $2)
            AND ($3::uuid IS NULL OR document_id = $3)
            AND ($4::uuid IS NULL OR notice_id = $4)
            ORDER BY action_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(org_id)
        .bind(requirement_id)
        .bind(document_id)
        .bind(notice_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }
}

/// Calculate next due date based on frequency using proper month arithmetic.
fn calculate_next_due_date(frequency: &str) -> Option<NaiveDate> {
    let today = chrono::Utc::now().date_naive();
    match frequency {
        "once" | "as_needed" => None,
        "monthly" => today.checked_add_months(Months::new(1)),
        "quarterly" => today.checked_add_months(Months::new(3)),
        "semi_annually" => today.checked_add_months(Months::new(6)),
        "annually" => today.checked_add_months(Months::new(12)),
        "biennially" => today.checked_add_months(Months::new(24)),
        // Default to one year if frequency is unrecognized.
        _ => today.checked_add_months(Months::new(12)),
    }
}
