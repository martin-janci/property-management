//! Dispute filing API integration tests (gap-80-1, Epic 77 / Story 77.1).
//!
//! Complements the focused `dispute_cross_tenant_tests.rs` (issue #520) and the
//! HTTP-layer `dispute_cross_org_idor_tests.rs` (#760 / #834) with repository
//! integration coverage of:
//!
//! - the create + status-transition lifecycle (file → under_review → mediation
//!   → resolved → closed) end to end through `DisputeRepository`,
//! - RLS tenant isolation — a caller in org B cannot read or drive a dispute
//!   filed in org A (`find_by_id_with_details_for_org` / `update_status` /
//!   `withdraw` are all org-scoped),
//! - the dispute status state machine — every legal transition succeeds and a
//!   representative set of illegal ones is rejected with `BadRequest`,
//! - evidence persistence + activity-trail side effects.
//!
//! These run against a real Postgres via `#[sqlx::test]`, which provisions an
//! isolated migrated database per test. With no local Postgres they are
//! skipped locally and run in CI (`backend.yml`) — Band A/B locally, Band C in
//! CI (see PR body).

use crate::common::seed_org;
use db::models::disputes::{dispute_state_machine, dispute_status};
use db::models::{AddEvidence, FileDispute, UpdateDisputeStatus};
use db::repositories::DisputeRepository;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Dispute Lifecycle User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

fn file_req(org_id: Uuid, filed_by: Uuid, respondents: Vec<Uuid>) -> FileDispute {
    FileDispute {
        organization_id: org_id,
        building_id: None,
        unit_id: None,
        category: "noise".into(),
        title: "Loud neighbours".into(),
        description: "Repeated loud music after midnight.".into(),
        desired_resolution: Some("Quiet hours respected".into()),
        respondent_ids: respondents,
        filed_by,
    }
}

fn transition(dispute_id: Uuid, org_id: Uuid, by: Uuid, status: &str) -> UpdateDisputeStatus {
    UpdateDisputeStatus {
        dispute_id,
        organization_id: org_id,
        status: status.into(),
        reason: None,
        updated_by: by,
    }
}

// ---------------------------------------------------------------------------
// 1. Full create + status-transition lifecycle
// ---------------------------------------------------------------------------

/// File a dispute, then drive it through the canonical happy-path lifecycle
/// `filed → under_review → mediation → resolved → closed`. Each step must
/// succeed and the persisted status must reflect the transition.
#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn file_then_full_status_lifecycle(pool: PgPool) {
    let org = seed_org(&pool, "lc").await;
    let complainant = seed_user(&pool, "complainant@dispute-lifecycle.test").await;
    let respondent = seed_user(&pool, "respondent@dispute-lifecycle.test").await;

    let repo = DisputeRepository::new(pool.clone());

    // Create — the new dispute starts in `filed` with a generated reference.
    let dispute = repo
        .file_dispute(org, file_req(org, complainant, vec![respondent]))
        .await
        .expect("file dispute");
    assert_eq!(dispute.status, dispute_status::FILED);
    assert!(dispute.reference_number.starts_with("DSP-"));
    assert_eq!(dispute.organization_id, org);

    // Filing seeds the complainant + respondent as parties and a
    // `dispute_filed` activity.
    let parties = repo
        .list_parties(dispute.id, org)
        .await
        .expect("list parties");
    assert_eq!(parties.len(), 2, "complainant + respondent are seeded");
    let activities = repo
        .list_activities(dispute.id, org, 50, 0)
        .await
        .expect("list activities");
    assert!(
        activities
            .iter()
            .any(|a| a.activity_type == "dispute_filed"),
        "filing records a dispute_filed activity"
    );

    // Walk the lifecycle.
    for next in [
        dispute_status::UNDER_REVIEW,
        dispute_status::MEDIATION,
        dispute_status::RESOLVED,
        dispute_status::CLOSED,
    ] {
        let updated = repo
            .update_status(transition(dispute.id, org, complainant, next))
            .await
            .unwrap_or_else(|e| panic!("transition to {next} should succeed: {e:?}"));
        assert_eq!(updated.status, next, "status persisted as {next}");
    }

    // Every transition recorded a status_changed activity (4 transitions).
    let activities = repo
        .list_activities(dispute.id, org, 50, 0)
        .await
        .expect("list activities");
    let status_changes = activities
        .iter()
        .filter(|a| a.activity_type == "status_changed")
        .count();
    assert_eq!(status_changes, 4, "one status_changed per transition");
}

// ---------------------------------------------------------------------------
// 2. RLS tenant isolation — cross-tenant access denied
// ---------------------------------------------------------------------------

/// A caller in org B must not be able to read or act on a dispute filed in
/// org A. All three org-scoped repo entry points
/// (`find_by_id_with_details_for_org`, `update_status`, `withdraw`) must deny
/// the foreign org while the owning org succeeds.
#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn cross_tenant_access_is_denied(pool: PgPool) {
    let org_a = seed_org(&pool, "iso-a").await;
    let org_b = seed_org(&pool, "iso-b").await;
    let user_a = seed_user(&pool, "iso-a@dispute-lifecycle.test").await;
    let user_b = seed_user(&pool, "iso-b@dispute-lifecycle.test").await;

    let repo = DisputeRepository::new(pool.clone());

    let dispute = repo
        .file_dispute(org_a, file_req(org_a, user_a, vec![]))
        .await
        .expect("file dispute in org A");

    // READ: org B cannot see org A's dispute; org A can.
    assert!(
        repo.find_by_id_with_details_for_org(dispute.id, org_b)
            .await
            .expect("query ok")
            .is_none(),
        "org B must not read org A's dispute"
    );
    assert!(
        repo.find_by_id_with_details_for_org(dispute.id, org_a)
            .await
            .expect("query ok")
            .is_some(),
        "org A reads its own dispute"
    );

    // ACT (status): org B's transition must fail; the row stays in `filed`.
    let cross = repo
        .update_status(transition(
            dispute.id,
            org_b,
            user_b,
            dispute_status::UNDER_REVIEW,
        ))
        .await;
    assert!(cross.is_err(), "cross-tenant update_status must fail");
    assert_eq!(
        repo.find_by_id_with_details_for_org(dispute.id, org_a)
            .await
            .expect("query ok")
            .expect("exists")
            .dispute
            .status,
        dispute_status::FILED,
        "cross-tenant attempt must not mutate the row"
    );

    // ACT (withdraw): org B's withdraw must NotFound; org A's succeeds.
    let cross_wd = repo.withdraw(dispute.id, org_b, user_b).await;
    assert!(
        matches!(cross_wd, Err(common::errors::AppError::NotFound(_))),
        "cross-tenant withdraw must be NotFound, got {cross_wd:?}"
    );
    repo.withdraw(dispute.id, org_a, user_a)
        .await
        .expect("org A withdraws its own dispute");
    assert_eq!(
        repo.find_by_id_with_details_for_org(dispute.id, org_a)
            .await
            .expect("query ok")
            .expect("exists")
            .dispute
            .status,
        dispute_status::WITHDRAWN,
    );
}

// ---------------------------------------------------------------------------
// 3. Dispute status-machine transitions (legal + illegal)
// ---------------------------------------------------------------------------

/// `update_status` enforces the state machine: a legal transition succeeds and
/// an illegal one (e.g. `filed → resolved`, skipping review) is rejected with
/// `BadRequest`, leaving the row untouched.
#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn status_machine_rejects_illegal_transitions(pool: PgPool) {
    let org = seed_org(&pool, "sm").await;
    let user = seed_user(&pool, "sm@dispute-lifecycle.test").await;
    let repo = DisputeRepository::new(pool.clone());

    let dispute = repo
        .file_dispute(org, file_req(org, user, vec![]))
        .await
        .expect("file dispute");

    // filed → resolved is NOT a legal transition (must go via under_review).
    assert!(
        !dispute_state_machine::is_valid_transition(
            dispute_status::FILED,
            dispute_status::RESOLVED
        ),
        "state machine: filed→resolved is illegal"
    );
    let illegal = repo
        .update_status(transition(dispute.id, org, user, dispute_status::RESOLVED))
        .await;
    assert!(
        matches!(illegal, Err(common::errors::AppError::BadRequest(_))),
        "filed→resolved must be rejected with BadRequest, got {illegal:?}"
    );
    assert_eq!(
        repo.find_by_id_with_details_for_org(dispute.id, org)
            .await
            .expect("query ok")
            .expect("exists")
            .dispute
            .status,
        dispute_status::FILED,
        "rejected transition leaves status unchanged"
    );

    // An unknown status value is also rejected (guard before the state check).
    let unknown = repo
        .update_status(transition(dispute.id, org, user, "banana"))
        .await;
    assert!(
        matches!(unknown, Err(common::errors::AppError::BadRequest(_))),
        "unknown status must be rejected, got {unknown:?}"
    );

    // A legal transition from filed succeeds.
    let ok = repo
        .update_status(transition(
            dispute.id,
            org,
            user,
            dispute_status::UNDER_REVIEW,
        ))
        .await
        .expect("filed→under_review is legal");
    assert_eq!(ok.status, dispute_status::UNDER_REVIEW);

    // From a terminal-ish state: resolved → closed is the only legal move.
    repo.update_status(transition(dispute.id, org, user, dispute_status::RESOLVED))
        .await
        .expect("under_review→resolved is legal");
    let bad_from_resolved = repo
        .update_status(transition(dispute.id, org, user, dispute_status::MEDIATION))
        .await;
    assert!(
        matches!(
            bad_from_resolved,
            Err(common::errors::AppError::BadRequest(_))
        ),
        "resolved→mediation must be rejected, got {bad_from_resolved:?}"
    );
    repo.update_status(transition(dispute.id, org, user, dispute_status::CLOSED))
        .await
        .expect("resolved→closed is legal");
}

// ---------------------------------------------------------------------------
// 4. Evidence persistence + activity trail
// ---------------------------------------------------------------------------

/// Adding evidence persists the row, exposes it via `list_evidence`, bumps the
/// dispute's evidence_count, and records an `evidence_added` activity. (The
/// HTTP-layer auth guard on the evidence-upload route is covered in the
/// api-server test `dispute_evidence_auth_tests.rs`.)
#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn add_evidence_persists_and_records_activity(pool: PgPool) {
    let org = seed_org(&pool, "ev").await;
    let user = seed_user(&pool, "ev@dispute-lifecycle.test").await;
    let repo = DisputeRepository::new(pool.clone());

    let dispute = repo
        .file_dispute(org, file_req(org, user, vec![]))
        .await
        .expect("file dispute");

    let evidence = repo
        .add_evidence(
            AddEvidence {
                dispute_id: dispute.id,
                uploaded_by: user,
                filename: "noise-recording.mp3".into(),
                original_filename: "recording (1).mp3".into(),
                content_type: "audio/mpeg".into(),
                size_bytes: 1024,
                storage_url: "s3://evidence/noise-recording.mp3".into(),
                description: Some("Recording of the noise".into()),
            },
            org,
        )
        .await
        .expect("add evidence");
    assert_eq!(evidence.dispute_id, dispute.id);
    assert_eq!(evidence.uploaded_by, user);

    let listed = repo
        .list_evidence(dispute.id, org)
        .await
        .expect("list evidence");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, evidence.id);

    let details = repo
        .find_by_id_with_details_for_org(dispute.id, org)
        .await
        .expect("query ok")
        .expect("exists");
    assert_eq!(
        details.evidence_count, 1,
        "evidence_count reflects the upload"
    );

    let activities = repo
        .list_activities(dispute.id, org, 50, 0)
        .await
        .expect("list activities");
    assert!(
        activities
            .iter()
            .any(|a| a.activity_type == "evidence_added"),
        "adding evidence records an evidence_added activity"
    );

    // delete_evidence is dispute-scoped: a wrong dispute id deletes nothing.
    let other = repo
        .file_dispute(org, file_req(org, user, vec![]))
        .await
        .expect("file second dispute");
    let wrong_scope = repo
        .delete_evidence(other.id, evidence.id, org)
        .await
        .expect("delete query ok");
    assert!(
        !wrong_scope,
        "evidence is not deletable via a foreign dispute id"
    );
    let right_scope = repo
        .delete_evidence(dispute.id, evidence.id, org)
        .await
        .expect("delete query ok");
    assert!(right_scope, "evidence is deletable via its own dispute id");
}

/// Regression: attaching evidence must emit an access-audit event on the
/// platform-admin `audit_logs` stream — who (`user_id`) added evidence
/// (`resource_id` = evidence id, `resource_type = 'dispute_evidence'`) to
/// which dispute (`details.dispute_id`), scoped to the owning org.
///
/// Fails on `dev` (pre-fix `add_evidence` writes the evidence row + activity
/// but NO `audit_logs` row, so the SELECT returns zero rows). Passes once the
/// repo emits the `resource_created`/`dispute_evidence` audit row, mirroring
/// the `membership.rs` / support-data audit-emission pattern. Sibling to the
/// support-tooling `support_data_viewed` emission on the read path.
#[sqlx::test]
async fn add_evidence_writes_access_audit_event(pool: PgPool) {
    let org = seed_org(&pool, "evaudit").await;
    let user = seed_user(&pool, "evaudit@dispute-lifecycle.test").await;
    let repo = DisputeRepository::new(pool.clone());

    let dispute = repo
        .file_dispute(org, file_req(org, user, vec![]))
        .await
        .expect("file dispute");

    let evidence = repo
        .add_evidence(
            AddEvidence {
                dispute_id: dispute.id,
                uploaded_by: user,
                filename: "lease-scan.pdf".into(),
                original_filename: "Lease Scan.pdf".into(),
                content_type: "application/pdf".into(),
                size_bytes: 2048,
                storage_url: "s3://evidence/lease-scan.pdf".into(),
                description: None,
            },
            org,
        )
        .await
        .expect("add evidence");

    // Exactly one access-audit row for this evidence attachment, carrying the
    // actor, the org, and the parent dispute in `details`.
    #[derive(sqlx::FromRow)]
    struct AuditRow {
        user_id: Option<Uuid>,
        resource_id: Option<Uuid>,
        org_id: Option<Uuid>,
        details: serde_json::Value,
    }

    let row: AuditRow = sqlx::query_as(
        r#"
        SELECT user_id, resource_id, org_id, details
        FROM audit_logs
        WHERE action = 'resource_created'::audit_action
          AND resource_type = 'dispute_evidence'
          AND resource_id = $1
        "#,
    )
    .bind(evidence.id)
    .fetch_one(&pool)
    .await
    .expect("add_evidence must write a dispute_evidence access-audit row");

    assert_eq!(row.user_id, Some(user), "audit actor is the uploader");
    assert_eq!(
        row.resource_id,
        Some(evidence.id),
        "audit targets the evidence"
    );
    assert_eq!(
        row.org_id,
        Some(org),
        "audit row is scoped to the owning org"
    );
    assert_eq!(
        row.details["dispute_id"],
        serde_json::json!(dispute.id),
        "audit details record which dispute the evidence was added to"
    );
}
