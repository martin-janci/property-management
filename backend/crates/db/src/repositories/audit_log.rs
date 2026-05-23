//! Audit log repository (Epic 9, Story 9.6).

use crate::models::audit_log::{ActionCount, AuditLog, AuditLogQuery, CreateAuditLog};
use crate::DbPool;
use sha2::{Digest, Sha256};
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for audit log operations.
#[derive(Clone)]
pub struct AuditLogRepository {
    pool: DbPool,
}

impl AuditLogRepository {
    /// Create a new repository instance.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new audit log entry.
    ///
    /// P1-04 (dev-team review): integrity_hash is precomputed client-side
    /// and supplied to a single INSERT inside a serializable transaction
    /// — the old two-statement INSERT+UPDATE pattern meant any actor
    /// with `UPDATE audit_logs` on the application role could rewrite the
    /// hash after insertion, undermining the immutability claim. The
    /// companion migration revokes UPDATE on `integrity_hash` from the
    /// app role; the application path only writes the hash exactly once,
    /// at INSERT time.
    pub async fn create(&self, data: CreateAuditLog) -> Result<AuditLog, SqlxError> {
        // Pre-generate the id + timestamp so we can hash before INSERT.
        let id = Uuid::new_v4();
        let created_at = chrono::Utc::now();

        // SERIALIZABLE so the (read latest hash → write new row) sequence
        // is consistent: two concurrent writers can't both read the same
        // previous_hash and break the chain.
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut *tx)
            .await?;

        let previous_hash: Option<String> = sqlx::query_scalar(
            r#"
            SELECT integrity_hash FROM audit_logs
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&mut *tx)
        .await?
        .flatten();

        // Build the to-be-stored row in memory so we can hash its
        // canonical representation before persistence. The hash domain
        // matches `compute_integrity_hash` so `verify_integrity`
        // recomputes the same digest.
        let mut hasher = Sha256::new();
        hasher.update(id.to_string().as_bytes());
        if let Some(user_id) = data.user_id {
            hasher.update(user_id.to_string().as_bytes());
        }
        hasher.update(format!("{:?}", data.action).as_bytes());
        if let Some(ref resource_type) = data.resource_type {
            hasher.update(resource_type.as_bytes());
        }
        if let Some(resource_id) = data.resource_id {
            hasher.update(resource_id.to_string().as_bytes());
        }
        hasher.update(created_at.to_rfc3339().as_bytes());
        if let Some(ref previous_hash) = previous_hash {
            hasher.update(previous_hash.as_bytes());
        }
        let integrity_hash = hex::encode(hasher.finalize());

        let log = sqlx::query_as::<_, AuditLog>(
            r#"
            INSERT INTO audit_logs (
                id, user_id, action, resource_type, resource_id, org_id,
                details, old_values, new_values, ip_address, user_agent,
                previous_hash, integrity_hash, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.user_id)
        .bind(&data.action)
        .bind(&data.resource_type)
        .bind(data.resource_id)
        .bind(data.org_id)
        .bind(&data.details)
        .bind(&data.old_values)
        .bind(&data.new_values)
        .bind(&data.ip_address)
        .bind(&data.user_agent)
        .bind(&previous_hash)
        .bind(&integrity_hash)
        .bind(created_at)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(log)
    }

    /// Compute integrity hash for an audit log entry.
    fn compute_integrity_hash(&self, log: &AuditLog) -> String {
        let mut hasher = Sha256::new();

        // Include key fields in the hash
        hasher.update(log.id.to_string().as_bytes());
        if let Some(user_id) = log.user_id {
            hasher.update(user_id.to_string().as_bytes());
        }
        hasher.update(format!("{:?}", log.action).as_bytes());
        if let Some(ref resource_type) = log.resource_type {
            hasher.update(resource_type.as_bytes());
        }
        if let Some(resource_id) = log.resource_id {
            hasher.update(resource_id.to_string().as_bytes());
        }
        hasher.update(log.created_at.to_rfc3339().as_bytes());
        if let Some(ref previous_hash) = log.previous_hash {
            hasher.update(previous_hash.as_bytes());
        }

        hex::encode(hasher.finalize())
    }

    /// Query audit logs with optional filters.
    pub async fn query(&self, query: AuditLogQuery) -> Result<Vec<AuditLog>, SqlxError> {
        let limit = query.limit.unwrap_or(100).min(1000);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, AuditLog>(
            r#"
            SELECT * FROM audit_logs
            WHERE ($1::uuid IS NULL OR user_id = $1)
              AND ($2::audit_action IS NULL OR action = $2)
              AND ($3::text IS NULL OR resource_type = $3)
              AND ($4::uuid IS NULL OR resource_id = $4)
              AND ($5::uuid IS NULL OR org_id = $5)
              AND ($6::timestamptz IS NULL OR created_at >= $6)
              AND ($7::timestamptz IS NULL OR created_at <= $7)
            ORDER BY created_at DESC
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(query.user_id)
        .bind(query.action)
        .bind(query.resource_type)
        .bind(query.resource_id)
        .bind(query.org_id)
        .bind(query.from_date)
        .bind(query.to_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Get audit logs for a specific user (for GDPR transparency).
    pub async fn get_user_logs(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLog>, SqlxError> {
        sqlx::query_as::<_, AuditLog>(
            r#"
            SELECT * FROM audit_logs
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit.min(1000))
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Get audit log count by action type for a date range.
    pub async fn get_action_counts(
        &self,
        from_date: Option<chrono::DateTime<chrono::Utc>>,
        to_date: Option<chrono::DateTime<chrono::Utc>>,
        org_id: Option<Uuid>,
    ) -> Result<Vec<ActionCount>, SqlxError> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT action::text, COUNT(*) as count
            FROM audit_logs
            WHERE ($1::timestamptz IS NULL OR created_at >= $1)
              AND ($2::timestamptz IS NULL OR created_at <= $2)
              AND ($3::uuid IS NULL OR org_id = $3)
            GROUP BY action
            ORDER BY count DESC
            "#,
        )
        .bind(from_date)
        .bind(to_date)
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(action, count)| ActionCount { action, count })
            .collect())
    }

    /// Verify the integrity of audit log chain.
    /// Returns true if all entries are valid, false if tampering is detected.
    pub async fn verify_integrity(&self, limit: Option<i64>) -> Result<bool, SqlxError> {
        let logs = sqlx::query_as::<_, AuditLog>(
            r#"
            SELECT * FROM audit_logs
            ORDER BY created_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit.unwrap_or(10000))
        .fetch_all(&self.pool)
        .await?;

        let mut previous_hash: Option<String> = None;

        for log in logs {
            // Check that previous_hash matches
            if log.previous_hash != previous_hash {
                tracing::warn!(
                    log_id = %log.id,
                    expected = ?previous_hash,
                    actual = ?log.previous_hash,
                    "Audit log chain integrity violation: previous hash mismatch"
                );
                return Ok(false);
            }

            // Verify integrity hash
            let computed_hash = self.compute_integrity_hash(&log);
            if let Some(ref stored_hash) = log.integrity_hash {
                if &computed_hash != stored_hash {
                    tracing::warn!(
                        log_id = %log.id,
                        expected = %stored_hash,
                        computed = %computed_hash,
                        "Audit log integrity violation: hash mismatch"
                    );
                    return Ok(false);
                }
            }

            previous_hash = log.integrity_hash;
        }

        Ok(true)
    }

    /// Get total count of audit logs matching query.
    pub async fn count(&self, query: AuditLogQuery) -> Result<i64, SqlxError> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM audit_logs
            WHERE ($1::uuid IS NULL OR user_id = $1)
              AND ($2::audit_action IS NULL OR action = $2)
              AND ($3::text IS NULL OR resource_type = $3)
              AND ($4::uuid IS NULL OR resource_id = $4)
              AND ($5::uuid IS NULL OR org_id = $5)
              AND ($6::timestamptz IS NULL OR created_at >= $6)
              AND ($7::timestamptz IS NULL OR created_at <= $7)
            "#,
        )
        .bind(query.user_id)
        .bind(query.action)
        .bind(query.resource_type)
        .bind(query.resource_id)
        .bind(query.org_id)
        .bind(query.from_date)
        .bind(query.to_date)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }
}
