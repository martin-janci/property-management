//! Regression tests for the privileged-mutation role-authz fix on the
//! violations router (code-review finding
//! `code-review-api-handlers-violations-missing-role-authz`).
//!
//! Audit history: every mutation handler in
//! `servers/api-server/src/routes/violations.rs` authenticated the caller and
//! read the org off the JWT `tenant_id` claim, but performed **no role check**.
//! Tenant *membership* (any non-null `tenant_id`) was the only gate, so an
//! ordinary Resident / Tenant could issue a violation, issue a fine
//! (enforcement action), record a payment, or *decide an appeal* — e.g. approve
//! the appeal of their own fine.
//!
//! The fix adds a `require_manager` gate (`TenantRole::is_manager()` —
//! SuperAdmin / PlatformAdmin / OrgAdmin / Manager / TechnicalManager) to the
//! privileged staff mutations. Self-service actions a resident legitimately
//! performs (filing an appeal, commenting) stay open to any member.
//!
//! Tenancy + role both come straight from the JWT claims, which `AuthUser`
//! reads without a DB membership lookup, so these tests mint a real access
//! token per role and drive the handlers end-to-end.

#![allow(dead_code)]

use axum::http::StatusCode;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// JWT minting (matches api_core::extractors::auth::Claims)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct TestClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

/// Mint an access token bound to `org_id` with an explicit `role` claim
/// (snake_case `role_type`, as serialized into real JWTs).
fn token_with_role(user_id: Uuid, org_id: Uuid, role: &str) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some(role.to_string()),
        email: format!("{role}@vio-authz.test"),
        name: format!("Violations {role}"),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Violations Authz User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_violation(pool: &PgPool, org_id: Uuid, number: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO violations (
            organization_id, violation_number, category, title, description, occurred_at
        )
        VALUES ($1, $2, 'noise', 'Loud party', 'Excessive noise after 22:00', NOW())
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(number)
    .fetch_one(pool)
    .await
    .expect("seed violation")
}

// ---------------------------------------------------------------------------
// A non-manager tenant member is forbidden from issuing a fine.
// ---------------------------------------------------------------------------

/// The core finding: a Resident (a legitimate tenant member) must NOT be able
/// to create an enforcement action / fine. Before the fix this returned 2xx;
/// after the fix it is 403.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_issue_fine(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "authz-fine").await;
    let resident = seed_user(&pool, "resident-fine@vio-authz.test").await;
    let violation_id = seed_violation(&pool, org, "VIO-AUTHZ-0001").await;

    let token = token_with_role(resident, org, "resident");
    let resp = app
        .execute(
            app.post(&format!("/api/v1/violations/{violation_id}/actions"))
                .bearer(&token)
                .json(json!({ "action_type": "first_fine", "fine_amount": "100.00" }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "a non-manager tenant member must NOT be able to issue a fine; got {} (body: {})",
        resp.status,
        resp.text()
    );
}

/// A non-manager must NOT be able to decide (approve/deny) an appeal — the
/// "approve their own appeal" half of the finding. The role gate rejects before
/// any appeal lookup, so a random appeal id still yields 403 for a resident.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_decide_appeal(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "authz-appeal").await;
    let resident = seed_user(&pool, "resident-appeal@vio-authz.test").await;

    let token = token_with_role(resident, org, "resident");
    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/violations/appeals/{}/decide",
                Uuid::new_v4()
            ))
            .bearer(&token)
            .json(json!({ "approved": true, "decision": "Approved" }))
            .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "a non-manager must NOT be able to decide an appeal; got {} (body: {})",
        resp.status,
        resp.text()
    );
}

/// Issuing a violation and managing community rules are likewise manager-only.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_issue_violation_or_manage_rules(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "authz-misc").await;
    let resident = seed_user(&pool, "resident-misc@vio-authz.test").await;
    let token = token_with_role(resident, org, "resident");

    let create_violation = app
        .execute(
            app.post("/api/v1/violations")
                .bearer(&token)
                .json(json!({
                    "category": "noise",
                    "title": "x",
                    "description": "y",
                    "occurred_at": chrono::Utc::now().to_rfc3339()
                }))
                .build(),
        )
        .await;
    assert_eq!(
        create_violation.status,
        StatusCode::FORBIDDEN,
        "resident must not issue a violation; got {}",
        create_violation.status
    );

    let create_rule = app
        .execute(
            app.post("/api/v1/violations/rules")
                .bearer(&token)
                .json(json!({ "rule_code": "X-1", "title": "t", "category": "noise" }))
                .build(),
        )
        .await;
    assert_eq!(
        create_rule.status,
        StatusCode::FORBIDDEN,
        "resident must not create a community rule; got {}",
        create_rule.status
    );
}

// ---------------------------------------------------------------------------
// Assignee org-membership guard on assign_violation
// (code-review finding
// `code-review-api-handlers-assign-violation-no-member-check`).
//
// A manager passes `require_manager`, but `assign_violation` previously fed the
// request's `assigned_to` straight to the repo with no check that the assignee
// belongs to the manager's own org — so a manager could assign a violation to
// any user UUID, including users outside their organization. The fix adds an
// `org_member_repo.is_member` gate on the assignee.
// ---------------------------------------------------------------------------

/// The core finding: a manager must NOT be able to assign a violation to a user
/// who is not an active member of the manager's organization. Before the fix
/// this returned 2xx; after the fix it is 403.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_cannot_assign_violation_to_non_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "assign-outsider").await;
    let manager = seed_user(&pool, "manager-assign@vio-authz.test").await;
    let violation_id = seed_violation(&pool, org, "VIO-AUTHZ-ASSIGN-0001").await;

    // An outsider user that exists but is NOT a member of `org`.
    let outsider = seed_user(&pool, "outsider@vio-authz.test").await;

    let token = token_with_role(manager, org, "manager");
    let resp = app
        .execute(
            app.post(&format!("/api/v1/violations/{violation_id}/assign"))
                .bearer(&token)
                .json(json!({ "assigned_to": outsider }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "a manager must NOT be able to assign a violation to a user outside \
         their org; got {} (body: {})",
        resp.status,
        resp.text()
    );
}

/// Positive control: assigning to a user who IS an active member of the org
/// succeeds (2xx), proving the guard rejects only outsiders rather than the
/// assign mutation itself.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_can_assign_violation_to_member(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "assign-member").await;
    let manager = seed_user(&pool, "manager-assign-ok@vio-authz.test").await;
    let violation_id = seed_violation(&pool, org, "VIO-AUTHZ-ASSIGN-0002").await;

    // An in-org member is a valid assignee.
    let member = seed_user(&pool, "member@vio-authz.test").await;
    seed_membership(&pool, org, member, "resident").await;

    let token = token_with_role(manager, org, "manager");
    let resp = app
        .execute(
            app.post(&format!("/api/v1/violations/{violation_id}/assign"))
                .bearer(&token)
                .json(json!({ "assigned_to": member }))
                .build(),
        )
        .await;

    assert!(
        resp.status.is_success(),
        "a manager must be able to assign a violation to an in-org member; \
         got {} (body: {})",
        resp.status,
        resp.text()
    );
    assert_ne!(
        resp.status,
        StatusCode::FORBIDDEN,
        "an in-org assignee must never be blocked by the membership guard"
    );
}

// ---------------------------------------------------------------------------
// Positive control: a manager is NOT blocked by the gate.
// ---------------------------------------------------------------------------

/// A Manager passes the gate and successfully issues a fine (2xx), proving the
/// fix rejects only unprivileged roles rather than the mutation itself.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_can_issue_fine(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "authz-mgr").await;
    let manager = seed_user(&pool, "manager@vio-authz.test").await;
    let violation_id = seed_violation(&pool, org, "VIO-AUTHZ-MGR-0001").await;

    let token = token_with_role(manager, org, "manager");
    let resp = app
        .execute(
            app.post(&format!("/api/v1/violations/{violation_id}/actions"))
                .bearer(&token)
                .json(json!({ "action_type": "first_fine", "fine_amount": "100.00" }))
                .build(),
        )
        .await;

    assert!(
        resp.status.is_success(),
        "a manager must be able to issue a fine; got {} (body: {})",
        resp.status,
        resp.text()
    );
    assert_ne!(
        resp.status,
        StatusCode::FORBIDDEN,
        "manager must never be blocked by the role gate"
    );
}

// ---------------------------------------------------------------------------
// Full privileged-mutation surface: every `require_manager`-gated route must
// reject a non-manager member with 403.
// ---------------------------------------------------------------------------

/// The gate runs as the first line of each handler, *before* any DB lookup, so
/// a random path id still yields 403 (see `resident_cannot_decide_appeal`).
/// This table enumerates the complete manager-only mutation surface of
/// `routes/violations.rs` so that dropping `require_manager` from any one of
/// them — or adding a new privileged mutation without the gate — trips a test.
///
/// Self-service routes deliberately NOT in this list (open to any member):
/// `POST /{id}/evidence` (add_evidence), `POST /{id}/appeals` (create_appeal),
/// `POST /{id}/comments` (add_comment). They get their own positive control in
/// `resident_can_file_appeal_and_comment`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_forbidden_on_every_privileged_mutation(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "authz-surface").await;
    let resident = seed_user(&pool, "resident-surface@vio-authz.test").await;
    let token = token_with_role(resident, org, "resident");

    let vid = Uuid::new_v4();
    let aid = Uuid::new_v4();
    let rid = Uuid::new_v4();
    let eid = Uuid::new_v4();
    let appeal = Uuid::new_v4();

    // (label, method, uri, body)
    enum M {
        Post,
        Put,
        Delete,
    }
    let cases: Vec<(&str, M, String, serde_json::Value)> = vec![
        (
            "POST /rules (create_rule)",
            M::Post,
            "/api/v1/violations/rules".to_string(),
            json!({ "rule_code": "X-1", "title": "t", "category": "noise" }),
        ),
        (
            "PUT /rules/{id} (update_rule)",
            M::Put,
            format!("/api/v1/violations/rules/{rid}"),
            json!({ "title": "t2" }),
        ),
        (
            "DELETE /rules/{id} (delete_rule)",
            M::Delete,
            format!("/api/v1/violations/rules/{rid}"),
            json!({}),
        ),
        (
            "POST / (create_violation)",
            M::Post,
            "/api/v1/violations".to_string(),
            json!({
                "category": "noise",
                "title": "x",
                "description": "y",
                "occurred_at": chrono::Utc::now().to_rfc3339()
            }),
        ),
        (
            "PUT /{id} (update_violation)",
            M::Put,
            format!("/api/v1/violations/{vid}"),
            json!({ "status": "resolved" }),
        ),
        (
            "POST /{id}/assign (assign_violation)",
            M::Post,
            format!("/api/v1/violations/{vid}/assign"),
            json!({ "assigned_to": Uuid::new_v4() }),
        ),
        (
            "DELETE /{id}/evidence/{eid} (delete_evidence)",
            M::Delete,
            format!("/api/v1/violations/{vid}/evidence/{eid}"),
            json!({}),
        ),
        (
            "POST /{id}/actions (create_action / issue fine)",
            M::Post,
            format!("/api/v1/violations/{vid}/actions"),
            json!({ "action_type": "first_fine", "fine_amount": "100.00" }),
        ),
        (
            "PUT /{id}/actions/{aid} (update_action)",
            M::Put,
            format!("/api/v1/violations/{vid}/actions/{aid}"),
            json!({ "status": "sent" }),
        ),
        (
            "POST /{id}/actions/{aid}/send (mark_action_sent)",
            M::Post,
            format!("/api/v1/violations/{vid}/actions/{aid}/send"),
            json!({ "method": "email" }),
        ),
        (
            "POST /{id}/actions/{aid}/payments (record_payment)",
            M::Post,
            format!("/api/v1/violations/{vid}/actions/{aid}/payments"),
            json!({ "amount": "50.00" }),
        ),
        (
            "PUT /appeals/{id} (update_appeal)",
            M::Put,
            format!("/api/v1/violations/appeals/{appeal}"),
            json!({ "status": "under_review" }),
        ),
        (
            "POST /appeals/{id}/decide (decide_appeal)",
            M::Post,
            format!("/api/v1/violations/appeals/{appeal}/decide"),
            json!({ "approved": true, "decision": "Approved" }),
        ),
    ];

    for (label, method, uri, body) in cases {
        let builder = match method {
            M::Post => app.post(&uri),
            M::Put => app.put(&uri),
            M::Delete => app.delete(&uri),
        };
        let resp = app.execute(builder.bearer(&token).json(body).build()).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "privileged mutation `{label}` must reject a non-manager with 403; got {} (body: {})",
            resp.status,
            resp.text()
        );
    }
}

// ---------------------------------------------------------------------------
// Negative control for the gate itself: the role gate must NOT over-block the
// self-service actions a resident legitimately performs.
// ---------------------------------------------------------------------------

/// A resident may file an appeal and add a comment on a violation — these are
/// intentionally NOT behind `require_manager`. This guards against an
/// over-broad future gate silently locking tenants out of their own
/// self-service surface (the counterpart invariant to the 403 tests above).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_can_file_appeal_and_comment(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "authz-selfservice").await;
    let resident = seed_user(&pool, "resident-self@vio-authz.test").await;
    let violation_id = seed_violation(&pool, org, "VIO-AUTHZ-SELF-0001").await;
    let token = token_with_role(resident, org, "resident");

    let appeal = app
        .execute(
            app.post(&format!("/api/v1/violations/{violation_id}/appeals"))
                .bearer(&token)
                .json(json!({ "reason": "I was not home that evening" }))
                .build(),
        )
        .await;
    assert!(
        appeal.status.is_success(),
        "a resident must be able to file an appeal (self-service); got {} (body: {})",
        appeal.status,
        appeal.text()
    );
    assert_ne!(
        appeal.status,
        StatusCode::FORBIDDEN,
        "filing an appeal must not be behind the manager gate"
    );

    let comment = app
        .execute(
            app.post(&format!("/api/v1/violations/{violation_id}/comments"))
                .bearer(&token)
                .json(json!({ "comment_type": "general", "content": "Please review" }))
                .build(),
        )
        .await;
    assert!(
        comment.status.is_success(),
        "a resident must be able to comment (self-service); got {} (body: {})",
        comment.status,
        comment.text()
    );
    assert_ne!(
        comment.status,
        StatusCode::FORBIDDEN,
        "commenting must not be behind the manager gate"
    );
}

// ---------------------------------------------------------------------------
// Internal-notes privilege leak (issue #2944): a non-manager must never see
// staff-only (`is_internal`) comments, even when passing `?include_internal`.
// ---------------------------------------------------------------------------

async fn seed_comment(pool: &PgPool, violation_id: Uuid, author: Uuid, internal: bool) {
    sqlx::query(
        r#"INSERT INTO violation_comments (violation_id, comment_type, content, is_internal, author_id)
           VALUES ($1, 'general', $2, $3, $4)"#,
    )
    .bind(violation_id)
    .bind(if internal { "STAFF ONLY note" } else { "public note" })
    .bind(internal)
    .bind(author)
    .execute(pool)
    .await
    .expect("seed comment");
}

/// A resident requesting `?include_internal=true` must have the flag silently
/// downgraded — only the public comment comes back, never the internal one. A
/// manager asking for the same sees both. Locks in the read-side half of the
/// violations role-authz surface (privilege-leak fix #2944).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_see_internal_comments_via_include_flag(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "authz-internal").await;
    let staff = seed_user(&pool, "staff-internal@vio-authz.test").await;
    let resident = seed_user(&pool, "resident-internal@vio-authz.test").await;
    let violation_id = seed_violation(&pool, org, "VIO-AUTHZ-INT-0001").await;

    seed_comment(&pool, violation_id, staff, false).await;
    seed_comment(&pool, violation_id, staff, true).await;

    // Resident explicitly asks for internal notes — must be downgraded.
    let resident_token = token_with_role(resident, org, "resident");
    let resident_resp = app
        .execute(
            app.get(&format!(
                "/api/v1/violations/{violation_id}/comments?include_internal=true"
            ))
            .bearer(&resident_token)
            .build(),
        )
        .await;
    assert!(
        resident_resp.status.is_success(),
        "resident may read public comments; got {} (body: {})",
        resident_resp.status,
        resident_resp.text()
    );
    let body = resident_resp.text();
    assert!(
        !body.contains("STAFF ONLY note"),
        "a non-manager must NOT receive internal comments even with \
         ?include_internal=true; body leaked them: {body}"
    );
    assert!(
        body.contains("public note"),
        "resident should still see public comments; body: {body}"
    );

    // Manager asking for the same sees the internal note.
    let manager_token = token_with_role(
        seed_user(&pool, "mgr-internal@vio-authz.test").await,
        org,
        "manager",
    );
    let manager_resp = app
        .execute(
            app.get(&format!(
                "/api/v1/violations/{violation_id}/comments?include_internal=true"
            ))
            .bearer(&manager_token)
            .build(),
        )
        .await;
    assert!(
        manager_resp.status.is_success(),
        "manager may read comments; got {} (body: {})",
        manager_resp.status,
        manager_resp.text()
    );
    assert!(
        manager_resp.text().contains("STAFF ONLY note"),
        "a manager requesting internal comments must receive them; body: {}",
        manager_resp.text()
    );
}
