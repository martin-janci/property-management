//! Happy-path integration tests for the financial endpoints (`/api/v1/financial/*`).
//!
//! Exercises 24 different endpoints covering financial accounts, transactions,
//! fee schedules, invoices, payments, and allocation/reconciliation.

#![allow(dead_code)]

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

use common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn financial_endpoints_happy_path(pool: PgPool) {
    // 1. Start wiremock server for Stripe Checkout mocking
    let stripe_server = MockServer::start().await;

    // 2. Configure Stripe environment variables to use our wiremock instance
    std::env::set_var("STRIPE_SECRET_KEY", "sk_test_mock");
    std::env::set_var("STRIPE_WEBHOOK_SECRET", "whsec_mock");
    std::env::set_var("STRIPE_SUCCESS_URL", "http://localhost/success");
    std::env::set_var("STRIPE_CANCEL_URL", "http://localhost/cancel");
    std::env::set_var("STRIPE_API_BASE", stripe_server.uri());

    // 3. Initialize the TestApp
    let app = TestApp::new(pool.clone()).await;

    // 4. Seed organization, user, and membership to authenticate requests
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "finhappy").await;

    // 5. Resolve user ID for payload seeding
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    // 6. Seed building and unit
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Happy Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, 'U-HAPPY-1') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed unit");

    // ========================================================================
    // 1. FINANCIAL ACCOUNTS (Story 11.1)
    // ========================================================================

    // 1.1 POST /api/v1/financial/accounts -> create_account
    let create_account_payload = json!({
        "organization_id": org_id,
        "building_id": building_id,
        "unit_id": unit_id,
        "name": "Operating Account",
        "account_type": "operating",
        "description": "Main operating account",
        "currency": "EUR",
        "opening_balance": "1000.00"
    });
    let resp = app
        .execute(
            app.post("/api/v1/financial/accounts")
                .bearer(&token)
                .json(&create_account_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_account should succeed: {}",
        resp.text()
    );
    let account = resp.json_value();
    let account_id_str = account["id"].as_str().expect("id field missing");
    let account_id = Uuid::parse_str(account_id_str).expect("invalid uuid");

    // Create a second account of type `unit_ledger` for get_unit_ledger testing
    let create_ledger_payload = json!({
        "organization_id": org_id,
        "building_id": building_id,
        "unit_id": unit_id,
        "name": "Unit Ledger Account",
        "account_type": "unit_ledger",
        "description": "Ledger for U-HAPPY-1",
        "currency": "EUR",
        "opening_balance": "0.00"
    });
    let resp_ledger = app
        .execute(
            app.post("/api/v1/financial/accounts")
                .bearer(&token)
                .json(&create_ledger_payload)
                .build(),
        )
        .await;
    assert_eq!(resp_ledger.status, StatusCode::CREATED);

    // 1.2 GET /api/v1/financial/accounts -> list_accounts
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/accounts?organization_id={org_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let accounts_list = resp.json_value();
    assert!(accounts_list
        .as_array()
        .map(|arr| arr.len() >= 2)
        .unwrap_or(false));

    // 1.3 GET /api/v1/financial/accounts/{id} -> get_account
    let resp = app
        .execute(
            app.get(&format!("/api/v1/financial/accounts/{account_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let account_details = resp.json_value();
    assert_eq!(account_details["account"]["id"], account_id_str);

    // 1.4 POST /api/v1/financial/accounts/{id}/transactions -> create_transaction
    let create_tx_payload = json!({
        "account_id": account_id,
        "amount": "100.00",
        "transaction_type": "credit",
        "category": "other",
        "description": "Initial Deposit Match"
    });
    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/financial/accounts/{account_id}/transactions"
            ))
            .bearer(&token)
            .json(&create_tx_payload)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED);

    // 1.5 GET /api/v1/financial/accounts/{id}/transactions -> list_transactions
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/accounts/{account_id}/transactions"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let tx_list = resp.json_value();
    assert!(tx_list
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));

    // 1.6 GET /api/v1/financial/units/{unit_id}/ledger -> get_unit_ledger
    let resp = app
        .execute(
            app.get(&format!("/api/v1/financial/units/{unit_id}/ledger"))
                .bearer(&token)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_unit_ledger should succeed: {}",
        resp.text()
    );
    let ledger_details = resp.json_value();
    assert_eq!(ledger_details["account"]["account_type"], "unit_ledger");

    // ========================================================================
    // 2. FEE SCHEDULES (Story 11.2)
    // ========================================================================

    // 2.1 POST /api/v1/financial/fee-schedules -> create_fee_schedule
    let create_schedule_payload = json!({
        "organization_id": org_id,
        "user_id": user_id,
        "building_id": building_id,
        "name": "Standard Maintenance Fee",
        "description": "Monthly building upkeep fee",
        "amount": "150.00",
        "currency": "EUR",
        "frequency": "monthly",
        "unit_filter": {},
        "billing_day": 1,
        "effective_from": "2026-01-01"
    });
    let resp = app
        .execute(
            app.post("/api/v1/financial/fee-schedules")
                .bearer(&token)
                .json(&create_schedule_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_fee_schedule should succeed: {}",
        resp.text()
    );
    let schedule = resp.json_value();
    let schedule_id_str = schedule["id"].as_str().expect("id missing");
    let schedule_id = Uuid::parse_str(schedule_id_str).expect("invalid schedule uuid");

    // 2.2 GET /api/v1/financial/fee-schedules -> list_fee_schedules
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/fee-schedules?organization_id={org_id}&building_id={building_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let schedules_list = resp.json_value();
    assert!(schedules_list
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));

    // 2.3 GET /api/v1/financial/fee-schedules/{id} -> get_fee_schedule
    let resp = app
        .execute(
            app.get(&format!("/api/v1/financial/fee-schedules/{schedule_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    // 2.4 POST /api/v1/financial/units/{unit_id}/fees -> assign_unit_fee
    let assign_fee_payload = json!({
        "organization_id": org_id,
        "fee_schedule_id": schedule_id,
        "effective_from": "2026-01-01"
    });
    let resp = app
        .execute(
            app.post(&format!("/api/v1/financial/units/{unit_id}/fees"))
                .bearer(&token)
                .json(&assign_fee_payload)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED);

    // 2.5 GET /api/v1/financial/units/{unit_id}/fees -> get_unit_fees
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/units/{unit_id}/fees?organization_id={org_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let unit_fees_list = resp.json_value();
    assert!(unit_fees_list
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));

    // ========================================================================
    // 3. INVOICES & PAYMENTS (Story 11.3, 11.4, 11.5)
    // ========================================================================

    // 3.1 POST /api/v1/financial/invoices -> create_invoice
    let create_invoice_payload = json!({
        "organization_id": org_id,
        "user_id": user_id,
        "unit_id": unit_id,
        "due_date": "2026-07-31",
        "currency": "EUR",
        "items": [
            {
                "description": "Monthly Maintenance Fee July 2026",
                "quantity": "1.00",
                "unit_price": "150.00"
            }
        ]
    });
    let resp = app
        .execute(
            app.post("/api/v1/financial/invoices")
                .bearer(&token)
                .json(&create_invoice_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_invoice should succeed: {}",
        resp.text()
    );
    let invoice = resp.json_value();
    let invoice_id_str = invoice["invoice"]["id"].as_str().expect("id missing");
    let invoice_id = Uuid::parse_str(invoice_id_str).expect("invalid invoice uuid");

    // 3.2 GET /api/v1/financial/invoices -> list_invoices
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/invoices?organization_id={org_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    // 3.3 GET /api/v1/financial/invoices/{id} -> get_invoice
    let resp = app
        .execute(
            app.get(&format!("/api/v1/financial/invoices/{invoice_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    // 3.4 GET /api/v1/financial/units/{unit_id}/invoices -> list_unit_invoices
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/units/{unit_id}/invoices?organization_id={org_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    // 3.5 POST /api/v1/financial/invoices/{id}/send -> send_invoice
    let resp = app
        .execute(
            app.post(&format!("/api/v1/financial/invoices/{invoice_id}/send"))
                .bearer(&token)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "send_invoice should succeed: {}",
        resp.text()
    );

    // 3.6 GET /api/v1/financial/invoices/{id}/pdf -> get_invoice_pdf
    let resp = app
        .execute(
            app.get(&format!("/api/v1/financial/invoices/{invoice_id}/pdf"))
                .bearer(&token)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(
        resp.headers
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok()),
        Some("application/pdf")
    );

    // 3.7 POST /api/v1/financial/invoices/{id}/checkout -> initiate_invoice_checkout
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/v1/checkout/sessions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "cs_happy_test_session_id",
            "url": "https://checkout.stripe.test/cs_happy_test_session_id"
        })))
        .expect(1)
        .mount(&stripe_server)
        .await;

    let resp = app
        .execute(
            app.post(&format!("/api/v1/financial/invoices/{invoice_id}/checkout"))
                .bearer(&token)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "checkout initiation should succeed: {}",
        resp.text()
    );
    let checkout_res = resp.json_value();
    assert_eq!(
        checkout_res["checkout_url"],
        "https://checkout.stripe.test/cs_happy_test_session_id"
    );

    // 3.8 POST /api/v1/financial/payments -> record_payment (Allocates to outstanding invoices)
    let record_payment_payload = json!({
        "organization_id": org_id,
        "user_id": user_id,
        "unit_id": unit_id,
        "amount": "150.00",
        "currency": "EUR",
        "payment_method": "bank_transfer",
        "invoice_ids": []
    });
    let resp = app
        .execute(
            app.post("/api/v1/financial/payments")
                .bearer(&token)
                .json(&record_payment_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "record_payment should succeed: {}",
        resp.text()
    );
    let payment = resp.json_value();
    let payment_id_str = payment["payment"]["id"].as_str().expect("id missing");
    let payment_id = Uuid::parse_str(payment_id_str).expect("invalid payment uuid");

    // 3.9 GET /api/v1/financial/payments -> list_payments
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/payments?organization_id={org_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    // 3.10 GET /api/v1/financial/payments/{id} -> get_payment
    let resp = app
        .execute(
            app.get(&format!("/api/v1/financial/payments/{payment_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    // 3.11 GET /api/v1/financial/units/{unit_id}/payments -> list_unit_payments
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/units/{unit_id}/payments?organization_id={org_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);

    // ========================================================================
    // 4. UNALLOCATED PAYMENTS & MANUAL ALLOCATION / AUTO-MATCH
    // ========================================================================

    // Create a new payment with no matching invoice, so it stays unallocated
    let record_unallocated_payload = json!({
        "organization_id": org_id,
        "user_id": user_id,
        "unit_id": unit_id,
        "amount": "50.00",
        "currency": "EUR",
        "payment_method": "cash",
        "invoice_ids": []
    });
    let resp = app
        .execute(
            app.post("/api/v1/financial/payments")
                .bearer(&token)
                .json(&record_unallocated_payload)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED);
    let unallocated_payment = resp.json_value();
    let unallocated_payment_id_str = unallocated_payment["id"].as_str().expect("id missing");
    let unallocated_payment_id =
        Uuid::parse_str(unallocated_payment_id_str).expect("invalid payment uuid");

    // 4.1 GET /api/v1/financial/payments/unallocated -> list_unallocated_payments
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/financial/payments/unallocated?organization_id={org_id}"
            ))
            .bearer(&token)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let unallocated_list = resp.json_value();
    assert!(unallocated_list
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));

    // Create a second invoice for manual allocation
    let create_invoice2_payload = json!({
        "organization_id": org_id,
        "user_id": user_id,
        "unit_id": unit_id,
        "due_date": "2026-08-31",
        "currency": "EUR",
        "items": [
            {
                "description": "Additional Fee Item",
                "quantity": "1.00",
                "unit_price": "50.00"
            }
        ]
    });
    let resp = app
        .execute(
            app.post("/api/v1/financial/invoices")
                .bearer(&token)
                .json(&create_invoice2_payload)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED);
    let invoice2 = resp.json_value();
    let invoice2_id_str = invoice2["invoice"]["id"].as_str().expect("id missing");
    let invoice2_id = Uuid::parse_str(invoice2_id_str).expect("invalid invoice uuid");

    // 4.2 POST /api/v1/financial/payments/{id}/allocate -> allocate_payment
    let allocate_payload = json!({
        "organization_id": org_id,
        "invoice_id": invoice2_id,
        "amount": "50.00"
    });
    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/financial/payments/{unallocated_payment_id}/allocate"
            ))
            .bearer(&token)
            .json(&allocate_payload)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "allocate_payment should succeed: {}",
        resp.text()
    );

    // 4.3 POST /api/v1/financial/payments/auto-match -> auto_match_payments
    // We create a third unallocated payment and a third unpaid invoice, both matching 30.00 EUR
    let record_unallocated3_payload = json!({
        "organization_id": org_id,
        "user_id": user_id,
        "unit_id": unit_id,
        "amount": "30.00",
        "currency": "EUR",
        "payment_method": "bank_transfer",
        "invoice_ids": []
    });
    let resp = app
        .execute(
            app.post("/api/v1/financial/payments")
                .bearer(&token)
                .json(&record_unallocated3_payload)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED);

    let create_invoice3_payload = json!({
        "organization_id": org_id,
        "user_id": user_id,
        "unit_id": unit_id,
        "due_date": "2026-09-30",
        "currency": "EUR",
        "items": [
            {
                "description": "Third Auto-match Fee Item",
                "quantity": "1.00",
                "unit_price": "30.00"
            }
        ]
    });
    let resp = app
        .execute(
            app.post("/api/v1/financial/invoices")
                .bearer(&token)
                .json(&create_invoice3_payload)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED);

    let auto_match_payload = json!({
        "organization_id": org_id
    });
    let resp = app
        .execute(
            app.post("/api/v1/financial/payments/auto-match")
                .bearer(&token)
                .json(&auto_match_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "auto_match_payments should succeed: {}",
        resp.text()
    );
    let auto_match_res = resp.json_value();
    assert!(auto_match_res["matched"].as_i64().expect("matched missing") >= 1);
}
