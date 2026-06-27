//! Integration tests for native accounting bank statements and reconciliation (PAP-206).
//! Exposes and tests GET/POST/CONFIRM/REJECT for bank statements and matches.

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, seed_org, TestApp};

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
         VALUES ($1, 'test_hash', 'PAP206 User', 'active', NOW()) RETURNING id",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_contact(pool: &PgPool, tenant_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO contact (tenant_id, name) VALUES ($1, $2) RETURNING id",
    )
    .bind(tenant_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed contact")
}

fn mint_jwt(user_id: Uuid, role: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;
    #[derive(Serialize)]
    struct Claims {
        sub: String,
        email: String,
        name: String,
        exp: i64,
        iat: i64,
        jti: String,
        token_type: String,
        role: String,
    }
    let now = chrono::Utc::now().timestamp();
    encode(
        &Header::default(),
        &Claims {
            sub: user_id.to_string(),
            email: "pap206@test.example".into(),
            name: "PAP206".into(),
            exp: now + 900,
            iat: now,
            jti: Uuid::new_v4().to_string(),
            token_type: "access".into(),
            role: role.into(),
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint_jwt")
}

// Helper to seed an invoice directly in the db
async fn seed_invoice(
    pool: &PgPool,
    tenant_id: Uuid,
    contact_id: Uuid,
    number: &str,
    total_amount: Decimal,
    variable_symbol: Option<&str>,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO invoice (
            tenant_id, contact_id, number, issue_date, due_date, currency, total_amount, base_amount, status, variable_symbol
        )
        VALUES ($1, $2, $3, '2026-01-01', '2026-01-15', 'EUR', $4, $4, 'issued', $5)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(contact_id)
    .bind(number)
    .bind(total_amount)
    .bind(variable_symbol)
    .fetch_one(pool)
    .await
    .expect("seed invoice")
}

fn build_multipart_csv(file_bytes: &[u8], filename: &str) -> (String, Vec<u8>) {
    let boundary = format!("testboundary{}", Uuid::new_v4().simple());
    let mut body: Vec<u8> = Vec::new();

    let file_header = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: text/csv\r\n\r\n"
    );
    body.extend_from_slice(file_header.as_bytes());
    body.extend_from_slice(file_bytes);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

    (format!("multipart/form-data; boundary={boundary}"), body)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_statement_and_trigger_matching(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "recon-upload").await;
    let manager_id = seed_user(&pool, "manager@recon-upload.test").await;
    seed_membership(&pool, org_id, manager_id, "manager").await;
    let token = mint_jwt(manager_id, "manager");
    
    // Seed an open invoice to match with variable symbol "2026001" and amount 120.00
    let contact_id = seed_contact(&pool, org_id, "Recon CRM").await;
    let inv_id = seed_invoice(&pool, org_id, contact_id, "INV-2026-001", Decimal::from(120), Some("2026001")).await;

    // CSV bytes content
    let csv_content = b"date,amount,currency,counterparty_iban,variable_symbol,reference\n2026-06-01,120.00,EUR,SK1111111111111111111111,2026001,rent payment\n";
    let (content_type, body_bytes) = build_multipart_csv(csv_content, "statement_june.csv");

    // 1. Resident cannot upload
    let resident_id = seed_user(&pool, "resident@recon-upload.test").await;
    seed_membership(&pool, org_id, resident_id, "resident").await;
    let resident_token = mint_jwt(resident_id, "resident");
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/accounting/statements")
        .header(header::AUTHORIZATION, format!("Bearer {resident_token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::CONTENT_TYPE, &content_type)
        .body(Body::from(body_bytes.clone()))
        .unwrap();
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN);

    // 2. Manager uploads statement
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/accounting/statements")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::CONTENT_TYPE, &content_type)
        .body(Body::from(body_bytes))
        .unwrap();
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    
    let stmt: serde_json::Value = resp.json();
    assert_eq!(stmt["source_filename"], "statement_june.csv");
    let stmt_uuid = Uuid::parse_str(stmt["id"].as_str().unwrap()).unwrap();

    // 3. List statements
    let req = app.get("/api/v1/accounting/statements")
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let list: serde_json::Value = resp.json();
    assert_eq!(list.as_array().unwrap().len(), 1);

    // 4. List statement lines
    let req = app.get(&format!("/api/v1/accounting/statements/{stmt_uuid}/lines"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let lines: serde_json::Value = resp.json();
    assert_eq!(lines.as_array().unwrap().len(), 1);
    assert_eq!(lines[0]["match_state"], "suggested"); // Match engine should have set to suggested
    
    let line_uuid = Uuid::parse_str(lines[0]["id"].as_str().unwrap()).unwrap();

    // 5. List matches for the line
    let req = app.get(&format!("/api/v1/accounting/lines/{line_uuid}/matches"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    let matches: serde_json::Value = resp.json();
    assert_eq!(matches.as_array().unwrap().len(), 1);
    assert_eq!(matches[0]["state"], "suggested");
    assert_eq!(matches[0]["invoice_id"], inv_id.to_string());
    
    let match_uuid = Uuid::parse_str(matches[0]["id"].as_str().unwrap()).unwrap();

    // 6. Confirm the match
    let req = app.post(&format!("/api/v1/accounting/matches/{match_uuid}/confirm"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);

    // Check line is now matched
    let line_state: String = sqlx::query_scalar("SELECT match_state FROM bank_statement_line WHERE id = $1")
        .bind(line_uuid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(line_state, "matched");

    // Check invoice is now paid
    let inv_status: String = sqlx::query_scalar("SELECT status FROM invoice WHERE id = $1")
        .bind(inv_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(inv_status, "paid");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reject_payment_match(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "recon-reject").await;
    let manager_id = seed_user(&pool, "manager@recon-reject.test").await;
    seed_membership(&pool, org_id, manager_id, "manager").await;
    let token = mint_jwt(manager_id, "manager");
    
    // Seed an open invoice
    let contact_id = seed_contact(&pool, org_id, "Recon CRM").await;
    let _inv_id = seed_invoice(&pool, org_id, contact_id, "INV-2026-002", Decimal::from(100), Some("2026002")).await;

    // Upload statement containing variable symbol "2026002"
    let csv_content = b"date,amount,currency,counterparty_iban,variable_symbol,reference\n2026-06-01,100.00,EUR,,2026002,rent payment\n";
    let (content_type, body_bytes) = build_multipart_csv(csv_content, "stmt.csv");

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/accounting/statements")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .header(header::CONTENT_TYPE, &content_type)
        .body(Body::from(body_bytes))
        .unwrap();
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);
    
    let stmt: serde_json::Value = resp.json();
    let stmt_uuid = Uuid::parse_str(stmt["id"].as_str().unwrap()).unwrap();

    // Get line uuid
    let lines_resp = app.execute(
        app.get(&format!("/api/v1/accounting/statements/{stmt_uuid}/lines"))
            .bearer(&token)
            .tenant(org_id)
            .build()
    ).await;
    let lines: serde_json::Value = lines_resp.json();
    let line_uuid = Uuid::parse_str(lines[0]["id"].as_str().unwrap()).unwrap();

    // Get match uuid
    let matches_resp = app.execute(
        app.get(&format!("/api/v1/accounting/lines/{line_uuid}/matches"))
            .bearer(&token)
            .tenant(org_id)
            .build()
    ).await;
    let matches: serde_json::Value = matches_resp.json();
    let match_uuid = Uuid::parse_str(matches[0]["id"].as_str().unwrap()).unwrap();

    // Reject match
    let req = app.post(&format!("/api/v1/accounting/matches/{match_uuid}/reject"))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK);

    // Assert statement line goes back to unmatched
    let line_state: String = sqlx::query_scalar("SELECT match_state FROM bank_statement_line WHERE id = $1")
        .bind(line_uuid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(line_state, "unmatched");
}
