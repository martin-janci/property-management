//! Integration tests for `POST /api/v1/admin/mfa/recovery/use`.
//!
//! Round-5 test-coverage gap sweep: the recovery handler is the lockout
//! escape hatch — a successful call opens the 15-minute MFA recency window
//! for the calling platform principal, which then unlocks the
//! `RequireCapability` gate. Prior to this file it had zero tests. The
//! scenarios mirror the contract enforced by
//! `backend/servers/api-server/src/routes/admin/mfa/recovery.rs`:
//!
//!   1. Happy path — valid unused code consumed → 200, `used_at` is set,
//!      `two_factor_auth_verifications` row inserted, codes_remaining
//!      decremented, audit row with outcome=Allowed +
//!      target_type=`mfa_recovery_used`.
//!   2. Invalid code — 422 `invalid_recovery_code`, no state change, audit
//!      row with outcome=Denied + target_type=`mfa_recovery_invalid`.
//!   3. Replay — a previously-used code (`used_at IS NOT NULL`) cannot be
//!      used a second time, even with the same plaintext: the `used_at IS
//!      NULL` filter hides it.
//!   4. Race / TOCTOU — two concurrent recovery POSTs with the same code:
//!      sequential analogue covered (the second call matches no candidate
//!      row); true concurrent variant skipped with a TODO note (see
//!      `concurrent_double_consume_skipped`).
//!
//! All tests are gated `#[ignore = "requires postgres + migrations"]` per
//! the established convention.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use api_server::services::{JwtService, TotpService};
use common::TestApp;

const TEST_JWT_SECRET: &str =
    "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

// ─── helpers (kept in-file so each test binary is self-contained) ─────────────

async fn seed_user(pool: &PgPool, email: &str, kind: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'h', 'MFA Recovery Test', 'active', NOW(), $2)
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(kind)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

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

async fn issue_recovery_codes(pool: &PgPool, user_id: Uuid, n: usize) -> Vec<String> {
    let totp = TotpService::new("Property Management".to_string());
    let (plain, hashed) = totp.generate_backup_codes().expect("gen backup");
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

fn mint_token(user_id: Uuid, kind: &str) -> String {
    let svc = JwtService::new(TEST_JWT_SECRET).expect("jwt service");
    svc.generate_access_token_with_kind(
        user_id,
        "mfa-recovery@test.local",
        "MFA Recovery Test",
        None,
        None,
        Some(kind.to_string()),
    )
    .expect("mint token")
}

fn set_env_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if std::env::var("RUST_ENV").is_err() {
            std::env::set_var("RUST_ENV", "development");
        }
    });
}

// ─── tests ────────────────────────────────────────────────────────────────────

/// (1) Happy path: a fresh, unused recovery code is accepted, marked used,
/// and a `two_factor_auth_verifications` row is inserted to open the
/// 15-minute recency window.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn happy_path_consumes_code_and_opens_window(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "happy-recovery@mfa-test.local", "platform").await;
    enroll_user_2fa(&pool, user).await;
    let codes = issue_recovery_codes(&pool, user, 5).await;
    let token = mint_token(user, "platform");

    let req = app
        .post("/api/v1/admin/mfa/recovery/use")
        .bearer(&token)
        .json(json!({ "code": codes[0] }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "expected 200, got {} body {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert!(body.get("message").is_some(), "body has message: {body}");
    assert_eq!(
        body.get("codes_remaining").and_then(|v| v.as_i64()),
        Some(4),
        "codes_remaining must report post-consume count, got {body}"
    );

    // Exactly one row has used_at set.
    let used: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mfa_recovery_codes WHERE user_id = $1 AND used_at IS NOT NULL",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(used, 1, "exactly one code must be marked used");

    // Verification row inserted (opens the 15-min window).
    let verifications: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM two_factor_auth_verifications WHERE user_id = $1 AND method = 'recovery_code'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        verifications, 1,
        "exactly one verification row must be inserted"
    );

    // Audit row with outcome=allowed.
    let audit_allowed: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM audit_logs
           WHERE user_id = $1
             AND resource_type = 'mfa_recovery_used'
             AND details->>'outcome' = 'allowed'"#,
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        audit_allowed >= 1,
        "expected at least one allowed audit row, got {audit_allowed}"
    );
}

/// (2) Invalid code: presenting a code that matches no stored hash yields
/// 422 `invalid_recovery_code`, leaves all codes unused, and emits a
/// Denied audit row tagged `mfa_recovery_invalid`.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn invalid_code_is_rejected_and_audited(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "bad-recovery@mfa-test.local", "platform").await;
    enroll_user_2fa(&pool, user).await;
    let _codes = issue_recovery_codes(&pool, user, 3).await;
    let token = mint_token(user, "platform");

    let req = app
        .post("/api/v1/admin/mfa/recovery/use")
        .bearer(&token)
        .json(json!({ "code": "NEVERMATCH" }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "bad code body: {}",
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(
        body.get("error").and_then(|v| v.as_str()),
        Some("invalid_recovery_code")
    );

    // No code consumed.
    let used: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mfa_recovery_codes WHERE user_id = $1 AND used_at IS NOT NULL",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(used, 0, "no code may be marked used on failed verify");

    // No window opened.
    let verifications: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM two_factor_auth_verifications WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        verifications, 0,
        "no verification row may be inserted on failed verify"
    );

    // Audit row with outcome=denied.
    let audit_denied: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM audit_logs
           WHERE user_id = $1
             AND resource_type = 'mfa_recovery_invalid'
             AND details->>'outcome' = 'denied'"#,
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        audit_denied >= 1,
        "expected denied audit row, got {audit_denied}"
    );
}

/// (3) Replay: a code that was already consumed (`used_at IS NOT NULL`)
/// cannot be replayed. The handler's `WHERE used_at IS NULL` filter on
/// the candidate-load query hides used rows entirely → the second call
/// behaves identically to "invalid code", returning 422
/// `invalid_recovery_code`.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn replay_of_used_code_is_rejected(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "replay-recovery@mfa-test.local", "platform").await;
    enroll_user_2fa(&pool, user).await;
    let codes = issue_recovery_codes(&pool, user, 2).await;
    let token = mint_token(user, "platform");

    // First call consumes codes[0].
    let req1 = app
        .post("/api/v1/admin/mfa/recovery/use")
        .bearer(&token)
        .json(json!({ "code": codes[0] }))
        .build();
    let resp1 = app.execute(req1).await;
    assert_eq!(resp1.status, StatusCode::OK, "first: {}", resp1.text());

    // Second call replays the same plaintext.
    let req2 = app
        .post("/api/v1/admin/mfa/recovery/use")
        .bearer(&token)
        .json(json!({ "code": codes[0] }))
        .build();
    let resp2 = app.execute(req2).await;
    assert_eq!(
        resp2.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "replay must 422, got {} body {}",
        resp2.status,
        resp2.text()
    );

    // Still exactly one consumed row.
    let used: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mfa_recovery_codes WHERE user_id = $1 AND used_at IS NOT NULL",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(used, 1, "replay must not bump the used count");

    // Still exactly one verification row (no second window opened).
    let verifications: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM two_factor_auth_verifications WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(verifications, 1, "replay must not open a second MFA window");
}

/// (3b) Not-enrolled: a platform principal with no `user_2fa.enabled =
/// true` row is rejected with 412 `not_enrolled`. The recovery handler
/// requires an active enrollment (recovery codes are only meaningful
/// once MFA is on).
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn not_enrolled_user_is_rejected(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "not-enrolled-recovery@mfa-test.local", "platform").await;
    // Intentionally no enrollment.
    let token = mint_token(user, "platform");

    let req = app
        .post("/api/v1/admin/mfa/recovery/use")
        .bearer(&token)
        .json(json!({ "code": "ANYTHING" }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::PRECONDITION_FAILED,
        "got {} body {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(
        body.get("error").and_then(|v| v.as_str()),
        Some("not_enrolled")
    );
}

/// (3c) Auth-bypass guard: a non-platform principal (kind = `staff`)
/// MUST NOT be able to use the recovery endpoint. The handler's
/// `is_platform()` check is the auth-bypass defense; without it any
/// authenticated user could request to consume codes belonging to
/// platform admins.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn non_platform_principal_is_rejected(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "staff-recovery@mfa-test.local", "staff").await;
    enroll_user_2fa(&pool, user).await;
    let codes = issue_recovery_codes(&pool, user, 1).await;
    let token = mint_token(user, "staff");

    let req = app
        .post("/api/v1/admin/mfa/recovery/use")
        .bearer(&token)
        .json(json!({ "code": codes[0] }))
        .build();
    let resp = app.execute(req).await;

    // Either the principal extractor's 403 ("tenant not resolved") or the
    // handler's `not_platform_principal` 403 — both are valid auth-bypass
    // defenses.
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "staff principal must be denied, got {} body {}",
        resp.status,
        resp.text()
    );

    // No code consumed, no window opened.
    let used: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mfa_recovery_codes WHERE user_id = $1 AND used_at IS NOT NULL",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(used, 0);
    let v: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM two_factor_auth_verifications WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(v, 0);
}

/// (4) No codes left: every code has `used_at IS NOT NULL` →
/// 410 Gone `no_recovery_codes`, denied audit row tagged
/// `mfa_recovery_no_codes`. Documents the documented branch in the handler.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn all_codes_exhausted_returns_gone(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "exhausted-recovery@mfa-test.local", "platform").await;
    enroll_user_2fa(&pool, user).await;
    let codes = issue_recovery_codes(&pool, user, 2).await;
    // Mark all as used directly.
    sqlx::query("UPDATE mfa_recovery_codes SET used_at = NOW() WHERE user_id = $1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();

    let token = mint_token(user, "platform");
    let req = app
        .post("/api/v1/admin/mfa/recovery/use")
        .bearer(&token)
        .json(json!({ "code": codes[0] }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(resp.status, StatusCode::GONE, "body: {}", resp.text());
    let body = resp.json_value();
    assert_eq!(
        body.get("error").and_then(|v| v.as_str()),
        Some("no_recovery_codes")
    );

    let audit_denied: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM audit_logs
           WHERE user_id = $1
             AND resource_type = 'mfa_recovery_no_codes'
             AND details->>'outcome' = 'denied'"#,
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(audit_denied >= 1);
}

/// TODO(concurrent-race): a genuine concurrent test — two simultaneous
/// POSTs of the same plaintext code, where exactly one succeeds and the
/// other observes `rows_affected == 0` on the UPDATE and is rejected as
/// `invalid_recovery_code`.
///
/// Why this is skipped:
///   * `#[sqlx::test]` gives each test a private DB, but the *router* in
///     `TestApp::new` is cheap to clone (it's an `Arc`-wrapped axum
///     `Router`). What's harder is forcing both spawns to actually
///     interleave inside the `BEGIN ... UPDATE ... COMMIT` window — without
///     a synchronization primitive injected into the handler, the test is
///     flaky (the first call may complete before the second even hits the
///     row lock, so the second call observes "no candidate" rather than
///     "concurrent UPDATE lost the race"). Both paths return 422
///     `invalid_recovery_code`, so the assertion still holds — but the
///     test no longer differentiates "lost the row-lock race" from "already
///     used" and stops being evidence for the TOCTOU guard specifically.
///   * The sequential test (`replay_of_used_code_is_rejected`) exercises
///     the SAME SQL guard (`AND used_at IS NULL` filter on the candidate
///     load + `rows_affected == 1` on the UPDATE). The TOCTOU-specific
///     path can be added later with a `tokio::sync::Barrier` injected
///     into a test-only handler wrapper.
///
/// Intentionally left as a `#[test]` (not `#[tokio::test]`) that is a no-op
/// — its purpose is to surface in `cargo test` output as a named TODO.
#[test]
#[ignore = "concurrent TOCTOU test not yet implemented — see comment"]
fn concurrent_double_consume_skipped() {
    // No-op: see TODO above. Sequential analogue lives in
    // `replay_of_used_code_is_rejected`.
}
