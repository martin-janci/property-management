//! Repository for ACC platform/security (EPIC-ACC-16). OWNED BY: BE-Platform.
//!
//! RLS-aware, generic-executor pattern (see
//! [`crate::repositories::accounting::AccountingRepository`]). Backs `acc_tag`,
//! `acc_share_link`, `acc_audit_log` (append-only), `acc_two_factor`
//! (migration 00196).
//!
//! All column names verified against `00196_acc_invoicing_platform.sql`.
//! RLS is set on the connection by the caller; SELECTs rely on the per-tenant
//! policy (no explicit `tenant_id = $` clause needed), but INSERTs MUST set
//! `tenant_id` to the request tenant. The `acc_audit_log` table is append-only
//! at the RLS layer (no UPDATE/DELETE policy) — only `record`/`list` are exposed
//! (UC-ACC-16.7 tamper-evidence).

use crate::models::acc_platform::{AccAuditLog, AccShareLink, AccTag, AccTwoFactor};
use crate::DbPool;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

/// Repository for ACC platform/security entities (EPIC-ACC-16).
#[derive(Debug, Clone)]
pub struct AccPlatformRepository {
    pub pool: DbPool,
}

impl AccPlatformRepository {
    /// Create a new platform repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // --- Tags (UC-ACC-05.14) ---

    /// List tags for an entity (e.g. an invoice). RLS scopes to the active tenant.
    pub async fn list_tags_rls<'e, E>(
        &self,
        executor: E,
        entity_type: &str,
        entity_id: Uuid,
    ) -> Result<Vec<AccTag>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // UC-ACC-05.14: list tags attached to one entity, newest first.
        sqlx::query_as::<_, AccTag>(
            "SELECT * FROM acc_tag \
             WHERE entity_type = $1 AND entity_id = $2 \
             ORDER BY tag ASC",
        )
        .bind(entity_type)
        .bind(entity_id)
        .fetch_all(executor)
        .await
    }

    /// Add a tag (idempotent on the unique tuple
    /// `(tenant_id, entity_type, entity_id, tag)`). Returns the existing row on
    /// conflict so the handler always gets a row back. (UC-ACC-05.14)
    pub async fn add_tag_rls<'e, E>(&self, executor: E, tag: AccTag) -> Result<AccTag, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        // ON CONFLICT … DO UPDATE (no-op SET) so RETURNING yields the row whether
        // it was just inserted or already existed — plain DO NOTHING would return
        // no row on conflict and break the handler's RETURNING contract.
        sqlx::query_as::<_, AccTag>(
            "INSERT INTO acc_tag (tenant_id, entity_type, entity_id, tag) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (tenant_id, entity_type, entity_id, tag) \
             DO UPDATE SET tag = EXCLUDED.tag \
             RETURNING *",
        )
        .bind(tag.tenant_id)
        .bind(&tag.entity_type)
        .bind(tag.entity_id)
        .bind(&tag.tag)
        .fetch_one(executor)
        .await
    }

    // --- Share links (UC-ACC-05.11) ---

    /// Create a capability-token share link for an invoice. (UC-ACC-05.11)
    pub async fn create_share_link_rls<'e, E>(
        &self,
        executor: E,
        link: AccShareLink,
    ) -> Result<AccShareLink, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccShareLink>(
            "INSERT INTO acc_share_link \
                 (tenant_id, invoice_id, token, expires_at, created_by) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING *",
        )
        .bind(link.tenant_id)
        .bind(link.invoice_id)
        .bind(&link.token)
        .bind(link.expires_at)
        .bind(link.created_by)
        .fetch_one(executor)
        .await
    }

    /// Resolve a share link by its opaque token (public, capability-based read).
    /// Returns None if missing; the caller enforces revoked/expired so the public
    /// endpoint can return a non-enumerating 404 for all three. (UC-ACC-05.11)
    ///
    /// NOTE: this is invoked from the PUBLIC share endpoint which has no tenant
    /// RLS context, so the lookup is by the globally-unique token only. The token
    /// is high-entropy and unguessable, which is the capability that authorizes
    /// the read. The handler must NOT expose tenant-identifying data.
    pub async fn find_share_link_by_token_rls<'e, E>(
        &self,
        executor: E,
        token: &str,
    ) -> Result<Option<AccShareLink>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccShareLink>("SELECT * FROM acc_share_link WHERE token = $1")
            .bind(token)
            .fetch_optional(executor)
            .await
    }

    /// Revoke a share link (UC-ACC-05.11). Idempotent: re-revoking keeps the
    /// first revocation timestamp via `COALESCE`.
    pub async fn revoke_share_link_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query(
            "UPDATE acc_share_link \
             SET revoked_at = COALESCE(revoked_at, NOW()) \
             WHERE id = $1",
        )
        .bind(id)
        .execute(executor)
        .await?;
        Ok(())
    }

    // --- Audit log (append-only, UC-ACC-16.7 / 01.10) ---

    /// Append an audit entry. INSERT-only (table has no UPDATE/DELETE policy).
    /// `id` and `created_at` are DB-defaulted; the supplied placeholders are
    /// ignored. Persisted in the SAME transaction as the audited mutation so an
    /// abort rolls back both together. (UC-ACC-16.7)
    pub async fn record_audit_rls<'e, E>(
        &self,
        executor: E,
        entry: AccAuditLog,
    ) -> Result<AccAuditLog, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccAuditLog>(
            "INSERT INTO acc_audit_log \
                 (tenant_id, actor_id, action, entity_type, entity_id, diff) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING *",
        )
        .bind(entry.tenant_id)
        .bind(entry.actor_id)
        .bind(&entry.action)
        .bind(&entry.entity_type)
        .bind(entry.entity_id)
        .bind(&entry.diff)
        .fetch_one(executor)
        .await
    }

    /// List audit entries for the current tenant, optionally filtered by entity
    /// type and/or id. Newest-first, capped by `limit`. (UC-ACC-16.7 / 01.10:
    /// filterable, tamper-evident view.) The `$2/$3 IS NULL OR …` idiom keeps a
    /// single prepared statement for all four filter combinations.
    pub async fn list_audit_rls<'e, E>(
        &self,
        executor: E,
        entity_type: Option<&str>,
        entity_id: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<AccAuditLog>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccAuditLog>(
            "SELECT * FROM acc_audit_log \
             WHERE ($1::text IS NULL OR entity_type = $1) \
               AND ($2::uuid IS NULL OR entity_id = $2) \
             ORDER BY created_at DESC \
             LIMIT $3",
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(limit)
        .fetch_all(executor)
        .await
    }

    // --- Two-factor (UC-ACC-16.1 / 01.9) ---

    /// Fetch the 2FA enrollment for a user in the current tenant. (UC-ACC-16.1)
    pub async fn find_two_factor_rls<'e, E>(
        &self,
        executor: E,
        user_id: Uuid,
    ) -> Result<Option<AccTwoFactor>, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccTwoFactor>("SELECT * FROM acc_two_factor WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(executor)
            .await
    }

    /// Upsert a 2FA enrollment (enroll / confirm / regenerate recovery codes).
    /// Keyed on the `(tenant_id, user_id)` unique constraint. The `updated_at`
    /// trigger maintains the timestamp. (UC-ACC-16.1)
    pub async fn upsert_two_factor_rls<'e, E>(
        &self,
        executor: E,
        two_factor: AccTwoFactor,
    ) -> Result<AccTwoFactor, sqlx::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, AccTwoFactor>(
            "INSERT INTO acc_two_factor \
                 (tenant_id, user_id, secret, enabled, confirmed_at, recovery_codes) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (tenant_id, user_id) DO UPDATE SET \
                 secret = EXCLUDED.secret, \
                 enabled = EXCLUDED.enabled, \
                 confirmed_at = EXCLUDED.confirmed_at, \
                 recovery_codes = EXCLUDED.recovery_codes \
             RETURNING *",
        )
        .bind(two_factor.tenant_id)
        .bind(two_factor.user_id)
        .bind(&two_factor.secret)
        .bind(two_factor.enabled)
        .bind(two_factor.confirmed_at)
        .bind(&two_factor.recovery_codes)
        .fetch_one(executor)
        .await
    }
}
