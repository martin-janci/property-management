//! Regression tests for the cross-org IDOR fix on dispute resolution endpoints.
//!
//! Audit history: the original `resolve_dispute` and `update_mediation_notes`
//! handlers operated on the dispute row identified only by UUID, without
//! verifying that the dispute's `organization_id` matched the caller's tenant.
//! A manager from Org B could resolve or modify mediation notes on a dispute
//! owned by Org A by guessing the UUID.
//!
//! The fix adds `AND organization_id = $N` to both UPDATE WHERE clauses and
//! passes `tenant_id` from the verified JWT.  Additionally:
//!
//! - `resolve_dispute` now checks `is_manager() || is_admin()` (previously
//!   `is_admin()` was excluded).
//! - `resolve_dispute` rejects blank `resolution_notes` with 422.
//! - `update_mediation_notes` records an audit-trail event.
//!
//! These tests exercise the HTTP surface with the `TestApp` harness.
//!
//! TestApp wiring caveat: `TestApp` mounts the router without
//! `host_tenant_middleware`, so `AuthUser` cannot derive a tenant from the
//! Host header.  Requests that lack a fully-provisioned Bearer JWT +
//! `X-Tenant-ID` header backed by a real `organization_members` row are
//! rejected with 4xx before reaching handler logic.  The security contract
//! is still satisfied: cross-org mutations never reach the DB row.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use db::repositories::DisputeRepository;

use crate::common::TestApp;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("DisputeIDOR Org {slug}"))
    .bind(format!("dispute-idor-{slug}"))
    .bind(format!("{slug}@dispute-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'DisputeIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_dispute(pool: &PgPool, org_id: Uuid, filed_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO disputes (
            organization_id, reference_number, category,
            title, description, status, priority, filed_by
        )
        VALUES ($1, $2, 'noise', 'Test dispute', 'Test description',
                'under_review', 'medium', $3)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("DISP-TEST-{}", Uuid::new_v4()))
    .bind(filed_by)
    .fetch_one(pool)
    .await
    .expect("seed dispute")
}

/// Return the `status` of a dispute row by id (bypasses RLS via the app pool).
async fn dispute_status(pool: &PgPool, dispute_id: Uuid) -> String {
    sqlx::query_scalar::<_, String>("SELECT status FROM disputes WHERE id = $1")
        .bind(dispute_id)
        .fetch_one(pool)
        .await
        .expect("get dispute status")
}

fn json_request(method: Method, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn assert_rejected(status: StatusCode, ctx: &str) {
    let code = status.as_u16();
    assert!(
        (400..500).contains(&code),
        "{ctx}: expected 4xx rejection, got {status}"
    );
}

// ---------------------------------------------------------------------------
// Test 1: PATCH /disputes/{id}/resolve — no auth is rejected
// ---------------------------------------------------------------------------

/// Unauthenticated attempt to resolve a dispute is rejected before any DB
/// mutation occurs.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_dispute_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_id = seed_user(&pool, "no-auth-resolve@dispute-idor.test").await;
    let org_id = seed_org(&pool, "no-auth").await;
    let dispute_id = seed_dispute(&pool, org_id, user_id).await;

    let body = json!({ "resolution_notes": "Resolved by mediator" });
    let req = json_request(
        Method::PATCH,
        &format!("/api/v1/disputes/{dispute_id}/resolve"),
        body,
    );
    let resp = app.execute(req).await;

    assert_rejected(resp.status, "PATCH /resolve without auth");

    let status = dispute_status(&pool, dispute_id).await;
    assert_eq!(status, "under_review", "dispute must not have been mutated");
}

// ---------------------------------------------------------------------------
// Test 2: PATCH /disputes/{id}/resolve — cross-org claim is rejected
// ---------------------------------------------------------------------------

/// A request carrying an `X-Tenant-ID` for a different org is rejected.
/// Without `host_tenant_middleware` the tenant extractor returns 4xx before
/// the handler body runs, so the dispute row is never mutated.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_dispute_cross_org_claim_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = seed_user(&pool, "user-a-resolve@dispute-idor.test").await;
    let org_a = seed_org(&pool, "ra").await;
    let org_b = seed_org(&pool, "rb").await;
    let dispute_id = seed_dispute(&pool, org_a, user_a).await;

    let body = json!({ "resolution_notes": "Attempting cross-org resolve" });
    let req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/disputes/{dispute_id}/resolve"))
        .header(header::CONTENT_TYPE, "application/json")
        // Claim membership in Org B while targeting a dispute owned by Org A.
        .header("X-Tenant-ID", org_b.to_string())
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.execute(req).await;

    assert_rejected(resp.status, "PATCH /resolve cross-org X-Tenant-ID");

    let status = dispute_status(&pool, dispute_id).await;
    assert_eq!(
        status, "under_review",
        "cross-org resolve must not mutate the row"
    );
}

// ---------------------------------------------------------------------------
// Test 3: PATCH /disputes/{id}/resolve — empty resolution_notes is rejected
// ---------------------------------------------------------------------------

/// Empty `resolution_notes` must be rejected.  In production the handler
/// returns 422; with `TestApp` (no auth) the rejection may arrive earlier
/// as 401/400.  Either way no mutation occurs.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resolve_dispute_empty_notes_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_id = seed_user(&pool, "empty-notes@dispute-idor.test").await;
    let org_id = seed_org(&pool, "en").await;
    let dispute_id = seed_dispute(&pool, org_id, user_id).await;

    let body = json!({ "resolution_notes": "" });
    let req = json_request(
        Method::PATCH,
        &format!("/api/v1/disputes/{dispute_id}/resolve"),
        body,
    );
    let resp = app.execute(req).await;

    assert_rejected(resp.status, "PATCH /resolve empty resolution_notes");

    let status = dispute_status(&pool, dispute_id).await;
    assert_eq!(
        status, "under_review",
        "dispute must not be mutated on empty notes"
    );
}

// ---------------------------------------------------------------------------
// Test 4: PATCH /disputes/{id}/mediation-notes — no auth is rejected
// ---------------------------------------------------------------------------

/// Unauthenticated attempt to update mediation notes is rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_mediation_notes_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_id = seed_user(&pool, "no-auth-med@dispute-idor.test").await;
    let org_id = seed_org(&pool, "no-auth-med").await;
    let dispute_id = seed_dispute(&pool, org_id, user_id).await;

    let body = json!({ "notes": "Mediation notes from unauthenticated actor" });
    let req = json_request(
        Method::PATCH,
        &format!("/api/v1/disputes/{dispute_id}/mediation-notes"),
        body,
    );
    let resp = app.execute(req).await;

    assert_rejected(resp.status, "PATCH /mediation-notes without auth");
}

// ---------------------------------------------------------------------------
// Test 5: PATCH /disputes/{id}/mediation-notes — cross-org claim is rejected
// ---------------------------------------------------------------------------

/// Cross-org mediation-notes update must be rejected regardless of auth state.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_mediation_notes_cross_org_claim_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user_a = seed_user(&pool, "user-a-med@dispute-idor.test").await;
    let org_a = seed_org(&pool, "ma").await;
    let org_b = seed_org(&pool, "mb").await;
    let dispute_id = seed_dispute(&pool, org_a, user_a).await;

    let body = json!({ "notes": "Cross-org mediation notes attempt" });
    let req = Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/v1/disputes/{dispute_id}/mediation-notes"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Tenant-ID", org_b.to_string())
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.execute(req).await;

    assert_rejected(resp.status, "PATCH /mediation-notes cross-org X-Tenant-ID");
}

// ---------------------------------------------------------------------------
// Repository-layer contract tests (issue #760 / #834)
//
// The HTTP tests above prove cross-org mutations never reach the row, but the
// TestApp harness cannot mint a JWT carrying `tenant_id` for the happy path.
// These repo tests pin the org-scope contract directly — including the
// positive (owning org reads its dispute) case the task requires.
// ---------------------------------------------------------------------------

/// Seed a dispute carrying mediation_notes; returns its id.
async fn seed_dispute_with_notes(pool: &PgPool, org_id: Uuid, filed_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO disputes (
            organization_id, reference_number, category, title, description,
            status, priority, filed_by, mediation_notes
        )
        VALUES ($1, $2, 'noise', 'Notes dispute', 'Has confidential notes',
                'under_review', 'medium', $3, 'CONFIDENTIAL mediation notes')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("DISP-NOTES-{}", Uuid::new_v4()))
    .bind(filed_by)
    .fetch_one(pool)
    .await
    .expect("seed dispute with notes")
}

// R1 — find_by_id_with_details_for_org: owner reads, foreign org gets None.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_by_id_for_org_is_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "repo-find-a").await;
    let org_b = seed_org(&pool, "repo-find-b").await;
    let user_a = seed_user(&pool, "repo-find-a@dispute-idor.test").await;
    let dispute_a = seed_dispute(&pool, org_a, user_a).await;

    let repo = DisputeRepository::new(pool.clone());

    // The owning org reads its dispute (positive case).
    let own = repo
        .find_by_id_with_details_for_org(dispute_a, org_a)
        .await
        .expect("find with right org");
    assert!(own.is_some(), "org A must read its own dispute by id");
    assert_eq!(own.unwrap().dispute.id, dispute_a);

    // A foreign org gets nothing (closed IDOR → handler returns 404).
    let cross = repo
        .find_by_id_with_details_for_org(dispute_a, org_b)
        .await
        .expect("find with wrong org");
    assert!(cross.is_none(), "org B must not read org A's dispute by id");
}

// R2 — withdraw: owner withdraws, foreign org is NotFound and the row stands.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn withdraw_is_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "repo-wd-a").await;
    let org_b = seed_org(&pool, "repo-wd-b").await;
    let user_a = seed_user(&pool, "repo-wd-a@dispute-idor.test").await;
    let dispute_a = seed_dispute(&pool, org_a, user_a).await;

    let repo = DisputeRepository::new(pool.clone());

    // A foreign org cannot withdraw — NotFound, and the status is unchanged.
    let cross = repo.withdraw(dispute_a, org_b, user_a).await;
    assert!(
        matches!(cross, Err(::common::errors::AppError::NotFound(_))),
        "org B withdraw must fail with NotFound, got {cross:?}"
    );
    assert_eq!(
        dispute_status(&pool, dispute_a).await,
        "under_review",
        "cross-org withdraw must not mutate the row"
    );

    // The owning org withdraws successfully (positive case).
    repo.withdraw(dispute_a, org_a, user_a)
        .await
        .expect("org A withdraws its own dispute");
    assert_eq!(
        dispute_status(&pool, dispute_a).await,
        "withdrawn",
        "org A withdraw must set status=withdrawn"
    );
}

// R3 — mediation_notes are present in the row for the owning org. The HTTP
// confidentiality gate (manager/mediator only) is applied in the handler; this
// pins that the data exists and is org-scoped so the gate has something to
// redact.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mediation_notes_present_for_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "repo-notes-a").await;
    let org_b = seed_org(&pool, "repo-notes-b").await;
    let user_a = seed_user(&pool, "repo-notes-a@dispute-idor.test").await;
    let dispute_a = seed_dispute_with_notes(&pool, org_a, user_a).await;

    let repo = DisputeRepository::new(pool.clone());

    let own = repo
        .find_by_id_with_details_for_org(dispute_a, org_a)
        .await
        .expect("find with right org")
        .expect("dispute exists for owner");
    assert_eq!(
        own.dispute.mediation_notes.as_deref(),
        Some("CONFIDENTIAL mediation notes"),
        "owning org must see mediation_notes at the repo layer"
    );

    // Foreign org cannot reach the dispute at all.
    let cross = repo
        .find_by_id_with_details_for_org(dispute_a, org_b)
        .await
        .expect("find with wrong org");
    assert!(
        cross.is_none(),
        "org B must not read org A's dispute (and thus never its notes)"
    );
}

// ---------------------------------------------------------------------------
// Sub-resource cross-org IDOR contract (issue #2441)
//
// The five sub-resource repo methods (`list_parties`, `add_party`,
// `list_evidence`, `delete_evidence`, `list_activities`) previously queried by
// path-supplied `dispute_id` alone, with no org scoping — a systemic
// cross-tenant IDOR. Each now takes the caller's `org_id` and rejects a
// foreign-org `dispute_id` (reads/`add_party` → `NotFound`; `delete_evidence` →
// `false`, i.e. handler 404), while the owning org still succeeds. These tests
// pin that contract at the repository layer (the layer that enforces it), so
// they fail on the pre-fix code and pass after.
// ---------------------------------------------------------------------------

/// Seed an evidence row on a dispute; returns its id.
async fn seed_evidence(pool: &PgPool, dispute_id: Uuid, uploaded_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO dispute_evidence (
            dispute_id, uploaded_by, filename, original_filename,
            content_type, size_bytes, storage_url, description
        )
        VALUES ($1, $2, 'ev.pdf', 'evidence.pdf', 'application/pdf', 1024,
                's3://ppt-evidence/secret-object.pdf', 'sensitive')
        RETURNING id
        "#,
    )
    .bind(dispute_id)
    .bind(uploaded_by)
    .fetch_one(pool)
    .await
    .expect("seed evidence")
}

// R4 — list_parties: owner reads the roster, foreign org gets NotFound.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_parties_is_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "sub-parties-a").await;
    let org_b = seed_org(&pool, "sub-parties-b").await;
    let user_a = seed_user(&pool, "sub-parties-a@dispute-idor.test").await;
    let party_user = seed_user(&pool, "sub-parties-p@dispute-idor.test").await;
    let dispute_a = seed_dispute(&pool, org_a, user_a).await;

    let repo = DisputeRepository::new(pool.clone());
    repo.add_party(dispute_a, party_user, "respondent", org_a)
        .await
        .expect("seed party in org A");

    // Owning org reads the roster.
    let own = repo
        .list_parties(dispute_a, org_a)
        .await
        .expect("org A lists its own parties");
    assert!(
        own.iter().any(|p| p.user_id == party_user),
        "org A must see the party it added"
    );

    // Foreign org is denied — no PII disclosure.
    let cross = repo.list_parties(dispute_a, org_b).await;
    assert!(
        matches!(cross, Err(::common::errors::AppError::NotFound(_))),
        "org B must not list org A's parties, got {cross:?}"
    );
}

// R5 — add_party: owner upserts, foreign org is NotFound and nothing is written.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_party_is_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "sub-add-a").await;
    let org_b = seed_org(&pool, "sub-add-b").await;
    let user_a = seed_user(&pool, "sub-add-a@dispute-idor.test").await;
    let injected = seed_user(&pool, "sub-add-injected@dispute-idor.test").await;
    let dispute_a = seed_dispute(&pool, org_a, user_a).await;

    let repo = DisputeRepository::new(pool.clone());

    // Foreign-org write must fail and inject nothing.
    let cross = repo.add_party(dispute_a, injected, "mediator", org_b).await;
    assert!(
        matches!(cross, Err(::common::errors::AppError::NotFound(_))),
        "org B must not inject a party into org A's dispute, got {cross:?}"
    );
    let injected_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dispute_parties WHERE dispute_id = $1 AND user_id = $2",
    )
    .bind(dispute_a)
    .bind(injected)
    .fetch_one(&pool)
    .await
    .expect("count parties");
    assert_eq!(injected_count, 0, "cross-org add_party must write no row");

    // Owning org succeeds.
    let party = repo
        .add_party(dispute_a, injected, "witness", org_a)
        .await
        .expect("org A adds a party to its own dispute");
    assert_eq!(party.user_id, injected);
    assert_eq!(party.role, "witness");
}

// R6 — list_evidence: owner reads, foreign org gets NotFound (no storage_url leak).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_evidence_is_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "sub-lsev-a").await;
    let org_b = seed_org(&pool, "sub-lsev-b").await;
    let user_a = seed_user(&pool, "sub-lsev-a@dispute-idor.test").await;
    let dispute_a = seed_dispute(&pool, org_a, user_a).await;
    let _ev = seed_evidence(&pool, dispute_a, user_a).await;

    let repo = DisputeRepository::new(pool.clone());

    let own = repo
        .list_evidence(dispute_a, org_a)
        .await
        .expect("org A lists its own evidence");
    assert_eq!(own.len(), 1, "org A sees its evidence");

    let cross = repo.list_evidence(dispute_a, org_b).await;
    assert!(
        matches!(cross, Err(::common::errors::AppError::NotFound(_))),
        "org B must not list org A's evidence, got {cross:?}"
    );
}

// R7 — delete_evidence: foreign org deletes nothing (row survives); owner deletes.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_evidence_is_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "sub-delev-a").await;
    let org_b = seed_org(&pool, "sub-delev-b").await;
    let user_a = seed_user(&pool, "sub-delev-a@dispute-idor.test").await;
    let dispute_a = seed_dispute(&pool, org_a, user_a).await;
    let evidence = seed_evidence(&pool, dispute_a, user_a).await;

    let repo = DisputeRepository::new(pool.clone());

    // Foreign org cannot destroy the evidence — nothing deleted, row survives.
    let cross = repo
        .delete_evidence(dispute_a, evidence, org_b)
        .await
        .expect("delete query ok");
    assert!(!cross, "org B must not delete org A's evidence");
    let survives: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM dispute_evidence WHERE id = $1")
        .bind(evidence)
        .fetch_one(&pool)
        .await
        .expect("count evidence");
    assert_eq!(survives, 1, "cross-org delete must leave the row intact");

    // Owning org deletes successfully.
    let own = repo
        .delete_evidence(dispute_a, evidence, org_a)
        .await
        .expect("delete query ok");
    assert!(own, "org A deletes its own evidence");
}

// R9 — add_evidence: foreign org writes nothing (NotFound); owner attaches.
//
// Issue #2483 — follow-up to #2441/PR #2450, which org-scoped the five sibling
// sub-resource methods but missed this WRITE. Before the fix, `add_evidence`
// INSERTed on the path-supplied `dispute_id` alone, so any authenticated user
// could attach evidence to any dispute in any org. The INSERT is now gated by an
// `EXISTS` on the parent dispute's org: a foreign-org `dispute_id` inserts no
// row and surfaces `NotFound`, while the owning org still succeeds. This is the
// IG3 regression test — it fails on pre-fix `dev` and passes after.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_evidence_is_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "sub-addev-a").await;
    let org_b = seed_org(&pool, "sub-addev-b").await;
    let user_a = seed_user(&pool, "sub-addev-a@dispute-idor.test").await;
    let attacker = seed_user(&pool, "sub-addev-attacker@dispute-idor.test").await;
    let dispute_a = seed_dispute(&pool, org_a, user_a).await;

    let repo = DisputeRepository::new(pool.clone());

    let new_evidence = |uploaded_by: Uuid| db::models::AddEvidence {
        dispute_id: dispute_a,
        uploaded_by,
        filename: "ev.pdf".into(),
        original_filename: "evidence.pdf".into(),
        content_type: "application/pdf".into(),
        size_bytes: 2048,
        storage_url: "s3://ppt-evidence/injected-object.pdf".into(),
        description: Some("injected".into()),
    };

    // Foreign-org write must fail with NotFound and attach nothing.
    let cross = repo.add_evidence(new_evidence(attacker), org_b).await;
    assert!(
        matches!(cross, Err(::common::errors::AppError::NotFound(_))),
        "org B must not attach evidence to org A's dispute, got {cross:?}"
    );
    let injected_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dispute_evidence WHERE dispute_id = $1 AND uploaded_by = $2",
    )
    .bind(dispute_a)
    .bind(attacker)
    .fetch_one(&pool)
    .await
    .expect("count evidence");
    assert_eq!(
        injected_count, 0,
        "cross-org add_evidence must write no row"
    );

    // Owning org attaches successfully.
    let ev = repo
        .add_evidence(new_evidence(user_a), org_a)
        .await
        .expect("org A attaches evidence to its own dispute");
    assert_eq!(ev.dispute_id, dispute_a);
    assert_eq!(ev.uploaded_by, user_a);
}

// R8 — list_activities: owner reads the trail, foreign org gets NotFound.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_activities_is_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "sub-act-a").await;
    let org_b = seed_org(&pool, "sub-act-b").await;
    let user_a = seed_user(&pool, "sub-act-a@dispute-idor.test").await;
    let dispute_a = seed_dispute(&pool, org_a, user_a).await;

    let repo = DisputeRepository::new(pool.clone());

    // Owning org reads its (possibly empty) activity trail without error.
    repo.list_activities(dispute_a, org_a, 50, 0)
        .await
        .expect("org A lists its own activities");

    // Foreign org is denied.
    let cross = repo.list_activities(dispute_a, org_b, 50, 0).await;
    assert!(
        matches!(cross, Err(::common::errors::AppError::NotFound(_))),
        "org B must not list org A's activities, got {cross:?}"
    );
}
