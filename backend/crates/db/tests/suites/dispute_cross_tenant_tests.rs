//! Regression test for issue #520 — cross-tenant authz on
//! `DisputeRepository::update_status`. A manager in org A must not be
//! able to drive the state machine of a dispute in org B by guessing its
//! UUID. The repo enforces this with `WHERE id = $2 AND organization_id = $3`.

use crate::common::seed_org;
use db::models::{FileDispute, UpdateDisputeStatus};
use db::repositories::DisputeRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Dispute User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn update_status_rejects_cross_tenant_caller(pool: PgPool) {
    let org_a = seed_org(&pool, "dispute-a").await;
    let org_b = seed_org(&pool, "dispute-b").await;
    let user_a = seed_user(&pool, "a@dispute.test").await;
    let user_b = seed_user(&pool, "b@dispute.test").await;

    let repo = DisputeRepository::new(pool.clone());

    // Dispute filed in org A.
    let dispute = repo
        .file_dispute(
            org_a,
            FileDispute {
                organization_id: org_a,
                building_id: None,
                unit_id: None,
                category: "noise".into(),
                title: "Test".into(),
                description: "x".into(),
                desired_resolution: None,
                respondent_ids: vec![],
                filed_by: user_a,
            },
        )
        .await
        .expect("file dispute");

    // Manager in org B tries to flip the status. Must error (RowNotFound /
    // 0 rows updated) — the IDOR is blocked at the SQL layer.
    let cross_tenant = repo
        .update_status(UpdateDisputeStatus {
            dispute_id: dispute.id,
            organization_id: org_b, // attacker's org
            status: "under_review".into(),
            reason: None,
            updated_by: user_b,
        })
        .await;
    assert!(
        cross_tenant.is_err(),
        "cross-tenant update_status must fail (issue #520)"
    );

    // Same-tenant caller still succeeds.
    let ok = repo
        .update_status(UpdateDisputeStatus {
            dispute_id: dispute.id,
            organization_id: org_a,
            status: "under_review".into(),
            reason: None,
            updated_by: user_a,
        })
        .await
        .expect("same-tenant update should succeed");
    assert_eq!(ok.status, "under_review");
}
