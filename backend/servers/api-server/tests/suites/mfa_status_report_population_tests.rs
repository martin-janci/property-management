#![allow(dead_code)]

//! Regression: `get_mfa_status_report` population mismatch (code-review finding
//! `code-review-api-handlers-mfa-report-population-mismatch`).
//!
//! The GDPR/security MFA-adoption report computed its two aggregates over
//! DIFFERENT populations:
//!
//!   * `mfa_enabled` = `SELECT COUNT(*) FROM user_2fa WHERE enabled = true`
//!     — every enabled 2FA row, regardless of the owning user's status.
//!   * `total_users` = `SELECT COUNT(*) FROM users WHERE status = 'active'`
//!     — active users only.
//!
//! Because the numerator was drawn from a SUPERSET of the denominator, a
//! suspended / pending / soft-deleted user that still owns an enabled
//! `user_2fa` row inflated the enabled count: `mfa_disabled_count` could go
//! NEGATIVE and `adoption_rate` could exceed 100 %. The fix scopes the enabled
//! count to active users too (`JOIN users ... WHERE u.status = 'active'`),
//! matching the sibling `get_privacy_settings_report`.
//!
//! The report counts are platform-global, so the test uses a `super_admin`
//! caller (RLS bypass) and asserts on DELTAS around a controlled seed — this is
//! robust against the rows the bootstrap migration already seeds into the test
//! DB.

use axum::http::StatusCode;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp};

const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";
const MFA_STATUS_PATH: &str = "/api/v1/compliance/security/mfa-status";

#[derive(Serialize)]
struct TestClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

/// Mint an access JWT for `user_id` with the given org and role.
fn mint_token(user_id: Uuid, org_id: Option<Uuid>, role: Option<&str>) -> String {
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        iat: now,
        exp: now + 3600,
        token_type: "access".to_string(),
        tenant_id: org_id,
        role: role.map(str::to_string),
        email: format!("{user_id}@mfa-pop.test"),
        name: "MFA Pop Test User".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint jwt")
}

/// Seed a user with an explicit `status` and return its id.
async fn seed_user_status(pool: &PgPool, status: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
           VALUES ($1, 'x', 'MFA Pop User', $2, NOW(), 'staff') RETURNING id"#,
    )
    .bind(format!("mfa-pop-{}@mfa-pop.test", Uuid::new_v4()))
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("seed user with status")
}

/// Give `user_id` an enabled TOTP 2FA row.
async fn enable_2fa(pool: &PgPool, user_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO user_2fa (user_id, secret, enabled, enabled_at)
           VALUES ($1, 'seed-secret', true, NOW())"#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("enable 2fa");
}

/// Parsed MFA-status aggregates.
struct Report {
    mfa_enabled_count: i64,
    mfa_disabled_count: i64,
    total_users: i64,
    adoption_rate: f64,
}

/// Call the MFA-status report endpoint and parse the response.
async fn fetch_report(app: &TestApp, token: &str, org_id: Uuid) -> Report {
    let resp = app
        .get(MFA_STATUS_PATH)
        .bearer(token)
        .tenant(org_id)
        .build();
    let r = app.execute(resp).await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "mfa-status report should return 200 for a super_admin caller, body: {}",
        r.text()
    );
    let body = r.json_value();
    Report {
        mfa_enabled_count: body["mfa_enabled_count"]
            .as_i64()
            .expect("mfa_enabled_count"),
        mfa_disabled_count: body["mfa_disabled_count"]
            .as_i64()
            .expect("mfa_disabled_count"),
        total_users: body["total_users"].as_i64().expect("total_users"),
        adoption_rate: body["adoption_rate"].as_f64().expect("adoption_rate"),
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mfa_report_enabled_count_excludes_non_active_users(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // super_admin caller: passes `require_platform_admin`, and the RLS bypass
    // lets the platform-global COUNT(*) see every user / user_2fa row.
    let org_id = seed_org(&pool, "mfa-pop").await;
    let admin = seed_user_status(&pool, "active").await;
    seed_membership(&pool, org_id, admin, "super_admin").await;
    let token = mint_token(admin, Some(org_id), Some("super_admin"));

    // Baseline: the bootstrap migration seeds rows into the test DB, so measure
    // around a baseline snapshot rather than asserting absolute counts.
    let before = fetch_report(&app, &token, org_id).await;

    // Controlled seed:
    //   a1 — active + enabled 2FA   (must count toward BOTH enabled and total)
    //   s1 — SUSPENDED + enabled 2FA (must count toward NEITHER)
    //   a2 — active, no 2FA          (must count toward total only)
    let a1 = seed_user_status(&pool, "active").await;
    enable_2fa(&pool, a1).await;
    let s1 = seed_user_status(&pool, "suspended").await;
    enable_2fa(&pool, s1).await;
    let _a2 = seed_user_status(&pool, "active").await;

    let after = fetch_report(&app, &token, org_id).await;

    let delta_total = after.total_users - before.total_users;
    let delta_enabled = after.mfa_enabled_count - before.mfa_enabled_count;

    // Two active users were added; the suspended user must not be counted.
    assert_eq!(
        delta_total, 2,
        "total_users must grow by the 2 new ACTIVE users (suspended excluded)"
    );

    // Core of the fix: the suspended user's enabled 2FA row must NOT be counted.
    // Pre-fix this delta is 2 (both enabled rows); post-fix it is 1.
    assert_eq!(
        delta_enabled, 1,
        "mfa_enabled_count must grow by 1 (only the ACTIVE user's 2FA); the \
         SUSPENDED user's enabled row must be excluded"
    );

    // Global invariants the population mismatch could violate outright.
    assert!(
        after.mfa_disabled_count >= 0,
        "mfa_disabled_count must never be negative, got {}",
        after.mfa_disabled_count
    );
    assert!(
        after.adoption_rate <= 100.0,
        "adoption_rate must never exceed 100%, got {}",
        after.adoption_rate
    );
    assert!(
        after.mfa_enabled_count <= after.total_users,
        "enabled (active-scoped) count {} must not exceed active total {}",
        after.mfa_enabled_count,
        after.total_users
    );
}
