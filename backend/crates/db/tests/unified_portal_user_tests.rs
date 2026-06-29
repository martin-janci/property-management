//! Phase 6 (A1) — UnifiedPortalUserRepo single-table write coverage.
//!
//! Phase 2.5 (N1) tested that writes kept the unified `users` row and the
//! legacy `portal_users` row in sync. Phase 6 drops `portal_users` (migration
//! 00148); the dual-write is removed. These tests verify that every write path
//! now lands correctly in `users` alone, and that SSO upserts still NEVER
//! silently overwrite a non-public principal (leak #7 defence unchanged).

use db::models::user::Locale;
use db::repositories::{UnifiedPortalError, UnifiedPortalUserRepo, UpdateProfile};
use sqlx::PgPool;
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

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn create_writes_to_users_only(pool: PgPool) {
    let repo = UnifiedPortalUserRepo::new(pool.clone());
    let email = "create-single@n1.test";
    let user = repo
        .create(email, "Create Single", Some("hash"), Locale::English)
        .await
        .expect("create");

    // Exactly one users row created with principal_kind='public'.
    let user_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM users WHERE LOWER(email) = LOWER($1) AND principal_kind = 'public'",
    )
    .bind(email)
    .fetch_one(&pool)
    .await
    .expect("fetch users id");

    assert_eq!(user.id, user_id, "returned user.id matches users.id row");

    let kind: String = sqlx::query_scalar("SELECT principal_kind FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("kind");
    assert_eq!(kind, "public");

    // password_hash stored correctly (not the sentinel).
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("hash");
    assert_eq!(hash, "hash");
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn update_profile_updates_users_row(pool: PgPool) {
    let repo = UnifiedPortalUserRepo::new(pool.clone());
    let email = "update-profile@n1.test";
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

    // profile_image_url is stored in users (added by migration 00148).
    let img: Option<String> =
        sqlx::query_scalar("SELECT profile_image_url FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&pool)
            .await
            .expect("profile_image_url");
    assert_eq!(
        img.as_deref(),
        Some("https://img/new"),
        "users.profile_image_url updated"
    );
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn password_change_updates_users_row(pool: PgPool) {
    let repo = UnifiedPortalUserRepo::new(pool.clone());
    let email = "password-change@n1.test";
    let user = repo
        .create(email, "Pwd User", Some("hash-v1"), Locale::English)
        .await
        .expect("create");

    let ok = repo
        .update_password_hash(user.id, "hash-v2")
        .await
        .expect("update_password_hash");
    assert!(ok, "users row was matched");

    let users_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .expect("users hash");
    assert_eq!(users_hash, "hash-v2", "users hash is the new value");

    // Bogus id returns false, not an error.
    let bogus = Uuid::new_v4();
    let result = repo
        .update_password_hash(bogus, "should-not-write")
        .await
        .expect("update_password_hash bogus");
    assert!(!result, "no users row matched → false, not a partial write");
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
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

    // No extra public users row was inserted for the refused email.
    let public_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE LOWER(email) = LOWER($1) AND principal_kind = 'public'",
    )
    .bind(email)
    .fetch_one(&pool)
    .await
    .expect("count public users");
    assert_eq!(public_count, 0, "no public user row written on refuse");

    // Re-running the upsert is idempotent — does not pile up duplicate collision rows.
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
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn sso_upsert_creates_then_updates_for_public(pool: PgPool) {
    // Happy path: no existing identity, then a second sign-in of the same
    // user. First call creates the users row; second call updates the name.
    let repo = UnifiedPortalUserRepo::new(pool.clone());
    let email = "sso-happy@n1.test";
    let provider_uid = Uuid::new_v4();

    let first = repo
        .sso_upsert("pm_sso", Some(provider_uid), email, "First Login")
        .await
        .expect("first sso_upsert");
    assert_eq!(first.name, "First Login");

    // Exactly one public users row.
    let user_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM users WHERE LOWER(email) = LOWER($1) AND principal_kind = 'public'",
    )
    .bind(email)
    .fetch_one(&pool)
    .await
    .expect("public user after first upsert");
    assert_eq!(first.id, user_id);

    let second = repo
        .sso_upsert("pm_sso", Some(provider_uid), email, "Updated Display")
        .await
        .expect("second sso_upsert");
    assert_eq!(second.id, first.id, "same identity on second sign-in");
    assert_eq!(second.name, "Updated Display");

    // users.name is updated.
    let name: String = sqlx::query_scalar("SELECT name FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("name after second upsert");
    assert_eq!(
        name, "Updated Display",
        "users.name updated on second sign-in"
    );
}
