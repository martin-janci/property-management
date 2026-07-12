//! Repo/DB-level FORCE-RLS regression test for the analytics **write** path on
//! **portal-owned** listings (#2244) — the write-path counterpart of the read
//! test in `portal_listing_analytics_force_rls_tests.rs` (#2199 / #2218).
//!
//! Background
//! ----------
//! `track_listing_view` (migration 00063) is SECURITY INVOKER. It upserts into
//! `listing_analytics`, a `FORCE ROW LEVEL SECURITY` table (migration 00113)
//! with an org-only policy and no portal-owner branch. Portal listings carry
//! `organization_id = NULL` (migration 00186), so for a portal listing run on
//! the RLS-subject reality-server role the `WITH CHECK` fails and the view is
//! never recorded — the table stays empty and `portal_get_listing_analytics`
//! reads back zeros end-to-end (#2244).
//!
//! Migration 00214 adds `portal_track_listing_view`, a SECURITY DEFINER upsert
//! that bypasses the org-only policy so portal views are actually recorded.
//!
//! This test binds a real `NOSUPERUSER NOBYPASSRLS` role — the way the
//! production reality-server role experiences FORCE RLS — and proves:
//!
//! 1. **Bug proof** — the old SECURITY INVOKER `track_listing_view` raises an
//!    RLS `WITH CHECK` violation for the portal listing under the bound role.
//! 2. **Fix** — `portal_track_listing_view` succeeds under the same role and
//!    each call increments `views`.
//! 3. **End-to-end regression lock** — after the definer writes, the owner's
//!    `portal_get_listing_analytics` reads the recorded views back non-zero
//!    (closing the #2199 user-visible symptom for good).
//!
//! Assertion (2)/(3) fail on `main` (no definer write path existed there);
//! together they are the regression lock for the write fix.

use sqlx::PgPool;
use uuid::Uuid;

async fn seed_portal_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind) \
         VALUES ($1, 'test_hash', $2, 'active', NOW(), 'public') RETURNING id",
    )
    .bind(format!("{tag}@portal-track-rls.test"))
    .bind(format!("PortalTrackRls {tag}"))
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
async fn portal_view_recorded_via_security_definer_under_force_rls(pool: PgPool) {
    let owner = seed_portal_user(&pool, "owner").await;
    let listing_id = seed_portal_listing(&pool, owner, "track-force-rls").await;

    let role = format!("portal_tr_rls_{}", &Uuid::new_v4().to_string()[..8]);

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE RLS actually binds. ---
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create test role");

    // Grant everything the writes would legitimately need, so the ONLY barrier
    // left standing is the FORCE-RLS policy itself.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT, INSERT, UPDATE ON listing_analytics TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant analytics privileges");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON listings TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant listings select");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION \
         track_listing_view(UUID, VARCHAR), \
         portal_track_listing_view(UUID, VARCHAR), \
         portal_get_listing_analytics(UUID, UUID, DATE, DATE), \
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

    // --- 1. Bug proof: the SECURITY INVOKER path is blocked by FORCE RLS. ---
    let invoker_result = sqlx::query("SELECT track_listing_view($1, 'website')")
        .bind(listing_id)
        .execute(&mut *conn)
        .await;
    assert!(
        invoker_result.is_err(),
        "the SECURITY INVOKER track_listing_view must fail the org-only \
         listing_analytics WITH CHECK for a portal listing under the \
         RLS-subject role (this is the #2244 bug)"
    );

    // --- 2. Fix: the SECURITY DEFINER path records the view. ---
    for _ in 0..3 {
        sqlx::query("SELECT portal_track_listing_view($1, 'website')")
            .bind(listing_id)
            .execute(&mut *conn)
            .await
            .expect("portal_track_listing_view must succeed under FORCE RLS");
    }

    // --- 3. End-to-end: the owner reads the recorded views back non-zero. ---
    let views: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(views), 0)::bigint \
         FROM portal_get_listing_analytics($1, $2, NULL, NULL)",
    )
    .bind(listing_id)
    .bind(owner)
    .fetch_one(&mut *conn)
    .await
    .expect("owner definer views");
    assert_eq!(
        views, 3,
        "each portal_track_listing_view call must increment views, and the \
         owner must read them back through portal_get_listing_analytics under \
         FORCE RLS (#2244 end-to-end regression lock)"
    );

    sqlx::query("RESET ROLE")
        .execute(&mut *conn)
        .await
        .expect("reset role");
    drop(conn);

    // --- Cleanup ---
    for stmt in [
        format!("REVOKE ALL ON listing_analytics, listings FROM \"{role}\""),
        format!(
            "REVOKE EXECUTE ON FUNCTION \
             track_listing_view(UUID, VARCHAR), \
             portal_track_listing_view(UUID, VARCHAR), \
             portal_get_listing_analytics(UUID, UUID, DATE, DATE), \
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
