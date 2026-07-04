//! Happy-path integration tests for the person-months endpoints.
//!
//! Exercises unit-level (`/api/v1/buildings/{bid}/units/{uid}/person-months`)
//! and building-level (`/api/v1/buildings/{bid}/person-months`) routes.

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn person_months_endpoints_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "pmhappy").await;
    let session = app.session(token, org_id);

    // Seed building
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Person Month St 1', 'Bratislava', '81101', 'SK') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed building");

    // Seed unit
    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation, floor, size_sqm)
           VALUES ($1, '1A', 1, 55.0) RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed unit");

    // ========================================================================
    // 1. UNIT-LEVEL PERSON MONTHS
    // ========================================================================

    let unit_pm_base = format!("/api/v1/buildings/{building_id}/units/{unit_id}/person-months");

    // 1.1 POST (upsert) -> upsert_person_month
    let create_payload = json!({
        "year": 2026,
        "month": 1,
        "count": 2,
        "source": "manual"
    });
    let resp = app
        .execute(session.post(&unit_pm_base).json(&create_payload).build())
        .await;
    resp.assert_status(StatusCode::OK);
    let pm = resp.json_value();
    let pm_id_str = pm["id"].as_str().expect("id missing").to_string();
    let pm_id = Uuid::parse_str(&pm_id_str).expect("invalid pm uuid");

    // 1.2 GET / -> get_unit_person_months (requires ?year=)
    let resp = app
        .execute(session.get(&format!("{unit_pm_base}?year=2026")).build())
        .await;
    resp.assert_status(StatusCode::OK);
    let list = resp.json_value();
    assert!(list.as_array().map(|a| !a.is_empty()).unwrap_or(false));

    // 1.3 GET /{id} -> get_person_month
    let resp = app
        .execute(session.get(&format!("{unit_pm_base}/{pm_id}")).build())
        .await;
    resp.assert_status(StatusCode::OK);
    let detail = resp.json_value();
    assert_eq!(detail["id"], pm_id_str);
    assert_eq!(detail["count"], 2);

    // 1.4 PUT /{id} -> update_person_month
    let update_payload = json!({
        "count": 3
    });
    let resp = app
        .execute(
            session
                .put(&format!("{unit_pm_base}/{pm_id}"))
                .json(&update_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
    let updated = resp.json_value();
    // PersonMonthResponse serializes the field as `count` (no serde rename).
    assert_eq!(updated["count"], 3);

    // 1.5 GET /yearly -> get_yearly_summary
    let resp = app
        .execute(
            session
                .get(&format!("{unit_pm_base}/yearly?year=2026"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 1.6 POST /calculate -> calculate_from_residents (body uses GetPersonMonthsQuery)
    let calc_payload = json!({
        "year": 2026,
        "month": 2
    }); // year required, month optional
    let resp = app
        .execute(
            session
                .post(&format!("{unit_pm_base}/calculate"))
                .json(&calc_payload)
                .build(),
        )
        .await;
    // 200 OK, or 400 if the unit has no residents to calculate from.
    // The pre-existing BIT-440 500 was fixed in PR #1991 (the
    // count_residents_for_month result is now decoded via a `::bigint` cast),
    // so a 500 here is a real regression and must fail loudly — do NOT
    // green-light INTERNAL_SERVER_ERROR.
    assert!(
        resp.status == StatusCode::OK || resp.status == StatusCode::BAD_REQUEST,
        "calculate_from_residents: unexpected {}",
        resp.status
    );

    // 1.7 DELETE /{id} -> delete_person_month
    // Handler returns `Ok(StatusCode::OK)` (200), not 204.
    let resp = app
        .execute(session.delete(&format!("{unit_pm_base}/{pm_id}")).build())
        .await;
    resp.assert_status(StatusCode::OK);

    // ========================================================================
    // 2. BUILDING-LEVEL PERSON MONTHS
    // ========================================================================

    let bldg_pm_base = format!("/api/v1/buildings/{building_id}/person-months");

    // 2.1 GET / -> list_building_person_months
    // BuildingPersonMonthsQuery requires both `year` and `month`; omitting
    // either makes the Query extractor reject the request with 400.
    let resp = app
        .execute(
            session
                .get(&format!("{bldg_pm_base}?year=2026&month=3"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.2 GET /summary -> get_building_summary (year + month both required)
    let resp = app
        .execute(
            session
                .get(&format!("{bldg_pm_base}/summary?year=2026&month=3"))
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);

    // 2.3 POST /bulk -> bulk_upsert_person_months
    let bulk_payload = json!({
        "year": 2026,
        "month": 3,
        "entries": [
            { "unit_id": unit_id, "count": 1 },
            { "unit_id": unit_id, "count": 2 }
        ]
    });
    let resp = app
        .execute(
            session
                .post(&format!("{bldg_pm_base}/bulk"))
                .json(&bulk_payload)
                .build(),
        )
        .await;
    resp.assert_status(StatusCode::OK);
}
