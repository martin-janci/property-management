//! OAuth token-usage analytics repository (Epic 10A — data audit).
//!
//! Records per-client OAuth token lifecycle events (issuance / refresh /
//! revocation) and rolls them up for the platform-admin ecosystem-health
//! dashboard. Recording is intended to be best-effort: callers on the token
//! issuance/refresh/revocation paths should log-and-ignore any error returned
//! by [`OAuthTokenEventRepository::record`] so analytics never breaks the
//! OAuth flow itself.

use crate::models::oauth_token_event::{
    CreateOAuthTokenEvent, OAuthClientTokenUsage, OAuthTokenEvent, OAuthTokenEventTotals,
};
use crate::DbPool;
use chrono::{DateTime, Utc};
use sqlx::Error as SqlxError;

/// Repository for OAuth token-usage analytics.
#[derive(Clone)]
pub struct OAuthTokenEventRepository {
    pool: DbPool,
}

impl OAuthTokenEventRepository {
    /// Create a new `OAuthTokenEventRepository`.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Record a single OAuth token lifecycle event.
    ///
    /// Best-effort: callers on the hot OAuth path should not fail token
    /// issuance if this returns an error — log and continue.
    pub async fn record(&self, data: CreateOAuthTokenEvent) -> Result<OAuthTokenEvent, SqlxError> {
        let event = sqlx::query_as::<_, OAuthTokenEvent>(
            r#"
            INSERT INTO oauth_token_events
                (event_type, client_id, user_id, token_type, grant_type, scopes)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(data.event_type.as_str())
        .bind(&data.client_id)
        .bind(data.user_id)
        .bind(data.token_type.as_str())
        .bind(&data.grant_type)
        .bind(sqlx::types::Json(&data.scopes))
        .fetch_one(&self.pool)
        .await?;

        Ok(event)
    }

    /// Per-client token-usage rollup for events at or after `since`, ordered by
    /// issuance volume. Powers the ecosystem-health dashboard's per-client view.
    pub async fn usage_per_client(
        &self,
        since: DateTime<Utc>,
    ) -> Result<Vec<OAuthClientTokenUsage>, SqlxError> {
        let rows = sqlx::query_as::<_, OAuthClientTokenUsage>(
            r#"
            SELECT
                e.client_id AS client_id,
                c.name      AS client_name,
                COUNT(*) FILTER (WHERE e.event_type = 'issued')    AS issued_count,
                COUNT(*) FILTER (WHERE e.event_type = 'refreshed') AS refreshed_count,
                COUNT(*) FILTER (WHERE e.event_type = 'revoked')   AS revoked_count,
                MAX(e.created_at)                                  AS last_event_at
            FROM oauth_token_events e
            LEFT JOIN oauth_clients c ON c.client_id = e.client_id
            WHERE e.created_at >= $1
            GROUP BY e.client_id, c.name
            ORDER BY issued_count DESC, e.client_id ASC
            "#,
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Whole-ecosystem token-usage totals for events at or after `since`.
    pub async fn totals(&self, since: DateTime<Utc>) -> Result<OAuthTokenEventTotals, SqlxError> {
        let totals = sqlx::query_as::<_, OAuthTokenEventTotals>(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE event_type = 'issued')    AS issued_count,
                COUNT(*) FILTER (WHERE event_type = 'refreshed') AS refreshed_count,
                COUNT(*) FILTER (WHERE event_type = 'revoked')   AS revoked_count,
                COUNT(DISTINCT client_id)                        AS active_clients
            FROM oauth_token_events
            WHERE created_at >= $1
            "#,
        )
        .bind(since)
        .fetch_one(&self.pool)
        .await?;

        Ok(totals)
    }

    /// Delete events older than `cutoff`. Retention housekeeping for the
    /// analytics table; returns the number of rows removed.
    pub async fn cleanup_before(&self, cutoff: DateTime<Utc>) -> Result<u64, SqlxError> {
        let result = sqlx::query(
            r#"
            DELETE FROM oauth_token_events WHERE created_at < $1
            "#,
        )
        .bind(cutoff)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::oauth_token_event::{CreateOAuthTokenEvent, OAuthTokenKind};
    use chrono::Duration;

    /// Insert a minimal OAuth client and return its `client_id`.
    async fn make_client(pool: &sqlx::PgPool, client_id: &str) {
        sqlx::query(
            "INSERT INTO oauth_clients (client_id, client_secret_hash, name) \
             VALUES ($1, 'secret-hash', 'Test Client')",
        )
        .bind(client_id)
        .execute(pool)
        .await
        .unwrap();
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn record_and_rollup_per_client(pool: sqlx::PgPool) {
        let repo = OAuthTokenEventRepository::new(pool.clone());
        make_client(&pool, "client-a").await;
        make_client(&pool, "client-b").await;

        // client-a: 2 issued, 1 refreshed, 1 revoked
        repo.record(CreateOAuthTokenEvent::issued(
            "client-a",
            None,
            vec!["profile".to_string()],
        ))
        .await
        .unwrap();
        repo.record(CreateOAuthTokenEvent::issued("client-a", None, vec![]))
            .await
            .unwrap();
        repo.record(CreateOAuthTokenEvent::refreshed("client-a", None, vec![]))
            .await
            .unwrap();
        repo.record(CreateOAuthTokenEvent::revoked(
            "client-a",
            None,
            OAuthTokenKind::Refresh,
        ))
        .await
        .unwrap();

        // client-b: 1 issued
        repo.record(CreateOAuthTokenEvent::issued("client-b", None, vec![]))
            .await
            .unwrap();

        let since = Utc::now() - Duration::hours(1);
        let usage = repo.usage_per_client(since).await.unwrap();
        assert_eq!(usage.len(), 2);

        // Ordered by issued_count DESC → client-a first.
        let a = &usage[0];
        assert_eq!(a.client_id, "client-a");
        assert_eq!(a.client_name.as_deref(), Some("Test Client"));
        assert_eq!(a.issued_count, 2);
        assert_eq!(a.refreshed_count, 1);
        assert_eq!(a.revoked_count, 1);
        assert!(a.last_event_at.is_some());

        let b = &usage[1];
        assert_eq!(b.client_id, "client-b");
        assert_eq!(b.issued_count, 1);
        assert_eq!(b.refreshed_count, 0);
        assert_eq!(b.revoked_count, 0);

        let totals = repo.totals(since).await.unwrap();
        assert_eq!(totals.issued_count, 3);
        assert_eq!(totals.refreshed_count, 1);
        assert_eq!(totals.revoked_count, 1);
        assert_eq!(totals.active_clients, 2);
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn window_and_cleanup_filter_by_time(pool: sqlx::PgPool) {
        let repo = OAuthTokenEventRepository::new(pool.clone());
        make_client(&pool, "client-a").await;

        // One recent event and one back-dated event.
        repo.record(CreateOAuthTokenEvent::issued("client-a", None, vec![]))
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO oauth_token_events (event_type, client_id, token_type, created_at) \
             VALUES ('issued', 'client-a', 'access', NOW() - INTERVAL '10 days')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Window of last hour sees only the recent event.
        let recent = repo.totals(Utc::now() - Duration::hours(1)).await.unwrap();
        assert_eq!(recent.issued_count, 1);

        // Full window sees both.
        let all = repo.totals(Utc::now() - Duration::days(30)).await.unwrap();
        assert_eq!(all.issued_count, 2);

        // Cleanup removes only the back-dated row.
        let removed = repo
            .cleanup_before(Utc::now() - Duration::days(1))
            .await
            .unwrap();
        assert_eq!(removed, 1);
        let after = repo.totals(Utc::now() - Duration::days(30)).await.unwrap();
        assert_eq!(after.issued_count, 1);
    }
}
