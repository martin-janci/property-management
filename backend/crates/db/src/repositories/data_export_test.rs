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

    /// Issue #2929 (follow-up to #2924): the *snapshot-consistency* guarantee —
    /// the one `report_summary_counts_agree_with_entries` above documents but
    /// cannot actually exercise, because with no concurrent writer the pre-fix
    /// two-`&pool`-reads code and the fixed single-transaction code return
    /// identical data. This test drives a concurrent commit *between* the two
    /// internal reads through the `count_summary` / `list_entries` seam and
    /// contrasts the two wirings:
    ///
    /// * **Shared REPEATABLE READ snapshot (the fix):** both reads run on the
    ///   same read-only `REPEATABLE READ` transaction. PostgreSQL freezes the
    ///   snapshot at the first statement, so a row committed by another
    ///   connection *after* the counts read is invisible to *both* reads —
    ///   `total_requests` still equals `entries.len()`.
    /// * **Two pooled connections (the pre-#2924 bug):** the same two statements
    ///   run on separate connections with a commit in between, so the entries
    ///   read sees a row the counts read did not — the counts and entries desync.
    ///
    /// The second half asserts the desync, so this test fails if the snapshot
    /// isolation is ever removed (the exact regression #2929 flags): dropping the
    /// shared transaction makes the two halves observe different row sets.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn report_summary_reads_share_one_snapshot_under_contention(pool: sqlx::PgPool) {
        let user = make_user(&pool, "snapshot@example.com").await;

        // Baseline: three committed rows before any read.
        insert_request(&pool, user, "pending", false).await;
        insert_request(&pool, user, "ready", false).await;
        insert_request(&pool, user, "downloaded", true).await;

        // --- Shared REPEATABLE READ snapshot: the production wiring. ---
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await
            .unwrap();

        // First read pins the transaction snapshot at the pre-insert state.
        let (total, _pending, _completed, _downloaded) =
            DataExportRepository::count_summary(&mut tx, None, None)
                .await
                .unwrap();
        assert_eq!(total, 3, "counts read observes the three baseline rows");

        // A concurrent writer commits a fourth matching row on another connection
        // AFTER the snapshot was pinned but BEFORE the entries read.
        insert_request(&pool, user, "pending", false).await;

        // Second read on the SAME transaction: the frozen snapshot hides the new
        // row, so it stays consistent with the counts read.
        let entries = DataExportRepository::list_entries(&mut tx, None, None, 1000)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        assert_eq!(
            total as usize,
            entries.len(),
            "shared snapshot: the concurrent insert is invisible to both reads, \
             so counts still agree with entries"
        );
        assert_eq!(
            entries.len(),
            3,
            "the row committed after the snapshot was pinned does not leak into entries"
        );

        // --- Control: two pooled connections (pre-#2924 behaviour). ---
        // Re-run the same two statements with no shared transaction and a commit
        // in between; the counts and entries must diverge — this is precisely the
        // desync the fix eliminates, and asserting it proves the test above would
        // FAIL if `report_summary` reverted to reading `&self.pool` twice.
        let mut c1 = pool.acquire().await.unwrap();
        let (total_unshared, _, _, _) = DataExportRepository::count_summary(&mut c1, None, None)
            .await
            .unwrap();
        drop(c1);

        insert_request(&pool, user, "pending", false).await; // commits between reads

        let mut c2 = pool.acquire().await.unwrap();
        let entries_unshared = DataExportRepository::list_entries(&mut c2, None, None, 1000)
            .await
            .unwrap();
        drop(c2);

        assert_ne!(
            total_unshared as usize,
            entries_unshared.len(),
            "without the shared snapshot the concurrent commit leaks into the \
             entries read but not the counts read — the two desync, which is what \
             report_summary's REPEATABLE READ transaction must prevent"
        );
        assert_eq!(
            entries_unshared.len(),
            total_unshared as usize + 1,
            "the entries read sees exactly the one row committed after the counts read"
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
