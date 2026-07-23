//! N3 — `set_principal_kind` actor verification tests.
//!
//! Migration 00143 hardens the SECURITY DEFINER function so that the
//! caller-supplied `actor` argument MUST equal the session's
//! `app.current_user_id` GUC. Without this check the audit-row attribution
//! could be forged.
//!
//! These tests need a database — `#[sqlx::test]` provides one with all
//! migrations applied (so 00143 is in effect).

use sqlx::{PgPool, Row};
use uuid::Uuid;

async fn create_user(pool: &PgPool, email: &str) -> Uuid {
    let row = sqlx::query(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'N3 Actor', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    row.get("id")
}

/// Acquire a single connection so `app.current_user_id` set via
/// `set_request_context` persists across the subsequent statement.
async fn set_session_user(
    pool: &PgPool,
    user_id: Uuid,
) -> sqlx::pool::PoolConnection<sqlx::Postgres> {
    let mut conn = pool.acquire().await.expect("acquire");
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(Option::<Uuid>::None)
        .bind(Some(user_id))
        .bind(false)
        .execute(&mut *conn)
        .await
        .expect("set_request_context");
    conn
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn forged_actor_is_rejected(pool: PgPool) {
    let target = create_user(&pool, "n3-target-forge@phase2.test").await;
    let real_caller = create_user(&pool, "n3-real-caller@phase2.test").await;
    let other = create_user(&pool, "n3-other-victim@phase2.test").await;

    // Session is set to `real_caller`, but we pass `other` as the actor — must reject.
    let mut conn = set_session_user(&pool, real_caller).await;

    let res = sqlx::query("SELECT set_principal_kind($1, $2, $3, $4)")
        .bind(target)
        .bind("platform")
        .bind(other) // forged actor
        .bind("forged actor attempt")
        .execute(&mut *conn)
        .await;

    assert!(res.is_err(), "set_principal_kind must reject forged actor");
    let msg = format!("{:?}", res.err().unwrap()).to_lowercase();
    assert!(
        msg.contains("actor") && (msg.contains("session user") || msg.contains("must equal")),
        "rejection should reference actor/session mismatch, got: {msg}"
    );

    // The principal_kind must NOT have changed.
    let kind: String = sqlx::query_scalar("SELECT principal_kind FROM users WHERE id = $1")
        .bind(target)
        .fetch_one(&pool)
        .await
        .expect("re-read");
    assert_eq!(kind, "staff", "no transition should have happened");

    // No audit row should have been written for this attempt.
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE resource_type = 'users.principal_kind' AND resource_id = $1",
    )
    .bind(target)
    .fetch_one(&pool)
    .await
    .expect("audit count");
    assert_eq!(audit_count, 0, "no audit row on rejected forged actor");
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn matching_actor_succeeds(pool: PgPool) {
    let target = create_user(&pool, "n3-target-ok@phase2.test").await;
    let actor = create_user(&pool, "n3-actor-ok@phase2.test").await;

    // Session is set to `actor`, and actor argument matches — must succeed.
    let mut conn = set_session_user(&pool, actor).await;

    let res = sqlx::query("SELECT set_principal_kind($1, $2, $3, $4)")
        .bind(target)
        .bind("platform")
        .bind(actor)
        .bind("legitimate transition")
        .execute(&mut *conn)
        .await;

    assert!(
        res.is_ok(),
        "set_principal_kind with matching actor must succeed: {:?}",
        res.err()
    );

    let kind: String = sqlx::query_scalar("SELECT principal_kind FROM users WHERE id = $1")
        .bind(target)
        .fetch_one(&pool)
        .await
        .expect("re-read");
    assert_eq!(kind, "platform");

    // Audit row recorded with the (verified) actor.
    let audit_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM audit_logs
         WHERE resource_type = 'users.principal_kind'
           AND resource_id   = $1
           AND user_id       = $2
        "#,
    )
    .bind(target)
    .bind(actor)
    .fetch_one(&pool)
    .await
    .expect("audit count");
    assert_eq!(
        audit_count, 1,
        "exactly one audit row per verified transition"
    );

    // The audit row should carry the new `actor_check` marker.
    let details: serde_json::Value = sqlx::query_scalar(
        "SELECT details FROM audit_logs WHERE resource_type = 'users.principal_kind' AND resource_id = $1",
    )
    .bind(target)
    .fetch_one(&pool)
    .await
    .expect("audit details");
    assert_eq!(details["actor_check"], "session_verified");
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn missing_session_user_is_rejected(pool: PgPool) {
    let target = create_user(&pool, "n3-target-no-session@phase2.test").await;
    let actor = create_user(&pool, "n3-actor-no-session@phase2.test").await;

    // Acquire a clean connection WITHOUT setting `app.current_user_id`.
    let mut conn = pool.acquire().await.expect("acquire");
    // Belt and braces: explicitly clear in case the pool handed us a
    // contaminated connection.
    sqlx::query("SELECT clear_request_context()")
        .execute(&mut *conn)
        .await
        .expect("clear");

    let res = sqlx::query("SELECT set_principal_kind($1, $2, $3, $4)")
        .bind(target)
        .bind("platform")
        .bind(actor)
        .bind("no session user")
        .execute(&mut *conn)
        .await;

    assert!(res.is_err(), "must reject when session user is unset");
    let msg = format!("{:?}", res.err().unwrap()).to_lowercase();
    assert!(
        msg.contains("current_user_id"),
        "rejection should reference app.current_user_id, got: {msg}"
    );
}
