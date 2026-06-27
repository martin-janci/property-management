//! Happy-path integration tests for the budget endpoints (`/api/v1/budgets/*`).
//!
//! Exercises 18 different endpoints covering budget lifecycle, items, categories,
//! actuals, variance reports, and dashboard.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn budgets_endpoints_happy_path(pool: PgPool) {
    // 1. Initialize the TestApp
    let app = TestApp::new(pool.clone()).await;

    // 2. Seed organization, user, and membership to authenticate requests
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "budgethappy").await;

    let session = app.session(token, org_id);

    // 3. Seed building
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Budget Street 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(&app.pool)
    .await
    .expect("seed building");

    // ========================================================================
    // 1. BUDGET CATEGORIES
    // ========================================================================

    // 1.1 POST /api/v1/budgets/categories -> create_category
    let create_cat_payload = json!({
        "organization_id": org_id,
        "name": "General Maintenance",
        "description": "Routine upkeep of building and systems",
        "sort_order": 10
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/budgets/categories")
                .json(&create_cat_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_category should succeed: {}",
        resp.text()
    );
    let category = resp.json_value();
    let category_id_str = category["id"].as_str().expect("id missing");
    let category_id = Uuid::parse_str(category_id_str).expect("invalid category uuid");

    // 1.2 GET /api/v1/budgets/categories -> list_categories
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/budgets/categories?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let categories_list = resp.json_value();
    assert!(categories_list
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));

    // 1.3 PUT /api/v1/budgets/categories/{id} -> update_category
    let update_cat_payload = json!({
        "organization_id": org_id,
        "name": "Upkeep & Repairs",
        "description": "Routine upkeep plus small repairs",
        "sort_order": 12
    });
    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/budgets/categories/{category_id}"))
                .json(&update_cat_payload)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let updated_cat = resp.json_value();
    assert_eq!(updated_cat["name"], "Upkeep & Repairs");

    // ========================================================================
    // 2. BUDGET LIFECYCLE (Story 24.1)
    // ========================================================================

    // 2.1 POST /api/v1/budgets -> create_budget
    let create_budget_payload = json!({
        "organization_id": org_id,
        "building_id": building_id,
        "fiscal_year": 2026,
        "name": "Standard Operating Budget",
        "notes": "Initial draft for 2026 financial planning"
    });
    let resp = app
        .execute(
            session
                .post("/api/v1/budgets")
                .json(&create_budget_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_budget should succeed: {}",
        resp.text()
    );
    let budget = resp.json_value();
    let budget_id_str = budget["id"].as_str().expect("id missing");
    let budget_id = Uuid::parse_str(budget_id_str).expect("invalid budget uuid");

    // Create a temporary draft budget to test delete
    let delete_budget_payload = json!({
        "organization_id": org_id,
        "building_id": building_id,
        "fiscal_year": 2027,
        "name": "Temp Delete Budget",
        "notes": "To be deleted"
    });
    let resp_temp = app
        .execute(
            session
                .post("/api/v1/budgets")
                .json(&delete_budget_payload)
                .build(),
        )
        .await;
    assert_eq!(resp_temp.status, StatusCode::CREATED);
    let temp_budget_id_str = resp_temp.json_value()["id"]
        .as_str()
        .expect("id missing")
        .to_string();
    let temp_budget_id = Uuid::parse_str(&temp_budget_id_str).unwrap();

    // 2.2 GET /api/v1/budgets -> list_budgets
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/budgets?organization_id={org_id}"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let budgets_list = resp.json_value();
    assert!(budgets_list
        .as_array()
        .map(|arr| arr.len() >= 2)
        .unwrap_or(false));

    // 2.3 GET /api/v1/budgets/{id} -> get_budget
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/budgets/{budget_id}?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let budget_details = resp.json_value();
    assert_eq!(budget_details["id"], budget_id_str);

    // 2.4 PUT /api/v1/budgets/{id} -> update_budget
    let update_budget_payload = json!({
        "organization_id": org_id,
        "name": "Main Operating Budget 2026",
        "notes": "Updated operating budget notes"
    });
    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/budgets/{budget_id}"))
                .json(&update_budget_payload)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let updated_budget = resp.json_value();
    assert_eq!(updated_budget["name"], "Main Operating Budget 2026");

    // ========================================================================
    // 3. BUDGET ITEMS (Story 24.2)
    // ========================================================================

    // 3.1 POST /api/v1/budgets/{id}/items -> add_budget_item
    let add_item_payload = json!({
        "category_id": category_id,
        "name": "Elevator Maintenance",
        "description": "Bi-annual inspections & repair contract",
        "budgeted_amount": "5000.00",
        "notes": "Contract signed in Jan"
    });
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/budgets/{budget_id}/items"))
                .json(&add_item_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add_budget_item should succeed: {}",
        resp.text()
    );
    let item = resp.json_value();
    let item_id_str = item["id"].as_str().expect("id missing");
    let item_id = Uuid::parse_str(item_id_str).expect("invalid item uuid");

    // 3.2 GET /api/v1/budgets/{id}/items -> list_budget_items
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/budgets/{budget_id}/items"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let items_list = resp.json_value();
    assert!(items_list
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));

    // 3.3 PUT /api/v1/budgets/items/{item_id} -> update_budget_item
    let update_item_payload = json!({
        "name": "Elevator Routine Maintenance",
        "budgeted_amount": "6000.00"
    });
    let resp = app
        .execute(
            session
                .put(&format!("/api/v1/budgets/items/{item_id}"))
                .json(&update_item_payload)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let updated_item = resp.json_value();
    assert_eq!(updated_item["name"], "Elevator Routine Maintenance");
    assert_eq!(updated_item["budgetedAmount"], "6000.00");

    // ========================================================================
    // 4. BUDGET PROGRESSION & WORKFLOW (Story 24.1 / 24.3)
    // ========================================================================

    // 4.1 POST /api/v1/budgets/{id}/submit -> submit_budget
    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/budgets/{budget_id}/submit?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "submit_budget should succeed: {}",
        resp.text()
    );
    let submitted_budget = resp.json_value();
    assert_eq!(submitted_budget["status"], "pending_approval");

    // 4.2 POST /api/v1/budgets/{id}/approve -> approve_budget
    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/budgets/{budget_id}/approve?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let approved_budget = resp.json_value();
    assert_eq!(approved_budget["status"], "approved");

    // 4.3 POST /api/v1/budgets/{id}/activate -> activate_budget
    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/budgets/{budget_id}/activate?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let active_budget = resp.json_value();
    assert_eq!(active_budget["status"], "active");

    // ========================================================================
    // 5. BUDGET ACTUALS & VARIANCE ALERTS (Story 24.3)
    // ========================================================================

    // 5.1 POST /api/v1/budgets/items/{item_id}/actuals -> record_actual
    let record_actual_payload = json!({
        "amount": "1500.00",
        "description": "Spring elevator inspection fee",
        "transaction_date": "2026-05-10"
    });
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/budgets/items/{item_id}/actuals"))
                .json(&record_actual_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "record_actual should succeed: {}",
        resp.text()
    );
    let actual = resp.json_value();
    let actual_id_str = actual["id"].as_str().expect("id missing");
    let actual_id = Uuid::parse_str(actual_id_str).expect("invalid actual uuid");

    // 5.2 GET /api/v1/budgets/items/{item_id}/actuals -> list_actuals
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/budgets/items/{item_id}/actuals"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let actuals_list = resp.json_value();
    assert!(actuals_list
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));

    // Record an over-budget actual to trigger an alert
    // Budgeted amount: 6000.00. Current actuals: 1500.00. Adding 5000.00 -> 6500.00 (108% of budget).
    let record_over_budget_payload = json!({
        "amount": "5000.00",
        "description": "Emergency elevator cable replacement",
        "transaction_date": "2026-06-12"
    });
    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/budgets/items/{item_id}/actuals"))
                .json(&record_over_budget_payload)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED);

    // 5.3 GET /api/v1/budgets/{id}/alerts -> list_variance_alerts
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/budgets/{budget_id}/alerts"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let alerts_list = resp.json_value();

    // If there is an alert generated, let's acknowledge it
    if let Some(alerts_arr) = alerts_list.as_array() {
        if !alerts_arr.is_empty() {
            let alert_id_str = alerts_arr[0]["id"].as_str().expect("id missing");
            let alert_id = Uuid::parse_str(alert_id_str).expect("invalid alert uuid");

            // 5.4 POST /api/v1/budgets/alerts/{id}/acknowledge -> acknowledge_alert
            let ack_payload = json!({
                "notes": "Acknowledged. Extra funds will be sourced from reserves."
            });
            let resp_ack = app
                .execute(
                    session
                        .post(&format!("/api/v1/budgets/alerts/{alert_id}/acknowledge"))
                        .json(&ack_payload)
                        .build(),
                )
                .await;
            assert_eq!(
                resp_ack.status,
                StatusCode::OK,
                "acknowledge_alert should succeed: {}",
                resp_ack.text()
            );
        }
    }

    // ========================================================================
    // 6. STATISTICS & REPORTS / DASHBOARD (Story 24.3)
    // ========================================================================

    // 6.1 GET /api/v1/budgets/{id}/summary -> get_budget_summary
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/budgets/{budget_id}/summary"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let summary = resp.json_value();
    assert_eq!(summary["totalBudgeted"], "6000.00");
    assert_eq!(summary["totalActual"], "6500.00");

    // 6.2 GET /api/v1/budgets/{id}/variance -> get_category_variance
    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/budgets/{budget_id}/variance"))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let variance = resp.json_value();
    assert!(variance
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));

    // 6.3 GET /api/v1/budgets/dashboard -> get_dashboard
    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/budgets/dashboard?organization_id={org_id}&building_id={building_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let dashboard = resp.json_value();
    assert!(dashboard["activeBudget"].is_object());

    // ========================================================================
    // 7. CLEANUP & DELETE RESTRICTIONS (Story 24.1 / 24.2)
    // ========================================================================

    // 7.1 POST /api/v1/budgets/{id}/close -> close_budget
    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/budgets/{budget_id}/close?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK);
    let closed_budget = resp.json_value();
    assert_eq!(closed_budget["status"], "closed");

    // 7.2 DELETE /api/v1/budgets/{id} against closed budget -> should fail with 400 (only draft deleted)
    let resp = app
        .execute(
            session
                .delete(&format!(
                    "/api/v1/budgets/{budget_id}?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST);

    // 7.3 DELETE /api/v1/budgets/{id} against the temp draft budget -> should succeed (204)
    let resp = app
        .execute(
            session
                .delete(&format!(
                    "/api/v1/budgets/{temp_budget_id}?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);

    // 7.4 DELETE /api/v1/budgets/categories/{id} -> delete_category
    let resp = app
        .execute(
            session
                .delete(&format!(
                    "/api/v1/budgets/categories/{category_id}?organization_id={org_id}"
                ))
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT);
}
