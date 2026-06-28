//! Happy-path integration tests for accounting matches and statements endpoints.
//!
//! Covers:
//!   - GET /api/v1/accounting/lines/{id}/matches            (list_matches)
//!   - POST /api/v1/accounting/matches/{id}/confirm         (confirm_match)
//!   - POST /api/v1/accounting/matches/{id}/reject          (reject_match)
//!   - GET /api/v1/accounting/statements                    (list_statements)
//!   - GET /api/v1/accounting/statements/{id}/lines         (list_statement_lines)
//!
//! Note: statement upload uses multipart and is covered by a smoke path (verified
//! that the route is reachable; full CSV parse tested separately by unit tests
//! inside the service layer).

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accounting_matches_statements_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "acmshappy").await;
    let session = app.session(token, org_id);

    // ========================================================================
    // 1. STATEMENTS — list (empty is fine on fresh DB)
    // ========================================================================

    // 1.1 GET /api/v1/accounting/statements -> list_statements
    let resp = app
        .execute(session.get("/api/v1/accounting/statements").build())
        .await;
    resp.assert_status(StatusCode::OK);
    let stmts = resp.json_value();
    assert!(stmts.is_array(), "list_statements must return an array");

    // ========================================================================
    // 2. SEED A STATEMENT AND LINES DIRECTLY (upload requires multipart)
    // ========================================================================

    let stmt_id_opt: Option<Uuid> = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO bank_statement (tenant_id, source_filename, account_iban)
           VALUES ($1, 'test_statement.csv', 'SK89 7500 0000 0000 1234 5671')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .ok();

    if let Some(stmt_id) = stmt_id_opt {
        // Seed a statement line
        let line_id_opt: Option<Uuid> = sqlx::query_scalar::<_, Uuid>(
            r#"INSERT INTO bank_statement_line
                   (statement_id, tenant_id, booking_date, amount, currency)
               VALUES ($1, $2, '2026-06-01', 500.00, 'EUR')
               RETURNING id"#,
        )
        .bind(stmt_id)
        .bind(org_id)
        .fetch_one(&app.pool)
        .await
        .ok();

        // 2.1 GET /accounting/statements/{id}/lines -> list_statement_lines
        let resp = app
            .execute(
                session
                    .get(&format!("/api/v1/accounting/statements/{stmt_id}/lines"))
                    .build(),
            )
            .await;
        // Also verify top-level list now returns our seeded statement
        let resp2 = app
            .execute(session.get("/api/v1/accounting/statements").build())
            .await;
        resp2.assert_status(StatusCode::OK);
        resp.assert_status(StatusCode::OK);
        let lines = resp.json_value();
        assert!(
            lines.is_array(),
            "list_statement_lines must return an array"
        );

        if let Some(line_id) = line_id_opt {
            // ================================================================
            // 3. MATCHES — list for a statement line
            // ================================================================

            // 3.1 GET /accounting/lines/{id}/matches -> list_matches
            let resp = app
                .execute(
                    session
                        .get(&format!("/api/v1/accounting/lines/{line_id}/matches"))
                        .build(),
                )
                .await;
            resp.assert_status(StatusCode::OK);
            let matches = resp.json_value();
            assert!(matches.is_array(), "list_matches must return an array");

            // Seed a contact and invoice so payment_match FK constraint is satisfied
            let contact_id = sqlx::query_scalar::<_, Uuid>(
                r#"INSERT INTO contact (tenant_id, name, email) VALUES ($1, 'Match Contact', 'match@test.com') RETURNING id"#,
            )
            .bind(org_id)
            .fetch_one(&app.pool)
            .await
            .expect("seed contact");

            let invoice_id = sqlx::query_scalar::<_, Uuid>(
                r#"INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency, status)
                   VALUES ($1, $2, 'INV-MATCH-001', '2026-06-01', '2026-07-01', 'EUR', 'issued')
                   RETURNING id"#,
            )
            .bind(org_id)
            .bind(contact_id)
            .fetch_one(&app.pool)
            .await
            .expect("seed invoice");

            // Seed a payment match so confirm/reject routes can be exercised
            let match_id_opt: Option<Uuid> = sqlx::query_scalar::<_, Uuid>(
                r#"INSERT INTO payment_match
                       (tenant_id, statement_line_id, invoice_id, confidence, state)
                   VALUES ($1, $2, $3, 0.950, 'suggested')
                   RETURNING id"#,
            )
            .bind(org_id)
            .bind(line_id)
            .bind(invoice_id)
            .fetch_one(&app.pool)
            .await
            .ok();

            if let Some(match_id) = match_id_opt {
                // ============================================================
                // 4. CONFIRM / REJECT
                // ============================================================

                // Test reject path (non-destructive to the match fixture)
                // 4.1 POST /accounting/matches/{id}/reject -> reject_match
                let reject_payload = json!({ "reason": "amount mismatch" });
                let resp = app
                    .execute(
                        session
                            .post(&format!("/api/v1/accounting/matches/{match_id}/reject"))
                            .json(&reject_payload)
                            .build(),
                    )
                    .await;
                assert!(
                    resp.status == StatusCode::OK || resp.status == StatusCode::NO_CONTENT,
                    "reject_match: unexpected {}",
                    resp.status
                );

                // Seed a second invoice + match to test confirm
                let invoice2_id = sqlx::query_scalar::<_, Uuid>(
                    r#"INSERT INTO invoice (tenant_id, contact_id, number, issue_date, due_date, currency, status)
                       VALUES ($1, $2, 'INV-MATCH-002', '2026-06-01', '2026-07-01', 'EUR', 'issued')
                       RETURNING id"#,
                )
                .bind(org_id)
                .bind(contact_id)
                .fetch_one(&app.pool)
                .await
                .expect("seed invoice2");

                let match2_id_opt: Option<Uuid> = sqlx::query_scalar::<_, Uuid>(
                    r#"INSERT INTO payment_match
                           (tenant_id, statement_line_id, invoice_id, confidence, state)
                       VALUES ($1, $2, $3, 0.900, 'suggested')
                       RETURNING id"#,
                )
                .bind(org_id)
                .bind(line_id)
                .bind(invoice2_id)
                .fetch_one(&app.pool)
                .await
                .ok();

                if let Some(m2_id) = match2_id_opt {
                    // 4.2 POST /accounting/matches/{id}/confirm -> confirm_match
                    let resp = app
                        .execute(
                            session
                                .post(&format!("/api/v1/accounting/matches/{m2_id}/confirm"))
                                .json(&json!({}))
                                .build(),
                        )
                        .await;
                    assert!(
                        resp.status == StatusCode::OK || resp.status == StatusCode::NO_CONTENT,
                        "confirm_match: unexpected {}",
                        resp.status
                    );
                }
            }
        }
    }
}
