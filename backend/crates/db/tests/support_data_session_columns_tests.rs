//! Regression tests for the Support Data session queries (Story 10B.5, #635).
//!
//! Background
//! ----------
//! `PlatformAdminRepository::get_user_sessions`, `revoke_user_sessions`, and
//! the `active_sessions` count inside `get_support_data` query the
//! `refresh_tokens` table. They originally referenced an `is_revoked` boolean
//! column (and `updated_at`) which does NOT exist on that table — migration
//! `00002_create_refresh_tokens.sql` only defines `revoked_at TIMESTAMPTZ`
//! (nullable) and has no `updated_at`. Because these are runtime `query`/
//! `query_as` calls (not compile-time-checked), `cargo check` accepted the
//! wrong column names and the bug only surfaced as a Postgres
//! `column "is_revoked" does not exist` error at request time.
//!
//! These tests run the three queries against a real schema-migrated database
//! via `#[sqlx::test]`, so they would have failed on the pre-fix code and now
//! prove the queries use the correct `revoked_at IS NULL` semantics.

use chrono::{Duration, Utc};
use db::repositories::PlatformAdminRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Support Test User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Insert a refresh token. `revoked` controls whether `revoked_at` is set;
/// `expired` controls whether `expires_at` is in the past.
async fn seed_refresh_token(pool: &PgPool, user_id: Uuid, hash: &str, revoked: bool, expired: bool) {
    let expires_at = if expired {
        Utc::now() - Duration::hours(1)
    } else {
        Utc::now() + Duration::days(7)
    };
    let revoked_at = if revoked { Some(Utc::now()) } else { None };

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at, revoked_at, user_agent, ip_address)
        VALUES ($1, $2, $3, $4, 'test-agent', '127.0.0.1')
        "#,
    )
    .bind(user_id)
    .bind(hash)
    .bind(expires_at)
    .bind(revoked_at)
    .execute(pool)
    .await
    .expect("seed refresh token");
}

/// `get_user_sessions` must list only active (non-revoked, non-expired) tokens
/// and must run without referencing a non-existent `is_revoked` column.
#[sqlx::test(migrations = "./migrations")]
async fn get_user_sessions_lists_only_active(pool: PgPool) {
    let user_id = seed_user(&pool, "sessions@example.com").await;
    seed_refresh_token(&pool, user_id, "active-1", false, false).await;
    seed_refresh_token(&pool, user_id, "active-2", false, false).await;
    seed_refresh_token(&pool, user_id, "revoked", true, false).await;
    seed_refresh_token(&pool, user_id, "expired", false, true).await;

    let repo = PlatformAdminRepository::new(pool);
    let sessions = repo
        .get_user_sessions(user_id)
        .await
        .expect("get_user_sessions should not error on column names");

    // Only the two active tokens count; revoked + expired are excluded.
    assert_eq!(sessions.len(), 2, "should list only active sessions");
}

/// `revoke_user_sessions` must revoke all currently-active tokens (and only
/// those) by setting `revoked_at`, returning the affected-row count.
#[sqlx::test(migrations = "./migrations")]
async fn revoke_user_sessions_revokes_active_only(pool: PgPool) {
    let user_id = seed_user(&pool, "revoke@example.com").await;
    seed_refresh_token(&pool, user_id, "r-active-1", false, false).await;
    seed_refresh_token(&pool, user_id, "r-active-2", false, false).await;
    seed_refresh_token(&pool, user_id, "r-already-revoked", true, false).await;

    let repo = PlatformAdminRepository::new(pool.clone());
    let revoked = repo
        .revoke_user_sessions(user_id)
        .await
        .expect("revoke_user_sessions should not error on column names");

    assert_eq!(revoked, 2, "should revoke the two active tokens only");

    // After revocation there should be no active sessions left.
    let sessions = repo
        .get_user_sessions(user_id)
        .await
        .expect("get_user_sessions after revoke");
    assert_eq!(sessions.len(), 0, "all sessions revoked");
}

/// `get_support_data().active_sessions` must count only active refresh tokens
/// and must run without referencing a non-existent `is_revoked` column.
#[sqlx::test(migrations = "./migrations")]
async fn support_data_active_sessions_counts_active_only(pool: PgPool) {
    let user_id = seed_user(&pool, "support-data@example.com").await;
    seed_refresh_token(&pool, user_id, "sd-active-1", false, false).await;
    seed_refresh_token(&pool, user_id, "sd-active-2", false, false).await;
    seed_refresh_token(&pool, user_id, "sd-active-3", false, false).await;
    seed_refresh_token(&pool, user_id, "sd-revoked", true, false).await;
    seed_refresh_token(&pool, user_id, "sd-expired", false, true).await;

    let repo = PlatformAdminRepository::new(pool);
    let data = repo
        .get_support_data()
        .await
        .expect("get_support_data should not error on column names");

    assert_eq!(
        data.active_sessions, 3,
        "active_sessions should count only non-revoked, non-expired tokens"
    );
}
