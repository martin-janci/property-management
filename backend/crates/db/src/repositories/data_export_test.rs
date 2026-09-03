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

    /// Issue #2929 (follow-up to #2924): drive a concurrent commit *into the
    /// gap between `report_summary`'s two internal reads* and prove that the
    /// public `report_summary` — not a hand-reconstructed copy of its logic —
    /// keeps its counts and entries on one snapshot. This test must FAIL on the
    /// pre-#2924 wiring (two separate `&self.pool` reads) and PASS on the fixed
    /// single-`REPEATABLE READ`-transaction wiring.
    ///
    /// The lever is a **single-connection pool** to the same test database.
    /// With exactly one connection, how `report_summary` uses that connection
    /// is observable:
    ///
    /// * **Fixed (`self.pool.begin()` + `&mut *tx`):** the one connection is
    ///   held for the *whole* `REPEATABLE READ` transaction — across both the
    ///   counts read and the entries read — and only returned to the pool at
    ///   `commit`. A writer sharing this pool therefore cannot run until
    ///   `report_summary` has finished, so its row lands *after* both reads and
    ///   both observe the same 3 baseline rows → `total_requests == entries.len()`.
    /// * **Pre-#2924 (`fetch_one(&self.pool)` then `fetch_all(&self.pool)`):**
    ///   the connection is returned to the pool *between* the two reads. sqlx
    ///   hands that freed connection to the FIFO-queued writer, which commits a
    ///   fourth matching row in the gap; the entries read then re-acquires and
    ///   sees 4 rows while the counts read saw 3 → `total_requests (3) !=
    ///   entries.len() (4)` and the assertion below FAILS.
    ///
    /// We queue the report task and the writer task on the single connection in
    /// FIFO order (report first, writer second) by holding the connection while
    /// both enqueue, then releasing it — so the interleaving is deterministic
    /// rather than timing-dependent.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn report_summary_reads_share_one_snapshot_under_contention(pool: sqlx::PgPool) {
        use sqlx::postgres::PgPoolOptions;
        use std::sync::Arc;
        use std::time::Duration;
        use tokio::sync::Notify;

        let user = make_user(&pool, "snapshot@example.com").await;

        // Baseline: three committed rows before any read.
        insert_request(&pool, user, "pending", false).await;
        insert_request(&pool, user, "ready", false).await;
        insert_request(&pool, user, "downloaded", true).await;

        // A second pool to the SAME ephemeral test database, capped at one
        // connection so report_summary's connection lifetime is observable.
        let capped = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect_with((*pool.connect_options()).clone())
            .await
            .expect("single-connection pool to the test database");

        let repo = DataExportRepository::new(capped.clone());

        // Hold the only connection so BOTH the report task and the writer task
        // must queue for it. sqlx grants a freed connection to waiters in FIFO
        // order, so enqueuing the report first and the writer second fixes the
        // interleaving deterministically.
        let hold = capped.acquire().await.expect("hold the single connection");

        let report_ready = Arc::new(Notify::new());
        let writer_gate = Arc::new(Notify::new());

        // Report task — becomes pool waiter #1.
        let report_task = {
            let repo = repo.clone();
            let report_ready = report_ready.clone();
            tokio::spawn(async move {
                report_ready.notify_one();
                repo.report_summary(None, None, 1000)
                    .await
                    .expect("report_summary")
            })
        };
        // Let the report task reach (and block on) its first pool acquire before
        // the writer enqueues, so the report is waiter #1.
        report_ready.notified().await;
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }

        // Writer task — becomes pool waiter #2. Uses the capped pool, so it
        // acquires the single connection only when it becomes free, which (on
        // the pre-fix wiring) is the gap between report_summary's two reads.
        let writer_task = {
            let capped = capped.clone();
            let writer_gate = writer_gate.clone();
            tokio::spawn(async move {
                writer_gate.notified().await;
                insert_request(&capped, user, "pending", false).await;
            })
        };
        writer_gate.notify_one();
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }

        // Release the connection: FIFO hands it to the report task first.
        drop(hold);

        let summary = report_task.await.expect("join report task");
        writer_task.await.expect("join writer task");

        // The invariant: with `limit` covering every row, `total_requests` and
        // `entries` are read from ONE snapshot, so they must agree. On the fixed
        // code the concurrent insert is invisible to both reads (3 == 3). If the
        // shared REPEATABLE READ transaction is ever removed, the insert leaks
        // into the entries read but not the counts read (3 != 4) and this fails —
        // the exact regression #2929 guards.
        assert_eq!(
            summary.total_requests as usize,
            summary.entries.len(),
            "report_summary counts and entries must come from one snapshot; a \
             desync here means the shared REPEATABLE READ transaction was \
             removed (regression #2929)"
        );
        assert_eq!(
            summary.entries.len(),
            3,
            "the row committed after report_summary pinned its snapshot must not \
             appear in the entries slice"
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
