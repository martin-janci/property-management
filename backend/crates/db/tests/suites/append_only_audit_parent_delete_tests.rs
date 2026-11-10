//! Gap audit R6 P1-4 — parents of append-only audit rows must stay deletable.
//!
//! `audit_logs`, `vote_audit_log` and `layout_change_events` reject UPDATE and
//! DELETE via triggers. Their FKs used `ON DELETE SET NULL` / `CASCADE`, which
//! Postgres runs as an UPDATE / DELETE on the audit table, so deleting the
//! parent organization, user or vote failed ("append-only: UPDATE rejected").
//! Migration 00245 drops those FKs. These tests pin that a tenant purge's
//! final `DELETE FROM organizations` and a user hard delete succeed while the
//! audit rows (and their historical ids) survive.

use sqlx::PgPool;
use uuid::Uuid;

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO organizations (name, slug, contact_email) VALUES ($1, $1, $1 || '@audit.test') RETURNING id",
    )
    .bind(slug)
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO users (email, password_hash, name, status) VALUES ($1, 'h', 'Audit User', 'active') RETURNING id",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn org_and_user_with_audit_rows_can_be_deleted(pool: PgPool) {
    let org = seed_org(&pool, "audit-purge").await;
    let user = seed_user(&pool, "audit-purge@audit.test").await;
    sqlx::query("INSERT INTO audit_logs (user_id, org_id, action) VALUES ($1, $2, 'login')")
        .bind(user)
        .bind(org)
        .execute(&pool)
        .await
        .expect("seed audit row");

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .expect("user hard delete must not be blocked by the append-only trigger");
    sqlx::query("DELETE FROM organizations WHERE id = $1")
        .bind(org)
        .execute(&pool)
        .await
        .expect("org delete (tenant purge) must not be blocked by the append-only trigger");

    let (kept_user, kept_org): (Option<Uuid>, Option<Uuid>) =
        sqlx::query_as("SELECT user_id, org_id FROM audit_logs WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .expect("audit row survives");
    assert_eq!(kept_user, Some(user));
    assert_eq!(kept_org, Some(org));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn audit_rows_stay_append_only(pool: PgPool) {
    let user = seed_user(&pool, "audit-immutable@audit.test").await;
    sqlx::query("INSERT INTO audit_logs (user_id, action) VALUES ($1, 'login')")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    let res = sqlx::query("UPDATE audit_logs SET user_id = NULL WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await;
    assert!(res.is_err(), "direct UPDATE must still be rejected");
}
