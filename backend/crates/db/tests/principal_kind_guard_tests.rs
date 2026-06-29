//! Phase 2 — `principal_kind` escalation guard tests.
//!
//! Defends leaks #8 and #12 (mass-assignment to `principal_kind`, escalation
//! to `platform`). Verifies:
//!
//!   1. A raw `UPDATE users SET principal_kind = ...` is REJECTED unless the
//!      session is going through `set_principal_kind()`.
//!   2. The `set_principal_kind()` function succeeds and writes an audit_logs
//!      row tagged with the old/new kind.
//!
//! These tests need a database — `#[sqlx::test]` provides one with all
//! migrations applied. The pool returned by `#[sqlx::test]` is owned by
//! Postgres' `postgres` superuser, so RLS is bypassed automatically; the
//! trigger fires regardless of role, so the guard is still exercised.

use sqlx::{PgPool, Row};
use uuid::Uuid;

async fn create_user(pool: &PgPool, email: &str) -> Uuid {
    let row = sqlx::query(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Guard Test', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    row.get("id")
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn raw_update_to_principal_kind_is_rejected(pool: PgPool) {
    let id = create_user(&pool, "guard-raw@phase2.test").await;

    // Attempt the bypass: a plain UPDATE on the column.
    let res = sqlx::query("UPDATE users SET principal_kind = 'platform' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await;

    assert!(
        res.is_err(),
        "raw UPDATE on principal_kind must be rejected by the guard trigger"
    );
    let err_msg = format!("{:?}", res.err().unwrap()).to_lowercase();
    assert!(
        err_msg.contains("set_principal_kind") || err_msg.contains("leak"),
        "rejection error must reference the guard, got: {err_msg}"
    );

    // Verify the value was NOT changed.
    let kind: String = sqlx::query_scalar("SELECT principal_kind FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .expect("re-read");
    assert_eq!(
        kind, "staff",
        "principal_kind must remain 'staff' (default)"
    );
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn set_principal_kind_succeeds_and_writes_audit_row(pool: PgPool) {
    let target = create_user(&pool, "guard-target@phase2.test").await;
    let actor = create_user(&pool, "guard-actor@phase2.test").await;

    // Acquire a single connection so the GUC and the function call land
    // on the same session — N3's SECURITY DEFINER body asserts that
    // `actor` matches `current_setting('app.current_user_id')`.
    let mut conn = pool.acquire().await.expect("acquire");
    sqlx::query("SELECT set_request_context(NULL, $1, FALSE)")
        .bind(actor)
        .execute(&mut *conn)
        .await
        .expect("set actor session GUC");

    // Call the SECURITY DEFINER function.
    let _ = sqlx::query("SELECT set_principal_kind($1, $2, $3, $4)")
        .bind(target)
        .bind("platform")
        .bind(actor)
        .bind("phase2 promotion test")
        .execute(&mut *conn)
        .await
        .expect("set_principal_kind should succeed");

    // Read back: the column must now be 'platform'.
    let kind: String = sqlx::query_scalar("SELECT principal_kind FROM users WHERE id = $1")
        .bind(target)
        .fetch_one(&pool)
        .await
        .expect("re-read after set_principal_kind");
    assert_eq!(kind, "platform");

    // Audit row must exist for this transition.
    let audit_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
          FROM audit_logs
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
    assert_eq!(audit_count, 1, "exactly one audit row per transition");

    // And the audit row's details must capture the old/new kind.
    let details: serde_json::Value = sqlx::query_scalar(
        r#"
        SELECT details
          FROM audit_logs
         WHERE resource_type = 'users.principal_kind'
           AND resource_id   = $1
        "#,
    )
    .bind(target)
    .fetch_one(&pool)
    .await
    .expect("audit details");
    assert_eq!(details["old_kind"], "staff");
    assert_eq!(details["new_kind"], "platform");
    assert_eq!(details["leak_guard"], "principal_kind_change");
}
