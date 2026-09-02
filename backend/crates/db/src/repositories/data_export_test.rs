//! Regression tests for `DataExportRepository::report_summary` (Epic 9).
//!
//! The GDPR data-export compliance endpoint (`GET /gdpr/data-exports`)
//! previously shipped a stub that returned an empty export list and hard-coded
//! `0` for the completed/downloaded counts, so platform admins saw materially
//! false GDPR figures. `report_summary` now derives all counts from
//! `data_export_requests`; these tests pin the aggregate semantics so the stub
//! cannot silently return.

#[cfg(test)]
mod tests {
    use crate::models::data_export::{CreateDataExportRequest, DataExportStatus, ExportFormat};
    use crate::repositories::data_export::DataExportRepository;
    use sqlx::Row;
    use uuid::Uuid;

    async fn make_user(pool: &sqlx::PgPool, email: &str) -> Uuid {
        sqlx::query(
            "INSERT INTO users (email, password_hash, name, status) \
             VALUES ($1, 'x', 'Export User', 'active') RETURNING id",
        )
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
        .get("id")
    }

    /// The repo only creates `pending` rows, so drive terminal statuses in
    /// directly. `downloaded` sets `downloaded_at` + a positive `download_count`.
    async fn insert_request(pool: &sqlx::PgPool, user_id: Uuid, status: &str, downloaded: bool) {
        sqlx::query(
            "INSERT INTO data_export_requests (user_id, status, downloaded_at, download_count) \
             VALUES ($1, $2::data_export_status, $3, $4)",
        )
        .bind(user_id)
        .bind(status)
        .bind(if downloaded {
            Some(chrono::Utc::now())
        } else {
            None
        })
        .bind(if downloaded { 1_i32 } else { 0_i32 })
        .execute(pool)
        .await
        .unwrap();
    }

    /// Counts must reflect the true state across every status — the crux of the
    /// hard-coded-zeros regression.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn report_summary_counts_reflect_true_state(pool: sqlx::PgPool) {
        let repo = DataExportRepository::new(pool.clone());
        let user = make_user(&pool, "report@example.com").await;

        // 2 in-flight, 3 completed (ready/downloaded/expired), 1 failed.
        insert_request(&pool, user, "pending", false).await;
        insert_request(&pool, user, "processing", false).await;
        insert_request(&pool, user, "ready", false).await;
        insert_request(&pool, user, "downloaded", true).await;
        insert_request(&pool, user, "expired", true).await;
        insert_request(&pool, user, "failed", false).await;

        let s = repo.report_summary(None, None, 100).await.unwrap();

        assert_eq!(s.total_requests, 6, "all requests counted");
        assert_eq!(s.pending_count, 2, "pending + processing are in-flight");
        assert_eq!(
            s.completed_count, 3,
            "ready + downloaded + expired are completed exports"
        );
        assert_eq!(
            s.downloaded_count, 2,
            "downloaded_at set on the downloaded + expired rows"
        );
        assert_eq!(
            s.entries.len(),
            6,
            "every request appears in the detail slice"
        );
        // The old stub returned exports=[] and 0 downloaded — this could not pass.
        assert_eq!(
            s.entries
                .iter()
                .filter(|e| e.downloaded_at.is_some())
                .count(),
            2,
        );
    }

    /// Issue #2924: the aggregate counts and the detail `entries` are two
    /// statements, so a concurrent write between them must not be able to
    /// desync them. `report_summary` runs both reads inside one REPEATABLE
    /// READ snapshot; here we pin the observable invariant — when `limit`
    /// covers every matching row, each aggregate count must exactly equal the
    /// count derived from `entries`. If the two statements ever saw different
    /// snapshots, this equality would break.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn report_summary_counts_agree_with_entries(pool: sqlx::PgPool) {
        let repo = DataExportRepository::new(pool.clone());
        let user = make_user(&pool, "consistent@example.com").await;

        insert_request(&pool, user, "pending", false).await;
        insert_request(&pool, user, "processing", false).await;
        insert_request(&pool, user, "ready", false).await;
        insert_request(&pool, user, "downloaded", true).await;
        insert_request(&pool, user, "expired", true).await;
        insert_request(&pool, user, "failed", false).await;

        // limit >= number of rows, so `entries` holds the full snapshot.
        let s = repo.report_summary(None, None, 1000).await.unwrap();

        assert_eq!(
            s.total_requests as usize,
            s.entries.len(),
            "total_requests must equal the number of entries in the same snapshot"
        );
        assert_eq!(
            s.pending_count as usize,
            s.entries
                .iter()
                .filter(|e| matches!(
                    e.status,
                    DataExportStatus::Pending | DataExportStatus::Processing
                ))
                .count(),
            "pending_count must match the in-flight entries in the same snapshot"
        );
        assert_eq!(
            s.completed_count as usize,
            s.entries
                .iter()
                .filter(|e| matches!(
                    e.status,
                    DataExportStatus::Ready
                        | DataExportStatus::Downloaded
                        | DataExportStatus::Expired
                ))
                .count(),
            "completed_count must match the completed entries in the same snapshot"
        );
        assert_eq!(
            s.downloaded_count as usize,
            s.entries
                .iter()
                .filter(|e| e.downloaded_at.is_some())
                .count(),
            "downloaded_count must match the downloaded entries in the same snapshot"
        );
    }

    /// `limit` bounds only the detail slice; the aggregate counts see all rows.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn report_summary_limit_bounds_entries_not_counts(pool: sqlx::PgPool) {
        let repo = DataExportRepository::new(pool.clone());
        let user = make_user(&pool, "limit@example.com").await;
        for _ in 0..5 {
            repo.create(CreateDataExportRequest {
                user_id: user,
                format: ExportFormat::Json,
                include_categories: None,
            })
            .await
            .unwrap();
        }

        let s = repo.report_summary(None, None, 2).await.unwrap();
        assert_eq!(s.total_requests, 5, "counts ignore the entry limit");
        assert_eq!(s.pending_count, 5);
        assert_eq!(s.entries.len(), 2, "entries respect the limit");
    }
}
