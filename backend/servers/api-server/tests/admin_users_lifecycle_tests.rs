//! Integration tests for `routes/admin/users_lifecycle.rs` — first-ever
//! coverage for the admin user-lifecycle handlers (Epic 1, Story 1.6).
//!
//! Flagged as TOP priority by the round-5 test coverage sweep: 603 LoC with
//! zero existing tests. We sample the highest-leverage scenarios:
//!
//!   * `POST /users/{id}/suspend` happy path — capability + recent MFA →
//!     200, status flips to `suspended`, refresh tokens revoked, an audit
//!     row is emitted by the `RequireCapability` extractor.
//!   * `POST /users/{id}/suspend` denied — admin without a capability grant
//!     gets 403 with `missing_capability` from the extractor.
//!   * `POST /users/{id}/suspend` already-suspended — the underlying
//!     `UserRepository::suspend` filters on `status='active'` so the route
//!     surfaces 404 `USER_NOT_FOUND`. Documents the current behavior
//!     (handler is NOT idempotent at the OK level, despite the task brief
//!     wording).
//!   * `POST /users/{id}/reactivate` happy path — suspended → active.
//!   * `POST /users/{id}/delete` happy path — soft-delete sets status to
//!     `deleted`.
//!   * `POST /users/{id}/delete` already-deleted — repository filter
//!     `status != 'deleted'` → 404.
//!   * `POST /users/{id}/suspend` self-suspension guard — admin acting on
//!     themselves → 400 `CANNOT_SELF_SUSPEND`.
//!
//! Scenarios skipped:
//!   * `set_principal_kind` — lives in `routes/admin/users.rs`, not in
//!     `users_lifecycle.rs`, so it's out of scope for this file.
//!   * Concurrent-suspend races — would need parallel client setup; the
//!     repository UPDATE is atomic so this would mostly be sqlx-level.
//!
//! Harness: `#[sqlx::test(migrator = "db::MIGRATOR")]` + `TestApp::new(pool)`,
//! mirroring `tests/auth_policy_enforcement_tests.rs` (DB seeding) and
//! `tests/token_scope_tests.rs` (JWT minting flow). The pre-existing
//! `tests/common/mod.rs` `TestApp` builds the full `create_router` so
//! `RequireCapability` and its admin extensions are wired exactly as in
//! production.
//!
//! All tests carry `#[ignore = "requires postgres + migrations"]` to match
//! the codebase convention — run with `cargo test --ignored`.

mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

// ===========================================================================
// Seed helpers
// ===========================================================================

/// Insert a user row directly. Returns the new user id.
async fn seed_user(pool: &PgPool, email: &str, status: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Test User', $2, NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Promote a user to `principal_kind = 'platform'` via the SECURITY DEFINER
/// helper installed by migration 00129. The plain UPDATE path is blocked by
/// a trigger; this function is the *only* sanctioned escalation route.
async fn promote_to_platform(pool: &PgPool, user_id: Uuid) {
    sqlx::query("SELECT set_principal_kind($1, 'platform'::varchar, NULL, 'test seed')")
        .bind(user_id)
        .execute(pool)
        .await
        .expect("set_principal_kind to platform");
}

/// Mark the user as fully MFA-enrolled (`user_2fa.enabled = true`) so the
/// extractor's enrollment gate (step 2.5) passes.
async fn enroll_mfa(pool: &PgPool, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO user_2fa (user_id, secret, enabled, enabled_at)
        VALUES ($1, 'test-totp-secret', true, NOW())
        ON CONFLICT (user_id) DO UPDATE
            SET enabled = true, enabled_at = NOW()
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("enroll mfa");
}

/// Record a recent MFA verification so the `recent_mfa` 15-minute window is
/// satisfied. Without this the extractor returns `mfa_required` (401).
async fn record_recent_mfa(pool: &PgPool, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO two_factor_auth_verifications (user_id, method, verified_at)
        VALUES ($1, 'totp', NOW())
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("record mfa verification");
}

/// Grant the named capability to `user_id`. The `granted_by` differs to
/// satisfy the leak-#21 no-self-grant invariant enforced at the repo layer.
async fn grant_capability(pool: &PgPool, user_id: Uuid, capability: &str) {
    // The `capability_grants.granted_by` column has a NOT NULL FK to users,
    // so we need a second platform user to serve as the granter. A nil/random
    // UUID would violate the FK.
    let granter = seed_user(
        pool,
        &format!("granter-{}@admin-lifecycle.test", Uuid::new_v4()),
        "active",
    )
    .await;
    sqlx::query(
        r#"
        INSERT INTO capability_grants (user_id, capability, granted_by, mfa_required)
        VALUES ($1, $2, $3, true)
        "#,
    )
    .bind(user_id)
    .bind(capability)
    .bind(granter)
    .execute(pool)
    .await
    .expect("grant capability");
}

/// Mint an access token via the production `JwtService` so the
/// `extract_admin_token` helper in `users_lifecycle.rs` (which validates via
/// the same service) accepts it. `roles` must include one of `ADMIN_ROLES`
/// (e.g. `super_admin`) or `has_admin_role` returns false → 403 before the
/// extractor even runs.
fn mint_admin_token(jwt_secret: &str, user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let svc = JwtService::new(jwt_secret).expect("jwt service");
    svc.generate_access_token(
        user_id,
        email,
        "Admin",
        None,
        Some(vec!["super_admin".to_string()]),
    )
    .expect("mint access token")
}

/// Seed a fully-authorized admin (platform kind, enrolled, recent MFA) AND
/// grant them the requested capability. Returns `(admin_id, bearer_token)`.
async fn seed_admin_with_capability(
    pool: &PgPool,
    jwt_secret: &str,
    capability: &str,
) -> (Uuid, String) {
    let email = format!("admin-{}@admin-lifecycle.test", Uuid::new_v4());
    let admin_id = seed_user(pool, &email, "active").await;
    promote_to_platform(pool, admin_id).await;
    enroll_mfa(pool, admin_id).await;
    record_recent_mfa(pool, admin_id).await;
    grant_capability(pool, admin_id, capability).await;
    let token = mint_admin_token(jwt_secret, admin_id, &email);
    (admin_id, token)
}

/// Same as [`seed_admin_with_capability`] but skips the capability grant —
/// used for the "capability denied" path.
async fn seed_admin_without_capability(pool: &PgPool, jwt_secret: &str) -> (Uuid, String) {
    let email = format!("admin-{}@admin-lifecycle.test", Uuid::new_v4());
    let admin_id = seed_user(pool, &email, "active").await;
    promote_to_platform(pool, admin_id).await;
    enroll_mfa(pool, admin_id).await;
    record_recent_mfa(pool, admin_id).await;
    let token = mint_admin_token(jwt_secret, admin_id, &email);
    (admin_id, token)
}

/// Insert a refresh token for `user_id` so suspend/delete can verify the
/// "revoke all sessions" side effect (`revoked_at` flips from NULL).
async fn seed_refresh_token(pool: &PgPool, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, NOW() + INTERVAL '7 days')
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(format!("hash-{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed refresh token")
}

async fn refresh_token_revoked(pool: &PgPool, token_id: Uuid) -> bool {
    let revoked: Option<bool> =
        sqlx::query_scalar("SELECT (revoked_at IS NOT NULL) FROM refresh_tokens WHERE id = $1")
            .bind(token_id)
            .fetch_optional(pool)
            .await
            .expect("query refresh token");
    revoked.unwrap_or(false)
}

async fn user_status(pool: &PgPool, user_id: Uuid) -> String {
    sqlx::query_scalar::<_, String>("SELECT status FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("read user status")
}

/// Count `audit_logs` rows authored by `actor_id` whose JSONB `details`
/// carry the given capability + outcome. The extractor writes ONE such row
/// per gate evaluation (allowed or denied), independent of whether the
/// handler itself records additional audit entries.
async fn audit_count(pool: &PgPool, actor_id: Uuid, capability: &str, outcome: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint
        FROM audit_logs
        WHERE user_id = $1
          AND details ->> 'capability' = $2
          AND details ->> 'outcome' = $3
        "#,
    )
    .bind(actor_id)
    .bind(capability)
    .bind(outcome)
    .fetch_one(pool)
    .await
    .expect("count audit rows")
}

// ===========================================================================
// Tests
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn suspend_user_happy_path_revokes_sessions_and_audits(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (admin_id, token) =
        seed_admin_with_capability(&pool, &app.config.jwt_secret, "users_write").await;

    let target_id = seed_user(&pool, "suspend-target@admin-lifecycle.test", "active").await;
    let refresh_id = seed_refresh_token(&pool, target_id).await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/admin/users/{}/suspend", target_id))
                .bearer(&token)
                .json(json!({ "reason": "test" }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);

    // User row flipped to suspended.
    assert_eq!(user_status(&pool, target_id).await, "suspended");

    // The target's active refresh token was revoked.
    assert!(
        refresh_token_revoked(&pool, refresh_id).await,
        "suspend must revoke the target's refresh tokens"
    );

    // RequireCapability emitted an `allowed` audit row for the admin.
    assert_eq!(
        audit_count(&pool, admin_id, "users_write", "allowed").await,
        1,
        "extractor must emit one allowed audit row"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn suspend_user_without_capability_is_forbidden(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (admin_id, token) = seed_admin_without_capability(&pool, &app.config.jwt_secret).await;

    let target_id = seed_user(&pool, "suspend-denied@admin-lifecycle.test", "active").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/admin/users/{}/suspend", target_id))
                .bearer(&token)
                .json(json!({ "reason": "denied"}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::FORBIDDEN);

    // Target row remains untouched.
    assert_eq!(user_status(&pool, target_id).await, "active");

    // Denial is audited too (extractor records both allowed/denied outcomes).
    assert_eq!(
        audit_count(&pool, admin_id, "users_write", "denied").await,
        1,
        "extractor must emit one denied audit row when the grant is missing"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn suspend_already_suspended_user_returns_404(pool: PgPool) {
    // Documents *current* behavior: the underlying `UserRepository::suspend`
    // filters on `status = 'active'`, so a second suspend returns `Ok(None)`
    // and the handler surfaces 404 USER_NOT_FOUND ("not found OR already
    // suspended"). This is NOT a no-op 200 — if the contract changes to
    // idempotent OK, flip this assertion.
    let app = TestApp::new(pool.clone()).await;
    let (_admin_id, token) =
        seed_admin_with_capability(&pool, &app.config.jwt_secret, "users_write").await;

    let target_id = seed_user(&pool, "already-suspended@admin-lifecycle.test", "suspended").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/admin/users/{}/suspend", target_id))
                .bearer(&token)
                .json(json!({}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::NOT_FOUND);
    // Status unchanged.
    assert_eq!(user_status(&pool, target_id).await, "suspended");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn reactivate_suspended_user_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (admin_id, token) =
        seed_admin_with_capability(&pool, &app.config.jwt_secret, "users_write").await;

    let target_id = seed_user(&pool, "reactivate-target@admin-lifecycle.test", "suspended").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/admin/users/{}/reactivate", target_id))
                .bearer(&token)
                .json(json!({ "reason": "appeal granted" }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    assert_eq!(user_status(&pool, target_id).await, "active");

    // Extractor emitted an allowed row for the reactivate call too.
    assert!(
        audit_count(&pool, admin_id, "users_write", "allowed").await >= 1,
        "extractor must audit the reactivate call"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn delete_user_happy_path_sets_status_deleted(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (admin_id, token) =
        seed_admin_with_capability(&pool, &app.config.jwt_secret, "users_write").await;

    let target_id = seed_user(&pool, "delete-target@admin-lifecycle.test", "active").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/admin/users/{}/delete", target_id))
                .bearer(&token)
                .json(json!({ "reason": "gdpr" }))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::OK);
    assert_eq!(user_status(&pool, target_id).await, "deleted");

    assert!(
        audit_count(&pool, admin_id, "users_write", "allowed").await >= 1,
        "extractor must audit the delete call"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn delete_already_deleted_user_returns_404(pool: PgPool) {
    // `UserRepository::soft_delete` filters on `status != 'deleted'`, so a
    // second delete returns `Ok(None)` → 404. Documents that the handler is
    // NOT idempotent at the OK level for already-deleted users.
    let app = TestApp::new(pool.clone()).await;
    let (_admin_id, token) =
        seed_admin_with_capability(&pool, &app.config.jwt_secret, "users_write").await;

    let target_id = seed_user(&pool, "already-deleted@admin-lifecycle.test", "deleted").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/admin/users/{}/delete", target_id))
                .bearer(&token)
                .json(json!({}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::NOT_FOUND);
    assert_eq!(user_status(&pool, target_id).await, "deleted");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "requires postgres + migrations"]
async fn suspend_self_is_rejected_with_400(pool: PgPool) {
    // The handler's first guard after token extraction is the self-action
    // check. An admin POSTing /users/{their_own_id}/suspend → 400
    // CANNOT_SELF_SUSPEND. Worth pinning so a refactor cannot silently let
    // admins lock themselves out.
    let app = TestApp::new(pool.clone()).await;
    let (admin_id, token) =
        seed_admin_with_capability(&pool, &app.config.jwt_secret, "users_write").await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/admin/users/{}/suspend", admin_id))
                .bearer(&token)
                .json(json!({}))
                .build(),
        )
        .await;

    resp.assert_status(StatusCode::BAD_REQUEST);
    // Admin remained active.
    assert_eq!(user_status(&pool, admin_id).await, "active");
}
