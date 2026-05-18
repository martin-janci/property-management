//! Phase 2 — portal_users → users merge collision tests.
//!
//! **Phase 6 note:** migration 00148 dropped `portal_users`. These tests
//! insert directly into that table, which no longer exists in a post-00148
//! schema. All tests in this file are marked `#[ignore]` until they are
//! re-targeted against a synthetic fixture that simulates the pre-Phase-2
//! data shape without relying on the physical `portal_users` table.
//!
//! The invariants these tests guard are now enforced at the application layer
//! (UnifiedPortalUserRepo::sso_upsert collision detection) rather than at the
//! database migration layer.
//!
//! Original contract (preserved for reference):
//!   * Email collision → row in `user_merge_collisions`, NO silent merge into
//!     `users`.
//!   * Non-collision → row in `users` with `principal_kind = 'public'` and
//!     `portal_origin_id` set to the source `portal_users.id`.

use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Insert a `portal_users` row directly. Returns the new id.
async fn seed_portal_user(pool: &PgPool, email: &str, name: &str) -> Uuid {
    let row = sqlx::query(
        r#"
        INSERT INTO portal_users (email, name, password_hash, provider, email_verified)
        VALUES ($1, $2, 'pw_hash', 'local', true)
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed portal_user");
    row.get("id")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    let row = sqlx::query(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'staff_hash', 'Pre-Existing Staff', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    row.get("id")
}

/// Re-runs the merge logic from migration 00132 on whatever `portal_users` /
/// `users` rows currently exist. The migration body is wrapped in a DO block;
/// we replicate the two SQL statements here so the test exercises the same
/// code path each time.
async fn run_merge(pool: &PgPool) {
    sqlx::query(
        r#"
        INSERT INTO user_merge_collisions (
            source_table, source_id, target_email, payload, status
        )
        SELECT 'portal_users',
               pu.id,
               pu.email,
               jsonb_build_object(
                   'portal_user_id', pu.id,
                   'portal_user_email', pu.email,
                   'reason', 'email_already_in_users_table'
               ),
               'pending'
          FROM portal_users pu
          JOIN users u ON LOWER(u.email) = LOWER(pu.email)
         WHERE NOT EXISTS (
                 SELECT 1 FROM users u2 WHERE u2.portal_origin_id = pu.id
           )
           AND NOT EXISTS (
                 SELECT 1 FROM user_merge_collisions c
                  WHERE c.source_table = 'portal_users'
                    AND c.source_id    = pu.id
                    AND c.status       = 'pending'
           )
        "#,
    )
    .execute(pool)
    .await
    .expect("collision insert");

    sqlx::query(
        r#"
        INSERT INTO users (
            email, password_hash, name, locale, status, email_verified_at,
            principal_kind, portal_origin_id
        )
        SELECT pu.email,
               COALESCE(pu.password_hash, '!sso-only-no-password'),
               pu.name,
               CASE WHEN pu.locale IN ('sk','cs','de','en') THEN pu.locale ELSE 'en' END,
               'active',
               CASE WHEN pu.email_verified THEN COALESCE(pu.updated_at, NOW()) ELSE NULL END,
               'public',
               pu.id
          FROM portal_users pu
         WHERE NOT EXISTS (
                 SELECT 1 FROM users u WHERE LOWER(u.email) = LOWER(pu.email)
           )
           AND NOT EXISTS (
                 SELECT 1 FROM users u2 WHERE u2.portal_origin_id = pu.id
           )
        "#,
    )
    .execute(pool)
    .await
    .expect("non-collision insert");
}

#[ignore = "Phase 6: portal_users table dropped (migration 00148); re-target before re-enabling"]
#[sqlx::test]
async fn collision_writes_collision_row_no_silent_merge(pool: PgPool) {
    let email = "collision@phase2.test";

    // Pre-existing users row (a staff member with this email).
    let staff_id = seed_user(&pool, email).await;
    // Portal user with the same email — different human.
    let portal_id = seed_portal_user(&pool, email, "Portal Person").await;

    run_merge(&pool).await;

    // Exactly one users row for this email — the pre-existing staff one.
    let user_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE LOWER(email) = LOWER($1)")
            .bind(email)
            .fetch_one(&pool)
            .await
            .expect("count users");
    assert_eq!(
        user_count, 1,
        "no silent merge: still exactly one users row"
    );

    let kind: String = sqlx::query_scalar("SELECT principal_kind FROM users WHERE id = $1")
        .bind(staff_id)
        .fetch_one(&pool)
        .await
        .expect("staff kind");
    assert_eq!(kind, "staff", "the staff row's kind is unchanged");

    // A collision row exists for the portal_user.
    let collision_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM user_merge_collisions
         WHERE source_table = 'portal_users'
           AND source_id    = $1
        "#,
    )
    .bind(portal_id)
    .fetch_one(&pool)
    .await
    .expect("count collisions");
    assert_eq!(
        collision_count, 1,
        "exactly one pending collision row for the portal_user"
    );
}

#[ignore = "Phase 6: portal_users table dropped (migration 00148); re-target before re-enabling"]
#[sqlx::test]
async fn non_collision_inserts_users_row_with_origin_pointer(pool: PgPool) {
    let portal_id = seed_portal_user(&pool, "fresh-portal@phase2.test", "Fresh Portal").await;

    run_merge(&pool).await;

    let row = sqlx::query(
        r#"
        SELECT id, principal_kind, portal_origin_id
          FROM users
         WHERE portal_origin_id = $1
        "#,
    )
    .bind(portal_id)
    .fetch_one(&pool)
    .await
    .expect("fetch merged user");

    let kind: String = row.get("principal_kind");
    let back: Uuid = row.get("portal_origin_id");
    assert_eq!(
        kind, "public",
        "merged portal user must be principal_kind=public"
    );
    assert_eq!(back, portal_id, "portal_origin_id back-pointer set");

    // No collision row should exist for this portal_user.
    let coll: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_merge_collisions WHERE source_id = $1")
            .bind(portal_id)
            .fetch_one(&pool)
            .await
            .expect("no collisions");
    assert_eq!(coll, 0);
}

#[ignore = "Phase 6: portal_users table dropped (migration 00148); re-target before re-enabling"]
#[sqlx::test]
async fn merge_is_idempotent(pool: PgPool) {
    let portal_id = seed_portal_user(&pool, "idempotent@phase2.test", "Once Only").await;

    run_merge(&pool).await;
    run_merge(&pool).await;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE portal_origin_id = $1")
        .bind(portal_id)
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count, 1, "second merge must not duplicate the user row");
}
