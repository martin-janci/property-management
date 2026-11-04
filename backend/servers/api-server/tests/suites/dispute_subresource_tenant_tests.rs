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
