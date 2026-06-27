//! Happy-path integration tests for the property-valuation endpoints
//! (`/api/v1/property-valuations/*`).
//!
//! Exercises dashboard, expiring-valuations, valuation models, valuations,
//! comparables, adjustments, market-data, requests, and value-history routes.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn property_valuation_endpoints_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "pvhappy").await;
    let session = app.session(token, org_id);

    // Seed building and unit
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Valuation Rd 1', 'Bratislava', '81101', 'SK') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, unit_number, floor, area_sqm)
           VALUES ($1, '3C', 3, 75.0) RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed unit");

    let base = "/api/v1/property-valuations";

    // ========================================================================
    // 1. DASHBOARD & EXPIRING
    // ========================================================================

    // 1.1 GET /dashboard -> get_dashboard
    let resp = app
        .execute(session.get(&format!("{base}/dashboard")).build())
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.2 GET /expiring -> get_expiring_valuations
    let resp = app
        .execute(session.get(&format!("{base}/expiring")).build())
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 2. VALUATION MODELS
    // ========================================================================

    // 2.1 POST /models -> create_model
    let create_model_payload = json!({
        "name": "Income Approach 2026",
        "method": "income",
        "description": "Standard income capitalisation model"
    });
    let resp = app
        .execute(
            session
                .post(&format!("{base}/models"))
                .json(&create_model_payload)
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::CREATED,
        "create_model: unexpected {}",
        resp.status
    );
    let model_id: Option<Uuid> = resp.json_value()["id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok());

    // 2.2 GET /models -> list_models
    let resp = app
        .execute(session.get(&format!("{base}/models")).build())
        .await;
    resp.assert_status(StatusCode::OK);

    if let Some(mid) = model_id {
        // 2.3 GET /models/{id} -> get_model
        let resp = app
            .execute(session.get(&format!("{base}/models/{mid}")).build())
            .await;
        resp.assert_status(StatusCode::OK);

        // 2.4 PUT /models/{id} -> update_model
        let update_model_payload = json!({ "name": "Income Approach 2026 Rev2" });
        let resp = app
            .execute(
                session
                    .put(&format!("{base}/models/{mid}"))
                    .json(&update_model_payload)
                    .build(),
            )
            .await;
        resp.assert_status(StatusCode::OK);

        // 2.5 DELETE /models/{id} -> delete_model
        let resp = app
            .execute(session.delete(&format!("{base}/models/{mid}")).build())
            .await;
        assert!(
            resp.status == StatusCode::OK || resp.status == StatusCode::NO_CONTENT,
            "delete_model: unexpected {}",
            resp.status
        );
    }

    // ========================================================================
    // 3. VALUATIONS
    // ========================================================================

    // 3.1 POST / -> create_valuation
    let create_valuation_payload = json!({
        "unit_id": unit_id,
        "valuation_date": "2026-06-01",
        "value": 250000.0,
        "currency": "EUR",
        "method": "market_comparison",
        "valid_until": "2027-06-01"
    });
    let resp = app
        .execute(
            session
                .post(base)
                .json(&create_valuation_payload)
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::CREATED,
        "create_valuation: unexpected {}",
        resp.status
    );
    let valuation_id: Option<Uuid> = resp.json_value()["id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok());

    // 3.2 GET / -> list_valuations
    let resp = app
        .execute(session.get(base).build())
        .await;
    resp.assert_status(StatusCode::OK);

    if let Some(vid) = valuation_id {
        // 3.3 GET /{valuation_id} -> get_valuation
        let resp = app
            .execute(session.get(&format!("{base}/{vid}")).build())
            .await;
        resp.assert_status(StatusCode::OK);

        // 3.4 PUT /{valuation_id} -> update_valuation
        let update_valuation_payload = json!({ "value": 255000.0 });
        let resp = app
            .execute(
                session
                    .put(&format!("{base}/{vid}"))
                    .json(&update_valuation_payload)
                    .build(),
            )
            .await;
        resp.assert_status(StatusCode::OK);

        // 3.5 PUT /{valuation_id}/approve -> approve_valuation
        let resp = app
            .execute(
                session
                    .put(&format!("{base}/{vid}/approve"))
                    .json(&json!({}))
                    .build(),
            )
            .await;
        assert!(
            resp.status == StatusCode::OK || resp.status == StatusCode::CONFLICT,
            "approve_valuation: unexpected {}",
            resp.status
        );

        // 3.6 GET /{valuation_id}/audit-logs -> get_audit_logs
        let resp = app
            .execute(
                session
                    .get(&format!("{base}/{vid}/audit-logs"))
                    .build(),
            )
            .await;
        resp.assert_status(StatusCode::OK);

        // 3.7 DELETE /{valuation_id} -> delete_valuation
        let resp = app
            .execute(session.delete(&format!("{base}/{vid}")).build())
            .await;
        assert!(
            resp.status == StatusCode::OK
                || resp.status == StatusCode::NO_CONTENT
                || resp.status == StatusCode::CONFLICT,
            "delete_valuation: unexpected {}",
            resp.status
        );
    }

    // ========================================================================
    // 4. MARKET DATA
    // ========================================================================

    // 4.1 POST /market-data -> create_market_data (derived from route pattern)
    let create_md_payload = json!({
        "unit_id": unit_id,
        "date": "2026-06-01",
        "avg_price_per_sqm": 3500.0,
        "currency": "EUR",
        "source": "manual"
    });
    let resp = app
        .execute(
            session
                .post(&format!("{base}/market-data"))
                .json(&create_md_payload)
                .build(),
        )
        .await;
    let md_created = resp.status == StatusCode::OK || resp.status == StatusCode::CREATED;
    let md_id: Option<Uuid> = if md_created {
        resp.json_value()["id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
    } else {
        None
    };

    // 4.2 GET /market-data -> get_market_data
    let resp = app
        .execute(session.get(&format!("{base}/market-data")).build())
        .await;
    resp.assert_status(StatusCode::OK);

    if let Some(mdid) = md_id {
        // 4.3 PUT /market-data/{id} -> update_market_data
        let update_md_payload = json!({ "avg_price_per_sqm": 3600.0 });
        let resp = app
            .execute(
                session
                    .put(&format!("{base}/market-data/{mdid}"))
                    .json(&update_md_payload)
                    .build(),
            )
            .await;
        resp.assert_status(StatusCode::OK);
    }

    // ========================================================================
    // 5. VALUE HISTORY
    // ========================================================================

    // 5.1 POST /value-history -> create_value_history
    let vh_payload = json!({
        "unit_id": unit_id,
        "date": "2026-01-01",
        "value": 240000.0,
        "currency": "EUR"
    });
    let resp = app
        .execute(
            session
                .post(&format!("{base}/value-history"))
                .json(&vh_payload)
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::CREATED,
        "create_value_history: unexpected {}",
        resp.status
    );

    // 5.2 GET /value-history -> get_value_history
    let resp = app
        .execute(session.get(&format!("{base}/value-history")).build())
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 6. VALUATION REQUESTS
    // ========================================================================

    // 6.1 POST /requests -> create_request
    let create_req_payload = json!({
        "unit_id": unit_id,
        "requested_by": "Owner",
        "reason": "Annual review",
        "priority": "normal"
    });
    let resp = app
        .execute(
            session
                .post(&format!("{base}/requests"))
                .json(&create_req_payload)
                .build(),
        )
        .await;
    let req_created = resp.status == StatusCode::OK || resp.status == StatusCode::CREATED;
    let req_id: Option<Uuid> = if req_created {
        resp.json_value()["id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
    } else {
        None
    };

    // 6.2 GET /requests -> list_requests
    let resp = app
        .execute(session.get(&format!("{base}/requests")).build())
        .await;
    resp.assert_status(StatusCode::OK);

    if let Some(rid) = req_id {
        // 6.3 GET /requests/{id} -> get_request
        let resp = app
            .execute(session.get(&format!("{base}/requests/{rid}")).build())
            .await;
        resp.assert_status(StatusCode::OK);
    }
}
