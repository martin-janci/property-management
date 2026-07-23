//! Happy-path integration tests for the accounting endpoints
//! (`/api/v1/accounting/{invoices,statements,lines,matches}`).
//!
//! The accounting module is manager-gated through `RlsConnection` (resolved from
//! the `X-Tenant-ID` header + an `organization_members` row) and, for writes,
//! the `tenant_id` JWT claim on `AuthUser`. `org_admin` (level 90) satisfies the
//! `Manager` (level 80) gate, so `create_authenticated_user_with_org` provides a
//! suitable principal; for `create_invoice` we additionally mint a token that
//! carries the `tenant_id` claim.
//!
//! Coverage is complementary to `accounting_contacts_authz_tests.rs`
//! (`list_contacts`) and `accounting_payment_match_state_machine_tests.rs`
//! (service-level, not HTTP). Endpoints covered (BIT-341, Wave 5):
//!   1. POST   /api/v1/accounting/invoices                 (create_invoice)
//!   2. GET    /api/v1/accounting/invoices                 (list_invoices)
//!   3. GET    /api/v1/accounting/invoices/{id}            (get_invoice)
//!   4. GET    /api/v1/accounting/invoices/{id}/items      (list_invoice_items)
//!   5. PATCH  /api/v1/accounting/invoices/{id}            (update_invoice)
//!   6. GET    /api/v1/accounting/statements               (list_statements)
//!   7. GET    /api/v1/accounting/statements/{id}/lines    (list_statement_lines)
//!   8. GET    /api/v1/accounting/lines/{id}/matches       (list_matches)
//!   9. POST   /api/v1/accounting/matches/{id}/confirm     (confirm_match)
//!  10. POST   /api/v1/accounting/matches/{id}/reject      (reject_match)
//!  11. DELETE /api/v1/accounting/invoices/{id}            (delete_invoice)

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

/// Mint an access token carrying a `tenant_id` claim, matching the on-the-wire
/// shape the `AuthUser` extractor expects. `create_invoice` requires the claim;
/// the rest of the endpoints resolve their tenant from the `X-Tenant-ID` header.
fn mint_tenant_token(user_id: Uuid, org_id: Uuid) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Claims {
        sub: String,
        exp: i64,
        iat: i64,
        token_type: String,
        tenant_id: String,
        role: Option<String>,
        email: String,
        name: String,
    }

    let now = chrono::Utc::now().timestamp();
    encode(
        &Header::default(),
        &Claims {
            sub: user_id.to_string(),
            exp: now + 900,
            iat: now,
            token_type: "access".into(),
            tenant_id: org_id.to_string(),
            role: None,
            email: "acc-happy@test.example".into(),
            name: "Accounting Happy".into(),
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint tenant token")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accounting_endpoints_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = TestUser::new();
    let (_login_token, org_id) = create_authenticated_user_with_org(&app, &user, "acchappy").await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    // Token carrying the tenant_id claim (required by create_invoice); it is a
    // valid access token so it also works for the RlsConnection-gated reads.
    let token = mint_tenant_token(user_id, org_id);
    let tenant = org_id.to_string();

    // Seed a billing contact (FK target for invoices).
    let contact_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, 'Acme Tenant') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed contact");

    // ========================================================================
    // 1. POST /api/v1/accounting/invoices -> create_invoice
    // ========================================================================
    let create_payload = json!({
        "tenant_id": org_id,
        "contact_id": contact_id,
        "number": "INV-HAPPY-1",
        "issue_date": "2026-01-01",
        "due_date": "2026-01-31",
        "currency": "CZK",
        "items": [
            { "description": "Consulting", "qty": "1", "unit_price": "100", "vat_rate": "0" }
        ]
    });
    let resp = app
        .execute(
            app.post("/api/v1/accounting/invoices")
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .json(&create_payload)
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
    let invoice_id = invoice["id"]
        .as_str()
        .expect("invoice id missing")
        .to_string();

    // ========================================================================
    // 2. GET /api/v1/accounting/invoices -> list_invoices
    // ========================================================================
    let resp = app
        .execute(
            app.get("/api/v1/accounting/invoices")
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_invoices should succeed: {}",
        resp.text()
    );
    assert!(resp
        .json_value()
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false));

    // 3. GET /api/v1/accounting/invoices/{id} -> get_invoice
    let resp = app
        .execute(
            app.get(&format!("/api/v1/accounting/invoices/{invoice_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_invoice should succeed: {}",
        resp.text()
    );

    // 4. GET /api/v1/accounting/invoices/{id}/items -> list_invoice_items
    let resp = app
        .execute(
            app.get(&format!("/api/v1/accounting/invoices/{invoice_id}/items"))
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_invoice_items should succeed: {}",
        resp.text()
    );
    assert!(resp
        .json_value()
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false));

    // 5. PATCH /api/v1/accounting/invoices/{id} -> update_invoice
    let resp = app
        .execute(
            app.patch(&format!("/api/v1/accounting/invoices/{invoice_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .json(json!({ "variable_symbol": "VS12345" }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_invoice should succeed: {}",
        resp.text()
    );

    // ========================================================================
    // Seed a bank statement + line + suggested match (mirrors the proven
    // service-level seeding) referencing a dedicated issued invoice so the
    // statement / match read + confirm/reject endpoints have real data.
    // ========================================================================
    let match_invoice = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency, total_amount, status) \
         VALUES ($1, $2, 'INV-MATCH-1', NOW(), NOW(), 'CZK', 100, 'issued') RETURNING id",
    )
    .bind(org_id)
    .bind(contact_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed match invoice");

    let statement_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO bank_statement (tenant_id, source_filename, account_iban) \
         VALUES ($1, 'statement.csv', 'SK0000000000000000000000') RETURNING id",
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed bank statement");

    let line_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO bank_statement_line (statement_id, tenant_id, booking_date, amount, currency, match_state) \
         VALUES ($1, $2, NOW(), 100, 'CZK', 'suggested') RETURNING id",
    )
    .bind(statement_id)
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed statement line");

    let match_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO payment_match (tenant_id, statement_line_id, invoice_id, confidence, state) \
         VALUES ($1, $2, $3, 1.0, 'suggested') RETURNING id",
    )
    .bind(org_id)
    .bind(line_id)
    .bind(match_invoice)
    .fetch_one(&app.pool)
    .await
    .expect("seed payment match");

    // 6. GET /api/v1/accounting/statements -> list_statements
    let resp = app
        .execute(
            app.get("/api/v1/accounting/statements")
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_statements should succeed: {}",
        resp.text()
    );
    assert!(resp
        .json_value()
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false));

    // 7. GET /api/v1/accounting/statements/{id}/lines -> list_statement_lines
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/accounting/statements/{statement_id}/lines"
            ))
            .bearer(&token)
            .header("X-Tenant-ID", &tenant)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_statement_lines should succeed: {}",
        resp.text()
    );
    assert!(resp
        .json_value()
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false));

    // 8. GET /api/v1/accounting/lines/{id}/matches -> list_matches
    let resp = app
        .execute(
            app.get(&format!("/api/v1/accounting/lines/{line_id}/matches"))
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_matches should succeed: {}",
        resp.text()
    );
    assert!(resp
        .json_value()
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false));

    // 9. POST /api/v1/accounting/matches/{id}/confirm -> confirm_match
    let resp = app
        .execute(
            app.post(&format!("/api/v1/accounting/matches/{match_id}/confirm"))
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "confirm_match should succeed: {}",
        resp.text()
    );

    // 10. POST /api/v1/accounting/matches/{id}/reject -> reject_match
    //     Rejecting a now-confirmed match unapplies it and returns 200.
    let resp = app
        .execute(
            app.post(&format!("/api/v1/accounting/matches/{match_id}/reject"))
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "reject_match should succeed: {}",
        resp.text()
    );

    // 11. DELETE /api/v1/accounting/invoices/{id} -> delete_invoice
    //     Delete the first invoice (no payment_match references it).
    let resp = app
        .execute(
            app.delete(&format!("/api/v1/accounting/invoices/{invoice_id}"))
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete_invoice should succeed: {}",
        resp.text()
    );
}
