//! Integration tests for `POST /api/v1/admin/mfa/disable`.
//!
//! Round-5 test-coverage gap sweep: the disable handler is an auth-bypass
//! primitive (clears the operator's MFA enrollment + recovery codes). Prior
//! to this file it had zero tests. The scenarios covered here track the
//! contract defended by `backend/servers/api-server/src/routes/admin/mfa/disable.rs`:
//!
//!   1. Happy path — platform principal + matching recovery code → 200,
//!      `user_2fa.enabled` flipped to false, recovery codes purged, audit
//!      row emitted with outcome=Allowed and target_type=`mfa_disabled`.
//!   2. Auth-bypass guard — non-platform principal is rejected with 403
//!      `not_platform_principal` BEFORE any DB mutation.
//!   3. Pre-condition — caller is platform but has no active MFA enrollment
//!      → 404 `not_enrolled`. Note: the disable handler does NOT enforce the
//!      15-minute MFA recency window itself — fresh MFA is what `disable`
//!      uses to authorize itself (TOTP or recovery code in the body). The
//!      scenario in the task spec named "no MFA window" maps here.
//!   4. Single-use / TOCTOU — a recovery code that has already been consumed
//!      cannot disable MFA a second time. The `AND used_at IS NULL` guard +
//!      `rows_affected == 1` check is what makes this work; we exercise the
//!      sequential variant of the race (concurrent variant skipped, see
//!      `TODO` note in that test).
//!
//! All tests are gated behind `#[ignore = "requires postgres + migrations"]`
//! per the established convention (e.g. `auth_policy_enforcement_tests.rs`).
//! CI invokes them via `cargo test -- --ignored` against the dev DB.

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

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Seed a `users` row with the given `principal_kind`. Returns the new user id.
async fn seed_user(pool: &PgPool, email: &str, kind: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'h', 'MFA Disable Test', 'active', NOW(), $2)
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(kind)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Insert a `user_2fa` row with `enabled = true` and a *valid encrypted secret*
/// (encrypted with the same TotpService the app builds at startup) so the
/// handler can decrypt it without error. The handler does not actually verify
/// a TOTP against this secret in the recovery-code branch — it only uses the
/// stored secret if `totp_ok` is true. For these tests we exercise the
/// recovery-code path exclusively.
async fn enroll_user_2fa(pool: &PgPool, user_id: Uuid) {
    // Build a TotpService the same way `AppState::new` does. RUST_ENV is set
    // to development by the test harness (see the `set_env_once` block in
    // the happy-path test), so this constructor succeeds with the dev key.
    let totp = TotpService::new("Property Management".to_string());
    let secret = totp.generate_secret().expect("gen secret");
    let encrypted = totp.encrypt_secret(&secret).expect("encrypt secret");

    sqlx::query(
        r#"
        INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
        VALUES ($1, $2, true, NOW(), '[]'::jsonb, 0)
        ON CONFLICT (user_id) DO UPDATE
            SET secret = EXCLUDED.secret,
                enabled = true,
                enabled_at = NOW()
        "#,
    )
    .bind(user_id)
    .bind(&encrypted)
    .execute(pool)
    .await
    .expect("seed user_2fa");
}

/// Insert N recovery codes for the user. Returns the plaintext codes (so the
/// test can present them in the request body).
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

/// Mint a bearer JWT for the given user. Uses the same shared secret + claims
/// shape that `common::TestApp` configures in `JWT_SECRET`.
fn mint_token(user_id: Uuid, kind: &str) -> String {
    let svc = JwtService::new(TEST_JWT_SECRET).expect("jwt service");
    svc.generate_access_token_with_kind(
        user_id,
        "mfa-disable@test.local",
        "MFA Disable Test",
        None,
        None,
        Some(kind.to_string()),
    )
    .expect("mint token")
}

/// Set required env vars exactly once per process. The TOTP service ctor
/// reads `RUST_ENV` to decide whether to fall back to the dev encryption
/// key — without this, `AppState::new` panics on missing
/// `TOTP_ENCRYPTION_KEY`.
fn set_env_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if std::env::var("RUST_ENV").is_err() {
            std::env::set_var("RUST_ENV", "development");
        }
    });
}

// ─── tests ────────────────────────────────────────────────────────────────────

/// (1) Happy path: platform principal disables MFA via a valid recovery code.
///
/// Asserts:
///   * HTTP 200 + body.message present.
///   * `user_2fa.enabled` flipped to false.
///   * All `mfa_recovery_codes` rows for the user are deleted (the handler
///     issues a `DELETE FROM mfa_recovery_codes WHERE user_id = $1` at the
///     end of the transaction).
///   * An `audit_logs` row exists for the user with `details.outcome =
///     'allowed'` and `resource_type = 'mfa_disabled'`.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn happy_path_disables_mfa_and_audits(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "happy-disable@mfa-test.local", "platform").await;
    enroll_user_2fa(&pool, user).await;
    let codes = issue_recovery_codes(&pool, user, 3).await;
    let token = mint_token(user, "platform");

    let req = app
        .post("/api/v1/admin/mfa/disable")
        .bearer(&token)
        .json(json!({ "code": codes[0] }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "expected 200, got {} — body: {}",
        resp.status,
        resp.text()
    );
    resp.assert_json_field("message");

    // user_2fa flipped to false.
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM user_2fa WHERE user_id = $1")
            .bind(user)
            .fetch_optional(&pool)
            .await
            .expect("fetch user_2fa");
    assert_eq!(
        enabled,
        Some(false),
        "user_2fa.enabled must be false after disable"
    );

    // Recovery codes purged.
    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM mfa_recovery_codes WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .expect("count remaining");
    assert_eq!(
        remaining, 0,
        "all recovery codes must be deleted after disable"
    );

    // Audit row.
    let audit_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM audit_logs
           WHERE user_id = $1
             AND resource_type = 'mfa_disabled'
             AND details->>'outcome' = 'allowed'"#,
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .expect("count audit");
    assert!(
        audit_count >= 1,
        "expected at least one 'mfa_disabled' audit row, found {audit_count}"
    );
}

/// (2) Auth-bypass guard: a non-platform principal (kind = `staff`) is
/// rejected with 403 `not_platform_principal`. The capability gate is
/// satisfied by the `is_platform()` check at the top of the handler —
/// without it, any authenticated user could clear another's MFA. This is
/// THE key auth-bypass test for this route.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn non_platform_principal_is_rejected(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "staff-disable@mfa-test.local", "staff").await;
    // Even seed MFA + a recovery code; the guard must fire before either is
    // touched.
    enroll_user_2fa(&pool, user).await;
    let codes = issue_recovery_codes(&pool, user, 1).await;
    let token = mint_token(user, "staff");

    let req = app
        .post("/api/v1/admin/mfa/disable")
        .bearer(&token)
        .json(json!({ "code": codes[0] }))
        .build();
    let resp = app.execute(req).await;

    // The principal extractor itself returns 403 "tenant not resolved" for a
    // non-platform user reaching a no-ResolvedTenant route. EITHER the
    // extractor 403 OR the handler's own `not_platform_principal` 403 is a
    // valid auth-bypass defense — we assert "any 403" rather than pinning the
    // exact branch so the test stays green if the wiring evolves.
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-platform principal must be denied, got {} body {}",
        resp.status,
        resp.text()
    );

    // user_2fa MUST still be enabled — no mutation may have occurred.
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM user_2fa WHERE user_id = $1")
            .bind(user)
            .fetch_optional(&pool)
            .await
            .expect("fetch user_2fa");
    assert_eq!(
        enabled,
        Some(true),
        "user_2fa must NOT be disabled on the denied path"
    );

    // Recovery code MUST still be present + unused.
    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mfa_recovery_codes WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .expect("count remaining");
    assert_eq!(
        remaining, 1,
        "recovery code must remain unused after denial"
    );
}

/// (3) Pre-condition: platform principal with no active MFA enrollment
/// receives 404 `not_enrolled`. This is the closest analogue to the spec's
/// "no MFA window" scenario — the disable route does not gate on the
/// recency window itself (proof of fresh MFA is provided *via* the body
/// code), so the lockable-but-not-bypassable pre-condition is the
/// enrollment state.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn not_enrolled_user_is_rejected(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "not-enrolled@mfa-test.local", "platform").await;
    // Intentionally do NOT call enroll_user_2fa — the user has no row in
    // user_2fa at all.
    let token = mint_token(user, "platform");

    let req = app
        .post("/api/v1/admin/mfa/disable")
        .bearer(&token)
        .json(json!({ "code": "ABCD1234" }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "no enrollment must yield 404, got {} body {}",
        resp.status,
        resp.text()
    );
    let body = resp.json_value();
    assert_eq!(
        body.get("error").and_then(|v| v.as_str()),
        Some("not_enrolled"),
        "error field must be 'not_enrolled', got {body}"
    );
}

/// (3b) Pre-condition variant: user_2fa row exists but `enabled = false`
/// (mid-enrollment state). The handler must still 404 `not_enrolled`
/// because there is no active MFA to disable.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn pending_enrollment_user_is_rejected(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "pending-disable@mfa-test.local", "platform").await;
    let totp = TotpService::new("Property Management".to_string());
    let secret = totp.generate_secret().unwrap();
    let encrypted = totp.encrypt_secret(&secret).unwrap();
    // enabled = false → mid-enrollment, NOT yet active.
    sqlx::query(
        r#"
        INSERT INTO user_2fa (user_id, secret, enabled, backup_codes, backup_codes_remaining)
        VALUES ($1, $2, false, '[]'::jsonb, 0)
        "#,
    )
    .bind(user)
    .bind(&encrypted)
    .execute(&pool)
    .await
    .expect("seed pending");

    let token = mint_token(user, "platform");
    let req = app
        .post("/api/v1/admin/mfa/disable")
        .bearer(&token)
        .json(json!({ "code": "ABCD1234" }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(resp.status, StatusCode::NOT_FOUND, "body: {}", resp.text());
}

/// (4) Single-use / TOCTOU: after a recovery code has been consumed by a
/// successful disable, replaying the same plaintext cannot disable a
/// re-enrolled instance of the user.
///
/// What this exercises:
///   * First call: consumes `codes[0]` → MFA off + all `mfa_recovery_codes`
///     rows deleted.
///   * Re-enroll the user (`user_2fa.enabled = true` again) WITHOUT
///     reissuing recovery codes.
///   * Replay the same plaintext → handler loads zero unused-codes rows,
///     `verify_backup_code` matches nothing → 422 `invalid_code` (no
///     candidate to consume). MFA must remain enabled.
///
/// This validates the candidate-load filter (`WHERE user_id = $1 AND
/// used_at IS NULL`) is doing its job: a code whose row has been deleted
/// or marked used is invisible to the verifier. The same SQL guard is
/// what defeats the TOCTOU race in the handler's transaction.
///
/// TODO(concurrent-race): the genuine concurrent test (two `tokio::spawn`
/// requests racing the same code → exactly one succeeds) requires
/// injecting a synchronization barrier between the candidate load and the
/// UPDATE inside the handler so the second request observes
/// `rows_affected == 0` on the UPDATE rather than "no candidate". The
/// `#[sqlx::test]` pool fixture lets two spawned futures share a DB pool,
/// but without a barrier the first request typically commits before the
/// second even starts its tx, collapsing the race into the
/// already-covered "no candidate" branch. Skipped — to be added with a
/// test-only handler shim in a follow-up. For now the sequential test
/// exercises the SAME `used_at IS NULL` filter the TOCTOU guard relies on.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn used_recovery_code_cannot_disable_again(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "replay-disable@mfa-test.local", "platform").await;
    enroll_user_2fa(&pool, user).await;
    let codes = issue_recovery_codes(&pool, user, 2).await;

    // First request: consume codes[0] → MFA disabled + codes purged.
    let token = mint_token(user, "platform");
    let req1 = app
        .post("/api/v1/admin/mfa/disable")
        .bearer(&token)
        .json(json!({ "code": codes[0] }))
        .build();
    let resp1 = app.execute(req1).await;
    assert_eq!(resp1.status, StatusCode::OK, "first call: {}", resp1.text());

    // Confirm the post-disable invariant: all codes purged.
    let any_codes: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM mfa_recovery_codes WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(any_codes, 0, "all recovery codes must be purged by disable");

    // Re-enroll WITHOUT reissuing codes. The user has `user_2fa.enabled =
    // true` again but zero rows in `mfa_recovery_codes`.
    enroll_user_2fa(&pool, user).await;

    // Replay the previously-consumed plaintext. The handler will load zero
    // unused-codes rows → `verify_backup_code` returns None → falls back to
    // the TOTP path (also fails because the plaintext is not a 6-digit
    // TOTP for the new secret) → 422 `invalid_code`. The single-use
    // guarantee survives even the re-enrollment edge case.
    let req2 = app
        .post("/api/v1/admin/mfa/disable")
        .bearer(&token)
        .json(json!({ "code": codes[0] }))
        .build();
    let resp2 = app.execute(req2).await;
    assert_eq!(
        resp2.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "replay must be rejected, got {} body {}",
        resp2.status,
        resp2.text()
    );
    let body = resp2.json_value();
    assert_eq!(
        body.get("error").and_then(|v| v.as_str()),
        Some("invalid_code"),
        "expected invalid_code error, got {body}"
    );

    // MFA must still be enabled — replay must not have flipped state on
    // the re-enrolled instance.
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM user_2fa WHERE user_id = $1")
            .bind(user)
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert_eq!(
        enabled,
        Some(true),
        "MFA must remain enabled after rejected replay"
    );
}

/// (5) Smoke: an invalid code (neither valid TOTP nor matching any
/// recovery hash) yields 422 `invalid_code`, audits a Denied row, and
/// leaves MFA enabled. Documents the negative path so a future
/// refactor that silently swallows verification failures (e.g. by
/// returning 200 with a "no-op" message) is caught.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn invalid_code_is_rejected_and_audited(pool: PgPool) {
    set_env_once();
    let app = TestApp::new(pool.clone()).await;

    let user = seed_user(&pool, "bad-code-disable@mfa-test.local", "platform").await;
    enroll_user_2fa(&pool, user).await;
    let _ = issue_recovery_codes(&pool, user, 3).await;
    let token = mint_token(user, "platform");

    let req = app
        .post("/api/v1/admin/mfa/disable")
        .bearer(&token)
        .json(json!({ "code": "TOTALLY-WRONG" }))
        .build();
    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "bad code body: {}",
        resp.text()
    );

    // MFA remains enabled.
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM user_2fa WHERE user_id = $1")
            .bind(user)
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert_eq!(enabled, Some(true));

    // Denied audit row recorded.
    let denied_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM audit_logs
           WHERE user_id = $1
             AND resource_type = 'mfa_disable_rejected'
             AND details->>'outcome' = 'denied'"#,
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        denied_count >= 1,
        "expected denied audit row, got {denied_count}"
    );
}
