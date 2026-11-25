//! Happy-path integration tests for the person-months endpoints.
//!
//! Exercises unit-level (`/api/v1/buildings/{bid}/units/{uid}/person-months`)
//! and building-level (`/api/v1/buildings/{bid}/person-months`) routes.

use axum::http::StatusCode;
use chrono::Datelike;
use common::tenant::TenantRole;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
    create_authenticated_user_with_org, mint_tenant_token, seed_membership, seed_org, TestApp,
    TestUser,
};

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

// ============================================================================
// Negative authorization / validation coverage for the submission endpoints
// (UC-10, pm-security). This pins the refusal contract for the resident→manager
// approval flow next to the happy path:
//
//   - resident submitting for a unit they don't live in   -> 403 NOT_RESIDENT
//   - submission for an out-of-range (future) period       -> 400 INVALID_PERIOD
//   - resident attempting a manager-only approve           -> 403 INSUFFICIENT_ROLE
//   - approving under the WRONG building's path            -> 404 NOT_FOUND
//   - approving a submission that was already decided       -> 409 ALREADY_DECIDED
//
// Overlap note: the 403/409/400 vectors are also exercised against the pending
// list in `person_month_submissions_tests.rs`; the cross-building 404 vector
// (`load_building_submission` rejecting a submission whose unit sits in another
// building) is unique to this suite. Kept here so the full negative matrix is
// discoverable alongside the person-month happy path.
// ============================================================================

struct SubmissionFixture {
    org: Uuid,
    building_a: Uuid,
    building_b: Uuid,
    own_unit: Uuid,
    other_unit: Uuid,
    resident: (Uuid, String),
    manager: (Uuid, String),
}

async fn seed_user(pool: &PgPool, label: &str) -> (Uuid, String) {
    let email = format!("{label}-{}@pm-neg.test", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name) VALUES ($1, 'x', $2) RETURNING id",
    )
    .bind(&email)
    .bind(label)
    .fetch_one(pool)
    .await
    .expect("seed user");
    (id, email)
}

async fn seed_building(pool: &PgPool, org: Uuid, street: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, $2, 'Bratislava', '81101', 'SK') RETURNING id"#,
    )
    .bind(org)
    .bind(street)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id",
    )
    .bind(building)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn submission_fixture(pool: &PgPool) -> SubmissionFixture {
    let org = seed_org(pool, "pm-neg").await;
    // Two buildings in the same org: the submission lives under A, and the
    // cross-building test approves it through B's path.
    let building_a = seed_building(pool, org, "Neg St A").await;
    let building_b = seed_building(pool, org, "Neg St B").await;
    let own_unit = seed_unit(pool, building_a, "1A").await;
    let other_unit = seed_unit(pool, building_a, "1B").await;

    let resident = seed_user(pool, "resident").await;
    let manager = seed_user(pool, "manager").await;
    seed_membership(pool, org, resident.0, "resident").await;
    seed_membership(pool, org, manager.0, "manager").await;
    // The resident lives in 1A only.
    sqlx::query(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type) VALUES ($1, $2, 'tenant')",
    )
    .bind(own_unit)
    .bind(resident.0)
    .execute(pool)
    .await
    .expect("residency");

    SubmissionFixture {
        org,
        building_a,
        building_b,
        own_unit,
        other_unit,
        resident,
        manager,
    }
}

fn this_month() -> (i32, i32) {
    let today = chrono::Utc::now().date_naive();
    (today.year(), today.month() as i32)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn person_month_submissions_negative_authz(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let f = submission_fixture(&pool).await;
    let resident_token =
        mint_tenant_token(f.resident.0, &f.resident.1, f.org, TenantRole::Resident);
    let manager_token = mint_tenant_token(f.manager.0, &f.manager.1, f.org, TenantRole::Manager);
    let (year, month) = this_month();

    // 1. Non-resident submit -> 403. The resident does not live in unit 1B.
    let other = format!(
        "/api/v1/buildings/{}/units/{}/person-months/submissions",
        f.building_a, f.other_unit
    );
    let resp = app
        .execute(
            app.post(&other)
                .bearer(&resident_token)
                .tenant(f.org)
                .json(json!({ "year": year, "month": month, "count": 1 }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    assert_eq!(resp.json_value()["code"], "NOT_RESIDENT", "{}", resp.text());

    // 2. Out-of-range period -> 400. A future period is rejected by
    //    `validate_submission` even for the caller's own unit.
    let own = format!(
        "/api/v1/buildings/{}/units/{}/person-months/submissions",
        f.building_a, f.own_unit
    );
    let resp = app
        .execute(
            app.post(&own)
                .bearer(&resident_token)
                .tenant(f.org)
                .json(json!({ "year": year + 1, "month": month, "count": 1 }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    assert_eq!(
        resp.json_value()["code"],
        "INVALID_PERIOD",
        "{}",
        resp.text()
    );

    // Seed a valid pending submission for the remaining decision vectors.
    let resp = app
        .execute(
            app.post(&own)
                .bearer(&resident_token)
                .tenant(f.org)
                .json(json!({ "year": year, "month": month, "count": 2 }))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let submission_id = resp.json_value()["id"].as_str().unwrap().to_string();

    // 3. Resident approve -> 403. Approval is manager-only.
    let approve_a = format!(
        "/api/v1/buildings/{}/person-months/submissions/{submission_id}/approve",
        f.building_a
    );
    let resp = app
        .execute(
            app.post(&approve_a)
                .bearer(&resident_token)
                .tenant(f.org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    assert_eq!(
        resp.json_value()["code"],
        "INSUFFICIENT_ROLE",
        "{}",
        resp.text()
    );

    // 4. Cross-building submission_id -> 404. A manager of building B cannot act
    //    on a submission whose unit lives in building A; `load_building_submission`
    //    returns NOT_FOUND rather than leaking it across buildings.
    let approve_b = format!(
        "/api/v1/buildings/{}/person-months/submissions/{submission_id}/approve",
        f.building_b
    );
    let resp = app
        .execute(
            app.post(&approve_b)
                .bearer(&manager_token)
                .tenant(f.org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    assert_eq!(resp.json_value()["code"], "NOT_FOUND", "{}", resp.text());

    // 5. Double-approve -> 409. The first approval (correct building) lands the
    //    count; a second decision on the same submission conflicts.
    let resp = app
        .execute(
            app.post(&approve_a)
                .bearer(&manager_token)
                .tenant(f.org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    assert_eq!(resp.json_value()["status"], "approved");

    let resp = app
        .execute(
            app.post(&approve_a)
                .bearer(&manager_token)
                .tenant(f.org)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CONFLICT, "{}", resp.text());
    assert_eq!(
        resp.json_value()["code"],
        "ALREADY_DECIDED",
        "{}",
        resp.text()
    );
}
