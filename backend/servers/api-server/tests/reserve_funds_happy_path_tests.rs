//! Happy-path (2xx) coverage for the governance `reserve_funds/` endpoints
//! (BIT-408 Wave 8, governance batch 3).
//!
//! All reserve-fund handlers acquire an [`RlsConnection`], which validates
//! tenant membership and sets the org/user GUCs. Under `TestApp` (no
//! `host_tenant_middleware`) the `X-Tenant-ID` header is the only tenant
//! source, and an active `organization_members` row is required. Each test
//! seeds an org + org-admin member, mints a real HS256 token, and drives the
//! endpoints asserting HTTP-level 2xx. `fund_alerts` has no create endpoint, so
//! the acknowledge/resolve tests seed one row directly (the test pool is
//! RLS-exempt).

#![allow(dead_code)]

mod common;

use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, seed_org, TestApp, TestConfig, TestResponse};

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
        email: "reserve-hp@test.test".to_string(),
        name: "Reserve HP User".to_string(),
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
           VALUES ($1, 'test_hash', 'Reserve User', 'active', NOW()) RETURNING id"#,
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
    pool: PgPool,
}

async fn setup(pool: PgPool, tag: &str) -> Ctx {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, tag).await;
    let user_id = seed_user(&pool, &format!("{tag}-{}@reserve-hp.test", Uuid::new_v4())).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let token = mint_token(user_id, org_id);
    Ctx {
        app,
        org_id,
        user_id,
        token,
        pool,
    }
}

impl Ctx {
    fn tenant(&self) -> String {
        self.org_id.to_string()
    }
    fn get(&self, uri: &str) -> common::RequestBuilder {
        self.app
            .get(uri)
            .bearer(&self.token)
            .header("X-Tenant-ID", &self.tenant())
    }
    fn post(&self, uri: &str, body: Value) -> common::RequestBuilder {
        self.app
            .post(uri)
            .bearer(&self.token)
            .header("X-Tenant-ID", &self.tenant())
            .json(body)
    }
    fn post_empty(&self, uri: &str) -> common::RequestBuilder {
        self.app
            .post(uri)
            .bearer(&self.token)
            .header("X-Tenant-ID", &self.tenant())
    }
    fn put(&self, uri: &str, body: Value) -> common::RequestBuilder {
        self.app
            .put(uri)
            .bearer(&self.token)
            .header("X-Tenant-ID", &self.tenant())
            .json(body)
    }

    async fn create_fund(&self, name: &str) -> Uuid {
        let resp = self
            .post(
                "/api/v1/reserve-funds",
                json!({ "name": name, "fund_type": "reserve" }),
            )
            .send()
            .await;
        ok(&resp, "create_fund");
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
// Funds + schedules + transactions + transfers
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reserve_funds_core_happy_path(pool: PgPool) {
    let ctx = setup(pool, "core").await;
    let fund_id = ctx.create_fund("Operating Fund").await;

    ok(&ctx.get("/api/v1/reserve-funds").send().await, "list_funds");
    ok(
        &ctx.get("/api/v1/reserve-funds/dashboard").send().await,
        "get_dashboard",
    );
    ok(
        &ctx.get(&format!("/api/v1/reserve-funds/{fund_id}"))
            .send()
            .await,
        "get_fund",
    );
    ok(
        &ctx.put(
            &format!("/api/v1/reserve-funds/{fund_id}"),
            json!({ "name": "Operating Fund (renamed)" }),
        )
        .send()
        .await,
        "update_fund",
    );
    ok(
        &ctx.get(&format!("/api/v1/reserve-funds/{fund_id}/health"))
            .send()
            .await,
        "get_fund_health",
    );

    // schedules
    let schedule = ctx
        .post(
            &format!("/api/v1/reserve-funds/{fund_id}/schedules"),
            json!({
                "name": "Monthly Contribution",
                "amount": "500.00",
                "frequency": "monthly",
                "start_date": "2026-01-01"
            }),
        )
        .send()
        .await;
    ok(&schedule, "create_schedule");
    let schedule_id = id_of(&schedule.json_value());
    ok(
        &ctx.get(&format!("/api/v1/reserve-funds/{fund_id}/schedules"))
            .send()
            .await,
        "list_schedules",
    );
    ok(
        &ctx.put(
            &format!("/api/v1/reserve-funds/{fund_id}/schedules/{schedule_id}"),
            json!({ "amount": "600.00" }),
        )
        .send()
        .await,
        "update_schedule",
    );

    // transactions
    ok(
        &ctx.post(
            &format!("/api/v1/reserve-funds/{fund_id}/transactions"),
            json!({ "transaction_type": "contribution", "amount": "500.00" }),
        )
        .send()
        .await,
        "record_transaction",
    );
    ok(
        &ctx.get(&format!("/api/v1/reserve-funds/{fund_id}/transactions"))
            .send()
            .await,
        "list_transactions",
    );

    // transfer between two funds
    let fund_b = ctx.create_fund("Emergency Fund").await;
    ok(
        &ctx.post(
            "/api/v1/reserve-funds/transfers",
            json!({ "from_fund_id": fund_id, "to_fund_id": fund_b, "amount": "100.00" }),
        )
        .send()
        .await,
        "transfer_funds",
    );
}

// ---------------------------------------------------------------------------
// Policies + projections + components
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reserve_funds_planning_happy_path(pool: PgPool) {
    let ctx = setup(pool, "plan").await;
    let fund_id = ctx.create_fund("Capital Fund").await;

    // policies
    ok(
        &ctx.post(
            &format!("/api/v1/reserve-funds/{fund_id}/policies"),
            json!({
                "name": "Conservative",
                "risk_level": "conservative",
                "cash_allocation_pct": "100.00",
                "bonds_allocation_pct": "0.00",
                "money_market_allocation_pct": "0.00",
                "other_allocation_pct": "0.00",
                "effective_date": "2026-01-01"
            }),
        )
        .send()
        .await,
        "create_policy",
    );
    ok(
        &ctx.get(&format!("/api/v1/reserve-funds/{fund_id}/policies"))
            .send()
            .await,
        "list_policies",
    );
    ok(
        &ctx.get(&format!("/api/v1/reserve-funds/{fund_id}/policies/active"))
            .send()
            .await,
        "get_active_policy",
    );

    // projections + items
    let projection = ctx
        .post(
            &format!("/api/v1/reserve-funds/{fund_id}/projections"),
            json!({
                "study_name": "2026 Reserve Study",
                "study_date": "2026-06-29",
                "projection_years": 30
            }),
        )
        .send()
        .await;
    ok(&projection, "create_projection");
    let projection_id = id_of(&projection.json_value());
    ok(
        &ctx.get(&format!(
            "/api/v1/reserve-funds/{fund_id}/projections/current"
        ))
        .send()
        .await,
        "get_current_projection",
    );
    ok(
        &ctx.post(
            &format!("/api/v1/reserve-funds/{fund_id}/projections/{projection_id}/items"),
            json!([{
                "projection_year": 1,
                "fiscal_year": 2026,
                "contributions": "5000.00",
                "interest_income": "75.00",
                "planned_expenditures": "2000.00",
                "beginning_balance": "5000.00",
                "ending_balance": "8075.00"
            }]),
        )
        .send()
        .await,
        "add_projection_items",
    );
    ok(
        &ctx.get(&format!(
            "/api/v1/reserve-funds/{fund_id}/projections/{projection_id}/items"
        ))
        .send()
        .await,
        "get_projection_items",
    );

    // components
    let component = ctx
        .post(
            &format!("/api/v1/reserve-funds/{fund_id}/components"),
            json!({ "name": "Roof" }),
        )
        .send()
        .await;
    ok(&component, "create_component");
    let component_id = id_of(&component.json_value());
    ok(
        &ctx.get(&format!("/api/v1/reserve-funds/{fund_id}/components"))
            .send()
            .await,
        "list_components",
    );
    ok(
        &ctx.put(
            &format!("/api/v1/reserve-funds/{fund_id}/components/{component_id}"),
            json!({ "condition_rating": 6 }),
        )
        .send()
        .await,
        "update_component",
    );
}

// ---------------------------------------------------------------------------
// Alerts (seeded directly; no create endpoint exists)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reserve_funds_alerts_happy_path(pool: PgPool) {
    let ctx = setup(pool, "alert").await;
    let fund_id = ctx.create_fund("Alerting Fund").await;

    let alert_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO fund_alerts (fund_id, alert_type, severity, title, message, is_active)
           VALUES ($1, 'low_balance', 'warning', 'Low Balance', 'Below minimum', true)
           RETURNING id"#,
    )
    .bind(fund_id)
    .fetch_one(&ctx.pool)
    .await
    .expect("seed alert");

    ok(
        &ctx.get("/api/v1/reserve-funds/alerts").send().await,
        "list_alerts",
    );
    ok(
        &ctx.post_empty(&format!(
            "/api/v1/reserve-funds/alerts/{alert_id}/acknowledge"
        ))
        .send()
        .await,
        "acknowledge_alert",
    );
    ok(
        &ctx.post_empty(&format!("/api/v1/reserve-funds/alerts/{alert_id}/resolve"))
            .send()
            .await,
        "resolve_alert",
    );
}
