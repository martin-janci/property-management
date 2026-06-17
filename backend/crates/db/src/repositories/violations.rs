//! Violation Tracking & Enforcement repository for Epic 142.

use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::violations::{
    AppealQuery, AppealStatus, CategoryCount, CommunityRule, CreateCommunityRule,
    CreateEnforcementAction, CreateViolation, CreateViolationAppeal, CreateViolationComment,
    CreateViolationEvidence, EnforcementAction, EnforcementQuery, FinePayment, RecordFinePayment,
    StatusCount, UpdateCommunityRule, UpdateEnforcementAction, UpdateViolation,
    UpdateViolationAppeal, Violation, ViolationAppeal, ViolationComment, ViolationDashboard,
    ViolationEvidence, ViolationNotification, ViolationQuery, ViolationStatistics, ViolationStatus,
    ViolationSummary, ViolatorHistory,
};

/// Repository for violation tracking operations.
#[derive(Clone)]
pub struct ViolationRepository {
    pool: PgPool,
}

impl ViolationRepository {
    /// Create a new repository instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    // COMMUNITY RULES
    // =========================================================================

    /// Create a new community rule.
    pub async fn create_rule(
        &self,
        org_id: Uuid,
        req: CreateCommunityRule,
        created_by: Uuid,
    ) -> Result<CommunityRule, sqlx::Error> {
        sqlx::query_as::<_, CommunityRule>(
            r#"
            INSERT INTO community_rules (
                organization_id, building_id, rule_code, title, description,
                category, first_offense_fine, second_offense_fine, third_offense_fine,
                max_fine, fine_escalation_days, effective_date, expiry_date,
                requires_board_approval, source_document_id, section_reference, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, COALESCE($12, CURRENT_DATE), $13, $14, $15, $16, $17)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(req.building_id)
        .bind(&req.rule_code)
        .bind(&req.title)
        .bind(&req.description)
        .bind(req.category)
        .bind(req.first_offense_fine)
        .bind(req.second_offense_fine)
        .bind(req.third_offense_fine)
        .bind(req.max_fine)
        .bind(req.fine_escalation_days)
        .bind(req.effective_date)
        .bind(req.expiry_date)
        .bind(req.requires_board_approval.unwrap_or(false))
        .bind(req.source_document_id)
        .bind(&req.section_reference)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a community rule by ID.
    /// Get a single community rule scoped to an organization (tenant-safe read).
    pub async fn get_rule_for_org(
        &self,
        rule_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<CommunityRule>, sqlx::Error> {
        sqlx::query_as::<_, CommunityRule>(
            "SELECT * FROM community_rules WHERE id = $1 AND organization_id = $2",
        )
        .bind(rule_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List community rules for an organization.
    pub async fn list_rules(
        &self,
        org_id: Uuid,
        building_id: Option<Uuid>,
        active_only: bool,
    ) -> Result<Vec<CommunityRule>, sqlx::Error> {
        let mut query = String::from("SELECT * FROM community_rules WHERE organization_id = $1");

        if building_id.is_some() {
            query.push_str(" AND (building_id = $2 OR building_id IS NULL)");
        }

        if active_only {
            query.push_str(" AND is_active = true");
        }

        query.push_str(" ORDER BY category, rule_code");

        if building_id.is_some() {
            sqlx::query_as::<_, CommunityRule>(sqlx::AssertSqlSafe(query))
                .bind(org_id)
                .bind(building_id)
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query_as::<_, CommunityRule>(sqlx::AssertSqlSafe(query))
                .bind(org_id)
                .fetch_all(&self.pool)
                .await
        }
    }

    /// Update a community rule (tenant-scoped).
    pub async fn update_rule(
        &self,
        rule_id: Uuid,
        org_id: Uuid,
        req: UpdateCommunityRule,
    ) -> Result<CommunityRule, sqlx::Error> {
        sqlx::query_as::<_, CommunityRule>(
            r#"
            UPDATE community_rules SET
                title = COALESCE($3, title),
                description = COALESCE($4, description),
                first_offense_fine = COALESCE($5, first_offense_fine),
                second_offense_fine = COALESCE($6, second_offense_fine),
                third_offense_fine = COALESCE($7, third_offense_fine),
                max_fine = COALESCE($8, max_fine),
                fine_escalation_days = COALESCE($9, fine_escalation_days),
                expiry_date = COALESCE($10, expiry_date),
                is_active = COALESCE($11, is_active),
                requires_board_approval = COALESCE($12, requires_board_approval),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(rule_id)
        .bind(org_id)
        .bind(&req.title)
        .bind(&req.description)
        .bind(req.first_offense_fine)
        .bind(req.second_offense_fine)
        .bind(req.third_offense_fine)
        .bind(req.max_fine)
        .bind(req.fine_escalation_days)
        .bind(req.expiry_date)
        .bind(req.is_active)
        .bind(req.requires_board_approval)
        .fetch_one(&self.pool)
        .await
    }

    /// Delete a community rule (tenant-scoped).
    pub async fn delete_rule(&self, rule_id: Uuid, org_id: Uuid) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM community_rules WHERE id = $1 AND organization_id = $2")
                .bind(rule_id)
                .bind(org_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    // =========================================================================
    // VIOLATIONS
    // =========================================================================

    /// Create a new violation.
    pub async fn create_violation(
        &self,
        org_id: Uuid,
        req: CreateViolation,
        reporter_id: Uuid,
    ) -> Result<Violation, sqlx::Error> {
        // Generate violation number
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM violations WHERE organization_id = $1 AND EXTRACT(YEAR FROM created_at) = EXTRACT(YEAR FROM NOW())",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let year = Utc::now().format("%Y");
        let violation_number = format!("VIO-{}-{:04}", year, count.0 + 1);

        // Check for previous violations by same violator
        let offense_number = if let Some(violator_id) = req.violator_id {
            let prev: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM violations WHERE organization_id = $1 AND violator_id = $2 AND status NOT IN ('dismissed')",
            )
            .bind(org_id)
            .bind(violator_id)
            .fetch_one(&self.pool)
            .await?;
            (prev.0 + 1) as i32
        } else {
            1
        };

        sqlx::query_as::<_, Violation>(
            r#"
            INSERT INTO violations (
                organization_id, building_id, unit_id, violation_number, rule_id,
                category, severity, status, title, description, location,
                violator_id, violator_name, violator_unit, reporter_id,
                occurred_at, evidence_description, witness_count, offense_number
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'reported', $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(req.building_id)
        .bind(req.unit_id)
        .bind(&violation_number)
        .bind(req.rule_id)
        .bind(req.category)
        .bind(req.severity.unwrap_or(crate::models::violations::ViolationSeverity::Minor))
        .bind(&req.title)
        .bind(&req.description)
        .bind(&req.location)
        .bind(req.violator_id)
        .bind(&req.violator_name)
        .bind(&req.violator_unit)
        .bind(reporter_id)
        .bind(req.occurred_at)
        .bind(&req.evidence_description)
        .bind(req.witness_count.unwrap_or(0))
        .bind(offense_number)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a violation by ID.
    /// Get a single violation scoped to an organization (tenant-safe read).
    pub async fn get_violation_for_org(
        &self,
        violation_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<Violation>, sqlx::Error> {
        sqlx::query_as::<_, Violation>(
            "SELECT * FROM violations WHERE id = $1 AND organization_id = $2",
        )
        .bind(violation_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List violations with filters.
    pub async fn list_violations(
        &self,
        org_id: Uuid,
        query: ViolationQuery,
    ) -> Result<Vec<Violation>, sqlx::Error> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, Violation>(
            r#"
            SELECT * FROM violations
            WHERE organization_id = $1
                AND ($2::uuid IS NULL OR building_id = $2)
                AND ($3::uuid IS NULL OR unit_id = $3)
                AND ($4::violation_category IS NULL OR category = $4)
                AND ($5::violation_severity IS NULL OR severity = $5)
                AND ($6::violation_status IS NULL OR status = $6)
                AND ($7::uuid IS NULL OR violator_id = $7)
                AND ($8::uuid IS NULL OR assigned_to = $8)
                AND ($9::timestamptz IS NULL OR occurred_at >= $9)
                AND ($10::timestamptz IS NULL OR occurred_at <= $10)
            ORDER BY occurred_at DESC
            LIMIT $11 OFFSET $12
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(query.unit_id)
        .bind(query.category)
        .bind(query.severity)
        .bind(query.status)
        .bind(query.violator_id)
        .bind(query.assigned_to)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Get violation summaries for list views.
    pub async fn list_violation_summaries(
        &self,
        org_id: Uuid,
        query: ViolationQuery,
    ) -> Result<Vec<ViolationSummary>, sqlx::Error> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, ViolationSummary>(
            r#"
            SELECT
                v.id,
                v.violation_number,
                v.title,
                v.category,
                v.severity,
                v.status,
                v.violator_name,
                v.violator_unit,
                v.occurred_at,
                EXISTS(SELECT 1 FROM enforcement_actions ea WHERE ea.violation_id = v.id AND ea.fine_amount > 0) as has_fine,
                (SELECT SUM(ea.fine_amount) FROM enforcement_actions ea WHERE ea.violation_id = v.id) as fine_amount
            FROM violations v
            WHERE v.organization_id = $1
                AND ($2::uuid IS NULL OR v.building_id = $2)
                AND ($3::violation_status IS NULL OR v.status = $3)
            ORDER BY v.occurred_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(query.status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a violation (tenant-scoped).
    pub async fn update_violation(
        &self,
        violation_id: Uuid,
        org_id: Uuid,
        req: UpdateViolation,
    ) -> Result<Violation, sqlx::Error> {
        let now = if req.status == Some(ViolationStatus::Resolved) {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query_as::<_, Violation>(
            r#"
            UPDATE violations SET
                severity = COALESCE($3, severity),
                status = COALESCE($4, status),
                assigned_to = COALESCE($5, assigned_to),
                resolution_notes = COALESCE($6, resolution_notes),
                resolution_type = COALESCE($7, resolution_type),
                resolved_at = COALESCE($8, resolved_at),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(violation_id)
        .bind(org_id)
        .bind(req.severity)
        .bind(req.status)
        .bind(req.assigned_to)
        .bind(&req.resolution_notes)
        .bind(&req.resolution_type)
        .bind(now)
        .fetch_one(&self.pool)
        .await
    }

    /// Assign a violation to staff (tenant-scoped).
    pub async fn assign_violation(
        &self,
        violation_id: Uuid,
        org_id: Uuid,
        assigned_to: Uuid,
    ) -> Result<Violation, sqlx::Error> {
        sqlx::query_as::<_, Violation>(
            r#"
            UPDATE violations SET
                assigned_to = $3,
                status = CASE WHEN status = 'reported' THEN 'under_review' ELSE status END,
                reviewed_at = CASE WHEN reviewed_at IS NULL THEN NOW() ELSE reviewed_at END,
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(violation_id)
        .bind(org_id)
        .bind(assigned_to)
        .fetch_one(&self.pool)
        .await
    }

    // =========================================================================
    // EVIDENCE
    // =========================================================================

    /// Add evidence to a violation (tenant-scoped via parent violation).
    ///
    /// The INSERT is gated on the parent `violations` row matching `org_id`, so
    /// an Org B caller cannot attach evidence to an Org A violation. If no parent
    /// row matches, no row is inserted and `RowNotFound` is returned.
    pub async fn add_evidence(
        &self,
        violation_id: Uuid,
        org_id: Uuid,
        req: CreateViolationEvidence,
        uploaded_by: Uuid,
    ) -> Result<ViolationEvidence, sqlx::Error> {
        sqlx::query_as::<_, ViolationEvidence>(
            r#"
            INSERT INTO violation_evidence (
                violation_id, file_name, file_type, file_size, storage_path,
                description, captured_at, uploaded_by
            )
            SELECT v.id, $3, $4, $5, $6, $7, $8, $9
            FROM violations v
            WHERE v.id = $1 AND v.organization_id = $2
            RETURNING *
            "#,
        )
        .bind(violation_id)
        .bind(org_id)
        .bind(&req.file_name)
        .bind(&req.file_type)
        .bind(req.file_size)
        .bind(&req.storage_path)
        .bind(&req.description)
        .bind(req.captured_at)
        .bind(uploaded_by)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
    }

    /// List evidence for a violation.
    pub async fn list_evidence(
        &self,
        violation_id: Uuid,
    ) -> Result<Vec<ViolationEvidence>, sqlx::Error> {
        sqlx::query_as::<_, ViolationEvidence>(
            "SELECT * FROM violation_evidence WHERE violation_id = $1 ORDER BY created_at",
        )
        .bind(violation_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Delete evidence (tenant-scoped via parent violation).
    pub async fn delete_evidence(
        &self,
        evidence_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM violation_evidence ve
            WHERE ve.id = $1
              AND EXISTS (
                  SELECT 1 FROM violations v
                  WHERE v.id = ve.violation_id AND v.organization_id = $2
              )
            "#,
        )
        .bind(evidence_id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    // =========================================================================
    // ENFORCEMENT ACTIONS
    // =========================================================================

    /// Create an enforcement action.
    pub async fn create_enforcement_action(
        &self,
        violation_id: Uuid,
        org_id: Uuid,
        req: CreateEnforcementAction,
        issued_by: Uuid,
    ) -> Result<EnforcementAction, sqlx::Error> {
        sqlx::query_as::<_, EnforcementAction>(
            r#"
            INSERT INTO enforcement_actions (
                violation_id, organization_id, action_type, status,
                fine_amount, due_date, description, notes,
                suspended_privileges, suspension_start, suspension_end, issued_by
            )
            VALUES ($1, $2, $3, 'pending', $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(violation_id)
        .bind(org_id)
        .bind(req.action_type)
        .bind(req.fine_amount)
        .bind(req.due_date)
        .bind(&req.description)
        .bind(&req.notes)
        .bind(&req.suspended_privileges)
        .bind(req.suspension_start)
        .bind(req.suspension_end)
        .bind(issued_by)
        .fetch_one(&self.pool)
        .await
    }

    /// Get an enforcement action by ID.
    /// Get a single enforcement action scoped to an organization (tenant-safe read).
    pub async fn get_enforcement_action_for_org(
        &self,
        action_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<EnforcementAction>, sqlx::Error> {
        sqlx::query_as::<_, EnforcementAction>(
            "SELECT * FROM enforcement_actions WHERE id = $1 AND organization_id = $2",
        )
        .bind(action_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List enforcement actions.
    pub async fn list_enforcement_actions(
        &self,
        org_id: Uuid,
        query: EnforcementQuery,
    ) -> Result<Vec<EnforcementAction>, sqlx::Error> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, EnforcementAction>(
            r#"
            SELECT * FROM enforcement_actions
            WHERE organization_id = $1
                AND ($2::uuid IS NULL OR violation_id = $2)
                AND ($3::enforcement_action_type IS NULL OR action_type = $3)
                AND ($4::enforcement_status IS NULL OR status = $4)
                AND ($5::bool IS NOT TRUE OR (due_date < CURRENT_DATE AND status = 'sent'))
            ORDER BY created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(org_id)
        .bind(query.violation_id)
        .bind(query.action_type)
        .bind(query.status)
        .bind(query.overdue_only)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Update an enforcement action (tenant-scoped).
    pub async fn update_enforcement_action(
        &self,
        action_id: Uuid,
        org_id: Uuid,
        req: UpdateEnforcementAction,
    ) -> Result<EnforcementAction, sqlx::Error> {
        sqlx::query_as::<_, EnforcementAction>(
            r#"
            UPDATE enforcement_actions SET
                status = COALESCE($3, status),
                notice_sent_at = COALESCE($4, notice_sent_at),
                notice_method = COALESCE($5, notice_method),
                notes = COALESCE($6, notes),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(action_id)
        .bind(org_id)
        .bind(req.status)
        .bind(req.notice_sent_at)
        .bind(&req.notice_method)
        .bind(&req.notes)
        .fetch_one(&self.pool)
        .await
    }

    /// Mark enforcement action as sent (tenant-scoped).
    pub async fn mark_action_sent(
        &self,
        action_id: Uuid,
        org_id: Uuid,
        method: &str,
    ) -> Result<EnforcementAction, sqlx::Error> {
        sqlx::query_as::<_, EnforcementAction>(
            r#"
            UPDATE enforcement_actions SET
                status = 'sent',
                notice_sent_at = NOW(),
                notice_method = $3,
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(action_id)
        .bind(org_id)
        .bind(method)
        .fetch_one(&self.pool)
        .await
    }

    // =========================================================================
    // APPEALS
    // =========================================================================

    /// Create an appeal.
    pub async fn create_appeal(
        &self,
        violation_id: Uuid,
        org_id: Uuid,
        req: CreateViolationAppeal,
        appellant_id: Uuid,
    ) -> Result<ViolationAppeal, sqlx::Error> {
        // Generate appeal number
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM violation_appeals WHERE organization_id = $1 AND EXTRACT(YEAR FROM submitted_at) = EXTRACT(YEAR FROM NOW())",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let year = Utc::now().format("%Y");
        let appeal_number = format!("APP-{}-{:04}", year, count.0 + 1);

        // Update violation status to disputed — org-scoped. If the violation
        // does not belong to this org, no row is updated; bail out *before*
        // inserting the appeal so an Org B caller cannot cascade onto Org A.
        let vio_updated = sqlx::query(
            "UPDATE violations SET status = 'disputed', updated_at = NOW() WHERE id = $1 AND organization_id = $2",
        )
        .bind(violation_id)
        .bind(org_id)
        .execute(&self.pool)
        .await?;
        if vio_updated.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }

        // Update enforcement action status if specified — scoped on org and on
        // the parent violation so the cascade can only touch this org's action.
        if let Some(action_id) = req.enforcement_action_id {
            sqlx::query(
                "UPDATE enforcement_actions SET status = 'appealed', updated_at = NOW() WHERE id = $1 AND organization_id = $2 AND violation_id = $3",
            )
            .bind(action_id)
            .bind(org_id)
            .bind(violation_id)
            .execute(&self.pool)
            .await?;
        }

        sqlx::query_as::<_, ViolationAppeal>(
            r#"
            INSERT INTO violation_appeals (
                violation_id, enforcement_action_id, organization_id, appeal_number,
                status, reason, requested_outcome, supporting_evidence, appellant_id
            )
            VALUES ($1, $2, $3, $4, 'submitted', $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(violation_id)
        .bind(req.enforcement_action_id)
        .bind(org_id)
        .bind(&appeal_number)
        .bind(&req.reason)
        .bind(&req.requested_outcome)
        .bind(&req.supporting_evidence)
        .bind(appellant_id)
        .fetch_one(&self.pool)
        .await
    }

    /// Get a single appeal scoped to an organization (tenant-safe read).
    pub async fn get_appeal_for_org(
        &self,
        appeal_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<ViolationAppeal>, sqlx::Error> {
        sqlx::query_as::<_, ViolationAppeal>(
            "SELECT * FROM violation_appeals WHERE id = $1 AND organization_id = $2",
        )
        .bind(appeal_id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List appeals.
    pub async fn list_appeals(
        &self,
        org_id: Uuid,
        query: AppealQuery,
    ) -> Result<Vec<ViolationAppeal>, sqlx::Error> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, ViolationAppeal>(
            r#"
            SELECT * FROM violation_appeals
            WHERE organization_id = $1
                AND ($2::appeal_status IS NULL OR status = $2)
                AND ($3::uuid IS NULL OR appellant_id = $3)
            ORDER BY submitted_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(org_id)
        .bind(query.status)
        .bind(query.appellant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Update an appeal (tenant-scoped).
    pub async fn update_appeal(
        &self,
        appeal_id: Uuid,
        org_id: Uuid,
        req: UpdateViolationAppeal,
        decided_by: Option<Uuid>,
    ) -> Result<ViolationAppeal, sqlx::Error> {
        let decision_date = if req.decision.is_some() {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query_as::<_, ViolationAppeal>(
            r#"
            UPDATE violation_appeals SET
                status = COALESCE($3, status),
                hearing_date = COALESCE($4, hearing_date),
                hearing_location = COALESCE($5, hearing_location),
                hearing_notes = COALESCE($6, hearing_notes),
                decision = COALESCE($7, decision),
                decision_date = COALESCE($8, decision_date),
                decided_by = COALESCE($9, decided_by),
                fine_adjustment = COALESCE($10, fine_adjustment),
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(appeal_id)
        .bind(org_id)
        .bind(req.status)
        .bind(req.hearing_date)
        .bind(&req.hearing_location)
        .bind(&req.hearing_notes)
        .bind(&req.decision)
        .bind(decision_date)
        .bind(decided_by)
        .bind(req.fine_adjustment)
        .fetch_one(&self.pool)
        .await
    }

    /// Decide on an appeal (tenant-scoped). The lookup and every cascade write
    /// are keyed by `org_id` so a caller cannot decide another tenant's appeal
    /// or mutate its violation / enforcement-action records.
    pub async fn decide_appeal(
        &self,
        appeal_id: Uuid,
        org_id: Uuid,
        approved: bool,
        decision: &str,
        fine_adjustment: Option<Decimal>,
        decided_by: Uuid,
    ) -> Result<ViolationAppeal, sqlx::Error> {
        let status = if approved {
            AppealStatus::Approved
        } else {
            AppealStatus::Denied
        };

        // Get the appeal (org-scoped) to update related records
        let appeal = self
            .get_appeal_for_org(appeal_id, org_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        // If approved, update violation status
        if approved {
            sqlx::query("UPDATE violations SET status = 'dismissed', resolution_notes = $2, updated_at = NOW() WHERE id = $1 AND organization_id = $3")
                .bind(appeal.violation_id)
                .bind(format!("Appeal approved: {}", decision))
                .bind(org_id)
                .execute(&self.pool)
                .await?;
        } else {
            // If denied, restore violation to confirmed
            sqlx::query(
                "UPDATE violations SET status = 'confirmed', updated_at = NOW() WHERE id = $1 AND organization_id = $2",
            )
            .bind(appeal.violation_id)
            .bind(org_id)
            .execute(&self.pool)
            .await?;
        }

        // Update enforcement action if there's a fine adjustment
        if let (Some(action_id), Some(adjustment)) = (appeal.enforcement_action_id, fine_adjustment)
        {
            sqlx::query(
                "UPDATE enforcement_actions SET fine_amount = GREATEST(0, fine_amount - $2), updated_at = NOW() WHERE id = $1 AND organization_id = $3",
            )
            .bind(action_id)
            .bind(adjustment)
            .bind(org_id)
            .execute(&self.pool)
            .await?;
        }

        sqlx::query_as::<_, ViolationAppeal>(
            r#"
            UPDATE violation_appeals SET
                status = $3,
                decision = $4,
                decision_date = NOW(),
                decided_by = $5,
                fine_adjustment = $6,
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $2
            RETURNING *
            "#,
        )
        .bind(appeal_id)
        .bind(org_id)
        .bind(status)
        .bind(decision)
        .bind(decided_by)
        .bind(fine_adjustment)
        .fetch_one(&self.pool)
        .await
    }

    // =========================================================================
    // COMMENTS
    // =========================================================================

    /// Add a comment to a violation (tenant-scoped via parent violation).
    ///
    /// The INSERT is gated on the parent `violations` row matching `org_id`, so
    /// an Org B caller cannot inject a comment (including `is_internal` notes)
    /// onto an Org A violation. If no parent row matches, no row is inserted and
    /// `RowNotFound` is returned.
    pub async fn add_comment(
        &self,
        violation_id: Uuid,
        org_id: Uuid,
        req: CreateViolationComment,
        author_id: Uuid,
    ) -> Result<ViolationComment, sqlx::Error> {
        sqlx::query_as::<_, ViolationComment>(
            r#"
            INSERT INTO violation_comments (violation_id, comment_type, content, is_internal, author_id)
            SELECT v.id, $3, $4, $5, $6
            FROM violations v
            WHERE v.id = $1 AND v.organization_id = $2
            RETURNING *
            "#,
        )
        .bind(violation_id)
        .bind(org_id)
        .bind(&req.comment_type)
        .bind(&req.content)
        .bind(req.is_internal.unwrap_or(false))
        .bind(author_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(sqlx::Error::RowNotFound)
    }

    /// List comments for a violation.
    pub async fn list_comments(
        &self,
        violation_id: Uuid,
        include_internal: bool,
    ) -> Result<Vec<ViolationComment>, sqlx::Error> {
        if include_internal {
            sqlx::query_as::<_, ViolationComment>(
                "SELECT * FROM violation_comments WHERE violation_id = $1 ORDER BY created_at",
            )
            .bind(violation_id)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, ViolationComment>(
                "SELECT * FROM violation_comments WHERE violation_id = $1 AND is_internal = false ORDER BY created_at",
            )
            .bind(violation_id)
            .fetch_all(&self.pool)
            .await
        }
    }

    // =========================================================================
    // PAYMENTS
    // =========================================================================

    /// Record a fine payment.
    pub async fn record_payment(
        &self,
        action_id: Uuid,
        org_id: Uuid,
        req: RecordFinePayment,
        recorded_by: Uuid,
    ) -> Result<FinePayment, sqlx::Error> {
        // Verify the enforcement action belongs to this org before recording a
        // payment against it (prevents cross-tenant payment injection).
        let owns_action: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM enforcement_actions WHERE id = $1 AND organization_id = $2)",
        )
        .bind(action_id)
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;
        if !owns_action {
            return Err(sqlx::Error::RowNotFound);
        }

        // Record the payment
        let payment = sqlx::query_as::<_, FinePayment>(
            r#"
            INSERT INTO fine_payments (
                enforcement_action_id, organization_id, amount, payment_method,
                transaction_reference, payer_id, payer_name, status, notes, recorded_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'completed', $8, $9)
            RETURNING *
            "#,
        )
        .bind(action_id)
        .bind(org_id)
        .bind(req.amount)
        .bind(&req.payment_method)
        .bind(&req.transaction_reference)
        .bind(req.payer_id)
        .bind(&req.payer_name)
        .bind(&req.notes)
        .bind(recorded_by)
        .fetch_one(&self.pool)
        .await?;

        // Update enforcement action paid amount (tenant-scoped)
        sqlx::query(
            r#"
            UPDATE enforcement_actions SET
                paid_amount = COALESCE(paid_amount, 0) + $2,
                paid_at = CASE WHEN COALESCE(paid_amount, 0) + $2 >= fine_amount THEN NOW() ELSE paid_at END,
                status = CASE WHEN COALESCE(paid_amount, 0) + $2 >= fine_amount THEN 'paid' ELSE status END,
                updated_at = NOW()
            WHERE id = $1 AND organization_id = $3
            "#,
        )
        .bind(action_id)
        .bind(req.amount)
        .bind(org_id)
        .execute(&self.pool)
        .await?;

        Ok(payment)
    }

    /// List payments for an enforcement action.
    pub async fn list_payments(&self, action_id: Uuid) -> Result<Vec<FinePayment>, sqlx::Error> {
        sqlx::query_as::<_, FinePayment>(
            "SELECT * FROM fine_payments WHERE enforcement_action_id = $1 ORDER BY created_at",
        )
        .bind(action_id)
        .fetch_all(&self.pool)
        .await
    }

    // =========================================================================
    // NOTIFICATIONS
    // =========================================================================

    /// Record a notification sent.
    pub async fn record_notification(
        &self,
        violation_id: Uuid,
        notification_type: &str,
        recipient_id: Option<Uuid>,
        recipient_email: Option<&str>,
        subject: Option<&str>,
        body: Option<&str>,
    ) -> Result<ViolationNotification, sqlx::Error> {
        sqlx::query_as::<_, ViolationNotification>(
            r#"
            INSERT INTO violation_notifications (
                violation_id, notification_type, recipient_id, recipient_email, subject, body
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(violation_id)
        .bind(notification_type)
        .bind(recipient_id)
        .bind(recipient_email)
        .bind(subject)
        .bind(body)
        .fetch_one(&self.pool)
        .await
    }

    /// List notifications for a violation.
    pub async fn list_notifications(
        &self,
        violation_id: Uuid,
    ) -> Result<Vec<ViolationNotification>, sqlx::Error> {
        sqlx::query_as::<_, ViolationNotification>(
            "SELECT * FROM violation_notifications WHERE violation_id = $1 ORDER BY sent_at DESC",
        )
        .bind(violation_id)
        .fetch_all(&self.pool)
        .await
    }

    // =========================================================================
    // DASHBOARD & REPORTS
    // =========================================================================

    /// Get violation dashboard data.
    pub async fn get_dashboard(&self, org_id: Uuid) -> Result<ViolationDashboard, sqlx::Error> {
        // Get counts
        let total_open: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM violations WHERE organization_id = $1 AND status NOT IN ('resolved', 'dismissed')",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let reported_this_month: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM violations WHERE organization_id = $1 AND created_at >= date_trunc('month', NOW())",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        let resolved_this_month: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM violations WHERE organization_id = $1 AND resolved_at >= date_trunc('month', NOW())",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        // Get pending fines
        let pending_fines: (Option<Decimal>,) = sqlx::query_as(
            "SELECT SUM(fine_amount - COALESCE(paid_amount, 0)) FROM enforcement_actions WHERE organization_id = $1 AND status IN ('pending', 'sent', 'acknowledged')",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        // Get collected this month
        let collected_this_month: (Option<Decimal>,) = sqlx::query_as(
            "SELECT SUM(amount) FROM fine_payments WHERE organization_id = $1 AND created_at >= date_trunc('month', NOW())",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        // Get by category
        let by_category = sqlx::query_as::<_, CategoryCount>(
            "SELECT category, COUNT(*) as count FROM violations WHERE organization_id = $1 AND status NOT IN ('resolved', 'dismissed') GROUP BY category",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        // Get by status
        let by_status = sqlx::query_as::<_, StatusCount>(
            "SELECT status, COUNT(*) as count FROM violations WHERE organization_id = $1 GROUP BY status",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        // Get recent violations
        let recent_violations = self
            .list_violation_summaries(
                org_id,
                ViolationQuery {
                    limit: Some(10),
                    ..Default::default()
                },
            )
            .await?;

        Ok(ViolationDashboard {
            total_open: total_open.0 as i32,
            reported_this_month: reported_this_month.0 as i32,
            resolved_this_month: resolved_this_month.0 as i32,
            pending_fines: pending_fines.0.unwrap_or(Decimal::ZERO),
            collected_this_month: collected_this_month.0.unwrap_or(Decimal::ZERO),
            by_category,
            by_status,
            recent_violations,
        })
    }

    /// Get violator history.
    pub async fn get_violator_history(
        &self,
        org_id: Uuid,
        violator_id: Option<Uuid>,
        unit_id: Option<Uuid>,
    ) -> Result<ViolatorHistory, sqlx::Error> {
        // Get violations
        let violations = sqlx::query_as::<_, Violation>(
            "SELECT * FROM violations WHERE organization_id = $1 AND ($2::uuid IS NULL OR violator_id = $2) AND ($3::uuid IS NULL OR unit_id = $3) ORDER BY occurred_at DESC",
        )
        .bind(org_id)
        .bind(violator_id)
        .bind(unit_id)
        .fetch_all(&self.pool)
        .await?;

        let total_violations = violations.len() as i32;

        // Get by category
        let violations_by_category = sqlx::query_as::<_, CategoryCount>(
            "SELECT category, COUNT(*) as count FROM violations WHERE organization_id = $1 AND ($2::uuid IS NULL OR violator_id = $2) AND ($3::uuid IS NULL OR unit_id = $3) GROUP BY category",
        )
        .bind(org_id)
        .bind(violator_id)
        .bind(unit_id)
        .fetch_all(&self.pool)
        .await?;

        // Get fines
        let total_fines: (Option<Decimal>,) = sqlx::query_as(
            r#"
            SELECT SUM(ea.fine_amount)
            FROM enforcement_actions ea
            JOIN violations v ON ea.violation_id = v.id
            WHERE v.organization_id = $1
                AND ($2::uuid IS NULL OR v.violator_id = $2)
                AND ($3::uuid IS NULL OR v.unit_id = $3)
            "#,
        )
        .bind(org_id)
        .bind(violator_id)
        .bind(unit_id)
        .fetch_one(&self.pool)
        .await?;

        let outstanding_fines: (Option<Decimal>,) = sqlx::query_as(
            r#"
            SELECT SUM(ea.fine_amount - COALESCE(ea.paid_amount, 0))
            FROM enforcement_actions ea
            JOIN violations v ON ea.violation_id = v.id
            WHERE v.organization_id = $1
                AND ($2::uuid IS NULL OR v.violator_id = $2)
                AND ($3::uuid IS NULL OR v.unit_id = $3)
                AND ea.status NOT IN ('paid', 'cancelled')
            "#,
        )
        .bind(org_id)
        .bind(violator_id)
        .bind(unit_id)
        .fetch_one(&self.pool)
        .await?;

        // Get recent as summaries
        let recent_violations = violations
            .iter()
            .take(10)
            .map(|v| ViolationSummary {
                id: v.id,
                violation_number: v.violation_number.clone(),
                title: v.title.clone(),
                category: v.category,
                severity: v.severity,
                status: v.status,
                violator_name: v.violator_name.clone(),
                violator_unit: v.violator_unit.clone(),
                occurred_at: v.occurred_at,
                has_fine: false,
                fine_amount: None,
            })
            .collect();

        let violator_name = violations.first().and_then(|v| v.violator_name.clone());

        Ok(ViolatorHistory {
            violator_id,
            violator_name,
            unit_id,
            total_violations,
            violations_by_category,
            total_fines: total_fines.0.unwrap_or(Decimal::ZERO),
            outstanding_fines: outstanding_fines.0.unwrap_or(Decimal::ZERO),
            recent_violations,
        })
    }

    /// Get violation statistics.
    pub async fn get_statistics(
        &self,
        org_id: Uuid,
        building_id: Option<Uuid>,
        period_type: &str,
    ) -> Result<Option<ViolationStatistics>, sqlx::Error> {
        sqlx::query_as::<_, ViolationStatistics>(
            r#"
            SELECT * FROM violation_statistics
            WHERE organization_id = $1
                AND ($2::uuid IS NULL OR building_id = $2)
                AND period_type = $3
            ORDER BY period_start DESC
            LIMIT 1
            "#,
        )
        .bind(org_id)
        .bind(building_id)
        .bind(period_type)
        .fetch_optional(&self.pool)
        .await
    }
}
