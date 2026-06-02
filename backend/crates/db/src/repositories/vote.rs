//! Vote repository (Epic 5: Building Voting & Decisions).
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
//! async fn create_poll(
//!     mut rls: RlsConnection,
//!     State(state): State<AppState>,
//!     Json(data): Json<CreateVoteRequest>,
//! ) -> Result<Json<Vote>> {
//!     let vote = state.vote_repo.create_poll_rls(rls.conn(), data).await?;
//!     rls.release().await;
//!     Ok(Json(vote))
//! }
//! ```

use crate::models::vote::{
    audit_action, vote_status, CancelVote, CastVote, CreateVote, CreateVoteAuditLog,
    CreateVoteComment, CreateVoteQuestion, EligibleUnit, HideVoteComment, OptionResult,
    PublishVote, QuestionResult, UpdateVote, UpdateVoteQuestion, Vote, VoteAuditLog, VoteComment,
    VoteCommentWithUser, VoteEligibility, VoteListQuery, VoteQuestion, VoteReceipt, VoteResponse,
    VoteResults, VoteSummary, VoteWithDetails,
};
use crate::DbPool;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use sqlx::{Error as SqlxError, Executor, FromRow, Postgres};
use uuid::Uuid;

/// Row struct for vote with details query.
#[derive(Debug, FromRow)]
struct VoteDetailsRow {
    // Vote fields
    pub id: Uuid,
    pub organization_id: Uuid,
    pub building_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: DateTime<Utc>,
    pub status: String,
    pub quorum_type: String,
    pub quorum_percentage: Option<i32>,
    pub allow_delegation: bool,
    pub anonymous_voting: bool,
    pub participation_count: Option<i32>,
    pub eligible_count: Option<i32>,
    pub quorum_met: Option<bool>,
    pub results: serde_json::Value,
    pub results_calculated_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub published_by: Option<Uuid>,
    pub published_at: Option<DateTime<Utc>>,
    pub cancelled_by: Option<Uuid>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancellation_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Joined fields
    pub building_name: String,
    pub created_by_name: String,
    pub question_count: i64,
    pub response_count: i64,
    pub comment_count: i64,
}

/// Row struct for comment with user.
#[derive(Debug, FromRow)]
struct CommentWithUserRow {
    pub id: Uuid,
    pub vote_id: Uuid,
    pub user_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub content: String,
    pub hidden: bool,
    pub hidden_by: Option<Uuid>,
    pub hidden_at: Option<DateTime<Utc>>,
    pub hidden_reason: Option<String>,
    pub ai_consent: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_name: String,
    pub reply_count: i64,
}

/// Row struct for eligible unit.
#[derive(Debug, FromRow)]
struct EligibleUnitRow {
    pub unit_id: Uuid,
    pub unit_designation: String,
    pub ownership_share: Decimal,
    pub is_owner: bool,
    pub is_delegated: bool,
    pub delegation_id: Option<Uuid>,
    pub already_voted: bool,
}

/// Repository for vote operations.
#[derive(Clone)]
pub struct VoteRepository {
    pool: DbPool,
}

impl VoteRepository {
    /// Create a new VoteRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // RLS-aware methods (recommended)
    // ========================================================================

    /// Create a new poll/vote with RLS context (Story 5.1).
    ///
    /// Use this method with an `RlsConnection` to ensure RLS policies are enforced.
    pub async fn create_poll_rls<'e, E>(
        &self,
        executor: E,
        data: CreateVote,
    ) -> Result<Vote, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let vote = sqlx::query_as::<_, Vote>(
            r#"
            INSERT INTO votes (
                organization_id, building_id, title, description,
                start_at, end_at, quorum_type, quorum_percentage,
                allow_delegation, anonymous_voting, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(data.building_id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.start_at)
        .bind(data.end_at)
        .bind(&data.quorum_type)
        .bind(data.quorum_percentage)
        .bind(data.allow_delegation.unwrap_or(true))
        .bind(data.anonymous_voting.unwrap_or(false))
        .bind(data.created_by)
        .fetch_one(executor)
        .await?;

        Ok(vote)
    }

    /// Find poll/vote by ID with RLS context.
    ///
    /// Use this method with an `RlsConnection` to ensure RLS policies are enforced.
    pub async fn find_poll_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<Vote>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let vote = sqlx::query_as::<_, Vote>(
            r#"
            SELECT * FROM votes WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        Ok(vote)
    }

    /// Cast a vote with RLS context (Story 5.3).
    ///
    /// Use this method with an `RlsConnection` to ensure RLS policies are enforced.
    ///
    /// Takes a `&mut PgConnection` (rather than a generic by-value executor) so
    /// that the ballot INSERT and the immutable audit-log INSERT both run on the
    /// SAME RLS-scoped connection, keeping the tenant context (`app.current_org_id`)
    /// in scope for the `vote_audit_log` insert policy (Story 5.7).
    pub async fn cast_vote_rls(
        &self,
        executor: &mut sqlx::PgConnection,
        data: CastVote,
    ) -> Result<VoteReceipt, SqlxError> {
        // Generate hash for ballot integrity
        let hash_input = format!(
            "{}:{}:{}:{}:{}",
            data.vote_id,
            data.user_id,
            data.unit_id,
            serde_json::to_string(&data.answers).unwrap_or_default(),
            Utc::now().timestamp_millis()
        );
        let response_hash = hex::encode(Sha256::digest(hash_input.as_bytes()));

        let is_delegated = data.delegation_id.is_some();

        // Fold the ownership-weight lookup into the INSERT as a scalar subquery so
        // the stored vote_weight reflects the unit's ownership_share (Story 5.2),
        // mirroring the legacy `COALESCE(ownership_share, 1.0)` semantics. The
        // subquery degrades safely to 1.0 if the units row is not visible/found.
        let response = sqlx::query_as::<_, VoteResponse>(
            r#"
            INSERT INTO vote_responses (
                vote_id, user_id, unit_id, delegation_id, is_delegated,
                answers, vote_weight, response_hash
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                COALESCE((SELECT ownership_share FROM units WHERE id = $3), 1.0),
                $7
            )
            ON CONFLICT (vote_id, unit_id) DO UPDATE
            SET
                user_id = EXCLUDED.user_id,
                delegation_id = EXCLUDED.delegation_id,
                is_delegated = EXCLUDED.is_delegated,
                answers = EXCLUDED.answers,
                response_hash = EXCLUDED.response_hash,
                submitted_at = NOW()
            RETURNING *
            "#,
        )
        .bind(data.vote_id)
        .bind(data.user_id)
        .bind(data.unit_id)
        .bind(data.delegation_id)
        .bind(is_delegated)
        .bind(&data.answers)
        .bind(&response_hash)
        .fetch_one(&mut *executor)
        .await?;

        // Write the immutable audit entry on the SAME RLS executor (Story 5.7).
        let audit_action = if is_delegated {
            audit_action::BALLOT_UPDATED
        } else {
            audit_action::BALLOT_CAST
        };

        Self::create_audit_entry_rls(
            &mut *executor,
            CreateVoteAuditLog {
                vote_id: data.vote_id,
                user_id: Some(data.user_id),
                action: audit_action.to_string(),
                data: serde_json::json!({
                    "unit_id": data.unit_id,
                    "response_hash": response_hash,
                    "is_delegated": is_delegated,
                }),
                ip_address: None,
                user_agent: None,
            },
        )
        .await?;

        // The RLS executor borrow has ended; recompute participation_count on the
        // pool so the live quorum/participation indicator reflects this ballot
        // while the vote is still active (Story 5.6).
        self.update_participation_count(data.vote_id).await?;

        // Generate confirmation number (first 8 chars of hash)
        let confirmation_number = response_hash[..8].to_uppercase();

        Ok(VoteReceipt {
            response_id: response.id,
            vote_id: response.vote_id,
            unit_id: response.unit_id,
            submitted_at: response.submitted_at,
            response_hash,
            confirmation_number,
        })
    }

    /// Get poll results with RLS context (Story 5.5).
    ///
    /// Use this method with an `RlsConnection` to ensure RLS policies are enforced.
    pub async fn get_poll_results_rls<'e, E>(
        &self,
        executor: E,
        vote_id: Uuid,
    ) -> Result<Option<VoteResults>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Get vote from RLS context
        let vote = sqlx::query_as::<_, Vote>(
            r#"
            SELECT * FROM votes WHERE id = $1
            "#,
        )
        .bind(vote_id)
        .fetch_optional(executor)
        .await?;

        match vote {
            Some(v) if v.is_closed() => {
                // Return cached results
                let results: VoteResults =
                    serde_json::from_value(v.results).unwrap_or(VoteResults {
                        vote_id,
                        participation_count: v.participation_count.unwrap_or(0),
                        eligible_count: v.eligible_count.unwrap_or(0),
                        participation_rate: 0.0,
                        quorum_met: v.quorum_met.unwrap_or(false),
                        questions: Vec::new(),
                        calculated_at: v.results_calculated_at.unwrap_or_else(Utc::now),
                    });
                Ok(Some(results))
            }
            Some(v) => {
                // Vote not closed yet - compute live tallies so an active-vote
                // results dashboard sees real participation/per-question data
                // (Story 5.6). Uses compute_results (no audit write) on self.pool,
                // reusing the already-loaded vote row. RLS scoping for this branch
                // relies on route-level authorization (the RLS executor only gated
                // the initial vote lookup above).
                let results = self.compute_results(&v).await?;
                Ok(Some(results))
            }
            None => Ok(None),
        }
    }

    /// List polls by building with RLS context.
    ///
    /// Use this method with an `RlsConnection` to ensure RLS policies are enforced.
    pub async fn list_polls_by_building_rls<'e, E>(
        &self,
        executor: E,
        building_id: Uuid,
    ) -> Result<Vec<VoteSummary>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let votes = sqlx::query_as::<_, VoteSummary>(
            r#"
            SELECT id, building_id, title, status::text as status, end_at,
                   quorum_type::text as quorum_type, participation_count, eligible_count, quorum_met
            FROM votes
            WHERE building_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(building_id)
        .fetch_all(executor)
        .await?;

        Ok(votes)
    }

    // ========================================================================
    // Legacy methods (use pool directly - migrate to RLS versions)
    // ========================================================================

    // ========================================================================
    // Vote CRUD
    // ========================================================================

    /// Create a new vote (Story 5.1).
    ///
    /// **Deprecated**: Use `create_poll_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.274",
        note = "Use create_poll_rls with RlsConnection instead"
    )]
    pub async fn create(&self, data: CreateVote) -> Result<Vote, SqlxError> {
        // Save fields needed for audit log before moving data into create_poll_rls
        let created_by = data.created_by;
        let title = data.title.clone();
        let quorum_type = data.quorum_type.clone();

        let vote = self.create_poll_rls(&self.pool, data).await?;

        // Create audit log entry (using legacy pool)
        self.create_audit_entry(CreateVoteAuditLog {
            vote_id: vote.id,
            user_id: Some(created_by),
            action: audit_action::VOTE_CREATED.to_string(),
            data: serde_json::json!({
                "title": title,
                "quorum_type": quorum_type,
            }),
            ip_address: None,
            user_agent: None,
        })
        .await?;

        Ok(vote)
    }

    /// Create a new vote (Story 5.1) - non-deprecated version for internal use.
    pub async fn create_vote(&self, data: CreateVote) -> Result<Vote, SqlxError> {
        let vote = sqlx::query_as::<_, Vote>(
            r#"
            INSERT INTO votes (
                organization_id, building_id, title, description,
                start_at, end_at, quorum_type, quorum_percentage,
                allow_delegation, anonymous_voting, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(data.organization_id)
        .bind(data.building_id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.start_at)
        .bind(data.end_at)
        .bind(&data.quorum_type)
        .bind(data.quorum_percentage)
        .bind(data.allow_delegation.unwrap_or(true))
        .bind(data.anonymous_voting.unwrap_or(false))
        .bind(data.created_by)
        .fetch_one(&self.pool)
        .await?;

        // Create audit log entry
        self.create_audit_entry(CreateVoteAuditLog {
            vote_id: vote.id,
            user_id: Some(data.created_by),
            action: audit_action::VOTE_CREATED.to_string(),
            data: serde_json::json!({
                "title": data.title,
                "quorum_type": data.quorum_type,
            }),
            ip_address: None,
            user_agent: None,
        })
        .await?;

        Ok(vote)
    }

    /// Find vote by ID.
    ///
    /// **Deprecated**: Use `find_poll_by_id_rls` with an RLS-enabled connection instead.
    #[deprecated(
        since = "0.2.274",
        note = "Use find_poll_by_id_rls with RlsConnection instead"
    )]
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Vote>, SqlxError> {
        self.find_poll_by_id_rls(&self.pool, id).await
    }

    /// Find vote with full details.
    pub async fn find_by_id_with_details(
        &self,
        id: Uuid,
    ) -> Result<Option<VoteWithDetails>, SqlxError> {
        let result = sqlx::query_as::<_, VoteDetailsRow>(
            r#"
            SELECT
                v.id, v.organization_id, v.building_id, v.title, v.description,
                v.start_at, v.end_at, v.status::text as status, v.quorum_type::text as quorum_type,
                v.quorum_percentage, v.allow_delegation, v.anonymous_voting,
                v.participation_count, v.eligible_count, v.quorum_met,
                v.results, v.results_calculated_at, v.created_by, v.published_by,
                v.published_at, v.cancelled_by, v.cancelled_at, v.cancellation_reason,
                v.created_at, v.updated_at,
                COALESCE(b.name, b.street) as building_name,
                u.name as created_by_name,
                (SELECT COUNT(*) FROM vote_questions WHERE vote_id = v.id) as question_count,
                (SELECT COUNT(*) FROM vote_responses WHERE vote_id = v.id) as response_count,
                (SELECT COUNT(*) FROM vote_comments WHERE vote_id = v.id AND hidden = false) as comment_count
            FROM votes v
            JOIN buildings b ON v.building_id = b.id
            JOIN users u ON v.created_by = u.id
            WHERE v.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| {
            let vote = Vote {
                id: row.id,
                organization_id: row.organization_id,
                building_id: row.building_id,
                title: row.title,
                description: row.description,
                start_at: row.start_at,
                end_at: row.end_at,
                status: row.status,
                quorum_type: row.quorum_type,
                quorum_percentage: row.quorum_percentage,
                allow_delegation: row.allow_delegation,
                anonymous_voting: row.anonymous_voting,
                participation_count: row.participation_count,
                eligible_count: row.eligible_count,
                quorum_met: row.quorum_met,
                results: row.results,
                results_calculated_at: row.results_calculated_at,
                created_by: row.created_by,
                published_by: row.published_by,
                published_at: row.published_at,
                cancelled_by: row.cancelled_by,
                cancelled_at: row.cancelled_at,
                cancellation_reason: row.cancellation_reason,
                created_at: row.created_at,
                updated_at: row.updated_at,
            };
            VoteWithDetails {
                vote,
                building_name: row.building_name,
                created_by_name: row.created_by_name,
                question_count: row.question_count,
                response_count: row.response_count,
                comment_count: row.comment_count,
            }
        }))
    }

    /// Find vote with full details, scoped to a single organization.
    ///
    /// `find_by_id_with_details` looks votes up by primary key alone, which
    /// lets a caller in org A read a vote that belongs to org B (cross-tenant
    /// IDOR). This variant adds `AND v.organization_id = $2` so a vote from a
    /// different tenant resolves to `None` (the route maps that to 404).
    pub async fn find_by_id_with_details_for_org(
        &self,
        id: Uuid,
        organization_id: Uuid,
    ) -> Result<Option<VoteWithDetails>, SqlxError> {
        let result = sqlx::query_as::<_, VoteDetailsRow>(
            r#"
            SELECT
                v.id, v.organization_id, v.building_id, v.title, v.description,
                v.start_at, v.end_at, v.status::text as status, v.quorum_type::text as quorum_type,
                v.quorum_percentage, v.allow_delegation, v.anonymous_voting,
                v.participation_count, v.eligible_count, v.quorum_met,
                v.results, v.results_calculated_at, v.created_by, v.published_by,
                v.published_at, v.cancelled_by, v.cancelled_at, v.cancellation_reason,
                v.created_at, v.updated_at,
                COALESCE(b.name, b.street) as building_name,
                u.name as created_by_name,
                (SELECT COUNT(*) FROM vote_questions WHERE vote_id = v.id) as question_count,
                (SELECT COUNT(*) FROM vote_responses WHERE vote_id = v.id) as response_count,
                (SELECT COUNT(*) FROM vote_comments WHERE vote_id = v.id AND hidden = false) as comment_count
            FROM votes v
            JOIN buildings b ON v.building_id = b.id
            JOIN users u ON v.created_by = u.id
            WHERE v.id = $1 AND v.organization_id = $2
            "#,
        )
        .bind(id)
        .bind(organization_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| {
            let vote = Vote {
                id: row.id,
                organization_id: row.organization_id,
                building_id: row.building_id,
                title: row.title,
                description: row.description,
                start_at: row.start_at,
                end_at: row.end_at,
                status: row.status,
                quorum_type: row.quorum_type,
                quorum_percentage: row.quorum_percentage,
                allow_delegation: row.allow_delegation,
                anonymous_voting: row.anonymous_voting,
                participation_count: row.participation_count,
                eligible_count: row.eligible_count,
                quorum_met: row.quorum_met,
                results: row.results,
                results_calculated_at: row.results_calculated_at,
                created_by: row.created_by,
                published_by: row.published_by,
                published_at: row.published_at,
                cancelled_by: row.cancelled_by,
                cancelled_at: row.cancelled_at,
                cancellation_reason: row.cancellation_reason,
                created_at: row.created_at,
                updated_at: row.updated_at,
            };
            VoteWithDetails {
                vote,
                building_name: row.building_name,
                created_by_name: row.created_by_name,
                question_count: row.question_count,
                response_count: row.response_count,
                comment_count: row.comment_count,
            }
        }))
    }

    /// List votes with filters.
    pub async fn list(
        &self,
        org_id: Uuid,
        query: VoteListQuery,
    ) -> Result<Vec<VoteSummary>, SqlxError> {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);

        // Build dynamic WHERE clause
        let mut conditions = vec!["organization_id = $1".to_string()];
        let mut param_idx = 2;

        if query.building_id.is_some() {
            conditions.push(format!("building_id = ${}", param_idx));
            param_idx += 1;
        }
        if query.status.is_some() {
            conditions.push(format!("status = ANY(${}::vote_status[])", param_idx));
            param_idx += 1;
        }
        if query.created_by.is_some() {
            conditions.push(format!("created_by = ${}", param_idx));
            param_idx += 1;
        }
        if query.from_date.is_some() {
            conditions.push(format!("end_at >= ${}", param_idx));
            param_idx += 1;
        }
        if query.to_date.is_some() {
            conditions.push(format!("end_at <= ${}", param_idx));
        }

        let where_clause = conditions.join(" AND ");

        let sql = format!(
            r#"
            SELECT id, building_id, title, status::text as status, end_at,
                   quorum_type::text as quorum_type, participation_count, eligible_count, quorum_met
            FROM votes
            WHERE {}
            ORDER BY created_at DESC
            LIMIT {} OFFSET {}
            "#,
            where_clause, limit, offset
        );

        let mut query_builder =
            sqlx::query_as::<_, VoteSummary>(sqlx::AssertSqlSafe(sql)).bind(org_id);

        if let Some(building_id) = query.building_id {
            query_builder = query_builder.bind(building_id);
        }
        if let Some(ref status) = query.status {
            query_builder = query_builder.bind(status);
        }
        if let Some(created_by) = query.created_by {
            query_builder = query_builder.bind(created_by);
        }
        if let Some(from_date) = query.from_date {
            query_builder = query_builder.bind(from_date);
        }
        if let Some(to_date) = query.to_date {
            query_builder = query_builder.bind(to_date);
        }

        let votes = query_builder.fetch_all(&self.pool).await?;
        Ok(votes)
    }

    /// List active votes for a building.
    ///
    /// **Deprecated**: Use `list_polls_by_building_rls` with an RLS-enabled connection instead.
    /// Note: This method only returns active votes. The RLS version returns all votes;
    /// filter by status as needed.
    #[deprecated(
        since = "0.2.274",
        note = "Use list_polls_by_building_rls with RlsConnection instead"
    )]
    pub async fn list_active_by_building(
        &self,
        building_id: Uuid,
    ) -> Result<Vec<VoteSummary>, SqlxError> {
        let votes = sqlx::query_as::<_, VoteSummary>(
            r#"
            SELECT id, building_id, title, status::text as status, end_at,
                   quorum_type::text as quorum_type, participation_count, eligible_count, quorum_met
            FROM votes
            WHERE building_id = $1 AND status = 'active'
            ORDER BY end_at ASC
            "#,
        )
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(votes)
    }

    /// Update vote details (only in draft status).
    pub async fn update(&self, id: Uuid, data: UpdateVote) -> Result<Vote, SqlxError> {
        let vote = sqlx::query_as::<_, Vote>(
            r#"
            UPDATE votes
            SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                start_at = COALESCE($4, start_at),
                end_at = COALESCE($5, end_at),
                quorum_type = COALESCE($6, quorum_type),
                quorum_percentage = COALESCE($7, quorum_percentage),
                allow_delegation = COALESCE($8, allow_delegation),
                anonymous_voting = COALESCE($9, anonymous_voting),
                updated_at = NOW()
            WHERE id = $1 AND status = 'draft'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(data.start_at)
        .bind(data.end_at)
        .bind(&data.quorum_type)
        .bind(data.quorum_percentage)
        .bind(data.allow_delegation)
        .bind(data.anonymous_voting)
        .fetch_one(&self.pool)
        .await?;

        Ok(vote)
    }

    /// Delete vote (only in draft status).
    pub async fn delete(&self, id: Uuid) -> Result<(), SqlxError> {
        sqlx::query("DELETE FROM votes WHERE id = $1 AND status = 'draft'")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ========================================================================
    // Workflow Operations
    // ========================================================================

    /// Publish a vote (Story 5.2).
    pub async fn publish(
        &self,
        id: Uuid,
        user_id: Uuid,
        data: PublishVote,
    ) -> Result<Vote, SqlxError> {
        // Calculate eligible count
        let eligible_count = self.count_eligible_units(id).await?;

        let now = Utc::now();
        let start_at = data.start_at.unwrap_or(now);
        let new_status = if start_at <= now {
            vote_status::ACTIVE
        } else {
            vote_status::SCHEDULED
        };

        let vote = sqlx::query_as::<_, Vote>(
            r#"
            UPDATE votes
            SET
                status = $2,
                start_at = $3,
                eligible_count = $4,
                published_by = $5,
                published_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND status = 'draft'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(new_status)
        .bind(start_at)
        .bind(eligible_count)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Create audit log entry
        self.create_audit_entry(CreateVoteAuditLog {
            vote_id: id,
            user_id: Some(user_id),
            action: audit_action::VOTE_PUBLISHED.to_string(),
            data: serde_json::json!({
                "status": new_status,
                "eligible_count": eligible_count,
            }),
            ip_address: None,
            user_agent: None,
        })
        .await?;

        Ok(vote)
    }

    /// Cancel a vote.
    pub async fn cancel(
        &self,
        id: Uuid,
        user_id: Uuid,
        data: CancelVote,
    ) -> Result<Vote, SqlxError> {
        let vote = sqlx::query_as::<_, Vote>(
            r#"
            UPDATE votes
            SET
                status = 'cancelled',
                cancelled_by = $2,
                cancelled_at = NOW(),
                cancellation_reason = $3,
                updated_at = NOW()
            WHERE id = $1 AND status != 'closed'
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&data.reason)
        .fetch_one(&self.pool)
        .await?;

        // Create audit log entry
        self.create_audit_entry(CreateVoteAuditLog {
            vote_id: id,
            user_id: Some(user_id),
            action: audit_action::VOTE_CANCELLED.to_string(),
            data: serde_json::json!({ "reason": data.reason }),
            ip_address: None,
            user_agent: None,
        })
        .await?;

        Ok(vote)
    }

    /// Close a vote and calculate results.
    pub async fn close(&self, id: Uuid) -> Result<Vote, SqlxError> {
        // Calculate results
        let results = self.calculate_results(id).await?;
        let results_json = serde_json::to_value(&results).unwrap_or_default();

        let vote = sqlx::query_as::<_, Vote>(
            r#"
            UPDATE votes
            SET
                status = 'closed',
                participation_count = $2,
                quorum_met = $3,
                results = $4,
                results_calculated_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(results.participation_count)
        .bind(results.quorum_met)
        .bind(&results_json)
        .fetch_one(&self.pool)
        .await?;

        // Create audit log entry
        self.create_audit_entry(CreateVoteAuditLog {
            vote_id: id,
            user_id: None,
            action: audit_action::VOTE_CLOSED.to_string(),
            data: serde_json::json!({
                "participation_count": results.participation_count,
                "quorum_met": results.quorum_met,
            }),
            ip_address: None,
            user_agent: None,
        })
        .await?;

        Ok(vote)
    }

    /// Activate scheduled votes that have reached their start time.
    pub async fn activate_scheduled_votes(&self) -> Result<Vec<Vote>, SqlxError> {
        let votes = sqlx::query_as::<_, Vote>(
            r#"
            UPDATE votes
            SET status = 'active', updated_at = NOW()
            WHERE status = 'scheduled' AND start_at <= NOW()
            RETURNING *
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(votes)
    }

    /// Close expired votes.
    pub async fn close_expired_votes(&self) -> Result<Vec<Uuid>, SqlxError> {
        let expired_ids: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT id FROM votes
            WHERE status = 'active' AND end_at <= NOW()
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut closed = Vec::new();
        for (id,) in expired_ids {
            self.close(id).await?;
            closed.push(id);
        }

        Ok(closed)
    }

    // ========================================================================
    // Questions (Story 5.1)
    // ========================================================================

    /// Add a question to a vote.
    pub async fn add_question(&self, data: CreateVoteQuestion) -> Result<VoteQuestion, SqlxError> {
        let options_json = serde_json::to_value(&data.options).unwrap_or_default();
        let display_order = data.display_order.unwrap_or(0);
        let is_required = data.is_required.unwrap_or(true);

        let question = sqlx::query_as::<_, VoteQuestion>(
            r#"
            INSERT INTO vote_questions (
                vote_id, question_text, description, question_type,
                options, display_order, is_required
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(data.vote_id)
        .bind(&data.question_text)
        .bind(&data.description)
        .bind(&data.question_type)
        .bind(&options_json)
        .bind(display_order)
        .bind(is_required)
        .fetch_one(&self.pool)
        .await?;

        // Create audit log entry
        self.create_audit_entry(CreateVoteAuditLog {
            vote_id: data.vote_id,
            user_id: None,
            action: audit_action::QUESTION_ADDED.to_string(),
            data: serde_json::json!({
                "question_id": question.id,
                "question_text": data.question_text,
                "question_type": data.question_type,
            }),
            ip_address: None,
            user_agent: None,
        })
        .await?;

        Ok(question)
    }

    /// Get questions for a vote.
    pub async fn get_questions(&self, vote_id: Uuid) -> Result<Vec<VoteQuestion>, SqlxError> {
        let questions = sqlx::query_as::<_, VoteQuestion>(
            r#"
            SELECT * FROM vote_questions
            WHERE vote_id = $1
            ORDER BY display_order
            "#,
        )
        .bind(vote_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(questions)
    }

    /// Update a question.
    pub async fn update_question(
        &self,
        id: Uuid,
        data: UpdateVoteQuestion,
    ) -> Result<VoteQuestion, SqlxError> {
        let options_json = data
            .options
            .as_ref()
            .map(|o| serde_json::to_value(o).unwrap_or_default());

        let question = sqlx::query_as::<_, VoteQuestion>(
            r#"
            UPDATE vote_questions
            SET
                question_text = COALESCE($2, question_text),
                description = COALESCE($3, description),
                options = COALESCE($4, options),
                display_order = COALESCE($5, display_order),
                is_required = COALESCE($6, is_required),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.question_text)
        .bind(&data.description)
        .bind(&options_json)
        .bind(data.display_order)
        .bind(data.is_required)
        .fetch_one(&self.pool)
        .await?;

        Ok(question)
    }

    /// Delete a question.
    pub async fn delete_question(&self, id: Uuid, vote_id: Uuid) -> Result<(), SqlxError> {
        sqlx::query("DELETE FROM vote_questions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        // Create audit log entry
        self.create_audit_entry(CreateVoteAuditLog {
            vote_id,
            user_id: None,
            action: audit_action::QUESTION_REMOVED.to_string(),
            data: serde_json::json!({ "question_id": id }),
            ip_address: None,
            user_agent: None,
        })
        .await?;

        Ok(())
    }

    // ========================================================================
    // Voting (Story 5.3)
    // ========================================================================

    /// Cast a vote (Story 5.3).
    ///
    /// **Deprecated**: Use `cast_vote_rls` with an RLS-enabled connection instead.
    /// Note: This legacy method includes additional functionality (vote weight lookup,
    /// audit logging, participation count update) that must be handled separately
    /// when using the RLS version.
    #[deprecated(
        since = "0.2.274",
        note = "Use cast_vote_rls with RlsConnection instead"
    )]
    pub async fn cast_vote(&self, data: CastVote) -> Result<VoteReceipt, SqlxError> {
        // Generate hash for ballot integrity
        let hash_input = format!(
            "{}:{}:{}:{}:{}",
            data.vote_id,
            data.user_id,
            data.unit_id,
            serde_json::to_string(&data.answers).unwrap_or_default(),
            Utc::now().timestamp_millis()
        );
        let response_hash = hex::encode(Sha256::digest(hash_input.as_bytes()));

        // Get vote weight from unit ownership_share
        let vote_weight: (Decimal,) = sqlx::query_as(
            r#"
            SELECT COALESCE(ownership_share, 1.0) as weight
            FROM units WHERE id = $1
            "#,
        )
        .bind(data.unit_id)
        .fetch_one(&self.pool)
        .await?;

        let is_delegated = data.delegation_id.is_some();

        let response = sqlx::query_as::<_, VoteResponse>(
            r#"
            INSERT INTO vote_responses (
                vote_id, user_id, unit_id, delegation_id, is_delegated,
                answers, vote_weight, response_hash
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (vote_id, unit_id) DO UPDATE
            SET
                user_id = EXCLUDED.user_id,
                delegation_id = EXCLUDED.delegation_id,
                is_delegated = EXCLUDED.is_delegated,
                answers = EXCLUDED.answers,
                response_hash = EXCLUDED.response_hash,
                submitted_at = NOW()
            RETURNING *
            "#,
        )
        .bind(data.vote_id)
        .bind(data.user_id)
        .bind(data.unit_id)
        .bind(data.delegation_id)
        .bind(is_delegated)
        .bind(&data.answers)
        .bind(vote_weight.0)
        .bind(&response_hash)
        .fetch_one(&self.pool)
        .await?;

        // Generate confirmation number (first 8 chars of hash)
        let confirmation_number = response_hash[..8].to_uppercase();

        // Create audit log entry
        let audit_action = if is_delegated {
            audit_action::BALLOT_UPDATED
        } else {
            audit_action::BALLOT_CAST
        };

        self.create_audit_entry(CreateVoteAuditLog {
            vote_id: data.vote_id,
            user_id: Some(data.user_id),
            action: audit_action.to_string(),
            data: serde_json::json!({
                "unit_id": data.unit_id,
                "response_hash": response_hash,
                "is_delegated": is_delegated,
            }),
            ip_address: None,
            user_agent: None,
        })
        .await?;

        // Update participation count
        self.update_participation_count(data.vote_id).await?;

        Ok(VoteReceipt {
            response_id: response.id,
            vote_id: response.vote_id,
            unit_id: response.unit_id,
            submitted_at: response.submitted_at,
            response_hash,
            confirmation_number,
        })
    }

    /// Get user's response for a vote.
    pub async fn get_user_response(
        &self,
        vote_id: Uuid,
        unit_id: Uuid,
    ) -> Result<Option<VoteResponse>, SqlxError> {
        let response = sqlx::query_as::<_, VoteResponse>(
            r#"
            SELECT * FROM vote_responses
            WHERE vote_id = $1 AND unit_id = $2
            "#,
        )
        .bind(vote_id)
        .bind(unit_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(response)
    }

    /// Check vote eligibility for a user (Story 5.3).
    #[allow(deprecated)]
    pub async fn check_eligibility(
        &self,
        vote_id: Uuid,
        user_id: Uuid,
    ) -> Result<VoteEligibility, SqlxError> {
        // Get vote details
        let vote = self
            .find_by_id(vote_id)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        // Get eligible units (owned or delegated)
        let eligible_units = sqlx::query_as::<_, EligibleUnitRow>(
            r#"
            WITH user_units AS (
                -- Owned units
                SELECT
                    u.id as unit_id,
                    u.designation as unit_designation,
                    u.ownership_share,
                    true as is_owner,
                    false as is_delegated,
                    NULL::uuid as delegation_id
                FROM units u
                JOIN unit_residents ur ON u.id = ur.unit_id
                WHERE u.building_id = $1
                  AND ur.user_id = $2
                  AND ur.resident_type = 'owner'
                  AND ur.move_out_date IS NULL

                UNION ALL

                -- Delegated units (if delegation allowed)
                SELECT
                    u.id as unit_id,
                    u.designation as unit_designation,
                    u.ownership_share,
                    false as is_owner,
                    true as is_delegated,
                    d.id as delegation_id
                FROM units u
                JOIN delegations d ON u.id = d.unit_id
                WHERE u.building_id = $1
                  AND d.delegate_id = $2
                  AND d.status = 'active'
                  AND d.scope IN ('voting', 'full')
                  AND (d.expires_at IS NULL OR d.expires_at > NOW())
                  AND $3 = true
            )
            SELECT
                uu.unit_id,
                uu.unit_designation,
                uu.ownership_share,
                uu.is_owner,
                uu.is_delegated,
                uu.delegation_id,
                EXISTS (
                    SELECT 1 FROM vote_responses vr
                    WHERE vr.vote_id = $4 AND vr.unit_id = uu.unit_id
                ) as already_voted
            FROM user_units uu
            "#,
        )
        .bind(vote.building_id)
        .bind(user_id)
        .bind(vote.allow_delegation)
        .bind(vote_id)
        .fetch_all(&self.pool)
        .await?;

        let units: Vec<EligibleUnit> = eligible_units
            .into_iter()
            .map(|row| EligibleUnit {
                unit_id: row.unit_id,
                unit_designation: row.unit_designation,
                ownership_share: row.ownership_share,
                is_owner: row.is_owner,
                is_delegated: row.is_delegated,
                delegation_id: row.delegation_id,
                already_voted: row.already_voted,
            })
            .collect();

        let can_vote = vote.can_vote() && units.iter().any(|u| !u.already_voted);
        let reason = if !vote.can_vote() {
            Some("Vote is not currently accepting ballots".to_string())
        } else if units.is_empty() {
            Some("User is not eligible to vote in this building".to_string())
        } else if units.iter().all(|u| u.already_voted) {
            Some("All eligible units have already voted".to_string())
        } else {
            None
        };

        Ok(VoteEligibility {
            vote_id,
            user_id,
            eligible_units: units,
            can_vote,
            reason,
        })
    }

    /// Count eligible units for a vote.
    #[allow(deprecated)]
    async fn count_eligible_units(&self, vote_id: Uuid) -> Result<i32, SqlxError> {
        let vote = self
            .find_by_id(vote_id)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(DISTINCT u.id)
            FROM units u
            JOIN unit_residents ur ON u.id = ur.unit_id
            WHERE u.building_id = $1
              AND ur.resident_type = 'owner'
              AND ur.move_out_date IS NULL
            "#,
        )
        .bind(vote.building_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 as i32)
    }

    /// Get the user IDs of eligible owners for a building (Story 5.1).
    ///
    /// Returns the distinct owner residents of the building's units — the
    /// recipients of a `VoteStarted` notification when a vote is published.
    /// Mirrors the scheduler's eligible-users query (scheduler.rs) so immediate
    /// and scheduled publishes notify the same audience. Note this counts user
    /// IDs, not units (unlike `count_eligible_units`).
    pub async fn get_eligible_owner_user_ids(
        &self,
        building_id: Uuid,
    ) -> Result<Vec<Uuid>, SqlxError> {
        let users: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT ur.user_id
            FROM unit_residents ur
            JOIN units u ON ur.unit_id = u.id
            WHERE u.building_id = $1
              AND ur.resident_type = 'owner'
              AND ur.move_out_date IS NULL
            "#,
        )
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(users.into_iter().map(|(id,)| id).collect())
    }

    /// Update participation count for a vote.
    async fn update_participation_count(&self, vote_id: Uuid) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            UPDATE votes
            SET participation_count = (
                SELECT COUNT(*) FROM vote_responses WHERE vote_id = $1
            ),
            updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(vote_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========================================================================
    // Comments (Story 5.4)
    // ========================================================================

    /// Add a comment to a vote.
    pub async fn add_comment(&self, data: CreateVoteComment) -> Result<VoteComment, SqlxError> {
        let comment = sqlx::query_as::<_, VoteComment>(
            r#"
            INSERT INTO vote_comments (vote_id, user_id, parent_id, content, ai_consent)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.vote_id)
        .bind(data.user_id)
        .bind(data.parent_id)
        .bind(&data.content)
        .bind(data.ai_consent)
        .fetch_one(&self.pool)
        .await?;

        // Create audit log entry
        self.create_audit_entry(CreateVoteAuditLog {
            vote_id: data.vote_id,
            user_id: Some(data.user_id),
            action: audit_action::COMMENT_ADDED.to_string(),
            data: serde_json::json!({
                "comment_id": comment.id,
                "parent_id": data.parent_id,
            }),
            ip_address: None,
            user_agent: None,
        })
        .await?;

        Ok(comment)
    }

    /// List comments for a vote.
    pub async fn list_comments(
        &self,
        vote_id: Uuid,
        include_hidden: bool,
    ) -> Result<Vec<VoteCommentWithUser>, SqlxError> {
        let hidden_filter = if include_hidden {
            ""
        } else {
            "AND c.hidden = false"
        };

        let sql = format!(
            r#"
            SELECT
                c.id, c.vote_id, c.user_id, c.parent_id, c.content, c.hidden,
                c.hidden_by, c.hidden_at, c.hidden_reason, c.ai_consent,
                c.created_at, c.updated_at,
                u.name as user_name,
                (SELECT COUNT(*) FROM vote_comments WHERE parent_id = c.id {}) as reply_count
            FROM vote_comments c
            JOIN users u ON c.user_id = u.id
            WHERE c.vote_id = $1 AND c.parent_id IS NULL {}
            ORDER BY c.created_at
            "#,
            hidden_filter, hidden_filter
        );

        let rows = sqlx::query_as::<_, CommentWithUserRow>(sqlx::AssertSqlSafe(sql))
            .bind(vote_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let comment = VoteComment {
                    id: row.id,
                    vote_id: row.vote_id,
                    user_id: row.user_id,
                    parent_id: row.parent_id,
                    content: row.content,
                    hidden: row.hidden,
                    hidden_by: row.hidden_by,
                    hidden_at: row.hidden_at,
                    hidden_reason: row.hidden_reason,
                    ai_consent: row.ai_consent,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                };
                VoteCommentWithUser {
                    comment,
                    user_name: row.user_name,
                    reply_count: row.reply_count,
                }
            })
            .collect())
    }

    /// List replies to a comment.
    pub async fn list_replies(
        &self,
        parent_id: Uuid,
        include_hidden: bool,
    ) -> Result<Vec<VoteCommentWithUser>, SqlxError> {
        let hidden_filter = if include_hidden {
            ""
        } else {
            "AND c.hidden = false"
        };

        let sql = format!(
            r#"
            SELECT
                c.id, c.vote_id, c.user_id, c.parent_id, c.content, c.hidden,
                c.hidden_by, c.hidden_at, c.hidden_reason, c.ai_consent,
                c.created_at, c.updated_at,
                u.name as user_name,
                (SELECT COUNT(*) FROM vote_comments WHERE parent_id = c.id {}) as reply_count
            FROM vote_comments c
            JOIN users u ON c.user_id = u.id
            WHERE c.parent_id = $1 {}
            ORDER BY c.created_at
            "#,
            hidden_filter, hidden_filter
        );

        let rows = sqlx::query_as::<_, CommentWithUserRow>(sqlx::AssertSqlSafe(sql))
            .bind(parent_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let comment = VoteComment {
                    id: row.id,
                    vote_id: row.vote_id,
                    user_id: row.user_id,
                    parent_id: row.parent_id,
                    content: row.content,
                    hidden: row.hidden,
                    hidden_by: row.hidden_by,
                    hidden_at: row.hidden_at,
                    hidden_reason: row.hidden_reason,
                    ai_consent: row.ai_consent,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                };
                VoteCommentWithUser {
                    comment,
                    user_name: row.user_name,
                    reply_count: row.reply_count,
                }
            })
            .collect())
    }

    /// Hide a comment.
    pub async fn hide_comment(
        &self,
        id: Uuid,
        hidden_by: Uuid,
        data: HideVoteComment,
    ) -> Result<VoteComment, SqlxError> {
        let comment = sqlx::query_as::<_, VoteComment>(
            r#"
            UPDATE vote_comments
            SET hidden = true, hidden_by = $2, hidden_at = NOW(), hidden_reason = $3, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(hidden_by)
        .bind(&data.reason)
        .fetch_one(&self.pool)
        .await?;

        // Create audit log entry
        self.create_audit_entry(CreateVoteAuditLog {
            vote_id: comment.vote_id,
            user_id: Some(hidden_by),
            action: audit_action::COMMENT_HIDDEN.to_string(),
            data: serde_json::json!({
                "comment_id": id,
                "reason": data.reason,
            }),
            ip_address: None,
            user_agent: None,
        })
        .await?;

        Ok(comment)
    }

    // ========================================================================
    // Results (Story 5.5)
    // ========================================================================

    /// Compute live tallies for a vote WITHOUT writing an audit entry.
    ///
    /// Pure read path shared by [`calculate_results`](Self::calculate_results)
    /// (which appends a `RESULTS_CALCULATED` audit entry afterwards) and the
    /// active-vote arm of [`get_poll_results_rls`](Self::get_poll_results_rls)
    /// (a read that must NOT spam the audit log on every dashboard poll).
    ///
    /// Queries `self.pool`, not the RLS executor, so callers must enforce
    /// route-level authorization before exposing the results (Story 5.6).
    async fn compute_results(&self, vote: &Vote) -> Result<VoteResults, SqlxError> {
        let vote_id = vote.id;
        let questions = self.get_questions(vote_id).await?;

        // Get all responses
        let responses = sqlx::query_as::<_, VoteResponse>(
            r#"
            SELECT * FROM vote_responses WHERE vote_id = $1
            "#,
        )
        .bind(vote_id)
        .fetch_all(&self.pool)
        .await?;

        let participation_count = responses.len() as i32;
        let eligible_count = vote.eligible_count.unwrap_or(0);
        let participation_rate = if eligible_count > 0 {
            (participation_count as f64 / eligible_count as f64) * 100.0
        } else {
            0.0
        };

        // Calculate quorum
        let quorum_met = self.check_quorum(vote, participation_count, eligible_count);

        // Calculate results for each question
        let mut question_results = Vec::new();

        for question in questions {
            let result = self
                .calculate_question_result(&question, &responses)
                .await?;
            question_results.push(result);
        }

        Ok(VoteResults {
            vote_id,
            participation_count,
            eligible_count,
            participation_rate,
            quorum_met,
            questions: question_results,
            calculated_at: Utc::now(),
        })
    }

    /// Calculate results for a vote.
    #[allow(deprecated)]
    pub async fn calculate_results(&self, vote_id: Uuid) -> Result<VoteResults, SqlxError> {
        let vote = self
            .find_by_id(vote_id)
            .await?
            .ok_or(SqlxError::RowNotFound)?;

        let results = self.compute_results(&vote).await?;

        // Create audit log entry
        self.create_audit_entry(CreateVoteAuditLog {
            vote_id,
            user_id: None,
            action: audit_action::RESULTS_CALCULATED.to_string(),
            data: serde_json::json!({
                "participation_count": results.participation_count,
                "participation_rate": results.participation_rate,
                "quorum_met": results.quorum_met,
            }),
            ip_address: None,
            user_agent: None,
        })
        .await?;

        Ok(results)
    }

    /// Check if quorum is met.
    fn check_quorum(&self, vote: &Vote, participation: i32, eligible: i32) -> bool {
        if eligible == 0 {
            return false;
        }

        let participation_rate = (participation as f64 / eligible as f64) * 100.0;
        let required_percentage = vote.quorum_percentage.unwrap_or(50) as f64;

        participation_rate >= required_percentage
    }

    /// Calculate result for a single question.
    async fn calculate_question_result(
        &self,
        question: &VoteQuestion,
        responses: &[VoteResponse],
    ) -> Result<QuestionResult, SqlxError> {
        let options: Vec<crate::models::vote::QuestionOption> =
            serde_json::from_value(question.options.clone()).unwrap_or_default();

        let mut option_results: Vec<OptionResult> = options
            .iter()
            .map(|opt| OptionResult {
                option_id: opt.id,
                option_text: opt.text.clone(),
                count: 0,
                weighted_count: 0.0,
                percentage: 0.0,
            })
            .collect();

        let mut total_votes = 0;
        let mut weighted_total = 0.0;

        // Count votes based on question type
        for response in responses {
            if let Some(answer) = response.answers.get(question.id.to_string()) {
                let weight: f64 = response.vote_weight.try_into().unwrap_or(1.0);

                match question.question_type.as_str() {
                    "yes_no" => {
                        // Answer is true/false, map to Yes/No options
                        if let Some(is_yes) = answer.as_bool() {
                            let idx = if is_yes { 0 } else { 1 };
                            if idx < option_results.len() {
                                option_results[idx].count += 1;
                                option_results[idx].weighted_count += weight;
                            }
                        }
                        total_votes += 1;
                        weighted_total += weight;
                    }
                    "single_choice" => {
                        // Answer is option_id
                        if let Some(option_id) = answer.as_str() {
                            if let Ok(opt_uuid) = Uuid::parse_str(option_id) {
                                for opt_result in option_results.iter_mut() {
                                    if opt_result.option_id == opt_uuid {
                                        opt_result.count += 1;
                                        opt_result.weighted_count += weight;
                                        break;
                                    }
                                }
                            }
                        }
                        total_votes += 1;
                        weighted_total += weight;
                    }
                    "multiple_choice" => {
                        // Answer is array of option_ids
                        if let Some(selected) = answer.as_array() {
                            for sel in selected {
                                if let Some(option_id) = sel.as_str() {
                                    if let Ok(opt_uuid) = Uuid::parse_str(option_id) {
                                        for opt_result in option_results.iter_mut() {
                                            if opt_result.option_id == opt_uuid {
                                                opt_result.count += 1;
                                                opt_result.weighted_count += weight;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        total_votes += 1;
                        weighted_total += weight;
                    }
                    "ranked" => {
                        // Ranked choice - apply weight based on rank position
                        if let Some(ranked) = answer.as_array() {
                            let num_options = ranked.len();
                            for (rank, sel) in ranked.iter().enumerate() {
                                if let Some(option_id) = sel.as_str() {
                                    if let Ok(opt_uuid) = Uuid::parse_str(option_id) {
                                        // Higher rank = more points (reverse index)
                                        let rank_weight = (num_options - rank) as f64 * weight
                                            / num_options as f64;
                                        for opt_result in option_results.iter_mut() {
                                            if opt_result.option_id == opt_uuid {
                                                opt_result.count += 1;
                                                opt_result.weighted_count += rank_weight;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        total_votes += 1;
                        weighted_total += weight;
                    }
                    _ => {}
                }
            }
        }

        // Calculate percentages
        for opt_result in option_results.iter_mut() {
            if weighted_total > 0.0 {
                opt_result.percentage = (opt_result.weighted_count / weighted_total) * 100.0;
            }
        }

        // Determine winner (highest weighted count)
        let winner = option_results
            .iter()
            .max_by(|a, b| a.weighted_count.partial_cmp(&b.weighted_count).unwrap())
            .map(|r| r.option_id);

        Ok(QuestionResult {
            question_id: question.id,
            question_text: question.question_text.clone(),
            question_type: question.question_type.clone(),
            total_votes,
            weighted_total,
            results: option_results,
            winner,
        })
    }

    /// Get results for a vote.
    ///
    /// **Deprecated**: Use `get_poll_results_rls` with an RLS-enabled connection instead.
    /// Note: This legacy method includes additional functionality (live result calculation)
    /// that must be handled separately when using the RLS version.
    #[deprecated(
        since = "0.2.274",
        note = "Use get_poll_results_rls with RlsConnection instead"
    )]
    #[allow(deprecated)]
    pub async fn get_results(&self, vote_id: Uuid) -> Result<Option<VoteResults>, SqlxError> {
        let vote = self.find_by_id(vote_id).await?;

        match vote {
            Some(v) if v.is_closed() => {
                // Return cached results
                let results: VoteResults =
                    serde_json::from_value(v.results).unwrap_or(VoteResults {
                        vote_id,
                        participation_count: v.participation_count.unwrap_or(0),
                        eligible_count: v.eligible_count.unwrap_or(0),
                        participation_rate: 0.0,
                        quorum_met: v.quorum_met.unwrap_or(false),
                        questions: Vec::new(),
                        calculated_at: v.results_calculated_at.unwrap_or_else(Utc::now),
                    });
                Ok(Some(results))
            }
            Some(_) => {
                // Vote not closed yet, calculate live results
                let results = self.calculate_results(vote_id).await?;
                Ok(Some(results))
            }
            None => Ok(None),
        }
    }

    // ========================================================================
    // Audit Log
    // ========================================================================

    /// Create an audit log entry on a caller-supplied executor.
    ///
    /// Executor-generic mirror of [`create_audit_entry`](Self::create_audit_entry)
    /// that runs the INSERT on the passed executor instead of `self.pool`. Used by
    /// the RLS ballot-cast path so the audit row is written under the same RLS
    /// session context (`app.current_org_id`) as the ballot itself (Story 5.7).
    pub async fn create_audit_entry_rls<'e, E>(
        executor: E,
        data: CreateVoteAuditLog,
    ) -> Result<VoteAuditLog, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // Generate hash of the data
        let data_str = serde_json::to_string(&data.data).unwrap_or_default();
        let data_hash = hex::encode(Sha256::digest(data_str.as_bytes()));

        let entry = sqlx::query_as::<_, VoteAuditLog>(
            r#"
            INSERT INTO vote_audit_log (vote_id, user_id, action, data_hash, data_snapshot, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6::inet, $7)
            RETURNING *
            "#,
        )
        .bind(data.vote_id)
        .bind(data.user_id)
        .bind(&data.action)
        .bind(&data_hash)
        .bind(&data.data)
        .bind(&data.ip_address)
        .bind(&data.user_agent)
        .fetch_one(executor)
        .await?;

        Ok(entry)
    }

    /// Create an audit log entry.
    pub async fn create_audit_entry(
        &self,
        data: CreateVoteAuditLog,
    ) -> Result<VoteAuditLog, SqlxError> {
        // Generate hash of the data
        let data_str = serde_json::to_string(&data.data).unwrap_or_default();
        let data_hash = hex::encode(Sha256::digest(data_str.as_bytes()));

        let entry = sqlx::query_as::<_, VoteAuditLog>(
            r#"
            INSERT INTO vote_audit_log (vote_id, user_id, action, data_hash, data_snapshot, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6::inet, $7)
            RETURNING *
            "#,
        )
        .bind(data.vote_id)
        .bind(data.user_id)
        .bind(&data.action)
        .bind(&data_hash)
        .bind(&data.data)
        .bind(&data.ip_address)
        .bind(&data.user_agent)
        .fetch_one(&self.pool)
        .await?;

        Ok(entry)
    }

    /// Get audit log for a vote.
    pub async fn get_audit_log(&self, vote_id: Uuid) -> Result<Vec<VoteAuditLog>, SqlxError> {
        let entries = sqlx::query_as::<_, VoteAuditLog>(
            r#"
            SELECT * FROM vote_audit_log
            WHERE vote_id = $1
            ORDER BY created_at
            "#,
        )
        .bind(vote_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    // ========================================================================
    // PDF Report Stub (Story 5.7)
    // ========================================================================

    /// Generate PDF report data (stub).
    #[allow(deprecated)]
    pub async fn generate_report_data(
        &self,
        vote_id: Uuid,
    ) -> Result<crate::models::vote::VoteReportData, SqlxError> {
        let vote = self
            .find_by_id(vote_id)
            .await?
            .ok_or(SqlxError::RowNotFound)?;
        let questions = self.get_questions(vote_id).await?;
        let results = self.get_results(vote_id).await?;

        // Get participation details
        let participation_details = sqlx::query_as::<_, crate::models::vote::ParticipationDetail>(
            r#"
            SELECT
                u.designation as unit_designation,
                EXISTS (SELECT 1 FROM vote_responses vr WHERE vr.vote_id = $1 AND vr.unit_id = u.id) as voted,
                u.ownership_share as vote_weight
            FROM units u
            JOIN unit_residents ur ON u.id = ur.unit_id
            WHERE u.building_id = $2
              AND ur.resident_type = 'owner'
              AND ur.move_out_date IS NULL
            ORDER BY u.designation
            "#,
        )
        .bind(vote_id)
        .bind(vote.building_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(crate::models::vote::VoteReportData {
            vote,
            questions,
            results,
            participation_details,
            generated_at: Utc::now(),
        })
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Count votes by organization.
    pub async fn count_by_organization(&self, org_id: Uuid) -> Result<i64, SqlxError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM votes WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    /// Count active votes by building.
    pub async fn count_active_by_building(&self, building_id: Uuid) -> Result<i64, SqlxError> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM votes
            WHERE building_id = $1 AND status = 'active'
            "#,
        )
        .bind(building_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    // ========================================================================
    // Epic 55: Reporting Analytics
    // ========================================================================

    /// Get voting participation report data (Epic 55, Story 55.2).
    pub async fn get_participation_report(
        &self,
        organization_id: Uuid,
        building_id: Option<Uuid>,
        from_date: chrono::NaiveDate,
        to_date: chrono::NaiveDate,
    ) -> Result<Vec<crate::models::reports::VoteParticipationDetail>, SqlxError> {
        let rows = sqlx::query_as::<_, crate::models::reports::VoteParticipationDetail>(
            r#"
            SELECT
                v.id as vote_id,
                v.title,
                v.status::text as status,
                v.start_at::text as start_at,
                v.end_at::text as end_at,
                COALESCE(
                    (SELECT COUNT(DISTINCT ur.user_id) FROM unit_residents ur
                     JOIN units u ON ur.unit_id = u.id
                     WHERE u.building_id = v.building_id AND ur.is_active = true),
                    0
                )::int8 as eligible_count,
                COALESCE(
                    (SELECT COUNT(DISTINCT vr.user_id) FROM vote_responses vr WHERE vr.vote_id = v.id),
                    0
                )::int8 as response_count,
                CASE
                    WHEN (SELECT COUNT(DISTINCT ur.user_id) FROM unit_residents ur
                          JOIN units u ON ur.unit_id = u.id
                          WHERE u.building_id = v.building_id AND ur.is_active = true) > 0
                    THEN (
                        (SELECT COUNT(DISTINCT vr.user_id) FROM vote_responses vr WHERE vr.vote_id = v.id)::float8 /
                        (SELECT COUNT(DISTINCT ur.user_id) FROM unit_residents ur
                         JOIN units u ON ur.unit_id = u.id
                         WHERE u.building_id = v.building_id AND ur.is_active = true)::float8 * 100.0
                    )
                    ELSE 0.0
                END as participation_rate,
                v.quorum_percentage as quorum_required,
                CASE
                    WHEN v.quorum_percentage IS NULL THEN true
                    WHEN (SELECT COUNT(DISTINCT ur.user_id) FROM unit_residents ur
                          JOIN units u ON ur.unit_id = u.id
                          WHERE u.building_id = v.building_id AND ur.is_active = true) = 0 THEN false
                    ELSE (
                        (SELECT COUNT(DISTINCT vr.user_id) FROM vote_responses vr WHERE vr.vote_id = v.id)::float8 /
                        (SELECT COUNT(DISTINCT ur.user_id) FROM unit_residents ur
                         JOIN units u ON ur.unit_id = u.id
                         WHERE u.building_id = v.building_id AND ur.is_active = true)::float8 * 100.0
                    ) >= v.quorum_percentage
                END as quorum_reached
            FROM votes v
            WHERE v.organization_id = $1
              AND v.created_at >= $2 AND v.created_at <= $3
              AND ($4::uuid IS NULL OR v.building_id = $4)
            ORDER BY v.created_at DESC
            "#,
        )
        .bind(organization_id)
        .bind(from_date)
        .bind(to_date)
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
