//! Cross-user IDOR regression test for
//! `POST /api/v1/users/me/mfa/recovery-codes/verify` (BIT-78 / GH #1302 + #1304).
//!
//! # Why this lives at the top level of `tests/`
//!
//! The functional flows for this endpoint exist under
//! `tests/integration/mfa_recovery_codes_tests.rs` (T1–T6), but that directory
//! is **never compiled** — nothing declares `mod integration;` and there is no
//! `tests/integration/main.rs`, so Cargo's auto-discovery skips it (see the note
//! in `tests/auth_enumeration_tests.rs`). A test placed there proves nothing
//! because it never runs. This file is a real top-level test target and runs
//! under `cargo test --workspace`.
//!
//! # What it proves (IG3)
//!
//!   **user A cannot consume or observe user B's recovery code.**
//!
//! It is constructed so it FAILS if the `WHERE user_id = $1` scoping were
//! dropped from the handler's queries. `#[sqlx::test]` connects as a pool that
//! is not subject to RLS enforcement, so the per-user isolation under test is
//! carried by the handler's `WHERE user_id` predicate running on a connection
//! whose `app.current_user_id` GUC is set (the BIT-78 `RlsConnection`
//! conversion) — defense-in-depth that also holds once these tables get
//! `FORCE ROW LEVEL SECURITY`. With the filter gone, user A's request would
//! load user B's unused codes, match B's plaintext against B's hash, and
//! consume it (200 + B's row marked `used_at`). The assertions below require
//! the opposite: A is rejected (400), every one of B's codes stays unused, and
//! B can still use the code afterwards.
//!
//! # Wiring notes
//!
//! * `TestApp` mounts the full router but no `host_tenant_middleware`, so the
//!   MFA handlers' `RlsConnection` extractor resolves the tenant from the
//!   `X-Tenant-ID` header + an active `organization_members` row. Each user is
//!   provisioned via `create_authenticated_user_with_org`, and every request
//!   carries `.tenant(org_id)`.
//! * 2FA enrolment + recovery codes are seeded directly in the DB (mirroring the
//!   live `admin_mfa_recovery_tests.rs`) rather than driven through the HTTP
//!   `/mfa/setup` + `/mfa/verify` flow, so the test exercises only the endpoint
//!   under audit and does not depend on TOTP timing / encryption config.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use api_server::services::TotpService;
use common::{cleanup_test_user, create_authenticated_user_with_org, TestApp, TestUser};

// ─── helpers ─────────────────────────────────────────────────────────────────

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("resolve user id")
}

/// Enable 2FA for `user_id` with an encrypted TOTP secret (enrolment state the
/// recovery-verify handler requires: `user_2fa.enabled = true`).
async fn enroll_user_2fa(pool: &PgPool, user_id: Uuid) {
    let totp = TotpService::new("Property Management".to_string());
    let secret = totp.generate_secret().expect("gen secret");
    let encrypted = totp.encrypt_secret(&secret).expect("encrypt secret");
    sqlx::query(
        r#"
        INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
        VALUES ($1, $2, true, NOW(), '[]'::jsonb, 0)
        ON CONFLICT (user_id) DO UPDATE SET enabled = true, enabled_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(&encrypted)
    .execute(pool)
    .await
    .expect("seed user_2fa");
}

/// Insert `n` single-use recovery codes for `user_id`; return the plaintext.
async fn issue_recovery_codes(pool: &PgPool, user_id: Uuid, n: usize) -> Vec<String> {
    let totp = TotpService::new("Property Management".to_string());
    let (plain, hashed) = totp.generate_backup_codes().expect("gen backup codes");
    let plain: Vec<String> = plain.into_iter().take(n).collect();
    let hashed: Vec<String> = hashed.into_iter().take(n).collect();
    for hash in &hashed {
        sqlx::query("INSERT INTO mfa_recovery_codes (user_id, code_hash) VALUES ($1, $2)")
            .bind(user_id)
            .bind(hash)
            .execute(pool)
            .await
            .expect("insert recovery code");
    }
    plain
}

async fn unused_code_count(pool: &PgPool, user_id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM mfa_recovery_codes WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("count unused recovery codes")
}

async fn post_recovery_verify(
    app: &TestApp,
    token: &str,
    org_id: Uuid,
    code: &str,
) -> (StatusCode, Value) {
    let req = app
        .post("/api/v1/users/me/mfa/recovery-codes/verify")
        .bearer(token)
        .tenant(org_id)
        .json(json!({ "code": code }))
        .build();
    let resp = app.execute(req).await;
    (resp.status, resp.json_value())
}

// ─── A cannot consume B's recovery code ──────────────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_user_a_cannot_consume_user_b_recovery_code(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = TestUser::new();
    let user_b = TestUser::new();
    cleanup_test_user(&pool, &user_a.email).await;
    cleanup_test_user(&pool, &user_b.email).await;

    let (token_a, org_a) = create_authenticated_user_with_org(&app, &user_a, "a").await;
    let (token_b, org_b) = create_authenticated_user_with_org(&app, &user_b, "b").await;
    let id_a = user_id_for(&pool, &user_a.email).await;
    let id_b = user_id_for(&pool, &user_b.email).await;

    // Both users enrol in MFA and receive their own single-use codes.
    enroll_user_2fa(&pool, id_a).await;
    enroll_user_2fa(&pool, id_b).await;
    let _codes_a = issue_recovery_codes(&pool, id_a, 10).await;
    let codes_b = issue_recovery_codes(&pool, id_b, 10).await;
    assert_eq!(unused_code_count(&pool, id_b).await, 10);

    // ── The attack: A presents B's plaintext recovery code on A's own session.
    // With per-user scoping intact this matches no row A may see, so it is
    // rejected exactly like any other wrong code.
    let victim_code = &codes_b[0];
    let (status, body) = post_recovery_verify(&app, &token_a, org_a, victim_code).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "user A presenting user B's recovery code must be rejected (400), \
         not silently consume B's code; body: {body}"
    );

    // ── Isolation invariant: B's codes are all untouched. If the WHERE user_id
    // filter were dropped, A's request above would have consumed B's code and
    // this count would be 9.
    assert_eq!(
        unused_code_count(&pool, id_b).await,
        10,
        "user A's attempt must NOT consume any of user B's recovery codes"
    );

    // ── And the code is genuinely valid: B can still consume it themselves,
    // ruling out a false pass where the code was simply malformed.
    let (b_status, b_body) = post_recovery_verify(&app, &token_b, org_b, victim_code).await;
    assert_eq!(
        b_status,
        StatusCode::OK,
        "user B must still consume their own (untouched) code; body: {b_body}"
    );
    assert_eq!(
        b_body["codesRemaining"],
        json!(9),
        "after B consumes one of their own codes, 9 remain"
    );

    cleanup_test_user(&pool, &user_a.email).await;
    cleanup_test_user(&pool, &user_b.email).await;
}

// ─── A's own consumption never touches B's codes ─────────────────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_user_a_consumption_does_not_affect_user_b_codes(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = TestUser::new();
    let user_b = TestUser::new();
    cleanup_test_user(&pool, &user_a.email).await;
    cleanup_test_user(&pool, &user_b.email).await;

    let (token_a, org_a) = create_authenticated_user_with_org(&app, &user_a, "a").await;
    let (_token_b, _org_b) = create_authenticated_user_with_org(&app, &user_b, "b").await;
    let id_a = user_id_for(&pool, &user_a.email).await;
    let id_b = user_id_for(&pool, &user_b.email).await;

    enroll_user_2fa(&pool, id_a).await;
    enroll_user_2fa(&pool, id_b).await;
    let codes_a = issue_recovery_codes(&pool, id_a, 10).await;
    let _codes_b = issue_recovery_codes(&pool, id_b, 10).await;

    // A consumes one of A's own codes successfully.
    let (status, body) = post_recovery_verify(&app, &token_a, org_a, &codes_a[0]).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "A's own code must be accepted; body: {body}"
    );
    assert_eq!(body["codesRemaining"], json!(9));

    // A's consumption is fully isolated to A: B still has all 10, and the
    // remaining count A observed (9) reflects A's own rows, never B's.
    assert_eq!(
        unused_code_count(&pool, id_a).await,
        9,
        "A consumed exactly one of A's own codes"
    );
    assert_eq!(
        unused_code_count(&pool, id_b).await,
        10,
        "B's codes are untouched by A's activity"
    );

    cleanup_test_user(&pool, &user_a.email).await;
    cleanup_test_user(&pool, &user_b.email).await;
}
