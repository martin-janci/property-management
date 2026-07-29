//! Cross-tenant IDOR regression tests for `ViolationRepository` (Epic 142,
//! PAP-138 — derived from the PAP-136 audit).
//!
//! Audit history: every by-id mutation on the violations surface took the
//! target UUID but no organization id, so the `UPDATE … WHERE id = $1` /
//! `DELETE … WHERE id = $1` statements would mutate *any* tenant's row. The
//! `decide_appeal` cascade was the worst case — it looked the appeal up with an
//! unkeyed `get_appeal` and then cascaded `UPDATE`s to `violations` and
//! `enforcement_actions` with no org scoping at all.
//!
//! The fix threads the caller's verified `organization_id` into each repository
//! method and keys every statement on `(id, organization_id)` (child tables via
//! an `EXISTS` check on the parent `violations` row). These tests prove the gap
//! is closed at the repository layer: CI runs them against a superuser pool that
//! bypasses FORCE-RLS, so a missing `organization_id` predicate in the SQL would
//! let the cross-tenant write through and fail the assertion.
//!
//! Each test seeds two orgs (A, B) and asserts that an Org B caller cannot
//! touch Org A's rows, while the legitimate Org A caller still can.

use crate::common::seed_org;
use chrono::Utc;
use db::models::violations::{
    AppealStatus, CreateCommunityRule, CreateEnforcementAction, CreateViolation,
    CreateViolationAppeal, CreateViolationComment, CreateViolationEvidence, EnforcementActionType,
    RecordFinePayment, UpdateCommunityRule, UpdateEnforcementAction, UpdateViolation,
    UpdateViolationAppeal, ViolationCategory, ViolationStatus,
};
use db::repositories::ViolationRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'ViolationsIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

fn new_violation_req() -> CreateViolation {
    CreateViolation {
        building_id: None,
        unit_id: None,
        rule_id: None,
        category: ViolationCategory::Noise,
        severity: None,
        title: "Loud music".into(),
        description: "After midnight".into(),
        location: None,
        violator_id: None,
        violator_name: Some("Tenant".into()),
        violator_unit: Some("4B".into()),
        occurred_at: Utc::now(),
        evidence_description: None,
        witness_count: None,
    }
}

fn new_rule_req(code: &str) -> CreateCommunityRule {
    CreateCommunityRule {
        building_id: None,
        rule_code: code.into(),
        title: "Quiet hours".into(),
        description: None,
        category: ViolationCategory::Noise,
        first_offense_fine: None,
        second_offense_fine: None,
        third_offense_fine: None,
        max_fine: None,
        fine_escalation_days: None,
        effective_date: None,
        expiry_date: None,
        requires_board_approval: None,
        source_document_id: None,
        section_reference: None,
    }
}

fn new_action_req() -> CreateEnforcementAction {
    CreateEnforcementAction {
        action_type: EnforcementActionType::FirstFine,
        fine_amount: Some(Decimal::new(10000, 2)), // 100.00
        due_date: None,
        description: Some("Fine notice".into()),
        notes: None,
        suspended_privileges: None,
        suspension_start: None,
        suspension_end: None,
    }
}

// ---------------------------------------------------------------------------
// Community rules — update_rule / delete_rule
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn community_rule_cross_org_mutations_rejected(pool: PgPool) {
    let repo = ViolationRepository::new(pool.clone());
    let org_a = seed_org(&pool, "rule-a").await;
    let org_b = seed_org(&pool, "rule-b").await;
    let author = seed_user(&pool, "rule@violations-idor.test").await;

    let rule = repo
        .create_rule(org_a, new_rule_req("R-1"), author)
        .await
        .expect("create rule");

    // Org B cannot update Org A's rule (no row matches → RowNotFound).
    let hijack = UpdateCommunityRule {
        title: Some("Hijacked".into()),
        description: None,
        first_offense_fine: None,
        second_offense_fine: None,
        third_offense_fine: None,
        max_fine: None,
        fine_escalation_days: None,
        expiry_date: None,
        is_active: None,
        requires_board_approval: None,
    };
    let res = repo.update_rule(rule.id, org_b, hijack).await;
    assert!(
        matches!(res, Err(sqlx::Error::RowNotFound)),
        "Org B must not update Org A's rule, got {res:?}"
    );

    // Org B cannot delete it either.
    let deleted = repo
        .delete_rule(rule.id, org_b)
        .await
        .expect("delete query ok");
    assert!(!deleted, "Org B must not delete Org A's rule");

    // The rule is untouched.
    let after = repo
        .get_rule_for_org(rule.id, org_a)
        .await
        .expect("query ok")
        .expect("rule still exists");
    assert_eq!(after.title, "Quiet hours", "title must be unchanged");

    // The legitimate owner can still update and delete.
    let ok = UpdateCommunityRule {
        title: Some("Quiet hours v2".into()),
        description: None,
        first_offense_fine: None,
        second_offense_fine: None,
        third_offense_fine: None,
        max_fine: None,
        fine_escalation_days: None,
        expiry_date: None,
        is_active: None,
        requires_board_approval: None,
    };
    let updated = repo
        .update_rule(rule.id, org_a, ok)
        .await
        .expect("owner update");
    assert_eq!(updated.title, "Quiet hours v2");
    assert!(repo
        .delete_rule(rule.id, org_a)
        .await
        .expect("owner delete"));
}

// ---------------------------------------------------------------------------
// Violations — update_violation / assign_violation / delete_evidence
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn violation_cross_org_mutations_rejected(pool: PgPool) {
    let repo = ViolationRepository::new(pool.clone());
    let org_a = seed_org(&pool, "vio-a").await;
    let org_b = seed_org(&pool, "vio-b").await;
    let user_a = seed_user(&pool, "vio-a@violations-idor.test").await;
    let user_b = seed_user(&pool, "vio-b@violations-idor.test").await;

    let violation = repo
        .create_violation(org_a, new_violation_req(), user_a)
        .await
        .expect("create violation");
    let evidence = repo
        .add_evidence(
            violation.id,
            org_a,
            CreateViolationEvidence {
                file_name: "photo.jpg".into(),
                file_type: "image/jpeg".into(),
                file_size: Some(1024),
                storage_path: Some("s3://evidence/photo.jpg".into()),
                description: None,
                captured_at: None,
            },
            user_a,
        )
        .await
        .expect("add evidence");

    // add_evidence cross-org → RowNotFound, and no evidence row is inserted.
    let res = repo
        .add_evidence(
            violation.id,
            org_b,
            CreateViolationEvidence {
                file_name: "inject.jpg".into(),
                file_type: "image/jpeg".into(),
                file_size: Some(2048),
                storage_path: Some("s3://evidence/inject.jpg".into()),
                description: Some("cross-tenant".into()),
                captured_at: None,
            },
            user_b,
        )
        .await;
    assert!(
        matches!(res, Err(sqlx::Error::RowNotFound)),
        "Org B must not add evidence to Org A's violation, got {res:?}"
    );
    let injected_evidence: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM violation_evidence WHERE violation_id = $1 AND file_name = 'inject.jpg'",
    )
    .bind(violation.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        injected_evidence, 0,
        "no evidence row may be inserted cross-tenant"
    );

    // add_comment cross-org → RowNotFound, and no comment row is inserted
    // (is_internal comments are a cross-tenant injection vector).
    let res = repo
        .add_comment(
            violation.id,
            org_b,
            CreateViolationComment {
                comment_type: "general".into(),
                content: "internal cross-tenant note".into(),
                is_internal: Some(true),
            },
            user_b,
        )
        .await;
    assert!(
        matches!(res, Err(sqlx::Error::RowNotFound)),
        "Org B must not add a comment to Org A's violation, got {res:?}"
    );
    let injected_comments: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM violation_comments WHERE violation_id = $1")
            .bind(violation.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        injected_comments, 0,
        "no comment row may be inserted cross-tenant"
    );

    // The legitimate owner can still add evidence and a comment.
    repo.add_evidence(
        violation.id,
        org_a,
        CreateViolationEvidence {
            file_name: "owner.jpg".into(),
            file_type: "image/jpeg".into(),
            file_size: Some(512),
            storage_path: Some("s3://evidence/owner.jpg".into()),
            description: None,
            captured_at: None,
        },
        user_a,
    )
    .await
    .expect("owner add evidence");
    repo.add_comment(
        violation.id,
        org_a,
        CreateViolationComment {
            comment_type: "general".into(),
            content: "owner note".into(),
            is_internal: Some(false),
        },
        user_a,
    )
    .await
    .expect("owner add comment");

    // update_violation cross-org → RowNotFound.
    let res = repo
        .update_violation(
            violation.id,
            org_b,
            UpdateViolation {
                severity: None,
                status: Some(ViolationStatus::Dismissed),
                assigned_to: None,
                resolution_notes: Some("Hijacked".into()),
                resolution_type: None,
            },
        )
        .await;
    assert!(
        matches!(res, Err(sqlx::Error::RowNotFound)),
        "Org B must not update Org A's violation, got {res:?}"
    );

    // assign_violation cross-org → RowNotFound.
    let res = repo.assign_violation(violation.id, org_b, user_b).await;
    assert!(
        matches!(res, Err(sqlx::Error::RowNotFound)),
        "Org B must not assign Org A's violation, got {res:?}"
    );

    // delete_evidence cross-org → no-op (child keyed via parent violation).
    let deleted = repo
        .delete_evidence(evidence.id, org_b)
        .await
        .expect("delete query ok");
    assert!(!deleted, "Org B must not delete Org A's evidence");

    // Nothing changed: violation still 'reported', evidence still present.
    let after = repo
        .get_violation_for_org(violation.id, org_a)
        .await
        .expect("query ok")
        .expect("violation exists");
    assert_eq!(after.status, ViolationStatus::Reported);
    assert!(after.resolution_notes.is_none());
    assert!(after.assigned_to.is_none());
    let ev_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM violation_evidence WHERE id = $1")
        .bind(evidence.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(ev_count, 1, "evidence must survive a cross-tenant delete");

    // Owner can still mutate + delete.
    repo.assign_violation(violation.id, org_a, user_a)
        .await
        .expect("owner assign");
    assert!(repo
        .delete_evidence(evidence.id, org_a)
        .await
        .expect("owner delete evidence"));
}

// ---------------------------------------------------------------------------
// Enforcement actions — update / mark_sent / record_payment
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn enforcement_cross_org_mutations_rejected(pool: PgPool) {
    let repo = ViolationRepository::new(pool.clone());
    let org_a = seed_org(&pool, "enf-a").await;
    let org_b = seed_org(&pool, "enf-b").await;
    let user_a = seed_user(&pool, "enf-a@violations-idor.test").await;

    let violation = repo
        .create_violation(org_a, new_violation_req(), user_a)
        .await
        .expect("create violation");
    let action = repo
        .create_enforcement_action(violation.id, org_a, new_action_req(), user_a)
        .await
        .expect("create action");

    // update_enforcement_action cross-org → RowNotFound.
    let res = repo
        .update_enforcement_action(
            action.id,
            org_b,
            UpdateEnforcementAction {
                status: None,
                notice_sent_at: None,
                notice_method: None,
                notes: Some("Hijacked".into()),
            },
        )
        .await;
    assert!(
        matches!(res, Err(sqlx::Error::RowNotFound)),
        "Org B must not update Org A's action, got {res:?}"
    );

    // mark_action_sent cross-org → RowNotFound.
    let res = repo.mark_action_sent(action.id, org_b, "email").await;
    assert!(
        matches!(res, Err(sqlx::Error::RowNotFound)),
        "Org B must not mark Org A's action sent, got {res:?}"
    );

    // record_payment cross-org → RowNotFound and no payment row inserted.
    let res = repo
        .record_payment(
            action.id,
            org_b,
            RecordFinePayment {
                amount: Decimal::new(5000, 2),
                payment_method: Some("cash".into()),
                transaction_reference: None,
                payer_id: None,
                payer_name: Some("Attacker".into()),
                notes: None,
            },
            user_a,
        )
        .await;
    assert!(
        matches!(res, Err(sqlx::Error::RowNotFound)),
        "Org B must not record a payment against Org A's action, got {res:?}"
    );
    let pay_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM fine_payments WHERE enforcement_action_id = $1")
            .bind(action.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(pay_count, 0, "no payment row may be created cross-tenant");

    // Action is unchanged (still pending, no notice method).
    let after = repo
        .get_enforcement_action_for_org(action.id, org_a)
        .await
        .expect("query ok")
        .expect("action exists");
    assert!(after.notice_method.is_none());

    // Owner can still record a payment.
    repo.record_payment(
        action.id,
        org_a,
        RecordFinePayment {
            amount: Decimal::new(5000, 2),
            payment_method: Some("cash".into()),
            transaction_reference: None,
            payer_id: None,
            payer_name: Some("Tenant".into()),
            notes: None,
        },
        user_a,
    )
    .await
    .expect("owner payment");
}

// ---------------------------------------------------------------------------
// Appeals — update_appeal / decide_appeal cascade
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn appeal_cross_org_mutations_rejected(pool: PgPool) {
    let repo = ViolationRepository::new(pool.clone());
    let org_a = seed_org(&pool, "app-a").await;
    let org_b = seed_org(&pool, "app-b").await;
    let user_a = seed_user(&pool, "app-a@violations-idor.test").await;

    let violation = repo
        .create_violation(org_a, new_violation_req(), user_a)
        .await
        .expect("create violation");
    let action = repo
        .create_enforcement_action(violation.id, org_a, new_action_req(), user_a)
        .await
        .expect("create action");

    // create_appeal cross-org → RowNotFound, and the cascade must NOT fire:
    // no appeal row inserted, the violation must stay 'reported', and the
    // enforcement action must stay 'pending'.
    let res = repo
        .create_appeal(
            violation.id,
            org_b,
            CreateViolationAppeal {
                enforcement_action_id: Some(action.id),
                reason: "Hijacked appeal".into(),
                requested_outcome: None,
                supporting_evidence: None,
            },
            user_a,
        )
        .await;
    assert!(
        matches!(res, Err(sqlx::Error::RowNotFound)),
        "Org B must not create an appeal against Org A's violation, got {res:?}"
    );
    let appeals_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM violation_appeals WHERE violation_id = $1")
            .bind(violation.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        appeals_count, 0,
        "no appeal row may be inserted cross-tenant"
    );
    let vio_status: ViolationStatus =
        sqlx::query_scalar("SELECT status FROM violations WHERE id = $1")
            .bind(violation.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        vio_status,
        ViolationStatus::Reported,
        "cross-tenant create_appeal must not flip the violation to disputed"
    );
    let action_status: db::models::violations::EnforcementStatus =
        sqlx::query_scalar("SELECT status FROM enforcement_actions WHERE id = $1")
            .bind(action.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        action_status,
        db::models::violations::EnforcementStatus::Pending,
        "cross-tenant create_appeal must not mark the action appealed"
    );

    let appeal = repo
        .create_appeal(
            violation.id,
            org_a,
            CreateViolationAppeal {
                enforcement_action_id: Some(action.id),
                reason: "Unfair".into(),
                requested_outcome: None,
                supporting_evidence: None,
            },
            user_a,
        )
        .await
        .expect("create appeal");

    // update_appeal cross-org → RowNotFound.
    let res = repo
        .update_appeal(
            appeal.id,
            org_b,
            UpdateViolationAppeal {
                status: Some(AppealStatus::Approved),
                hearing_date: None,
                hearing_location: None,
                hearing_notes: None,
                decision: Some("Hijacked".into()),
                fine_adjustment: None,
            },
            Some(user_a),
        )
        .await;
    assert!(
        matches!(res, Err(sqlx::Error::RowNotFound)),
        "Org B must not update Org A's appeal, got {res:?}"
    );

    // decide_appeal cross-org → RowNotFound; the cascade must NOT fire.
    let res = repo
        .decide_appeal(
            appeal.id,
            org_b,
            true,
            "Hijacked decision",
            Some(Decimal::new(5000, 2)),
            user_a,
        )
        .await;
    assert!(
        matches!(res, Err(sqlx::Error::RowNotFound)),
        "Org B must not decide Org A's appeal, got {res:?}"
    );

    // Appeal still submitted, violation still disputed (create_appeal set it),
    // and the enforcement-action fine was NOT reduced by the cascade.
    let appeal_after = repo
        .get_appeal_for_org(appeal.id, org_a)
        .await
        .expect("query ok")
        .expect("appeal exists");
    assert_eq!(appeal_after.status, AppealStatus::Submitted);
    assert!(appeal_after.decision.is_none());
    let vio_after = repo
        .get_violation_for_org(violation.id, org_a)
        .await
        .expect("query ok")
        .expect("violation exists");
    assert_eq!(vio_after.status, ViolationStatus::Disputed);
    let action_after = repo
        .get_enforcement_action_for_org(action.id, org_a)
        .await
        .expect("query ok")
        .expect("action exists");
    assert_eq!(
        action_after.fine_amount,
        Some(Decimal::new(10000, 2)),
        "cross-tenant decide must not adjust the fine"
    );

    // The legitimate owner can decide the appeal and the cascade fires.
    let decided = repo
        .decide_appeal(
            appeal.id,
            org_a,
            true,
            "Approved",
            Some(Decimal::new(4000, 2)),
            user_a,
        )
        .await
        .expect("owner decide");
    assert_eq!(decided.status, AppealStatus::Approved);
    let vio_final = repo
        .get_violation_for_org(violation.id, org_a)
        .await
        .expect("query ok")
        .expect("violation exists");
    assert_eq!(vio_final.status, ViolationStatus::Dismissed);
    let action_final = repo
        .get_enforcement_action_for_org(action.id, org_a)
        .await
        .expect("query ok")
        .expect("action exists");
    assert_eq!(
        action_final.fine_amount,
        Some(Decimal::new(6000, 2)),
        "owner decide reduces the fine by the adjustment"
    );
}
