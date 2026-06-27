//! Happy path integration tests for the multi-currency endpoints (`/api/v1/multi-currency/*`).
//!
//! Exercises 20+ different endpoints covering currency configuration, exchange rate management,
//! cross-currency transactions, cross-border lease management, reporting, and dashboard.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use chrono::NaiveDate;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn multi_currency_endpoints_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "mchappy").await;
    let session = app.session(token, org_id);

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

    // 1.1 POST /api/v1/multi-currency/config -> create_or_update_currency_config
    let create_config_payload = json!({
        "base_currency": "EUR",
        "enabled_currencies": ["EUR", "USD", "CZK"],
        "display_currency": "USD",
        "show_original_amount": true,
        "decimal_places": 2,
        "exchange_rate_source": "ecb",
        "auto_update_rates": true,
        "update_frequency_hours": 24,
        "rounding_mode": "half_up"
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/multi-currency/config")
                .json(&create_config_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.2 PUT /api/v1/multi-currency/config -> update_currency_config
    let update_config_payload = json!({
        "rounding_mode": "half_down"
    });
    let resp = app
        .execute(
            session
                .put("/api/v1/multi-currency/config")
                .json(&update_config_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.3 GET /api/v1/multi-currency/config -> get_currency_config
    let resp = app
        .execute(session.get("/api/v1/multi-currency/config").build())
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 2. PROPERTY CURRENCY CONFIGURATION
    // ========================================================================

    // 2.1 POST /api/v1/multi-currency/properties -> create_property_currency_config
    let create_prop_payload = json!({
        "building_id": building_id,
        "default_currency": "USD",
        "country": "SK",
        "vat_rate": "20.00",
        "requires_local_reporting": true,
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/multi-currency/properties")
                .json(&create_prop_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);

    // 2.2 PUT /api/v1/multi-currency/properties/{building_id} -> update_property_currency_config
    let update_prop_payload = json!({
        "default_currency": "EUR"
    });
    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/multi-currency/properties/{building_id}"))
                .json(&update_prop_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.3 GET /api/v1/multi-currency/properties/{building_id} -> get_property_currency_config
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/multi-currency/properties/{building_id}"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.4 GET /api/v1/multi-currency/properties -> list_property_currency_configs
    let resp = app
        .execute(session.get("/api/v1/multi-currency/properties").build())
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 3. EXCHANGE RATES
    // ========================================================================

    // 3.1 POST /api/v1/multi-currency/rates -> create_exchange_rate
    let create_rate_payload = json!({
        "from_currency": "EUR",
        "to_currency": "USD",
        "rate": "1.10",
        "rate_date": "2026-06-27",
        "source": "ecb"
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/multi-currency/rates")
                .json(&create_rate_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);

    // 3.2 GET /api/v1/multi-currency/rates -> list_exchange_rates
    let resp = app
        .execute(
            session
                .get("/api/v1/multi-currency/rates?from_currency=EUR&to_currency=USD")
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.3 GET /api/v1/multi-currency/rates/latest -> get_latest_exchange_rate
    let resp = app
        .execute(
            session
                .get("/api/v1/multi-currency/rates/latest?from_currency=EUR&to_currency=USD")
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.4 POST /api/v1/multi-currency/rates/override -> override_exchange_rate
    let override_payload = json!({
        "from_currency": "EUR",
        "to_currency": "USD",
        "rate": "1.15",
        "rate_date": "2026-06-27",
        "reason": "Test Override"
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/multi-currency/rates/override")
                .json(&override_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);

    // 3.5 POST /api/v1/multi-currency/rates/fetch -> fetch_exchange_rates
    let resp = app
        .execute(session.post("/api/v1/multi-currency/rates/fetch").build())
        .await;
    // Allow either success or bad gateway/internal error depending on offline status
    assert!(
        resp.status == StatusCode::OK
            || resp.status == StatusCode::BAD_GATEWAY
            || resp.status == StatusCode::INTERNAL_SERVER_ERROR
    );

    // ========================================================================
    // 4. CROSS-CURRENCY TRANSACTIONS
    // ========================================================================

    // 4.1 POST /api/v1/multi-currency/transactions -> create_transaction
    let source_id = Uuid::new_v4();
    let create_tx_payload = json!({
        "building_id": building_id,
        "source_type": "invoice",
        "source_id": source_id,
        "original_currency": "USD",
        "original_amount": "100.00",
        "override_rate": "1.10",
        "override_reason": "Override Rate"
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/multi-currency/transactions")
                .json(&create_tx_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let tx = resp.json_value();
    let tx_id = Uuid::parse_str(tx["id"].as_str().unwrap()).unwrap();

    // 4.2 GET /api/v1/multi-currency/transactions/{id} -> get_transaction
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/multi-currency/transactions/{tx_id}"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 4.3 PUT /api/v1/multi-currency/transactions/{id}/rate -> update_transaction_rate
    let update_rate_payload = json!({
        "new_rate": "1.12",
        "reason": "Corrected Rate"
    });
    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/multi-currency/transactions/{tx_id}/rate"))
                .json(&update_rate_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 4.4 GET /api/v1/multi-currency/transactions -> list_transactions
    let resp = app
        .execute(session.get("/api/v1/multi-currency/transactions").build())
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 5. CROSS-BORDER LEASES
    // ========================================================================

    // 5.1 POST /api/v1/multi-currency/cross-border -> create_cross_border_lease
    let lease_id = Uuid::new_v4();
    let create_lease_payload = json!({
        "lease_id": lease_id,
        "property_country": "SK",
        "property_currency": "EUR",
        "tenant_country": "CZ",
        "tenant_tax_id": "CZ12345678",
        "tenant_vat_number": "CZ12345678",
        "lease_currency": "EUR",
        "payment_currency": "CZK",
        "convert_at_invoice_date": true,
        "convert_at_payment_date": false,
        "local_vat_applicable": true,
        "vat_rate": "20.00"
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/multi-currency/cross-border")
                .json(&create_lease_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);

    // 5.2 PUT /api/v1/multi-currency/cross-border/{lease_id} -> update_cross_border_lease
    let update_lease_payload = json!({
        "compliance_status": "compliant",
        "compliance_notes": "All checks passed"
    });
    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/multi-currency/cross-border/{lease_id}"))
                .json(&update_lease_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 5.3 GET /api/v1/multi-currency/cross-border/{lease_id} -> get_cross_border_lease
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/multi-currency/cross-border/{lease_id}"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 5.4 GET /api/v1/multi-currency/cross-border -> list_cross_border_leases
    let resp = app
        .execute(session.get("/api/v1/multi-currency/cross-border").build())
        .await;
    resp.assert_status(StatusCode::OK);

    // 5.5 GET /api/v1/multi-currency/cross-border/compliance/{country} -> get_compliance_requirements
    let resp = app
        .execute(
            session
                .get("/api/v1/multi-currency/cross-border/compliance/SK")
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 6. REPORTS
    // ========================================================================

    // 6.1 POST /api/v1/multi-currency/reports/configs -> create_report_config
    let create_report_config_payload = json!({
        "name": "Consolidated EU Report",
        "description": "Monthly consolidation in EUR",
        "report_currency": "EUR",
        "show_original_currencies": true,
        "show_conversion_details": true,
        "rate_date_type": "end_of_period",
        "group_by_currency": true,
        "group_by_property": true,
        "is_saved": true
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/multi-currency/reports/configs")
                .json(&create_report_config_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let report_config = resp.json_value();
    let report_config_id = Uuid::parse_str(report_config["id"].as_str().unwrap()).unwrap();

    // 6.2 GET /api/v1/multi-currency/reports/configs -> list_report_configs
    let resp = app
        .execute(
            session
                .get("/api/v1/multi-currency/reports/configs")
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 6.3 POST /api/v1/multi-currency/reports/generate -> generate_report
    let generate_payload = json!({
        "period_start": "2026-06-01",
        "period_end": "2026-06-30",
        "report_currency": "EUR",
        "config_id": report_config_id
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/multi-currency/reports/generate")
                .json(&generate_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);

    // 6.4 GET /api/v1/multi-currency/reports/snapshots -> list_report_snapshots
    let resp = app
        .execute(
            session
                .get("/api/v1/multi-currency/reports/snapshots")
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 6.5 GET /api/v1/multi-currency/reports/exposure -> get_currency_exposure
    let resp = app
        .execute(
            session
                .get("/api/v1/multi-currency/reports/exposure")
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 7. DASHBOARD & STATISTICS
    // ========================================================================

    // 7.1 GET /api/v1/multi-currency/dashboard -> get_dashboard
    let resp = app
        .execute(session.get("/api/v1/multi-currency/dashboard").build())
        .await;
    resp.assert_status(StatusCode::OK);

    // 7.2 GET /api/v1/multi-currency/statistics -> get_statistics
    let resp = app
        .execute(session.get("/api/v1/multi-currency/statistics").build())
        .await;
    resp.assert_status(StatusCode::OK);
}
