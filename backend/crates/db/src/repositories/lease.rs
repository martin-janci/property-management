//! Lease repository (Epic 19: Lease Management & Tenant Screening).
//!
//! # RLS Integration
//!
//! This repository supports two usage patterns:
//!
//! 1. **RLS-aware** (recommended): Use methods with `_rls` suffix that accept an executor
//!    with RLS context already set (e.g., from `RlsConnection`).
//!
//! 2. **Legacy**: Use methods without suffix that use the internal pool. These do NOT
//!    enforce RLS and should be migrated to the RLS-aware pattern.
//!
//! ## Example
//!
//! ```rust,ignore
//! async fn create_lease(
//!     mut rls: RlsConnection,
//!     State(state): State<AppState>,
//!     Json(data): Json<CreateLeaseRequest>,
//! ) -> Result<Json<Lease>> {
//!     let lease = state.lease_repo.create_lease_rls(rls.conn(), org_id, user_id, data).await?;
//!     rls.release().await;
//!     Ok(Json(lease))
//! }
//! ```

use crate::models::lease::{
    lease_status, screening_status, ApplicationListQuery, ApplicationSummary, CreateAmendment,
    CreateApplication, CreateLease, CreateLeaseTemplate, CreateReminder, ExpirationOverview,
    InitiateScreening, Lease, LeaseAmendment, LeaseListQuery, LeasePayment, LeaseReminder,
    LeaseStatistics, LeaseSummary, LeaseTemplate, LeaseWithDetails, PaymentSummary, RecordPayment,
    RenewLease, ReviewApplication, ScreeningConsent, ScreeningSummary, SubmitApplication,
    TenantApplication, TenantScreening, TerminateLease, UpdateApplication, UpdateLease,
    UpdateLeaseTemplate, UpdateScreeningResult,
};
use crate::DbPool;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, Executor, Postgres};
use uuid::Uuid;

/// Explicit `leases` column list for `query_as::<_, Lease>` reads.
///
/// `leases.status` (`lease_status`) and `leases.termination_reason`
/// (`termination_reason`) are Postgres ENUM columns, but the `Lease` model
/// decodes them as `String` — sqlx cannot decode a user-defined enum straight
/// into `String`, so a bare `SELECT *` fails at fetch with a `ColumnDecode`
/// ("mismatched types") error. Casting the enum columns to `text` follows the
/// established `status::text` precedent (#859, delegation `scopes::text[]`
/// in PR #1972).
const LEASE_COLUMNS: &str = "id, organization_id, unit_id, application_id, template_id, \
     landlord_user_id, landlord_name, landlord_address, \
     tenant_user_id, tenant_name, tenant_email, tenant_phone, occupants, \
     start_date, end_date, term_months, is_fixed_term, \
     monthly_rent, security_deposit, deposit_held_by, rent_due_day, \
     late_fee_amount, late_fee_grace_days, utilities_included, parking_spaces, storage_units, \
     pets_allowed, pet_deposit, max_occupants, smoking_allowed, \
     document_url, document_version, status::text AS status, \
     landlord_signed_at, tenant_signed_at, signature_request_id, \
     terminated_at, termination_reason::text AS termination_reason, termination_notes, \
     termination_initiated_by, previous_lease_id, renewed_to_lease_id, \
     renewal_offered_at, renewal_offer_expires_at, \
     notes, created_by, created_at, updated_at";

/// Canonical single-lease `SELECT` (`WHERE id = $1`) projecting
/// [`LEASE_COLUMNS`] so the `status` / `termination_reason` Postgres enums are
/// cast to `text`. Shared by `find_lease_by_id_rls` and
/// `get_lease_with_details_rls` so the projection can't drift back to a bare
/// `SELECT *` (regression #2169, where `get_lease_with_details_rls` still used
/// `SELECT *` and 500'd on GET /leases/{id}).
#[inline]
fn select_lease_by_id_sql() -> String {
    format!("SELECT {LEASE_COLUMNS} FROM leases WHERE id = $1")
}

/// Wrap a lease `INSERT`/`UPDATE` body with a `RETURNING` clause that projects
/// [`LEASE_COLUMNS`] (enum columns cast to `text`), never `RETURNING *`. Every
/// lease write method routes through this helper so a `RETURNING *` — which
/// yields native enum types and fails `Lease` decode with `ColumnDecode` — can't
/// be reintroduced.
#[inline]
fn lease_write_sql(body: &str) -> String {
    format!("{body}\nRETURNING {LEASE_COLUMNS}")
}

/// Repository for lease operations.
#[derive(Clone)]
pub struct LeaseRepository {
    pool: DbPool,
}

impl LeaseRepository {
    /// Create a new LeaseRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // RLS-aware methods (recommended)
    // ========================================================================

    // ------------------------------------------------------------------------
    // Applications (Story 19.1) - RLS versions
    // ------------------------------------------------------------------------

    /// Create tenant application with RLS context.
    ///
    /// Use this method with an `RlsConnection` to ensure RLS policies are enforced.
    pub async fn create_application_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        data: CreateApplication,
    ) -> Result<TenantApplication, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let app = sqlx::query_as::<_, TenantApplication>(
            r#"
            INSERT INTO tenant_applications (
                organization_id, unit_id, applicant_name, applicant_email, applicant_phone,
                date_of_birth, national_id, current_address, current_landlord_name,
                current_landlord_phone, current_rent_amount, current_tenancy_start,
                employer_name, employer_phone, job_title, employment_start, monthly_income,
                desired_move_in, desired_lease_term_months, proposed_rent, co_applicants,
                source, referral_code, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, 'draft')
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(data.unit_id)
        .bind(&data.applicant_name)
        .bind(&data.applicant_email)
        .bind(&data.applicant_phone)
        .bind(data.date_of_birth)
        .bind(&data.national_id)
        .bind(&data.current_address)
        .bind(&data.current_landlord_name)
        .bind(&data.current_landlord_phone)
        .bind(data.current_rent_amount)
        .bind(data.current_tenancy_start)
        .bind(&data.employer_name)
        .bind(&data.employer_phone)
        .bind(&data.job_title)
        .bind(data.employment_start)
        .bind(data.monthly_income)
        .bind(data.desired_move_in)
        .bind(data.desired_lease_term_months)
        .bind(data.proposed_rent)
        .bind(&data.co_applicants)
        .bind(&data.source)
        .bind(&data.referral_code)
        .fetch_one(executor)
        .await?;

        Ok(app)
    }

    /// Find application by ID with RLS context.
    pub async fn find_application_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<TenantApplication>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let app = sqlx::query_as::<_, TenantApplication>(
            r#"SELECT * FROM tenant_applications WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(app)
    }

    /// Update application with RLS context.
    pub async fn update_application_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateApplication,
    ) -> Result<TenantApplication, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let app = sqlx::query_as::<_, TenantApplication>(
            r#"
            UPDATE tenant_applications SET
                applicant_name = COALESCE($2, applicant_name),
                applicant_email = COALESCE($3, applicant_email),
                applicant_phone = COALESCE($4, applicant_phone),
                date_of_birth = COALESCE($5, date_of_birth),
                national_id = COALESCE($6, national_id),
                current_address = COALESCE($7, current_address),
                current_landlord_name = COALESCE($8, current_landlord_name),
                current_landlord_phone = COALESCE($9, current_landlord_phone),
                current_rent_amount = COALESCE($10, current_rent_amount),
                current_tenancy_start = COALESCE($11, current_tenancy_start),
                employer_name = COALESCE($12, employer_name),
                employer_phone = COALESCE($13, employer_phone),
                job_title = COALESCE($14, job_title),
                employment_start = COALESCE($15, employment_start),
                monthly_income = COALESCE($16, monthly_income),
                desired_move_in = COALESCE($17, desired_move_in),
                desired_lease_term_months = COALESCE($18, desired_lease_term_months),
                proposed_rent = COALESCE($19, proposed_rent),
                co_applicants = COALESCE($20, co_applicants),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.applicant_name)
        .bind(&data.applicant_email)
        .bind(&data.applicant_phone)
        .bind(data.date_of_birth)
        .bind(&data.national_id)
        .bind(&data.current_address)
        .bind(&data.current_landlord_name)
        .bind(&data.current_landlord_phone)
        .bind(data.current_rent_amount)
        .bind(data.current_tenancy_start)
        .bind(&data.employer_name)
        .bind(&data.employer_phone)
        .bind(&data.job_title)
        .bind(data.employment_start)
        .bind(data.monthly_income)
        .bind(data.desired_move_in)
        .bind(data.desired_lease_term_months)
        .bind(data.proposed_rent)
        .bind(&data.co_applicants)
        .fetch_one(executor)
        .await?;

        Ok(app)
    }

    /// Submit application with RLS context.
    pub async fn submit_application_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: SubmitApplication,
    ) -> Result<TenantApplication, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let app = sqlx::query_as::<_, TenantApplication>(
            r#"
            UPDATE tenant_applications SET
                status = 'submitted',
                documents = COALESCE($2, documents),
                submitted_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND status = 'draft'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.documents)
        .fetch_one(executor)
        .await?;

        Ok(app)
    }

    /// Review application with RLS context.
    pub async fn review_application_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        reviewer_id: Uuid,
        data: ReviewApplication,
    ) -> Result<TenantApplication, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let app = sqlx::query_as::<_, TenantApplication>(
            r#"
            UPDATE tenant_applications SET
                status = $2::tenant_application_status,
                reviewed_by = $3,
                reviewed_at = NOW(),
                decision_notes = $4,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.status)
        .bind(reviewer_id)
        .bind(&data.decision_notes)
        .fetch_one(executor)
        .await?;

        Ok(app)
    }

    /// Delete application with RLS context.
    pub async fn delete_application_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<bool, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(r#"DELETE FROM tenant_applications WHERE id = $1"#)
            .bind(id)
            .execute(executor)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List applications for organization with RLS context.
    pub async fn list_applications_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: ApplicationListQuery,
    ) -> Result<(Vec<ApplicationSummary>, i64), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(20);
        let offset = query.offset.unwrap_or(0);

        // Note: For RLS version, we need to use a single executor call.
        // We'll get count in a subquery or accept that we need two calls.
        // For simplicity, we'll return total as 0 and let the caller make a separate count call if needed.

        // Get applications with summaries
        let apps = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, String, String, Option<chrono::DateTime<Utc>>, Option<Decimal>, Option<NaiveDate>, i64, bool)>(
            r#"
            SELECT
                a.id, a.unit_id, u.designation, b.name,
                a.applicant_name, a.applicant_email, a.status::text,
                a.submitted_at, a.monthly_income, a.desired_move_in,
                (SELECT COUNT(*) FROM tenant_screenings WHERE application_id = a.id) as screening_count,
                COALESCE((SELECT bool_and(passed) FROM tenant_screenings WHERE application_id = a.id AND status = 'completed'), false) as screening_passed
            FROM tenant_applications a
            JOIN units u ON u.id = a.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE a.organization_id = $1
                AND ($2::uuid IS NULL OR a.unit_id = $2)
                AND ($3::text IS NULL OR a.status = $3::tenant_application_status)
            ORDER BY a.submitted_at DESC NULLS LAST, a.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(org_id)
        .bind(query.unit_id)
        .bind(&query.status)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await?;

        let summaries = apps
            .into_iter()
            .map(
                |(
                    id,
                    unit_id,
                    unit_name,
                    building_name,
                    applicant_name,
                    applicant_email,
                    status,
                    submitted_at,
                    monthly_income,
                    desired_move_in,
                    screening_count,
                    screening_passed,
                )| {
                    ApplicationSummary {
                        id,
                        unit_id,
                        unit_name,
                        building_name,
                        applicant_name,
                        applicant_email,
                        status,
                        submitted_at,
                        monthly_income,
                        desired_move_in,
                        screening_count,
                        screening_passed,
                    }
                },
            )
            .collect();

        // For RLS version, we return -1 to indicate count was not fetched
        // Caller should use count_applications_rls if total is needed
        Ok((summaries, -1))
    }

    /// Count applications for organization with RLS context.
    pub async fn count_applications_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: &ApplicationListQuery,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let (total,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM tenant_applications
            WHERE organization_id = $1
                AND ($2::uuid IS NULL OR unit_id = $2)
                AND ($3::text IS NULL OR status = $3::tenant_application_status)
            "#,
        )
        .bind(org_id)
        .bind(query.unit_id)
        .bind(&query.status)
        .fetch_one(executor)
        .await?;

        Ok(total)
    }

    // ------------------------------------------------------------------------
    // Screening (Story 19.2) - RLS versions
    // ------------------------------------------------------------------------

    /// Initiate screening for application with RLS context.
    pub async fn initiate_screening_rls<'e, E>(
        &self,
        executor: E,
        application_id: Uuid,
        org_id: Uuid,
        data: InitiateScreening,
    ) -> Result<Vec<TenantScreening>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Note: For RLS version, we execute a single batch insert
        // This is more efficient and uses only one executor call
        let screening_types_json = serde_json::to_value(&data.screening_types).unwrap_or_default();
        let provider = data.provider.as_deref();

        let screenings = sqlx::query_as::<_, TenantScreening>(
            r#"
            WITH inserted AS (
                INSERT INTO tenant_screenings (
                    application_id, organization_id, screening_type, provider, status, consent_requested_at
                )
                SELECT $1, $2, st::screening_type, $3, 'pending_consent', NOW()
                FROM jsonb_array_elements_text($4::jsonb) AS st
                RETURNING *
            ),
            app_update AS (
                UPDATE tenant_applications SET status = 'screening_pending' WHERE id = $1
            )
            SELECT * FROM inserted
            "#,
        )
        .bind(application_id)
        .bind(org_id)
        .bind(provider)
        .bind(&screening_types_json)
        .fetch_all(executor)
        .await?;

        Ok(screenings)
    }

    /// Submit screening consent with RLS context.
    pub async fn submit_screening_consent_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: ScreeningConsent,
    ) -> Result<TenantScreening, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let screening = sqlx::query_as::<_, TenantScreening>(
            r#"
            UPDATE tenant_screenings SET
                status = 'consent_received',
                consent_received_at = NOW(),
                consent_document_url = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.consent_document_url)
        .fetch_one(executor)
        .await?;

        Ok(screening)
    }

    /// Start screening process with RLS context.
    pub async fn start_screening_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<TenantScreening, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let screening = sqlx::query_as::<_, TenantScreening>(
            r#"
            UPDATE tenant_screenings SET
                status = 'in_progress',
                started_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(executor)
        .await?;

        Ok(screening)
    }

    /// Update screening result with RLS context.
    pub async fn update_screening_result_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateScreeningResult,
    ) -> Result<TenantScreening, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let status = if data.passed.unwrap_or(false) {
            screening_status::COMPLETED
        } else if data.passed == Some(false) {
            screening_status::FAILED
        } else {
            screening_status::COMPLETED
        };

        // Update screening and application status in a single query using CTE
        let screening = sqlx::query_as::<_, TenantScreening>(
            r#"
            WITH updated_screening AS (
                UPDATE tenant_screenings SET
                    status = $2::screening_status,
                    result_summary = $3,
                    risk_score = $4,
                    passed = $5,
                    flags = $6,
                    completed_at = NOW(),
                    expires_at = NOW() + INTERVAL '90 days',
                    updated_at = NOW()
                WHERE id = $1
                RETURNING *
            ),
            app_update AS (
                UPDATE tenant_applications SET status = 'screening_complete'
                WHERE id = (SELECT application_id FROM updated_screening)
                AND NOT EXISTS (
                    SELECT 1 FROM tenant_screenings
                    WHERE application_id = (SELECT application_id FROM updated_screening)
                    AND id != $1
                    AND status NOT IN ('completed', 'failed', 'expired')
                )
            )
            SELECT * FROM updated_screening
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(&data.result_summary)
        .bind(data.risk_score)
        .bind(data.passed)
        .bind(&data.flags)
        .fetch_one(executor)
        .await?;

        Ok(screening)
    }

    /// Get screenings for application with RLS context.
    pub async fn get_screenings_for_application_rls<'e, E>(
        &self,
        executor: E,
        application_id: Uuid,
    ) -> Result<Vec<ScreeningSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let screenings = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                Option<String>,
                String,
                Option<i32>,
                Option<bool>,
                Option<chrono::DateTime<Utc>>,
            ),
        >(
            r#"
            SELECT id, screening_type, provider, status, risk_score, passed, completed_at
            FROM tenant_screenings
            WHERE application_id = $1
            ORDER BY created_at
            "#,
        )
        .bind(application_id)
        .fetch_all(executor)
        .await?;

        Ok(screenings
            .into_iter()
            .map(
                |(id, screening_type, provider, status, risk_score, passed, completed_at)| {
                    ScreeningSummary {
                        id,
                        screening_type,
                        provider,
                        status,
                        risk_score,
                        passed,
                        completed_at,
                    }
                },
            )
            .collect())
    }

    // ------------------------------------------------------------------------
    // Lease Templates (Story 19.3) - RLS versions
    // ------------------------------------------------------------------------

    /// Create lease template with RLS context.
    pub async fn create_template_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateLeaseTemplate,
    ) -> Result<LeaseTemplate, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // If marking as default, unset other defaults and insert in a single transaction using CTE
        let template = sqlx::query_as::<_, LeaseTemplate>(
            r#"
            WITH clear_defaults AS (
                UPDATE lease_templates SET is_default = false
                WHERE organization_id = $1 AND $10 = true
            )
            INSERT INTO lease_templates (
                organization_id, name, description, content_html, content_variables,
                default_term_months, default_security_deposit_months, default_notice_period_days,
                clauses, is_default, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.content_html)
        .bind(&data.content_variables)
        .bind(data.default_term_months)
        .bind(data.default_security_deposit_months)
        .bind(data.default_notice_period_days)
        .bind(&data.clauses)
        .bind(data.is_default.unwrap_or(false))
        .bind(user_id)
        .fetch_one(executor)
        .await?;

        Ok(template)
    }

    /// Find template by ID with RLS context.
    pub async fn find_template_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<LeaseTemplate>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let template =
            sqlx::query_as::<_, LeaseTemplate>(r#"SELECT * FROM lease_templates WHERE id = $1"#)
                .bind(id)
                .fetch_optional(executor)
                .await?;

        Ok(template)
    }

    /// Update template with RLS context.
    pub async fn update_template_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        org_id: Uuid,
        data: UpdateLeaseTemplate,
    ) -> Result<LeaseTemplate, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // If marking as default, unset other defaults and update in a single query using CTE
        let template = sqlx::query_as::<_, LeaseTemplate>(
            r#"
            WITH clear_defaults AS (
                UPDATE lease_templates SET is_default = false
                WHERE organization_id = $2 AND id != $1 AND $10 = true
            )
            UPDATE lease_templates SET
                name = COALESCE($3, name),
                description = COALESCE($4, description),
                content_html = COALESCE($5, content_html),
                content_variables = COALESCE($6, content_variables),
                default_term_months = COALESCE($7, default_term_months),
                default_security_deposit_months = COALESCE($8, default_security_deposit_months),
                default_notice_period_days = COALESCE($9, default_notice_period_days),
                clauses = COALESCE($11, clauses),
                is_default = COALESCE($10, is_default),
                is_active = COALESCE($12, is_active),
                version = version + 1,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(org_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.content_html)
        .bind(&data.content_variables)
        .bind(data.default_term_months)
        .bind(data.default_security_deposit_months)
        .bind(data.default_notice_period_days)
        .bind(data.is_default)
        .bind(&data.clauses)
        .bind(data.is_active)
        .fetch_one(executor)
        .await?;

        Ok(template)
    }

    /// List templates for organization with RLS context.
    pub async fn list_templates_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<LeaseTemplate>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let templates = sqlx::query_as::<_, LeaseTemplate>(
            r#"
            SELECT * FROM lease_templates
            WHERE organization_id = $1 AND is_active = true
            ORDER BY is_default DESC, name ASC
            "#,
        )
        .bind(org_id)
        .fetch_all(executor)
        .await?;

        Ok(templates)
    }

    // ------------------------------------------------------------------------
    // Leases (Story 19.3, 19.4) - RLS versions
    // ------------------------------------------------------------------------

    /// Create lease with RLS context.
    pub async fn create_lease_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateLease,
    ) -> Result<Lease, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let lease = sqlx::query_as::<_, Lease>(sqlx::AssertSqlSafe(lease_write_sql(
            r#"
            INSERT INTO leases (
                organization_id, unit_id, application_id, template_id,
                landlord_user_id, landlord_name, landlord_address,
                tenant_user_id, tenant_name, tenant_email, tenant_phone, occupants,
                start_date, end_date, term_months, is_fixed_term,
                monthly_rent, security_deposit, deposit_held_by, rent_due_day,
                late_fee_amount, late_fee_grace_days,
                utilities_included, parking_spaces, storage_units,
                pets_allowed, pet_deposit, max_occupants, smoking_allowed,
                notes, status, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, 'draft', $31)
            "#,
        )))
        .bind(org_id)
        .bind(data.unit_id)
        .bind(data.application_id)
        .bind(data.template_id)
        .bind(data.landlord_user_id)
        .bind(&data.landlord_name)
        .bind(&data.landlord_address)
        .bind(data.tenant_user_id)
        .bind(&data.tenant_name)
        .bind(&data.tenant_email)
        .bind(&data.tenant_phone)
        .bind(&data.occupants)
        .bind(data.start_date)
        .bind(data.end_date)
        .bind(data.term_months)
        .bind(data.is_fixed_term.unwrap_or(true))
        .bind(data.monthly_rent)
        .bind(data.security_deposit)
        .bind(&data.deposit_held_by)
        .bind(data.rent_due_day.unwrap_or(1))
        .bind(data.late_fee_amount)
        .bind(data.late_fee_grace_days)
        .bind(&data.utilities_included)
        .bind(data.parking_spaces.unwrap_or(0))
        .bind(data.storage_units.unwrap_or(0))
        .bind(data.pets_allowed.unwrap_or(false))
        .bind(data.pet_deposit)
        .bind(data.max_occupants)
        .bind(data.smoking_allowed.unwrap_or(false))
        .bind(&data.notes)
        .bind(user_id)
        .fetch_one(executor)
        .await?;

        Ok(lease)
    }

    /// Find lease by ID with RLS context.
    pub async fn find_lease_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Lease>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let lease = sqlx::query_as::<_, Lease>(sqlx::AssertSqlSafe(select_lease_by_id_sql()))
            .bind(id)
            .fetch_optional(executor)
            .await?;

        Ok(lease)
    }

    /// Update lease with RLS context.
    pub async fn update_lease_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: UpdateLease,
    ) -> Result<Lease, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let lease = sqlx::query_as::<_, Lease>(sqlx::AssertSqlSafe(lease_write_sql(
            r#"
            UPDATE leases SET
                landlord_address = COALESCE($2, landlord_address),
                tenant_phone = COALESCE($3, tenant_phone),
                occupants = COALESCE($4, occupants),
                utilities_included = COALESCE($5, utilities_included),
                parking_spaces = COALESCE($6, parking_spaces),
                storage_units = COALESCE($7, storage_units),
                pets_allowed = COALESCE($8, pets_allowed),
                pet_deposit = COALESCE($9, pet_deposit),
                max_occupants = COALESCE($10, max_occupants),
                smoking_allowed = COALESCE($11, smoking_allowed),
                notes = COALESCE($12, notes),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )))
        .bind(id)
        .bind(&data.landlord_address)
        .bind(&data.tenant_phone)
        .bind(&data.occupants)
        .bind(&data.utilities_included)
        .bind(data.parking_spaces)
        .bind(data.storage_units)
        .bind(data.pets_allowed)
        .bind(data.pet_deposit)
        .bind(data.max_occupants)
        .bind(data.smoking_allowed)
        .bind(&data.notes)
        .fetch_one(executor)
        .await?;

        Ok(lease)
    }

    /// Mark a lease as sent for signature, binding it to the signature request
    /// created in the hardened signature-request subsystem.
    ///
    /// Records the `signature_request_id` and transitions a `draft` lease to
    /// `pending_signature`. The actual signing (landlord/tenant) and the
    /// resulting status transitions are owned by the signature-request
    /// subsystem (`routes::signatures`), not by lease-native logic — this is
    /// Option A wiring for issue #977.
    pub async fn mark_sent_for_signature_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        signature_request_id: Uuid,
    ) -> Result<Lease, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let lease = sqlx::query_as::<_, Lease>(sqlx::AssertSqlSafe(lease_write_sql(
            r#"
            UPDATE leases SET
                signature_request_id = $2,
                status = CASE WHEN status = 'draft' THEN 'pending_signature' ELSE status END,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )))
        .bind(id)
        .bind(signature_request_id)
        .fetch_one(executor)
        .await?;

        Ok(lease)
    }

    /// Terminate lease with RLS context.
    pub async fn terminate_lease_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        user_id: Uuid,
        data: TerminateLease,
    ) -> Result<Lease, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let lease = sqlx::query_as::<_, Lease>(sqlx::AssertSqlSafe(lease_write_sql(
            r#"
            UPDATE leases SET
                status = 'terminated',
                terminated_at = COALESCE($4, NOW()),
                termination_reason = $2::termination_reason,
                termination_notes = $3,
                termination_initiated_by = $5,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )))
        .bind(id)
        .bind(&data.termination_reason)
        .bind(&data.termination_notes)
        .bind(data.effective_date)
        .bind(user_id)
        .fetch_one(executor)
        .await?;

        Ok(lease)
    }

    /// Renew lease with RLS context.
    ///
    /// Note: This method creates a new lease based on an existing one.
    /// The new lease is created in 'draft' status and the old lease is marked as 'renewed'.
    pub async fn renew_lease_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        user_id: Uuid,
        data: RenewLease,
    ) -> Result<Lease, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Use a CTE to create new lease and update old one in a single query.
        // The inner `RETURNING *` only feeds the CTE; the outer `SELECT` casts
        // the lease enum columns via LEASE_COLUMNS so the row decodes into `Lease`.
        let new_lease = sqlx::query_as::<_, Lease>(sqlx::AssertSqlSafe(format!(
            r#"
            WITH new_lease AS (
                INSERT INTO leases (
                    organization_id, unit_id, template_id,
                    landlord_user_id, landlord_name, landlord_address,
                    tenant_user_id, tenant_name, tenant_email, tenant_phone, occupants,
                    start_date, end_date, term_months, is_fixed_term,
                    monthly_rent, security_deposit, deposit_held_by, rent_due_day,
                    late_fee_amount, late_fee_grace_days,
                    utilities_included, parking_spaces, storage_units,
                    pets_allowed, pet_deposit, max_occupants, smoking_allowed,
                    notes, status, previous_lease_id, created_by
                )
                SELECT
                    organization_id, unit_id, template_id,
                    landlord_user_id, landlord_name, landlord_address,
                    tenant_user_id, tenant_name, tenant_email, tenant_phone, occupants,
                    end_date + INTERVAL '1 day', $2, $3, is_fixed_term,
                    COALESCE($4, monthly_rent), COALESCE($5, security_deposit), deposit_held_by, rent_due_day,
                    late_fee_amount, late_fee_grace_days,
                    utilities_included, parking_spaces, storage_units,
                    pets_allowed, pet_deposit, max_occupants, smoking_allowed,
                    $6, 'draft', id, $7
                FROM leases WHERE id = $1
                RETURNING *
            ),
            update_old AS (
                UPDATE leases SET
                    status = 'renewed',
                    renewed_to_lease_id = (SELECT id FROM new_lease),
                    updated_at = NOW()
                WHERE id = $1
            )
            SELECT {LEASE_COLUMNS} FROM new_lease
            "#,
        )))
        .bind(id)
        .bind(data.new_end_date)
        .bind(data.term_months)
        .bind(data.new_monthly_rent)
        .bind(data.new_security_deposit)
        .bind(&data.notes)
        .bind(user_id)
        .fetch_one(executor)
        .await?;

        Ok(new_lease)
    }

    /// List leases for organization with RLS context.
    pub async fn list_leases_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: LeaseListQuery,
    ) -> Result<(Vec<LeaseSummary>, i64), SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(20);
        let offset = query.offset.unwrap_or(0);
        let today = Utc::now().date_naive();

        // Get leases
        let leases = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                String,
                String,
                String,
                NaiveDate,
                NaiveDate,
                Decimal,
                String,
                i64,
            ),
        >(
            r#"
            SELECT
                l.id, l.unit_id, u.designation, b.name,
                l.tenant_name, l.tenant_email,
                l.start_date, l.end_date, l.monthly_rent, l.status::text,
                (l.end_date - $6::date)::int8 as days_until_expiry
            FROM leases l
            JOIN units u ON u.id = l.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE l.organization_id = $1
                AND ($2::uuid IS NULL OR l.unit_id = $2)
                AND ($3::uuid IS NULL OR l.tenant_user_id = $3)
                AND ($4::text IS NULL OR l.status = $4::lease_status)
                AND ($5::int IS NULL OR l.end_date <= $6::date + ($5 || ' days')::interval)
            ORDER BY l.end_date ASC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(org_id)
        .bind(query.unit_id)
        .bind(query.tenant_id)
        .bind(&query.status)
        .bind(query.expiring_within_days)
        .bind(today)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await?;

        let summaries = leases
            .into_iter()
            .map(
                |(
                    id,
                    unit_id,
                    unit_name,
                    building_name,
                    tenant_name,
                    tenant_email,
                    start_date,
                    end_date,
                    monthly_rent,
                    status,
                    days_until_expiry,
                )| {
                    LeaseSummary {
                        id,
                        unit_id,
                        unit_name,
                        building_name,
                        tenant_name,
                        tenant_email,
                        start_date,
                        end_date,
                        monthly_rent,
                        status,
                        days_until_expiry,
                    }
                },
            )
            .collect();

        // For RLS version, return -1 to indicate count was not fetched
        Ok((summaries, -1))
    }

    /// Count leases for organization with RLS context.
    pub async fn count_leases_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        query: &LeaseListQuery,
    ) -> Result<i64, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let today = Utc::now().date_naive();

        let (total,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM leases l
            WHERE l.organization_id = $1
                AND ($2::uuid IS NULL OR l.unit_id = $2)
                AND ($3::uuid IS NULL OR l.tenant_user_id = $3)
                AND ($4::text IS NULL OR l.status = $4::lease_status)
                AND ($5::int IS NULL OR l.end_date <= $6::date + ($5 || ' days')::interval)
            "#,
        )
        .bind(org_id)
        .bind(query.unit_id)
        .bind(query.tenant_id)
        .bind(&query.status)
        .bind(query.expiring_within_days)
        .bind(today)
        .fetch_one(executor)
        .await?;

        Ok(total)
    }

    /// Get lease with full details with RLS context.
    ///
    /// Note: This is a simplified RLS version that returns the lease without amendments,
    /// payments, and reminders. For full details, make separate calls using
    /// `list_amendments_rls`, `list_upcoming_payments_rls`, and `list_pending_reminders_rls`.
    pub async fn get_lease_with_details_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<LeaseWithDetails>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Get lease first. Uses the shared LEASE_COLUMNS projection (enum columns
        // cast to text) — a bare `SELECT *` here returned native enum types and
        // failed `Lease` decode with ColumnDecode, 500'ing GET /leases/{id} (#2169).
        let lease = match sqlx::query_as::<_, Lease>(sqlx::AssertSqlSafe(select_lease_by_id_sql()))
            .bind(id)
            .fetch_optional(executor)
            .await?
        {
            Some(l) => l,
            None => return Ok(None),
        };

        // Note: For RLS version, we can't make additional queries with the same executor
        // since it's consumed. Return lease with empty related data.
        // Callers should use list_amendments_rls, list_upcoming_payments_rls, etc.
        // with a fresh executor if they need the related data.
        Ok(Some(LeaseWithDetails {
            lease,
            unit_name: String::new(),      // Simplified for RLS version
            building_name: String::new(),  // Simplified for RLS version
            amendments: Vec::new(),        // Simplified for RLS version
            upcoming_payments: Vec::new(), // Simplified for RLS version
            reminders: Vec::new(),         // Simplified for RLS version
        }))
    }

    // ------------------------------------------------------------------------
    // Amendments - RLS versions
    // ------------------------------------------------------------------------

    /// Create lease amendment with RLS context.
    pub async fn create_amendment_rls<'e, E>(
        &self,
        executor: E,
        lease_id: Uuid,
        user_id: Uuid,
        data: CreateAmendment,
    ) -> Result<LeaseAmendment, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let amendment = sqlx::query_as::<_, LeaseAmendment>(
            r#"
            INSERT INTO lease_amendments (
                lease_id, amendment_number, title, description, changes, effective_date, created_by
            )
            VALUES (
                $1,
                (SELECT COALESCE(MAX(amendment_number), 0) + 1 FROM lease_amendments WHERE lease_id = $1),
                $2, $3, $4, $5, $6
            )
            RETURNING *
            "#,
        )
        .bind(lease_id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.changes)
        .bind(data.effective_date)
        .bind(user_id)
        .fetch_one(executor)
        .await?;

        Ok(amendment)
    }

    /// List amendments for lease with RLS context.
    pub async fn list_amendments_rls<'e, E>(
        &self,
        executor: E,
        lease_id: Uuid,
    ) -> Result<Vec<LeaseAmendment>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let amendments = sqlx::query_as::<_, LeaseAmendment>(
            r#"SELECT * FROM lease_amendments WHERE lease_id = $1 ORDER BY amendment_number"#,
        )
        .bind(lease_id)
        .fetch_all(executor)
        .await?;

        Ok(amendments)
    }

    // ------------------------------------------------------------------------
    // Payments - RLS versions
    // ------------------------------------------------------------------------

    /// Generate payment schedule for lease with RLS context.
    ///
    /// Note: This creates payment records for each month from lease start to end.
    pub async fn generate_payment_schedule_rls<'e, E>(
        &self,
        executor: E,
        lease_id: Uuid,
    ) -> Result<Vec<LeasePayment>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Use a single query with generate_series to create all payments
        let payments = sqlx::query_as::<_, LeasePayment>(
            r#"
            WITH lease_info AS (
                SELECT id, organization_id, start_date, end_date, monthly_rent, rent_due_day
                FROM leases WHERE id = $1
            ),
            months AS (
                SELECT generate_series(
                    date_trunc('month', (SELECT start_date FROM lease_info)),
                    (SELECT end_date FROM lease_info),
                    '1 month'::interval
                )::date as month_start
            )
            INSERT INTO lease_payments (lease_id, organization_id, due_date, amount, payment_type, description)
            SELECT
                $1,
                (SELECT organization_id FROM lease_info),
                make_date(
                    EXTRACT(YEAR FROM m.month_start)::int,
                    EXTRACT(MONTH FROM m.month_start)::int,
                    LEAST((SELECT rent_due_day FROM lease_info), EXTRACT(DAY FROM (m.month_start + INTERVAL '1 month' - INTERVAL '1 day'))::int)
                ),
                (SELECT monthly_rent FROM lease_info),
                'rent',
                'Monthly rent'
            FROM months m
            WHERE m.month_start < (SELECT end_date FROM lease_info)
            ON CONFLICT DO NOTHING
            RETURNING *
            "#,
        )
        .bind(lease_id)
        .fetch_all(executor)
        .await?;

        Ok(payments)
    }

    /// Record payment with RLS context.
    pub async fn record_payment_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
        data: RecordPayment,
    ) -> Result<LeasePayment, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let payment = sqlx::query_as::<_, LeasePayment>(
            r#"
            UPDATE lease_payments SET
                paid_at = COALESCE($2, NOW()),
                paid_amount = $3,
                payment_method = $4,
                payment_reference = $5,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.paid_at)
        .bind(data.paid_amount)
        .bind(&data.payment_method)
        .bind(&data.payment_reference)
        .fetch_one(executor)
        .await?;

        Ok(payment)
    }

    /// Get overdue payments with RLS context.
    pub async fn get_overdue_payments_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<Vec<PaymentSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let today = Utc::now().date_naive();

        let payments = sqlx::query_as::<_, (Uuid, NaiveDate, Decimal, String, Option<chrono::DateTime<Utc>>, Option<Decimal>, bool, Option<Decimal>)>(
            r#"
            SELECT id, due_date, amount, payment_type, paid_at, paid_amount, is_late, late_fee_applied
            FROM lease_payments
            WHERE organization_id = $1 AND due_date < $2 AND paid_at IS NULL
            ORDER BY due_date
            "#,
        )
        .bind(org_id)
        .bind(today)
        .fetch_all(executor)
        .await?;

        Ok(payments
            .into_iter()
            .map(
                |(
                    id,
                    due_date,
                    amount,
                    payment_type,
                    paid_at,
                    paid_amount,
                    is_late,
                    late_fee_applied,
                )| {
                    PaymentSummary {
                        id,
                        due_date,
                        amount,
                        payment_type,
                        paid_at,
                        paid_amount,
                        is_late,
                        late_fee_applied,
                    }
                },
            )
            .collect())
    }

    /// List upcoming payments for lease with RLS context.
    pub async fn list_upcoming_payments_rls<'e, E>(
        &self,
        executor: E,
        lease_id: Uuid,
        limit: i64,
    ) -> Result<Vec<LeasePayment>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let today = Utc::now().date_naive();

        let payments = sqlx::query_as::<_, LeasePayment>(
            r#"SELECT * FROM lease_payments WHERE lease_id = $1 AND due_date >= $2 ORDER BY due_date LIMIT $3"#,
        )
        .bind(lease_id)
        .bind(today)
        .bind(limit)
        .fetch_all(executor)
        .await?;

        Ok(payments)
    }

    // ------------------------------------------------------------------------
    // Reminders (Story 19.5) - RLS versions
    // ------------------------------------------------------------------------

    /// Create reminder with RLS context.
    pub async fn create_reminder_rls<'e, E>(
        &self,
        executor: E,
        lease_id: Uuid,
        data: CreateReminder,
    ) -> Result<LeaseReminder, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let reminder = sqlx::query_as::<_, LeaseReminder>(
            r#"
            INSERT INTO lease_reminders (
                lease_id, reminder_type, trigger_date, days_before_event,
                subject, message, recipients, is_recurring, recurrence_pattern
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(lease_id)
        .bind(&data.reminder_type)
        .bind(data.trigger_date)
        .bind(data.days_before_event)
        .bind(&data.subject)
        .bind(&data.message)
        .bind(&data.recipients)
        .bind(data.is_recurring.unwrap_or(false))
        .bind(&data.recurrence_pattern)
        .fetch_one(executor)
        .await?;

        Ok(reminder)
    }

    /// List pending reminders for lease with RLS context.
    pub async fn list_pending_reminders_rls<'e, E>(
        &self,
        executor: E,
        lease_id: Uuid,
    ) -> Result<Vec<LeaseReminder>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let reminders = sqlx::query_as::<_, LeaseReminder>(
            r#"SELECT * FROM lease_reminders WHERE lease_id = $1 AND sent_at IS NULL ORDER BY trigger_date"#,
        )
        .bind(lease_id)
        .fetch_all(executor)
        .await?;

        Ok(reminders)
    }

    /// Get expiration overview with RLS context.
    ///
    /// Note: This is a simplified version that returns lease counts by expiration period.
    pub async fn get_expiration_overview_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<ExpirationOverview, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let today = Utc::now().date_naive();
        let day_30 = today + Duration::days(30);
        let day_60 = today + Duration::days(60);
        let day_90 = today + Duration::days(90);

        // Get all counts and lists in a single query using CASE expressions
        let leases = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                String,
                String,
                String,
                NaiveDate,
                NaiveDate,
                Decimal,
                String,
                i64,
            ),
        >(
            r#"
            SELECT
                l.id, l.unit_id, u.designation, b.name,
                l.tenant_name, l.tenant_email,
                l.start_date, l.end_date, l.monthly_rent, l.status::text,
                (l.end_date - $2::date)::int8 as days_until_expiry
            FROM leases l
            JOIN units u ON u.id = l.unit_id
            JOIN buildings b ON b.id = u.building_id
            WHERE l.organization_id = $1
                AND l.status = 'active'
                AND l.end_date <= $5
            ORDER BY l.end_date ASC
            "#,
        )
        .bind(org_id)
        .bind(today)
        .bind(day_30)
        .bind(day_60)
        .bind(day_90)
        .fetch_all(executor)
        .await?;

        let mut expiring_30_days = Vec::new();
        let mut expiring_60_days = Vec::new();
        let mut expiring_90_days = Vec::new();

        for (
            id,
            unit_id,
            unit_name,
            building_name,
            tenant_name,
            tenant_email,
            start_date,
            end_date,
            monthly_rent,
            status,
            days_until_expiry,
        ) in leases
        {
            let summary = LeaseSummary {
                id,
                unit_id,
                unit_name,
                building_name,
                tenant_name,
                tenant_email,
                start_date,
                end_date,
                monthly_rent,
                status,
                days_until_expiry,
            };

            if end_date <= day_30 {
                expiring_30_days.push(summary);
            } else if end_date <= day_60 {
                expiring_60_days.push(summary);
            } else {
                expiring_90_days.push(summary);
            }
        }

        let total_expiring_soon =
            (expiring_30_days.len() + expiring_60_days.len() + expiring_90_days.len()) as i64;

        Ok(ExpirationOverview {
            expiring_30_days,
            expiring_60_days,
            expiring_90_days,
            total_active_leases: -1, // Simplified - caller can make separate count call
            total_expiring_soon,
        })
    }

    // ------------------------------------------------------------------------
    // Statistics - RLS versions
    // ------------------------------------------------------------------------

    /// Get lease statistics with RLS context.
    pub async fn get_statistics_rls<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
    ) -> Result<LeaseStatistics, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let today = Utc::now().date_naive();
        let day_90 = today + Duration::days(90);

        // Get all statistics in a single query
        let stats = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, Option<Decimal>, i64, i64)>(
            r#"
            SELECT
                (SELECT COUNT(*) FROM leases WHERE organization_id = $1) as total_leases,
                (SELECT COUNT(*) FROM leases WHERE organization_id = $1 AND status = 'active') as active_leases,
                (SELECT COUNT(*) FROM leases WHERE organization_id = $1 AND status = 'pending_signature') as pending_signatures,
                (SELECT COUNT(*) FROM leases WHERE organization_id = $1 AND status = 'active' AND end_date <= $2) as expiring_soon,
                (SELECT COUNT(*) FROM tenant_applications WHERE organization_id = $1) as total_applications,
                (SELECT COUNT(*) FROM tenant_applications WHERE organization_id = $1 AND status IN ('submitted', 'under_review', 'screening_pending')) as pending_applications,
                (SELECT COALESCE(SUM(monthly_rent), 0) FROM leases WHERE organization_id = $1 AND status = 'active') as total_monthly_rent,
                (SELECT COUNT(*) FROM units WHERE organization_id = $1) as total_units,
                (SELECT COUNT(DISTINCT unit_id) FROM leases WHERE organization_id = $1 AND status = 'active') as occupied_units
            "#,
        )
        .bind(org_id)
        .bind(day_90)
        .fetch_one(executor)
        .await?;

        let (
            total_leases,
            active_leases,
            pending_signatures,
            expiring_soon,
            total_applications,
            pending_applications,
            total_monthly_rent,
            total_units,
            occupied_units,
        ) = stats;

        let occupancy_rate = if total_units > 0 {
            (occupied_units as f64 / total_units as f64) * 100.0
        } else {
            0.0
        };

        Ok(LeaseStatistics {
            total_leases,
            active_leases,
            pending_signatures,
            expiring_soon,
            total_applications,
            pending_applications,
            total_monthly_rent: total_monthly_rent.unwrap_or_default(),
            occupancy_rate,
        })
    }

    // ========================================================================
    // Legacy methods (use pool directly - migrate to RLS versions)
    // ========================================================================

    // ------------------------------------------------------------------------
    // Applications (Story 19.1) - Legacy versions
    // ------------------------------------------------------------------------

    /// Create tenant application.
    ///
    /// **Deprecated**: Use `create_application_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use create_application_rls with RlsConnection instead"
    )]
    pub async fn create_application(
        &self,
        org_id: Uuid,
        data: CreateApplication,
    ) -> Result<TenantApplication, SqlxError> {
        self.create_application_rls(&self.pool, org_id, data).await
    }

    /// Find application by ID.
    ///
    /// **Deprecated**: Use `find_application_by_id_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use find_application_by_id_rls with RlsConnection instead"
    )]
    pub async fn find_application_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<TenantApplication>, SqlxError> {
        self.find_application_by_id_rls(&self.pool, id).await
    }

    /// Update application.
    ///
    /// **Deprecated**: Use `update_application_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use update_application_rls with RlsConnection instead"
    )]
    pub async fn update_application(
        &self,
        id: Uuid,
        data: UpdateApplication,
    ) -> Result<TenantApplication, SqlxError> {
        self.update_application_rls(&self.pool, id, data).await
    }

    /// Submit application.
    ///
    /// **Deprecated**: Use `submit_application_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use submit_application_rls with RlsConnection instead"
    )]
    pub async fn submit_application(
        &self,
        id: Uuid,
        data: SubmitApplication,
    ) -> Result<TenantApplication, SqlxError> {
        self.submit_application_rls(&self.pool, id, data).await
    }

    /// Review application.
    ///
    /// **Deprecated**: Use `review_application_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use review_application_rls with RlsConnection instead"
    )]
    pub async fn review_application(
        &self,
        id: Uuid,
        reviewer_id: Uuid,
        data: ReviewApplication,
    ) -> Result<TenantApplication, SqlxError> {
        self.review_application_rls(&self.pool, id, reviewer_id, data)
            .await
    }

    /// Delete application.
    ///
    /// **Deprecated**: Use `delete_application_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use delete_application_rls with RlsConnection instead"
    )]
    pub async fn delete_application(&self, id: Uuid) -> Result<bool, SqlxError> {
        self.delete_application_rls(&self.pool, id).await
    }

    /// List applications for organization.
    ///
    /// **Deprecated**: Use `list_applications_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use list_applications_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn list_applications(
        &self,
        org_id: Uuid,
        query: ApplicationListQuery,
    ) -> Result<(Vec<ApplicationSummary>, i64), SqlxError> {
        // For legacy version, we need to get the count separately
        let total = self
            .count_applications_rls(&self.pool, org_id, &query)
            .await?;
        let (summaries, _) = self
            .list_applications_rls(&self.pool, org_id, query)
            .await?;
        Ok((summaries, total))
    }

    // ------------------------------------------------------------------------
    // Screening (Story 19.2) - Legacy versions
    // ------------------------------------------------------------------------

    /// Initiate screening for application.
    ///
    /// **Deprecated**: Use `initiate_screening_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use initiate_screening_rls with RlsConnection instead"
    )]
    pub async fn initiate_screening(
        &self,
        application_id: Uuid,
        org_id: Uuid,
        data: InitiateScreening,
    ) -> Result<Vec<TenantScreening>, SqlxError> {
        self.initiate_screening_rls(&self.pool, application_id, org_id, data)
            .await
    }

    /// Submit screening consent.
    ///
    /// **Deprecated**: Use `submit_screening_consent_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use submit_screening_consent_rls with RlsConnection instead"
    )]
    pub async fn submit_screening_consent(
        &self,
        id: Uuid,
        data: ScreeningConsent,
    ) -> Result<TenantScreening, SqlxError> {
        self.submit_screening_consent_rls(&self.pool, id, data)
            .await
    }

    /// Start screening process.
    ///
    /// **Deprecated**: Use `start_screening_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use start_screening_rls with RlsConnection instead"
    )]
    pub async fn start_screening(&self, id: Uuid) -> Result<TenantScreening, SqlxError> {
        self.start_screening_rls(&self.pool, id).await
    }

    /// Update screening result.
    ///
    /// **Deprecated**: Use `update_screening_result_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use update_screening_result_rls with RlsConnection instead"
    )]
    pub async fn update_screening_result(
        &self,
        id: Uuid,
        data: UpdateScreeningResult,
    ) -> Result<TenantScreening, SqlxError> {
        self.update_screening_result_rls(&self.pool, id, data).await
    }

    /// Get screenings for application.
    ///
    /// **Deprecated**: Use `get_screenings_for_application_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_screenings_for_application_rls with RlsConnection instead"
    )]
    pub async fn get_screenings_for_application(
        &self,
        application_id: Uuid,
    ) -> Result<Vec<ScreeningSummary>, SqlxError> {
        self.get_screenings_for_application_rls(&self.pool, application_id)
            .await
    }

    // ------------------------------------------------------------------------
    // Lease Templates (Story 19.3) - Legacy versions
    // ------------------------------------------------------------------------

    /// Create lease template.
    ///
    /// **Deprecated**: Use `create_template_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use create_template_rls with RlsConnection instead"
    )]
    pub async fn create_template(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateLeaseTemplate,
    ) -> Result<LeaseTemplate, SqlxError> {
        self.create_template_rls(&self.pool, org_id, user_id, data)
            .await
    }

    /// Find template by ID.
    ///
    /// **Deprecated**: Use `find_template_by_id_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use find_template_by_id_rls with RlsConnection instead"
    )]
    pub async fn find_template_by_id(&self, id: Uuid) -> Result<Option<LeaseTemplate>, SqlxError> {
        self.find_template_by_id_rls(&self.pool, id).await
    }

    /// Update template.
    ///
    /// **Deprecated**: Use `update_template_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use update_template_rls with RlsConnection instead"
    )]
    pub async fn update_template(
        &self,
        id: Uuid,
        org_id: Uuid,
        data: UpdateLeaseTemplate,
    ) -> Result<LeaseTemplate, SqlxError> {
        self.update_template_rls(&self.pool, id, org_id, data).await
    }

    /// List templates for organization.
    ///
    /// **Deprecated**: Use `list_templates_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use list_templates_rls with RlsConnection instead"
    )]
    pub async fn list_templates(&self, org_id: Uuid) -> Result<Vec<LeaseTemplate>, SqlxError> {
        self.list_templates_rls(&self.pool, org_id).await
    }

    // ------------------------------------------------------------------------
    // Leases (Story 19.3, 19.4) - Legacy versions
    // ------------------------------------------------------------------------

    /// Create lease.
    ///
    /// **Deprecated**: Use `create_lease_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use create_lease_rls with RlsConnection instead"
    )]
    pub async fn create_lease(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        data: CreateLease,
    ) -> Result<Lease, SqlxError> {
        self.create_lease_rls(&self.pool, org_id, user_id, data)
            .await
    }

    /// Find lease by ID.
    ///
    /// **Deprecated**: Use `find_lease_by_id_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use find_lease_by_id_rls with RlsConnection instead"
    )]
    pub async fn find_lease_by_id(&self, id: Uuid) -> Result<Option<Lease>, SqlxError> {
        self.find_lease_by_id_rls(&self.pool, id).await
    }

    /// Update lease.
    ///
    /// **Deprecated**: Use `update_lease_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use update_lease_rls with RlsConnection instead"
    )]
    pub async fn update_lease(&self, id: Uuid, data: UpdateLease) -> Result<Lease, SqlxError> {
        self.update_lease_rls(&self.pool, id, data).await
    }

    /// Terminate lease.
    ///
    /// **Deprecated**: Use `terminate_lease_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use terminate_lease_rls with RlsConnection instead"
    )]
    pub async fn terminate_lease(
        &self,
        id: Uuid,
        user_id: Uuid,
        data: TerminateLease,
    ) -> Result<Lease, SqlxError> {
        self.terminate_lease_rls(&self.pool, id, user_id, data)
            .await
    }

    /// Renew lease.
    ///
    /// **Deprecated**: Use `renew_lease_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use renew_lease_rls with RlsConnection instead"
    )]
    pub async fn renew_lease(
        &self,
        id: Uuid,
        user_id: Uuid,
        data: RenewLease,
    ) -> Result<Lease, SqlxError> {
        self.renew_lease_rls(&self.pool, id, user_id, data).await
    }

    /// List leases for organization.
    ///
    /// **Deprecated**: Use `list_leases_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use list_leases_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn list_leases(
        &self,
        org_id: Uuid,
        query: LeaseListQuery,
    ) -> Result<(Vec<LeaseSummary>, i64), SqlxError> {
        // For legacy version, we need to get the count separately
        let total = self.count_leases_rls(&self.pool, org_id, &query).await?;
        let (summaries, _) = self.list_leases_rls(&self.pool, org_id, query).await?;
        Ok((summaries, total))
    }

    /// Get lease with full details.
    ///
    /// **Deprecated**: Use `get_lease_with_details_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_lease_with_details_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn get_lease_with_details(
        &self,
        id: Uuid,
    ) -> Result<Option<LeaseWithDetails>, SqlxError> {
        let lease = match self.find_lease_by_id(id).await? {
            Some(l) => l,
            None => return Ok(None),
        };

        // Get unit and building names
        let (unit_name, building_name): (String, String) = sqlx::query_as(
            r#"SELECT u.designation, b.name FROM units u JOIN buildings b ON b.id = u.building_id WHERE u.id = $1"#,
        )
        .bind(lease.unit_id)
        .fetch_one(&self.pool)
        .await?;

        // Get amendments
        let amendments = sqlx::query_as::<_, LeaseAmendment>(
            r#"SELECT * FROM lease_amendments WHERE lease_id = $1 ORDER BY amendment_number"#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        // Get upcoming payments
        let today = Utc::now().date_naive();
        let upcoming_payments = sqlx::query_as::<_, LeasePayment>(
            r#"SELECT * FROM lease_payments WHERE lease_id = $1 AND due_date >= $2 ORDER BY due_date LIMIT 5"#,
        )
        .bind(id)
        .bind(today)
        .fetch_all(&self.pool)
        .await?;

        // Get reminders
        let reminders = sqlx::query_as::<_, LeaseReminder>(
            r#"SELECT * FROM lease_reminders WHERE lease_id = $1 AND sent_at IS NULL ORDER BY trigger_date"#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(LeaseWithDetails {
            lease,
            unit_name,
            building_name,
            amendments,
            upcoming_payments,
            reminders,
        }))
    }

    // ------------------------------------------------------------------------
    // Amendments - Legacy versions
    // ------------------------------------------------------------------------

    /// Create lease amendment.
    ///
    /// **Deprecated**: Use `create_amendment_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use create_amendment_rls with RlsConnection instead"
    )]
    pub async fn create_amendment(
        &self,
        lease_id: Uuid,
        user_id: Uuid,
        data: CreateAmendment,
    ) -> Result<LeaseAmendment, SqlxError> {
        self.create_amendment_rls(&self.pool, lease_id, user_id, data)
            .await
    }

    // ------------------------------------------------------------------------
    // Payments - Legacy versions
    // ------------------------------------------------------------------------

    /// Generate payment schedule for lease.
    ///
    /// **Deprecated**: Use `generate_payment_schedule_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use generate_payment_schedule_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn generate_payment_schedule(
        &self,
        lease_id: Uuid,
    ) -> Result<Vec<LeasePayment>, SqlxError> {
        let lease = self
            .find_lease_by_id(lease_id)
            .await?
            .ok_or_else(|| SqlxError::RowNotFound)?;

        let mut payments = Vec::new();
        let mut current_date = lease.start_date;

        // Generate monthly rent payments
        while current_date < lease.end_date {
            let due_date = NaiveDate::from_ymd_opt(
                current_date.year(),
                current_date.month(),
                lease.rent_due_day as u32,
            )
            .unwrap_or(current_date);

            let payment = sqlx::query_as::<_, LeasePayment>(
                r#"
                INSERT INTO lease_payments (lease_id, organization_id, due_date, amount, payment_type, description)
                VALUES ($1, $2, $3, $4, 'rent', 'Monthly rent')
                ON CONFLICT DO NOTHING
                RETURNING *
                "#,
            )
            .bind(lease_id)
            .bind(lease.organization_id)
            .bind(due_date)
            .bind(lease.monthly_rent)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(p) = payment {
                payments.push(p);
            }

            // Move to next month
            current_date += Duration::days(32);
            current_date = NaiveDate::from_ymd_opt(current_date.year(), current_date.month(), 1)
                .unwrap_or(current_date);
        }

        Ok(payments)
    }

    /// Record payment.
    ///
    /// **Deprecated**: Use `record_payment_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use record_payment_rls with RlsConnection instead"
    )]
    pub async fn record_payment(
        &self,
        id: Uuid,
        data: RecordPayment,
    ) -> Result<LeasePayment, SqlxError> {
        self.record_payment_rls(&self.pool, id, data).await
    }

    /// Get overdue payments.
    ///
    /// **Deprecated**: Use `get_overdue_payments_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_overdue_payments_rls with RlsConnection instead"
    )]
    pub async fn get_overdue_payments(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<PaymentSummary>, SqlxError> {
        self.get_overdue_payments_rls(&self.pool, org_id).await
    }

    // ------------------------------------------------------------------------
    // Reminders (Story 19.5) - Legacy versions
    // ------------------------------------------------------------------------

    /// Create reminder.
    ///
    /// **Deprecated**: Use `create_reminder_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use create_reminder_rls with RlsConnection instead"
    )]
    pub async fn create_reminder(
        &self,
        lease_id: Uuid,
        data: CreateReminder,
    ) -> Result<LeaseReminder, SqlxError> {
        self.create_reminder_rls(&self.pool, lease_id, data).await
    }

    /// Get expiration overview.
    ///
    /// **Deprecated**: Use `get_expiration_overview_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_expiration_overview_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn get_expiration_overview(
        &self,
        org_id: Uuid,
    ) -> Result<ExpirationOverview, SqlxError> {
        let today = Utc::now().date_naive();
        let day_30 = today + Duration::days(30);
        let day_60 = today + Duration::days(60);
        let day_90 = today + Duration::days(90);

        // Get leases expiring in each period
        let get_expiring = |start: NaiveDate, end: NaiveDate| async move {
            self.list_leases(
                org_id,
                LeaseListQuery {
                    status: Some(lease_status::ACTIVE.to_string()),
                    expiring_within_days: Some((end - today).num_days() as i32),
                    ..Default::default()
                },
            )
            .await
            .map(|(leases, _)| {
                leases
                    .into_iter()
                    .filter(|l| l.end_date >= start && l.end_date <= end)
                    .collect::<Vec<_>>()
            })
        };

        let expiring_30_days = get_expiring(today, day_30).await?;
        let expiring_60_days = get_expiring(day_30, day_60).await?;
        let expiring_90_days = get_expiring(day_60, day_90).await?;

        let (total_active_leases,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM leases WHERE organization_id = $1 AND status = 'active'"#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let total_expiring_soon =
            (expiring_30_days.len() + expiring_60_days.len() + expiring_90_days.len()) as i64;

        Ok(ExpirationOverview {
            expiring_30_days,
            expiring_60_days,
            expiring_90_days,
            total_active_leases,
            total_expiring_soon,
        })
    }

    // ------------------------------------------------------------------------
    // Statistics - Legacy versions
    // ------------------------------------------------------------------------

    /// Get lease statistics.
    ///
    /// **Deprecated**: Use `get_statistics_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.276",
        note = "Use get_statistics_rls with RlsConnection instead"
    )]
    pub async fn get_statistics(&self, org_id: Uuid) -> Result<LeaseStatistics, SqlxError> {
        self.get_statistics_rls(&self.pool, org_id).await
    }
}

impl Default for LeaseListQuery {
    fn default() -> Self {
        Self {
            unit_id: None,
            tenant_id: None,
            status: None,
            expiring_within_days: None,
            limit: Some(20),
            offset: Some(0),
        }
    }
}
