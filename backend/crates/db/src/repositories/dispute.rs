//! Dispute resolution repository (Epic 77).
//! Provides database operations for disputes, mediation, resolutions, and enforcement.

use crate::models::disputes::*;
use crate::DbPool;
use chrono::{DateTime, Utc};
use common::errors::AppError;
use sqlx::Row;
use uuid::Uuid;

/// Build the structural `{"from": ..., "to": ...}` payload recorded in
/// `dispute_activities.metadata` on every `status_changed` activity (issue
/// #2533).
///
/// Recording the transition structurally lets the funnel / stage-latency KPIs
/// ask "did this dispute ever reach stage X?" with an exact
/// `metadata->>'to' = 'X'` predicate instead of parsing the free-text
/// `description` with a fragile `LIKE '%to ''X''%'` heuristic. `from` is `None`
/// for transitions where the prior state is not loaded (e.g. `withdraw`), in
/// which case only `to` is written.
fn status_transition_metadata(from: Option<&str>, to: &str) -> serde_json::Value {
    match from {
        Some(from) => serde_json::json!({ "from": from, "to": to }),
        None => serde_json::json!({ "to": to }),
    }
}

/// Update session request.
#[derive(Debug, Clone)]
pub struct UpdateSessionData {
    pub scheduled_at: Option<chrono::DateTime<Utc>>,
    pub duration_minutes: Option<i32>,
    pub location: Option<String>,
    pub meeting_url: Option<String>,
    pub status: Option<String>,
}

/// Update attendance request.
#[derive(Debug, Clone)]
pub struct UpdateAttendanceData {
    pub confirmed: Option<bool>,
    pub attended: Option<bool>,
    pub notes: Option<String>,
}

#[derive(Clone)]
pub struct DisputeRepository {
    pool: DbPool,
}

impl DisputeRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ======================== Disputes (Story 77.1) ========================

    pub async fn file_dispute(&self, _org_id: Uuid, req: FileDispute) -> Result<Dispute, AppError> {
        // Generate reference number
        let reference_number = format!("DSP-{}", Uuid::new_v4().to_string()[..8].to_uppercase());

        let dispute = sqlx::query_as::<_, Dispute>(
            r#"
            INSERT INTO disputes (organization_id, building_id, unit_id, reference_number, category,
                                  title, description, desired_resolution, filed_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, organization_id, building_id, unit_id, reference_number, category,
                      title, description, desired_resolution, status, priority, filed_by,
                      assigned_to, resolved_at, resolution_notes, mediation_notes,
                      created_at, updated_at
            "#,
        )
        .bind(req.organization_id)
        .bind(req.building_id)
        .bind(req.unit_id)
        .bind(&reference_number)
        .bind(&req.category)
        .bind(&req.title)
        .bind(&req.description)
        .bind(&req.desired_resolution)
        .bind(req.filed_by)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Add complainant as party (org derived from the dispute we just
        // created, so the org-scope guard in `add_party` is satisfied).
        self.add_party(
            dispute.id,
            req.filed_by,
            party_role::COMPLAINANT,
            dispute.organization_id,
        )
        .await?;

        // Add respondents as parties
        for respondent_id in req.respondent_ids {
            self.add_party(
                dispute.id,
                respondent_id,
                party_role::RESPONDENT,
                dispute.organization_id,
            )
            .await?;
        }

        // Record activity
        self.record_activity(
            dispute.id,
            req.filed_by,
            activity_type::DISPUTE_FILED,
            format!("Dispute filed: {}", dispute.title),
            None,
        )
        .await?;

        Ok(dispute)
    }

    pub async fn list(
        &self,
        org_id: Uuid,
        query: DisputeQuery,
    ) -> Result<Vec<DisputeSummary>, AppError> {
        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);

        let disputes = sqlx::query_as::<_, DisputeSummary>(
            r#"
            SELECT d.id, d.reference_number, d.category, d.title, d.status, d.priority,
                   u.name as filed_by_name,
                   a.name as assigned_to_name,
                   (SELECT COUNT(*) FROM dispute_parties WHERE dispute_id = d.id) as party_count,
                   d.created_at, d.updated_at
            FROM disputes d
            JOIN users u ON u.id = d.filed_by
            LEFT JOIN users a ON a.id = d.assigned_to
            WHERE d.organization_id = $1
              AND ($2::uuid IS NULL OR d.building_id = $2)
              AND ($3::varchar IS NULL OR d.category = $3)
              AND ($4::varchar IS NULL OR d.status = $4)
              AND ($5::varchar IS NULL OR d.priority = $5)
              AND ($6::uuid IS NULL OR d.filed_by = $6)
              AND ($7::uuid IS NULL OR d.assigned_to = $7)
              AND ($8::text IS NULL OR d.title ILIKE '%' || $8 || '%' OR d.description ILIKE '%' || $8 || '%')
              AND ($9::timestamptz IS NULL OR d.created_at >= $9)
              AND ($10::timestamptz IS NULL OR d.created_at <= $10)
            ORDER BY d.created_at DESC
            LIMIT $11 OFFSET $12
            "#,
        )
        .bind(org_id)
        .bind(query.building_id)
        .bind(&query.category)
        .bind(&query.status)
        .bind(&query.priority)
        .bind(query.filed_by)
        .bind(query.assigned_to)
        .bind(&query.search)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(disputes)
    }

    /// Fetch a dispute with details, scoped to the caller's organization.
    ///
    /// Issue #760 / #834 (cross-tenant IDOR): the previous
    /// `find_by_id_with_details` looked the dispute up by primary key alone,
    /// relying on RLS that the management API pool does not enforce. The
    /// `organization_id = $2` predicate closes the leak — a caller targeting a
    /// dispute in another org gets `None` (surfaced as 404), never the row.
    pub async fn find_by_id_with_details_for_org(
        &self,
        id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<DisputeWithDetails>, AppError> {
        // Get dispute
        let dispute = sqlx::query_as::<_, Dispute>(
            r#"
            SELECT id, organization_id, building_id, unit_id, reference_number, category,
                   title, description, desired_resolution, status, priority, filed_by,
                   assigned_to, resolved_at, resolution_notes, mediation_notes,
                   created_at, updated_at
            FROM disputes
            WHERE id = $1 AND organization_id = $2
            "#,
        )
        .bind(id)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let Some(dispute) = dispute else {
            return Ok(None);
        };

        // Get parties with user details
        let party_rows = sqlx::query(
            r#"
            SELECT dp.id, dp.dispute_id, dp.user_id, dp.role, dp.notified_at, dp.responded_at, dp.created_at,
                   u.name as user_name, u.email as user_email
            FROM dispute_parties dp
            JOIN users u ON u.id = dp.user_id
            WHERE dp.dispute_id = $1
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let parties: Vec<DisputePartyWithUser> = party_rows
            .iter()
            .map(|row| DisputePartyWithUser {
                party: DisputeParty {
                    id: row.get("id"),
                    dispute_id: row.get("dispute_id"),
                    user_id: row.get("user_id"),
                    role: row.get("role"),
                    notified_at: row.get("notified_at"),
                    responded_at: row.get("responded_at"),
                    created_at: row.get("created_at"),
                },
                user_name: row.get("user_name"),
                user_email: row.get("user_email"),
            })
            .collect();

        // Get counts
        let evidence_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM dispute_evidence WHERE dispute_id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        let activity_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM dispute_activities WHERE dispute_id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        let session_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM mediation_sessions WHERE dispute_id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        // Get active resolution
        let active_resolution = sqlx::query_as::<_, DisputeResolution>(
            r#"
            SELECT id, dispute_id, proposed_by, resolution_text, terms, status,
                   proposed_at, accepted_at, implemented_at, created_at, updated_at
            FROM dispute_resolutions
            WHERE dispute_id = $1 AND status NOT IN ('rejected', 'implemented')
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Get pending actions
        let pending_actions = sqlx::query_as::<_, ActionItem>(
            r#"
            SELECT id, dispute_id, resolution_id, resolution_term_id, assigned_to, title,
                   description, due_date, status, completed_at, completion_notes,
                   reminder_sent_at, escalated_at, created_at, updated_at
            FROM action_items
            WHERE dispute_id = $1 AND status IN ('pending', 'in_progress')
            ORDER BY due_date ASC
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(Some(DisputeWithDetails {
            dispute,
            parties,
            evidence_count,
            activity_count,
            session_count,
            active_resolution,
            pending_actions,
        }))
    }

    pub async fn update_status(&self, req: UpdateDisputeStatus) -> Result<Dispute, AppError> {
        // Guard: target status must be a known value.
        if !dispute_status::ALL.contains(&req.status.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Unknown dispute status '{}'. Valid: {}",
                req.status,
                dispute_status::ALL.join(", ")
            )));
        }

        // Issue #520: enforce tenancy on the SELECT so a manager in org A
        // cannot drive a dispute in org B by guessing its UUID. The
        // organization_id filter mirrors the `file_dispute` / `list` /
        // `get_statistics` handlers, which all scope by org. Map "no row"
        // to 404 so the response shape cannot be used as a cross-tenant
        // existence oracle.
        let current_status: Option<String> = sqlx::query_scalar(
            "SELECT status FROM disputes WHERE id = $1 AND organization_id = $2",
        )
        .bind(req.dispute_id)
        .bind(req.organization_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let current_status = current_status
            .ok_or_else(|| AppError::NotFound(format!("Dispute {} not found", req.dispute_id)))?;

        // Enforce state machine — reject illegal transitions.
        if !dispute_state_machine::is_valid_transition(&current_status, &req.status) {
            let allowed = dispute_state_machine::allowed_transitions(&current_status);
            return Err(AppError::BadRequest(format!(
                "Invalid status transition '{}' to '{}'. Allowed from '{}': [{}]",
                current_status,
                req.status,
                current_status,
                allowed.join(", ")
            )));
        }

        let dispute = sqlx::query_as::<_, Dispute>(
            r#"
            UPDATE disputes
            SET status = $1
            WHERE id = $2 AND organization_id = $3 AND status = $4
            RETURNING id, organization_id, building_id, unit_id, reference_number, category,
                      title, description, desired_resolution, status, priority, filed_by,
                      assigned_to, resolved_at, resolution_notes, mediation_notes,
                      created_at, updated_at
            "#,
        )
        .bind(&req.status)
        .bind(req.dispute_id)
        .bind(req.organization_id)
        .bind(&current_status)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::BadRequest("Invalid status transition".to_string()))?;

        // Record activity.
        let description = match &req.reason {
            Some(reason) => format!(
                "Status changed from '{}' to '{}': {}",
                current_status, req.status, reason
            ),
            None => format!(
                "Status changed from '{}' to '{}'",
                current_status, req.status
            ),
        };
        self.record_activity(
            req.dispute_id,
            req.updated_by,
            activity_type::STATUS_CHANGED,
            description,
            Some(status_transition_metadata(
                Some(&current_status),
                &req.status,
            )),
        )
        .await?;

        Ok(dispute)
    }

    /// Resolve a dispute — transitions status to `resolved`, sets `resolved_at` and
    /// `resolution_notes`. Only permitted from `under_review`, `mediation`,
    /// `awaiting_response`, or `escalated` states (anything that can reach `resolved`
    /// in the state machine). Note: `filed` cannot transition directly to `resolved`.
    pub async fn resolve_dispute(&self, req: ResolveDispute) -> Result<Dispute, AppError> {
        let now = Utc::now();

        // Load current status to enforce state machine.
        let current_status: Option<String> =
            sqlx::query_scalar("SELECT status FROM disputes WHERE id = $1")
                .bind(req.dispute_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        let current_status = current_status
            .ok_or_else(|| AppError::NotFound(format!("Dispute {} not found", req.dispute_id)))?;

        if !dispute_state_machine::is_valid_transition(&current_status, dispute_status::RESOLVED) {
            let allowed = dispute_state_machine::allowed_transitions(&current_status);
            return Err(AppError::BadRequest(format!(
                "Cannot resolve dispute from status '{}'. Allowed transitions from '{}': [{}]",
                current_status,
                current_status,
                allowed.join(", ")
            )));
        }

        let dispute = sqlx::query_as::<_, Dispute>(
            r#"
            UPDATE disputes
            SET status = 'resolved',
                resolved_at = $1,
                resolution_notes = $2,
                updated_at = NOW()
            WHERE id = $3 AND status = $4 AND organization_id = $5
            RETURNING id, organization_id, building_id, unit_id, reference_number, category,
                      title, description, desired_resolution, status, priority, filed_by,
                      assigned_to, resolved_at, resolution_notes, mediation_notes,
                      created_at, updated_at
            "#,
        )
        .bind(now)
        .bind(&req.resolution_notes)
        .bind(req.dispute_id)
        .bind(&current_status)
        .bind(req.organization_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| {
            AppError::BadRequest("Concurrent status change prevented resolve".to_string())
        })?;

        self.record_activity(
            req.dispute_id,
            req.resolved_by,
            activity_type::STATUS_CHANGED,
            format!(
                "Dispute resolved: {}",
                req.resolution_notes.chars().take(100).collect::<String>()
            ),
            Some(status_transition_metadata(
                Some(&current_status),
                dispute_status::RESOLVED,
            )),
        )
        .await?;

        Ok(dispute)
    }

    /// Update mediation notes on a dispute (does not change status).
    pub async fn update_mediation_notes(
        &self,
        req: UpdateMediationNotes,
    ) -> Result<Dispute, AppError> {
        let dispute = sqlx::query_as::<_, Dispute>(
            r#"
            UPDATE disputes
            SET mediation_notes = $1,
                updated_at = NOW()
            WHERE id = $2 AND organization_id = $3
            RETURNING id, organization_id, building_id, unit_id, reference_number, category,
                      title, description, desired_resolution, status, priority, filed_by,
                      assigned_to, resolved_at, resolution_notes, mediation_notes,
                      created_at, updated_at
            "#,
        )
        .bind(&req.notes)
        .bind(req.dispute_id)
        .bind(req.organization_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Dispute {} not found", req.dispute_id)))?;

        self.record_activity(
            req.dispute_id,
            req.updated_by,
            activity_type::COMMENT_ADDED,
            "Mediation notes updated".to_string(),
            None,
        )
        .await?;

        Ok(dispute)
    }

    /// Withdraw a dispute, scoped to the caller's organization.
    ///
    /// Issue #760 / #834: the UPDATE is gated by `organization_id = $2` so a
    /// caller from another org cannot withdraw a foreign dispute by guessing
    /// its UUID. Returns `NotFound` when no row in the caller's org matches —
    /// the handler maps this to 404 so the response is not a cross-tenant
    /// existence oracle.
    pub async fn withdraw(&self, id: Uuid, org_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE disputes SET status = 'withdrawn' WHERE id = $1 AND organization_id = $2",
        )
        .bind(id)
        .bind(org_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Dispute not found".to_string()));
        }

        // `withdraw` does not load the prior status, so only `to` is recorded —
        // still structural (retires the free-text `LIKE` heuristic for
        // withdrawals), just without a `from` (issue #2533).
        self.record_activity(
            id,
            user_id,
            activity_type::STATUS_CHANGED,
            "Dispute withdrawn".to_string(),
            Some(status_transition_metadata(None, dispute_status::WITHDRAWN)),
        )
        .await?;

        Ok(())
    }

    /// Verify a dispute exists and belongs to `org_id`, returning `NotFound`
    /// otherwise.
    ///
    /// Issue #2441: the dispute sub-resource tables (`dispute_parties`,
    /// `dispute_evidence`, `dispute_activities`) are not RLS-protected on this
    /// pool — that is why `get_dispute` derives the org from the JWT and calls
    /// `find_by_id_with_details_for_org`. The sub-resource read/write methods
    /// share this guard so a caller in org A cannot reach org B's rows by
    /// guessing a `dispute_id`. Mapping "no such row in this org" to `NotFound`
    /// keeps the response from becoming a cross-tenant existence oracle.
    async fn ensure_dispute_in_org(&self, dispute_id: Uuid, org_id: Uuid) -> Result<(), AppError> {
        let found: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM disputes WHERE id = $1 AND organization_id = $2")
                .bind(dispute_id)
                .bind(org_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        if found.is_none() {
            return Err(AppError::NotFound("Dispute not found".to_string()));
        }
        Ok(())
    }

    /// List the parties on a dispute, scoped to the caller's organization
    /// (issue #2441). A foreign-org `dispute_id` yields `NotFound`.
    pub async fn list_parties(
        &self,
        dispute_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<DisputeParty>, AppError> {
        self.ensure_dispute_in_org(dispute_id, org_id).await?;

        let parties = sqlx::query_as::<_, DisputeParty>(
            r#"
            SELECT id, dispute_id, user_id, role, notified_at, responded_at, created_at
            FROM dispute_parties
            WHERE dispute_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(dispute_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(parties)
    }

    /// Add (or upsert) a party on a dispute, scoped to `org_id` (issue #2441).
    ///
    /// The INSERT is gated by an `EXISTS` predicate on the parent dispute's
    /// organization, so the write is atomic — a caller from another org cannot
    /// inject or overwrite a party by guessing the `dispute_id`. A foreign-org
    /// (or unknown) `dispute_id` selects no source row, upserts nothing, and
    /// surfaces as `NotFound`.
    pub async fn add_party(
        &self,
        dispute_id: Uuid,
        user_id: Uuid,
        role: &str,
        org_id: Uuid,
    ) -> Result<DisputeParty, AppError> {
        let party = sqlx::query_as::<_, DisputeParty>(
            r#"
            INSERT INTO dispute_parties (dispute_id, user_id, role)
            SELECT $1, $2, $3
            WHERE EXISTS (
                SELECT 1 FROM disputes WHERE id = $1 AND organization_id = $4
            )
            ON CONFLICT (dispute_id, user_id) DO UPDATE SET role = EXCLUDED.role
            RETURNING id, dispute_id, user_id, role, notified_at, responded_at, created_at
            "#,
        )
        .bind(dispute_id)
        .bind(user_id)
        .bind(role)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Dispute not found".to_string()))?;

        Ok(party)
    }

    /// List the evidence on a dispute, scoped to the caller's organization
    /// (issue #2441). A foreign-org `dispute_id` yields `NotFound`, so evidence
    /// metadata (including the S3 `storage_url`) never leaks across tenants.
    pub async fn list_evidence(
        &self,
        dispute_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<DisputeEvidence>, AppError> {
        self.ensure_dispute_in_org(dispute_id, org_id).await?;

        let evidence = sqlx::query_as::<_, DisputeEvidence>(
            r#"
            SELECT id, dispute_id, uploaded_by, filename, original_filename, content_type,
                   size_bytes, storage_url, description, created_at
            FROM dispute_evidence
            WHERE dispute_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(dispute_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(evidence)
    }

    /// Attach evidence to a dispute, scoped to the caller's organization
    /// (issue #2483 — follow-up to #2441/PR #2450, which org-scoped the sibling
    /// sub-resource methods but missed this write).
    ///
    /// The INSERT is gated by an `EXISTS` predicate on the parent dispute's
    /// organization, so the write is atomic — a caller from another org cannot
    /// attach evidence by guessing the `dispute_id`. A foreign-org (or unknown)
    /// `dispute_id` selects no source row, inserts nothing, and surfaces as
    /// `NotFound`. The follow-up `record_activity` is gated on the same success,
    /// so no phantom activity row is written for a rejected attempt.
    pub async fn add_evidence(
        &self,
        req: AddEvidence,
        org_id: Uuid,
    ) -> Result<DisputeEvidence, AppError> {
        let evidence = sqlx::query_as::<_, DisputeEvidence>(
            r#"
            INSERT INTO dispute_evidence (dispute_id, uploaded_by, filename, original_filename,
                                          content_type, size_bytes, storage_url, description)
            SELECT $1, $2, $3, $4, $5, $6, $7, $8
            WHERE EXISTS (
                SELECT 1 FROM disputes WHERE id = $1 AND organization_id = $9
            )
            RETURNING id, dispute_id, uploaded_by, filename, original_filename, content_type,
                      size_bytes, storage_url, description, created_at
            "#,
        )
        .bind(req.dispute_id)
        .bind(req.uploaded_by)
        .bind(&req.filename)
        .bind(&req.original_filename)
        .bind(&req.content_type)
        .bind(req.size_bytes)
        .bind(&req.storage_url)
        .bind(&req.description)
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Dispute not found".to_string()))?;

        self.record_activity(
            req.dispute_id,
            req.uploaded_by,
            activity_type::EVIDENCE_ADDED,
            format!("Evidence added: {}", req.original_filename),
            None,
        )
        .await?;

        Ok(evidence)
    }

    /// Delete an evidence row, scoped to the caller's organization via the
    /// parent dispute (issue #2441).
    ///
    /// The `EXISTS` predicate on `disputes.organization_id` makes the delete
    /// atomic and tenant-safe — a caller in org A cannot destroy org B's
    /// evidence by guessing the `dispute_id`/`evidence_id`. Returns `false`
    /// (→ handler 404) when nothing in the caller's org matched. Mirrors the
    /// sibling `ViolationRepository::delete_evidence` guard.
    pub async fn delete_evidence(
        &self,
        dispute_id: Uuid,
        evidence_id: Uuid,
        org_id: Uuid,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM dispute_evidence de
            WHERE de.id = $1
              AND de.dispute_id = $2
              AND EXISTS (
                  SELECT 1 FROM disputes d
                  WHERE d.id = de.dispute_id AND d.organization_id = $3
              )
            "#,
        )
        .bind(evidence_id)
        .bind(dispute_id)
        .bind(org_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    /// List the activity trail on a dispute, scoped to the caller's
    /// organization (issue #2441). A foreign-org `dispute_id` yields `NotFound`.
    pub async fn list_activities(
        &self,
        dispute_id: Uuid,
        org_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<DisputeActivity>, AppError> {
        self.ensure_dispute_in_org(dispute_id, org_id).await?;

        let activities = sqlx::query_as::<_, DisputeActivity>(
            r#"
            SELECT id, dispute_id, actor_id, activity_type, description, metadata, created_at
            FROM dispute_activities
            WHERE dispute_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(dispute_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(activities)
    }

    async fn record_activity(
        &self,
        dispute_id: Uuid,
        actor_id: Uuid,
        activity_type: &str,
        description: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO dispute_activities (dispute_id, actor_id, activity_type, description, metadata)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(dispute_id)
        .bind(actor_id)
        .bind(activity_type)
        .bind(&description)
        .bind(&metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn get_statistics(&self, org_id: Uuid) -> Result<DisputeStatistics, AppError> {
        let total_disputes: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM disputes WHERE organization_id = $1")
                .bind(org_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

        let by_status = sqlx::query_as::<_, StatusCount>(
            r#"
            SELECT status, COUNT(*) as count
            FROM disputes
            WHERE organization_id = $1
            GROUP BY status
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let by_category = sqlx::query_as::<_, CategoryCount>(
            r#"
            SELECT category, COUNT(*) as count
            FROM disputes
            WHERE organization_id = $1
            GROUP BY category
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let by_priority = sqlx::query_as::<_, PriorityCount>(
            r#"
            SELECT priority, COUNT(*) as count
            FROM disputes
            WHERE organization_id = $1
            GROUP BY priority
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let avg_resolution_days: Option<f64> = sqlx::query_scalar(
            r#"
            SELECT AVG(EXTRACT(DAY FROM (updated_at - created_at)))
            FROM disputes
            WHERE organization_id = $1 AND status = 'resolved'
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let pending_actions: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM action_items ai
            JOIN disputes d ON d.id = ai.dispute_id
            WHERE d.organization_id = $1 AND ai.status = 'pending'
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let overdue_actions: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM action_items ai
            JOIN disputes d ON d.id = ai.dispute_id
            WHERE d.organization_id = $1 AND ai.status = 'pending' AND ai.due_date < NOW()
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(DisputeStatistics {
            total_disputes,
            by_status,
            by_category,
            by_priority,
            avg_resolution_days,
            pending_actions,
            overdue_actions,
        })
    }

    /// Compute the dispute lifecycle KPIs (funnel + time-to-resolution) for the
    /// cohort of disputes filed in `[window_start, window_end)`, scoped to the
    /// caller's organization (issue #2533).
    ///
    /// This is the SQL-backed implementation of the `dispute.funnel.*` /
    /// `dispute.ttr.*` metrics defined in `docs/data/dispute-lifecycle-kpis.md`.
    /// It is a reporting query (not hot-path); callers should cache/schedule it.
    ///
    /// `reached_mediation` reads the structured
    /// `dispute_activities.metadata->>'to'` written by `update_status` on every
    /// `status_changed` (see [`status_transition_metadata`]), falling back to the
    /// legacy free-text `description LIKE` match so pre-#2533 rows still count.
    /// Rates are `None` when the cohort is empty. TTR percentiles use
    /// `percentile_cont` and are reported in hours.
    pub async fn get_dispute_kpis(
        &self,
        org_id: Uuid,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> Result<DisputeKpis, AppError> {
        // Funnel base counts (robust tier — status + resolved_at only).
        let funnel_row = sqlx::query(
            r#"
            SELECT
                COUNT(*)                                        AS filed,
                COUNT(*) FILTER (WHERE resolved_at IS NOT NULL) AS reached_resolved,
                COUNT(*) FILTER (WHERE status = 'mediation')    AS currently_in_mediation
            FROM disputes
            WHERE organization_id = $1
              AND created_at >= $2
              AND created_at < $3
            "#,
        )
        .bind(org_id)
        .bind(window_start)
        .bind(window_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let filed: i64 = funnel_row.get("filed");
        let reached_resolved: i64 = funnel_row.get("reached_resolved");
        let currently_in_mediation: i64 = funnel_row.get("currently_in_mediation");

        // `reached_mediation` (enriched tier) — structural `metadata->>'to'`
        // with a legacy free-text fallback for pre-#2533 activity rows.
        let reached_mediation: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(DISTINCT d.id)
            FROM disputes d
            JOIN dispute_activities a ON a.dispute_id = d.id
            WHERE d.organization_id = $1
              AND d.created_at >= $2
              AND d.created_at < $3
              AND a.activity_type = 'status_changed'
              AND (
                    a.metadata->>'to' = 'mediation'
                 OR a.description LIKE '%to ''mediation''%'
              )
            "#,
        )
        .bind(org_id)
        .bind(window_start)
        .bind(window_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Null-denominator rule: a rate over an empty cohort is `None`, not 0.
        let rate = |num: i64| -> Option<f64> {
            if filed == 0 {
                None
            } else {
                Some(num as f64 / filed as f64)
            }
        };

        let funnel = DisputeFunnelKpis {
            filed,
            reached_mediation,
            reached_resolved,
            currently_in_mediation,
            mediation_rate: rate(reached_mediation),
            resolution_rate: rate(reached_resolved),
        };

        // Time-to-resolution percentiles (hours), resolved cohort only.
        let ttr_row = sqlx::query(
            r#"
            SELECT
                COUNT(*) AS ttr_count,
                (EXTRACT(EPOCH FROM percentile_cont(0.50) WITHIN GROUP (ORDER BY resolved_at - created_at)) / 3600.0)::float8 AS p50_hours,
                (EXTRACT(EPOCH FROM percentile_cont(0.90) WITHIN GROUP (ORDER BY resolved_at - created_at)) / 3600.0)::float8 AS p90_hours,
                (EXTRACT(EPOCH FROM percentile_cont(0.95) WITHIN GROUP (ORDER BY resolved_at - created_at)) / 3600.0)::float8 AS p95_hours,
                (EXTRACT(EPOCH FROM AVG(resolved_at - created_at)) / 3600.0)::float8 AS mean_hours,
                (EXTRACT(EPOCH FROM MAX(resolved_at - created_at)) / 3600.0)::float8 AS max_hours
            FROM disputes
            WHERE organization_id = $1
              AND resolved_at IS NOT NULL
              AND created_at >= $2
              AND created_at < $3
            "#,
        )
        .bind(org_id)
        .bind(window_start)
        .bind(window_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let ttr = DisputeTtrKpis {
            count: ttr_row.get("ttr_count"),
            p50_hours: ttr_row.get("p50_hours"),
            p90_hours: ttr_row.get("p90_hours"),
            p95_hours: ttr_row.get("p95_hours"),
            mean_hours: ttr_row.get("mean_hours"),
            max_hours: ttr_row.get("max_hours"),
        };

        Ok(DisputeKpis {
            window_start,
            window_end,
            funnel,
            ttr,
        })
    }

    // ======================== Mediation (Story 77.2) ========================

    pub async fn list_sessions(&self, dispute_id: Uuid) -> Result<Vec<MediationSession>, AppError> {
        let sessions = sqlx::query_as::<_, MediationSession>(
            r#"
            SELECT id, dispute_id, mediator_id, session_type, scheduled_at, duration_minutes,
                   location, meeting_url, status, notes, outcome, created_at, updated_at
            FROM mediation_sessions
            WHERE dispute_id = $1
            ORDER BY scheduled_at DESC
            "#,
        )
        .bind(dispute_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(sessions)
    }

    pub async fn schedule_session(
        &self,
        req: ScheduleSession,
    ) -> Result<MediationSession, AppError> {
        let session = sqlx::query_as::<_, MediationSession>(
            r#"
            INSERT INTO mediation_sessions (dispute_id, mediator_id, session_type, scheduled_at,
                                            duration_minutes, location, meeting_url)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, dispute_id, mediator_id, session_type, scheduled_at, duration_minutes,
                      location, meeting_url, status, notes, outcome, created_at, updated_at
            "#,
        )
        .bind(req.dispute_id)
        .bind(req.mediator_id)
        .bind(&req.session_type)
        .bind(req.scheduled_at)
        .bind(req.duration_minutes)
        .bind(&req.location)
        .bind(&req.meeting_url)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Create attendance records for attendees
        for party_id in req.attendee_party_ids {
            sqlx::query("INSERT INTO session_attendances (session_id, party_id) VALUES ($1, $2)")
                .bind(session.id)
                .bind(party_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }

        self.record_activity(
            req.dispute_id,
            req.mediator_id,
            activity_type::SESSION_SCHEDULED,
            format!("Mediation session scheduled for {}", req.scheduled_at),
            None,
        )
        .await?;

        Ok(session)
    }

    pub async fn find_session_by_id(
        &self,
        dispute_id: Uuid,
        session_id: Uuid,
    ) -> Result<Option<MediationSession>, AppError> {
        let session = sqlx::query_as::<_, MediationSession>(
            r#"
            SELECT id, dispute_id, mediator_id, session_type, scheduled_at, duration_minutes,
                   location, meeting_url, status, notes, outcome, created_at, updated_at
            FROM mediation_sessions
            WHERE id = $1 AND dispute_id = $2
            "#,
        )
        .bind(session_id)
        .bind(dispute_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(session)
    }

    pub async fn update_session(
        &self,
        id: Uuid,
        data: UpdateSessionData,
    ) -> Result<MediationSession, AppError> {
        let session = sqlx::query_as::<_, MediationSession>(
            r#"
            UPDATE mediation_sessions
            SET scheduled_at = COALESCE($1, scheduled_at),
                duration_minutes = COALESCE($2, duration_minutes),
                location = COALESCE($3, location),
                meeting_url = COALESCE($4, meeting_url),
                status = COALESCE($5, status)
            WHERE id = $6
            RETURNING id, dispute_id, mediator_id, session_type, scheduled_at, duration_minutes,
                      location, meeting_url, status, notes, outcome, created_at, updated_at
            "#,
        )
        .bind(data.scheduled_at)
        .bind(data.duration_minutes)
        .bind(&data.location)
        .bind(&data.meeting_url)
        .bind(&data.status)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(session)
    }

    pub async fn cancel_session(&self, id: Uuid) -> Result<MediationSession, AppError> {
        let session = sqlx::query_as::<_, MediationSession>(
            r#"
            UPDATE mediation_sessions
            SET status = 'cancelled'
            WHERE id = $1
            RETURNING id, dispute_id, mediator_id, session_type, scheduled_at, duration_minutes,
                      location, meeting_url, status, notes, outcome, created_at, updated_at
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(session)
    }

    pub async fn list_attendance(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<SessionAttendance>, AppError> {
        let attendance = sqlx::query_as::<_, SessionAttendance>(
            r#"
            SELECT id, session_id, party_id, confirmed, attended, notes, created_at, updated_at
            FROM session_attendances
            WHERE session_id = $1
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(attendance)
    }

    pub async fn update_attendance(
        &self,
        session_id: Uuid,
        party_id: Uuid,
        data: UpdateAttendanceData,
    ) -> Result<SessionAttendance, AppError> {
        let attendance = sqlx::query_as::<_, SessionAttendance>(
            r#"
            UPDATE session_attendances
            SET confirmed = COALESCE($1, confirmed),
                attended = COALESCE($2, attended),
                notes = COALESCE($3, notes)
            WHERE session_id = $4 AND party_id = $5
            RETURNING id, session_id, party_id, confirmed, attended, notes, created_at, updated_at
            "#,
        )
        .bind(data.confirmed)
        .bind(data.attended)
        .bind(&data.notes)
        .bind(session_id)
        .bind(party_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(attendance)
    }

    pub async fn record_session_notes(
        &self,
        req: RecordSessionNotes,
    ) -> Result<MediationSession, AppError> {
        let session = sqlx::query_as::<_, MediationSession>(
            r#"
            UPDATE mediation_sessions
            SET notes = $1, outcome = $2, status = 'completed'
            WHERE id = $3
            RETURNING id, dispute_id, mediator_id, session_type, scheduled_at, duration_minutes,
                      location, meeting_url, status, notes, outcome, created_at, updated_at
            "#,
        )
        .bind(&req.notes)
        .bind(&req.outcome)
        .bind(req.session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(session)
    }

    pub async fn list_submissions(
        &self,
        dispute_id: Uuid,
    ) -> Result<Vec<PartySubmission>, AppError> {
        let submissions = sqlx::query_as::<_, PartySubmission>(
            r#"
            SELECT id, dispute_id, party_id, submission_type, content, is_visible_to_all, created_at
            FROM party_submissions
            WHERE dispute_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(dispute_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(submissions)
    }

    pub async fn submit_response(&self, req: SubmitResponse) -> Result<PartySubmission, AppError> {
        let submission = sqlx::query_as::<_, PartySubmission>(
            r#"
            INSERT INTO party_submissions (dispute_id, party_id, submission_type, content, is_visible_to_all)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, dispute_id, party_id, submission_type, content, is_visible_to_all, created_at
            "#,
        )
        .bind(req.dispute_id)
        .bind(req.party_id)
        .bind(&req.submission_type)
        .bind(&req.content)
        .bind(req.is_visible_to_all)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(submission)
    }

    pub async fn find_party_by_user(
        &self,
        dispute_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<DisputeParty>, AppError> {
        let party = sqlx::query_as::<_, DisputeParty>(
            r#"
            SELECT id, dispute_id, user_id, role, notified_at, responded_at, created_at
            FROM dispute_parties
            WHERE dispute_id = $1 AND user_id = $2
            "#,
        )
        .bind(dispute_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(party)
    }

    pub async fn get_mediation_case(
        &self,
        dispute_id: Uuid,
    ) -> Result<Option<MediationCase>, AppError> {
        // Get dispute
        let dispute = sqlx::query_as::<_, Dispute>(
            r#"
            SELECT id, organization_id, building_id, unit_id, reference_number, category,
                   title, description, desired_resolution, status, priority, filed_by,
                   assigned_to, resolved_at, resolution_notes, mediation_notes,
                   created_at, updated_at
            FROM disputes
            WHERE id = $1
            "#,
        )
        .bind(dispute_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let Some(dispute) = dispute else {
            return Ok(None);
        };

        let _sessions = self.list_sessions(dispute_id).await?;
        let submissions = self.list_submissions(dispute_id).await?;
        let resolutions = self.list_resolutions(dispute_id).await?;

        Ok(Some(MediationCase {
            dispute,
            sessions: vec![], // TODO: Include attendance from _sessions
            submissions,
            resolutions,
        }))
    }

    // ======================== Resolution Tracking (Story 77.3) ========================

    pub async fn list_resolutions(
        &self,
        dispute_id: Uuid,
    ) -> Result<Vec<DisputeResolution>, AppError> {
        let resolutions = sqlx::query_as::<_, DisputeResolution>(
            r#"
            SELECT id, dispute_id, proposed_by, resolution_text, terms, status,
                   proposed_at, accepted_at, implemented_at, created_at, updated_at
            FROM dispute_resolutions
            WHERE dispute_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(dispute_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(resolutions)
    }

    pub async fn propose_resolution(
        &self,
        req: ProposeResolution,
    ) -> Result<DisputeResolution, AppError> {
        let resolution = sqlx::query_as::<_, DisputeResolution>(
            r#"
            INSERT INTO dispute_resolutions (dispute_id, proposed_by, resolution_text, terms)
            VALUES ($1, $2, $3, $4)
            RETURNING id, dispute_id, proposed_by, resolution_text, terms, status,
                      proposed_at, accepted_at, implemented_at, created_at, updated_at
            "#,
        )
        .bind(req.dispute_id)
        .bind(req.proposed_by)
        .bind(&req.resolution_text)
        .bind(sqlx::types::Json(&req.terms))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        self.record_activity(
            req.dispute_id,
            req.proposed_by,
            activity_type::RESOLUTION_PROPOSED,
            "Resolution proposed".to_string(),
            None,
        )
        .await?;

        Ok(resolution)
    }

    pub async fn get_resolution_with_votes(
        &self,
        id: Uuid,
    ) -> Result<Option<ResolutionWithVotes>, AppError> {
        let resolution = sqlx::query_as::<_, DisputeResolution>(
            r#"
            SELECT id, dispute_id, proposed_by, resolution_text, terms, status,
                   proposed_at, accepted_at, implemented_at, created_at, updated_at
            FROM dispute_resolutions
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let Some(resolution) = resolution else {
            return Ok(None);
        };

        let votes = sqlx::query_as::<_, ResolutionVote>(
            r#"
            SELECT id, resolution_id, party_id, accepted, comments, voted_at
            FROM resolution_votes
            WHERE resolution_id = $1
            "#,
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let acceptance_rate = if votes.is_empty() {
            0.0
        } else {
            votes.iter().filter(|v| v.accepted).count() as f64 / votes.len() as f64
        };

        Ok(Some(ResolutionWithVotes {
            resolution,
            votes,
            acceptance_rate,
        }))
    }

    pub async fn vote_on_resolution(
        &self,
        req: VoteOnResolution,
    ) -> Result<ResolutionVote, AppError> {
        let vote = sqlx::query_as::<_, ResolutionVote>(
            r#"
            INSERT INTO resolution_votes (resolution_id, party_id, accepted, comments)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (resolution_id, party_id) DO UPDATE
            SET accepted = $3, comments = $4, voted_at = NOW()
            RETURNING id, resolution_id, party_id, accepted, comments, voted_at
            "#,
        )
        .bind(req.resolution_id)
        .bind(req.party_id)
        .bind(req.accepted)
        .bind(&req.comments)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(vote)
    }

    pub async fn accept_resolution(
        &self,
        id: Uuid,
        _user_id: Uuid,
    ) -> Result<DisputeResolution, AppError> {
        let now = Utc::now();

        let resolution = sqlx::query_as::<_, DisputeResolution>(
            r#"
            UPDATE dispute_resolutions
            SET status = 'accepted', accepted_at = $1
            WHERE id = $2
            RETURNING id, dispute_id, proposed_by, resolution_text, terms, status,
                      proposed_at, accepted_at, implemented_at, created_at, updated_at
            "#,
        )
        .bind(now)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(resolution)
    }

    pub async fn implement_resolution(
        &self,
        id: Uuid,
        _user_id: Uuid,
    ) -> Result<DisputeResolution, AppError> {
        let now = Utc::now();

        let resolution = sqlx::query_as::<_, DisputeResolution>(
            r#"
            UPDATE dispute_resolutions
            SET status = 'implemented', implemented_at = $1
            WHERE id = $2
            RETURNING id, dispute_id, proposed_by, resolution_text, terms, status,
                      proposed_at, accepted_at, implemented_at, created_at, updated_at
            "#,
        )
        .bind(now)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(resolution)
    }

    // ======================== Resolution Enforcement (Story 77.4) ========================

    pub async fn list_action_items(&self, dispute_id: Uuid) -> Result<Vec<ActionItem>, AppError> {
        let items = sqlx::query_as::<_, ActionItem>(
            r#"
            SELECT id, dispute_id, resolution_id, resolution_term_id, assigned_to, title,
                   description, due_date, status, completed_at, completion_notes,
                   reminder_sent_at, escalated_at, created_at, updated_at
            FROM action_items
            WHERE dispute_id = $1
            ORDER BY due_date ASC
            "#,
        )
        .bind(dispute_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(items)
    }

    pub async fn create_action_item(&self, req: CreateActionItem) -> Result<ActionItem, AppError> {
        let item = sqlx::query_as::<_, ActionItem>(
            r#"
            INSERT INTO action_items (dispute_id, resolution_id, resolution_term_id, assigned_to,
                                      title, description, due_date)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, dispute_id, resolution_id, resolution_term_id, assigned_to, title,
                      description, due_date, status, completed_at, completion_notes,
                      reminder_sent_at, escalated_at, created_at, updated_at
            "#,
        )
        .bind(req.dispute_id)
        .bind(req.resolution_id)
        .bind(&req.resolution_term_id)
        .bind(req.assigned_to)
        .bind(&req.title)
        .bind(&req.description)
        .bind(req.due_date)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(item)
    }

    pub async fn find_action_item(&self, id: Uuid) -> Result<Option<ActionItem>, AppError> {
        let item = sqlx::query_as::<_, ActionItem>(
            r#"
            SELECT id, dispute_id, resolution_id, resolution_term_id, assigned_to, title,
                   description, due_date, status, completed_at, completion_notes,
                   reminder_sent_at, escalated_at, created_at, updated_at
            FROM action_items
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(item)
    }

    pub async fn update_action_item(
        &self,
        id: Uuid,
        title: Option<String>,
        description: Option<String>,
        due_date: Option<chrono::DateTime<Utc>>,
        status: Option<String>,
    ) -> Result<ActionItem, AppError> {
        let item = sqlx::query_as::<_, ActionItem>(
            r#"
            UPDATE action_items
            SET title = COALESCE($1, title),
                description = COALESCE($2, description),
                due_date = COALESCE($3, due_date),
                status = COALESCE($4, status)
            WHERE id = $5
            RETURNING id, dispute_id, resolution_id, resolution_term_id, assigned_to, title,
                      description, due_date, status, completed_at, completion_notes,
                      reminder_sent_at, escalated_at, created_at, updated_at
            "#,
        )
        .bind(&title)
        .bind(&description)
        .bind(due_date)
        .bind(&status)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(item)
    }

    pub async fn complete_action_item(
        &self,
        req: CompleteActionItem,
    ) -> Result<ActionItem, AppError> {
        let now = Utc::now();

        let item = sqlx::query_as::<_, ActionItem>(
            r#"
            UPDATE action_items
            SET status = 'completed', completed_at = $1, completion_notes = $2
            WHERE id = $3
            RETURNING id, dispute_id, resolution_id, resolution_term_id, assigned_to, title,
                      description, due_date, status, completed_at, completion_notes,
                      reminder_sent_at, escalated_at, created_at, updated_at
            "#,
        )
        .bind(now)
        .bind(&req.completion_notes)
        .bind(req.action_item_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(item)
    }

    pub async fn send_action_reminder(&self, action_id: Uuid) -> Result<ActionItem, AppError> {
        let now = Utc::now();

        let item = sqlx::query_as::<_, ActionItem>(
            r#"
            UPDATE action_items
            SET reminder_sent_at = $1
            WHERE id = $2
            RETURNING id, dispute_id, resolution_id, resolution_term_id, assigned_to, title,
                      description, due_date, status, completed_at, completion_notes,
                      reminder_sent_at, escalated_at, created_at, updated_at
            "#,
        )
        .bind(now)
        .bind(action_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(item)
    }

    pub async fn list_overdue_actions(&self, org_id: Uuid) -> Result<Vec<ActionItem>, AppError> {
        let items = sqlx::query_as::<_, ActionItem>(
            r#"
            SELECT ai.id, ai.dispute_id, ai.resolution_id, ai.resolution_term_id, ai.assigned_to,
                   ai.title, ai.description, ai.due_date, ai.status, ai.completed_at,
                   ai.completion_notes, ai.reminder_sent_at, ai.escalated_at, ai.created_at, ai.updated_at
            FROM action_items ai
            JOIN disputes d ON d.id = ai.dispute_id
            WHERE d.organization_id = $1 AND ai.status = 'pending' AND ai.due_date < NOW()
            ORDER BY ai.due_date ASC
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(items)
    }

    pub async fn list_escalations(&self, dispute_id: Uuid) -> Result<Vec<Escalation>, AppError> {
        let escalations = sqlx::query_as::<_, Escalation>(
            r#"
            SELECT id, dispute_id, action_item_id, escalated_by, escalated_to, reason,
                   severity, resolved, resolved_at, resolution_notes, created_at, updated_at
            FROM escalations
            WHERE dispute_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(dispute_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(escalations)
    }

    pub async fn create_escalation(&self, req: CreateEscalation) -> Result<Escalation, AppError> {
        let escalation = sqlx::query_as::<_, Escalation>(
            r#"
            INSERT INTO escalations (dispute_id, action_item_id, escalated_by, escalated_to, reason, severity)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, dispute_id, action_item_id, escalated_by, escalated_to, reason,
                      severity, resolved, resolved_at, resolution_notes, created_at, updated_at
            "#,
        )
        .bind(req.dispute_id)
        .bind(req.action_item_id)
        .bind(req.escalated_by)
        .bind(req.escalated_to)
        .bind(&req.reason)
        .bind(&req.severity)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Update action item if linked
        if let Some(action_id) = req.action_item_id {
            sqlx::query(
                "UPDATE action_items SET escalated_at = NOW(), status = 'escalated' WHERE id = $1",
            )
            .bind(action_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        self.record_activity(
            req.dispute_id,
            req.escalated_by,
            activity_type::ESCALATED,
            format!("Escalation created: {}", req.reason),
            None,
        )
        .await?;

        Ok(escalation)
    }

    pub async fn resolve_escalation(&self, req: ResolveEscalation) -> Result<Escalation, AppError> {
        let now = Utc::now();

        let escalation = sqlx::query_as::<_, Escalation>(
            r#"
            UPDATE escalations
            SET resolved = true, resolved_at = $1, resolution_notes = $2
            WHERE id = $3
            RETURNING id, dispute_id, action_item_id, escalated_by, escalated_to, reason,
                      severity, resolved, resolved_at, resolution_notes, created_at, updated_at
            "#,
        )
        .bind(now)
        .bind(&req.resolution_notes)
        .bind(req.escalation_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(escalation)
    }

    pub async fn get_party_actions_dashboard(
        &self,
        user_id: Uuid,
    ) -> Result<PartyActionsDashboard, AppError> {
        let now = Utc::now();

        let pending = sqlx::query_as::<_, ActionItem>(
            r#"
            SELECT id, dispute_id, resolution_id, resolution_term_id, assigned_to, title,
                   description, due_date, status, completed_at, completion_notes,
                   reminder_sent_at, escalated_at, created_at, updated_at
            FROM action_items
            WHERE assigned_to = $1 AND status = 'pending' AND due_date >= $2
            ORDER BY due_date ASC
            "#,
        )
        .bind(user_id)
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let overdue = sqlx::query_as::<_, ActionItem>(
            r#"
            SELECT id, dispute_id, resolution_id, resolution_term_id, assigned_to, title,
                   description, due_date, status, completed_at, completion_notes,
                   reminder_sent_at, escalated_at, created_at, updated_at
            FROM action_items
            WHERE assigned_to = $1 AND status = 'pending' AND due_date < $2
            ORDER BY due_date ASC
            "#,
        )
        .bind(user_id)
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let completed_recently = sqlx::query_as::<_, ActionItem>(
            r#"
            SELECT id, dispute_id, resolution_id, resolution_term_id, assigned_to, title,
                   description, due_date, status, completed_at, completion_notes,
                   reminder_sent_at, escalated_at, created_at, updated_at
            FROM action_items
            WHERE assigned_to = $1 AND status = 'completed' AND completed_at > NOW() - INTERVAL '7 days'
            ORDER BY completed_at DESC
            LIMIT 10
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let total_pending = pending.len() as i64;
        let total_overdue = overdue.len() as i64;

        Ok(PartyActionsDashboard {
            user_id,
            pending,
            overdue,
            completed_recently,
            total_pending,
            total_overdue,
        })
    }

    pub async fn get_party_actions(
        &self,
        _org_id: Uuid,
        user_id: Uuid,
    ) -> Result<PartyActionsDashboard, AppError> {
        self.get_party_actions_dashboard(user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn status_transition_metadata_records_from_and_to() {
        // update_status / resolve_dispute path — both endpoints of the
        // transition are recorded structurally so the funnel KPI can match on
        // metadata->>'to' instead of parsing free text (issue #2533).
        let meta = status_transition_metadata(Some("under_review"), "mediation");
        assert_eq!(meta["from"], "under_review");
        assert_eq!(meta["to"], "mediation");
        // Exactly the two structural keys, nothing else.
        let obj = meta.as_object().expect("metadata is a JSON object");
        assert_eq!(obj.len(), 2);
    }

    #[test]
    fn status_transition_metadata_omits_from_when_absent() {
        // withdraw path — prior status not loaded, so only `to` is recorded.
        let meta = status_transition_metadata(None, dispute_status::WITHDRAWN);
        assert_eq!(meta["to"], "withdrawn");
        assert!(meta.get("from").is_none());
        let obj = meta.as_object().expect("metadata is a JSON object");
        assert_eq!(obj.len(), 1);
    }

    #[test]
    fn status_transition_metadata_matches_enriched_funnel_predicate() {
        // The reached_mediation query keys on metadata->>'to' = 'mediation'.
        // Guard that the value we write is exactly the literal that predicate
        // (and dispute_status::MEDIATION) expects.
        let meta = status_transition_metadata(Some("awaiting_response"), dispute_status::MEDIATION);
        assert_eq!(meta["to"].as_str(), Some("mediation"));
    }

    /// Asserts the JSON shape returned by `get_dispute_kpis` — the contract a
    /// dashboard / scheduled export consumes (issue #2533). The DB round-trip
    /// itself is covered by integration tests against a live Postgres; here we
    /// pin the serialized field names, nesting, and the null-denominator rule.
    #[test]
    fn dispute_kpis_serializes_to_expected_shape() {
        let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let kpis = DisputeKpis {
            window_start: start,
            window_end: end,
            funnel: DisputeFunnelKpis {
                filed: 10,
                reached_mediation: 4,
                reached_resolved: 6,
                currently_in_mediation: 1,
                mediation_rate: Some(0.4),
                resolution_rate: Some(0.6),
            },
            ttr: DisputeTtrKpis {
                count: 6,
                p50_hours: Some(12.5),
                p90_hours: Some(48.0),
                p95_hours: Some(72.0),
                mean_hours: Some(20.0),
                max_hours: Some(96.0),
            },
        };

        let v = serde_json::to_value(&kpis).expect("serialize");

        // Top-level shape.
        assert!(v.get("window_start").is_some());
        assert!(v.get("window_end").is_some());

        // Funnel block.
        let funnel = &v["funnel"];
        for key in [
            "filed",
            "reached_mediation",
            "reached_resolved",
            "currently_in_mediation",
            "mediation_rate",
            "resolution_rate",
        ] {
            assert!(funnel.get(key).is_some(), "funnel missing `{key}`");
        }
        assert_eq!(funnel["filed"], 10);
        assert_eq!(funnel["reached_mediation"], 4);

        // TTR block.
        let ttr = &v["ttr"];
        for key in [
            "count",
            "p50_hours",
            "p90_hours",
            "p95_hours",
            "mean_hours",
            "max_hours",
        ] {
            assert!(ttr.get(key).is_some(), "ttr missing `{key}`");
        }
        assert_eq!(ttr["count"], 6);
    }

    #[test]
    fn dispute_kpis_empty_cohort_reports_null_rates() {
        // Null-denominator rule: an empty cohort must serialize rates as JSON
        // null (not 0.0) so a dashboard shows "no data", not a misleading 0%.
        let start = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap();
        let kpis = DisputeKpis {
            window_start: start,
            window_end: end,
            funnel: DisputeFunnelKpis {
                filed: 0,
                reached_mediation: 0,
                reached_resolved: 0,
                currently_in_mediation: 0,
                mediation_rate: None,
                resolution_rate: None,
            },
            ttr: DisputeTtrKpis {
                count: 0,
                p50_hours: None,
                p90_hours: None,
                p95_hours: None,
                mean_hours: None,
                max_hours: None,
            },
        };

        let v = serde_json::to_value(&kpis).expect("serialize");
        assert!(v["funnel"]["mediation_rate"].is_null());
        assert!(v["funnel"]["resolution_rate"].is_null());
        assert!(v["ttr"]["p50_hours"].is_null());
    }
}
