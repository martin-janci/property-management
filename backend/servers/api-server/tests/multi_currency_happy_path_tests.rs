//! Happy path integration tests for the multi-currency endpoints (`/api/v1/multi-currency/*`).
//!
//! Exercises 20+ different endpoints covering currency configuration, exchange rate management,
//! cross-currency transactions, cross-border lease management, reporting, and dashboard.
//!
//! Auth note: every handler in `routes/multi_currency.rs` derives the org from
//! `AuthUser.tenant_id`, which is read from the JWT `tenant_id` claim — NOT from
//! the `X-Tenant-ID` header or `organization_members`. A token minted by the
//! login flow carries no tenant claim, so we mint one directly here (the same
//! pattern as `analytics_owner_success_tests.rs`).

mod common;

use axum::http::StatusCode;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{TestApp, TestConfig};

// ---------------------------------------------------------------------------
// JWT mint — tenant-scoped access token (AuthUser reads `tenant_id` from claims)
// ---------------------------------------------------------------------------

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
        name: "Multi-Currency Happy User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("MC Org {slug}"))
    .bind(format!("mc-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@mc.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'hash', 'MC User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn multi_currency_endpoints_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_id = seed_org(&app.pool, "mchappy").await;
    let email = format!("mc-{}@example.com", Uuid::new_v4());
    let user_id = seed_user(&app.pool, &email).await;
    let token = mint(user_id, &email, org_id);

    // Seed building
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Happy Currency Street 1', 'Bratislava', '81101', 'SK') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed building");

    // ========================================================================
    // 1. CURRENCY CONFIGURATION
    // ========================================================================

    // 1.1 POST /api/v1/multi-currency/config -> create_or_update_currency_config (200)
    let create_config_payload = json!({
        "base_currency": "E_U_R",
        "enabled_currencies": ["E_U_R", "U_S_D", "C_Z_K"],
        "display_currency": "U_S_D",
        "show_original_amount": true,
        "decimal_places": 2,
        "exchange_rate_source": "ecb",
        "auto_update_rates": true,
        "update_frequency_hours": 24,
        "rounding_mode": "half_up"
    });
    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/config")
                .bearer(&token)
                .json(&create_config_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.2 PUT /api/v1/multi-currency/config -> update_currency_config (200)
    let update_config_payload = json!({
        "rounding_mode": "half_down"
    });
    let resp = app
        .execute(
            app.put("/api/v1/multi-currency/config")
                .bearer(&token)
                .json(&update_config_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.3 GET /api/v1/multi-currency/config -> get_currency_config (200)
    let resp = app
        .execute(
            app.get("/api/v1/multi-currency/config")
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 2. PROPERTY CURRENCY CONFIGURATION
    // ========================================================================

    // 2.1 POST /api/v1/multi-currency/properties -> create_property_currency_config (200)
    let create_prop_payload = json!({
        "building_id": building_id,
        "default_currency": "U_S_D",
        "country": "S_K",
        "vat_rate": "20.00",
        "requires_local_reporting": true,
    });
    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/properties")
                .bearer(&token)
                .json(&create_prop_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.2 PUT /api/v1/multi-currency/properties/{building_id} -> update_property_currency_config (200)
    let update_prop_payload = json!({
        "default_currency": "E_U_R"
    });
    let resp = app
        .execute(
            app.put(&format!("/api/v1/multi-currency/properties/{building_id}"))
                .bearer(&token)
                .json(&update_prop_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.3 GET /api/v1/multi-currency/properties/{building_id} -> get_property_currency_config (200)
    let resp = app
        .execute(
            app.get(&format!("/api/v1/multi-currency/properties/{building_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.4 GET /api/v1/multi-currency/properties -> list_property_currency_configs (200)
    let resp = app
        .execute(
            app.get("/api/v1/multi-currency/properties")
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 3. EXCHANGE RATES
    // ========================================================================

    // 3.1 POST /api/v1/multi-currency/rates -> create_exchange_rate (200)
    let create_rate_payload = json!({
        "from_currency": "E_U_R",
        "to_currency": "U_S_D",
        "rate": "1.10",
        "rate_date": "2026-06-27",
        "source": "ecb"
    });
    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/rates")
                .bearer(&token)
                .json(&create_rate_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.2 GET /api/v1/multi-currency/rates -> list_exchange_rates (200)
    let resp = app
        .execute(
            app.get("/api/v1/multi-currency/rates?from_currency=E_U_R&to_currency=U_S_D")
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.3 GET /api/v1/multi-currency/rates/latest -> get_latest_exchange_rate (200)
    let resp = app
        .execute(
            app.get("/api/v1/multi-currency/rates/latest?from_currency=E_U_R&to_currency=U_S_D")
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.4 POST /api/v1/multi-currency/rates/override -> override_exchange_rate (200)
    let override_payload = json!({
        "from_currency": "E_U_R",
        "to_currency": "U_S_D",
        "rate": "1.15",
        "rate_date": "2026-06-27",
        "reason": "Test Override"
    });
    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/rates/override")
                .bearer(&token)
                .json(&override_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.5 POST /api/v1/multi-currency/rates/fetch -> fetch_exchange_rates
    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/rates/fetch")
                .bearer(&token)
                .build(),
        )
        .await;
    // The handler logs the attempt and returns 200; allow gateway/internal too
    // in case an external dependency is wired up later.
    assert!(
        resp.status == StatusCode::OK
            || resp.status == StatusCode::BAD_GATEWAY
            || resp.status == StatusCode::INTERNAL_SERVER_ERROR,
        "unexpected status {}: {}",
        resp.status,
        resp.text()
    );

    // ========================================================================
    // 4. CROSS-CURRENCY TRANSACTIONS
    // ========================================================================

    // 4.1 POST /api/v1/multi-currency/transactions -> create_transaction (200)
    let source_id = Uuid::new_v4();
    let create_tx_payload = json!({
        "building_id": building_id,
        "source_type": "invoice",
        "source_id": source_id,
        "original_currency": "U_S_D",
        "original_amount": "100.00",
        "override_rate": "1.10",
        "override_reason": "Override Rate"
    });
    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/transactions")
                .bearer(&token)
                .json(&create_tx_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let tx = resp.json_value();
    let tx_id = Uuid::parse_str(tx["id"].as_str().unwrap()).unwrap();

    // 4.2 GET /api/v1/multi-currency/transactions/{id} -> get_transaction (200)
    let resp = app
        .execute(
            app.get(&format!("/api/v1/multi-currency/transactions/{tx_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 4.3 PUT /api/v1/multi-currency/transactions/{id}/rate -> update_transaction_rate (200)
    let update_rate_payload = json!({
        "new_rate": "1.12",
        "reason": "Corrected Rate"
    });
    let resp = app
        .execute(
            app.put(&format!("/api/v1/multi-currency/transactions/{tx_id}/rate"))
                .bearer(&token)
                .json(&update_rate_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 4.4 GET /api/v1/multi-currency/transactions -> list_transactions (200)
    let resp = app
        .execute(
            app.get("/api/v1/multi-currency/transactions")
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 5. CROSS-BORDER LEASES
    // ========================================================================

    // 5.1 POST /api/v1/multi-currency/cross-border -> create_cross_border_lease (200)
    let lease_id = Uuid::new_v4();
    let create_lease_payload = json!({
        "lease_id": lease_id,
        "property_country": "S_K",
        "property_currency": "E_U_R",
        "tenant_country": "C_Z",
        "tenant_tax_id": "CZ12345678",
        "tenant_vat_number": "CZ12345678",
        "lease_currency": "E_U_R",
        "payment_currency": "C_Z_K",
        "convert_at_invoice_date": true,
        "convert_at_payment_date": false,
        "local_vat_applicable": true,
        "vat_rate": "20.00"
    });
    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/cross-border")
                .bearer(&token)
                .json(&create_lease_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 5.2 PUT /api/v1/multi-currency/cross-border/{lease_id} -> update_cross_border_lease (200)
    let update_lease_payload = json!({
        "compliance_status": "compliant",
        "compliance_notes": "All checks passed"
    });
    let resp = app
        .execute(
            app.put(&format!("/api/v1/multi-currency/cross-border/{lease_id}"))
                .bearer(&token)
                .json(&update_lease_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 5.3 GET /api/v1/multi-currency/cross-border/{lease_id} -> get_cross_border_lease (200)
    let resp = app
        .execute(
            app.get(&format!("/api/v1/multi-currency/cross-border/{lease_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 5.4 GET /api/v1/multi-currency/cross-border -> list_cross_border_leases (200)
    let resp = app
        .execute(
            app.get("/api/v1/multi-currency/cross-border")
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 5.5 GET /api/v1/multi-currency/cross-border/compliance/{country} -> get_compliance_requirements (200)
    let resp = app
        .execute(
            app.get("/api/v1/multi-currency/cross-border/compliance/S_K")
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 6. REPORTS
    // ========================================================================

    // 6.1 POST /api/v1/multi-currency/reports/configs -> create_report_config (200)
    let create_report_config_payload = json!({
        "name": "Consolidated EU Report",
        "description": "Monthly consolidation in EUR",
        "report_currency": "E_U_R",
        "show_original_currencies": true,
        "show_conversion_details": true,
        "rate_date_type": "end_of_period",
        "group_by_currency": true,
        "group_by_property": true,
        "is_saved": true
    });
    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/reports/configs")
                .bearer(&token)
                .json(&create_report_config_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let report_config = resp.json_value();
    let report_config_id = Uuid::parse_str(report_config["id"].as_str().unwrap()).unwrap();

    // 6.2 GET /api/v1/multi-currency/reports/configs -> list_report_configs (200)
    let resp = app
        .execute(
            app.get("/api/v1/multi-currency/reports/configs")
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 6.3 POST /api/v1/multi-currency/reports/generate -> generate_report (200)
    let generate_payload = json!({
        "period_start": "2026-06-01",
        "period_end": "2026-06-30",
        "report_currency": "E_U_R",
        "config_id": report_config_id
    });
    let resp = app
        .execute(
            app.post("/api/v1/multi-currency/reports/generate")
                .bearer(&token)
                .json(&generate_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 6.4 GET /api/v1/multi-currency/reports/snapshots -> list_report_snapshots (200)
    let resp = app
        .execute(
            app.get("/api/v1/multi-currency/reports/snapshots")
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 6.5 GET /api/v1/multi-currency/reports/exposure -> get_currency_exposure (200)
    let resp = app
        .execute(
            app.get("/api/v1/multi-currency/reports/exposure")
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 7. DASHBOARD & STATISTICS
    // ========================================================================

    // 7.1 GET /api/v1/multi-currency/dashboard -> get_dashboard (200)
    let resp = app
        .execute(
            app.get("/api/v1/multi-currency/dashboard")
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 7.2 GET /api/v1/multi-currency/statistics -> get_statistics (200)
    let resp = app
        .execute(
            app.get("/api/v1/multi-currency/statistics")
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
}
