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
    let verified_at = if verified { Some(chrono::Utc::now()) } else { None };
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

#[sqlx::test]
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

#[sqlx::test]
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

#[sqlx::test]
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

#[sqlx::test]
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

#[sqlx::test]
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

#[sqlx::test]
async fn revoke_loads_policy_as_liveness_check(pool: PgPool) {
    // Revoke has no positive policy gate today, but it MUST load the policy
    // (so a corrupted row aborts the revoke). This test just verifies the
    // load path is exercised — a missing policy still returns Ok via the
    // default-fallback contract.
    let org = seed_org(&pool, "revoke-liveness").await;
    let user = seed_user(&pool, "revoke-liveness@n2-int.test", true).await;

    let enforcer = AuthPolicyEnforcer::new(pool.clone());
    let result = enforcer.check_membership_revoke(org, user).await;

    assert!(result.is_ok(), "revoke liveness check must pass: {:?}", result.err());
}
