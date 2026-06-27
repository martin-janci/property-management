//! Happy-path and IDOR security integration tests for the accounting invoices endpoints (`/api/v1/accounting/invoices/*`).
//!
//! Exercises list_contacts, create_invoice, list_invoices, get_invoice, update_invoice,
//! list_invoice_items, and delete_invoice, as well as cross-org tenant isolation checks.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accounting_invoices_happy_path_and_idor(pool: PgPool) {
    // ========================================================================
    // 1. SETUP ORGANIZATIONS, USERS, AND MEMBERSHIPS
    // ========================================================================

    let app = TestApp::new(pool.clone()).await;

    // Org A (Happy Path & Target)
    let user_a = TestUser::new();
    let (token_a, org_a_id) = create_authenticated_user_with_org(&app, &user_a, "orga").await;
    // Org B (Attacker / Foreign Tenant)
    let user_b = TestUser::new();
    let (token_b, org_b_id) = create_authenticated_user_with_org(&app, &user_b, "orgb").await;

    // Seed Contact A in Org A
    let contact_a_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO contact (tenant_id, name, email, address)
           VALUES ($1, 'Contact A', 'contacta@test.example', '123 St A') RETURNING id"#,
    )
    .bind(org_a_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed contact A");

    // Seed Contact B in Org B
    let _contact_b_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO contact (tenant_id, name, email, address)
           VALUES ($1, 'Contact B', 'contactb@test.example', '456 St B') RETURNING id"#,
    )
    .bind(org_b_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed contact B");

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
    // 3. CROSS-ORG IDOR SECURITY CHECKS (Org B tries to access/modify Org A)
    // ========================================================================

    // 3.1 GET invoice: Org B tries to view Invoice A -> should fail (404 Not Found or 403 Forbidden)
    let resp = app
        .execute(
            app.get(&format!("/api/v1/accounting/invoices/{invoice_id}"))
                .bearer(&token_b)
                .tenant(org_b_id) // using Org B's tenant context
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::NOT_FOUND || resp.status == StatusCode::FORBIDDEN,
        "Org B got status {} when viewing Org A's invoice (expected 404/403)",
        resp.status
    );

    // 3.2 PATCH invoice: Org B tries to update Invoice A -> should fail (404 or 403)
    let resp = app
        .execute(
            app.patch(&format!("/api/v1/accounting/invoices/{invoice_id}"))
                .bearer(&token_b)
                .tenant(org_b_id)
                .json(json!({"number": "INV-HACKED"}))
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::NOT_FOUND || resp.status == StatusCode::FORBIDDEN,
        "Org B got status {} when updating Org A's invoice",
        resp.status
    );

    // 3.3 POST invoice: Org B tries to create invoice using Contact A -> should fail (400 Bad Request / Contact not found)
    let resp = app
        .execute(
            app.post("/api/v1/accounting/invoices")
                .bearer(&token_b)
                .tenant(org_b_id)
                .json(json!({
                    "tenant_id": org_b_id,
                    "contact_id": contact_a_id, // cross-tenant contact
                    "number": "INV-B-001",
                    "issue_date": "2026-06-27",
                    "due_date": "2026-07-27",
                    "currency": "EUR",
                    "items": [
                        {
                            "description": "Hack item",
                            "qty": "1",
                            "unit_price": "1000.00",
                            "vat_rate": "20.00"
                        }
                    ]
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "Org B got status {} when using Org A's contact",
        resp.status
    );

    // 3.4 Org B tries to access Org A's tenant directly in header with Org B's token -> should fail with 403 Forbidden
    let resp = app
        .execute(
            app.get("/api/v1/accounting/invoices")
                .bearer(&token_b)
                .tenant(org_a_id) // using Org A's tenant
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "Org B got status {} when accessing Org A's tenant via header",
        resp.status
    );

    // ========================================================================
    // 4. CLEANUP / DELETE ROUTINE
    // ========================================================================

    // 4.1 DELETE invoice: Org B tries to delete Invoice A -> should fail (404 or 403)
    let resp = app
        .execute(
            app.delete(&format!("/api/v1/accounting/invoices/{invoice_id}"))
                .bearer(&token_b)
                .tenant(org_b_id)
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::NOT_FOUND || resp.status == StatusCode::FORBIDDEN,
        "Org B got status {} when deleting Org A's invoice",
        resp.status
    );

    // 4.2 DELETE invoice: Org A deletes its own invoice -> should succeed (204 No Content or 200 OK)
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
