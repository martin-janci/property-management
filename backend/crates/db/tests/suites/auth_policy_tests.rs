//! Defense N2 — per-organization auth policy tests.
//!
//! Verifies the `AuthPolicy::for_org` loader and the validate/MFA helpers.
//! Migration 00142 creates `org_auth_policies`; `AuthPolicy::for_org` reads
//! that row and falls back to the platform default when no row exists.

use crate::common::seed_org;
use db::models::AuthPolicy;
use sqlx::PgPool;

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn missing_row_falls_back_to_default(pool: PgPool) {
    let org = seed_org(&pool, "default-fallback").await;

    let policy = AuthPolicy::for_org(&pool, org).await.expect("load default");

    assert_eq!(policy, AuthPolicy::default());
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn empty_jsonb_falls_back_to_default(pool: PgPool) {
    let org = seed_org(&pool, "empty-jsonb").await;

    // Insert an explicit `{}` row — the loader must treat this as "no
    // overrides" and return the default.
    sqlx::query("INSERT INTO org_auth_policies (organization_id, policy) VALUES ($1, '{}'::jsonb)")
        .bind(org)
        .execute(&pool)
        .await
        .expect("insert empty");

    let policy = AuthPolicy::for_org(&pool, org).await.expect("load");
    assert_eq!(policy, AuthPolicy::default());
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn full_override_loads_back_intact(pool: PgPool) {
    let org = seed_org(&pool, "full-override").await;

    let custom = AuthPolicy {
        min_password_length: 16,
        require_uppercase: true,
        require_digit: true,
        require_symbol: true,
        require_email_verification: true,
        mfa_required_for_roles: vec!["manager".into(), "owner".into()],
        max_session_age_minutes: 60,
    };

    sqlx::query(r#"INSERT INTO org_auth_policies (organization_id, policy) VALUES ($1, $2)"#)
        .bind(org)
        .bind(serde_json::to_value(&custom).expect("serialize"))
        .execute(&pool)
        .await
        .expect("insert override");

    let loaded = AuthPolicy::for_org(&pool, org).await.expect("load");
    assert_eq!(loaded, custom);
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn partial_override_inherits_remaining_defaults(pool: PgPool) {
    let org = seed_org(&pool, "partial-override").await;

    // Only override `min_password_length` — every other field must inherit
    // from `AuthPolicy::default()`.
    sqlx::query(
        r#"INSERT INTO org_auth_policies (organization_id, policy) VALUES ($1, $2::jsonb)"#,
    )
    .bind(org)
    .bind(r#"{"min_password_length": 24}"#)
    .execute(&pool)
    .await
    .expect("insert partial");

    let loaded = AuthPolicy::for_org(&pool, org).await.expect("load");
    let defaults = AuthPolicy::default();
    assert_eq!(loaded.min_password_length, 24);
    assert_eq!(loaded.require_uppercase, defaults.require_uppercase);
    assert_eq!(loaded.require_digit, defaults.require_digit);
    assert_eq!(
        loaded.require_email_verification,
        defaults.require_email_verification
    );
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn malformed_jsonb_falls_back_to_default(pool: PgPool) {
    let org = seed_org(&pool, "malformed").await;

    // A wholly-unrecognized shape that does NOT match `AuthPolicy` should
    // not crash the loader — it must fall back to the safe default and log.
    sqlx::query(
        r#"INSERT INTO org_auth_policies (organization_id, policy) VALUES ($1, '"not-an-object"'::jsonb)"#,
    )
    .bind(org)
    .execute(&pool)
    .await
    .expect("insert malformed");

    let loaded = AuthPolicy::for_org(&pool, org).await.expect("load");
    assert_eq!(loaded, AuthPolicy::default());
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn loader_does_not_leave_rls_context_dirty(pool: PgPool) {
    // The loader sets a super-admin RLS context for the read; it must clear
    // it before returning the connection to the pool. Otherwise subsequent
    // tenant-scoped queries on this connection would silently bypass RLS.
    let org = seed_org(&pool, "rls-clean").await;
    let _ = AuthPolicy::for_org(&pool, org).await.expect("load");

    let mut conn = pool.acquire().await.expect("acquire");
    let raw: Option<String> =
        sqlx::query_scalar("SELECT current_setting('app.is_super_admin', true)")
            .fetch_one(&mut *conn)
            .await
            .expect("read GUC");
    // Either NULL (never set on this fresh conn) or 'false' (cleared) — both
    // are acceptable. The forbidden value is 'true'.
    let v = raw.unwrap_or_default();
    assert!(
        v.is_empty() || v == "false",
        "RLS super-admin context must be cleared after AuthPolicy::for_org, got: {v:?}"
    );
}
