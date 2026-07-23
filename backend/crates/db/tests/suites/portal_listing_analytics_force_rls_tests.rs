//! Repo/DB-level FORCE-RLS regression test for per-listing analytics reads on
//! **portal-owned** listings (#2199).
//!
//! Background
//! ----------
//! PR #2183 shipped `GET /api/v1/my/listings/{id}/analytics`. Its read path did
//! a plain `SELECT * FROM listing_analytics` on the RLS-subject reality-server
//! pool. `listing_analytics` is `FORCE ROW LEVEL SECURITY` (migration 00113)
//! with an **org-only** policy and no portal-owner branch, and portal listings
//! carry `organization_id = NULL` (migration 00186). So for every portal-owned
//! listing the read filtered out all rows and the endpoint returned a zeroed
//! summary / empty daily series — even for a listing with recorded views.
//!
//! The existing api-level test (`portal_listings_my_list_analytics_tests.rs`)
//! could not catch this: `#[sqlx::test]` connects as the Postgres SUPERUSER,
//! which bypasses RLS entirely, and it never seeded analytics rows (so a zeroed
//! body was indistinguishable from "RLS filtered every row").
//!
//! This test closes both gaps. It seeds analytics for a portal listing, then
//! `SET ROLE`s to a plain `NOSUPERUSER NOBYPASSRLS` role so `FORCE` actually
//! binds — the way the production reality-server role experiences it — and:
//!
//! 1. **Bug proof** — a plain `SELECT FROM listing_analytics` under the bound
//!    role returns 0 rows (the org-only policy filters the portal listing).
//! 2. **Fix** — `portal_get_listing_analytics(listing, owner, …)` (migration
//!    00213, SECURITY DEFINER) returns the seeded rows under the same bound role.
//! 3. **Ownership gate** — the same function called with a *non-owner* user id
//!    returns 0 rows.
//!
//! Assertion (1) fails on `main` (no SECURITY DEFINER read path existed there);
//! (2) is the regression lock for the fix.

use sqlx::PgPool;
use uuid::Uuid;

async fn seed_portal_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind) \
         VALUES ($1, 'test_hash', $2, 'active', NOW(), 'public') RETURNING id",
    )
    .bind(format!("{tag}@portal-analytics-rls.test"))
    .bind(format!("PortalAnalyticsRls {tag}"))
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

/// Seed an analytics row directly as the superuser (RLS-exempt) so the test
/// role's read is exercised against real recorded data.
async fn seed_analytics_raw(pool: &PgPool, listing_id: Uuid, views: i32) {
    sqlx::query(
        "INSERT INTO listing_analytics (listing_id, date, views, source_website) \
         VALUES ($1, CURRENT_DATE, $2, $2)",
    )
    .bind(listing_id)
    .bind(views)
    .execute(pool)
    .await
    .expect("seed analytics row (superuser)");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn portal_analytics_readable_via_security_definer_under_force_rls(pool: PgPool) {
    let owner = seed_portal_user(&pool, "owner").await;
    let stranger = seed_portal_user(&pool, "stranger").await;
    let listing_id = seed_portal_listing(&pool, owner, "analytics-force-rls").await;
    seed_analytics_raw(&pool, listing_id, 7).await;

    let role = format!("portal_an_rls_{}", &Uuid::new_v4().to_string()[..8]);

    // --- NOSUPERUSER NOBYPASSRLS role so FORCE RLS actually binds. ---
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "CREATE ROLE \"{role}\" NOSUPERUSER NOBYPASSRLS"
    )))
    .execute(&pool)
    .await
    .expect("create test role");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT SELECT ON listing_analytics, listings TO \"{role}\""
    )))
    .execute(&pool)
    .await
    .expect("grant select");

    sqlx::query(sqlx::AssertSqlSafe(format!(
        "GRANT EXECUTE ON FUNCTION \
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

    // --- 1. Bug proof: plain RLS-subject SELECT sees zero rows. ---
    let direct: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM listing_analytics WHERE listing_id = $1")
            .bind(listing_id)
            .fetch_one(&mut *conn)
            .await
            .expect("direct select count");
    assert_eq!(
        direct, 0,
        "the org-only listing_analytics policy must filter a portal listing's \
         analytics for the RLS-subject role (this is the #2199 bug)"
    );

    // --- 2. Fix: SECURITY DEFINER function returns the owner's rows. ---
    let via_owner: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM portal_get_listing_analytics($1, $2, NULL, NULL)")
            .bind(listing_id)
            .bind(owner)
            .fetch_one(&mut *conn)
            .await
            .expect("owner definer count");
    assert_eq!(
        via_owner, 1,
        "portal_get_listing_analytics must return the seeded analytics row for \
         the listing owner under FORCE RLS (#2199 fix)"
    );

    let views: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(views), 0)::bigint \
         FROM portal_get_listing_analytics($1, $2, NULL, NULL)",
    )
    .bind(listing_id)
    .bind(owner)
    .fetch_one(&mut *conn)
    .await
    .expect("owner definer views");
    assert_eq!(views, 7, "recorded views must be visible to the owner");

    // --- 3. Ownership gate: a non-owner sees no rows. ---
    let via_stranger: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM portal_get_listing_analytics($1, $2, NULL, NULL)")
            .bind(listing_id)
            .bind(stranger)
            .fetch_one(&mut *conn)
            .await
            .expect("stranger definer count");
    assert_eq!(
        via_stranger, 0,
        "portal_get_listing_analytics must return nothing for a non-owner"
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
