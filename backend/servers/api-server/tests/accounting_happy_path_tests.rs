//! Happy-path integration tests for the accounting endpoints (`/api/v1/accounting/*`).
//!
//! Exercises contacts and invoices routes for the accounting MVP (PAP-206).

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accounting_endpoints_happy_path(pool: PgPool) {
    // 1. Initialize the TestApp
    let app = TestApp::new(pool.clone()).await;

    // 2. Seed organization, user, and membership to authenticate requests
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "acchappy").await;

    let session = app.session(token, org_id);

    // 3. Seed a contact directly in the DB since there's no CRM POST route yet
    let contact_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO contact (tenant_id, name, email) VALUES ($1, 'John Doe CRM', 'john.doe@example.com') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed contact");

    // ========================================================================
    // 1. CONTACTS
    // ========================================================================

    // 1.1 GET /api/v1/accounting/contacts -> list_contacts
    let resp = app
        .execute(
            session
                .get("/api/v1/accounting/contacts")
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let contacts_list = resp.json_value();
    assert!(contacts_list.as_array().map(|arr| !arr.is_empty()).unwrap_or(false));
    assert_eq!(contacts_list[0]["name"], "John Doe CRM");

    // ========================================================================
    // 2. INVOICES
    // ========================================================================

    // 2.1 POST /api/v1/accounting/invoices -> create_invoice
    let create_invoice_payload = json!({
        "contact_id": contact_id,
        "number": "INV-2026-0001",
        "issue_date": "2026-06-27",
        "due_date": "2026-07-27",
        "currency": "EUR",
        "variable_symbol": "20260001",
        "status": "draft",
        "items": [
            {
                "description": "Property Management Fee",
                "qty": "1.00",
                "unit_price": "250.00",
                "vat_rate": "20.0"
            }
        ]
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/accounting/invoices")
                .json(&create_invoice_payload)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "create_invoice should succeed: {}", resp.text());
    let invoice = resp.json_value();
    let invoice_id_str = invoice["id"].as_str().expect("id missing");
    let invoice_id = Uuid::parse_str(invoice_id_str).expect("invalid invoice uuid");

    // Create a temporary invoice to test delete
    let temp_invoice_payload = json!({
        "contact_id": contact_id,
        "number": "INV-2026-TEMP",
        "issue_date": "2026-06-27",
        "due_date": "2026-07-27",
        "currency": "EUR",
        "status": "draft",
        "items": [
            {
                "description": "Temporary Fee",
                "qty": "1.00",
                "unit_price": "10.00",
                "vat_rate": "0.0"
            }
        ]
    });
    let resp_temp = app
        .execute(
            session
                .post("/api/v1/accounting/invoices")
                .json(&temp_invoice_payload)
                .build(),
        )
        .await;
    assert_eq!(resp_temp.status, StatusCode::CREATED);
    let temp_invoice_id_str = resp_temp.json_value()["id"].as_str().expect("id missing").to_string();
    let temp_invoice_id = Uuid::parse_str(&temp_invoice_id_str).unwrap();

    // 2.2 GET /api/v1/accounting/invoices -> list_invoices
    let resp = app
        .execute(
            session
                .get("/api/v1/accounting/invoices")
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let invoices_list = resp.json_value();
    assert!(invoices_list.as_array().map(|arr| arr.len() >= 2).unwrap_or(false));

    // 2.3 GET /api/v1/accounting/invoices/{id} -> get_invoice
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/accounting/invoices/{invoice_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let invoice_details = resp.json_value();
    assert_eq!(invoice_details["id"], invoice_id_str);

    // 2.4 GET /api/v1/accounting/invoices/{id}/items -> list_invoice_items
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/accounting/invoices/{invoice_id}/items"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let items_list = resp.json_value();
    assert!(items_list.as_array().map(|arr| !arr.is_empty()).unwrap_or(false));
    assert_eq!(items_list[0]["description"], "Property Management Fee");

    // 2.5 PATCH /api/v1/accounting/invoices/{id} -> update_invoice
    let update_invoice_payload = json!({
        "number": "INV-2026-0001-REV",
        "variable_symbol": "20260002",
        "status": "issued"
    });
    let resp = app
        .execute(
            session
                .patch(&format!("/api/v1/accounting/invoices/{invoice_id}"))
                .json(&update_invoice_payload)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "update_invoice should succeed: {}", resp.text());
    let updated_invoice = resp.json_value();
    assert_eq!(updated_invoice["number"], "INV-2026-0001-REV");
    assert_eq!(updated_invoice["status"], "issued");

    // 2.6 DELETE /api/v1/accounting/invoices/{id} -> delete_invoice
    let resp = app
        .execute(
            session
                .delete(&format!("/api/v1/accounting/invoices/{temp_invoice_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);

    // Verify it was deleted
    let resp_verify = app
        .execute(
            session
                .get(&format!("/api/v1/accounting/invoices/{temp_invoice_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp_verify.status, StatusCode::NOT_FOUND);
}
