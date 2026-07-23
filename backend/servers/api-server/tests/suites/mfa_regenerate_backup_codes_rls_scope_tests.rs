//! RLS-scope regression test for
//! `POST /api/v1/auth/mfa/backup-codes/regenerate` → `regenerate_backup_codes`
//! (PR #1292 / PAP-168, part of the PAP-150 RLS-handler burndown).
//!
//! # Why this test exists
//!
//! PR #1292 converted the self-service MFA handlers in
//! `servers/api-server/src/routes/mfa.rs` off the raw `&state.db` pool and onto
//! the request's `RlsConnection`. `regenerate_backup_codes` now:
//!
//!   * loads the caller's `user_2fa` row via `get_by_user_id_rls(&mut
//!     **rls.conn(), user_id)`, and
//!   * writes the fresh hashed backup codes via
//!     `regenerate_backup_codes_rls(&mut **rls.conn(), user_id, …)`, whose
//!     statement is `UPDATE user_2fa SET backup_codes = $2,
//!     backup_codes_remaining = $3 … WHERE user_id = $1` — running on a
//!     connection whose `app.current_user_id` GUC is set for the caller.
//!
//! The whole RLS conversion of mfa.rs shipped without a regression test, and it
//! also landed broken and had to be hotfixed (PR #1287). The sibling
//! `mfa_disable_rls_scope_tests.rs` locks the `disable_mfa` handler and
//! `mfa_recovery_cross_user_idor_tests.rs` locks `verify_recovery_code`, but
//! **nothing asserts the row-isolation of the backup-code regeneration path**.
//! `mfa_e2e_tests.rs::test_mfa_backup_codes_regeneration` only proves the happy
//! path (the caller gets 10 fresh codes back) for a single user — it never
//! checks that a *second* user's codes are left alone.
//!
//! # What it proves (IG3)
//!
//!   **User A regenerating their own backup codes rewrites ONLY user A's
//!   `user_2fa` row; user B's `backup_codes` and `backup_codes_remaining` are
//!   left byte-for-byte intact.**
//!
//! This is the cross-user isolation the `WHERE user_id = $1` predicate + the
//! `app.current_user_id`-scoped RLS connection provide. The test is constructed
//! to FAIL if the scoping regressed — e.g. if the `UPDATE` ever dropped its
//! `WHERE user_id` predicate, or if the write moved back onto the raw
//! `&state.db` pool such that the self-policy on `user_2fa` (migration 00149)
//! could no longer enforce on a connection whose `app.current_user_id` GUC is
//! set. Under either regression, A's regeneration would overwrite B's row too
//! and the `id_b` assertions below would observe A's new 10-code state instead
//! of B's untouched 7-code state.
//!
//! # Wiring notes
//!
//! Mirrors `mfa_disable_rls_scope_tests.rs`:
//!   * `#[sqlx::test]` runs against a migrated throwaway DB whose pool is not
//!     itself subject to RLS enforcement, so the isolation under test is carried
//!     by the handler's connection scoping, not by the test harness.
//!   * `TestApp` mounts the full router with no host-tenant middleware, so the
//!     MFA handlers' `RlsConnection` resolves the tenant from `X-Tenant-ID` + an
//!     active `organization_members` row. Every request carries `.tenant(org)`.
//!   * 2FA enrolment is seeded directly in the DB. The caller (A) is seeded with
//!     a known base32 TOTP secret so the test can compute a valid current code
//!     for the `regenerate` verification step without depending on the setup
//!     flow; the passive user (B) is seeded with a distinctive backup-code set
//!     so any clobber is detectable.

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

use crate::common::{cleanup_test_user, create_authenticated_user_with_org, TestApp, TestUser};
use api_server::services::TotpService;

// ─── helpers (kept local; mirror mfa_disable_rls_scope_tests.rs) ─────────────

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("resolve user id")
}

/// Compute the current TOTP code for a base32-encoded secret, using the same
/// params `TotpService` (and `mfa_e2e_tests.rs::current_totp_code`) use.
fn current_totp_code(secret: &str) -> String {
    let secret_bytes = Secret::Encoded(secret.to_string())
        .to_bytes()
        .expect("valid base32 secret");
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,  // skew
        30, // step
        secret_bytes,
        Some("Property Management".to_string()),
        "test".to_string(),
    )
    .expect("valid TOTP params");
    totp.generate_current().expect("valid system clock")
}

/// Enable 2FA for `user_id` with the given base32 secret (encrypted the same way
/// the app's `TotpService` stores it) and a known backup-code state.
async fn enroll_user_2fa(
    pool: &PgPool,
    user_id: Uuid,
    secret_base32: &str,
    backup_codes: &Value,
    remaining: i32,
) {
    let totp = TotpService::new("Property Management".to_string());
    let encrypted = totp.encrypt_secret(secret_base32).expect("encrypt secret");
    sqlx::query(
        r#"
        INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
        VALUES ($1, $2, true, NOW(), $3, $4)
        ON CONFLICT (user_id) DO UPDATE
          SET enabled = true, enabled_at = NOW(),
              secret = EXCLUDED.secret,
              backup_codes = EXCLUDED.backup_codes,
              backup_codes_remaining = EXCLUDED.backup_codes_remaining
        "#,
    )
    .bind(user_id)
    .bind(&encrypted)
    .bind(backup_codes)
    .bind(remaining)
    .execute(pool)
    .await
    .expect("seed user_2fa");
}

/// Read a user's stored backup codes (jsonb) and remaining count.
async fn backup_state(pool: &PgPool, user_id: Uuid) -> (Value, i32) {
    sqlx::query_as::<_, (Value, i32)>(
        "SELECT backup_codes, backup_codes_remaining FROM user_2fa WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("read user_2fa backup state")
}

async fn post_regenerate(
    app: &TestApp,
    token: &str,
    org_id: Uuid,
    code: &str,
) -> (StatusCode, Value) {
    let req = app
        .post("/api/v1/auth/mfa/backup-codes/regenerate")
        .bearer(token)
        .tenant(org_id)
        .json(json!({ "code": code }))
        .build();
    let resp = app.execute(req).await;
    (resp.status, resp.json_value())
}

// ─── A regenerating A's backup codes does not touch B's row ──────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_regenerate_backup_codes_rewrites_only_callers_row(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = TestUser::new();
    let user_b = TestUser::new();
    cleanup_test_user(&pool, &user_a.email).await;
    cleanup_test_user(&pool, &user_b.email).await;

    let (token_a, org_a) = create_authenticated_user_with_org(&app, &user_a, "a").await;
    let (_token_b, _org_b) = create_authenticated_user_with_org(&app, &user_b, "b").await;
    let id_a = user_id_for(&pool, &user_a.email).await;
    let id_b = user_id_for(&pool, &user_b.email).await;

    // A is enrolled with a known secret so we can produce a valid current TOTP
    // code for the regenerate verification step. A starts with a distinctive
    // 3-code backup set.
    let totp = TotpService::new("Property Management".to_string());
    let secret_a = totp.generate_secret().expect("gen secret A");
    let codes_a_before = json!(["a-old-1", "a-old-2", "a-old-3"]);
    enroll_user_2fa(&pool, id_a, &secret_a, &codes_a_before, 3).await;

    // B is a passive bystander with a DISTINCTIVE 7-code backup set; B never
    // makes a request. B's row must be identical before and after A's call.
    let secret_b = totp.generate_secret().expect("gen secret B");
    let codes_b = json!(["b-1", "b-2", "b-3", "b-4", "b-5", "b-6", "b-7"]);
    enroll_user_2fa(&pool, id_b, &secret_b, &codes_b, 7).await;

    let (b_codes_before, b_remaining_before) = backup_state(&pool, id_b).await;
    assert_eq!(b_remaining_before, 7);
    assert_eq!(b_codes_before, codes_b);

    // ── A regenerates their own backup codes with a valid TOTP code. The
    // handler verifies the code against A's decrypted secret, generates 10 fresh
    // hashed codes, and writes them via `regenerate_backup_codes_rls` on the
    // per-user RLS connection (#1292).
    let (status, body) =
        post_regenerate(&app, &token_a, org_a, &current_totp_code(&secret_a)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "A must be able to regenerate their own backup codes with a valid TOTP code; body: {body}"
    );
    assert_eq!(
        body["backupCodes"].as_array().map(|c| c.len()),
        Some(10),
        "regenerate must return 10 fresh backup codes"
    );

    // ── A's own row is rewritten: 10 fresh codes, remaining = 10.
    let (_a_codes_after, a_remaining_after) = backup_state(&pool, id_a).await;
    assert_eq!(
        a_remaining_after, 10,
        "A's backup_codes_remaining must reflect the freshly generated set"
    );

    // ── Isolation invariant (the #1292 regression guard): B is untouched. If the
    // UPDATE's `WHERE user_id = $1` scoping regressed, or the write ran outside
    // the per-user RLS context, B's row would have been overwritten with A's new
    // 10-code state and these assertions would observe remaining = 10 / A's
    // codes instead of B's original 7-code set.
    let (b_codes_after, b_remaining_after) = backup_state(&pool, id_b).await;
    assert_eq!(
        b_remaining_after, 7,
        "user A regenerating backup codes must NOT change user B's remaining count"
    );
    assert_eq!(
        b_codes_after, codes_b,
        "user A regenerating backup codes must NOT overwrite user B's backup codes"
    );

    cleanup_test_user(&pool, &user_a.email).await;
    cleanup_test_user(&pool, &user_b.email).await;
}
