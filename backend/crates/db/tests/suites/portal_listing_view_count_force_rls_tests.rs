//! Repo/DB-level FORCE-RLS regression test for the public listing-detail
//! **view-count read** on **portal-owned** listings — the read-back counterpart
//! of the write test in `portal_track_listing_view_force_rls_tests.rs` (#2244).
//!
//! Background
//! ----------
//! The public listing-detail handler (`GET /api/v1/listings/{id}`) records views
//! via the SECURITY DEFINER `portal_track_listing_view` (00214), so views ARE
//! persisted. But it built its response with a hardcoded `view_count: 0` — the
//! read side was never wired, so the public page under-reported every listing's
//! views to zero.
//!
//! A naive read fix (`SELECT SUM(views) FROM listing_analytics`) does not work
//! for portal listings: `listing_analytics` is `FORCE ROW LEVEL SECURITY`
//! (migration 00113) with an org-only policy and no portal-owner branch. Portal
//! listings carry `organization_id = NULL` (migration 00186) and the public
//! reality-server pool is RLS-subject with no org context, so a direct SELECT
//! reads back zero — the same RLS trap as the write path (#2244).
//!
//! Migration 00220 adds `portal_get_listing_view_count`, a SECURITY DEFINER
//! aggregate that bypasses the org-only policy so the public read reports the
//! real recorded total.
//!
//! This test binds a real `NOSUPERUSER NOBYPASSRLS` role — the way the
//! production reality-server role experiences FORCE RLS — and proves:
//!
//! 1. **Bug proof** — a direct `SELECT SUM(views)` reads back 0 for the portal
//!    listing under the bound role even though views were recorded.
//! 2. **Fix** — `RealityPortalRepository::get_view_count` (routing through the
//!    SECURITY DEFINER `portal_get_listing_view_count`) reads the real total.
//!
//! Assertion (2) fails on `main` (no definer read path existed there); it is the
//! regression lock for the read fix.

use sqlx::PgPool;
use uuid::Uuid;

async fn seed_portal_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind) \
         VALUES ($1, 'test_hash', $2, 'active', NOW(), 'public') RETURNING id",
    )
    .bind(format!("{tag}@portal-viewcount-rls.test"))
    .bind(format!("PortalViewCountRls {tag}"))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("seed_portal_user({tag}): {e}"))
}

/// Create a portal-owned listing (`organization_id IS NULL`,
/// `portal_owner_id = user_id`) via the SECURITY DEFINER repo path.
async fn seed_portal_listing(pool: &PgPool, user_id: Uuid, title: &str) -> Uuid {
    let repo = db::repositories::RealityPortalRepository::new(pool.clone());
    repo.create_portal_listing(
        user_id,
        title,
        Some("desc"),
        "apartment",
        "sale",
        rust_decimal::Decimal::new(100_000, 0),
        "EUR",
        "Test Street 1",
        "Bratislava",
        "81101",
        "SK",
        None,
        None,
        None,
        None,
    )
    .await
    .expect("seed_portal_listing")
    .id
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn portal_view_count_read_via_security_definer_under_force_rls(pool: PgPool) {
    let owner = seed_portal_user(&pool, "owner").await;
    let listing_id = seed_portal_listing(&pool, owner, "viewcount-force-rls").await;

    // Record three views through the SECURITY DEFINER write path (00214).
    let repo = db::repositories::RealityPortalRepository::new(pool.clone());
    for _ in 0..3 {
        repo.track_view(listing_id, "website")
            .await
            .expect("track_view must record a portal view");
    }

    let role = format!("portal_vc_rls_{}", &Uuid::new_v4().to_string()[..8]);

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE RLS actually binds. ---
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create test role");

    // Grant everything the reads would legitimately need, so the ONLY barrier
    // left standing is the FORCE-RLS policy itself.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON listing_analytics, listings TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select privileges");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION \
         portal_get_listing_view_count(UUID), \
         is_super_admin(), get_current_org_id() TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant execute");

    // All role-bound work runs on ONE dedicated connection.
    let mut conn = pool.acquire().await.expect("acquire connection");

    sqlx::query(sqlx::AssertSqlSafe(format!("SET ROLE \"{role}\"")))
        .execute(&mut *conn)
        .await
        .expect("set role");

    // --- 1. Bug proof: a direct SUM is filtered to zero by FORCE RLS. ---
    let direct_total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(views), 0)::bigint FROM listing_analytics WHERE listing_id = $1",
    )
    .bind(listing_id)
    .fetch_one(&mut *conn)
    .await
    .expect("direct sum query");
    assert_eq!(
        direct_total, 0,
        "a direct SELECT SUM(views) on the RLS-subject role must read back 0 \
         for a portal listing (org-only listing_analytics policy has no \
         portal-owner branch) — this is the hardcoded-zero bug's root cause"
    );

    // --- 2. Fix: the SECURITY DEFINER path reads the real recorded total. ---
    let definer_total: i64 = sqlx::query_scalar("SELECT portal_get_listing_view_count($1)")
        .bind(listing_id)
        .fetch_one(&mut *conn)
        .await
        .expect("portal_get_listing_view_count must succeed under FORCE RLS");
    assert_eq!(
        definer_total, 3,
        "portal_get_listing_view_count must bypass the org-only RLS policy and \
         return the real recorded view total for a portal listing (read-side \
         regression lock for the hardcoded view_count: 0)"
    );

    sqlx::query("RESET ROLE")
        .execute(&mut *conn)
        .await
        .expect("reset role");
    drop(conn);

    // --- 3. Repo method reads the same total on the superuser test pool. ---
    let repo_total = repo
        .get_view_count(listing_id)
        .await
        .expect("get_view_count repo method");
    assert_eq!(
        repo_total, 3,
        "RealityPortalRepository::get_view_count must return the recorded total"
    );

    // --- Cleanup ---
    for stmt in [
        format!("REVOKE ALL ON listing_analytics, listings FROM \"{role}\""),
        format!(
            "REVOKE EXECUTE ON FUNCTION \
             portal_get_listing_view_count(UUID), \
             is_super_admin(), get_current_org_id() FROM \"{role}\""
        ),
        format!("DROP OWNED BY \"{role}\""),
        format!("DROP ROLE IF EXISTS \"{role}\""),
    ] {
        sqlx::query(sqlx::AssertSqlSafe(stmt))
            .execute(&pool)
            .await
            .ok();
    }
}
