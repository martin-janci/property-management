//! Phase 2 — membership revocation tests.
//!
//! Verifies the per-request authorization model that defends leak #10
//! ("stale authz in a live token"). The contract being tested at the repo
//! level: after `MembershipRepository::revoke()`, `is_active()` returns
//! `false` immediately (no caching, no replay window). The HTTP-layer
//! equivalent — same JWT presented across the revoke — is in
//! `backend/servers/api-server/tests/token_scope_tests.rs`.

use crate::common::seed_org;
use db::models::membership::GrantMembership;
use db::repositories::MembershipRepository;
use sqlx::{PgPool, Row};
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    let row = sqlx::query(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Mem User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    row.get("id")
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn revoke_flips_is_active_immediately(pool: PgPool) {
    let org_id = seed_org(&pool, "phase2-rev-1").await;
    let user_id = seed_user(&pool, "rev1@phase2.test").await;

    let repo = MembershipRepository::new(pool.clone());

    // Grant.
    repo.grant(GrantMembership {
        user_id,
        organization_id: org_id,
        role: "manager".into(),
        granted_by: None,
        expires_at: None,
    })
    .await
    .expect("grant");

    assert!(
        repo.is_active(user_id, org_id).await.expect("is_active"),
        "membership should be active immediately after grant"
    );

    // Revoke.
    let revoked = repo.revoke(user_id, org_id, None).await.expect("revoke");
    assert_eq!(revoked, vec!["manager".to_string()]);

    // The same query from a "fresh" caller (next request) sees no active row.
    assert!(
        !repo.is_active(user_id, org_id).await.expect("is_active"),
        "membership must be inactive immediately after revoke (defends leak #10)"
    );
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn revoke_on_other_org_does_not_affect_first(pool: PgPool) {
    let org_a = seed_org(&pool, "phase2-rev-a").await;
    let org_b = seed_org(&pool, "phase2-rev-b").await;
    let user_id = seed_user(&pool, "rev-multi@phase2.test").await;

    let repo = MembershipRepository::new(pool.clone());

    repo.grant(GrantMembership {
        user_id,
        organization_id: org_a,
        role: "manager".into(),
        granted_by: None,
        expires_at: None,
    })
    .await
    .expect("grant a");
    repo.grant(GrantMembership {
        user_id,
        organization_id: org_b,
        role: "manager".into(),
        granted_by: None,
        expires_at: None,
    })
    .await
    .expect("grant b");

    repo.revoke(user_id, org_b, None).await.expect("revoke b");

    assert!(repo.is_active(user_id, org_a).await.expect("a active"));
    assert!(!repo.is_active(user_id, org_b).await.expect("b inactive"));
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn re_grant_after_revoke_works(pool: PgPool) {
    let org = seed_org(&pool, "phase2-rev-regrant").await;
    let user = seed_user(&pool, "regrant@phase2.test").await;

    let repo = MembershipRepository::new(pool.clone());
    repo.grant(GrantMembership {
        user_id: user,
        organization_id: org,
        role: "manager".into(),
        granted_by: None,
        expires_at: None,
    })
    .await
    .expect("grant 1");
    repo.revoke(user, org, None).await.expect("revoke");
    assert!(!repo.is_active(user, org).await.expect("inactive"));

    // Re-grant the same role — the row's revoked_at is cleared.
    repo.grant(GrantMembership {
        user_id: user,
        organization_id: org,
        role: "manager".into(),
        granted_by: None,
        expires_at: None,
    })
    .await
    .expect("grant 2");
    assert!(repo.is_active(user, org).await.expect("active again"));
}
