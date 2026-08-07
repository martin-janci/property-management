//! Happy-path (2xx) coverage for the governance `disputes.rs` endpoints
//! (BIT-408 Wave 8, governance batch 3).
//!
//! Like `violations.rs`, the dispute handlers authenticate with [`AuthUser`]:
//! org scoping comes from the JWT `tenant_id` claim and the manager/admin role
//! gates read the JWT `role` claim. We therefore mint a raw HS256 token with
//! `tenant_id = org_id` and `role = "manager"`. `file_dispute` additionally
//! takes `organization_id` in the body (set to the same org). Two users are
//! seeded so respondent/assignee FKs resolve.

#![allow(dead_code)]

use crate::common;

use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_org, TestApp, TestConfig, TestResponse};

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

fn mint_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: "disputes-hp@test.test".to_string(),
        name: "Disputes HP User".to_string(),
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
           VALUES ($1, 'test_hash', 'Disputes User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

struct Ctx {
    app: TestApp,
    org_id: Uuid,
    user_id: Uuid,
    respondent_id: Uuid,
    token: String,
}

async fn setup(pool: PgPool, tag: &str) -> Ctx {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, tag).await;
    let user_id = seed_user(
        &pool,
        &format!("{tag}-a-{}@disputes-hp.test", Uuid::new_v4()),
    )
    .await;
    let respondent_id = seed_user(
        &pool,
        &format!("{tag}-b-{}@disputes-hp.test", Uuid::new_v4()),
    )
    .await;
    let token = mint_token(user_id, org_id);
    Ctx {
        app,
        org_id,
        user_id,
        respondent_id,
        token,
    }
}

impl Ctx {
    fn get(&self, uri: &str) -> common::RequestBuilder {
        self.app.get(uri).bearer(&self.token)
    }
    fn post(&self, uri: &str, body: Value) -> common::RequestBuilder {
        self.app.post(uri).bearer(&self.token).json(body)
    }
    fn post_empty(&self, uri: &str) -> common::RequestBuilder {
        self.app.post(uri).bearer(&self.token)
    }
    fn patch(&self, uri: &str, body: Value) -> common::RequestBuilder {
        self.app.patch(uri).bearer(&self.token).json(body)
    }
    fn delete(&self, uri: &str) -> common::RequestBuilder {
        self.app.delete(uri).bearer(&self.token)
    }

    async fn file_dispute(&self) -> Uuid {
        let resp = self
            .post(
                "/api/v1/disputes",
                json!({
                    "organization_id": self.org_id,
                    "category": "noise",
                    "title": "Excessive noise",
                    "description": "Loud music late at night",
                    "respondent_ids": [self.respondent_id]
                }),
            )
            .send()
            .await;
        ok(&resp, "file_dispute");
        id_of(&resp.json_value())
    }
}

fn ok(resp: &TestResponse, what: &str) {
    assert!(
        resp.status.is_success(),
        "{what} should 2xx, got {}: {}",
        resp.status,
        resp.text()
    );
}

fn id_of(v: &Value) -> Uuid {
    v.get("id")
        .and_then(|x| x.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("expected id field in {v}"))
}

// ---------------------------------------------------------------------------
// Filing, listing, parties, evidence, submissions, resolutions, status
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn dispute_lifecycle_happy_path(pool: PgPool) {
    let ctx = setup(pool, "life").await;
    let id = ctx.file_dispute().await;
    let org = ctx.org_id;

    ok(
        &ctx.get(&format!("/api/v1/disputes?organization_id={org}"))
            .send()
            .await,
        "list_disputes",
    );
    ok(
        &ctx.get(&format!(
            "/api/v1/disputes/statistics?organization_id={org}"
        ))
        .send()
        .await,
        "get_statistics",
    );
    ok(
        &ctx.get(&format!("/api/v1/disputes/{id}")).send().await,
        "get_dispute",
    );

    // move into review so manager-only mutators / resolve transitions are legal
    ok(
        &ctx.patch(
            &format!("/api/v1/disputes/{id}"),
            json!({ "status": "under_review" }),
        )
        .send()
        .await,
        "update_dispute_status",
    );

    // parties
    ok(
        &ctx.post(
            &format!("/api/v1/disputes/{id}/parties"),
            json!({ "user_id": ctx.respondent_id, "role": "witness" }),
        )
        .send()
        .await,
        "add_party",
    );
    ok(
        &ctx.get(&format!("/api/v1/disputes/{id}/parties"))
            .send()
            .await,
        "list_parties",
    );

    // evidence
    let evidence = ctx
        .post(
            &format!("/api/v1/disputes/{id}/evidence"),
            json!({
                "filename": "dsp-evidence.pdf",
                "original_filename": "evidence.pdf",
                "content_type": "application/pdf",
                "size_bytes": 1024,
                "storage_url": "s3://bucket/dsp-evidence.pdf"
            }),
        )
        .send()
        .await;
    ok(&evidence, "add_evidence");
    let evidence_id = id_of(&evidence.json_value());
    ok(
        &ctx.get(&format!("/api/v1/disputes/{id}/evidence"))
            .send()
            .await,
        "list_evidence",
    );
    ok(
        &ctx.delete(&format!("/api/v1/disputes/{id}/evidence/{evidence_id}",))
            .send()
            .await,
        "delete_evidence",
    );

    // activities + submissions (filer is a party → submit allowed)
    ok(
        &ctx.get(&format!("/api/v1/disputes/{id}/activities"))
            .send()
            .await,
        "list_activities",
    );
    ok(
        &ctx.post(
            &format!("/api/v1/disputes/{id}/submissions"),
            json!({
                "submission_type": "response",
                "content": "Our position on the matter",
                "is_visible_to_all": true
            }),
        )
        .send()
        .await,
        "submit_response",
    );
    ok(
        &ctx.get(&format!("/api/v1/disputes/{id}/submissions"))
            .send()
            .await,
        "list_submissions",
    );
    ok(
        &ctx.get(&format!("/api/v1/disputes/{id}/mediation-case"))
            .send()
            .await,
        "get_mediation_case",
    );

    // mediation notes (manager)
    ok(
        &ctx.patch(
            &format!("/api/v1/disputes/{id}/mediation-notes"),
            json!({ "notes": "Confidential mediator notes" }),
        )
        .send()
        .await,
        "update_mediation_notes",
    );

    // resolutions
    let resolution = ctx
        .post(
            &format!("/api/v1/disputes/{id}/resolutions"),
            json!({
                "resolution_text": "Agree to quiet hours",
                "terms": [{
                    "id": "term-1",
                    "description": "Quiet 22:00-08:00",
                    "completed": false
                }]
            }),
        )
        .send()
        .await;
    ok(&resolution, "propose_resolution");
    let resolution_id = id_of(&resolution.json_value());
    ok(
        &ctx.get(&format!("/api/v1/disputes/{id}/resolutions"))
            .send()
            .await,
        "list_resolutions",
    );
    ok(
        &ctx.get(&format!(
            "/api/v1/disputes/{id}/resolutions/{resolution_id}"
        ))
        .send()
        .await,
        "get_resolution",
    );
    ok(
        &ctx.post(
            &format!("/api/v1/disputes/{id}/resolutions/{resolution_id}/vote"),
            json!({ "accepted": true }),
        )
        .send()
        .await,
        "vote_on_resolution",
    );
    ok(
        &ctx.post_empty(&format!(
            "/api/v1/disputes/{id}/resolutions/{resolution_id}/accept"
        ))
        .send()
        .await,
        "accept_resolution",
    );
    ok(
        &ctx.post_empty(&format!(
            "/api/v1/disputes/{id}/resolutions/{resolution_id}/implement"
        ))
        .send()
        .await,
        "implement_resolution",
    );

    // resolve the dispute (legal from under_review)
    ok(
        &ctx.patch(
            &format!("/api/v1/disputes/{id}/resolve"),
            json!({ "resolution_notes": "Parties reached agreement" }),
        )
        .send()
        .await,
        "resolve_dispute",
    );
}

// ---------------------------------------------------------------------------
// Action items + escalations + my/overdue dashboards
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn dispute_actions_escalations_happy_path(pool: PgPool) {
    let ctx = setup(pool, "act").await;
    let id = ctx.file_dispute().await;
    let org = ctx.org_id;

    let action = ctx
        .post(
            &format!("/api/v1/disputes/{id}/actions"),
            json!({
                "assigned_to": ctx.user_id,
                "title": "Follow up",
                "description": "Check compliance",
                "due_date": (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339()
            }),
        )
        .send()
        .await;
    ok(&action, "create_action_item");
    let action_id = id_of(&action.json_value());

    ok(
        &ctx.get(&format!("/api/v1/disputes/{id}/actions"))
            .send()
            .await,
        "list_action_items",
    );
    ok(
        &ctx.get(&format!("/api/v1/disputes/{id}/actions/{action_id}"))
            .send()
            .await,
        "get_action_item",
    );
    ok(
        &ctx.patch(
            &format!("/api/v1/disputes/{id}/actions/{action_id}"),
            json!({ "status": "in_progress" }),
        )
        .send()
        .await,
        "update_action_item",
    );
    ok(
        &ctx.post(
            &format!("/api/v1/disputes/{id}/actions/{action_id}/complete"),
            json!({ "completion_notes": "Done" }),
        )
        .send()
        .await,
        "complete_action_item",
    );
    ok(
        &ctx.post_empty(&format!("/api/v1/disputes/{id}/actions/{action_id}/remind"))
            .send()
            .await,
        "send_action_reminder",
    );

    // escalations
    let escalation = ctx
        .post(
            &format!("/api/v1/disputes/{id}/escalations"),
            json!({ "reason": "Not resolved in time", "severity": "warning" }),
        )
        .send()
        .await;
    ok(&escalation, "create_escalation");
    let escalation_id = id_of(&escalation.json_value());
    ok(
        &ctx.get(&format!("/api/v1/disputes/{id}/escalations"))
            .send()
            .await,
        "list_escalations",
    );
    ok(
        &ctx.post(
            &format!("/api/v1/disputes/{id}/escalations/{escalation_id}/resolve"),
            json!({ "resolution_notes": "Handled" }),
        )
        .send()
        .await,
        "resolve_escalation",
    );

    ok(
        &ctx.get(&format!(
            "/api/v1/disputes/my-actions?organization_id={org}"
        ))
        .send()
        .await,
        "get_my_actions",
    );
    ok(
        &ctx.get(&format!(
            "/api/v1/disputes/overdue-actions?organization_id={org}"
        ))
        .send()
        .await,
        "list_overdue_actions",
    );
}

// ---------------------------------------------------------------------------
// Mediation sessions
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn dispute_sessions_happy_path(pool: PgPool) {
    let ctx = setup(pool, "sess").await;
    let id = ctx.file_dispute().await;

    // find the complainant party id (the filer is auto-added)
    let parties = ctx
        .get(&format!("/api/v1/disputes/{id}/parties"))
        .send()
        .await;
    ok(&parties, "list_parties");
    let party_id = parties.json_value()[0]["id"]
        .as_str()
        .and_then(|s| s.parse::<Uuid>().ok())
        .expect("party id");

    let session = ctx
        .post(
            &format!("/api/v1/disputes/{id}/sessions"),
            json!({
                "session_type": "in_person",
                "scheduled_at": (chrono::Utc::now() + chrono::Duration::days(3)).to_rfc3339(),
                "duration_minutes": 60,
                "location": "Room A",
                "attendee_party_ids": [party_id]
            }),
        )
        .send()
        .await;
    ok(&session, "schedule_session");
    let session_id = id_of(&session.json_value());

    ok(
        &ctx.get(&format!("/api/v1/disputes/{id}/sessions"))
            .send()
            .await,
        "list_sessions",
    );
    ok(
        &ctx.get(&format!("/api/v1/disputes/{id}/sessions/{session_id}"))
            .send()
            .await,
        "get_session",
    );
    ok(
        &ctx.patch(
            &format!("/api/v1/disputes/{id}/sessions/{session_id}"),
            json!({ "location": "Room B" }),
        )
        .send()
        .await,
        "update_session",
    );
    ok(
        &ctx.get(&format!(
            "/api/v1/disputes/{id}/sessions/{session_id}/attendance"
        ))
        .send()
        .await,
        "get_attendance",
    );
    ok(
        &ctx.patch(
            &format!("/api/v1/disputes/{id}/sessions/{session_id}/attendance/{party_id}"),
            json!({ "confirmed": true, "attended": true }),
        )
        .send()
        .await,
        "update_attendance",
    );
    ok(
        &ctx.post(
            &format!("/api/v1/disputes/{id}/sessions/{session_id}/notes"),
            json!({ "notes": "Session complete", "outcome": "agreed" }),
        )
        .send()
        .await,
        "record_notes",
    );
    ok(
        &ctx.post_empty(&format!(
            "/api/v1/disputes/{id}/sessions/{session_id}/cancel"
        ))
        .send()
        .await,
        "cancel_session",
    );
}

// ---------------------------------------------------------------------------
// KPIs reporting endpoint (issue #2562 — wire orphaned get_dispute_kpis)
// ---------------------------------------------------------------------------

/// GET /api/v1/disputes/kpis returns the DisputeKpis JSON contract for the
/// caller's org cohort. Files one dispute inside the window and asserts the
/// full funnel + TTR shape, plus that the filed dispute is counted.
///
/// Issue #2575: this `#[sqlx::test]` DB-contract case remains under the
/// systemic BIT-440 dev-hostage quarantine (un-quarantining it in isolation,
/// while BIT-440 is unresolved, reintroduces the failing test that PR #1984
/// quarantined to unblock the gate — see the BIT-417 de-quarantine precedent,
/// which only lifted the marker once BIT-440 cleared). It must be
/// de-quarantined together with the rest of the BIT-440 suite when that
/// ticket clears. The endpoint's *executing* coverage now lives in
/// `dispute_kpis_rejects_inverted_window` below, which is a plain
/// `#[tokio::test]` outside BIT-440 scope.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440 (de-quarantine tracked by #2575 when BIT-440 clears)"]
async fn dispute_kpis_happy_path(pool: PgPool) {
    let ctx = setup(pool, "kpis").await;
    // Seed a cohort member (created_at defaults to NOW(), inside the window).
    let _id = ctx.file_dispute().await;

    // Window brackets NOW(); Z-suffixed RFC3339 avoids `+00:00` query encoding.
    let start = (chrono::Utc::now() - chrono::Duration::days(1))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let end = (chrono::Utc::now() + chrono::Duration::days(1))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let resp = ctx
        .get(&format!(
            "/api/v1/disputes/kpis?window_start={start}&window_end={end}"
        ))
        .send()
        .await;
    ok(&resp, "get_kpis");

    let v = resp.json_value();

    // Top-level contract.
    assert!(v.get("window_start").is_some(), "window_start present: {v}");
    assert!(v.get("window_end").is_some(), "window_end present: {v}");

    // Funnel sub-object contract.
    let funnel = v.get("funnel").expect("funnel object present");
    for key in [
        "filed",
        "reached_mediation",
        "reached_resolved",
        "currently_in_mediation",
        "mediation_rate",
        "resolution_rate",
    ] {
        assert!(funnel.get(key).is_some(), "funnel.{key} present: {funnel}");
    }
    let filed = funnel["filed"].as_i64().expect("filed is an integer");
    assert!(
        filed >= 1,
        "filed cohort should include our dispute: {funnel}"
    );

    // TTR sub-object contract.
    let ttr = v.get("ttr").expect("ttr object present");
    for key in [
        "count",
        "p50_hours",
        "p90_hours",
        "p95_hours",
        "mean_hours",
        "max_hours",
    ] {
        assert!(ttr.get(key).is_some(), "ttr.{key} present: {ttr}");
    }
    assert!(
        ttr["count"].as_i64().is_some(),
        "ttr.count is an integer: {ttr}"
    );

    // Issue #2575: an inverted window (start after end) is rejected with a
    // 422 VALIDATION_ERROR rather than silently returning a degenerate
    // zero-count 200. Asserted here too so the DB-contract case covers the
    // validation branch once BIT-440 clears and this test runs.
    let inverted = ctx
        .get(&format!(
            "/api/v1/disputes/kpis?window_start={end}&window_end={start}"
        ))
        .send()
        .await;
    assert_eq!(
        inverted.status.as_u16(),
        422,
        "inverted window should be rejected with 422, got {}: {}",
        inverted.status,
        inverted.text()
    );
    assert_eq!(
        inverted.json_value().get("code").and_then(|c| c.as_str()),
        Some("VALIDATION_ERROR"),
        "inverted-window error code should be VALIDATION_ERROR: {}",
        inverted.text()
    );
}

/// Validation guard (issue #2575): an inverted or empty reporting window
/// (`window_start >= window_end`) is rejected with `422 VALIDATION_ERROR`
/// *before* the repository is consulted, so a caller gets an explicit error
/// instead of a degenerate zero-count `DisputeKpis` with a `200`.
///
/// This is deliberately a plain `#[tokio::test]`, not `#[sqlx::test]`: the
/// validation branch returns before any DB access (`require_org` reads the JWT
/// claim, then the ordering check fires), so it needs no live Postgres and is
/// therefore NOT subject to the BIT-440 dev-hostage quarantine that gates the
/// `#[sqlx::test]` contract case above. It runs on every CI/dev `cargo test`,
/// giving the endpoint executing coverage of its validation contract. The
/// lazy pool never connects — the same pattern the `push_fanout.rs` /
/// `voice_commands.rs` pure-path unit tests use.
#[tokio::test]
async fn dispute_kpis_rejects_inverted_window() {
    let pool = sqlx::PgPool::connect_lazy("postgres://never-used:never@127.0.0.1:1/never")
        .expect("connect_lazy builds without connecting");
    let app = TestApp::new(pool).await;
    let token = mint_token(Uuid::new_v4(), Uuid::new_v4());

    // Z-suffixed RFC3339 avoids `+00:00` query encoding (mirrors the happy path).
    let later = (chrono::Utc::now() + chrono::Duration::days(1))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let earlier = (chrono::Utc::now() - chrono::Duration::days(1))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    // Inverted bounds: window_start strictly AFTER window_end.
    let inverted = app
        .get(&format!(
            "/api/v1/disputes/kpis?window_start={later}&window_end={earlier}"
        ))
        .bearer(&token)
        .send()
        .await;
    assert_eq!(
        inverted.status.as_u16(),
        422,
        "inverted window should be rejected with 422, got {}: {}",
        inverted.status,
        inverted.text()
    );
    assert_eq!(
        inverted.json_value().get("code").and_then(|c| c.as_str()),
        Some("VALIDATION_ERROR"),
        "error code should be VALIDATION_ERROR: {}",
        inverted.text()
    );

    // Empty bounds: `[t, t)` is empty, so equal start/end is rejected too.
    let same = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let empty = app
        .get(&format!(
            "/api/v1/disputes/kpis?window_start={same}&window_end={same}"
        ))
        .bearer(&token)
        .send()
        .await;
    assert_eq!(
        empty.status.as_u16(),
        422,
        "empty window (start == end) should be rejected with 422, got {}: {}",
        empty.status,
        empty.text()
    );
}

// ---------------------------------------------------------------------------
// Withdraw (separate dispute so it doesn't collide with the lifecycle one)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn dispute_withdraw_happy_path(pool: PgPool) {
    let ctx = setup(pool, "wd").await;
    let id = ctx.file_dispute().await;
    ok(
        &ctx.delete(&format!("/api/v1/disputes/{id}")).send().await,
        "withdraw_dispute",
    );
}
