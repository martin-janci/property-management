//! Round-3 runtime audit regressions for request payloads.
//!
//! - #7: money fields sent as JSON numbers (`650`) were rejected with 422
//!   "expected a Decimal" because rust_decimal had `serde-str`. Decimals must
//!   deserialize from numbers *and* strings, and still serialize as strings.
//! - #12: an IoT sensor with an unknown `connection_type` hit the DB CHECK and
//!   returned 500; it must be a 400 naming the allowed values.

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

struct Ctx {
    app: TestApp,
    token: String,
    org_id: Uuid,
    user_id: Uuid,
    building_id: Uuid,
    unit_id: Uuid,
}

async fn setup(pool: PgPool, slug: &str) -> Ctx {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, slug).await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    let building_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, street, city, postal_code, country) \
         VALUES ($1, 'Numeric 1', 'Bratislava', '81101', 'Slovakia') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    let unit_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, 'N-1') RETURNING id",
    )
    .bind(building_id)
    .fetch_one(&app.pool)
    .await
    .unwrap();
    Ctx {
        app,
        token,
        org_id,
        user_id,
        building_id,
        unit_id,
    }
}

impl Ctx {
    async fn post(&self, path: &str, body: serde_json::Value) -> crate::common::TestResponse {
        self.app
            .execute(
                self.app
                    .post(path)
                    .bearer(&self.token)
                    .tenant(self.org_id)
                    .json(body)
                    .build(),
            )
            .await
    }
}

// ---------------------------------------------------------------------------
// #7 — Decimal accepts JSON numbers
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn invoice_create_accepts_numeric_amounts(pool: PgPool) {
    let ctx = setup(pool, "num-inv").await;
    let resp = ctx
        .post(
            "/api/v1/financial/invoices",
            json!({
                "organization_id": ctx.org_id,
                "user_id": ctx.user_id,
                "unit_id": ctx.unit_id,
                "due_date": "2026-07-31",
                "currency": "EUR",
                "items": [
                    { "description": "Fee", "quantity": 1, "unit_price": 650 },
                    { "description": "Extra", "quantity": "2", "unit_price": 12.5 }
                ]
            }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let body = resp.json_value();
    // Output format unchanged: Decimal still serializes as a JSON string.
    let total = &body["invoice"]["total"];
    assert!(total.is_string(), "total must stay a JSON string: {total}");
    let total: rust_decimal::Decimal = total.as_str().unwrap().parse().unwrap();
    assert_eq!(total, rust_decimal::Decimal::new(675, 0));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn payment_create_accepts_numeric_amount(pool: PgPool) {
    let ctx = setup(pool, "num-pay").await;
    let resp = ctx
        .post(
            "/api/v1/financial/payments",
            json!({
                "organization_id": ctx.org_id,
                "user_id": ctx.user_id,
                "unit_id": ctx.unit_id,
                "amount": 650,
                "currency": "EUR",
                "payment_method": "bank_transfer",
                "invoice_ids": []
            }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let amount = &resp.json_value()["payment"]["amount"];
    assert!(
        amount.is_string(),
        "amount must stay a JSON string: {amount}"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn lease_create_accepts_numeric_amounts(pool: PgPool) {
    let ctx = setup(pool, "num-lease").await;
    let resp = ctx
        .post(
            "/api/v1/leases",
            json!({
                "organization_id": ctx.org_id,
                "unit_id": ctx.unit_id,
                "landlord_name": "Landlord",
                "tenant_name": "Tenant",
                "tenant_email": "tenant@num-lease.test",
                "start_date": "2026-07-01",
                "end_date": "2027-06-30",
                "term_months": 12,
                "monthly_rent": 650,
                "security_deposit": 1300.5,
                "late_fee_amount": 10
            }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let body = resp.json_value();
    assert!(body["monthly_rent"].is_string(), "{body}");
}

// ---------------------------------------------------------------------------
// #12 — IoT sensor enum validation
// ---------------------------------------------------------------------------

fn sensor_body(ctx: &Ctx, connection_type: &str) -> serde_json::Value {
    json!({
        "organization_id": ctx.org_id,
        "building_id": ctx.building_id,
        "name": "Lobby thermometer",
        "sensor_type": "temperature",
        "connection_type": connection_type,
        "created_by": ctx.user_id
    })
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sensor_create_rejects_unknown_connection_type(pool: PgPool) {
    let ctx = setup(pool, "iot-conn").await;
    let resp = ctx
        .post("/api/v1/iot/sensors", sensor_body(&ctx, "modbus"))
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    let text = resp.text();
    assert!(text.contains("connection_type"), "{text}");
    assert!(text.contains("mqtt"), "error lists allowed values: {text}");

    let mut bad_type = sensor_body(&ctx, "mqtt");
    bad_type["sensor_type"] = json!("thermometer");
    let resp = ctx.post("/api/v1/iot/sensors", bad_type).await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());

    for ok in ["api", "mqtt", "webhook", "polling"] {
        let resp = ctx.post("/api/v1/iot/sensors", sensor_body(&ctx, ok)).await;
        assert_eq!(resp.status, StatusCode::CREATED, "{ok}: {}", resp.text());
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sensor_update_rejects_unknown_connection_type_and_status(pool: PgPool) {
    let ctx = setup(pool, "iot-upd").await;
    let resp = ctx
        .post("/api/v1/iot/sensors", sensor_body(&ctx, "api"))
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let id = resp.json_value()["id"].as_str().unwrap().to_string();

    for body in [
        json!({ "connection_type": "http" }),
        json!({ "status": "broken" }),
    ] {
        let resp = ctx
            .app
            .execute(
                ctx.app
                    .put(&format!("/api/v1/iot/sensors/{id}"))
                    .bearer(&ctx.token)
                    .tenant(ctx.org_id)
                    .json(&body)
                    .build(),
            )
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "{body}: {}",
            resp.text()
        );
    }
}
