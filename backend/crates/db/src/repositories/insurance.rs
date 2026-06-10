//! Insurance repository for Epic 22.
//!
//! Handles all database operations for insurance policies, claims, and reminders.
//!
//! # RLS Integration (PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on every insurance table (`insurance_policies`,
//! `insurance_claims`, `insurance_claim_documents`, `insurance_claim_history`,
//! `insurance_policy_documents`, `insurance_renewal_reminders`). Under `FORCE`
//! the api-server's owner connection is no longer exempt, so a query issued on a
//! connection without `app.current_org_id` set collapses to deny-all (own-org
//! reads return empty, writes fail).
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
//! pool**, so there is no way to issue a query that bypasses RLS. This mirrors
//! the `work_order.rs` / `budget.rs` precedent. The explicit `organization_id`
//! filters are retained as defence-in-depth alongside the RLS policy.

use sqlx::{Executor, PgConnection, Postgres};
use uuid::Uuid;

use crate::models::{
    AddClaimDocument, AddPolicyDocument, ClaimStatusSummary, CreateInsuranceClaim,
    CreateInsurancePolicy, CreateRenewalReminder, ExpiringPolicy, InsuranceClaim,
    InsuranceClaimDocument, InsuranceClaimHistory, InsuranceClaimQuery, InsuranceClaimWithPolicy,
    InsurancePolicy, InsurancePolicyDocument, InsurancePolicyQuery, InsuranceRenewalReminder,
    InsuranceStatistics, PolicyTypeSummary, UpdateInsuranceClaim, UpdateInsurancePolicy,
    UpdateRenewalReminder,
};
use crate::DbPool;

/// Repository for insurance management operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Clone)]
pub struct InsuranceRepository;

impl InsuranceRepository {
    /// Create a new InsuranceRepository.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: DbPool) -> Self {
        Self
    }

    // ============================================
    // Insurance Policy Operations
    // ============================================

    /// Create a new insurance policy.
    pub async fn create_policy<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        data: CreateInsurancePolicy,
    ) -> Result<InsurancePolicy, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsurancePolicy>(
            r#"
            INSERT INTO insurance_policies (
                organization_id, policy_number, policy_name, provider_name,
                provider_contact, provider_phone, provider_email, policy_type,
                coverage_amount, deductible, premium_amount, premium_frequency,
                building_id, unit_id, coverage_description, effective_date,
                expiration_date, renewal_date, auto_renew, terms, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(&data.policy_number)
        .bind(&data.policy_name)
        .bind(&data.provider_name)
        .bind(&data.provider_contact)
        .bind(&data.provider_phone)
        .bind(&data.provider_email)
        .bind(&data.policy_type)
        .bind(data.coverage_amount)
        .bind(data.deductible)
        .bind(data.premium_amount)
        .bind(&data.premium_frequency)
        .bind(data.building_id)
        .bind(data.unit_id)
        .bind(&data.coverage_description)
        .bind(data.effective_date)
        .bind(data.expiration_date)
        .bind(data.renewal_date)
        .bind(data.auto_renew)
        .bind(&data.terms)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Find insurance policy by ID.
    pub async fn find_policy_by_id<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        policy_id: Uuid,
    ) -> Result<Option<InsurancePolicy>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsurancePolicy>(
            "SELECT * FROM insurance_policies WHERE id = $1 AND organization_id = $2",
        )
        .bind(policy_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
    }

    /// List insurance policies with filters.
    pub async fn list_policies<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        query: InsurancePolicyQuery,
    ) -> Result<Vec<InsurancePolicy>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, InsurancePolicy>(
            r#"
            SELECT * FROM insurance_policies
            WHERE organization_id = $1
              AND ($2::VARCHAR IS NULL OR policy_type = $2)
              AND ($3::VARCHAR IS NULL OR status = $3)
              AND ($4::UUID IS NULL OR building_id = $4)
              AND ($5::UUID IS NULL OR unit_id = $5)
              AND ($6::VARCHAR IS NULL OR provider_name ILIKE '%' || $6 || '%')
              AND ($7::INTEGER IS NULL OR expiration_date <= CURRENT_DATE + $7)
            ORDER BY expiration_date ASC, created_at DESC
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(organization_id)
        .bind(&query.policy_type)
        .bind(&query.status)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(&query.provider_name)
        .bind(query.expiring_within_days)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update an insurance policy.
    pub async fn update_policy<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        policy_id: Uuid,
        data: UpdateInsurancePolicy,
    ) -> Result<Option<InsurancePolicy>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsurancePolicy>(
            r#"
            UPDATE insurance_policies SET
                policy_number = COALESCE($3, policy_number),
                policy_name = COALESCE($4, policy_name),
                provider_name = COALESCE($5, provider_name),
                provider_contact = COALESCE($6, provider_contact),
                provider_phone = COALESCE($7, provider_phone),
                provider_email = COALESCE($8, provider_email),
                policy_type = COALESCE($9, policy_type),
                coverage_amount = COALESCE($10, coverage_amount),
                deductible = COALESCE($11, deductible),
                premium_amount = COALESCE($12, premium_amount),
                premium_frequency = COALESCE($13, premium_frequency),
                building_id = COALESCE($14, building_id),
                unit_id = COALESCE($15, unit_id),
                coverage_description = COALESCE($16, coverage_description),
                effective_date = COALESCE($17, effective_date),
                expiration_date = COALESCE($18, expiration_date),
                renewal_date = COALESCE($19, renewal_date),
                status = COALESCE($20, status),
                auto_renew = COALESCE($21, auto_renew),
                terms = COALESCE($22, terms),
                metadata = COALESCE($23, metadata),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(policy_id)
        .bind(organization_id)
        .bind(&data.policy_number)
        .bind(&data.policy_name)
        .bind(&data.provider_name)
        .bind(&data.provider_contact)
        .bind(&data.provider_phone)
        .bind(&data.provider_email)
        .bind(&data.policy_type)
        .bind(data.coverage_amount)
        .bind(data.deductible)
        .bind(data.premium_amount)
        .bind(&data.premium_frequency)
        .bind(data.building_id)
        .bind(data.unit_id)
        .bind(&data.coverage_description)
        .bind(data.effective_date)
        .bind(data.expiration_date)
        .bind(data.renewal_date)
        .bind(&data.status)
        .bind(data.auto_renew)
        .bind(&data.terms)
        .bind(&data.metadata)
        .fetch_optional(executor)
        .await
    }

    /// Delete an insurance policy.
    pub async fn delete_policy<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        policy_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("DELETE FROM insurance_policies WHERE id = $1 AND organization_id = $2")
                .bind(policy_id)
                .bind(organization_id)
                .execute(executor)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get expiring policies.
    pub async fn get_expiring_policies<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        days_ahead: i32,
    ) -> Result<Vec<ExpiringPolicy>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, ExpiringPolicy>(
            r#"
            SELECT
                id AS policy_id,
                policy_number,
                policy_name,
                policy_type,
                provider_name,
                expiration_date,
                (expiration_date - CURRENT_DATE)::INTEGER AS days_until_expiry,
                coverage_amount,
                auto_renew
            FROM insurance_policies
            WHERE organization_id = $1
              AND status = 'active'
              AND expiration_date <= CURRENT_DATE + $2
            ORDER BY expiration_date
            "#,
        )
        .bind(organization_id)
        .bind(days_ahead)
        .fetch_all(executor)
        .await
    }

    // ============================================
    // Policy Document Operations
    // ============================================

    /// Add document to policy.
    pub async fn add_policy_document<'e, E>(
        &self,
        executor: E,
        policy_id: Uuid,
        data: AddPolicyDocument,
    ) -> Result<InsurancePolicyDocument, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsurancePolicyDocument>(
            r#"
            INSERT INTO insurance_policy_documents (policy_id, document_id, document_type)
            VALUES ($1, $2, COALESCE($3, 'policy'))
            RETURNING *
            "#,
        )
        .bind(policy_id)
        .bind(data.document_id)
        .bind(&data.document_type)
        .fetch_one(executor)
        .await
    }

    /// List policy documents.
    pub async fn list_policy_documents<'e, E>(
        &self,
        executor: E,
        policy_id: Uuid,
    ) -> Result<Vec<InsurancePolicyDocument>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsurancePolicyDocument>(
            "SELECT * FROM insurance_policy_documents WHERE policy_id = $1 ORDER BY created_at DESC",
        )
        .bind(policy_id)
        .fetch_all(executor)
        .await
    }

    /// Remove document from policy.
    pub async fn remove_policy_document<'e, E>(
        &self,
        executor: E,
        policy_id: Uuid,
        document_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            "DELETE FROM insurance_policy_documents WHERE policy_id = $1 AND document_id = $2",
        )
        .bind(policy_id)
        .bind(document_id)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ============================================
    // Insurance Claim Operations
    // ============================================

    /// Create a new insurance claim.
    pub async fn create_claim<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        submitted_by: Uuid,
        data: CreateInsuranceClaim,
    ) -> Result<InsuranceClaim, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceClaim>(
            r#"
            INSERT INTO insurance_claims (
                organization_id, policy_id, claim_number, incident_date,
                incident_description, incident_location, building_id, unit_id,
                fault_id, claimed_amount, currency, submitted_by, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, COALESCE($11, 'EUR'), $12, $13)
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(data.policy_id)
        .bind(&data.claim_number)
        .bind(data.incident_date)
        .bind(&data.incident_description)
        .bind(&data.incident_location)
        .bind(data.building_id)
        .bind(data.unit_id)
        .bind(data.fault_id)
        .bind(data.claimed_amount)
        .bind(&data.currency)
        .bind(submitted_by)
        .bind(&data.metadata)
        .fetch_one(executor)
        .await
    }

    /// Find insurance claim by ID.
    pub async fn find_claim_by_id<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        claim_id: Uuid,
    ) -> Result<Option<InsuranceClaim>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceClaim>(
            "SELECT * FROM insurance_claims WHERE id = $1 AND organization_id = $2",
        )
        .bind(claim_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
    }

    /// Find insurance claim with policy details.
    pub async fn find_claim_with_policy<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        claim_id: Uuid,
    ) -> Result<Option<InsuranceClaimWithPolicy>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceClaimWithPolicy>(
            r#"
            SELECT
                c.*,
                p.policy_number,
                p.policy_name,
                p.policy_type,
                p.provider_name
            FROM insurance_claims c
            JOIN insurance_policies p ON c.policy_id = p.id
            WHERE c.id = $1 AND c.organization_id = $2
            "#,
        )
        .bind(claim_id)
        .bind(organization_id)
        .fetch_optional(executor)
        .await
    }

    /// List insurance claims with filters.
    pub async fn list_claims<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        query: InsuranceClaimQuery,
    ) -> Result<Vec<InsuranceClaimWithPolicy>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, InsuranceClaimWithPolicy>(
            r#"
            SELECT
                c.*,
                p.policy_number,
                p.policy_name,
                p.policy_type,
                p.provider_name
            FROM insurance_claims c
            JOIN insurance_policies p ON c.policy_id = p.id
            WHERE c.organization_id = $1
              AND ($2::UUID IS NULL OR c.policy_id = $2)
              AND ($3::VARCHAR IS NULL OR c.status = $3)
              AND ($4::UUID IS NULL OR c.building_id = $4)
              AND ($5::UUID IS NULL OR c.unit_id = $5)
              AND ($6::DATE IS NULL OR c.incident_date >= $6)
              AND ($7::DATE IS NULL OR c.incident_date <= $7)
            ORDER BY c.created_at DESC
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(organization_id)
        .bind(query.policy_id)
        .bind(&query.status)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.incident_date_from)
        .bind(query.incident_date_to)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update an insurance claim.
    pub async fn update_claim<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        claim_id: Uuid,
        data: UpdateInsuranceClaim,
    ) -> Result<Option<InsuranceClaim>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceClaim>(
            r#"
            UPDATE insurance_claims SET
                claim_number = COALESCE($3, claim_number),
                provider_claim_number = COALESCE($4, provider_claim_number),
                incident_date = COALESCE($5, incident_date),
                incident_description = COALESCE($6, incident_description),
                incident_location = COALESCE($7, incident_location),
                building_id = COALESCE($8, building_id),
                unit_id = COALESCE($9, unit_id),
                fault_id = COALESCE($10, fault_id),
                claimed_amount = COALESCE($11, claimed_amount),
                approved_amount = COALESCE($12, approved_amount),
                deductible_applied = COALESCE($13, deductible_applied),
                status = COALESCE($14, status),
                adjuster_name = COALESCE($15, adjuster_name),
                adjuster_phone = COALESCE($16, adjuster_phone),
                adjuster_email = COALESCE($17, adjuster_email),
                resolution_notes = COALESCE($18, resolution_notes),
                denial_reason = COALESCE($19, denial_reason),
                metadata = COALESCE($20, metadata),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(claim_id)
        .bind(organization_id)
        .bind(&data.claim_number)
        .bind(&data.provider_claim_number)
        .bind(data.incident_date)
        .bind(&data.incident_description)
        .bind(&data.incident_location)
        .bind(data.building_id)
        .bind(data.unit_id)
        .bind(data.fault_id)
        .bind(data.claimed_amount)
        .bind(data.approved_amount)
        .bind(data.deductible_applied)
        .bind(&data.status)
        .bind(&data.adjuster_name)
        .bind(&data.adjuster_phone)
        .bind(&data.adjuster_email)
        .bind(&data.resolution_notes)
        .bind(&data.denial_reason)
        .bind(&data.metadata)
        .fetch_optional(executor)
        .await
    }

    /// Submit a claim.
    pub async fn submit_claim<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        claim_id: Uuid,
        submitted_by: Uuid,
    ) -> Result<Option<InsuranceClaim>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceClaim>(
            r#"
            UPDATE insurance_claims SET
                status = 'submitted',
                submitted_by = $3,
                submitted_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2 AND status = 'draft'
            RETURNING *
            "#,
        )
        .bind(claim_id)
        .bind(organization_id)
        .bind(submitted_by)
        .fetch_optional(executor)
        .await
    }

    /// Review a claim (approve, deny, etc.).
    /// Only claims with status: submitted, under_review, or information_requested can be reviewed.
    #[allow(clippy::too_many_arguments)]
    pub async fn review_claim<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        claim_id: Uuid,
        reviewed_by: Uuid,
        new_status: &str,
        approved_amount: Option<rust_decimal::Decimal>,
        denial_reason: Option<&str>,
        resolution_notes: Option<&str>,
    ) -> Result<Option<InsuranceClaim>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceClaim>(
            r#"
            UPDATE insurance_claims SET
                status = $3,
                reviewed_by = $4,
                reviewed_at = NOW(),
                approved_amount = COALESCE($5, approved_amount),
                denial_reason = COALESCE($6, denial_reason),
                resolution_notes = COALESCE($7, resolution_notes),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
              AND status IN ('submitted', 'under_review', 'information_requested')
            RETURNING *
            "#,
        )
        .bind(claim_id)
        .bind(organization_id)
        .bind(new_status)
        .bind(reviewed_by)
        .bind(approved_amount)
        .bind(denial_reason)
        .bind(resolution_notes)
        .fetch_optional(executor)
        .await
    }

    /// Record payment for a claim.
    /// Only claims with status: approved or partially_approved can receive payments.
    pub async fn record_claim_payment<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        claim_id: Uuid,
        payment_amount: rust_decimal::Decimal,
    ) -> Result<Option<InsuranceClaim>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceClaim>(
            r#"
            UPDATE insurance_claims SET
                paid_amount = COALESCE(paid_amount, 0) + $3,
                status = CASE
                    WHEN COALESCE(paid_amount, 0) + $3 >= COALESCE(approved_amount, 0) THEN 'paid'
                    ELSE status
                END,
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
              AND status IN ('approved', 'partially_approved')
            RETURNING *
            "#,
        )
        .bind(claim_id)
        .bind(organization_id)
        .bind(payment_amount)
        .fetch_optional(executor)
        .await
    }

    /// Delete an insurance claim.
    pub async fn delete_claim<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        claim_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            "DELETE FROM insurance_claims WHERE id = $1 AND organization_id = $2 AND status = 'draft'",
        )
        .bind(claim_id)
        .bind(organization_id)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get claim history.
    pub async fn get_claim_history<'e, E>(
        &self,
        executor: E,
        claim_id: Uuid,
    ) -> Result<Vec<InsuranceClaimHistory>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceClaimHistory>(
            "SELECT * FROM insurance_claim_history WHERE claim_id = $1 ORDER BY created_at DESC",
        )
        .bind(claim_id)
        .fetch_all(executor)
        .await
    }

    // ============================================
    // Claim Document Operations
    // ============================================

    /// Add document to claim.
    pub async fn add_claim_document<'e, E>(
        &self,
        executor: E,
        claim_id: Uuid,
        data: AddClaimDocument,
    ) -> Result<InsuranceClaimDocument, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceClaimDocument>(
            r#"
            INSERT INTO insurance_claim_documents (claim_id, document_id, document_type)
            VALUES ($1, $2, COALESCE($3, 'evidence'))
            RETURNING *
            "#,
        )
        .bind(claim_id)
        .bind(data.document_id)
        .bind(&data.document_type)
        .fetch_one(executor)
        .await
    }

    /// List claim documents.
    pub async fn list_claim_documents<'e, E>(
        &self,
        executor: E,
        claim_id: Uuid,
    ) -> Result<Vec<InsuranceClaimDocument>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceClaimDocument>(
            "SELECT * FROM insurance_claim_documents WHERE claim_id = $1 ORDER BY created_at DESC",
        )
        .bind(claim_id)
        .fetch_all(executor)
        .await
    }

    /// Remove document from claim.
    pub async fn remove_claim_document<'e, E>(
        &self,
        executor: E,
        claim_id: Uuid,
        document_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            "DELETE FROM insurance_claim_documents WHERE claim_id = $1 AND document_id = $2",
        )
        .bind(claim_id)
        .bind(document_id)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ============================================
    // Renewal Reminder Operations
    // ============================================

    /// Create renewal reminder.
    pub async fn create_reminder<'e, E>(
        &self,
        executor: E,
        policy_id: Uuid,
        data: CreateRenewalReminder,
    ) -> Result<InsuranceRenewalReminder, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceRenewalReminder>(
            r#"
            INSERT INTO insurance_renewal_reminders (policy_id, days_before_expiry, reminder_type)
            VALUES ($1, $2, COALESCE($3, 'email'))
            RETURNING *
            "#,
        )
        .bind(policy_id)
        .bind(data.days_before_expiry)
        .bind(&data.reminder_type)
        .fetch_one(executor)
        .await
    }

    /// List policy reminders.
    pub async fn list_policy_reminders<'e, E>(
        &self,
        executor: E,
        policy_id: Uuid,
    ) -> Result<Vec<InsuranceRenewalReminder>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceRenewalReminder>(
            "SELECT * FROM insurance_renewal_reminders WHERE policy_id = $1 ORDER BY days_before_expiry DESC",
        )
        .bind(policy_id)
        .fetch_all(executor)
        .await
    }

    /// Update renewal reminder.
    pub async fn update_reminder<'e, E>(
        &self,
        executor: E,
        reminder_id: Uuid,
        data: UpdateRenewalReminder,
    ) -> Result<Option<InsuranceRenewalReminder>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, InsuranceRenewalReminder>(
            r#"
            UPDATE insurance_renewal_reminders SET
                days_before_expiry = COALESCE($2, days_before_expiry),
                reminder_type = COALESCE($3, reminder_type),
                is_active = COALESCE($4, is_active),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(reminder_id)
        .bind(data.days_before_expiry)
        .bind(&data.reminder_type)
        .bind(data.is_active)
        .fetch_optional(executor)
        .await
    }

    /// Delete renewal reminder.
    pub async fn delete_reminder<'e, E>(
        &self,
        executor: E,
        reminder_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query("DELETE FROM insurance_renewal_reminders WHERE id = $1")
            .bind(reminder_id)
            .execute(executor)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Mark reminder as sent.
    pub async fn mark_reminder_sent<'e, E>(
        &self,
        executor: E,
        reminder_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result =
            sqlx::query("UPDATE insurance_renewal_reminders SET sent_at = NOW() WHERE id = $1")
                .bind(reminder_id)
                .execute(executor)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get pending reminders to send.
    ///
    /// Multi-statement (a join read, then a per-policy lookup), so it takes a
    /// connection and reborrows it for each query.
    pub async fn get_pending_reminders(
        &self,
        conn: &mut PgConnection,
        organization_id: Uuid,
    ) -> Result<Vec<(InsuranceRenewalReminder, InsurancePolicy)>, sqlx::Error> {
        // Get reminders that are due and haven't been sent
        let reminders = sqlx::query_as::<_, InsuranceRenewalReminder>(
            r#"
            SELECT r.*
            FROM insurance_renewal_reminders r
            JOIN insurance_policies p ON r.policy_id = p.id
            WHERE p.organization_id = $1
              AND p.status = 'active'
              AND r.is_active = TRUE
              AND r.sent_at IS NULL
              AND p.expiration_date - r.days_before_expiry <= CURRENT_DATE
            ORDER BY p.expiration_date
            "#,
        )
        .bind(organization_id)
        .fetch_all(&mut *conn)
        .await?;

        let mut results = Vec::new();
        for reminder in reminders {
            if let Some(policy) = self
                .find_policy_by_id(&mut *conn, organization_id, reminder.policy_id)
                .await?
            {
                results.push((reminder, policy));
            }
        }

        Ok(results)
    }

    // ============================================
    // Statistics and Reporting
    // ============================================

    /// Get insurance statistics for organization.
    ///
    /// Multi-statement (policy + claim aggregates), so it takes a connection and
    /// reborrows it for each query.
    pub async fn get_statistics(
        &self,
        conn: &mut PgConnection,
        organization_id: Uuid,
    ) -> Result<InsuranceStatistics, sqlx::Error> {
        let policy_stats = sqlx::query_as::<_, (i64, i64, i64, Option<rust_decimal::Decimal>, Option<rust_decimal::Decimal>)>(
            r#"
            SELECT
                COUNT(*) AS total_policies,
                COUNT(*) FILTER (WHERE status = 'active') AS active_policies,
                COUNT(*) FILTER (WHERE status = 'active' AND expiration_date <= CURRENT_DATE + 30) AS expiring_soon,
                SUM(coverage_amount) FILTER (WHERE status = 'active') AS total_coverage,
                SUM(premium_amount) FILTER (WHERE status = 'active') AS total_premiums
            FROM insurance_policies
            WHERE organization_id = $1
            "#,
        )
        .bind(organization_id)
        .fetch_one(&mut *conn)
        .await?;

        let claim_stats = sqlx::query_as::<_, (i64, i64, Option<rust_decimal::Decimal>, Option<rust_decimal::Decimal>)>(
            r#"
            SELECT
                COUNT(*) AS total_claims,
                COUNT(*) FILTER (WHERE status NOT IN ('closed', 'withdrawn', 'denied', 'paid')) AS open_claims,
                SUM(claimed_amount) AS total_claimed,
                SUM(paid_amount) AS total_paid
            FROM insurance_claims
            WHERE organization_id = $1
            "#,
        )
        .bind(organization_id)
        .fetch_one(&mut *conn)
        .await?;

        Ok(InsuranceStatistics {
            total_policies: policy_stats.0,
            active_policies: policy_stats.1,
            expiring_soon: policy_stats.2,
            total_coverage: policy_stats.3,
            total_premiums: policy_stats.4,
            total_claims: claim_stats.0,
            open_claims: claim_stats.1,
            total_claimed: claim_stats.2,
            total_paid: claim_stats.3,
        })
    }

    /// Get claim summary by status.
    pub async fn get_claim_summary_by_status<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<Vec<ClaimStatusSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, ClaimStatusSummary>(
            r#"
            SELECT
                status,
                COUNT(*) AS count,
                SUM(claimed_amount) AS total_claimed,
                SUM(approved_amount) AS total_approved,
                SUM(paid_amount) AS total_paid
            FROM insurance_claims
            WHERE organization_id = $1
            GROUP BY status
            ORDER BY count DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(executor)
        .await
    }

    /// Get policy summary by type.
    pub async fn get_policy_summary_by_type<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
    ) -> Result<Vec<PolicyTypeSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, PolicyTypeSummary>(
            r#"
            SELECT
                policy_type,
                COUNT(*) AS policy_count,
                SUM(coverage_amount) AS total_coverage,
                SUM(premium_amount) AS total_premiums
            FROM insurance_policies
            WHERE organization_id = $1 AND status = 'active'
            GROUP BY policy_type
            ORDER BY policy_count DESC
            "#,
        )
        .bind(organization_id)
        .fetch_all(executor)
        .await
    }
}
