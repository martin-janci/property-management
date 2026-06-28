//! Happy-path integration tests for the market-pricing endpoints (`/api/v1/pricing/*`).
//!
//! Exercises regions, data points, recommendations, CMA (Comparative Market Analysis),
//! and pricing history routes.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn market_pricing_endpoints_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "mphappy").await;
    let session = app.session(token, org_id);

    // Seed building and unit for price history / current-rent routes
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Pricing Ave 1', 'Bratislava', '81101', 'SK') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation, floor, size_sqm, unit_type)
           VALUES ($1, '2B', 2, 60.0, 'apartment') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed unit");

    let base = "/api/v1/pricing";

    // ========================================================================
    // 1. REGIONS
    // ========================================================================

    // 1.1 POST /regions -> create_region
    let create_region_payload = json!({
        "name": "Bratislava I",
        "country_code": "SK",
        "city": "Bratislava",
        "postal_codes": ["81101", "81102"]
    });
    let resp = app
        .execute(
            session
                .post(&format!("{base}/regions"))
                .json(&create_region_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::CREATED);
    let region = resp.json_value();
    let region_id_str = region["id"].as_str().expect("region id").to_string();
    let region_id = Uuid::parse_str(&region_id_str).expect("uuid");

    // 1.2 GET /regions -> list_regions
    let resp = app
        .execute(session.get(&format!("{base}/regions")).build())
        .await;
    resp.assert_status(StatusCode::OK);
    let regions = resp.json_value();
    assert!(
        regions["regions"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "list_regions: expected non-empty regions array, got {regions}"
    );

    // 1.3 GET /regions/{id} -> get_region
    let resp = app
        .execute(session.get(&format!("{base}/regions/{region_id}")).build())
        .await;
    resp.assert_status(StatusCode::OK);
    assert_eq!(resp.json_value()["id"], region_id_str);

    // 1.4 PUT /regions/{id} -> update_region
    let update_region_payload = json!({
        "name": "Bratislava I Updated"
    });
    let resp = app
        .execute(
            session
                .put(&format!("{base}/regions/{region_id}"))
                .json(&update_region_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 2. DATA POINTS
    // ========================================================================

    // 2.1 POST /data -> add_data_point
    // CreateMarketDataPoint requires region_id, property_type, size_sqm, monthly_rent;
    // source/currency default. Handler returns the created data point directly (200 OK).
    let add_dp_payload = json!({
        "region_id": region_id,
        "property_type": "apartment",
        "size_sqm": 60.0,
        "monthly_rent": 750.0,
        "price_per_sqm": 12.5,
        "currency": "EUR",
        "source": "manual"
    });
    let resp = app
        .execute(
            session
                .post(&format!("{base}/data"))
                .json(&add_dp_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.2 GET /data -> list_data_points (returns { "data_points": [...] })
    let resp = app
        .execute(session.get(&format!("{base}/data")).build())
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 3. STATISTICS
    // ========================================================================

    // 3.1 POST /statistics/generate -> generate_statistics
    // GenerateStatisticsRequest requires region_id, period_start, period_end (NaiveDate).
    let gen_payload = json!({
        "region_id": region_id,
        "property_type": "apartment",
        "period_start": "2026-01-01",
        "period_end": "2026-06-30"
    });
    let resp = app
        .execute(
            session
                .post(&format!("{base}/statistics/generate"))
                .json(&gen_payload)
                .build(),
        )
        .await;
    // OK or 422 if not enough data points
    assert!(
        resp.status == StatusCode::OK
            || resp.status == StatusCode::UNPROCESSABLE_ENTITY
            || resp.status == StatusCode::BAD_REQUEST,
        "generate_statistics: unexpected {}",
        resp.status
    );

    // 3.2 GET /statistics/{region_id} -> get_statistics
    let resp = app
        .execute(
            session
                .get(&format!("{base}/statistics/{region_id}"))
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::NOT_FOUND,
        "get_statistics: unexpected {}",
        resp.status
    );

    // ========================================================================
    // 4. RECOMMENDATIONS
    // ========================================================================

    // 4.1 POST /recommendations/request -> request_recommendation
    // RequestPricingRecommendation only needs unit_id (+ optional currency). The
    // handler is RLS-scoped to the tenant and falls back to statistical pricing
    // when the LLM is unavailable, so it returns 200 with the recommendation.
    let rec_req_payload = json!({
        "unit_id": unit_id,
        "currency": "EUR"
    });
    let resp = app
        .execute(
            session
                .post(&format!("{base}/recommendations/request"))
                .json(&rec_req_payload)
                .build(),
        )
        .await;
    let rec_created = resp.status == StatusCode::OK || resp.status == StatusCode::CREATED;
    let rec_id: Option<Uuid> = if rec_created {
        resp.json_value()["id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok())
    } else {
        None
    };

    // 4.2 GET /recommendations -> list_recommendations
    let resp = app
        .execute(session.get(&format!("{base}/recommendations")).build())
        .await;
    resp.assert_status(StatusCode::OK);

    if let Some(rid) = rec_id {
        // 4.3 GET /recommendations/{id} -> get_recommendation
        let resp = app
            .execute(
                session
                    .get(&format!("{base}/recommendations/{rid}"))
                    .build(),
            )
            .await;
        resp.assert_status(StatusCode::OK);

        // 4.4 POST /recommendations/{id}/reject -> reject_recommendation
        let reject_payload = json!({ "rejection_reason": "too high" });
        let resp = app
            .execute(
                session
                    .post(&format!("{base}/recommendations/{rid}/reject"))
                    .json(&reject_payload)
                    .build(),
            )
            .await;
        assert!(
            resp.status == StatusCode::OK || resp.status == StatusCode::NO_CONTENT,
            "reject_recommendation: unexpected {}",
            resp.status
        );
    }

    // ========================================================================
    // 5. UNIT PRICING HISTORY & CURRENT RENT
    // ========================================================================

    // 5.1 GET /units/{unit_id}/current-rent -> get_current_rent
    let resp = app
        .execute(
            session
                .get(&format!("{base}/units/{unit_id}/current-rent"))
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::NOT_FOUND,
        "get_current_rent: unexpected {}",
        resp.status
    );

    // 5.2 POST /units/{unit_id}/price -> record_price_change
    // RecordPriceChange requires unit_id (overwritten from path by the handler,
    // but still required for deserialization), effective_date and monthly_rent.
    let price_payload = json!({
        "unit_id": unit_id,
        "monthly_rent": 650.0,
        "currency": "EUR",
        "effective_date": "2026-06-01",
        "change_reason": "market adjustment"
    });
    let resp = app
        .execute(
            session
                .post(&format!("{base}/units/{unit_id}/price"))
                .json(&price_payload)
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::CREATED,
        "record_price_change: unexpected {}",
        resp.status
    );

    // 5.3 GET /units/{unit_id}/history -> get_pricing_history
    let resp = app
        .execute(
            session
                .get(&format!("{base}/units/{unit_id}/history"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 6. CMA (Comparative Market Analysis)
    // ========================================================================

    // 6.1 POST /cma -> create_cma
    // CreateComparativeMarketAnalysis requires name + properties_to_compare (Vec<Uuid>);
    // region_id / property_type are optional. Handler returns the CMA directly (200 OK).
    let create_cma_payload = json!({
        "name": "Q1 2026 CMA",
        "region_id": region_id,
        "property_type": "apartment",
        "properties_to_compare": [unit_id]
    });
    let resp = app
        .execute(
            session
                .post(&format!("{base}/cma"))
                .json(&create_cma_payload)
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::CREATED,
        "create_cma: unexpected {}",
        resp.status
    );
    let cma_id: Option<Uuid> =
        if resp.status == StatusCode::OK || resp.status == StatusCode::CREATED {
            resp.json_value()["id"]
                .as_str()
                .and_then(|s| Uuid::parse_str(s).ok())
        } else {
            None
        };

    // 6.2 GET /cma -> list_cmas
    let resp = app
        .execute(session.get(&format!("{base}/cma")).build())
        .await;
    resp.assert_status(StatusCode::OK);

    if let Some(cid) = cma_id {
        // 6.3 GET /cma/{id} -> get_cma
        let resp = app
            .execute(session.get(&format!("{base}/cma/{cid}")).build())
            .await;
        resp.assert_status(StatusCode::OK);

        // 6.4 PUT /cma/{id} -> update_cma
        let update_cma_payload = json!({ "name": "Q1 2026 CMA Updated" });
        let resp = app
            .execute(
                session
                    .put(&format!("{base}/cma/{cid}"))
                    .json(&update_cma_payload)
                    .build(),
            )
            .await;
        resp.assert_status(StatusCode::OK);

        // 6.5 DELETE /cma/{id} -> delete_cma
        let resp = app
            .execute(session.delete(&format!("{base}/cma/{cid}")).build())
            .await;
        assert!(
            resp.status == StatusCode::OK
                || resp.status == StatusCode::NO_CONTENT
                || resp.status == StatusCode::NOT_FOUND,
            "delete_cma: unexpected {}",
            resp.status
        );
    }

    // ========================================================================
    // 7. COMPARABLES (run while the region still exists)
    // ========================================================================

    // 7.1 GET /comparables -> get_comparables
    // ComparablesParams requires region_id, property_type and size_sqm as query
    // params; the handler verifies region ownership, then returns
    // { "comparables": [...] } (200).
    let resp = app
        .execute(
            session
                .get(&format!(
                    "{base}/comparables?region_id={region_id}&property_type=apartment&size_sqm=60"
                ))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 8. DASHBOARD
    // ========================================================================

    // 8.1 GET /dashboard -> get_pricing_dashboard (all query params optional)
    let resp = app
        .execute(session.get(&format!("{base}/dashboard")).build())
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.5 DELETE /regions/{id} -> delete_region (done last; comparables above
    // depend on the region existing)
    let resp = app
        .execute(
            session
                .delete(&format!("{base}/regions/{region_id}"))
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::OK
            || resp.status == StatusCode::NO_CONTENT
            || resp.status == StatusCode::CONFLICT,
        "delete_region: unexpected {}",
        resp.status
    );
}
