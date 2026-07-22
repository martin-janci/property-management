//! Happy-path integration tests for the property-valuation endpoints
//! (`/api/v1/property-valuations/*`, Epic 138).
//!
//! Exercises dashboard, expiring-valuations, valuation models, valuations,
//! market-data, value-history and valuation-request routes.
//!
//! Auth model: the handlers read the organization from `AuthUser.tenant_id`,
//! which is a JWT claim — NOT the `X-Tenant-ID` header. A login token issued by
//! the auth flow carries no `tenant_id`, so we mint a tenant-scoped access token
//! directly (mirroring `analytics_owner_success_tests.rs`) and seed the
//! org + user + membership the principal needs.

use axum::http::StatusCode;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// JWT mint — tenant-scoped access token matching api-core `Claims`
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
        name: "Property Valuation User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'hash', 'PV User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn property_valuation_endpoints_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // --- Seed org / user / membership for a tenant-scoped principal ---------
    let org_id = seed_org(&pool, "pvhappy").await;
    let email = format!("pv-{}@pv.test", Uuid::new_v4());
    let user_id = seed_user(&pool, &email).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let token = mint(user_id, &email, org_id);

    // --- Seed building + unit. `property_id` on avm_* tables FKs units(id) ---
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Valuation Rd 1', 'Bratislava', '81101', 'SK') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation, floor, unit_type)
           VALUES ($1, '3C', 3, 'apartment') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(&pool)
    .await
    .expect("seed unit");

    let base = "/api/v1/property-valuations";

    // ========================================================================
    // 1. DASHBOARD & EXPIRING
    // ========================================================================

    // 1.1 GET /dashboard -> get_dashboard
    let resp = app
        .execute(app.get(&format!("{base}/dashboard")).bearer(&token).build())
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.2 GET /expiring -> get_expiring_valuations
    let resp = app
        .execute(app.get(&format!("{base}/expiring")).bearer(&token).build())
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 2. VALUATION MODELS
    // ========================================================================

    // 2.1 POST /models -> create_model (returns 201 CREATED)
    let create_model_payload = json!({
        "name": "Income Approach 2026",
        "description": "Standard income capitalisation model",
        "model_type": "income_approach"
    });
    let resp = app
        .execute(
            app.post(&format!("{base}/models"))
                .bearer(&token)
                .json(&create_model_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let model_id = Uuid::parse_str(resp.json_value()["id"].as_str().expect("model id"))
        .expect("parse model id");

    // 2.2 GET /models -> list_models
    let resp = app
        .execute(app.get(&format!("{base}/models")).bearer(&token).build())
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.3 GET /models/{id} -> get_model
    let resp = app
        .execute(
            app.get(&format!("{base}/models/{model_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.4 PUT /models/{id} -> update_model (returns 200 Json)
    let update_model_payload = json!({ "name": "Income Approach 2026 Rev2" });
    let resp = app
        .execute(
            app.put(&format!("{base}/models/{model_id}"))
                .bearer(&token)
                .json(&update_model_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.5 DELETE /models/{id} -> delete_model (returns 204 NO_CONTENT)
    let resp = app
        .execute(
            app.delete(&format!("{base}/models/{model_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::NO_CONTENT);

    // ========================================================================
    // 3. VALUATIONS
    // ========================================================================

    // 3.1 POST / -> create_valuation (returns 201 CREATED).
    // `property_id` FKs units(id); estimated_value is the required value field.
    let create_valuation_payload = json!({
        "property_id": unit_id,
        "building_id": building_id,
        "valuation_date": "2026-06-01",
        "effective_date": "2026-06-01",
        "expiration_date": "2027-06-01",
        "estimated_value": 250000.0,
        "confidence_level": "high"
    });
    let resp = app
        .execute(
            app.post(base)
                .bearer(&token)
                .json(&create_valuation_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let valuation_id = Uuid::parse_str(resp.json_value()["id"].as_str().expect("valuation id"))
        .expect("parse valuation id");

    // 3.2 GET / -> list_valuations
    let resp = app.execute(app.get(base).bearer(&token).build()).await;
    resp.assert_status(StatusCode::OK);

    // 3.3 GET /{valuation_id} -> get_valuation
    let resp = app
        .execute(
            app.get(&format!("{base}/{valuation_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.4 PUT /{valuation_id} -> update_valuation (returns 200 Json)
    let update_valuation_payload = json!({ "estimated_value": 255000.0 });
    let resp = app
        .execute(
            app.put(&format!("{base}/{valuation_id}"))
                .bearer(&token)
                .json(&update_valuation_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.5 PUT /{valuation_id}/approve -> approve_valuation (returns 200 Json)
    let resp = app
        .execute(
            app.put(&format!("{base}/{valuation_id}/approve"))
                .bearer(&token)
                .json(json!({}))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.6 GET /{valuation_id}/audit-logs -> get_audit_logs
    let resp = app
        .execute(
            app.get(&format!("{base}/{valuation_id}/audit-logs"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.7 DELETE /{valuation_id} -> delete_valuation (returns 204 NO_CONTENT)
    let resp = app
        .execute(
            app.delete(&format!("{base}/{valuation_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::NO_CONTENT);

    // ========================================================================
    // 4. MARKET DATA
    // ========================================================================

    // 4.1 POST /market-data -> create_market_data (returns 201 CREATED).
    // Required fields are period_start / period_end (NaiveDate).
    let create_md_payload = json!({
        "city": "Bratislava",
        "property_type": "apartment",
        "period_start": "2026-05-01",
        "period_end": "2026-05-31",
        "price_per_sqm_average": 3500.0,
        "data_source": "manual"
    });
    let resp = app
        .execute(
            app.post(&format!("{base}/market-data"))
                .bearer(&token)
                .json(&create_md_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let md_id = Uuid::parse_str(resp.json_value()["id"].as_str().expect("market data id"))
        .expect("parse market data id");

    // 4.2 GET /market-data -> get_market_data
    let resp = app
        .execute(
            app.get(&format!("{base}/market-data"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 4.3 PUT /market-data/{id} -> update_market_data (returns 200 Json)
    let update_md_payload = json!({ "price_per_sqm_average": 3600.0 });
    let resp = app
        .execute(
            app.put(&format!("{base}/market-data/{md_id}"))
                .bearer(&token)
                .json(&update_md_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 5. VALUE HISTORY  (route is /properties/{property_id}/history)
    // ========================================================================

    // 5.1 POST /properties/{property_id}/history -> create_value_history (201).
    // property_id is overridden by the path but is still required to deserialize.
    let vh_payload = json!({
        "property_id": unit_id,
        "record_date": "2026-01-01",
        "estimated_value": 240000.0,
        "value_source": "manual"
    });
    let resp = app
        .execute(
            app.post(&format!("{base}/properties/{unit_id}/history"))
                .bearer(&token)
                .json(&vh_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);

    // 5.2 GET /properties/{property_id}/history -> get_value_history
    let resp = app
        .execute(
            app.get(&format!("{base}/properties/{unit_id}/history"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 6. VALUATION REQUESTS
    // ========================================================================

    // 6.1 POST /requests -> create_request (returns 201 CREATED).
    // property_id FKs units(id); priority is an i32; request_type/purpose optional.
    let create_req_payload = json!({
        "property_id": unit_id,
        "request_type": "standard",
        "purpose": "sale",
        "priority": 3,
        "requester_notes": "Annual review"
    });
    let resp = app
        .execute(
            app.post(&format!("{base}/requests"))
                .bearer(&token)
                .json(&create_req_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let req_id = Uuid::parse_str(resp.json_value()["id"].as_str().expect("request id"))
        .expect("parse request id");

    // 6.2 GET /requests -> list_requests
    let resp = app
        .execute(app.get(&format!("{base}/requests")).bearer(&token).build())
        .await;
    resp.assert_status(StatusCode::OK);

    // 6.3 GET /requests/{id} -> get_request
    let resp = app
        .execute(
            app.get(&format!("{base}/requests/{req_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
}

/// Happy-path coverage for the comparables / adjustments / features / reports /
/// update-request endpoints (BIT-420, bucket B). These are the 15
/// property-valuation endpoints that had no prior happy-path test. Auth is the
/// same tenant-scoped `AuthUser` model as `property_valuation_endpoints_happy_path`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn property_valuation_comparables_features_reports_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_id = seed_org(&pool, "pvext").await;
    let email = format!("pvx-{}@pv.test", Uuid::new_v4());
    let user_id = seed_user(&pool, &email).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let token = mint(user_id, &email, org_id);

    // Seed building + unit. `property_id` on avm_* tables FKs units(id).
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Comparable Rd 2', 'Bratislava', '81102', 'SK') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation, floor, unit_type)
           VALUES ($1, '5A', 5, 'apartment') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(&pool)
    .await
    .expect("seed unit");

    let base = "/api/v1/property-valuations";

    // Parent valuation for comparables + reports.
    let create_valuation_payload = json!({
        "property_id": unit_id,
        "building_id": building_id,
        "valuation_date": "2026-06-01",
        "effective_date": "2026-06-01",
        "expiration_date": "2027-06-01",
        "estimated_value": 300000.0,
        "confidence_level": "high"
    });
    let resp = app
        .execute(
            app.post(base)
                .bearer(&token)
                .json(&create_valuation_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let valuation_id = Uuid::parse_str(resp.json_value()["id"].as_str().expect("valuation id"))
        .expect("parse valuation id");

    // ========================================================================
    // 1. COMPARABLES
    // ========================================================================

    // 1.1 POST /{valuation_id}/comparables -> create_comparable (201).
    // sale_date + sale_price are the only required fields; valuation_id is
    // overridden from the path.
    let create_comp_payload = json!({
        "sale_date": "2026-03-01",
        "sale_price": 240000.0,
        "property_type": "apartment",
        "total_area_sqm": 75.0,
        "year_built": 2010
    });
    let resp = app
        .execute(
            app.post(&format!("{base}/{valuation_id}/comparables"))
                .bearer(&token)
                .json(&create_comp_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let comparable_id = Uuid::parse_str(resp.json_value()["id"].as_str().expect("comparable id"))
        .expect("parse comparable id");

    // 1.2 GET /{valuation_id}/comparables -> list_comparables
    let resp = app
        .execute(
            app.get(&format!("{base}/{valuation_id}/comparables"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.3 PUT /comparables/{comparable_id} -> update_comparable (200 Json)
    let update_comp_payload = json!({ "weight": 0.5, "is_verified": true });
    let resp = app
        .execute(
            app.put(&format!("{base}/comparables/{comparable_id}"))
                .bearer(&token)
                .json(&update_comp_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 2. ADJUSTMENTS (children of a comparable)
    // ========================================================================

    // 2.1 POST /comparables/{comparable_id}/adjustments -> create_adjustment (201)
    let create_adj_payload = json!({
        "comparable_id": comparable_id,
        "adjustment_type": "location",
        "adjustment_name": "Location premium",
        "adjustment_amount": 5000.0,
        "justification": "Closer to city centre"
    });
    let resp = app
        .execute(
            app.post(&format!("{base}/comparables/{comparable_id}/adjustments"))
                .bearer(&token)
                .json(&create_adj_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let adjustment_id = Uuid::parse_str(resp.json_value()["id"].as_str().expect("adjustment id"))
        .expect("parse adjustment id");

    // 2.2 GET /comparables/{comparable_id}/adjustments -> list_adjustments
    let resp = app
        .execute(
            app.get(&format!("{base}/comparables/{comparable_id}/adjustments"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.3 DELETE /adjustments/{adjustment_id} -> delete_adjustment (204)
    let resp = app
        .execute(
            app.delete(&format!("{base}/adjustments/{adjustment_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::NO_CONTENT);

    // 2.4 DELETE /comparables/{comparable_id} -> delete_comparable (204)
    let resp = app
        .execute(
            app.delete(&format!("{base}/comparables/{comparable_id}"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::NO_CONTENT);

    // ========================================================================
    // 3. PROPERTY FEATURES (route is /properties/{property_id}/features)
    // ========================================================================

    // 3.1 POST /properties/{property_id}/features -> create_features (201).
    // property_id is overridden by the path but still required to deserialize.
    let create_feat_payload = json!({
        "property_id": unit_id,
        "total_area_sqm": 80.0,
        "living_area_sqm": 70.0,
        "bedrooms": 3,
        "has_garage": true,
        "condition": "good"
    });
    let resp = app
        .execute(
            app.post(&format!("{base}/properties/{unit_id}/features"))
                .bearer(&token)
                .json(&create_feat_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let feature_id = Uuid::parse_str(resp.json_value()["id"].as_str().expect("feature id"))
        .expect("parse feature id");

    // 3.2 GET /properties/{property_id}/features -> get_features
    let resp = app
        .execute(
            app.get(&format!("{base}/properties/{unit_id}/features"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 3.3 PUT /features/{feature_id} -> update_features (200 Json)
    let update_feat_payload = json!({ "condition_score": 8, "has_pool": true });
    let resp = app
        .execute(
            app.put(&format!("{base}/features/{feature_id}"))
                .bearer(&token)
                .json(&update_feat_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 4. REPORTS
    // ========================================================================

    // 4.1 POST /{valuation_id}/reports -> create_report (201).
    // valuation_id is overridden by the path but required to deserialize.
    let create_report_payload = json!({
        "valuation_id": valuation_id,
        "report_type": "summary",
        "title": "Valuation Report",
        "executive_summary": "Estimated value 300k"
    });
    let resp = app
        .execute(
            app.post(&format!("{base}/{valuation_id}/reports"))
                .bearer(&token)
                .json(&create_report_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let report_id = Uuid::parse_str(resp.json_value()["id"].as_str().expect("report id"))
        .expect("parse report id");

    // 4.2 GET /{valuation_id}/reports -> list_reports
    let resp = app
        .execute(
            app.get(&format!("{base}/{valuation_id}/reports"))
                .bearer(&token)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 4.3 PUT /reports/{report_id} -> update_report (200 Json)
    let update_report_payload = json!({ "title": "Valuation Report Rev2" });
    let resp = app
        .execute(
            app.put(&format!("{base}/reports/{report_id}"))
                .bearer(&token)
                .json(&update_report_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 4.4 PUT /reports/{report_id}/sign -> sign_report (200 Json)
    let resp = app
        .execute(
            app.put(&format!("{base}/reports/{report_id}/sign"))
                .bearer(&token)
                .json(json!({}))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 5. UPDATE VALUATION REQUEST
    // ========================================================================

    // 5.1 POST /requests -> create_request (201) — parent for the update.
    let create_req_payload = json!({
        "property_id": unit_id,
        "request_type": "standard",
        "purpose": "sale",
        "priority": 3
    });
    let resp = app
        .execute(
            app.post(&format!("{base}/requests"))
                .bearer(&token)
                .json(&create_req_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let req_id = Uuid::parse_str(resp.json_value()["id"].as_str().expect("request id"))
        .expect("parse request id");

    // 5.2 PUT /requests/{request_id} -> update_request (200 Json)
    let update_req_payload = json!({ "priority": 2, "internal_notes": "Reviewed" });
    let resp = app
        .execute(
            app.put(&format!("{base}/requests/{req_id}"))
                .bearer(&token)
                .json(&update_req_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
}
