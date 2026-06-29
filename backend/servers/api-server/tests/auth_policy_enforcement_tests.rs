//! Integration tests for the AuthPolicyEnforcer wiring (Defense N2 / leak #13).
//!
//! These tests exercise the enforcer DIRECTLY against a real database — the
//! HTTP-layer wiring (axum handler → enforcer → DB) is covered by handler
//! plumbing already; what matters here is the policy semantics:
//!
//!   * Happy path: an org with `require_email_verification = true` accepts
//!     a grant to a verified user.
//!   * Rejection path: the same org REJECTS a grant to an unverified user.
//!   * Default-policy path: no `org_auth_policies` row → no email-verification
//!     gate → grant proceeds for an unverified user (status quo).

#![allow(dead_code)]

use api_server::services::{AuthPolicyEnforcer, AuthPolicyError};
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Enforcer {slug}"))
    .bind(format!("enforcer-{slug}"))
    .bind(format!("{slug}@n2-int.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str, verified: bool) -> Uuid {
    let verified_at = if verified {
        Some(chrono::Utc::now())
    } else {
        None
    };
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'N2 Int', 'active', $2)
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(verified_at)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn require_verified_email(pool: &PgPool, org_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO org_auth_policies (organization_id, policy)
        VALUES ($1, '{"require_email_verification": true}'::jsonb)
        ON CONFLICT (organization_id) DO UPDATE SET policy = EXCLUDED.policy
        "#,
    )
    .bind(org_id)
    .execute(pool)
    .await
    .expect("set policy");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn grant_to_verified_user_under_strict_policy_succeeds(pool: PgPool) {
    let org = seed_org(&pool, "happy").await;
    let user = seed_user(&pool, "happy-verified@n2-int.test", true).await;
    require_verified_email(&pool, org).await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_membership_grant(org, user).await;

    assert!(
        result.is_ok(),
        "verified user under strict policy should pass: {:?}",
        result.err()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn grant_to_unverified_user_under_strict_policy_is_rejected(pool: PgPool) {
    let org = seed_org(&pool, "rejected").await;
    let user = seed_user(&pool, "rejected-unverified@n2-int.test", false).await;
    require_verified_email(&pool, org).await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_membership_grant(org, user).await;

    assert!(
        matches!(result, Err(AuthPolicyError::EmailNotVerified)),
        "unverified user under strict policy must be rejected, got: {result:?}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn grant_to_unverified_user_under_default_policy_succeeds(pool: PgPool) {
    // No `org_auth_policies` row → default policy → no verification gate →
    // unverified user can be granted (status quo). This documents that
    // adopting AuthPolicyEnforcer is purely additive: orgs that don't set a
    // policy keep their existing behavior.
    let org = seed_org(&pool, "default").await;
    let user = seed_user(&pool, "default-unverified@n2-int.test", false).await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_membership_grant(org, user).await;

    assert!(
        result.is_ok(),
        "default policy should not gate on email verification: {:?}",
        result.err()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn capability_grant_per_user_picks_first_membership(pool: PgPool) {
    use db::models::membership::GrantMembership;
    use db::repositories::MembershipRepository;

    let org = seed_org(&pool, "cap-grant").await;
    let user = seed_user(&pool, "cap-grant-unverified@n2-int.test", false).await;
    require_verified_email(&pool, org).await;

    // Grant the user a membership in the strict-policy org.
    let mem_repo = MembershipRepository::new(pool.clone());
    mem_repo
        .grant(GrantMembership {
            user_id: user,
            organization_id: org,
            role: "manager".into(),
            granted_by: None,
            expires_at: None,
        })
        .await
        .expect("seed membership");

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_capability_grant_for_user(user).await;

    assert!(
        matches!(result, Err(AuthPolicyError::EmailNotVerified)),
        "capability grant for unverified member of strict-policy org must reject, got: {result:?}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn capability_grant_per_user_with_no_memberships_passes(pool: PgPool) {
    // A platform-only principal with no memberships has no per-org policy
    // to enforce — the grant proceeds (governed by platform defaults).
    let user = seed_user(&pool, "platform-only@n2-int.test", false).await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_capability_grant_for_user(user).await;

    assert!(
        result.is_ok(),
        "platform-only principal should not be gated by per-org policy: {:?}",
        result.err()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn revoke_loads_policy_as_liveness_check(pool: PgPool) {
    // Revoke has no positive policy gate today, but it MUST load the policy
    // (so a corrupted row aborts the revoke). This test just verifies the
    // load path is exercised — a missing policy still returns Ok via the
    // default-fallback contract.
    let org = seed_org(&pool, "revoke-liveness").await;
    let user = seed_user(&pool, "revoke-liveness@n2-int.test", true).await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_membership_revoke(org, user).await;

    assert!(
        result.is_ok(),
        "revoke liveness check must pass: {:?}",
        result.err()
    );
}

// ============================================================================
// D2.1 — capability revoke / platform-action enforcement
// ============================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn capability_revoke_platform_action_passes(pool: PgPool) {
    // D2.1: capability_grants are platform-scoped. The enforcer's
    // `check_capability_revoke` delegates to `check_platform_action`, which
    // loads the platform-default policy as a liveness check. The principal-
    // kind + recent-MFA invariants are owned by the `RequireCapability`
    // extractor — this test just asserts the enforcer's call is non-blocking
    // when the platform default is loadable.
    let actor = seed_user(&pool, "platform-actor@n2-int.test", true).await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_capability_revoke(actor).await;

    assert!(
        result.is_ok(),
        "platform-action liveness check must pass: {:?}",
        result.err()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn check_platform_action_does_not_require_org(pool: PgPool) {
    // The platform-action gate must be callable WITHOUT any membership in
    // scope. This documents the platform-scoping decision (capabilities
    // never carry org_id) — see
    // docs/multitenancy/decisions/capability-platform-scope.md.
    let actor = seed_user(&pool, "no-org-actor@n2-int.test", true).await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_platform_action(actor).await;

    assert!(
        result.is_ok(),
        "platform-action must not require an org: {:?}",
        result.err()
    );
}

// ============================================================================
// D2.2 — login MFA enforcement against per-org policy
// ============================================================================

async fn require_mfa_for_role(pool: &PgPool, org_id: Uuid, role: &str) {
    let policy = serde_json::json!({
        "mfa_required_for_roles": [role],
    });
    sqlx::query(
        r#"
        INSERT INTO org_auth_policies (organization_id, policy)
        VALUES ($1, $2::jsonb)
        ON CONFLICT (organization_id) DO UPDATE SET policy = EXCLUDED.policy
        "#,
    )
    .bind(org_id)
    .bind(policy)
    .execute(pool)
    .await
    .expect("set policy");
}

async fn grant_membership(pool: &PgPool, user_id: Uuid, org_id: Uuid, role: &str) {
    use db::models::membership::GrantMembership;
    use db::repositories::MembershipRepository;
    let mem_repo = MembershipRepository::new(pool.clone());
    mem_repo
        .grant(GrantMembership {
            user_id,
            organization_id: org_id,
            role: role.into(),
            granted_by: None,
            expires_at: None,
        })
        .await
        .expect("seed membership");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_with_mfa_required_role_and_no_mfa_is_rejected(pool: PgPool) {
    // Org policy demands MFA for the `manager` role; the user holds that role
    // and has not presented an MFA factor → MfaRequired.
    let org = seed_org(&pool, "login-mfa-reject").await;
    let user = seed_user(&pool, "login-mfa-reject@n2-int.test", true).await;
    grant_membership(&pool, user, org, "manager").await;
    require_mfa_for_role(&pool, org, "manager").await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_login(user, false).await;

    assert!(
        matches!(result, Err(AuthPolicyError::MfaRequired(ref r)) if r == "manager"),
        "login without MFA under MFA-required policy must reject with MfaRequired(manager), got: {result:?}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_with_mfa_required_role_and_mfa_presented_succeeds(pool: PgPool) {
    // Same setup, but the user supplied an MFA factor in the login request →
    // the enforcer accepts the login.
    let org = seed_org(&pool, "login-mfa-ok").await;
    let user = seed_user(&pool, "login-mfa-ok@n2-int.test", true).await;
    grant_membership(&pool, user, org, "manager").await;
    require_mfa_for_role(&pool, org, "manager").await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_login(user, true).await;

    assert!(
        result.is_ok(),
        "login with MFA presented under MFA-required policy must succeed: {:?}",
        result.err()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_under_default_policy_passes_without_mfa(pool: PgPool) {
    // No org policy → no MFA roles → login proceeds without an MFA factor.
    // This documents the additive contract: orgs that opt out of per-org
    // policy keep the platform default behavior.
    let user = seed_user(&pool, "login-default-policy@n2-int.test", true).await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_login(user, false).await;

    assert!(
        result.is_ok(),
        "default policy must not gate login on MFA: {:?}",
        result.err()
    );
}

// ============================================================================
// D2.2 — password change against per-org policy
// ============================================================================

async fn require_strict_password(pool: &PgPool, org_id: Uuid) {
    let policy = serde_json::json!({
        "min_password_length": 16,
        "require_uppercase": true,
        "require_digit": true,
        "require_symbol": true,
    });
    sqlx::query(
        r#"
        INSERT INTO org_auth_policies (organization_id, policy)
        VALUES ($1, $2::jsonb)
        ON CONFLICT (organization_id) DO UPDATE SET policy = EXCLUDED.policy
        "#,
    )
    .bind(org_id)
    .bind(policy)
    .execute(pool)
    .await
    .expect("set strict password policy");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn password_change_too_short_under_strict_policy_is_rejected(pool: PgPool) {
    // A 12-char password meets the platform default (8 min) but violates the
    // org's 16-char minimum → enforcer rejects.
    let org = seed_org(&pool, "pw-strict").await;
    let user = seed_user(&pool, "pw-strict@n2-int.test", true).await;
    grant_membership(&pool, user, org, "owner").await;
    require_strict_password(&pool, org).await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer
        .check_password_change(user, "Sh0rt!Pass12") // 12 chars, has classes
        .await;

    let violations = match result {
        Err(AuthPolicyError::PasswordPolicy(v)) => v,
        other => panic!("expected PasswordPolicy violations, got {other:?}"),
    };
    assert!(
        violations.iter().any(|m| m.contains("at least 16")),
        "expected min-length violation, got {violations:?}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn password_change_meeting_strict_policy_succeeds(pool: PgPool) {
    let org = seed_org(&pool, "pw-strict-ok").await;
    let user = seed_user(&pool, "pw-strict-ok@n2-int.test", true).await;
    grant_membership(&pool, user, org, "owner").await;
    require_strict_password(&pool, org).await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer
        .check_password_change(user, "LongEnough#Pass1234")
        .await;

    assert!(
        result.is_ok(),
        "password meeting strict policy must succeed: {:?}",
        result.err()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn password_change_for_org_less_user_uses_platform_default(pool: PgPool) {
    // A user with no memberships → strictest-across is platform default
    // (8 min, no class requirements).
    let user = seed_user(&pool, "pw-no-org@n2-int.test", true).await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_password_change(user, "anything8").await;

    assert!(
        result.is_ok(),
        "default policy must accept an 8-char password for an org-less user: {:?}",
        result.err()
    );
}
