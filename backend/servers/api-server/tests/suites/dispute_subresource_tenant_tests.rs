//! Tenant-isolation tests for the dispute sub-resource routes: mediation
//! sessions, submissions, resolutions (Story 77.3), action items and
//! escalations (Story 77.4).
//!
//! These handlers used to address rows by the path ids only and ignore the
//! caller's JWT tenant, so any authenticated user could read or mutate another
//! org's resolutions / action items / escalations by guessing UUIDs, or pair
//! one of their own disputes with a foreign child id. They now verify that the
//! dispute belongs to the caller's org and that the child belongs to that
//! dispute; every mismatch is 404 (not 403) so ids are not an existence oracle.
//! Accepting / implementing a resolution additionally requires manager/admin.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig, TestResponse};

#[derive(Serialize)]
struct Claims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint_token(user_id: Uuid, org_id: Uuid, role: &str) -> String {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some(role.to_string()),
        email: "dispute-sub@test.local".to_string(),
        name: "Dispute Sub Tester".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(TestConfig::default().jwt_secret.as_bytes()),
    )
    .expect("mint token")
}

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Dispute Sub User', 'active', NOW()) RETURNING id"#,
    )
    .bind(format!("{tag}-{}@dispute-sub.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// One org with a dispute and one of each sub-resource.
struct OrgFixture {
    org: Uuid,
    user: Uuid,
    dispute: Uuid,
    resolution: Uuid,
    action: Uuid,
    escalation: Uuid,
    session: Uuid,
}

async fn seed_fixture(pool: &PgPool, tag: &str) -> OrgFixture {
    let org = seed_org(pool, &format!("dsub-{tag}")).await;
    let user = seed_user(pool, tag).await;
    seed_membership(pool, org, user, "manager").await;

    let dispute = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO disputes (organization_id, reference_number, category, title,
                                 description, status, priority, filed_by)
           VALUES ($1, $2, 'noise', 'Dispute', 'Desc', 'under_review', 'medium', $3)
           RETURNING id"#,
    )
    .bind(org)
    .bind(format!("DSUB-{}", Uuid::new_v4()))
    .bind(user)
    .fetch_one(pool)
    .await
    .expect("seed dispute");

    // The filer is a party so vote/submit paths can resolve a party row.
    sqlx::query(
        "INSERT INTO dispute_parties (dispute_id, user_id, role) VALUES ($1, $2, 'complainant')",
    )
    .bind(dispute)
    .bind(user)
    .execute(pool)
    .await
    .expect("seed party");

    let resolution = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO dispute_resolutions (dispute_id, proposed_by, resolution_text)
           VALUES ($1, $2, 'Quiet hours after 22:00') RETURNING id"#,
    )
    .bind(dispute)
    .bind(user)
    .fetch_one(pool)
    .await
    .expect("seed resolution");

    let action = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO action_items (dispute_id, assigned_to, title, description, due_date)
           VALUES ($1, $2, 'Install mat', 'Sound mat', NOW() + INTERVAL '7 days')
           RETURNING id"#,
    )
    .bind(dispute)
    .bind(user)
    .fetch_one(pool)
    .await
    .expect("seed action item");

    let escalation = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO escalations (dispute_id, escalated_by, reason, severity)
           VALUES ($1, $2, 'Ignored', 'medium') RETURNING id"#,
    )
    .bind(dispute)
    .bind(user)
    .fetch_one(pool)
    .await
    .expect("seed escalation");

    let session = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO mediation_sessions (dispute_id, mediator_id, session_type, scheduled_at)
           VALUES ($1, $2, 'in_person', NOW() + INTERVAL '3 days') RETURNING id"#,
    )
    .bind(dispute)
    .bind(user)
    .fetch_one(pool)
    .await
    .expect("seed session");

    OrgFixture {
        org,
        user,
        dispute,
        resolution,
        action,
        escalation,
        session,
    }
}

async fn call(
    app: &TestApp,
    token: &str,
    org: Uuid,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> TestResponse {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string());
    let req = match body {
        Some(b) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    app.execute(req).await
}

async fn scalar_text(pool: &PgPool, sql: &'static str, id: Uuid) -> String {
    sqlx::query_scalar::<_, String>(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("scalar")
}

/// Every resolution / action-item / escalation / session route addressed at
/// org A's dispute by an org-B manager is 404 and mutates nothing.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_sub_resource_access_is_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let a = seed_fixture(&pool, "a").await;
    let b = seed_fixture(&pool, "b").await;
    let token_b = mint_token(b.user, b.org, "manager");

    let d = a.dispute;
    let due = (Utc::now() + Duration::days(5)).to_rfc3339();
    let cases: Vec<(Method, String, Option<Value>)> = vec![
        // Mediation (Story 77.2)
        (Method::GET, format!("/api/v1/disputes/{d}/sessions"), None),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/sessions/{}", a.session),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/sessions/{}/cancel", a.session),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/sessions/{}/attendance", a.session),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/submissions"),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/mediation-case"),
            None,
        ),
        // Resolutions (Story 77.3)
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/resolutions"),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/resolutions"),
            Some(json!({
                "dispute_id": d, "proposed_by": b.user,
                "resolution_text": "foreign", "terms": []
            })),
        ),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/resolutions/{}", a.resolution),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/resolutions/{}/vote", a.resolution),
            Some(json!({ "accepted": true })),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/resolutions/{}/accept", a.resolution),
            None,
        ),
        (
            Method::POST,
            format!(
                "/api/v1/disputes/{d}/resolutions/{}/implement",
                a.resolution
            ),
            None,
        ),
        // Action items (Story 77.4)
        (Method::GET, format!("/api/v1/disputes/{d}/actions"), None),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/actions"),
            Some(json!({
                "dispute_id": d, "assigned_to": b.user, "title": "x",
                "description": "x", "due_date": due
            })),
        ),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/actions/{}", a.action),
            None,
        ),
        (
            Method::PATCH,
            format!("/api/v1/disputes/{d}/actions/{}", a.action),
            Some(json!({ "status": "completed" })),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/actions/{}/complete", a.action),
            Some(json!({ "completion_notes": "done" })),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/actions/{}/remind", a.action),
            None,
        ),
        // Escalations (Story 77.4)
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/escalations"),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/escalations"),
            Some(json!({ "reason": "x", "severity": "high" })),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/escalations/{}/resolve", a.escalation),
            Some(json!({ "resolution_notes": "closed" })),
        ),
    ];

    for (method, uri, body) in cases {
        let resp = call(&app, &token_b, b.org, method.clone(), &uri, body).await;
        assert_eq!(
            resp.status,
            StatusCode::NOT_FOUND,
            "{method} {uri} cross-org must be 404, got {}: {}",
            resp.status,
            resp.text()
        );
    }

    assert_eq!(
        scalar_text(
            &pool,
            "SELECT status FROM dispute_resolutions WHERE id = $1",
            a.resolution
        )
        .await,
        "proposed"
    );
    assert_eq!(
        scalar_text(
            &pool,
            "SELECT status FROM action_items WHERE id = $1",
            a.action
        )
        .await,
        "pending"
    );
    assert_eq!(
        scalar_text(
            &pool,
            "SELECT status FROM mediation_sessions WHERE id = $1",
            a.session
        )
        .await,
        "scheduled"
    );
    let esc_resolved: bool = sqlx::query_scalar("SELECT resolved FROM escalations WHERE id = $1")
        .bind(a.escalation)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!esc_resolved, "foreign escalation must not be resolved");
    let foreign_rows: i64 = sqlx::query_scalar(
        "SELECT (SELECT COUNT(*) FROM dispute_resolutions WHERE dispute_id = $1) \
              + (SELECT COUNT(*) FROM action_items WHERE dispute_id = $1) \
              + (SELECT COUNT(*) FROM escalations WHERE dispute_id = $1)",
    )
    .bind(a.dispute)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(foreign_rows, 3, "no rows may be added to a foreign dispute");
}

/// Pairing the caller's OWN dispute id with a foreign child id is also 404:
/// the child must belong to the dispute in the path, and body-linked ids
/// (`resolution_id` on action items, `action_item_id` on escalations) too.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn own_dispute_with_foreign_child_id_is_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let a = seed_fixture(&pool, "ma").await;
    let b = seed_fixture(&pool, "mb").await;
    let token_b = mint_token(b.user, b.org, "manager");
    let d = b.dispute;
    let due = (Utc::now() + Duration::days(5)).to_rfc3339();

    let cases: Vec<(Method, String, Option<Value>)> = vec![
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/resolutions/{}/accept", a.resolution),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/resolutions/{}", a.resolution),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/actions/{}/complete", a.action),
            Some(json!({ "completion_notes": "done" })),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/escalations/{}/resolve", a.escalation),
            Some(json!({ "resolution_notes": "closed" })),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/sessions/{}/cancel", a.session),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/escalations"),
            Some(json!({ "reason": "x", "severity": "high", "action_item_id": a.action })),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/actions"),
            Some(json!({
                "dispute_id": d, "resolution_id": a.resolution, "assigned_to": b.user,
                "title": "x", "description": "x", "due_date": due
            })),
        ),
    ];

    for (method, uri, body) in cases {
        let resp = call(&app, &token_b, b.org, method.clone(), &uri, body).await;
        assert_eq!(
            resp.status,
            StatusCode::NOT_FOUND,
            "{method} {uri} with foreign child id must be 404, got {}: {}",
            resp.status,
            resp.text()
        );
    }

    assert_eq!(
        scalar_text(
            &pool,
            "SELECT status FROM dispute_resolutions WHERE id = $1",
            a.resolution
        )
        .await,
        "proposed"
    );
    assert_eq!(
        scalar_text(
            &pool,
            "SELECT status FROM action_items WHERE id = $1",
            a.action
        )
        .await,
        "pending"
    );
}

/// Same-org callers keep working on every guarded route family.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn same_org_sub_resource_access_works(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let a = seed_fixture(&pool, "s").await;
    let token = mint_token(a.user, a.org, "manager");
    let d = a.dispute;

    let ok_cases: Vec<(Method, String, Option<Value>)> = vec![
        (Method::GET, format!("/api/v1/disputes/{d}/sessions"), None),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/sessions/{}", a.session),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/submissions"),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/resolutions"),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/resolutions/{}", a.resolution),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/resolutions/{}/vote", a.resolution),
            Some(json!({ "accepted": true })),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/resolutions/{}/accept", a.resolution),
            None,
        ),
        (
            Method::POST,
            format!(
                "/api/v1/disputes/{d}/resolutions/{}/implement",
                a.resolution
            ),
            None,
        ),
        (Method::GET, format!("/api/v1/disputes/{d}/actions"), None),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/actions/{}", a.action),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/escalations"),
            Some(json!({ "reason": "late", "severity": "high", "action_item_id": a.action })),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/actions/{}/complete", a.action),
            Some(json!({ "completion_notes": "done" })),
        ),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/escalations"),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/escalations/{}/resolve", a.escalation),
            Some(json!({ "resolution_notes": "closed" })),
        ),
    ];

    for (method, uri, body) in ok_cases {
        let resp = call(&app, &token, a.org, method.clone(), &uri, body).await;
        assert!(
            resp.status.is_success(),
            "{method} {uri} same-org should 2xx, got {}: {}",
            resp.status,
            resp.text()
        );
    }

    assert_eq!(
        scalar_text(
            &pool,
            "SELECT status FROM dispute_resolutions WHERE id = $1",
            a.resolution
        )
        .await,
        "implemented"
    );
    assert_eq!(
        scalar_text(
            &pool,
            "SELECT status FROM action_items WHERE id = $1",
            a.action
        )
        .await,
        "completed"
    );
}

/// Accepting / implementing a resolution is manager/admin only.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accept_and_implement_resolution_require_manager(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let a = seed_fixture(&pool, "r").await;
    let resident = seed_user(&pool, "resident").await;
    seed_membership(&pool, a.org, resident, "resident").await;
    let token = mint_token(resident, a.org, "resident");
    let d = a.dispute;

    for action in ["accept", "implement"] {
        let uri = format!("/api/v1/disputes/{d}/resolutions/{}/{action}", a.resolution);
        let resp = call(&app, &token, a.org, Method::POST, &uri, None).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{action} by a resident must be 403, got {}: {}",
            resp.status,
            resp.text()
        );
    }
    assert_eq!(
        scalar_text(
            &pool,
            "SELECT status FROM dispute_resolutions WHERE id = $1",
            a.resolution
        )
        .await,
        "proposed"
    );
}

// =============================================================================
// Role rules within the owning org (sessions, action items, escalations)
// =============================================================================

/// Seed an org member with `member_role` and optionally add them to the
/// fixture's dispute as a party with `party_role`. Returns the user and token.
async fn seed_actor(
    pool: &PgPool,
    f: &OrgFixture,
    tag: &str,
    member_role: &str,
    party_role: Option<&str>,
) -> (Uuid, String) {
    let user = seed_user(pool, tag).await;
    seed_membership(pool, f.org, user, member_role).await;
    if let Some(role) = party_role {
        sqlx::query("INSERT INTO dispute_parties (dispute_id, user_id, role) VALUES ($1, $2, $3)")
            .bind(f.dispute)
            .bind(user)
            .bind(role)
            .execute(pool)
            .await
            .expect("seed party");
    }
    (user, mint_token(user, f.org, member_role))
}

fn session_body() -> Value {
    json!({
        "session_type": "in_person",
        "scheduled_at": (Utc::now() + Duration::days(2)).to_rfc3339(),
    })
}

fn action_body(assignee: Uuid) -> Value {
    json!({
        "assigned_to": assignee,
        "title": "Fix the leak",
        "description": "Within a week",
        "due_date": (Utc::now() + Duration::days(7)).to_rfc3339(),
    })
}

async fn count(pool: &PgPool, sql: &'static str, id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("count")
}

/// A same-org resident who is not a party to the dispute cannot schedule or
/// read sessions, create action items, escalate or resolve escalations.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_non_party_is_403_on_mediation_writes(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let f = seed_fixture(&pool, "np").await;
    let (_, token) = seed_actor(&pool, &f, "np-res", "resident", None).await;
    let d = f.dispute;

    let cases: Vec<(Method, String, Option<Value>)> = vec![
        (Method::GET, format!("/api/v1/disputes/{d}/sessions"), None),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/sessions/{}", f.session),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/disputes/{d}/sessions/{}/attendance", f.session),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/sessions"),
            Some(session_body()),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/sessions/{}/cancel", f.session),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/actions"),
            Some(action_body(f.user)),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/actions/{}/complete", f.action),
            Some(json!({ "completion_notes": "x" })),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/escalations"),
            Some(json!({ "reason": "late", "severity": "warning" })),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/escalations/{}/resolve", f.escalation),
            Some(json!({ "resolution_notes": "closed" })),
        ),
    ];
    for (method, uri, body) in cases {
        let resp = call(&app, &token, f.org, method.clone(), &uri, body).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{method} {uri} by non-party resident must be 403, got {}: {}",
            resp.status,
            resp.text()
        );
    }

    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM mediation_sessions WHERE dispute_id = $1",
            d
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM action_items WHERE dispute_id = $1",
            d
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM escalations WHERE dispute_id = $1",
            d
        )
        .await,
        1
    );
}

/// The dispute's mediator (a resident by org role) may schedule sessions,
/// record notes and create action items.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mediator_can_schedule_and_create_actions(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let f = seed_fixture(&pool, "med").await;
    let (mediator, token) = seed_actor(&pool, &f, "med", "resident", Some("mediator")).await;
    let d = f.dispute;

    let resp = call(
        &app,
        &token,
        f.org,
        Method::POST,
        &format!("/api/v1/disputes/{d}/sessions"),
        Some(session_body()),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let session: Value = resp.json();
    assert_eq!(session["mediator_id"], json!(mediator));
    assert_eq!(session["dispute_id"], json!(d));

    let resp = call(
        &app,
        &token,
        f.org,
        Method::POST,
        &format!("/api/v1/disputes/{d}/sessions/{}/notes", f.session),
        Some(json!({ "notes": "Both parties attended" })),
    )
    .await;
    assert!(resp.status.is_success(), "{}: {}", resp.status, resp.text());

    let resp = call(
        &app,
        &token,
        f.org,
        Method::POST,
        &format!("/api/v1/disputes/{d}/actions"),
        Some(action_body(f.user)),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());

    // Mediator may read sessions too.
    let resp = call(
        &app,
        &token,
        f.org,
        Method::GET,
        &format!("/api/v1/disputes/{d}/sessions"),
        None,
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());

    // ... but cannot resolve escalations (manager only).
    let resp = call(
        &app,
        &token,
        f.org,
        Method::POST,
        &format!("/api/v1/disputes/{d}/escalations/{}/resolve", f.escalation),
        Some(json!({ "resolution_notes": "closed" })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
}

/// A non-mediator party may read sessions and escalate, but may not schedule
/// sessions, create action items or resolve escalations.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn party_can_escalate_but_not_resolve(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let f = seed_fixture(&pool, "pty").await;
    let (party, token) = seed_actor(&pool, &f, "pty", "resident", Some("respondent")).await;
    let d = f.dispute;

    let resp = call(
        &app,
        &token,
        f.org,
        Method::GET,
        &format!("/api/v1/disputes/{d}/sessions"),
        None,
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());

    let resp = call(
        &app,
        &token,
        f.org,
        Method::POST,
        &format!("/api/v1/disputes/{d}/escalations"),
        Some(json!({ "reason": "not complied", "severity": "warning" })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let esc: Value = resp.json();
    assert_eq!(esc["escalated_by"], json!(party));
    let esc_id = esc["id"].as_str().expect("escalation id").to_string();

    for (method, uri, body) in [
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/escalations/{esc_id}/resolve"),
            Some(json!({ "resolution_notes": "closed" })),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/sessions"),
            Some(session_body()),
        ),
        (
            Method::POST,
            format!("/api/v1/disputes/{d}/actions"),
            Some(action_body(party)),
        ),
    ] {
        let resp = call(&app, &token, f.org, method.clone(), &uri, body).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{method} {uri} by party must be 403, got {}: {}",
            resp.status,
            resp.text()
        );
    }
}

/// The assignee may complete their own action item (but not update it); a
/// different resident may not complete it.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn assignee_can_complete_own_action_item(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let f = seed_fixture(&pool, "asg").await;
    let (assignee, token) = seed_actor(&pool, &f, "asg", "resident", Some("respondent")).await;
    let (_, other_token) = seed_actor(&pool, &f, "asg-o", "resident", Some("witness")).await;
    let d = f.dispute;

    let item = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO action_items (dispute_id, assigned_to, title, description, due_date)
           VALUES ($1, $2, 'Repaint', 'Hallway', NOW() + INTERVAL '7 days') RETURNING id"#,
    )
    .bind(d)
    .bind(assignee)
    .fetch_one(&pool)
    .await
    .expect("seed action item");

    let complete = format!("/api/v1/disputes/{d}/actions/{item}/complete");
    let resp = call(
        &app,
        &other_token,
        f.org,
        Method::POST,
        &complete,
        Some(json!({ "completion_notes": "not mine" })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    let resp = call(
        &app,
        &token,
        f.org,
        Method::PATCH,
        &format!("/api/v1/disputes/{d}/actions/{item}"),
        Some(json!({ "title": "Changed" })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    let resp = call(
        &app,
        &token,
        f.org,
        Method::POST,
        &complete,
        Some(json!({ "completion_notes": "done" })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    assert_eq!(
        scalar_text(&pool, "SELECT status FROM action_items WHERE id = $1", item).await,
        "completed"
    );
}

/// `assigned_to` / `escalated_to` must be active members of the caller's org.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn foreign_org_assignee_or_escalation_target_is_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let a = seed_fixture(&pool, "fa").await;
    let b = seed_fixture(&pool, "fb").await;
    let token = mint_token(a.user, a.org, "manager");
    let d = a.dispute;

    let resp = call(
        &app,
        &token,
        a.org,
        Method::POST,
        &format!("/api/v1/disputes/{d}/actions"),
        Some(action_body(b.user)),
    )
    .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    assert_eq!(resp.json::<Value>()["code"], "INVALID_ASSIGNEE");

    let resp = call(
        &app,
        &token,
        a.org,
        Method::POST,
        &format!("/api/v1/disputes/{d}/escalations"),
        Some(json!({ "reason": "x", "severity": "warning", "escalated_to": b.user })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    assert_eq!(resp.json::<Value>()["code"], "INVALID_ESCALATION_TARGET");

    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM action_items WHERE dispute_id = $1",
            d
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM escalations WHERE dispute_id = $1",
            d
        )
        .await,
        1
    );

    // A same-org member is accepted.
    let resp = call(
        &app,
        &token,
        a.org,
        Method::POST,
        &format!("/api/v1/disputes/{d}/escalations"),
        Some(json!({ "reason": "x", "severity": "warning", "escalated_to": a.user })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
}

/// `dispute_id` / `proposed_by` come from the path / JWT: bodies without them
/// succeed, and bodies that still carry them (older clients) are not rejected
/// and cannot override them.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn bodies_without_path_or_jwt_fields_are_accepted(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let f = seed_fixture(&pool, "dto").await;
    let other = seed_fixture(&pool, "dto-o").await;
    let token = mint_token(f.user, f.org, "manager");
    let d = f.dispute;

    let resp = call(
        &app,
        &token,
        f.org,
        Method::POST,
        &format!("/api/v1/disputes/{d}/resolutions"),
        Some(json!({
            "resolution_text": "Quiet hours",
            "terms": [{ "id": "term-1", "description": "No noise after 22:00", "completed": false }],
        })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let r: Value = resp.json();
    assert_eq!(r["dispute_id"], json!(d));
    assert_eq!(r["proposed_by"], json!(f.user));

    let resp = call(
        &app,
        &token,
        f.org,
        Method::POST,
        &format!("/api/v1/disputes/{d}/actions"),
        Some(action_body(f.user)),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    assert_eq!(resp.json::<Value>()["dispute_id"], json!(d));

    // Legacy shape (frontend sends placeholders) — extra fields ignored.
    let resp = call(
        &app,
        &token,
        f.org,
        Method::POST,
        &format!("/api/v1/disputes/{d}/resolutions"),
        Some(json!({
            "dispute_id": other.dispute,
            "proposed_by": "00000000-0000-0000-0000-000000000000",
            "resolution_text": "Second proposal",
            "terms": [],
        })),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let r: Value = resp.json();
    assert_eq!(r["dispute_id"], json!(d));
    assert_eq!(r["proposed_by"], json!(f.user));

    let mut legacy_action = action_body(f.user);
    legacy_action["dispute_id"] = json!(other.dispute);
    let resp = call(
        &app,
        &token,
        f.org,
        Method::POST,
        &format!("/api/v1/disputes/{d}/actions"),
        Some(legacy_action),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    assert_eq!(resp.json::<Value>()["dispute_id"], json!(d));
    assert_eq!(
        count(
            &pool,
            "SELECT COUNT(*) FROM action_items WHERE dispute_id = $1",
            other.dispute
        )
        .await,
        1
    );
}
