//! Delegation repository (Epic 3, Story 3.4).

use crate::models::delegation::{
    CreateDelegation, Delegation, DelegationAuditLog, DelegationSummary, UpdateDelegation,
};
use crate::DbPool;
use chrono::Utc;
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Explicit column list for `Delegation` rows.
///
/// #859: sqlx 0.9 cannot decode a Postgres ENUM column into a Rust `String`,
/// so the `status` (`delegation_status`) enum is cast to text. The same applies
/// to `scopes`, whose column type is `delegation_scope[]` (an array of the enum)
/// — it must be cast to `text[]` to decode into `Vec<String>`. The rest map 1:1
/// to the struct fields. Used instead of `SELECT *` / `RETURNING *`.
const DELEGATION_COLUMNS: &str = "id, owner_user_id, delegate_user_id, unit_id, \
     scopes::text[] AS scopes, status::text AS status, start_date, end_date, \
     invitation_token, invitation_sent_at, accepted_at, declined_at, revoked_at, \
     revoked_reason, created_at, updated_at";

/// Repository for delegation operations.
#[derive(Clone)]
pub struct DelegationRepository {
    pool: DbPool,
}

impl DelegationRepository {
    /// Create a new DelegationRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new delegation.
    pub async fn create(
        &self,
        owner_user_id: Uuid,
        data: CreateDelegation,
    ) -> Result<Delegation, SqlxError> {
        let start_date = data.start_date.unwrap_or_else(|| Utc::now().date_naive());
        let invitation_token = generate_token();

        let delegation = sqlx::query_as::<_, Delegation>(sqlx::AssertSqlSafe(format!(
            r#"
            INSERT INTO delegations (
                owner_user_id, delegate_user_id, unit_id, scopes,
                status, start_date, end_date, invitation_token, invitation_sent_at
            )
            VALUES ($1, $2, $3, $4::text[]::delegation_scope[], 'pending', $5, $6, $7, NOW())
            RETURNING {DELEGATION_COLUMNS}
            "#
        )))
        .bind(owner_user_id)
        .bind(data.delegate_user_id)
        .bind(data.unit_id)
        .bind(&data.scopes)
        .bind(start_date)
        .bind(data.end_date)
        .bind(&invitation_token)
        .fetch_one(&self.pool)
        .await?;

        // Log creation
        self.log_action(delegation.id, "created", Some(owner_user_id), None)
            .await?;

        Ok(delegation)
    }

    /// Find delegation by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Delegation>, SqlxError> {
        let delegation = sqlx::query_as::<_, Delegation>(sqlx::AssertSqlSafe(format!(
            r#"SELECT {DELEGATION_COLUMNS} FROM delegations WHERE id = $1"#
        )))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(delegation)
    }

    /// Find delegation by invitation token.
    pub async fn find_by_token(&self, token: &str) -> Result<Option<Delegation>, SqlxError> {
        let delegation = sqlx::query_as::<_, Delegation>(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT {DELEGATION_COLUMNS} FROM delegations
            WHERE invitation_token = $1 AND status = 'pending'
            "#
        )))
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(delegation)
    }

    /// Find delegations by owner.
    pub async fn find_by_owner(
        &self,
        owner_user_id: Uuid,
    ) -> Result<Vec<DelegationSummary>, SqlxError> {
        // #859: cast the `status` enum to text so sqlx 0.9 can decode it as String.
        let delegations = sqlx::query_as::<_, DelegationSummary>(
            r#"
            SELECT id, owner_user_id, delegate_user_id, unit_id, scopes::text[] AS scopes, status::text AS status
            FROM delegations
            WHERE owner_user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(owner_user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(delegations)
    }

    /// Find delegations by delegate.
    pub async fn find_by_delegate(
        &self,
        delegate_user_id: Uuid,
    ) -> Result<Vec<DelegationSummary>, SqlxError> {
        // #859: cast the `status` enum to text so sqlx 0.9 can decode it as String.
        let delegations = sqlx::query_as::<_, DelegationSummary>(
            r#"
            SELECT id, owner_user_id, delegate_user_id, unit_id, scopes::text[] AS scopes, status::text AS status
            FROM delegations
            WHERE delegate_user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(delegate_user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(delegations)
    }

    /// Find active delegations for a delegate.
    pub async fn find_active_for_delegate(
        &self,
        delegate_user_id: Uuid,
    ) -> Result<Vec<Delegation>, SqlxError> {
        let delegations = sqlx::query_as::<_, Delegation>(sqlx::AssertSqlSafe(format!(
            r#"
            SELECT {DELEGATION_COLUMNS} FROM delegations
            WHERE delegate_user_id = $1
              AND status = 'active'
              AND start_date <= CURRENT_DATE
              AND (end_date IS NULL OR end_date >= CURRENT_DATE)
            ORDER BY created_at DESC
            "#
        )))
        .bind(delegate_user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(delegations)
    }

    /// Accept a delegation.
    pub async fn accept(
        &self,
        id: Uuid,
        delegate_user_id: Uuid,
    ) -> Result<Option<Delegation>, SqlxError> {
        let delegation = sqlx::query_as::<_, Delegation>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE delegations
            SET status = 'active', accepted_at = NOW(), invitation_token = NULL, updated_at = NOW()
            WHERE id = $1 AND delegate_user_id = $2 AND status = 'pending'
            RETURNING {DELEGATION_COLUMNS}
            "#
        )))
        .bind(id)
        .bind(delegate_user_id)
        .fetch_optional(&self.pool)
        .await?;

        if delegation.is_some() {
            self.log_action(id, "accepted", Some(delegate_user_id), None)
                .await?;
        }

        Ok(delegation)
    }

    /// Decline a delegation.
    pub async fn decline(
        &self,
        id: Uuid,
        delegate_user_id: Uuid,
    ) -> Result<Option<Delegation>, SqlxError> {
        let delegation = sqlx::query_as::<_, Delegation>(sqlx::AssertSqlSafe(format!(r#"
            UPDATE delegations
            SET status = 'declined', declined_at = NOW(), invitation_token = NULL, updated_at = NOW()
            WHERE id = $1 AND delegate_user_id = $2 AND status = 'pending'
            RETURNING {DELEGATION_COLUMNS}
            "#)))
        .bind(id)
        .bind(delegate_user_id)
        .fetch_optional(&self.pool)
        .await?;

        if delegation.is_some() {
            self.log_action(id, "declined", Some(delegate_user_id), None)
                .await?;
        }

        Ok(delegation)
    }

    /// Revoke a delegation.
    pub async fn revoke(
        &self,
        id: Uuid,
        owner_user_id: Uuid,
        reason: Option<&str>,
    ) -> Result<Option<Delegation>, SqlxError> {
        let delegation = sqlx::query_as::<_, Delegation>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE delegations
            SET status = 'revoked', revoked_at = NOW(), revoked_reason = $3, updated_at = NOW()
            WHERE id = $1 AND owner_user_id = $2 AND status IN ('pending', 'active')
            RETURNING {DELEGATION_COLUMNS}
            "#
        )))
        .bind(id)
        .bind(owner_user_id)
        .bind(reason)
        .fetch_optional(&self.pool)
        .await?;

        if delegation.is_some() {
            self.log_action(
                id,
                "revoked",
                Some(owner_user_id),
                reason.map(|r| serde_json::json!({"reason": r})),
            )
            .await?;
        }

        Ok(delegation)
    }

    /// Update a delegation.
    pub async fn update(
        &self,
        id: Uuid,
        owner_user_id: Uuid,
        data: UpdateDelegation,
    ) -> Result<Option<Delegation>, SqlxError> {
        let delegation = sqlx::query_as::<_, Delegation>(sqlx::AssertSqlSafe(format!(
            r#"
            UPDATE delegations
            SET
                scopes = COALESCE($3::text[]::delegation_scope[], scopes),
                end_date = COALESCE($4, end_date),
                updated_at = NOW()
            WHERE id = $1 AND owner_user_id = $2 AND status IN ('pending', 'active')
            RETURNING {DELEGATION_COLUMNS}
            "#
        )))
        .bind(id)
        .bind(owner_user_id)
        .bind(&data.scopes)
        .bind(data.end_date)
        .fetch_optional(&self.pool)
        .await?;

        Ok(delegation)
    }

    /// Check if user has delegation for a scope.
    pub async fn has_delegation(
        &self,
        delegate_user_id: Uuid,
        unit_id: Uuid,
        scope: &str,
    ) -> Result<bool, SqlxError> {
        let exists: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM delegations
                WHERE delegate_user_id = $1
                  AND status = 'active'
                  AND (unit_id = $2 OR unit_id IS NULL)
                  AND ($3 = ANY(scopes::text[]) OR 'all' = ANY(scopes::text[]))
                  AND start_date <= CURRENT_DATE
                  AND (end_date IS NULL OR end_date >= CURRENT_DATE)
            )
            "#,
        )
        .bind(delegate_user_id)
        .bind(unit_id)
        .bind(scope)
        .fetch_one(&self.pool)
        .await?;

        Ok(exists.0)
    }

    /// Log a delegation action.
    async fn log_action(
        &self,
        delegation_id: Uuid,
        action: &str,
        actor_user_id: Option<Uuid>,
        details: Option<serde_json::Value>,
    ) -> Result<(), SqlxError> {
        sqlx::query(
            r#"
            INSERT INTO delegation_audit_log (delegation_id, action, actor_user_id, details)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(delegation_id)
        .bind(action)
        .bind(actor_user_id)
        .bind(details.unwrap_or_else(|| serde_json::json!({})))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get audit log for a delegation.
    pub async fn get_audit_log(
        &self,
        delegation_id: Uuid,
    ) -> Result<Vec<DelegationAuditLog>, SqlxError> {
        let logs = sqlx::query_as::<_, DelegationAuditLog>(
            r#"
            SELECT * FROM delegation_audit_log
            WHERE delegation_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(delegation_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }
}

/// Generate a random invitation token.
///
/// Uses OS CSPRNG directly — the token is a shared secret between inviter and
/// invitee, so entropy must not depend on thread-local state initialization.
fn generate_token() -> String {
    use rand::TryRng;
    let mut bytes = [0u8; 32];
    rand::rngs::SysRng
        .try_fill_bytes(&mut bytes)
        .expect("OS rng failed");
    hex::encode(bytes)
}
