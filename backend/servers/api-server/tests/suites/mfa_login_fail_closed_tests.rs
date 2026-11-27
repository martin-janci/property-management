//! Login-time 2FA enforcement regression suite (PR #3848, code-review finding
//! `code-review-api-handlers-auth-2fa-flag-fail-false`).
//!
//! PR #3848 fixed a fail-OPEN authentication hole: the login handler loaded the
//! user's 2FA record with `if let Ok(Some(rec)) = lookup`, which silently
//! dropped the `Err` arm, so a 2FA-enrolled account was treated as "2FA
//! disabled" and could authenticate without a second factor. The fix routes the
//! lookup through `mfa_gate_for`, which maps `Err` -> `MfaGate::LookupFailed`
//! and makes the handler fail closed (500 `MFA_CHECK_ERROR`).
//!
//! The fix shipped with inline *unit* coverage of the pure `mfa_gate_for`
//! helper (`mfa_gate_tests` in `src/routes/auth/mod.rs`), but no dedicated
//! end-to-end test exercising the login endpoint itself. This suite closes that
//! gap: it drives the real `/api/v1/auth/login` router against a seeded
//! database and locks the user-facing invariant that the fail-open bug
//! violated — **an enrolled user is never issued tokens without presenting a
//! second factor** — plus the complementary guard that a *disabled* 2FA record
//! must not over-gate a normal login.
//!
//! The `Err`-lookup -> `LookupFailed` -> 500 mapping cannot be reproduced
//! through the public HTTP API (the real repository does not error on demand);
//! that specific arm stays covered by the inline `mfa_gate_tests` unit module.

use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{verify_user_email, TestApp, TestUser};

/// Look up a registered user's id by email.
async fn user_id_by_email(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("registered user must exist")
}

/// Give `user_id` a 2FA row with the requested `enabled` flag.
async fn seed_2fa(pool: &PgPool, user_id: Uuid, enabled: bool) {
    sqlx::query(
        r#"INSERT INTO user_2fa (user_id, secret, enabled, enabled_at)
           VALUES ($1, 'seed-secret', $2, CASE WHEN $2 THEN NOW() ELSE NULL END)"#,
    )
    .bind(user_id)
    .bind(enabled)
    .execute(pool)
    .await
    .expect("seed 2fa row");
}

/// Register + verify a user, returning its id. Mirrors the canonical
/// register-then-verify flow used across the auth suites.
async fn register_verified_user(app: &TestApp, pool: &PgPool, user: &TestUser) -> Uuid {
    let reg = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    app.execute(reg).await;
    verify_user_email(pool, &user.email).await;
    user_id_by_email(pool, &user.email).await
}

/// Core no-bypass invariant: an enrolled (enabled) 2FA user who logs in WITHOUT
/// a `twoFactorCode` must be challenged (`mfaRequired: true`) and must NOT
/// receive usable tokens. Pre-fix, a dropped lookup `Err` could collapse this
/// account to "2FA disabled" and hand it an access token outright.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn enrolled_user_without_code_is_challenged_not_logged_in(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let user_id = register_verified_user(&app, &pool, &user).await;
    seed_2fa(&pool, user_id, true).await;

    let login = app
        .post("/api/v1/auth/login")
        .json(user.login_body())
        .build();
    let response = app.execute(login).await;

    // Challenge response is a 200 with the MFA-required marker, never a
    // token-bearing success.
    response.assert_status(axum::http::StatusCode::OK);
    let json = response.json_value();

    assert_eq!(
        json["mfaRequired"].as_bool(),
        Some(true),
        "enrolled user must be flagged mfaRequired, got: {json}"
    );
    assert_eq!(
        json["accessToken"].as_str(),
        Some(""),
        "enrolled user must NOT receive an access token before the 2nd factor, got: {json}"
    );
    assert_eq!(
        json["refreshToken"].as_str(),
        Some(""),
        "enrolled user must NOT receive a refresh token before the 2nd factor, got: {json}"
    );
}

/// Complementary guard: a *present-but-disabled* 2FA row must take the gate's
/// `Skip` path — the user logs in normally and receives tokens. This pins the
/// boundary so a future "fail closed harder" change can't accidentally start
/// gating accounts that never enabled a second factor.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disabled_2fa_record_does_not_gate_login(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let user_id = register_verified_user(&app, &pool, &user).await;
    seed_2fa(&pool, user_id, false).await;

    let login = app
        .post("/api/v1/auth/login")
        .json(user.login_body())
        .build();
    let response = app.execute(login).await;

    response.assert_status(axum::http::StatusCode::OK);
    let json = response.json_value();

    // A disabled record is not a challenge: real tokens are issued and the
    // mfaRequired marker is absent (serde skips it when `None`).
    assert!(
        json.get("mfaRequired").is_none(),
        "disabled 2FA must not raise an MFA challenge, got: {json}"
    );
    assert!(
        !json["accessToken"].as_str().unwrap_or("").is_empty(),
        "disabled 2FA user must receive a real access token, got: {json}"
    );
    assert_eq!(json["tokenType"].as_str(), Some("Bearer"), "got: {json}");
}
