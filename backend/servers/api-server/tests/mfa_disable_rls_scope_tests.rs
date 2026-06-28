//! RLS-scope regression test for `POST /api/v1/auth/mfa/disable` → `disable_mfa`
//! (PR #1292 / PAP-168, part of the PAP-150 RLS-handler burndown).
//!
//! # Why this test exists
//!
//! PR #1292 converted the `disable_mfa` handler off the raw `&state.db` pool and
//! onto the request's `RlsConnection`:
//!
//!   * the unused-recovery-code lookup now runs on `&mut **rls.conn()`, and
//!   * the atomic disable transaction (`UPDATE mfa_recovery_codes … WHERE
//!     user_id = $1` + `user_2fa` disable) is opened on that same RLS
//!     connection (`(&mut **rls.conn()).begin()`), so `app.current_user_id` is
//!     set for its whole duration.
//!
//! That change shipped without a regression test (the test-gap this file
//! closes). The user-self `disable_mfa` endpoint is distinct from the admin
//! `POST /api/v1/admin/mfa/disable` handler exercised by
//! `admin_mfa_disable_tests.rs`, and is NOT covered by
//! `mfa_recovery_cross_user_idor_tests.rs` (which exercises
//! `verify_recovery_code`). Nothing else at the top level of `tests/` asserts
//! that disabling MFA only touches the caller's own recovery codes.
//!
//! # What it proves (IG3)
//!
//!   **User A disabling their own MFA invalidates ONLY user A's recovery codes;
//!   user B's codes are left fully intact.**
//!
//! This is the cross-user isolation the `WHERE user_id = $1` predicate +
//! `app.current_user_id`-scoped RLS connection provide. The test is constructed
//! to FAIL if the scoping regressed — e.g. if the `UPDATE` were ever to drop the
//! `WHERE user_id` predicate, or if the invalidation moved back onto the raw
//! pool such that the self-policy on `mfa_recovery_codes` (migration 00149)
//! could no longer enforce on a connection whose `app.current_user_id` GUC is
//! set. Under either regression, B's codes would be wiped alongside A's and the
//! `id_b` assertion below would observe 0 unused codes instead of the required
//! full set.
//!
//! # Wiring notes
//!
//! Mirrors `mfa_recovery_cross_user_idor_tests.rs`:
//!   * `#[sqlx::test]` runs against a migrated throwaway DB whose pool is not
//!     itself subject to RLS enforcement, so the isolation under test is carried
//!     by the handler's connection scoping, not by the test harness.
//!   * `TestApp` mounts the full router with no host-tenant middleware, so the
//!     MFA handlers' `RlsConnection` resolves the tenant from `X-Tenant-ID` + an
//!     active `organization_members` row. Every request carries `.tenant(org)`.
//!   * 2FA enrolment + recovery codes are seeded directly in the DB so the test
//!     exercises only the endpoint under audit and does not depend on TOTP
//!     timing / encryption config.

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use api_server::services::TotpService;
use common::{cleanup_test_user, create_authenticated_user_with_org, TestApp, TestUser};

// ─── helpers (kept local; mirror the recovery IDOR test) ─────────────────────

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("resolve user id")
}

/// Enable 2FA for `user_id` with an encrypted TOTP secret (the state
/// `disable_mfa` requires: `user_2fa.enabled = true`).
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

/// Insert `n` single-use recovery codes for `user_id`; return the plaintext set.
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

async fn mfa_enabled(pool: &PgPool, user_id: Uuid) -> Option<bool> {
    sqlx::query_scalar::<_, bool>("SELECT enabled FROM user_2fa WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .expect("read user_2fa.enabled")
}

async fn post_disable(app: &TestApp, token: &str, org_id: Uuid, code: &str) -> (StatusCode, Value) {
    let req = app
        .post("/api/v1/auth/mfa/disable")
        .bearer(token)
        .tenant(org_id)
        .json(json!({ "code": code }))
        .build();
    let resp = app.execute(req).await;
    (resp.status, resp.json_value())
}

// ─── A disabling A's MFA does not touch B's recovery codes ───────────────────

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_disable_mfa_invalidates_only_callers_recovery_codes(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = TestUser::new();
    let user_b = TestUser::new();
    cleanup_test_user(&pool, &user_a.email).await;
    cleanup_test_user(&pool, &user_b.email).await;

    let (token_a, org_a) = create_authenticated_user_with_org(&app, &user_a, "a").await;
    let (_token_b, _org_b) = create_authenticated_user_with_org(&app, &user_b, "b").await;
    let id_a = user_id_for(&pool, &user_a.email).await;
    let id_b = user_id_for(&pool, &user_b.email).await;

    // Both users enrol in MFA and receive their own single-use recovery codes.
    enroll_user_2fa(&pool, id_a).await;
    enroll_user_2fa(&pool, id_b).await;
    let codes_a = issue_recovery_codes(&pool, id_a, 10).await;
    let _codes_b = issue_recovery_codes(&pool, id_b, 10).await;
    assert_eq!(unused_code_count(&pool, id_a).await, 10);
    assert_eq!(unused_code_count(&pool, id_b).await, 10);

    // ── A disables their own MFA, confirming with one of A's own recovery codes.
    // disable_mfa loads unused codes on the RLS connection (#1292), matches the
    // submitted code, then in one RLS-scoped transaction marks ALL of A's unused
    // codes used and flips user_2fa.enabled = false.
    let (status, body) = post_disable(&app, &token_a, org_a, &codes_a[0]).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "A must be able to disable their own MFA with their own recovery code; body: {body}"
    );

    // ── A's own MFA is off and all of A's recovery codes are invalidated.
    assert_eq!(
        mfa_enabled(&pool, id_a).await,
        Some(false),
        "A's MFA must be disabled after the call"
    );
    assert_eq!(
        unused_code_count(&pool, id_a).await,
        0,
        "disabling MFA must invalidate all of the caller's unused recovery codes"
    );

    // ── Isolation invariant (the #1292 regression guard): B is untouched. If the
    // UPDATE's `WHERE user_id = $1` scoping regressed, or the invalidation ran
    // outside the per-user RLS context, B's codes would have been wiped too and
    // this count would be 0.
    assert_eq!(
        unused_code_count(&pool, id_b).await,
        10,
        "user A disabling MFA must NOT invalidate any of user B's recovery codes"
    );
    assert_eq!(
        mfa_enabled(&pool, id_b).await,
        Some(true),
        "user A disabling MFA must NOT disable user B's MFA"
    );

    cleanup_test_user(&pool, &user_a.email).await;
    cleanup_test_user(&pool, &user_b.email).await;
}
