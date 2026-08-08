//! Platform Admin repository (Epic 10B).
//!
//! Repository for platform-wide administrative operations including
//! organization management with cross-tenant queries.
//!
//! # Why runtime `sqlx::query` instead of the compile-time `query!` macros (#1851)
//!
//! Every query here uses the runtime, *unchecked* `sqlx::query` / `query_as` /
//! `query_scalar` forms rather than the compile-time-checked `query!` family.
//! This is a **deliberate, repo-wide convention**, not an oversight: the
//! checked macros require a live DB connection (or a committed `.sqlx/` offline
//! cache via `cargo sqlx prepare`) at build time, but this workspace compiles
//! DB-free — CI runs `SQLX_OFFLINE=true` and there is **no `.sqlx/` cache** in
//! the tree (see the same rationale documented at
//! `rental.rs::find_airbnb_connection_by_listing_id`). Introducing a `query!`
//! here in isolation would fail the `check` / `fmt-clippy` gates, which compile
//! with `SQLX_OFFLINE=true`.
//!
//! Because the SQL is therefore validated at *runtime*, the `refresh_tokens`
//! session queries (`get_user_sessions`, `revoke_user_sessions`, and the
//! `active_sessions` count) are guarded against column/typo regressions by the
//! DB-backed `#[sqlx::test]` in
//! `backend/crates/db/tests/support_data_session_columns_tests.rs` (added with
//! the #1829 fix) — keep that test in lock-step with any edit to those queries.
//! Migrating the whole repo to the checked macros is tracked as part of
//! establishing a workspace-wide `cargo sqlx prepare` offline-cache workflow,
//! not a per-file change.

use crate::models::fault::StatusCount as FaultStatusCount;
use crate::models::platform_admin::{
    AdminOrganizationDetail, OrganizationDetailMetrics, OrganizationMetrics, PlatformSettings,
};
use crate::models::Organization;
use crate::repositories::fault::fault_counts_by_status;
use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for platform admin operations.
#[derive(Clone)]
pub struct PlatformAdminRepository {
    pool: DbPool,
}

impl PlatformAdminRepository {
    /// Create a new PlatformAdminRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Read the singleton platform settings row (migration 00228).
    ///
    /// The row is seeded by the migration, so this always returns a row on a
    /// migrated database.
    pub async fn get_platform_settings(&self) -> Result<PlatformSettings, SqlxError> {
        sqlx::query_as::<_, PlatformSettings>(
            r#"
            SELECT maintenance_mode, signup_enabled, support_email, updated_at, updated_by
            FROM platform_settings
            WHERE id = 1
            "#,
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Patch the singleton platform settings row. `None` fields are left
    /// unchanged (partial `PATCH` semantics). Returns the updated row.
    pub async fn update_platform_settings(
        &self,
        maintenance_mode: Option<bool>,
        signup_enabled: Option<bool>,
        support_email: Option<String>,
        actor: Uuid,
    ) -> Result<PlatformSettings, SqlxError> {
        sqlx::query_as::<_, PlatformSettings>(
            r#"
            UPDATE platform_settings
            SET maintenance_mode = COALESCE($1, maintenance_mode),
                signup_enabled   = COALESCE($2, signup_enabled),
                support_email    = COALESCE($3, support_email),
                updated_by       = $4,
                updated_at       = NOW()
            WHERE id = 1
            RETURNING maintenance_mode, signup_enabled, support_email, updated_at, updated_by
            "#,
        )
        .bind(maintenance_mode)
        .bind(signup_enabled)
        .bind(support_email)
        .bind(actor)
        .fetch_one(&self.pool)
        .await
    }

    /// List all organizations with metrics (platform admin view).
    /// This is a cross-tenant query that bypasses RLS.
    pub async fn list_organizations_with_metrics(
        &self,
        offset: i64,
        limit: i64,
        status_filter: Option<&str>,
        search: Option<&str>,
    ) -> Result<(Vec<OrganizationMetrics>, i64), SqlxError> {
        // Build dynamic WHERE clause
        let mut conditions = vec!["status != 'deleted'".to_string()];
        let mut param_idx = 2; // $1 and $2 are limit and offset

        if status_filter.is_some() {
            param_idx += 1;
            conditions.push(format!("status = ${}", param_idx));
        }

        if search.is_some() {
            param_idx += 1;
            conditions.push(format!(
                "(LOWER(name) LIKE '%' || LOWER(${}::text) || '%' OR LOWER(slug) LIKE '%' || LOWER(${}::text) || '%')",
                param_idx, param_idx
            ));
        }

        let where_clause = conditions.join(" AND ");

        let count_query = format!(
            "SELECT COUNT(*) FROM organization_metrics WHERE {}",
            where_clause
        );
        let data_query = format!(
            r#"
            SELECT
                organization_id, name, slug, status, created_at, updated_at,
                suspended_at, suspended_by, suspension_reason,
                member_count, active_member_count, building_count, unit_count
            FROM organization_metrics
            WHERE {}
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            where_clause
        );

        // Execute count query
        let mut count_q = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(count_query));
        if let Some(status) = status_filter {
            count_q = count_q.bind(status);
        }
        if let Some(s) = search {
            count_q = count_q.bind(s);
        }
        let total = count_q.fetch_one(&self.pool).await?;

        // Execute data query
        let mut data_q = sqlx::query_as::<_, OrganizationMetrics>(sqlx::AssertSqlSafe(data_query))
            .bind(limit)
            .bind(offset);
        if let Some(status) = status_filter {
            data_q = data_q.bind(status);
        }
        if let Some(s) = search {
            data_q = data_q.bind(s);
        }
        let orgs = data_q.fetch_all(&self.pool).await?;

        Ok((orgs, total))
    }

    /// Get organization details with metrics.
    pub async fn get_organization_detail(
        &self,
        org_id: Uuid,
    ) -> Result<Option<AdminOrganizationDetail>, SqlxError> {
        // Get organization base data
        let org = sqlx::query_as::<_, Organization>(
            r#"
            SELECT * FROM organizations WHERE id = $1 AND status != 'deleted'
            "#,
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        let org = match org {
            Some(o) => o,
            None => return Ok(None),
        };

        // Get metrics
        let metrics = sqlx::query_as::<_, OrganizationMetrics>(
            r#"
            SELECT
                organization_id, name, slug, status, created_at, updated_at,
                suspended_at, suspended_by, suspension_reason,
                member_count, active_member_count, building_count, unit_count
            FROM organization_metrics
            WHERE organization_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        // Get suspension info from organizations table (may have been added by migration)
        let suspension_info =
            sqlx::query_as::<_, (Option<DateTime<Utc>>, Option<Uuid>, Option<String>)>(
                r#"
            SELECT suspended_at, suspended_by, suspension_reason
            FROM organizations
            WHERE id = $1
            "#,
            )
            .bind(org_id)
            .fetch_optional(&self.pool)
            .await?
            .unwrap_or((None, None, None));

        let detail = AdminOrganizationDetail {
            id: org.id,
            name: org.name,
            slug: org.slug,
            contact_email: org.contact_email,
            logo_url: org.logo_url,
            status: org.status,
            created_at: org.created_at,
            updated_at: org.updated_at,
            suspended_at: suspension_info.0,
            suspended_by: suspension_info.1,
            suspension_reason: suspension_info.2,
            metrics: metrics
                .map(|m| OrganizationDetailMetrics {
                    member_count: m.member_count,
                    active_member_count: m.active_member_count,
                    building_count: m.building_count,
                    unit_count: m.unit_count,
                })
                .unwrap_or(OrganizationDetailMetrics {
                    member_count: 0,
                    active_member_count: 0,
                    building_count: 0,
                    unit_count: 0,
                }),
        };

        Ok(Some(detail))
    }

    /// Suspend an organization with reason and admin tracking.
    pub async fn suspend_organization(
        &self,
        org_id: Uuid,
        admin_id: Uuid,
        reason: &str,
    ) -> Result<Option<Organization>, SqlxError> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET
                status = 'suspended',
                suspended_at = NOW(),
                suspended_by = $2,
                suspension_reason = $3,
                updated_at = NOW()
            WHERE id = $1 AND status = 'active'
            RETURNING *
            "#,
        )
        .bind(org_id)
        .bind(admin_id)
        .bind(reason)
        .fetch_optional(&self.pool)
        .await?;

        Ok(org)
    }

    /// Reactivate a suspended organization.
    pub async fn reactivate_organization(
        &self,
        org_id: Uuid,
    ) -> Result<Option<Organization>, SqlxError> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET
                status = 'active',
                suspended_at = NULL,
                suspended_by = NULL,
                suspension_reason = NULL,
                updated_at = NOW()
            WHERE id = $1 AND status = 'suspended'
            RETURNING *
            "#,
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(org)
    }

    /// Get all active session tokens for organization members.
    /// Used for cascade session invalidation on org suspension.
    pub async fn get_org_member_user_ids(&self, org_id: Uuid) -> Result<Vec<Uuid>, SqlxError> {
        let user_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT user_id FROM organization_members
            WHERE organization_id = $1 AND status = 'active'
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(user_ids)
    }

    /// Get platform statistics summary.
    ///
    /// Note: `total_buildings` / `total_units` count all rows. The `buildings`
    /// and `units` tables have no `deleted_at` column and no soft-delete
    /// mechanism — their `status` column is constrained to `'active'` or
    /// `'archived'`, neither of which represents deletion. Counting all rows
    /// matches the schema honestly; the previous `WHERE deleted_at IS NULL`
    /// predicate referenced a non-existent column and caused this endpoint
    /// to 500 on every load.
    pub async fn get_platform_stats(&self) -> Result<PlatformStats, SqlxError> {
        let stats = sqlx::query_as::<_, PlatformStats>(
            r#"
            SELECT
                (SELECT COUNT(*) FROM organizations WHERE status = 'active') as active_orgs,
                (SELECT COUNT(*) FROM organizations WHERE status = 'suspended') as suspended_orgs,
                (SELECT COUNT(*) FROM users WHERE status = 'active') as active_users,
                (SELECT COUNT(*) FROM buildings) as total_buildings,
                (SELECT COUNT(*) FROM units) as total_units
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(stats)
    }

    // ==================== Support Data Access (Story 10B.5) ====================

    /// Search users for support purposes.
    pub async fn search_users_for_support(
        &self,
        query: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<SupportUserInfo>, i64), SqlxError> {
        // Build dynamic WHERE clause with proper parameter binding
        let mut conditions = Vec::new();
        let mut param_idx = 2; // $1 and $2 are limit and offset

        if status.is_some() {
            param_idx += 1;
            conditions.push(format!("u.status = ${}", param_idx));
        }

        if query.is_some() {
            param_idx += 1;
            // Issue #1008: `users` has a single `name` column (no
            // display_name/first_name/last_name); search email + name.
            conditions.push(format!(
                "(LOWER(u.email) LIKE '%' || LOWER(${}::text) || '%' OR LOWER(u.name) LIKE '%' || LOWER(${}::text) || '%')",
                param_idx, param_idx
            ));
        }

        let where_clause = if conditions.is_empty() {
            "1=1".to_string()
        } else {
            conditions.join(" AND ")
        };

        let count_query = format!("SELECT COUNT(*) FROM users u WHERE {}", where_clause);

        let data_query = format!(
            r#"
            SELECT u.id, u.email, u.name AS display_name, NULL::text AS first_name,
                   NULL::text AS last_name, u.status,
                   (u.email_verified_at IS NOT NULL) AS email_verified,
                   u.created_at, u.updated_at, NULL::timestamptz AS last_login_at
            FROM users u
            WHERE {}
            ORDER BY u.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            where_clause
        );

        // Execute count query with bound parameters
        let mut count_q = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(count_query));
        if let Some(s) = status {
            count_q = count_q.bind(s);
        }
        if let Some(q) = query {
            count_q = count_q.bind(q);
        }
        let total = count_q.fetch_one(&self.pool).await?;

        // Execute data query with bound parameters
        let mut data_q = sqlx::query_as::<_, SupportUserInfo>(sqlx::AssertSqlSafe(data_query))
            .bind(limit)
            .bind(offset);
        if let Some(s) = status {
            data_q = data_q.bind(s);
        }
        if let Some(q) = query {
            data_q = data_q.bind(q);
        }
        let users = data_q.fetch_all(&self.pool).await?;

        Ok((users, total))
    }

    /// Get user details for support.
    pub async fn get_user_for_support(
        &self,
        user_id: Uuid,
    ) -> Result<Option<SupportUserInfo>, SqlxError> {
        let user = sqlx::query_as::<_, SupportUserInfo>(
            r#"
            SELECT id, email, name AS display_name, NULL::text AS first_name,
                   NULL::text AS last_name, status,
                   (email_verified_at IS NOT NULL) AS email_verified,
                   created_at, updated_at, NULL::timestamptz AS last_login_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Get user organization memberships for support.
    pub async fn get_user_memberships(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<SupportUserMembership>, SqlxError> {
        let memberships = sqlx::query_as::<_, SupportUserMembership>(
            r#"
            SELECT om.organization_id, o.name as organization_name,
                   COALESCE(r.name, 'Member') as role_name, om.created_at as joined_at
            FROM organization_members om
            JOIN organizations o ON o.id = om.organization_id
            LEFT JOIN roles r ON r.id = om.role_id
            WHERE om.user_id = $1 AND om.status = 'active'
            ORDER BY om.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(memberships)
    }

    /// Get user active sessions for support.
    pub async fn get_user_sessions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<SupportUserSession>, SqlxError> {
        let sessions = sqlx::query_as::<_, SupportUserSession>(
            r#"
            SELECT id, created_at, expires_at, last_used_at, user_agent, ip_address
            FROM refresh_tokens
            WHERE user_id = $1 AND revoked_at IS NULL AND expires_at > NOW()
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    /// Get user activity log for support.
    pub async fn get_user_activity_log(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<SupportActivityLog>, SqlxError> {
        let logs = sqlx::query_as::<_, SupportActivityLog>(
            r#"
            SELECT id, action, resource_type, resource_id, details, created_at
            FROM audit_logs
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    /// Revoke all sessions for a user (support action).
    pub async fn revoke_user_sessions(&self, user_id: Uuid) -> Result<i64, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked_at = NOW()
            WHERE user_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }
}

/// Platform-wide statistics.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct PlatformStats {
    pub active_orgs: i64,
    pub suspended_orgs: i64,
    pub active_users: i64,
    pub total_buildings: i64,
    pub total_units: i64,
}

/// User info for support purposes (read-only).
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SupportUserInfo {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub status: String,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

/// User organization membership for support view.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SupportUserMembership {
    pub organization_id: Uuid,
    pub organization_name: String,
    pub role_name: String,
    pub joined_at: DateTime<Utc>,
}

/// User session info for support.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SupportUserSession {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}

/// Activity log entry for support.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SupportActivityLog {
    pub id: Uuid,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Tenant diagnostics returned by `GET /api/v1/platform-admin/support-data`.
///
/// Aggregates user counts, active-session count, and fault status breakdown
/// across the entire platform (cross-tenant, bypasses RLS).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SupportData {
    /// Total number of organisations (tenants) in the system, all statuses.
    pub total_orgs: i64,
    /// Total number of users in the system.
    pub total_users: i64,
    /// Number of users with status `'active'`.
    pub active_users: i64,
    /// Number of users with status `'pending'` (email not yet verified).
    pub pending_users: i64,
    /// Number of users with status `'suspended'`.
    pub suspended_users: i64,
    /// Non-expired, non-revoked refresh tokens — proxy for active sessions.
    pub active_sessions: i64,
    /// Total faults across all organisations.
    pub total_faults: i64,
    /// Per-status fault counts, ordered by count descending.
    pub fault_by_status: Vec<FaultStatusCount>,
}

impl PlatformAdminRepository {
    /// Get platform tenant diagnostics for the support-data endpoint.
    ///
    /// Runs five focused cross-tenant queries:
    ///   1. Total organisation count.
    ///   2. User counts grouped by `status`.
    ///   3. Active session count (refresh tokens with `revoked_at IS NULL` and
    ///      not yet expired).
    ///   4. Total fault count.
    ///   5. Fault counts per `status` enum value.
    ///
    /// All queries run inside a single `REPEATABLE READ` transaction so the
    /// aggregate counters reflect a consistent snapshot — without this,
    /// concurrent writes between queries can make `total_faults` disagree
    /// with the sum of `fault_by_status`, or `active_sessions` lag behind
    /// `total_users`. See issue #628.
    ///
    /// All queries bypass RLS — caller must hold `AuditRead` capability.
    pub async fn get_support_data(&self) -> Result<SupportData, SqlxError> {
        let mut tx = self.pool.begin().await?;

        // Pin all subsequent reads in this tx to a single MVCC snapshot.
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await?;

        // 1. Total organisation count (all statuses)
        let total_orgs: i64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM organizations")
            .fetch_one(&mut *tx)
            .await?;

        // 2. User counts per status
        let user_rows = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT status, COUNT(*) AS cnt
            FROM users
            GROUP BY status
            "#,
        )
        .fetch_all(&mut *tx)
        .await?;

        let mut total_users: i64 = 0;
        let mut active_users: i64 = 0;
        let mut pending_users: i64 = 0;
        let mut suspended_users: i64 = 0;
        for (status, cnt) in &user_rows {
            total_users += cnt;
            match status.as_str() {
                "active" => active_users = *cnt,
                "pending" => pending_users = *cnt,
                "suspended" => suspended_users = *cnt,
                _ => {}
            }
        }

        // 3. Active sessions — non-revoked, non-expired refresh tokens
        let active_sessions: i64 = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM refresh_tokens
            WHERE revoked_at IS NULL AND expires_at > NOW()
            "#,
        )
        .fetch_one(&mut *tx)
        .await?;

        // 4. Total fault count
        let total_faults: i64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM faults")
            .fetch_one(&mut *tx)
            .await?;

        // 5. Fault counts per status — canonical KPI definition shared with the
        //    owner/portfolio fault statistics (single source of truth). Platform
        //    scope => no organisation/building filter.
        let fault_by_status = fault_counts_by_status(&mut *tx, None, None).await?;

        tx.commit().await?;

        Ok(SupportData {
            total_orgs,
            total_users,
            active_users,
            pending_users,
            suspended_users,
            active_sessions,
            total_faults,
            fault_by_status,
        })
    }
}

// ==================== Support Tooling Analytics Events (#635) ====================

/// Identifies which support-tooling action was performed.
///
/// These three variants map 1-to-1 onto the `event_kind` CHECK constraint
/// defined in migration 00163.  They are stored as snake_case strings so the
/// DB column is human-readable without a lookup table.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SupportToolingEventKind {
    /// Admin opened / refreshed the Support Data overview page.
    SupportDataViewed,
    /// Admin ran a per-user lookup or free-text search.
    SupportUserSearched,
    /// Admin revoked all active sessions for a target user.
    SupportSessionsRevoked,
}

impl SupportToolingEventKind {
    /// Stable database string.  MUST match the `event_kind` CHECK in
    /// migration 00163.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SupportDataViewed => "support_data_viewed",
            Self::SupportUserSearched => "support_user_searched",
            Self::SupportSessionsRevoked => "support_sessions_revoked",
        }
    }
}

impl std::fmt::Display for SupportToolingEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Props for `support_data_viewed`.
///
/// Captures the snapshot values visible to the admin at the time of the page
/// visit so support-tooling usage can be correlated with the platform state
/// that was shown.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SupportDataViewedProps {
    /// Platform-wide tenant count (total organisations, all statuses).
    pub tenant_count: i64,
    /// Total fault count across all organisations at time of view.
    pub fault_total: i64,
}

/// Props for `support_user_searched`.
///
/// Records search metadata so repeated lookups can be spotted in an audit
/// query.  The raw query string is NOT stored — it commonly contains email
/// addresses (PII); only the character count is persisted.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SupportUserSearchedProps {
    /// Character count of the free-text query, or `None` for unfiltered listings.
    /// The literal string is deliberately omitted to avoid storing PII (emails).
    pub query_length: Option<i64>,
    /// Status filter applied, if any.
    pub status_filter: Option<String>,
    /// Number of results returned.
    pub result_count: i64,
}

/// Props for `support_sessions_revoked`.
///
/// Links the revocation to the targeted user so the event is meaningful
/// without joining to other tables.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SupportSessionsRevokedProps {
    /// The user whose sessions were revoked.
    pub target_user_id: Uuid,
    /// Number of sessions actually revoked in this operation.
    pub revoked_count: i64,
}

/// A persisted row from `support_tooling_events`.
///
/// Returned by `log_support_tooling_event` so callers have the row id and
/// timestamp for logging or structured spans.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct SupportToolingEventRow {
    pub id: Uuid,
    pub event_kind: String,
    pub admin_user_id: Uuid,
    pub props: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}

impl PlatformAdminRepository {
    /// Append a support-tooling analytics event to `support_tooling_events`.
    ///
    /// The method is deliberately fire-and-forget in production callers: a
    /// failure to persist a tracking event must **not** fail the user-facing
    /// response.  Callers are expected to log the error and continue.
    ///
    /// # Arguments
    /// * `admin_user_id` — UUID of the platform admin who performed the action.
    /// * `kind`          — Which of the three event kinds fired.
    /// * `props`         — Structured payload serialised to JSONB.
    pub async fn log_support_tooling_event(
        &self,
        admin_user_id: Uuid,
        kind: SupportToolingEventKind,
        props: serde_json::Value,
    ) -> Result<SupportToolingEventRow, SqlxError> {
        sqlx::query_as::<_, SupportToolingEventRow>(
            r#"
            INSERT INTO support_tooling_events (event_kind, admin_user_id, props)
            VALUES ($1, $2, $3)
            RETURNING id, event_kind, admin_user_id, props, occurred_at
            "#,
        )
        .bind(kind.as_str())
        .bind(admin_user_id)
        .bind(props)
        .fetch_one(&self.pool)
        .await
    }

    /// Prune `support_tooling_events` older than the retention period, returning
    /// the number of rows deleted.
    ///
    /// This is the retention (TTL) entry point for the append-only admin audit
    /// trail (migration `00222_support_tooling_events_retention.sql`). It calls
    /// the DB-native `cleanup_old_support_tooling_events(retention_days)`
    /// function, mirroring the tracing / health-monitoring retention jobs
    /// (`cleanup_old_traces`, `cleanup_old_health_check_results`). The function
    /// opens the sanctioned retention path so the immutability trigger permits
    /// these — and only these — deletes; every other `UPDATE`/`DELETE` on the
    /// table remains rejected.
    ///
    /// `retention_days` defaults to 730 (24 months) at the SQL layer; callers
    /// pass an explicit value here so the policy is visible at the call site.
    pub async fn cleanup_old_support_tooling_events(
        &self,
        retention_days: i32,
    ) -> Result<i64, SqlxError> {
        let deleted = sqlx::query_scalar::<_, i64>("SELECT cleanup_old_support_tooling_events($1)")
            .bind(retention_days)
            .fetch_one(&self.pool)
            .await?;

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_stats_struct() {
        let stats = PlatformStats {
            active_orgs: 10,
            suspended_orgs: 2,
            active_users: 100,
            total_buildings: 50,
            total_units: 500,
        };

        assert_eq!(stats.active_orgs, 10);
        assert_eq!(stats.suspended_orgs, 2);
    }

    // ==================== SupportToolingEventKind unit tests ====================

    #[test]
    fn event_kind_strings_are_stable_and_match_db_constraint() {
        assert_eq!(
            SupportToolingEventKind::SupportDataViewed.as_str(),
            "support_data_viewed"
        );
        assert_eq!(
            SupportToolingEventKind::SupportUserSearched.as_str(),
            "support_user_searched"
        );
        assert_eq!(
            SupportToolingEventKind::SupportSessionsRevoked.as_str(),
            "support_sessions_revoked"
        );
    }

    #[test]
    fn event_kind_display_matches_as_str() {
        for kind in [
            SupportToolingEventKind::SupportDataViewed,
            SupportToolingEventKind::SupportUserSearched,
            SupportToolingEventKind::SupportSessionsRevoked,
        ] {
            assert_eq!(format!("{kind}"), kind.as_str());
        }
    }

    #[test]
    fn support_data_viewed_props_round_trips_json() {
        let props = SupportDataViewedProps {
            tenant_count: 42,
            fault_total: 1234,
        };
        let json = serde_json::to_value(&props).expect("serialize");
        assert_eq!(json["tenant_count"], 42);
        assert_eq!(json["fault_total"], 1234);
        let back: SupportDataViewedProps = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back.tenant_count, 42);
        assert_eq!(back.fault_total, 1234);
    }

    #[test]
    fn support_user_searched_props_round_trips_json() {
        let props = SupportUserSearchedProps {
            query_length: Some(17), // len("alice@example.com") — no PII stored
            status_filter: Some("active".into()),
            result_count: 3,
        };
        let json = serde_json::to_value(&props).expect("serialize");
        assert_eq!(json["query_length"], 17);
        assert_eq!(json["status_filter"], "active");
        assert_eq!(json["result_count"], 3);
    }

    #[test]
    fn support_sessions_revoked_props_round_trips_json() {
        let target = Uuid::new_v4();
        let props = SupportSessionsRevokedProps {
            target_user_id: target,
            revoked_count: 5,
        };
        let json = serde_json::to_value(&props).expect("serialize");
        assert_eq!(json["target_user_id"].as_str().unwrap(), target.to_string());
        assert_eq!(json["revoked_count"], 5);
    }

    #[test]
    fn event_kind_serde_round_trip() {
        let kinds = [
            SupportToolingEventKind::SupportDataViewed,
            SupportToolingEventKind::SupportUserSearched,
            SupportToolingEventKind::SupportSessionsRevoked,
        ];
        for kind in kinds {
            let json = serde_json::to_value(kind).expect("serialize");
            let back: SupportToolingEventKind = serde_json::from_value(json).expect("deserialize");
            assert_eq!(kind, back);
        }
    }
}
