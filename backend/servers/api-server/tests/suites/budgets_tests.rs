//! Happy-path and lifecycle integration tests for the budgets endpoints (`/api/v1/budgets/*`).
//!
//! Exercises the budgets, budget items, budget categories, actuals, alerts, and dashboard endpoints.

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn budget_module_happy_path_lifecycle(pool: PgPool) {
    // 1. Initialize the TestApp
    let app = TestApp::new(pool.clone()).await;

    // 2. Seed organization, user, and membership to authenticate requests
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "budhappy").await;

    // 3. Resolve user ID for verification
    let _user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    // 4. Seed building
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
        "name": "Testing Utilities",
        "description": "Electricity and water utilities"
    });
    let resp = app
        .execute(
            app.post("/api/v1/budgets/categories")
                .bearer(&token)
                .tenant(org_id)
                .json(&create_cat_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_category failed: {}",
        resp.text()
    );
    let category = resp.json_value();
    let category_id_str = category["id"].as_str().expect("id missing");
    let category_id = Uuid::parse_str(category_id_str).expect("invalid uuid");

    // 1.2 GET /api/v1/budgets/categories -> list_categories
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/budgets/categories?organization_id={org_id}"
            ))
            .bearer(&token)
            .tenant(org_id)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_categories failed: {}",
        resp.text()
    );
    let categories_list = resp.json_value();
    assert!(categories_list
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));

    // 1.3 PUT /api/v1/budgets/categories/{id} -> update_category
    let update_cat_payload = json!({
        "organization_id": org_id,
        "name": "Updated Utilities",
        "description": "Updated utility desc"
    });
    let resp = app
        .execute(
            app.put(&format!("/api/v1/budgets/categories/{category_id}"))
                .bearer(&token)
                .tenant(org_id)
                .json(&update_cat_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_category failed: {}",
        resp.text()
    );
    let updated_cat = resp.json_value();
    assert_eq!(updated_cat["name"], "Updated Utilities");

    // ========================================================================
    // 2. BUDGET LIFECYCLE (Create, List, Get, Update)
    // ========================================================================

    // 2.1 POST /api/v1/budgets/ -> create_budget
    let create_budget_payload = json!({
        "organization_id": org_id,
        "building_id": building_id,
        "fiscal_year": 2026,
        "name": "Annual Budget 2026",
        "notes": "Draft version"
    });
    let resp = app
        .execute(
            app.post("/api/v1/budgets")
                .bearer(&token)
                .tenant(org_id)
                .json(&create_budget_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "create_budget failed: {}",
        resp.text()
    );
    let budget = resp.json_value();
    let budget_id_str = budget["id"].as_str().expect("budget id missing");
    let budget_id = Uuid::parse_str(budget_id_str).expect("invalid uuid");
    assert_eq!(budget["status"], "draft");

    // 2.2 GET /api/v1/budgets/ -> list_budgets
    let resp = app
        .execute(
            app.get(&format!("/api/v1/budgets?organization_id={org_id}"))
                .bearer(&token)
                .tenant(org_id)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_budgets failed: {}",
        resp.text()
    );
    let budgets_list = resp.json_value();
    assert!(budgets_list
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));

    // 2.3 GET /api/v1/budgets/{id} -> get_budget
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/budgets/{budget_id}?organization_id={org_id}"
            ))
            .bearer(&token)
            .tenant(org_id)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_budget failed: {}",
        resp.text()
    );
    let budget_details = resp.json_value();
    assert_eq!(budget_details["id"], budget_id_str);

    // 2.4 PUT /api/v1/budgets/{id} -> update_budget
    let update_budget_payload = json!({
        "organization_id": org_id,
        "name": "Updated Budget 2026",
        "notes": "Updated notes"
    });
    let resp = app
        .execute(
            app.put(&format!("/api/v1/budgets/{budget_id}"))
                .bearer(&token)
                .tenant(org_id)
                .json(&update_budget_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_budget failed: {}",
        resp.text()
    );
    let updated_budget = resp.json_value();
    assert_eq!(updated_budget["name"], "Updated Budget 2026");

    // ========================================================================
    // 3. BUDGET ITEMS (Add, List, Update)
    // ========================================================================

    // 3.1 POST /api/v1/budgets/{id}/items -> add_budget_item
    let add_item_payload = json!({
        "category_id": category_id,
        "name": "Electricity",
        "description": "Power grid cost",
        "budgeted_amount": "5000.00",
        "notes": "Estimated"
    });
    let resp = app
        .execute(
            app.post(&format!("/api/v1/budgets/{budget_id}/items"))
                .bearer(&token)
                .tenant(org_id)
                .json(&add_item_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add_budget_item failed: {}",
        resp.text()
    );
    let budget_item = resp.json_value();
    let item_id_str = budget_item["id"].as_str().expect("item id missing");
    let item_id = Uuid::parse_str(item_id_str).expect("invalid uuid");

    // 3.2 GET /api/v1/budgets/{id}/items -> list_budget_items
    let resp = app
        .execute(
            app.get(&format!("/api/v1/budgets/{budget_id}/items"))
                .bearer(&token)
                .tenant(org_id)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_budget_items failed: {}",
        resp.text()
    );
    let items_list = resp.json_value();
    assert!(items_list
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));

    // 3.3 PUT /api/v1/budgets/items/{item_id} -> update_budget_item
    let update_item_payload = json!({
        "name": "Electricity Updated",
        "budgeted_amount": "5500.00"
    });
    let resp = app
        .execute(
            app.put(&format!("/api/v1/budgets/items/{item_id}"))
                .bearer(&token)
                .tenant(org_id)
                .json(&update_item_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_budget_item failed: {}",
        resp.text()
    );
    let updated_item = resp.json_value();
    assert_eq!(updated_item["name"], "Electricity Updated");

    // ========================================================================
    // 4. BUDGET WORKFLOW (Submit, Approve, Activate)
    // ========================================================================

    // 4.1 POST /api/v1/budgets/{id}/submit -> submit_budget
    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/budgets/{budget_id}/submit?organization_id={org_id}"
            ))
            .bearer(&token)
            .tenant(org_id)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "submit_budget failed: {}",
        resp.text()
    );
    let submitted_budget = resp.json_value();
    assert_eq!(submitted_budget["status"], "pending_approval");

    // 4.2 POST /api/v1/budgets/{id}/approve -> approve_budget
    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/budgets/{budget_id}/approve?organization_id={org_id}"
            ))
            .bearer(&token)
            .tenant(org_id)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "approve_budget failed: {}",
        resp.text()
    );
    let approved_budget = resp.json_value();
    assert_eq!(approved_budget["status"], "approved");

    // 4.3 POST /api/v1/budgets/{id}/activate -> activate_budget
    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/budgets/{budget_id}/activate?organization_id={org_id}"
            ))
            .bearer(&token)
            .tenant(org_id)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "activate_budget failed: {}",
        resp.text()
    );
    let activated_budget = resp.json_value();
    assert_eq!(activated_budget["status"], "active");

    // ========================================================================
    // 5. RECORD ACTUALS
    // ========================================================================

    // 5.1 POST /api/v1/budgets/items/{item_id}/actuals -> record_actual
    let record_actual_payload = json!({
        "amount": "6000.00", // exercises variance warning (exceeds budgeted amount of 5500.00)
        "description": "Jan 2026 bill",
        "transaction_date": "2026-01-31"
    });
    let resp = app
        .execute(
            app.post(&format!("/api/v1/budgets/items/{item_id}/actuals"))
                .bearer(&token)
                .tenant(org_id)
                .json(&record_actual_payload)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "record_actual failed: {}",
        resp.text()
    );
    let actual = resp.json_value();
    let actual_id_str = actual["id"].as_str().expect("actual id missing");
    let _actual_id = Uuid::parse_str(actual_id_str).expect("invalid uuid");

    // 5.2 GET /api/v1/budgets/items/{item_id}/actuals -> list_actuals
    let resp = app
        .execute(
            app.get(&format!("/api/v1/budgets/items/{item_id}/actuals"))
                .bearer(&token)
                .tenant(org_id)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_actuals failed: {}",
        resp.text()
    );
    let actuals_list = resp.json_value();
    assert!(actuals_list
        .as_array()
        .map(|arr| !arr.is_empty())
        .unwrap_or(false));

    // ========================================================================
    // 6. STATISTICS, ALERTS, AND DASHBOARD
    // ========================================================================

    // 6.1 GET /api/v1/budgets/{id}/summary -> get_budget_summary
    let resp = app
        .execute(
            app.get(&format!("/api/v1/budgets/{budget_id}/summary"))
                .bearer(&token)
                .tenant(org_id)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_budget_summary failed: {}",
        resp.text()
    );
    let summary = resp.json_value();
    assert!(
        summary["total_budgeted"].as_str().is_some()
            || summary["total_budgeted"].as_f64().is_some()
    );

    // 6.2 GET /api/v1/budgets/{id}/variance -> get_category_variance
    let resp = app
        .execute(
            app.get(&format!("/api/v1/budgets/{budget_id}/variance"))
                .bearer(&token)
                .tenant(org_id)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_category_variance failed: {}",
        resp.text()
    );

    // 6.3 GET /api/v1/budgets/{id}/alerts -> list_variance_alerts
    let resp = app
        .execute(
            app.get(&format!("/api/v1/budgets/{budget_id}/alerts"))
                .bearer(&token)
                .tenant(org_id)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_variance_alerts failed: {}",
        resp.text()
    );
    let alerts_list = resp.json_value();
    if let Some(alerts) = alerts_list.as_array() {
        if !alerts.is_empty() {
            let alert_id_str = alerts[0]["id"].as_str().expect("alert id missing");
            let alert_id = Uuid::parse_str(alert_id_str).expect("invalid uuid");

            // 6.4 POST /api/v1/budgets/alerts/{id}/acknowledge -> acknowledge_alert
            let ack_payload = json!({
                "notes": "Acknowledged electricity over budget"
            });
            let ack_resp = app
                .execute(
                    app.post(&format!("/api/v1/budgets/alerts/{alert_id}/acknowledge"))
                        .bearer(&token)
                        .tenant(org_id)
                        .json(&ack_payload)
                        .build(),
                )
                .await;
            assert_eq!(
                ack_resp.status,
                StatusCode::OK,
                "acknowledge_alert failed: {}",
                ack_resp.text()
            );
        }
    }

    // 6.5 GET /api/v1/budgets/dashboard -> get_dashboard
    let resp = app
        .execute(
            app.get(&format!(
                "/api/v1/budgets/dashboard?organization_id={org_id}&building_id={building_id}"
            ))
            .bearer(&token)
            .tenant(org_id)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_dashboard failed: {}",
        resp.text()
    );

    // ========================================================================
    // 7. CLEANUP / DELETE ROUTINES (Close & Delete Budget/Category/Items)
    // ========================================================================

    // 7.1 POST /api/v1/budgets/{id}/close -> close_budget
    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/budgets/{budget_id}/close?organization_id={org_id}"
            ))
            .bearer(&token)
            .tenant(org_id)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "close_budget failed: {}",
        resp.text()
    );
    let closed_budget = resp.json_value();
    assert_eq!(closed_budget["status"], "closed");

    // 7.2 DELETE /api/v1/budgets/items/{item_id} -> delete_budget_item
    let resp = app
        .execute(
            app.delete(&format!("/api/v1/budgets/items/{item_id}"))
                .bearer(&token)
                .tenant(org_id)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete_budget_item failed: {}",
        resp.text()
    );

    // 7.3 DELETE /api/v1/budgets/categories/{id} -> delete_category
    let resp = app
        .execute(
            app.delete(&format!(
                "/api/v1/budgets/categories/{category_id}?organization_id={org_id}"
            ))
            .bearer(&token)
            .tenant(org_id)
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete_category failed: {}",
        resp.text()
    );
}
