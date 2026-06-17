//! Regression test for issue #520 — cross-tenant authz on
//! `DisputeRepository::update_status`. A manager in org A must not be
//! able to drive the state machine of a dispute in org B by guessing its
//! UUID. The repo enforces this with `WHERE id = $2 AND organization_id = $3`.

use db::models::{AddEvidence, FileDispute, UpdateDisputeStatus};
use db::repositories::DisputeRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Dispute {slug}"))
    .bind(slug)
    .bind(format!("{slug}@dispute.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

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

// ---------------------------------------------------------------------------
// add_evidence — cross-org IDOR (BIT-73)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_evidence_cross_org_rejected(pool: PgPool) {
    let repo = DisputeRepository::new(pool.clone());
    let org_a = seed_org(&pool, "ev-idor-a").await;
    let org_b = seed_org(&pool, "ev-idor-b").await;
    let user_a = seed_user(&pool, "ev-idor-a@dispute.test").await;
    let user_b = seed_user(&pool, "ev-idor-b@dispute.test").await;

    let dispute = repo
        .file_dispute(
            org_a,
            FileDispute {
                organization_id: org_a,
                building_id: None,
                unit_id: None,
                category: "noise".into(),
                title: "IDOR test".into(),
                description: "BIT-73".into(),
                desired_resolution: None,
                respondent_ids: vec![],
                filed_by: user_a,
            },
        )
        .await
        .expect("file dispute");

    // Org B cannot attach evidence to Org A's dispute.
    let res = repo
        .add_evidence(
            org_b,
            AddEvidence {
                dispute_id: dispute.id,
                uploaded_by: user_b,
                filename: "hack.pdf".into(),
                original_filename: "hack.pdf".into(),
                content_type: "application/pdf".into(),
                size_bytes: 1024,
                storage_url: "s3://evil/hack.pdf".into(),
                description: None,
            },
        )
        .await;
    assert!(
        res.is_err(),
        "Org B must not add evidence to Org A's dispute, got {res:?}"
    );

    // No evidence row was inserted.
    let ev_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM dispute_evidence WHERE dispute_id = $1")
            .bind(dispute.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        ev_count, 0,
        "no evidence must be persisted after cross-org attempt"
    );

    // Legitimate owner can still add evidence.
    let ev = repo
        .add_evidence(
            org_a,
            AddEvidence {
                dispute_id: dispute.id,
                uploaded_by: user_a,
                filename: "real.pdf".into(),
                original_filename: "real.pdf".into(),
                content_type: "application/pdf".into(),
                size_bytes: 2048,
                storage_url: "s3://real/real.pdf".into(),
                description: None,
            },
        )
        .await
        .expect("owner add_evidence");
    assert_eq!(ev.dispute_id, dispute.id);
}
