//! Happy-path integration tests for the budget **capital-plan**, **forecast**,
//! and `delete_budget` endpoints (`/api/v1/budgets/*`).
//!
//! Wave 8 batch 2 (BIT-406). These endpoints are the capital-planning and
//! forecasting sub-module of `budgets.rs` plus the one budget-CRUD verb that
//! the existing `budgets_tests.rs` did not exercise (`DELETE /budgets/{id}`).
//! All of them were classified `partial` (real handler, no happy-path test) in
//! `docs/endpoint-checklist/groups/finance.md` and had no coverage in any
//! suite. This file drives each through its 2xx success path so they can be
//! promoted to `done`.
//!
//! Like `budgets_tests.rs`, every handler here is gated through `RlsConnection`,
//! which resolves tenant context from the `X-Tenant-ID` header plus an active
//! `organization_members` row. `create_authenticated_user_with_org` provisions
//! an `org_admin` membership, so a plain login token + `.tenant(org_id)` header
//! is a sufficient principal — no `tenant_id` JWT claim is required.
//!
//! Endpoints covered (14):
//!   Capital plans (8)
//!     1. POST   /api/v1/budgets/capital-plans            (create_capital_plan)
//!     2. GET    /api/v1/budgets/capital-plans            (list_capital_plans)
//!     3. GET    /api/v1/budgets/capital-plans/summary    (get_yearly_capital_summary)
//!     4. GET    /api/v1/budgets/capital-plans/{id}       (get_capital_plan)
//!     5. PUT    /api/v1/budgets/capital-plans/{id}       (update_capital_plan)
//!     6. POST   /api/v1/budgets/capital-plans/{id}/start (start_capital_plan)
//!     7. POST   /api/v1/budgets/capital-plans/{id}/complete (complete_capital_plan)
//!     8. DELETE /api/v1/budgets/capital-plans/{id}       (delete_capital_plan)
//!   Forecasts (5)
//!     9. POST   /api/v1/budgets/forecasts                (create_forecast)
//!    10. GET    /api/v1/budgets/forecasts                (list_forecasts)
//!    11. GET    /api/v1/budgets/forecasts/{id}           (get_forecast)
//!    12. PUT    /api/v1/budgets/forecasts/{id}           (update_forecast)
//!    13. DELETE /api/v1/budgets/forecasts/{id}           (delete_forecast)
//!   Budget CRUD (1)
//!    14. DELETE /api/v1/budgets/{id}                     (delete_budget)

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

/// Seed a building owned by `org_id` and return its id.
async fn seed_building(app: &TestApp, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Capital Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed building")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn capital_plans_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "caphappy").await;
    let building_id = seed_building(&app, org_id).await;

    // ------------------------------------------------------------------
    // 1. POST /api/v1/budgets/capital-plans -> create_capital_plan
    // ------------------------------------------------------------------
    let create_payload = json!({
        "organization_id": org_id,
        "building_id": building_id,
        "name": "Roof Replacement",
        "description": "Full roof replacement for building A",
        "estimated_cost": "50000.00",
        "funding_source": "reserve_fund",
        "target_year": 2027,
        "target_quarter": 2,
        "priority": "high",
        "notes": "Planned capital expense"
    });
    let resp = app
        .execute(
            app.post("/api/v1/budgets/capital-plans")
                .bearer(&token)
                .tenant(org_id)
                .json(&create_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_capital_plan failed: {}",
        resp.text()
    );
    let plan = resp.json_value();
    let plan_id_str = plan["id"].as_str().expect("plan id missing");
    let plan_id = Uuid::parse_str(plan_id_str).expect("invalid plan uuid");

    // ------------------------------------------------------------------
    // 2. GET /api/v1/budgets/capital-plans -> list_capital_plans
    // ------------------------------------------------------------------
    let list_url = format!("/api/v1/budgets/capital-plans?organization_id={org_id}");
    let resp = app
        .execute(app.get(&list_url).bearer(&token).tenant(org_id).build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_capital_plans failed: {}",
        resp.text()
    );
    let plans = resp.json_value();
    assert!(
        plans.as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "expected at least one capital plan: {plans}"
    );

    // ------------------------------------------------------------------
    // 3. GET /api/v1/budgets/capital-plans/summary -> get_yearly_capital_summary
    // ------------------------------------------------------------------
    let summary_url = format!("/api/v1/budgets/capital-plans/summary?organization_id={org_id}");
    let resp = app
        .execute(app.get(&summary_url).bearer(&token).tenant(org_id).build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_yearly_capital_summary failed: {}",
        resp.text()
    );

    // ------------------------------------------------------------------
    // 4. GET /api/v1/budgets/capital-plans/{id} -> get_capital_plan
    // ------------------------------------------------------------------
    let get_url = format!("/api/v1/budgets/capital-plans/{plan_id}?organization_id={org_id}");
    let resp = app
        .execute(app.get(&get_url).bearer(&token).tenant(org_id).build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_capital_plan failed: {}",
        resp.text()
    );

    // ------------------------------------------------------------------
    // 5. PUT /api/v1/budgets/capital-plans/{id} -> update_capital_plan
    // ------------------------------------------------------------------
    let update_payload = json!({
        "organization_id": org_id,
        "name": "Roof Replacement (revised)",
        "estimated_cost": "52000.00",
        "priority": "critical"
    });
    let update_url = format!("/api/v1/budgets/capital-plans/{plan_id}");
    let resp = app
        .execute(
            app.put(&update_url)
                .bearer(&token)
                .tenant(org_id)
                .json(&update_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_capital_plan failed: {}",
        resp.text()
    );

    // ------------------------------------------------------------------
    // 6. POST /api/v1/budgets/capital-plans/{id}/start -> start_capital_plan
    //    State transition (planned -> in_progress); assert a 2xx success.
    // ------------------------------------------------------------------
    let start_url =
        format!("/api/v1/budgets/capital-plans/{plan_id}/start?organization_id={org_id}");
    let resp = app
        .execute(app.post(&start_url).bearer(&token).tenant(org_id).build())
        .await;
    assert!(
        resp.status.is_success(),
        "start_capital_plan should succeed, got {}: {}",
        resp.status,
        resp.text()
    );

    // ------------------------------------------------------------------
    // 7. POST /api/v1/budgets/capital-plans/{id}/complete -> complete_capital_plan
    // ------------------------------------------------------------------
    let complete_payload = json!({
        "organization_id": org_id,
        "actual_cost": "48000.00"
    });
    let complete_url = format!("/api/v1/budgets/capital-plans/{plan_id}/complete");
    let resp = app
        .execute(
            app.post(&complete_url)
                .bearer(&token)
                .tenant(org_id)
                .json(&complete_payload)
                .build(),
        )
        .await;
    assert!(
        resp.status.is_success(),
        "complete_capital_plan should succeed, got {}: {}",
        resp.status,
        resp.text()
    );

    // ------------------------------------------------------------------
    // 8. DELETE /api/v1/budgets/capital-plans/{id} -> delete_capital_plan
    //    Use a fresh plan (never started) so it is in a deletable state.
    // ------------------------------------------------------------------
    let throwaway_payload = json!({
        "organization_id": org_id,
        "building_id": building_id,
        "name": "Lobby Refurbishment",
        "estimated_cost": "10000.00",
        "funding_source": "operating",
        "target_year": 2028
    });
    let resp = app
        .execute(
            app.post("/api/v1/budgets/capital-plans")
                .bearer(&token)
                .tenant(org_id)
                .json(&throwaway_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "seed plan: {}",
        resp.text()
    );
    let throwaway = resp.json_value();
    let throwaway_id = throwaway["id"].as_str().expect("id missing");

    let delete_url =
        format!("/api/v1/budgets/capital-plans/{throwaway_id}?organization_id={org_id}");
    let resp = app
        .execute(
            app.delete(&delete_url)
                .bearer(&token)
                .tenant(org_id)
                .build(),
        )
        .await;
    assert!(
        resp.status.is_success(),
        "delete_capital_plan should succeed, got {}: {}",
        resp.status,
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn forecasts_and_budget_delete_happy_path(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "fcsthappy").await;
    let building_id = seed_building(&app, org_id).await;

    // ------------------------------------------------------------------
    // 9. POST /api/v1/budgets/forecasts -> create_forecast
    // ------------------------------------------------------------------
    let create_payload = json!({
        "organization_id": org_id,
        "building_id": building_id,
        "name": "5-Year Reserve Forecast",
        "forecast_type": "reserve",
        "start_year": 2027,
        "end_year": 2031,
        "inflation_rate": "2.5",
        "parameters": { "base_contribution": 1000 },
        "notes": "Reserve fund growth forecast"
    });
    let resp = app
        .execute(
            app.post("/api/v1/budgets/forecasts")
                .bearer(&token)
                .tenant(org_id)
                .json(&create_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_forecast failed: {}",
        resp.text()
    );
    let forecast = resp.json_value();
    let forecast_id_str = forecast["id"].as_str().expect("forecast id missing");
    let forecast_id = Uuid::parse_str(forecast_id_str).expect("invalid forecast uuid");

    // ------------------------------------------------------------------
    // 10. GET /api/v1/budgets/forecasts -> list_forecasts
    // ------------------------------------------------------------------
    let list_url = format!("/api/v1/budgets/forecasts?organization_id={org_id}");
    let resp = app
        .execute(app.get(&list_url).bearer(&token).tenant(org_id).build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_forecasts failed: {}",
        resp.text()
    );
    let forecasts = resp.json_value();
    assert!(
        forecasts.as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "expected at least one forecast: {forecasts}"
    );

    // ------------------------------------------------------------------
    // 11. GET /api/v1/budgets/forecasts/{id} -> get_forecast
    // ------------------------------------------------------------------
    let get_url = format!("/api/v1/budgets/forecasts/{forecast_id}?organization_id={org_id}");
    let resp = app
        .execute(app.get(&get_url).bearer(&token).tenant(org_id).build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_forecast failed: {}",
        resp.text()
    );

    // ------------------------------------------------------------------
    // 12. PUT /api/v1/budgets/forecasts/{id} -> update_forecast
    // ------------------------------------------------------------------
    let update_payload = json!({
        "organization_id": org_id,
        "name": "5-Year Reserve Forecast (revised)",
        "inflation_rate": "3.0",
        "notes": "Revised inflation assumption"
    });
    let update_url = format!("/api/v1/budgets/forecasts/{forecast_id}");
    let resp = app
        .execute(
            app.put(&update_url)
                .bearer(&token)
                .tenant(org_id)
                .json(&update_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_forecast failed: {}",
        resp.text()
    );

    // ------------------------------------------------------------------
    // 13. DELETE /api/v1/budgets/forecasts/{id} -> delete_forecast
    // ------------------------------------------------------------------
    let delete_url = format!("/api/v1/budgets/forecasts/{forecast_id}?organization_id={org_id}");
    let resp = app
        .execute(
            app.delete(&delete_url)
                .bearer(&token)
                .tenant(org_id)
                .build(),
        )
        .await;
    assert!(
        resp.status.is_success(),
        "delete_forecast should succeed, got {}: {}",
        resp.status,
        resp.text()
    );

    // ------------------------------------------------------------------
    // 14. DELETE /api/v1/budgets/{id} -> delete_budget
    //     A freshly-created budget is in `draft`, the only deletable state.
    // ------------------------------------------------------------------
    let budget_payload = json!({
        "organization_id": org_id,
        "building_id": building_id,
        "fiscal_year": 2027,
        "name": "Disposable Draft Budget",
        "notes": "Created solely to exercise delete_budget"
    });
    let resp = app
        .execute(
            app.post("/api/v1/budgets")
                .bearer(&token)
                .tenant(org_id)
                .json(&budget_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_budget (for delete) failed: {}",
        resp.text()
    );
    let budget = resp.json_value();
    let budget_id = budget["id"].as_str().expect("budget id missing");

    let delete_budget_url = format!("/api/v1/budgets/{budget_id}?organization_id={org_id}");
    let resp = app
        .execute(
            app.delete(&delete_budget_url)
                .bearer(&token)
                .tenant(org_id)
                .build(),
        )
        .await;
    assert!(
        resp.status.is_success(),
        "delete_budget should succeed, got {}: {}",
        resp.status,
        resp.text()
    );
}
