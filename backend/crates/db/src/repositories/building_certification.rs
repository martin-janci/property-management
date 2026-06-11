//! Building Certification Repository for Epic 137: Smart Building Certification.
//!
//! # RLS Integration (PAP-67 / PAP-80)
//!
//! Migration `00179` (PAP-62) put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on the building-certification cluster:
//! `building_certifications`, `certification_credits`, `certification_documents`,
//! `certification_milestones`, `certification_benchmarks`,
//! `certification_audit_logs`, `certification_costs`, and
//! `certification_reminders` (8 tables). Under `FORCE` the api-server's
//! table-owner connection is no longer exempt, so a query issued on a connection
//! without `app.current_org_id` set collapses to deny-all (own-org reads return
//! empty, writes fail the policy `WITH CHECK`).
//!
//! This repository therefore holds **no pool**. Every method takes an executor
//! whose connection already has RLS context set — in handlers this comes from
//! the `RlsConnection` extractor via `&mut **rls.conn()`:
//!
//! * single-statement methods take a generic `executor: E`;
//! * multi-statement methods that compose helpers take `conn: &mut PgConnection`
//!   and reborrow `&mut *conn` per call.
//!
//! Because the repo cannot reach a raw pool, there is no path that bypasses RLS.
//! This mirrors the `work_order.rs` / `reserve_funds.rs` precedent.
//!
//! The explicit `organization_id = $n` predicates are retained as
//! defense-in-depth: the authoritative org is the caller's `rls.tenant_id()`,
//! so the SQL filter and the RLS context can never disagree, and cross-tenant
//! by-id reads return no row (→ 404) under both layers.

use sqlx::{Executor, PgConnection, PgPool, Postgres};
use uuid::Uuid;

use crate::models::building_certification::{
    BuildingCertification, CertificationAuditLog, CertificationBenchmark, CertificationCost,
    CertificationCredit, CertificationDashboard, CertificationDocument, CertificationFilters,
    CertificationLevelCount, CertificationMilestone, CertificationProgram,
    CertificationProgramCount, CertificationReminder, CertificationWithCredits,
    CreateBuildingCertification, CreateCertificationBenchmark, CreateCertificationCost,
    CreateCertificationCredit, CreateCertificationDocument, CreateCertificationMilestone,
    CreateCertificationReminder, UpdateBuildingCertification, UpdateCertificationCredit,
    UpdateCertificationMilestone,
};

/// Repository for building certification operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Clone)]
pub struct BuildingCertificationRepository;

impl BuildingCertificationRepository {
    /// Create a new repository instance.
    ///
    /// The pool argument is retained for construction-site compatibility with
    /// the other repositories on `AppState`; this repo deliberately does not
    /// store it (see module docs — all queries run on a context-set connection
    /// supplied by the handler's `RlsConnection`).
    pub fn new(_pool: PgPool) -> Self {
        Self
    }

    // ==================== Building Certifications ====================

    /// Create a new building certification.
    pub async fn create_certification<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        input: CreateBuildingCertification,
        user_id: Option<Uuid>,
    ) -> Result<BuildingCertification, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let percentage = if let (Some(achieved), Some(possible)) =
            (input.total_points_achieved, input.total_points_possible)
        {
            if possible > 0 {
                Some(
                    rust_decimal::Decimal::from(achieved * 100)
                        / rust_decimal::Decimal::from(possible),
                )
            } else {
                None
            }
        } else {
            None
        };

        sqlx::query_as::<_, BuildingCertification>(
            r#"
            INSERT INTO building_certifications (
                organization_id, building_id, program, version, level, status,
                total_points_possible, total_points_achieved, percentage_achieved,
                application_date, certification_date, expiration_date,
                certificate_number, project_id, assessor_name, assessor_organization,
                certificate_url, scorecard_url, notes,
                application_fee, certification_fee, annual_fee, created_by
            ) VALUES (
                $1, $2, $3, $4, $5, COALESCE($6, 'planning'),
                $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19,
                $20, $21, $22, $23
            )
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(input.building_id)
        .bind(input.program)
        .bind(&input.version)
        .bind(input.level)
        .bind(input.status)
        .bind(input.total_points_possible)
        .bind(input.total_points_achieved)
        .bind(percentage)
        .bind(input.application_date)
        .bind(input.certification_date)
        .bind(input.expiration_date)
        .bind(&input.certificate_number)
        .bind(&input.project_id)
        .bind(&input.assessor_name)
        .bind(&input.assessor_organization)
        .bind(&input.certificate_url)
        .bind(&input.scorecard_url)
        .bind(&input.notes)
        .bind(input.application_fee)
        .bind(input.certification_fee)
        .bind(input.annual_fee)
        .bind(user_id)
        .fetch_one(executor)
        .await
    }

    /// Get a certification by ID, scoped to the caller's organization.
    ///
    /// Returns `None` if the certification does not exist **or** belongs to a
    /// different organization, so cross-tenant by-id reads (IDOR) surface as a
    /// 404. Under FORCE-RLS the connection's org context is a second,
    /// database-enforced guard on top of the explicit predicate.
    pub async fn get_certification<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        cert_id: Uuid,
    ) -> Result<Option<BuildingCertification>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BuildingCertification>(
            r#"
            SELECT * FROM building_certifications
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(org_id)
        .bind(cert_id)
        .fetch_optional(executor)
        .await
    }

    /// List certifications with filters.
    pub async fn list_certifications<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        filters: CertificationFilters,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<BuildingCertification>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let mut query =
            String::from("SELECT * FROM building_certifications WHERE organization_id = $1");
        let mut param_count = 1;

        if filters.building_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND building_id = ${param_count}"));
        }
        if filters.program.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND program = ${param_count}"));
        }
        if filters.level.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND level = ${param_count}"));
        }
        if filters.status.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND status = ${param_count}"));
        }
        if filters.expiring_within_days.is_some() {
            param_count += 1;
            query.push_str(&format!(
                " AND expiration_date <= CURRENT_DATE + ${param_count}::INTEGER * INTERVAL '1 day'"
            ));
        }

        query.push_str(&format!(
            " ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            param_count + 1,
            param_count + 2
        ));

        let mut q =
            sqlx::query_as::<_, BuildingCertification>(sqlx::AssertSqlSafe(query)).bind(org_id);

        if let Some(building_id) = filters.building_id {
            q = q.bind(building_id);
        }
        if let Some(program) = filters.program {
            q = q.bind(program);
        }
        if let Some(level) = filters.level {
            q = q.bind(level);
        }
        if let Some(status) = filters.status {
            q = q.bind(status);
        }
        if let Some(days) = filters.expiring_within_days {
            q = q.bind(days);
        }

        q.bind(limit).bind(offset).fetch_all(executor).await
    }

    /// Update a certification, scoped to the caller's organization.
    ///
    /// The `organization_id = $1` predicate means an update targeting another
    /// org's certification matches zero rows and surfaces as `None` (→ 404),
    /// not a silent cross-tenant write.
    pub async fn update_certification<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        cert_id: Uuid,
        input: UpdateBuildingCertification,
    ) -> Result<Option<BuildingCertification>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BuildingCertification>(
            r#"
            UPDATE building_certifications SET
                version = COALESCE($3, version),
                level = COALESCE($4, level),
                status = COALESCE($5, status),
                total_points_possible = COALESCE($6, total_points_possible),
                total_points_achieved = COALESCE($7, total_points_achieved),
                application_date = COALESCE($8, application_date),
                certification_date = COALESCE($9, certification_date),
                expiration_date = COALESCE($10, expiration_date),
                renewal_date = COALESCE($11, renewal_date),
                certificate_number = COALESCE($12, certificate_number),
                project_id = COALESCE($13, project_id),
                assessor_name = COALESCE($14, assessor_name),
                assessor_organization = COALESCE($15, assessor_organization),
                certificate_url = COALESCE($16, certificate_url),
                scorecard_url = COALESCE($17, scorecard_url),
                notes = COALESCE($18, notes),
                application_fee = COALESCE($19, application_fee),
                certification_fee = COALESCE($20, certification_fee),
                annual_fee = COALESCE($21, annual_fee),
                total_investment = COALESCE($22, total_investment),
                updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(cert_id)
        .bind(&input.version)
        .bind(input.level)
        .bind(input.status)
        .bind(input.total_points_possible)
        .bind(input.total_points_achieved)
        .bind(input.application_date)
        .bind(input.certification_date)
        .bind(input.expiration_date)
        .bind(input.renewal_date)
        .bind(&input.certificate_number)
        .bind(&input.project_id)
        .bind(&input.assessor_name)
        .bind(&input.assessor_organization)
        .bind(&input.certificate_url)
        .bind(&input.scorecard_url)
        .bind(&input.notes)
        .bind(input.application_fee)
        .bind(input.certification_fee)
        .bind(input.annual_fee)
        .bind(input.total_investment)
        .fetch_optional(executor)
        .await
    }

    /// Delete a certification, scoped to the caller's organization.
    pub async fn delete_certification<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        cert_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            DELETE FROM building_certifications
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(org_id)
        .bind(cert_id)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get certification with credits summary.
    ///
    /// Multi-statement (org-scoped fetch + aggregate), so it takes a connection
    /// and reborrows it per query.
    pub async fn get_certification_with_credits(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
        cert_id: Uuid,
    ) -> Result<Option<CertificationWithCredits>, sqlx::Error> {
        let cert = self.get_certification(&mut *conn, org_id, cert_id).await?;

        if let Some(certification) = cert {
            let credits_summary: (i64, i64, i64, i64) = sqlx::query_as(
                r#"
                SELECT
                    COUNT(*) FILTER (WHERE status = 'achieved') as credits_achieved,
                    COUNT(*) as credits_total,
                    COUNT(*) FILTER (WHERE is_prerequisite AND status = 'achieved') as prerequisites_met,
                    COUNT(*) FILTER (WHERE is_prerequisite) as prerequisites_total
                FROM certification_credits
                WHERE organization_id = $1 AND certification_id = $2
                "#,
            )
            .bind(org_id)
            .bind(cert_id)
            .fetch_one(&mut *conn)
            .await?;

            Ok(Some(CertificationWithCredits {
                certification,
                credits_achieved: credits_summary.0,
                credits_total: credits_summary.1,
                prerequisites_met: credits_summary.2,
                prerequisites_total: credits_summary.3,
            }))
        } else {
            Ok(None)
        }
    }

    // ==================== Certification Credits ====================

    /// Create a certification credit.
    pub async fn create_credit<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        input: CreateCertificationCredit,
    ) -> Result<CertificationCredit, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationCredit>(
            r#"
            INSERT INTO certification_credits (
                organization_id, certification_id, category, credit_code, credit_name,
                description, points_possible, points_achieved, is_prerequisite,
                status, documentation_status, evidence_urls, notes,
                responsible_user_id, due_date
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, COALESCE($8, 0), COALESCE($9, false),
                COALESCE($10, 'not_started'), COALESCE($11, 'missing'), $12, $13, $14, $15
            )
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(input.certification_id)
        .bind(input.category)
        .bind(&input.credit_code)
        .bind(&input.credit_name)
        .bind(&input.description)
        .bind(input.points_possible)
        .bind(input.points_achieved)
        .bind(input.is_prerequisite)
        .bind(&input.status)
        .bind(&input.documentation_status)
        .bind(
            serde_json::to_value(input.evidence_urls.unwrap_or_default())
                .unwrap_or(serde_json::Value::Array(vec![])),
        )
        .bind(&input.notes)
        .bind(input.responsible_user_id)
        .bind(input.due_date)
        .fetch_one(executor)
        .await
    }

    /// Get a credit by ID, scoped to the caller's organization.
    pub async fn get_credit<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        credit_id: Uuid,
    ) -> Result<Option<CertificationCredit>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationCredit>(
            r#"
            SELECT * FROM certification_credits
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(org_id)
        .bind(credit_id)
        .fetch_optional(executor)
        .await
    }

    /// List credits for a certification.
    pub async fn list_credits<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        cert_id: Uuid,
    ) -> Result<Vec<CertificationCredit>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationCredit>(
            r#"
            SELECT * FROM certification_credits
            WHERE organization_id = $1 AND certification_id = $2
            ORDER BY category, credit_code
            "#,
        )
        .bind(org_id)
        .bind(cert_id)
        .fetch_all(executor)
        .await
    }

    /// Update a credit, scoped to the caller's organization.
    pub async fn update_credit<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        credit_id: Uuid,
        input: UpdateCertificationCredit,
    ) -> Result<Option<CertificationCredit>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationCredit>(
            r#"
            UPDATE certification_credits SET
                category = COALESCE($3, category),
                credit_code = COALESCE($4, credit_code),
                credit_name = COALESCE($5, credit_name),
                description = COALESCE($6, description),
                points_possible = COALESCE($7, points_possible),
                points_achieved = COALESCE($8, points_achieved),
                is_prerequisite = COALESCE($9, is_prerequisite),
                status = COALESCE($10, status),
                documentation_status = COALESCE($11, documentation_status),
                evidence_urls = COALESCE($12, evidence_urls),
                notes = COALESCE($13, notes),
                responsible_user_id = COALESCE($14, responsible_user_id),
                due_date = COALESCE($15, due_date),
                updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(credit_id)
        .bind(input.category)
        .bind(&input.credit_code)
        .bind(&input.credit_name)
        .bind(&input.description)
        .bind(input.points_possible)
        .bind(input.points_achieved)
        .bind(input.is_prerequisite)
        .bind(&input.status)
        .bind(&input.documentation_status)
        .bind(
            input.evidence_urls.map(|urls| {
                serde_json::to_value(&urls).unwrap_or(serde_json::Value::Array(vec![]))
            }),
        )
        .bind(&input.notes)
        .bind(input.responsible_user_id)
        .bind(input.due_date)
        .fetch_optional(executor)
        .await
    }

    /// Delete a credit, scoped to the caller's organization.
    pub async fn delete_credit<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        credit_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            DELETE FROM certification_credits
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(org_id)
        .bind(credit_id)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== Certification Documents ====================

    /// Create a certification document.
    pub async fn create_document<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        input: CreateCertificationDocument,
        user_id: Option<Uuid>,
    ) -> Result<CertificationDocument, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationDocument>(
            r#"
            INSERT INTO certification_documents (
                organization_id, certification_id, credit_id, document_type,
                title, description, file_url, file_size_bytes, file_type,
                uploaded_by
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(input.certification_id)
        .bind(input.credit_id)
        .bind(&input.document_type)
        .bind(&input.title)
        .bind(&input.description)
        .bind(&input.file_url)
        .bind(input.file_size_bytes)
        .bind(&input.file_type)
        .bind(user_id)
        .fetch_one(executor)
        .await
    }

    /// List documents for a certification.
    pub async fn list_documents<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        cert_id: Uuid,
    ) -> Result<Vec<CertificationDocument>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationDocument>(
            r#"
            SELECT * FROM certification_documents
            WHERE organization_id = $1 AND certification_id = $2 AND is_current = true
            ORDER BY document_type, created_at DESC
            "#,
        )
        .bind(org_id)
        .bind(cert_id)
        .fetch_all(executor)
        .await
    }

    /// Delete a document, scoped to the caller's organization.
    pub async fn delete_document<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        doc_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            DELETE FROM certification_documents
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(org_id)
        .bind(doc_id)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== Certification Milestones ====================

    /// Create a certification milestone.
    pub async fn create_milestone<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        input: CreateCertificationMilestone,
    ) -> Result<CertificationMilestone, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationMilestone>(
            r#"
            INSERT INTO certification_milestones (
                organization_id, certification_id, milestone_name, description,
                phase, target_date, status, depends_on_milestone_id, assigned_to, notes
            ) VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, 'pending'), $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(input.certification_id)
        .bind(&input.milestone_name)
        .bind(&input.description)
        .bind(&input.phase)
        .bind(input.target_date)
        .bind(&input.status)
        .bind(input.depends_on_milestone_id)
        .bind(input.assigned_to)
        .bind(&input.notes)
        .fetch_one(executor)
        .await
    }

    /// List milestones for a certification.
    pub async fn list_milestones<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        cert_id: Uuid,
    ) -> Result<Vec<CertificationMilestone>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationMilestone>(
            r#"
            SELECT * FROM certification_milestones
            WHERE organization_id = $1 AND certification_id = $2
            ORDER BY target_date NULLS LAST, created_at
            "#,
        )
        .bind(org_id)
        .bind(cert_id)
        .fetch_all(executor)
        .await
    }

    /// Update a milestone, scoped to the caller's organization.
    pub async fn update_milestone<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        milestone_id: Uuid,
        input: UpdateCertificationMilestone,
    ) -> Result<Option<CertificationMilestone>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationMilestone>(
            r#"
            UPDATE certification_milestones SET
                milestone_name = COALESCE($3, milestone_name),
                description = COALESCE($4, description),
                phase = COALESCE($5, phase),
                target_date = COALESCE($6, target_date),
                actual_date = COALESCE($7, actual_date),
                status = COALESCE($8, status),
                depends_on_milestone_id = COALESCE($9, depends_on_milestone_id),
                assigned_to = COALESCE($10, assigned_to),
                notes = COALESCE($11, notes),
                updated_at = NOW()
            WHERE organization_id = $1 AND id = $2
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(milestone_id)
        .bind(&input.milestone_name)
        .bind(&input.description)
        .bind(&input.phase)
        .bind(input.target_date)
        .bind(input.actual_date)
        .bind(&input.status)
        .bind(input.depends_on_milestone_id)
        .bind(input.assigned_to)
        .bind(&input.notes)
        .fetch_optional(executor)
        .await
    }

    /// Delete a milestone, scoped to the caller's organization.
    pub async fn delete_milestone<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        milestone_id: Uuid,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query(
            r#"
            DELETE FROM certification_milestones
            WHERE organization_id = $1 AND id = $2
            "#,
        )
        .bind(org_id)
        .bind(milestone_id)
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get upcoming milestones across all certifications.
    pub async fn get_upcoming_milestones<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        days_ahead: i32,
    ) -> Result<Vec<CertificationMilestone>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationMilestone>(
            r#"
            SELECT * FROM certification_milestones
            WHERE organization_id = $1
              AND status IN ('pending', 'in_progress')
              AND target_date <= CURRENT_DATE + $2 * INTERVAL '1 day'
            ORDER BY target_date NULLS LAST
            "#,
        )
        .bind(org_id)
        .bind(days_ahead)
        .fetch_all(executor)
        .await
    }

    // ==================== Certification Benchmarks ====================

    /// Create a certification benchmark.
    pub async fn create_benchmark<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        input: CreateCertificationBenchmark,
    ) -> Result<CertificationBenchmark, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationBenchmark>(
            r#"
            INSERT INTO certification_benchmarks (
                organization_id, certification_id, metric_name, metric_unit,
                building_value, benchmark_25th_percentile, benchmark_50th_percentile,
                benchmark_75th_percentile, benchmark_source, percentile_rank,
                measurement_period_start, measurement_period_end
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(input.certification_id)
        .bind(&input.metric_name)
        .bind(&input.metric_unit)
        .bind(input.building_value)
        .bind(input.benchmark_25th_percentile)
        .bind(input.benchmark_50th_percentile)
        .bind(input.benchmark_75th_percentile)
        .bind(&input.benchmark_source)
        .bind(input.percentile_rank)
        .bind(input.measurement_period_start)
        .bind(input.measurement_period_end)
        .fetch_one(executor)
        .await
    }

    /// List benchmarks for a certification.
    pub async fn list_benchmarks<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        cert_id: Uuid,
    ) -> Result<Vec<CertificationBenchmark>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationBenchmark>(
            r#"
            SELECT * FROM certification_benchmarks
            WHERE organization_id = $1 AND certification_id = $2
            ORDER BY metric_name
            "#,
        )
        .bind(org_id)
        .bind(cert_id)
        .fetch_all(executor)
        .await
    }

    // ==================== Certification Costs ====================

    /// Create a certification cost.
    pub async fn create_cost<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        input: CreateCertificationCost,
        user_id: Option<Uuid>,
    ) -> Result<CertificationCost, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationCost>(
            r#"
            INSERT INTO certification_costs (
                organization_id, certification_id, cost_type, description,
                amount, currency, incurred_date, paid_date, invoice_number,
                vendor_name, vendor_id, created_by
            ) VALUES ($1, $2, $3, $4, $5, COALESCE($6, 'USD'), $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(input.certification_id)
        .bind(&input.cost_type)
        .bind(&input.description)
        .bind(input.amount)
        .bind(&input.currency)
        .bind(input.incurred_date)
        .bind(input.paid_date)
        .bind(&input.invoice_number)
        .bind(&input.vendor_name)
        .bind(input.vendor_id)
        .bind(user_id)
        .fetch_one(executor)
        .await
    }

    /// List costs for a certification.
    pub async fn list_costs<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        cert_id: Uuid,
    ) -> Result<Vec<CertificationCost>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationCost>(
            r#"
            SELECT * FROM certification_costs
            WHERE organization_id = $1 AND certification_id = $2
            ORDER BY incurred_date DESC NULLS LAST
            "#,
        )
        .bind(org_id)
        .bind(cert_id)
        .fetch_all(executor)
        .await
    }

    /// Get total costs for a certification.
    pub async fn get_total_costs<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        cert_id: Uuid,
    ) -> Result<rust_decimal::Decimal, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result: (rust_decimal::Decimal,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(amount), 0) as total
            FROM certification_costs
            WHERE organization_id = $1 AND certification_id = $2
            "#,
        )
        .bind(org_id)
        .bind(cert_id)
        .fetch_one(executor)
        .await?;

        Ok(result.0)
    }

    // ==================== Certification Reminders ====================

    /// Create a certification reminder.
    pub async fn create_reminder<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        input: CreateCertificationReminder,
    ) -> Result<CertificationReminder, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationReminder>(
            r#"
            INSERT INTO certification_reminders (
                organization_id, certification_id, reminder_type, days_before,
                message, notify_users, notify_roles
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(input.certification_id)
        .bind(&input.reminder_type)
        .bind(input.days_before)
        .bind(&input.message)
        .bind(
            serde_json::to_value(input.notify_users.unwrap_or_default())
                .unwrap_or(serde_json::Value::Array(vec![])),
        )
        .bind(
            serde_json::to_value(input.notify_roles.unwrap_or_default())
                .unwrap_or(serde_json::Value::Array(vec![])),
        )
        .fetch_one(executor)
        .await
    }

    /// List reminders for a certification.
    pub async fn list_reminders<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        cert_id: Uuid,
    ) -> Result<Vec<CertificationReminder>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationReminder>(
            r#"
            SELECT * FROM certification_reminders
            WHERE organization_id = $1 AND certification_id = $2
            ORDER BY reminder_type, days_before
            "#,
        )
        .bind(org_id)
        .bind(cert_id)
        .fetch_all(executor)
        .await
    }

    // ==================== Audit Logs ====================

    /// Create an audit log entry.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_audit_log<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        cert_id: Uuid,
        action: &str,
        entity_type: Option<&str>,
        entity_id: Option<Uuid>,
        previous_value: Option<serde_json::Value>,
        new_value: Option<serde_json::Value>,
        user_id: Option<Uuid>,
        notes: Option<&str>,
    ) -> Result<CertificationAuditLog, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationAuditLog>(
            r#"
            INSERT INTO certification_audit_logs (
                organization_id, certification_id, action, entity_type,
                entity_id, previous_value, new_value, performed_by, notes
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(cert_id)
        .bind(action)
        .bind(entity_type)
        .bind(entity_id)
        .bind(previous_value)
        .bind(new_value)
        .bind(user_id)
        .bind(notes)
        .fetch_one(executor)
        .await
    }

    /// Get audit logs for a certification.
    pub async fn list_audit_logs<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        cert_id: Uuid,
        limit: i64,
    ) -> Result<Vec<CertificationAuditLog>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, CertificationAuditLog>(
            r#"
            SELECT * FROM certification_audit_logs
            WHERE organization_id = $1 AND certification_id = $2
            ORDER BY performed_at DESC
            LIMIT $3
            "#,
        )
        .bind(org_id)
        .bind(cert_id)
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    // ==================== Dashboard ====================

    /// Get certification dashboard summary.
    ///
    /// Multi-statement (several aggregates + upcoming-milestones lookup), so it
    /// takes a connection and reborrows it per query.
    pub async fn get_dashboard(
        &self,
        conn: &mut PgConnection,
        org_id: Uuid,
    ) -> Result<CertificationDashboard, sqlx::Error> {
        // Get counts
        let counts: (i64, i64, i64, i64, rust_decimal::Decimal) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE status = 'achieved' OR status = 'renewed') as active,
                COUNT(*) FILTER (WHERE expiration_date <= CURRENT_DATE + INTERVAL '90 days' AND status IN ('achieved', 'renewed')) as expiring_soon,
                COUNT(*) FILTER (WHERE status IN ('planning', 'in_progress', 'under_review')) as in_progress,
                COALESCE(SUM(total_investment), 0) as total_investment
            FROM building_certifications
            WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(&mut *conn)
        .await?;

        // Get counts by program
        let by_program: Vec<(CertificationProgram, i64)> = sqlx::query_as(
            r#"
            SELECT program, COUNT(*) as count
            FROM building_certifications
            WHERE organization_id = $1
            GROUP BY program
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&mut *conn)
        .await?;

        // Get counts by level
        let by_level: Vec<(
            crate::models::building_certification::CertificationLevel,
            i64,
        )> = sqlx::query_as(
            r#"
            SELECT level, COUNT(*) as count
            FROM building_certifications
            WHERE organization_id = $1
            GROUP BY level
            ORDER BY count DESC
            "#,
        )
        .bind(org_id)
        .fetch_all(&mut *conn)
        .await?;

        // Get upcoming milestones
        let upcoming_milestones = self.get_upcoming_milestones(&mut *conn, org_id, 30).await?;

        Ok(CertificationDashboard {
            total_certifications: counts.0,
            active_certifications: counts.1,
            expiring_soon: counts.2,
            in_progress: counts.3,
            by_program: by_program
                .into_iter()
                .map(|(program, count)| CertificationProgramCount { program, count })
                .collect(),
            by_level: by_level
                .into_iter()
                .map(|(level, count)| CertificationLevelCount { level, count })
                .collect(),
            upcoming_milestones,
            total_investment: counts.4,
        })
    }

    /// Get certifications expiring soon.
    pub async fn get_expiring_certifications<'e, E>(
        &self,
        executor: E,
        org_id: Uuid,
        days_ahead: i32,
    ) -> Result<Vec<BuildingCertification>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, BuildingCertification>(
            r#"
            SELECT * FROM building_certifications
            WHERE organization_id = $1
              AND status IN ('achieved', 'renewed')
              AND expiration_date IS NOT NULL
              AND expiration_date <= CURRENT_DATE + $2 * INTERVAL '1 day'
            ORDER BY expiration_date
            "#,
        )
        .bind(org_id)
        .bind(days_ahead)
        .fetch_all(executor)
        .await
    }
}
