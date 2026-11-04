//! Round-4 authorization regression tests for the finance cluster
//! (`/api/v1/financial`, `/vendors`, `/reserve-funds`, `/subscriptions`,
//! `/accounting`).
//!
//! Each handler group is checked three ways:
//!   * cross-org: an id owned by another organization is `404`;
//!   * role: a `tenant` member (DB role) on a manager-only mutation is `403`;
//!   * owner: a manager of the owning org succeeds.
//!
//! Roles are always seeded in `organization_members`; the JWT carries no
//! role, so every decision here comes from the database.

#![allow(dead_code)]

use axum::http::StatusCode;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, RequestBuilder, TestApp, TestConfig, TestResponse};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool) -> (Uuid, String) {
    let email = format!("r4-{}@finance-authz.test", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Finance R4 User', 'active', NOW()) RETURNING id"#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    (id, email)
}

/// Login-format access token (no tenant claim, no role claim).
fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "Finance R4 User", None, None)
        .expect("mint access token")
}

/// A user with an active membership of `role` in `org`.
struct Member {
    org: Uuid,
    user: Uuid,
    token: String,
}

async fn member(pool: &PgPool, org: Uuid, role: &str) -> Member {
    let (user, email) = seed_user(pool).await;
    seed_membership(pool, org, user, role).await;
    Member {
        org,
        user,
        token: mint_token(user, &email),
    }
}

async fn seed_unit(pool: &PgPool, org: Uuid) -> Uuid {
    let building = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'R4 Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building");
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, 'R4-1') RETURNING id",
    )
    .bind(building)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

impl Member {
    /// Request with bearer + `X-Tenant-ID` of the member's org.
    fn req(&self, app: &TestApp, method: &str, uri: &str) -> RequestBuilder {
        let b = match method {
            "GET" => app.get(uri),
            "POST" => app.post(uri),
            "PUT" => app.put(uri),
            "PATCH" => app.patch(uri),
            "DELETE" => app.delete(uri),
            _ => unreachable!(),
        };
        b.bearer(&self.token).tenant(self.org)
    }
}

fn id_of(resp: &TestResponse) -> Uuid {
    Uuid::parse_str(resp.json_value()["id"].as_str().expect("id")).expect("uuid")
}

fn expect(resp: &TestResponse, status: StatusCode, ctx: &str) {
    assert_eq!(resp.status, status, "{ctx}: body = {}", resp.text());
}

// ===========================================================================
// /api/v1/financial
// ===========================================================================

async fn create_fin_invoice(app: &TestApp, m: &Member, unit: Uuid, amount: &str) -> Uuid {
    let resp = m
        .req(app, "POST", "/api/v1/financial/invoices")
        .json(json!({
            "organization_id": m.org,
            "user_id": m.user,
            "unit_id": unit,
            "due_date": "2026-07-31",
            "currency": "EUR",
            "items": [{ "description": "Fee", "quantity": "1.00", "unit_price": amount }]
        }))
        .send()
        .await;
    expect(&resp, StatusCode::CREATED, "owner create_invoice");
    Uuid::parse_str(resp.json_value()["invoice"]["id"].as_str().unwrap()).unwrap()
}

async fn invoice_balance(pool: &PgPool, id: Uuid) -> Decimal {
    sqlx::query_scalar("SELECT balance_due FROM invoices WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn payment_body(org: Uuid, unit: Uuid, invoice_ids: Vec<Uuid>) -> Value {
    json!({
        "organization_id": org,
        "user_id": Uuid::nil(),
        "unit_id": unit,
        "amount": "100.00",
        "currency": "EUR",
        "payment_method": "bank_transfer",
        "invoice_ids": invoice_ids
    })
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn financial_record_payment_cannot_touch_other_org(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "r4fa").await;
    let org_b = seed_org(&pool, "r4fb").await;
    let mgr_a = member(&pool, org_a, "manager").await;
    let mgr_b = member(&pool, org_b, "manager").await;
    let unit_a = seed_unit(&pool, org_a).await;
    let unit_b = seed_unit(&pool, org_b).await;
    let invoice_a = create_fin_invoice(&app, &mgr_a, unit_a, "100.00").await;
    let before = invoice_balance(&pool, invoice_a).await;

    // Org B pays "for" org A's unit (auto-allocation path).
    let resp = mgr_b
        .req(&app, "POST", "/api/v1/financial/payments")
        .json(payment_body(org_b, unit_a, vec![]))
        .send()
        .await;
    expect(&resp, StatusCode::NOT_FOUND, "cross-org unit_id");

    // Org B targets org A's invoice explicitly from its own unit.
    let resp = mgr_b
        .req(&app, "POST", "/api/v1/financial/payments")
        .json(payment_body(org_b, unit_b, vec![invoice_a]))
        .send()
        .await;
    expect(&resp, StatusCode::NOT_FOUND, "cross-org invoice_ids");

    assert_eq!(invoice_balance(&pool, invoice_a).await, before);
    let credits: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM unit_credit_balances WHERE unit_id = $1")
            .bind(unit_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(credits, 0, "org A's unit must not receive a foreign credit");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn financial_mutations_require_manager_role(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r4ft").await;
    let tenant = member(&pool, org, "tenant").await;
    let unit = seed_unit(&pool, org).await;

    let cases = [
        (
            "/api/v1/financial/accounts",
            json!({ "organization_id": org, "name": "X", "account_type": "operating" }),
        ),
        (
            "/api/v1/financial/invoices",
            json!({
                "organization_id": org, "user_id": Uuid::nil(), "unit_id": unit,
                "due_date": "2026-07-31",
                "items": [{ "description": "x", "unit_price": "1.00" }]
            }),
        ),
        (
            "/api/v1/financial/payments",
            payment_body(org, unit, vec![]),
        ),
        (
            "/api/v1/financial/payments/auto-match",
            json!({ "organization_id": org }),
        ),
    ];
    for (uri, body) in cases {
        let resp = tenant.req(&app, "POST", uri).json(body).send().await;
        expect(&resp, StatusCode::FORBIDDEN, uri);
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn financial_owner_manager_records_payment(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r4fo").await;
    let mgr = member(&pool, org, "manager").await;
    let unit = seed_unit(&pool, org).await;
    let invoice = create_fin_invoice(&app, &mgr, unit, "100.00").await;

    let resp = mgr
        .req(&app, "POST", "/api/v1/financial/payments")
        .json(payment_body(org, unit, vec![invoice]))
        .send()
        .await;
    expect(&resp, StatusCode::CREATED, "owner record_payment");
    assert_eq!(invoice_balance(&pool, invoice).await, Decimal::ZERO);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn financial_unit_fees_are_org_scoped(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "r4ufa").await;
    let org_b = seed_org(&pool, "r4ufb").await;
    let mgr_a = member(&pool, org_a, "manager").await;
    let mgr_b = member(&pool, org_b, "manager").await;
    let unit_a = seed_unit(&pool, org_a).await;
    let unit_b = seed_unit(&pool, org_b).await;

    let building_of = |unit: Uuid| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, Uuid>("SELECT building_id FROM units WHERE id = $1")
                .bind(unit)
                .fetch_one(&pool)
                .await
                .unwrap()
        }
    };
    let building_a = building_of(unit_a).await;
    let building_b = building_of(unit_b).await;
    let schedule_body = |org: Uuid, building: Uuid| {
        json!({
            "organization_id": org, "user_id": Uuid::nil(), "building_id": building,
            "name": "Fee", "amount": "10.00",
            "frequency": "monthly", "effective_from": "2026-01-01"
        })
    };
    let resp = mgr_a
        .req(&app, "POST", "/api/v1/financial/fee-schedules")
        .json(schedule_body(org_a, building_a))
        .send()
        .await;
    expect(&resp, StatusCode::CREATED, "org A fee schedule");
    let schedule_a = id_of(&resp);
    let resp = mgr_b
        .req(&app, "POST", "/api/v1/financial/fee-schedules")
        .json(schedule_body(org_b, building_b))
        .send()
        .await;
    expect(&resp, StatusCode::CREATED, "org B fee schedule");
    let schedule_b = id_of(&resp);

    let assign = |org: Uuid, schedule: Uuid| json!({ "organization_id": org, "fee_schedule_id": schedule, "effective_from": "2026-01-01" });

    // Owner: assign and read back.
    let resp = mgr_a
        .req(
            &app,
            "POST",
            &format!("/api/v1/financial/units/{unit_a}/fees"),
        )
        .json(assign(org_a, schedule_a))
        .send()
        .await;
    expect(&resp, StatusCode::CREATED, "owner assign_unit_fee");

    // Cross-org unit (write and read) and cross-org fee schedule.
    let resp = mgr_b
        .req(
            &app,
            "POST",
            &format!("/api/v1/financial/units/{unit_a}/fees"),
        )
        .json(assign(org_b, schedule_b))
        .send()
        .await;
    expect(&resp, StatusCode::NOT_FOUND, "assign fee to foreign unit");
    let resp = mgr_b
        .req(
            &app,
            "POST",
            &format!("/api/v1/financial/units/{unit_b}/fees"),
        )
        .json(assign(org_b, schedule_a))
        .send()
        .await;
    expect(&resp, StatusCode::NOT_FOUND, "assign foreign fee schedule");
    let resp = mgr_b
        .req(
            &app,
            "GET",
            &format!("/api/v1/financial/units/{unit_a}/fees?organization_id={org_b}"),
        )
        .send()
        .await;
    expect(&resp, StatusCode::NOT_FOUND, "read foreign unit fees");

    // Tenant on the manager-only mutation.
    let tenant_a = member(&pool, org_a, "tenant").await;
    let resp = tenant_a
        .req(
            &app,
            "POST",
            &format!("/api/v1/financial/units/{unit_a}/fees"),
        )
        .json(assign(org_a, schedule_a))
        .send()
        .await;
    expect(&resp, StatusCode::FORBIDDEN, "tenant assign_unit_fee");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn financial_create_rejects_foreign_nested_ids(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "r4fna").await;
    let org_b = seed_org(&pool, "r4fnb").await;
    let mgr_b = member(&pool, org_b, "manager").await;
    let unit_a = seed_unit(&pool, org_a).await;

    let resp = mgr_b
        .req(&app, "POST", "/api/v1/financial/invoices")
        .json(json!({
            "organization_id": org_b, "user_id": mgr_b.user, "unit_id": unit_a,
            "due_date": "2026-07-31",
            "items": [{ "description": "x", "unit_price": "1.00" }]
        }))
        .send()
        .await;
    expect(&resp, StatusCode::NOT_FOUND, "invoice for foreign unit");

    let resp = mgr_b
        .req(&app, "POST", "/api/v1/financial/accounts")
        .json(json!({
            "organization_id": org_b, "unit_id": unit_a,
            "name": "Shadow ledger", "account_type": "unit_ledger"
        }))
        .send()
        .await;
    expect(&resp, StatusCode::NOT_FOUND, "account for foreign unit");
}

// ===========================================================================
// /api/v1/vendors
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn vendor_mutations_require_manager_and_stay_in_org(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "r4va").await;
    let org_b = seed_org(&pool, "r4vb").await;
    let mgr_a = member(&pool, org_a, "manager").await;
    let tenant_a = member(&pool, org_a, "tenant").await;
    let mgr_b = member(&pool, org_b, "manager").await;

    // Owner manager creates a vendor and an invoice.
    let resp = mgr_a
        .req(&app, "POST", "/api/v1/vendors")
        .json(json!({ "organization_id": org_a, "company_name": "Acme", "services": [] }))
        .send()
        .await;
    expect(&resp, StatusCode::CREATED, "owner create_vendor");
    let vendor = id_of(&resp);
    let resp = mgr_a
        .req(&app, "POST", "/api/v1/vendors/invoices")
        .json(json!({
            "organization_id": org_a, "vendor_id": vendor, "invoice_number": "V-1",
            "invoice_date": "2026-06-01", "due_date": "2026-07-01",
            "subtotal": "100.00", "total_amount": "100.00"
        }))
        .send()
        .await;
    expect(&resp, StatusCode::CREATED, "owner create vendor invoice");
    let invoice = id_of(&resp);

    // Tenant: every mutation is 403.
    let resp = tenant_a
        .req(&app, "POST", "/api/v1/vendors")
        .json(json!({ "organization_id": org_a, "company_name": "Evil", "services": [] }))
        .send()
        .await;
    expect(&resp, StatusCode::FORBIDDEN, "tenant create_vendor");
    let resp = tenant_a
        .req(
            &app,
            "POST",
            &format!("/api/v1/vendors/invoices/{invoice}/approve"),
        )
        .send()
        .await;
    expect(&resp, StatusCode::FORBIDDEN, "tenant approve_invoice");
    let resp = tenant_a
        .req(&app, "DELETE", &format!("/api/v1/vendors/{vendor}"))
        .send()
        .await;
    expect(&resp, StatusCode::FORBIDDEN, "tenant delete_vendor");

    // Cross-org manager: 404.
    let resp = mgr_b
        .req(
            &app,
            "POST",
            &format!("/api/v1/vendors/invoices/{invoice}/approve"),
        )
        .send()
        .await;
    expect(&resp, StatusCode::NOT_FOUND, "cross-org approve_invoice");

    // Owner approves.
    let resp = mgr_a
        .req(
            &app,
            "POST",
            &format!("/api/v1/vendors/invoices/{invoice}/approve"),
        )
        .send()
        .await;
    expect(&resp, StatusCode::OK, "owner approve_invoice");
}

// ===========================================================================
// /api/v1/reserve-funds
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reserve_fund_mutations_require_manager_and_nested_ids_match(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "r4ra").await;
    let org_b = seed_org(&pool, "r4rb").await;
    let mgr_a = member(&pool, org_a, "manager").await;
    let tenant_a = member(&pool, org_a, "tenant").await;
    let mgr_b = member(&pool, org_b, "manager").await;

    let create_fund = |name: &'static str| json!({ "name": name, "fund_type": "reserve" });
    let resp = mgr_a
        .req(&app, "POST", "/api/v1/reserve-funds")
        .json(create_fund("Fund 1"))
        .send()
        .await;
    expect(&resp, StatusCode::OK, "owner create_fund");
    let fund_1 = id_of(&resp);
    let resp = mgr_a
        .req(&app, "POST", "/api/v1/reserve-funds")
        .json(create_fund("Fund 2"))
        .send()
        .await;
    expect(&resp, StatusCode::OK, "owner create second fund");
    let fund_2 = id_of(&resp);

    let resp = mgr_a
        .req(
            &app,
            "POST",
            &format!("/api/v1/reserve-funds/{fund_1}/components"),
        )
        .json(json!({ "name": "Roof" }))
        .send()
        .await;
    expect(&resp, StatusCode::OK, "owner create_component");
    let component = id_of(&resp);

    // Tenant: mutations are 403.
    let resp = tenant_a
        .req(&app, "POST", "/api/v1/reserve-funds")
        .json(create_fund("Tenant fund"))
        .send()
        .await;
    expect(&resp, StatusCode::FORBIDDEN, "tenant create_fund");
    let resp = tenant_a
        .req(&app, "POST", "/api/v1/reserve-funds/transfers")
        .json(json!({ "from_fund_id": fund_1, "to_fund_id": fund_2, "amount": "1.00" }))
        .send()
        .await;
    expect(&resp, StatusCode::FORBIDDEN, "tenant transfer_funds");

    // The component belongs to fund 1, not fund 2.
    let resp = mgr_a
        .req(
            &app,
            "PUT",
            &format!("/api/v1/reserve-funds/{fund_2}/components/{component}"),
        )
        .json(json!({ "condition_rating": 2 }))
        .send()
        .await;
    expect(
        &resp,
        StatusCode::NOT_FOUND,
        "component under the wrong fund",
    );

    // Cross-org manager.
    let resp = mgr_b
        .req(
            &app,
            "PUT",
            &format!("/api/v1/reserve-funds/{fund_1}/components/{component}"),
        )
        .json(json!({ "condition_rating": 2 }))
        .send()
        .await;
    expect(&resp, StatusCode::NOT_FOUND, "cross-org update_component");

    // Owner, correct nesting.
    let resp = mgr_a
        .req(
            &app,
            "PUT",
            &format!("/api/v1/reserve-funds/{fund_1}/components/{component}"),
        )
        .json(json!({ "condition_rating": 2 }))
        .send()
        .await;
    expect(&resp, StatusCode::OK, "owner update_component");
}

// ===========================================================================
// /api/v1/subscriptions
// ===========================================================================

async fn seed_subscription(pool: &PgPool, org: Uuid) -> (Uuid, Uuid) {
    let plan = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO subscription_plans (name, display_name, monthly_price)
           VALUES ($1, 'R4 Plan', 10.00) RETURNING id"#,
    )
    .bind(format!("r4-plan-{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed plan");
    let sub = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organization_subscriptions
               (organization_id, plan_id, current_period_start, current_period_end)
           VALUES ($1, $2, NOW(), NOW() + INTERVAL '30 days') RETURNING id"#,
    )
    .bind(org)
    .bind(plan)
    .fetch_one(pool)
    .await
    .expect("seed subscription");
    let invoice = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO subscription_invoices
               (organization_id, subscription_id, invoice_number, invoice_date, due_date,
                subtotal, total_amount, currency, status)
           VALUES ($1, $2, $3, '2026-06-27', '2026-07-27', 10.00, 10.00, 'USD', 'draft')
           RETURNING id"#,
    )
    .bind(org)
    .bind(sub)
    .bind(format!("R4-{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed subscription invoice");
    (sub, invoice)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn subscription_mutations_are_role_gated_and_org_scoped(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "r4sa").await;
    let org_b = seed_org(&pool, "r4sb").await;
    let admin_a = member(&pool, org_a, "org_admin").await;
    let resident_a = member(&pool, org_a, "resident").await;
    let admin_b = member(&pool, org_b, "org_admin").await;
    let (sub_a, invoice_a) = seed_subscription(&pool, org_a).await;

    // Resident cannot cancel the org's plan.
    let resp = resident_a
        .req(
            &app,
            "POST",
            &format!("/api/v1/subscriptions/{sub_a}/cancel"),
        )
        .json(json!({}))
        .send()
        .await;
    expect(&resp, StatusCode::FORBIDDEN, "resident cancel_subscription");

    // Org admins cannot settle or void their own platform invoice.
    for action in ["pay", "void"] {
        let resp = admin_a
            .req(
                &app,
                "POST",
                &format!("/api/v1/subscriptions/invoices/{invoice_a}/{action}"),
            )
            .send()
            .await;
        expect(&resp, StatusCode::FORBIDDEN, action);
    }
    let status: String =
        sqlx::query_scalar("SELECT status::text FROM subscription_invoices WHERE id = $1")
            .bind(invoice_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "draft");

    // Another org's admin cannot touch org A's subscription.
    let resp = admin_b
        .req(
            &app,
            "POST",
            &format!("/api/v1/subscriptions/{sub_a}/cancel"),
        )
        .json(json!({}))
        .send()
        .await;
    expect(
        &resp,
        StatusCode::NOT_FOUND,
        "cross-org cancel_subscription",
    );

    // The owning org admin can.
    let resp = admin_a
        .req(
            &app,
            "POST",
            &format!("/api/v1/subscriptions/{sub_a}/cancel"),
        )
        .json(json!({}))
        .send()
        .await;
    expect(&resp, StatusCode::OK, "owner cancel_subscription");
}

// ===========================================================================
// /api/v1/accounting
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accounting_contacts_and_statements_are_tenant_filtered(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "r4aa").await;
    let org_b = seed_org(&pool, "r4ab").await;
    let mgr_a = member(&pool, org_a, "manager").await;

    sqlx::query("INSERT INTO contact (tenant_id, name) VALUES ($1, 'Foreign Contact')")
        .bind(org_b)
        .execute(&pool)
        .await
        .expect("seed foreign contact");
    let foreign_statement = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO bank_statement (tenant_id, source_filename, account_iban)
           VALUES ($1, 'b.csv', 'SK0000000000000000000000') RETURNING id"#,
    )
    .bind(org_b)
    .fetch_one(&pool)
    .await
    .expect("seed foreign statement");
    sqlx::query(
        r#"INSERT INTO bank_statement_line (statement_id, tenant_id, booking_date, amount, currency)
           VALUES ($1, $2, '2026-06-01', 10.00, 'EUR')"#,
    )
    .bind(foreign_statement)
    .bind(org_b)
    .execute(&pool)
    .await
    .expect("seed foreign statement line");

    let resp = mgr_a
        .req(&app, "GET", "/api/v1/accounting/contacts")
        .send()
        .await;
    expect(&resp, StatusCode::OK, "list_contacts");
    assert_eq!(resp.json_value().as_array().map(Vec::len), Some(0));

    let resp = mgr_a
        .req(&app, "GET", "/api/v1/accounting/statements")
        .send()
        .await;
    expect(&resp, StatusCode::OK, "list_statements");
    assert_eq!(resp.json_value().as_array().map(Vec::len), Some(0));

    let resp = mgr_a
        .req(
            &app,
            "GET",
            &format!("/api/v1/accounting/statements/{foreign_statement}/lines"),
        )
        .send()
        .await;
    expect(&resp, StatusCode::OK, "list_statement_lines");
    assert_eq!(resp.json_value().as_array().map(Vec::len), Some(0));
}

/// `create_invoice` used to require the JWT `tenant_id` claim, which login
/// tokens do not carry, so it returned 403 for everyone. It now takes the
/// tenant from the validated `X-Tenant-ID` membership.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accounting_create_invoice_works_with_login_token(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r4ai").await;
    let mgr = member(&pool, org, "manager").await;
    let contact = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, 'Buyer') RETURNING id",
    )
    .bind(org)
    .fetch_one(&pool)
    .await
    .expect("seed contact");

    let resp = mgr
        .req(&app, "POST", "/api/v1/accounting/invoices")
        .json(json!({
            "tenant_id": Uuid::nil(),
            "contact_id": contact,
            "number": "R4-1",
            "issue_date": "2026-06-01",
            "due_date": "2026-07-01",
            "currency": "EUR",
            "items": [{
                "description": "Service",
                "qty": "1",
                "unit_price": "100.00",
                "vat_rate": "20"
            }]
        }))
        .send()
        .await;
    expect(&resp, StatusCode::CREATED, "create accounting invoice");
    assert_eq!(
        resp.json_value()["tenant_id"].as_str(),
        Some(org.to_string().as_str())
    );
}
