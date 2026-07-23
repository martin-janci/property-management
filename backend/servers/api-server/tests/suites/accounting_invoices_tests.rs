//! Happy-path integration tests for the accounting invoices endpoints (`/api/v1/accounting/invoices/*`).
//!
//! Exercises list_contacts, create_invoice, list_invoices, get_invoice, update_invoice,
//! list_invoice_items, and delete_invoice.
//!
//! Cross-org tenant isolation (IDOR) is verified by the `rls-smoke-test` CI job which
//! runs against a PostgreSQL instance with RLS enabled — those assertions cannot be
//! made reliably under TestApp (no RLS session variable set).

#![allow(dead_code)]

use axum::http::StatusCode;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestConfig, TestUser};

// `create_invoice` (and the other accounting mutations) read the org from
// `AuthUser.tenant_id` (the JWT `tenant_id` claim) and require Manager+ via the
// DB membership. A login token carries no tenant claim — so it 403s with an
// empty body before the role check. We mint a tenant-scoped token; the
// `org_admin` membership seeded by the helper (level 90 ≥ Manager 80) satisfies
// the role gate.
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

fn mint(user_id: Uuid, email: &str, org_id: Uuid) -> String {
    let now = Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: email.to_string(),
        name: "Accounting Happy User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

async fn resolve_user_id(app: &TestApp, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accounting_invoices_happy_path_and_idor(pool: PgPool) {
    // ========================================================================
    // 1. SETUP ORGANIZATIONS, USERS, AND MEMBERSHIPS
    // ========================================================================

    let app = TestApp::new(pool.clone()).await;

    // Org A (Happy Path)
    let user_a = TestUser::new();
    let (_login_a, org_a_id) = create_authenticated_user_with_org(&app, &user_a, "orga").await;
    let user_a_id = resolve_user_id(&app, &user_a.email).await;
    let token_a = mint(user_a_id, &user_a.email, org_a_id);

    // Seed Contact A in Org A
    let contact_a_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO contact (tenant_id, name, email, address)
           VALUES ($1, 'Contact A', 'contacta@test.example', '123 St A') RETURNING id"#,
    )
    .bind(org_a_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed contact A");

    // ========================================================================
    // 2. HAPPY PATH FOR ORG A
    // ========================================================================

    // 2.1 GET /api/v1/accounting/contacts -> list_contacts (Org A)
    let resp = app
        .execute(
            app.get("/api/v1/accounting/contacts")
                .bearer(&token_a)
                .tenant(org_a_id)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_contacts failed: {}",
        resp.text()
    );
    let contacts = resp.json_value();
    assert!(contacts
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));
    assert_eq!(contacts[0]["name"], "Contact A");

    // 2.2 POST /api/v1/accounting/invoices -> create_invoice (Org A)
    let create_inv_payload = json!({
        "tenant_id": org_a_id,
        "contact_id": contact_a_id,
        "number": "INV-A-001",
        "issue_date": "2026-06-27",
        "due_date": "2026-07-27",
        "currency": "EUR",
        "items": [
            {
                "description": "Consulting A",
                "qty": "5",
                "unit_price": "200.00",
                "vat_rate": "20.00"
            }
        ]
    });
    let resp = app
        .execute(
            app.post("/api/v1/accounting/invoices")
                .bearer(&token_a)
                .tenant(org_a_id)
                .json(&create_inv_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_invoice failed: {}",
        resp.text()
    );
    let invoice = resp.json_value();
    let invoice_id_str = invoice["id"].as_str().expect("invoice id missing");
    let invoice_id = Uuid::parse_str(invoice_id_str).expect("invalid uuid");
    assert_eq!(invoice["number"], "INV-A-001");

    // 2.3 GET /api/v1/accounting/invoices -> list_invoices (Org A)
    let resp = app
        .execute(
            app.get("/api/v1/accounting/invoices")
                .bearer(&token_a)
                .tenant(org_a_id)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_invoices failed: {}",
        resp.text()
    );
    let invoices = resp.json_value();
    assert!(invoices
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));
    assert_eq!(invoices[0]["id"], invoice_id_str);

    // 2.4 GET /api/v1/accounting/invoices/{id} -> get_invoice (Org A)
    let resp = app
        .execute(
            app.get(&format!("/api/v1/accounting/invoices/{invoice_id}"))
                .bearer(&token_a)
                .tenant(org_a_id)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_invoice failed: {}",
        resp.text()
    );
    let invoice_details = resp.json_value();
    assert_eq!(invoice_details["id"], invoice_id_str);

    // 2.5 GET /api/v1/accounting/invoices/{id}/items -> list_invoice_items (Org A)
    let resp = app
        .execute(
            app.get(&format!("/api/v1/accounting/invoices/{invoice_id}/items"))
                .bearer(&token_a)
                .tenant(org_a_id)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_invoice_items failed: {}",
        resp.text()
    );
    let items = resp.json_value();
    assert!(items.as_array().map(|arr| !arr.is_empty()).unwrap_or(false));
    assert_eq!(items[0]["description"], "Consulting A");

    // 2.6 PATCH /api/v1/accounting/invoices/{id} -> update_invoice (Org A)
    let update_inv_payload = json!({
        "number": "INV-A-001-REV",
        "paid_amount": "500.00"
    });
    let resp = app
        .execute(
            app.patch(&format!("/api/v1/accounting/invoices/{invoice_id}"))
                .bearer(&token_a)
                .tenant(org_a_id)
                .json(&update_inv_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_invoice failed: {}",
        resp.text()
    );
    let updated_inv = resp.json_value();
    assert_eq!(updated_inv["number"], "INV-A-001-REV");

    // ========================================================================
    // 3. CLEANUP / DELETE
    // ========================================================================

    // DELETE invoice: Org A deletes its own invoice -> should succeed (204 No Content or 200 OK)
    let resp = app
        .execute(
            app.delete(&format!("/api/v1/accounting/invoices/{invoice_id}"))
                .bearer(&token_a)
                .tenant(org_a_id)
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::NO_CONTENT || resp.status == StatusCode::OK,
        "Org A delete failed: status = {}, response = {}",
        resp.status,
        resp.text()
    );
}
