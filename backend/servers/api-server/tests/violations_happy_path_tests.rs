//! Happy-path (2xx) coverage for the governance `violations.rs` endpoints
//! (BIT-408 Wave 8, governance batch 3).
//!
//! `violations.rs` handlers authenticate with [`AuthUser`] and read the org
//! straight off the JWT `tenant_id` claim (no `organization_members` lookup),
//! so — like `violations_cross_org_idor_tests.rs` — we mint a raw HS256 token
//! carrying `tenant_id = org_id` and `role = "manager"`. The signing user is
//! seeded into `users` so `reporter_id`/`assigned_to` FKs resolve.

#![allow(dead_code)]

mod common;

use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_org, TestApp, TestConfig, TestResponse};

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

fn mint_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: "violations-hp@test.test".to_string(),
        name: "Violations HP User".to_string(),
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
           VALUES ($1, 'test_hash', 'Violations User', 'active', NOW()) RETURNING id"#,
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
    token: String,
}

async fn setup(pool: PgPool, tag: &str) -> Ctx {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, tag).await;
    let email = format!("{tag}-{}@violations-hp.test", Uuid::new_v4());
    let user_id = seed_user(&pool, &email).await;
    let token = mint_token(user_id, org_id);
    Ctx {
        app,
        org_id,
        user_id,
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
    fn put(&self, uri: &str, body: Value) -> common::RequestBuilder {
        self.app.put(uri).bearer(&self.token).json(body)
    }
    fn delete(&self, uri: &str) -> common::RequestBuilder {
        self.app.delete(uri).bearer(&self.token)
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
// Community rules CRUD
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn violation_rules_crud_happy_path(pool: PgPool) {
    let ctx = setup(pool, "rules").await;

    let create = ctx
        .post(
            "/api/v1/violations/rules",
            json!({
                "rule_code": "NOISE-001",
                "title": "Quiet hours after 22:00",
                "category": "noise"
            }),
        )
        .send()
        .await;
    ok(&create, "create_rule");
    let rule_id = id_of(&create.json_value());

    ok(&ctx.get("/api/v1/violations/rules").send().await, "list_rules");
    ok(
        &ctx.get(&format!("/api/v1/violations/rules/{rule_id}"))
            .send()
            .await,
        "get_rule",
    );
    ok(
        &ctx.put(
            &format!("/api/v1/violations/rules/{rule_id}"),
            json!({ "is_active": false }),
        )
        .send()
        .await,
        "update_rule",
    );
    ok(
        &ctx.delete(&format!("/api/v1/violations/rules/{rule_id}"))
            .send()
            .await,
        "delete_rule",
    );
}

// ---------------------------------------------------------------------------
// Violations + evidence + enforcement actions + payments + appeals + comments
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn violations_full_flow_happy_path(pool: PgPool) {
    let ctx = setup(pool, "vio").await;

    // create + read + update + assign
    let create = ctx
        .post(
            "/api/v1/violations",
            json!({
                "category": "noise",
                "title": "Loud music at night",
                "description": "Music past midnight",
                "occurred_at": chrono::Utc::now().to_rfc3339()
            }),
        )
        .send()
        .await;
    ok(&create, "create_violation");
    let violation_id = id_of(&create.json_value());

    ok(&ctx.get("/api/v1/violations").send().await, "list_violations");
    ok(
        &ctx.get(&format!("/api/v1/violations/{violation_id}"))
            .send()
            .await,
        "get_violation",
    );
    ok(
        &ctx.put(
            &format!("/api/v1/violations/{violation_id}"),
            json!({ "status": "confirmed" }),
        )
        .send()
        .await,
        "update_violation",
    );
    ok(
        &ctx.post(
            &format!("/api/v1/violations/{violation_id}/assign"),
            json!({ "assigned_to": ctx.user_id }),
        )
        .send()
        .await,
        "assign_violation",
    );

    // evidence
    let evidence = ctx
        .post(
            &format!("/api/v1/violations/{violation_id}/evidence"),
            json!({ "file_name": "photo.jpg", "file_type": "image/jpeg" }),
        )
        .send()
        .await;
    ok(&evidence, "add_evidence");
    ok(
        &ctx.get(&format!("/api/v1/violations/{violation_id}/evidence"))
            .send()
            .await,
        "list_evidence",
    );

    // enforcement action + payment
    let action = ctx
        .post(
            &format!("/api/v1/violations/{violation_id}/actions"),
            json!({ "action_type": "first_fine", "fine_amount": "100.00" }),
        )
        .send()
        .await;
    ok(&action, "create_action");
    let action_id = id_of(&action.json_value());

    ok(
        &ctx.get(&format!("/api/v1/violations/{violation_id}/actions"))
            .send()
            .await,
        "list_actions",
    );
    ok(
        &ctx.get(&format!(
            "/api/v1/violations/{violation_id}/actions/{action_id}"
        ))
        .send()
        .await,
        "get_action",
    );
    ok(
        &ctx.put(
            &format!("/api/v1/violations/{violation_id}/actions/{action_id}"),
            json!({ "status": "sent" }),
        )
        .send()
        .await,
        "update_action",
    );
    ok(
        &ctx.post(
            &format!("/api/v1/violations/{violation_id}/actions/{action_id}/send"),
            json!({ "method": "email" }),
        )
        .send()
        .await,
        "mark_action_sent",
    );
    ok(
        &ctx.post(
            &format!("/api/v1/violations/{violation_id}/actions/{action_id}/payments"),
            json!({ "amount": "50.00" }),
        )
        .send()
        .await,
        "record_payment",
    );
    ok(
        &ctx.get(&format!(
            "/api/v1/violations/{violation_id}/actions/{action_id}/payments"
        ))
        .send()
        .await,
        "list_payments",
    );

    // comments
    ok(
        &ctx.post(
            &format!("/api/v1/violations/{violation_id}/comments"),
            json!({ "comment_type": "note", "content": "Investigated" }),
        )
        .send()
        .await,
        "add_comment",
    );
    ok(
        &ctx.get(&format!("/api/v1/violations/{violation_id}/comments"))
            .send()
            .await,
        "list_comments",
    );

    // appeals
    let appeal = ctx
        .post(
            &format!("/api/v1/violations/{violation_id}/appeals"),
            json!({ "reason": "I dispute this violation" }),
        )
        .send()
        .await;
    ok(&appeal, "create_appeal");
    let appeal_id = id_of(&appeal.json_value());

    ok(
        &ctx.get("/api/v1/violations/appeals").send().await,
        "list_appeals",
    );
    ok(
        &ctx.get(&format!("/api/v1/violations/appeals/{appeal_id}"))
            .send()
            .await,
        "get_appeal",
    );
    ok(
        &ctx.put(
            &format!("/api/v1/violations/appeals/{appeal_id}"),
            json!({ "status": "under_review" }),
        )
        .send()
        .await,
        "update_appeal",
    );
    ok(
        &ctx.post(
            &format!("/api/v1/violations/appeals/{appeal_id}/decide"),
            json!({ "approved": false, "decision": "Appeal denied" }),
        )
        .send()
        .await,
        "decide_appeal",
    );
}

// ---------------------------------------------------------------------------
// Dashboards / reporting
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn violations_dashboards_happy_path(pool: PgPool) {
    let ctx = setup(pool, "dash").await;

    ok(
        &ctx.get("/api/v1/violations/dashboard").send().await,
        "get_dashboard",
    );
    ok(
        &ctx.get("/api/v1/violations/history").send().await,
        "get_violator_history",
    );
    ok(
        &ctx.get("/api/v1/violations/statistics").send().await,
        "get_statistics",
    );
}
