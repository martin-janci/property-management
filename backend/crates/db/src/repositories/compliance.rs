//! Compliance Repository (Epic 67).
//!
//! Handles DSA transparency reports, content moderation cases,
//! and moderation action templates.

use crate::models::compliance::{
    CreateModerationCase, DsaReportStatus, DsaReportSummary, DsaTransparencyReport,
    ModeratedContentType, ModerationActionTemplate, ModerationActionType, ModerationCase,
    ModerationStatus, TakeModerationAction, ViolationType,
};
use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for DSA compliance and content moderation operations.
#[derive(Clone)]
pub struct ComplianceRepository {
    pool: DbPool,
}

impl ComplianceRepository {
    /// Create a new ComplianceRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // DSA TRANSPARENCY REPORTS
    // ========================================================================

    /// Create a new DSA transparency report.
    pub async fn create_dsa_report(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        generated_by: Uuid,
    ) -> Result<DsaTransparencyReport, SqlxError> {
        // Calculate statistics from moderation_cases for the period
        let stats = self
            .calculate_report_stats(period_start, period_end)
            .await?;

        let report = sqlx::query_as::<_, DsaTransparencyReport>(
            r#"
            INSERT INTO dsa_transparency_reports (
                period_start, period_end, status,
                total_moderation_actions, content_removed_count, content_restricted_count,
                warnings_issued_count, user_reports_received, user_reports_resolved,
                avg_resolution_time_hours, automated_decisions_count, automated_decisions_overturned,
                appeals_received, appeals_upheld, appeals_rejected,
                content_type_breakdown, violation_type_breakdown,
                generated_at, generated_by
            )
            VALUES ($1, $2, 'generated', $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, NOW(), $17)
            RETURNING *
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .bind(stats.total_moderation_actions)
        .bind(stats.content_removed)
        .bind(stats.content_restricted)
        .bind(stats.warnings_issued)
        .bind(stats.user_reports_received)
        .bind(stats.user_reports_resolved)
        .bind(stats.avg_resolution_time_hours)
        .bind(stats.automated_decisions)
        .bind(stats.automated_decisions_overturned)
        .bind(stats.appeals_received)
        .bind(stats.appeals_upheld)
        .bind(stats.appeals_rejected)
        .bind(serde_json::Value::Null) // content_type_breakdown - calculated separately
        .bind(serde_json::Value::Null) // violation_type_breakdown - calculated separately
        .bind(generated_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(report)
    }

    /// Calculate report statistics for a period.
    async fn calculate_report_stats(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<DsaReportSummary, SqlxError> {
        // Total moderation actions (cases with a decision)
        let (total_moderation_actions,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE decided_at IS NOT NULL
                AND decided_at >= $1 AND decided_at < $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        // Content removed
        let (content_removed,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE decision = 'remove'
                AND decided_at >= $1 AND decided_at < $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        // Content restricted
        let (content_restricted,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE decision = 'restrict'
                AND decided_at >= $1 AND decided_at < $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        // Warnings issued
        let (warnings_issued,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE decision = 'warn'
                AND decided_at >= $1 AND decided_at < $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        // User reports received
        let (user_reports_received,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE report_source = 'user'
                AND created_at >= $1 AND created_at < $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        // User reports resolved
        let (user_reports_resolved,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE report_source = 'user'
                AND decided_at IS NOT NULL
                AND created_at >= $1 AND created_at < $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        // Average resolution time
        let avg_resolution_time_hours: Option<f64> = sqlx::query_scalar(
            r#"
            SELECT AVG(EXTRACT(EPOCH FROM (decided_at - created_at)) / 3600.0)::float
            FROM moderation_cases
            WHERE decided_at IS NOT NULL
                AND created_at >= $1 AND created_at < $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        // Automated decisions
        let (automated_decisions,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE report_source = 'automated'
                AND decided_at IS NOT NULL
                AND created_at >= $1 AND created_at < $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        // Automated decisions overturned (appealed and upheld)
        let (automated_decisions_overturned,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE report_source = 'automated'
                AND appeal_decision = 'upheld'
                AND created_at >= $1 AND created_at < $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        // Appeals received
        let (appeals_received,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE appeal_filed = TRUE
                AND appeal_filed_at >= $1 AND appeal_filed_at < $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        // Appeals upheld
        let (appeals_upheld,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE appeal_decision = 'upheld'
                AND appeal_decided_at >= $1 AND appeal_decided_at < $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        // Appeals rejected
        let (appeals_rejected,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE appeal_decision = 'rejected'
                AND appeal_decided_at >= $1 AND appeal_decided_at < $2
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await?;

        Ok(DsaReportSummary {
            total_moderation_actions,
            content_removed,
            content_restricted,
            warnings_issued,
            user_reports_received,
            user_reports_resolved,
            avg_resolution_time_hours,
            automated_decisions,
            automated_decisions_overturned,
            appeals_received,
            appeals_upheld,
            appeals_rejected,
        })
    }

    /// Get DSA report by ID.
    pub async fn get_dsa_report(
        &self,
        id: Uuid,
    ) -> Result<Option<DsaTransparencyReport>, SqlxError> {
        let report = sqlx::query_as::<_, DsaTransparencyReport>(
            r#"SELECT * FROM dsa_transparency_reports WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(report)
    }

    /// List DSA reports.
    pub async fn list_dsa_reports(
        &self,
        status: Option<DsaReportStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DsaTransparencyReport>, SqlxError> {
        let reports = sqlx::query_as::<_, DsaTransparencyReport>(
            r#"
            SELECT * FROM dsa_transparency_reports
            WHERE ($1::dsa_report_status IS NULL OR status = $1)
            ORDER BY period_end DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(reports)
    }

    /// Publish a DSA report.
    pub async fn publish_dsa_report(&self, id: Uuid) -> Result<DsaTransparencyReport, SqlxError> {
        let report = sqlx::query_as::<_, DsaTransparencyReport>(
            r#"
            UPDATE dsa_transparency_reports SET
                status = 'published',
                published_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(report)
    }

    // ========================================================================
    // CONTENT MODERATION CASES
    // ========================================================================

    /// Create a moderation case (user report).
    pub async fn create_moderation_case(
        &self,
        data: CreateModerationCase,
        reported_by: Uuid,
        content_owner_id: Uuid,
        organization_id: Option<Uuid>,
    ) -> Result<ModerationCase, SqlxError> {
        // Calculate priority based on violation type
        let priority = match data.violation_type {
            Some(ViolationType::IllegalContent) | Some(ViolationType::Violence) => 1,
            Some(ViolationType::Fraud) | Some(ViolationType::HateSpeech) => 2,
            Some(ViolationType::Harassment) | Some(ViolationType::Privacy) => 3,
            _ => 3,
        };

        let case = sqlx::query_as::<_, ModerationCase>(
            r#"
            INSERT INTO moderation_cases (
                content_type, content_id, content_owner_id, organization_id,
                report_source, reported_by, violation_type, report_reason, priority
            )
            VALUES ($1, $2, $3, $4, 'user', $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(data.content_type)
        .bind(data.content_id)
        .bind(content_owner_id)
        .bind(organization_id)
        .bind(reported_by)
        .bind(data.violation_type)
        .bind(&data.report_reason)
        .bind(priority)
        .fetch_one(&self.pool)
        .await?;

        Ok(case)
    }

    /// Resolve the real owner (user id) of a piece of moderated content.
    ///
    /// Returns `Ok(None)` when the content row cannot be found or the content
    /// type has no resolvable owner, so callers can reject the request rather
    /// than storing a bogus owner (PAP-38). When `organization_id` is provided,
    /// tenant-scoped tables are filtered by it to prevent cross-tenant
    /// resolution.
    pub async fn resolve_content_owner(
        &self,
        content_type: ModeratedContentType,
        content_id: Uuid,
        organization_id: Option<Uuid>,
    ) -> Result<Option<Uuid>, SqlxError> {
        let owner: Option<(Uuid,)> = match content_type {
            ModeratedContentType::Listing => {
                sqlx::query_as(
                    r#"SELECT created_by FROM listings
                       WHERE id = $1 AND ($2::uuid IS NULL OR organization_id = $2)"#,
                )
                .bind(content_id)
                .bind(organization_id)
                .fetch_optional(&self.pool)
                .await?
            }
            ModeratedContentType::ListingPhoto => {
                sqlx::query_as(
                    r#"SELECT l.created_by FROM listing_photos p
                       JOIN listings l ON l.id = p.listing_id
                       WHERE p.id = $1 AND ($2::uuid IS NULL OR l.organization_id = $2)"#,
                )
                .bind(content_id)
                .bind(organization_id)
                .fetch_optional(&self.pool)
                .await?
            }
            ModeratedContentType::Announcement => {
                sqlx::query_as(
                    r#"SELECT author_id FROM announcements
                       WHERE id = $1 AND ($2::uuid IS NULL OR organization_id = $2)"#,
                )
                .bind(content_id)
                .bind(organization_id)
                .fetch_optional(&self.pool)
                .await?
            }
            ModeratedContentType::Document => {
                sqlx::query_as(
                    r#"SELECT created_by FROM documents
                       WHERE id = $1 AND ($2::uuid IS NULL OR organization_id = $2)"#,
                )
                .bind(content_id)
                .bind(organization_id)
                .fetch_optional(&self.pool)
                .await?
            }
            ModeratedContentType::Message => {
                // Messages carry no org column; scope via the parent thread so a
                // reporter cannot resolve a message outside their tenant (PAP-48).
                sqlx::query_as(
                    r#"SELECT m.sender_id FROM messages m
                       JOIN message_threads t ON t.id = m.thread_id
                       WHERE m.id = $1 AND ($2::uuid IS NULL OR t.organization_id = $2)"#,
                )
                .bind(content_id)
                .bind(organization_id)
                .fetch_optional(&self.pool)
                .await?
            }
            ModeratedContentType::CommunityPost => {
                // Posts reach their org via group -> building; scope explicitly
                // rather than relying on RLS session context (PAP-48).
                sqlx::query_as(
                    r#"SELECT p.author_id FROM community_posts p
                       JOIN community_groups g ON g.id = p.group_id
                       JOIN buildings b ON b.id = g.building_id
                       WHERE p.id = $1 AND ($2::uuid IS NULL OR b.organization_id = $2)"#,
                )
                .bind(content_id)
                .bind(organization_id)
                .fetch_optional(&self.pool)
                .await?
            }
            ModeratedContentType::Comment => {
                // Comments reach their org via post -> group -> building (PAP-48).
                sqlx::query_as(
                    r#"SELECT c.author_id FROM community_comments c
                       JOIN community_posts p ON p.id = c.post_id
                       JOIN community_groups g ON g.id = p.group_id
                       JOIN buildings b ON b.id = g.building_id
                       WHERE c.id = $1 AND ($2::uuid IS NULL OR b.organization_id = $2)"#,
                )
                .bind(content_id)
                .bind(organization_id)
                .fetch_optional(&self.pool)
                .await?
            }
            ModeratedContentType::UserProfile => {
                // A profile's content_id is the profiled user; that user is the
                // owner. Confirm the user exists and — when an org is supplied —
                // is an active member of that org, so a reporter cannot resolve
                // a user outside their tenant (PAP-48).
                sqlx::query_as(
                    r#"SELECT u.id FROM users u
                       WHERE u.id = $1
                         AND ($2::uuid IS NULL OR EXISTS (
                             SELECT 1 FROM organization_members om
                             WHERE om.user_id = u.id
                               AND om.organization_id = $2
                               AND om.status = 'active'
                         ))"#,
                )
                .bind(content_id)
                .bind(organization_id)
                .fetch_optional(&self.pool)
                .await?
            }
            // Review has no backing table in the current schema.
            ModeratedContentType::Review => None,
        };

        Ok(owner.map(|(id,)| id))
    }

    /// Get moderation case by ID.
    pub async fn get_moderation_case(&self, id: Uuid) -> Result<Option<ModerationCase>, SqlxError> {
        let case =
            sqlx::query_as::<_, ModerationCase>(r#"SELECT * FROM moderation_cases WHERE id = $1"#)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(case)
    }

    /// List moderation cases (queue).
    #[allow(clippy::too_many_arguments)]
    pub async fn list_moderation_cases(
        &self,
        status: Option<ModerationStatus>,
        content_type: Option<ModeratedContentType>,
        violation_type: Option<ViolationType>,
        priority: Option<i32>,
        assigned_to: Option<Uuid>,
        unassigned_only: bool,
        sort_by: Option<&str>,
        sort_order: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ModerationCase>, i64), SqlxError> {
        // Count total
        let (total,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE ($1::moderation_status IS NULL OR status = $1)
                AND ($2::moderated_content_type IS NULL OR content_type = $2)
                AND ($3::violation_type IS NULL OR violation_type = $3)
                AND ($4::integer IS NULL OR priority = $4)
                AND ($5::uuid IS NULL OR assigned_to = $5)
                AND ($6 = FALSE OR assigned_to IS NULL)
            "#,
        )
        .bind(status)
        .bind(content_type)
        .bind(violation_type)
        .bind(priority)
        .bind(assigned_to)
        .bind(unassigned_only)
        .fetch_one(&self.pool)
        .await?;

        // Determine sort order
        let order_clause = match (sort_by, sort_order) {
            (Some("priority"), Some("asc")) => "ORDER BY priority ASC, created_at ASC",
            (Some("priority"), _) => "ORDER BY priority ASC, created_at DESC",
            (Some("age"), Some("asc")) => "ORDER BY created_at ASC",
            (Some("age"), _) => "ORDER BY created_at DESC",
            _ => "ORDER BY priority ASC, created_at DESC",
        };

        let query = format!(
            r#"
            SELECT * FROM moderation_cases
            WHERE ($1::moderation_status IS NULL OR status = $1)
                AND ($2::moderated_content_type IS NULL OR content_type = $2)
                AND ($3::violation_type IS NULL OR violation_type = $3)
                AND ($4::integer IS NULL OR priority = $4)
                AND ($5::uuid IS NULL OR assigned_to = $5)
                AND ($6 = FALSE OR assigned_to IS NULL)
            {}
            LIMIT $7 OFFSET $8
            "#,
            order_clause
        );

        let cases = sqlx::query_as::<_, ModerationCase>(sqlx::AssertSqlSafe(query))
            .bind(status)
            .bind(content_type)
            .bind(violation_type)
            .bind(priority)
            .bind(assigned_to)
            .bind(unassigned_only)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        Ok((cases, total))
    }

    /// Get moderation queue statistics.
    pub async fn get_moderation_queue_stats(&self) -> Result<ModerationQueueStats, SqlxError> {
        let (pending_count,): (i64,) =
            sqlx::query_as(r#"SELECT COUNT(*) FROM moderation_cases WHERE status = 'pending'"#)
                .fetch_one(&self.pool)
                .await?;

        let (under_review_count,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(*) FROM moderation_cases WHERE status = 'under_review'"#,
        )
        .fetch_one(&self.pool)
        .await?;

        // Cases by priority
        let by_priority: Vec<(i32, i64)> = sqlx::query_as(
            r#"
            SELECT priority, COUNT(*) as count
            FROM moderation_cases
            WHERE status IN ('pending', 'under_review')
            GROUP BY priority
            ORDER BY priority
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        // Average resolution time
        let avg_resolution_time_hours: f64 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(AVG(EXTRACT(EPOCH FROM (decided_at - created_at)) / 3600.0)::float, 0.0)
            FROM moderation_cases
            WHERE decided_at IS NOT NULL
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        // Overdue count (pending for more than 24 hours)
        let (overdue_count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE status IN ('pending', 'under_review')
                AND created_at < NOW() - INTERVAL '24 hours'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(ModerationQueueStats {
            pending_count,
            under_review_count,
            by_priority: by_priority
                .into_iter()
                .map(|(p, c)| PriorityCount {
                    priority: p,
                    count: c,
                })
                .collect(),
            by_violation_type: vec![], // Could be expanded
            avg_resolution_time_hours,
            overdue_count,
        })
    }

    /// Assign a moderation case.
    pub async fn assign_moderation_case(
        &self,
        id: Uuid,
        moderator_id: Uuid,
    ) -> Result<ModerationCase, SqlxError> {
        let case = sqlx::query_as::<_, ModerationCase>(
            r#"
            UPDATE moderation_cases SET
                assigned_to = $2,
                assigned_at = NOW(),
                status = CASE
                    WHEN status = 'pending' THEN 'under_review'::moderation_status
                    ELSE status
                END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(moderator_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(case)
    }

    /// Take moderation action.
    pub async fn take_moderation_action(
        &self,
        id: Uuid,
        action: TakeModerationAction,
        decided_by: Uuid,
    ) -> Result<ModerationCase, SqlxError> {
        // Determine the resulting status based on action
        let new_status = match action.action {
            ModerationActionType::Remove => ModerationStatus::Removed,
            ModerationActionType::Restrict => ModerationStatus::Restricted,
            ModerationActionType::Warn => ModerationStatus::Warned,
            ModerationActionType::Approve => ModerationStatus::Approved,
            ModerationActionType::Ignore => ModerationStatus::Approved,
            ModerationActionType::Escalate => ModerationStatus::UnderReview,
        };

        let case = sqlx::query_as::<_, ModerationCase>(
            r#"
            UPDATE moderation_cases SET
                status = $2,
                decision = $3,
                decision_rationale = $4,
                action_template_id = $5,
                decided_by = $6,
                decided_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(new_status)
        .bind(action.action)
        .bind(&action.rationale)
        .bind(action.template_id)
        .bind(decided_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(case)
    }

    /// File an appeal.
    pub async fn file_appeal(&self, id: Uuid, reason: &str) -> Result<ModerationCase, SqlxError> {
        let case = sqlx::query_as::<_, ModerationCase>(
            r#"
            UPDATE moderation_cases SET
                appeal_filed = TRUE,
                appeal_reason = $2,
                appeal_filed_at = NOW(),
                status = 'appealed',
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reason)
        .fetch_one(&self.pool)
        .await?;

        Ok(case)
    }

    /// Decide an appeal.
    pub async fn decide_appeal(
        &self,
        id: Uuid,
        decision: &str,
        _rationale: &str,
        decided_by: Uuid,
    ) -> Result<ModerationCase, SqlxError> {
        let new_status = match decision {
            "upheld" => ModerationStatus::AppealApproved,
            _ => ModerationStatus::AppealRejected,
        };

        let case = sqlx::query_as::<_, ModerationCase>(
            r#"
            UPDATE moderation_cases SET
                appeal_decision = $2,
                appeal_decided_by = $3,
                appeal_decided_at = NOW(),
                status = $4,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(decision)
        .bind(decided_by)
        .bind(new_status)
        .fetch_one(&self.pool)
        .await?;

        Ok(case)
    }

    // ========================================================================
    // MODERATION ACTION TEMPLATES
    // ========================================================================

    /// List active moderation action templates.
    pub async fn list_action_templates(&self) -> Result<Vec<ModerationActionTemplate>, SqlxError> {
        let templates = sqlx::query_as::<_, ModerationActionTemplate>(
            r#"
            SELECT * FROM moderation_action_templates
            WHERE is_active = TRUE
            ORDER BY violation_type, name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(templates)
    }

    /// Get action template by ID.
    pub async fn get_action_template(
        &self,
        id: Uuid,
    ) -> Result<Option<ModerationActionTemplate>, SqlxError> {
        let template = sqlx::query_as::<_, ModerationActionTemplate>(
            r#"SELECT * FROM moderation_action_templates WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(template)
    }

    /// Get user's previous violation count.
    pub async fn get_user_violation_count(&self, user_id: Uuid) -> Result<i32, SqlxError> {
        let (count,): (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM moderation_cases
            WHERE content_owner_id = $1
                AND decision IN ('remove', 'restrict', 'warn')
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count as i32)
    }
}

/// Moderation queue statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModerationQueueStats {
    pub pending_count: i64,
    pub under_review_count: i64,
    pub by_priority: Vec<PriorityCount>,
    pub by_violation_type: Vec<ViolationTypeCount>,
    pub avg_resolution_time_hours: f64,
    pub overdue_count: i64,
}

/// Count by priority level.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PriorityCount {
    pub priority: i32,
    pub count: i64,
}

/// Violation type count.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ViolationTypeCount {
    pub violation_type: String,
    pub count: i64,
}
