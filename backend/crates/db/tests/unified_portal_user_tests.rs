//! Phase 2.5 (N1) — UnifiedPortalUserRepo dual-write coverage.
//!
//! Verifies the forward-going invariant that every write path keeps the
//! unified `users` row and the legacy `portal_users` row in sync, and that
//! SSO upserts NEVER silently overwrite a non-public principal who already
//! owns the email (defends leak #7 the same way the merge migration does).

use db::models::user::Locale;
use db::repositories::{UnifiedPortalError, UnifiedPortalUserRepo, UpdateProfile};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Insert a `users` row with the given email + principal_kind directly so
/// we can stage collision scenarios. Returns the new id.
async fn seed_user_with_kind(pool: &PgPool, email: &str, kind: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'staff_hash', 'Pre-Existing', 'active', NOW(), $2)
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(kind)
    .fetch_one(pool)
    .await
    .expect("seed users row")
}

/// Read both (users.id, portal_users.id) for a freshly created public user
/// by email, asserting one row each. Returns the pair of ids.
async fn fetch_dual_ids(pool: &PgPool, email: &str) -> (Uuid, Uuid) {
    let user_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM users WHERE LOWER(email) = LOWER($1) AND principal_kind = 'public'",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("fetch users id");
    let portal_id: Uuid =
        sqlx::query_scalar("SELECT id FROM portal_users WHERE LOWER(email) = LOWER($1)")
            .bind(email)
            .fetch_one(pool)
            .await
            .expect("fetch portal_users id");
    (user_id, portal_id)
}

#[sqlx::test]
async fn create_writes_both_tables(pool: PgPool) {
    let repo = UnifiedPortalUserRepo::new(pool.clone());
    let email = "create-both@n1.test";
    let user = repo
        .create(email, "Create Both", Some("hash"), Locale::English)
        .await
        .expect("create");

    let (users_id, portal_id) = fetch_dual_ids(&pool, email).await;

    // users.id matches the returned user.
    assert_eq!(user.id, users_id, "returned user.id matches users.id row");
    // The back-pointer is set to the portal_users row.
    let pointed: Uuid = sqlx::query_scalar("SELECT portal_origin_id FROM users WHERE id = $1")
        .bind(users_id)
        .fetch_one(&pool)
        .await
        .expect("portal_origin_id");
    assert_eq!(
        pointed, portal_id,
        "users.portal_origin_id back-points at the new portal_users row"
    );
    // The legacy back-pointer (portal_users.pm_user_id) points at users.id.
    let pm_user_id: Uuid = sqlx::query_scalar("SELECT pm_user_id FROM portal_users WHERE id = $1")
        .bind(portal_id)
        .fetch_one(&pool)
        .await
        .expect("portal_users.pm_user_id");
    assert_eq!(
        pm_user_id, users_id,
        "portal_users.pm_user_id back-points at the new users row"
    );
    // principal_kind is set correctly.
    let kind: String = sqlx::query_scalar("SELECT principal_kind FROM users WHERE id = $1")
        .bind(users_id)
        .fetch_one(&pool)
        .await
        .expect("kind");
    assert_eq!(kind, "public");
}

#[sqlx::test]
async fn update_profile_mirrors_to_both(pool: PgPool) {
    let repo = UnifiedPortalUserRepo::new(pool.clone());
    let email = "update-mirror@n1.test";
    let user = repo
        .create(email, "Old Name", Some("hash"), Locale::English)
        .await
        .expect("create");

    let updated = repo
        .update_profile(
            user.id,
            UpdateProfile {
                name: Some("New Name".to_string()),
                profile_image_url: Some("https://img/new".to_string()),
                locale: Some("sk".to_string()),
            },
        )
        .await
        .expect("update_profile")
        .expect("user found");

    assert_eq!(updated.name, "New Name", "users.name reflects update");
    assert_eq!(updated.locale, "sk", "users.locale reflects update");

    let (_, portal_id) = fetch_dual_ids(&pool, email).await;
    let row = sqlx::query("SELECT name, profile_image_url, locale FROM portal_users WHERE id = $1")
        .bind(portal_id)
        .fetch_one(&pool)
        .await
        .expect("read portal_users");
    let p_name: String = row.get("name");
    let p_image: Option<String> = row.get("profile_image_url");
    let p_locale: String = row.get("locale");
    assert_eq!(p_name, "New Name", "portal_users.name mirrored");
    assert_eq!(
        p_image.as_deref(),
        Some("https://img/new"),
        "portal_users.profile_image_url mirrored"
    );
    assert_eq!(p_locale, "sk", "portal_users.locale mirrored");
}

#[sqlx::test]
async fn password_change_atomic(pool: PgPool) {
    // Atomicity: this is a single Postgres transaction in the repo
    // implementation. We can't easily fault-inject here without a custom
    // wrapper around the connection, so the test asserts the "happy" path
    // (both tables updated to the same value) and a comment documents the
    // tx-boundary contract.
    //
    // The behavior under fault: if the second UPDATE (portal_users) failed,
    // `tx.commit()` would never run and the first UPDATE would roll back.
    // Therefore the only observable states are (old, old) and (new, new).
    // (new, old) and (old, new) are unrepresentable.

    let repo = UnifiedPortalUserRepo::new(pool.clone());
    let email = "password-atomic@n1.test";
    let user = repo
        .create(email, "Pwd User", Some("hash-v1"), Locale::English)
        .await
        .expect("create");

    let ok = repo
        .update_password_hash(user.id, "hash-v2")
        .await
        .expect("update_password_hash");
    assert!(ok, "users row was matched");

    let (users_id, portal_id) = fetch_dual_ids(&pool, email).await;

    let users_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(users_id)
        .fetch_one(&pool)
        .await
        .expect("users hash");
    let portal_hash: Option<String> =
        sqlx::query_scalar("SELECT password_hash FROM portal_users WHERE id = $1")
            .bind(portal_id)
            .fetch_one(&pool)
            .await
            .expect("portal hash");
    assert_eq!(users_hash, "hash-v2", "users hash is the new value");
    assert_eq!(
        portal_hash.as_deref(),
        Some("hash-v2"),
        "portal_users hash mirrored to the new value (same tx as users update)"
    );

    // Refuse on a non-existent user id is a no-op (returns false), not a
    // partial write.
    let bogus = Uuid::new_v4();
    let result = repo
        .update_password_hash(bogus, "should-not-write")
        .await
        .expect("update_password_hash bogus");
    assert!(!result, "no users row matched → false, not a partial write");
}

#[sqlx::test]
async fn sso_upsert_handles_collision(pool: PgPool) {
    // Stage: a STAFF principal already owns the email in `users`. An SSO
    // sign-in for the same email arrives. The repo MUST refuse rather than
    // silently overwrite the staff row, and a `user_merge_collisions` row
    // must be queued for human review.
    let email = "collide@n1.test";
    let staff_id = seed_user_with_kind(&pool, email, "staff").await;

    let repo = UnifiedPortalUserRepo::new(pool.clone());
    let provider_uid = Uuid::new_v4();
    let result = repo
        .sso_upsert("pm_sso", Some(provider_uid), email, "Should Not Overwrite")
        .await;

    match result {
        Err(UnifiedPortalError::Collision { existing_user_id }) => {
            assert_eq!(
                existing_user_id, staff_id,
                "collision identifies the staff row"
            );
        }
        other => panic!("expected Collision error, got {other:?}"),
    }

    // The staff row is unchanged.
    let kind: String = sqlx::query_scalar("SELECT principal_kind FROM users WHERE id = $1")
        .bind(staff_id)
        .fetch_one(&pool)
        .await
        .expect("staff kind");
    assert_eq!(kind, "staff", "staff principal_kind not mutated");

    let name: String = sqlx::query_scalar("SELECT name FROM users WHERE id = $1")
        .bind(staff_id)
        .fetch_one(&pool)
        .await
        .expect("staff name");
    assert_eq!(name, "Pre-Existing", "staff name not overwritten");

    // Collision row queued.
    let collision_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM user_merge_collisions
         WHERE source_table = 'users'
           AND source_id    = $1
           AND status       = 'pending'
        "#,
    )
    .bind(staff_id)
    .fetch_one(&pool)
    .await
    .expect("count collisions");
    assert_eq!(collision_count, 1, "exactly one pending collision row");

    // No portal_users row was inserted for this email — the refuse came
    // before the portal mirror.
    let portal_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM portal_users WHERE LOWER(email) = LOWER($1)")
            .bind(email)
            .fetch_one(&pool)
            .await
            .expect("count portal_users");
    assert_eq!(portal_count, 0, "no portal mirror written on refuse");

    // Re-running the upsert is idempotent — does not pile up duplicate
    // collision rows.
    let _ = repo
        .sso_upsert("pm_sso", Some(provider_uid), email, "Try Again")
        .await
        .expect_err("second attempt also refuses");
    let collision_count_2: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM user_merge_collisions
         WHERE source_table = 'users'
           AND source_id    = $1
        "#,
    )
    .bind(staff_id)
    .fetch_one(&pool)
    .await
    .expect("count collisions 2");
    assert_eq!(
        collision_count_2, 1,
        "collision insert is idempotent; no duplicates queued"
    );
}

#[sqlx::test]
async fn sso_upsert_creates_then_updates_for_public(pool: PgPool) {
    // Happy path: no existing identity, then a second sign-in of the same
    // user. First call creates both rows; second call updates the name on
    // both without raising a collision.
    let repo = UnifiedPortalUserRepo::new(pool.clone());
    let email = "sso-happy@n1.test";
    let provider_uid = Uuid::new_v4();

    let first = repo
        .sso_upsert("pm_sso", Some(provider_uid), email, "First Login")
        .await
        .expect("first sso_upsert");
    assert_eq!(first.name, "First Login");

    let (_, portal_id) = fetch_dual_ids(&pool, email).await;
    let p_name: String = sqlx::query_scalar("SELECT name FROM portal_users WHERE id = $1")
        .bind(portal_id)
        .fetch_one(&pool)
        .await
        .expect("portal name");
    assert_eq!(p_name, "First Login", "portal mirror has the same name");

    let second = repo
        .sso_upsert("pm_sso", Some(provider_uid), email, "Updated Display")
        .await
        .expect("second sso_upsert");
    assert_eq!(second.id, first.id, "same identity on second sign-in");
    assert_eq!(second.name, "Updated Display");

    let p_name_2: String = sqlx::query_scalar("SELECT name FROM portal_users WHERE id = $1")
        .bind(portal_id)
        .fetch_one(&pool)
        .await
        .expect("portal name 2");
    assert_eq!(
        p_name_2, "Updated Display",
        "portal mirror reflects the updated name"
    );
}
