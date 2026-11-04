//! Authorization, tenant-scoping and status-machine tests for the budgets
//! endpoints (`/api/v1/budgets/*`).
//!
//! Covers the gaps closed in `routes/budgets.rs`:
//! * mutations require a manager-level role (resident → 403);
//! * `organization_id` must match the active tenant, and id-only routes
//!   (`/{id}/items`, `/items/{item_id}`, `/{id}/summary`, …) resolve their
//!   budget inside the active tenant (cross-org ids → 404). The test database
//!   role is a superuser, so RLS does not apply here — these assertions
//!   exercise the explicit handler checks;
//! * budget lines change only in `draft`, actuals only in `active` (400);
//! * the creator cannot approve their own budget (403);
//! * an unknown / foreign `category_id` is a 400, not a 500.

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
    create_authenticated_user, seed_membership, seed_org, TestApp, TestResponse, TestUser,
};

struct Member {
    token: String,
    id: Uuid,
}

async fn member(app: &TestApp, org_id: Uuid, role: &str) -> Member {
    let user = TestUser::new();
    let (token, _) = create_authenticated_user(app, &user).await;
    let id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");
    seed_membership(&app.pool, org_id, id, role).await;
    Member { token, id }
}

async fn post(app: &TestApp, who: &Member, org_id: Uuid, uri: &str, body: Value) -> TestResponse {
    app.execute(
        app.post(uri)
            .bearer(&who.token)
            .tenant(org_id)
            .json(&body)
            .build(),
    )
    .await
}

async fn post_empty(app: &TestApp, who: &Member, org_id: Uuid, uri: &str) -> TestResponse {
    app.execute(app.post(uri).bearer(&who.token).tenant(org_id).build())
        .await
}

async fn put(app: &TestApp, who: &Member, org_id: Uuid, uri: &str, body: Value) -> TestResponse {
    app.execute(
        app.put(uri)
            .bearer(&who.token)
            .tenant(org_id)
            .json(&body)
            .build(),
    )
    .await
}

async fn get(app: &TestApp, who: &Member, org_id: Uuid, uri: &str) -> TestResponse {
    app.execute(app.get(uri).bearer(&who.token).tenant(org_id).build())
        .await
}

async fn delete(app: &TestApp, who: &Member, org_id: Uuid, uri: &str) -> TestResponse {
    app.execute(app.delete(uri).bearer(&who.token).tenant(org_id).build())
        .await
}

fn id_of(resp: &TestResponse) -> Uuid {
    Uuid::parse_str(resp.json_value()["id"].as_str().expect("id missing")).expect("uuid")
}

async fn create_category(app: &TestApp, who: &Member, org_id: Uuid, name: &str) -> Uuid {
    let resp = post(
        app,
        who,
        org_id,
        "/api/v1/budgets/categories",
        json!({ "organization_id": org_id, "name": name }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    id_of(&resp)
}

async fn create_budget(app: &TestApp, who: &Member, org_id: Uuid, year: i32) -> Uuid {
    let resp = post(
        app,
        who,
        org_id,
        "/api/v1/budgets",
        json!({ "organization_id": org_id, "fiscal_year": year, "name": format!("Budget {year}") }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    id_of(&resp)
}

async fn add_item(
    app: &TestApp,
    who: &Member,
    org_id: Uuid,
    budget_id: Uuid,
    category_id: Uuid,
) -> TestResponse {
    post(
        app,
        who,
        org_id,
        &format!("/api/v1/budgets/{budget_id}/items"),
        json!({ "category_id": category_id, "name": "Line", "budgeted_amount": "100.00" }),
    )
    .await
}

fn actual_body() -> Value {
    json!({ "amount": "10.00", "description": "Invoice", "transaction_date": "2026-03-01" })
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_mutate_budgets(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&app.pool, "budauthz-res").await;
    let manager = member(&app, org_id, "manager").await;
    let resident = member(&app, org_id, "resident").await;

    // create
    let resp = post(
        &app,
        &resident,
        org_id,
        "/api/v1/budgets",
        json!({ "organization_id": org_id, "fiscal_year": 2026, "name": "Sneaky" }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    let category_id = create_category(&app, &manager, org_id, "Utilities").await;
    let budget_id = create_budget(&app, &manager, org_id, 2026).await;
    let resp = add_item(&app, &manager, org_id, budget_id, category_id).await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let item_id = id_of(&resp);

    // budget-level mutations
    let q = format!("organization_id={org_id}");
    for uri in [
        format!("/api/v1/budgets/{budget_id}/submit?{q}"),
        format!("/api/v1/budgets/{budget_id}/approve?{q}"),
        format!("/api/v1/budgets/{budget_id}/activate?{q}"),
        format!("/api/v1/budgets/{budget_id}/close?{q}"),
    ] {
        let resp = post_empty(&app, &resident, org_id, &uri).await;
        assert_eq!(resp.status, StatusCode::FORBIDDEN, "{uri}: {}", resp.text());
    }
    let resp = put(
        &app,
        &resident,
        org_id,
        &format!("/api/v1/budgets/{budget_id}"),
        json!({ "organization_id": org_id, "name": "Renamed" }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let resp = delete(
        &app,
        &resident,
        org_id,
        &format!("/api/v1/budgets/{budget_id}?{q}"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    // item / actual / category mutations
    let resp = add_item(&app, &resident, org_id, budget_id, category_id).await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let resp = put(
        &app,
        &resident,
        org_id,
        &format!("/api/v1/budgets/items/{item_id}"),
        json!({ "name": "X" }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let resp = delete(
        &app,
        &resident,
        org_id,
        &format!("/api/v1/budgets/items/{item_id}"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let resp = post(
        &app,
        &resident,
        org_id,
        &format!("/api/v1/budgets/items/{item_id}/actuals"),
        actual_body(),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let resp = post(
        &app,
        &resident,
        org_id,
        "/api/v1/budgets/categories",
        json!({ "organization_id": org_id, "name": "Mine" }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let resp = delete(
        &app,
        &resident,
        org_id,
        &format!("/api/v1/budgets/categories/{category_id}?{q}"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    // Reads stay available to members.
    let resp = get(
        &app,
        &resident,
        org_id,
        &format!("/api/v1/budgets/{budget_id}?{q}"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    let resp = get(
        &app,
        &resident,
        org_id,
        &format!("/api/v1/budgets/{budget_id}/items"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());

    // Nothing was changed by the resident.
    let (status, items): (String, i64) = sqlx::query_as(
        "SELECT b.status, (SELECT COUNT(*) FROM budget_items WHERE budget_id = b.id) \
         FROM budgets b WHERE b.id = $1",
    )
    .bind(budget_id)
    .fetch_one(&app.pool)
    .await
    .expect("budget row");
    assert_eq!(status, "draft");
    assert_eq!(items, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_budget_and_item_ids_are_not_found(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&app.pool, "budauthz-a").await;
    let org_b = seed_org(&app.pool, "budauthz-b").await;
    let manager_a = member(&app, org_a, "manager").await;
    let manager_b = member(&app, org_b, "manager").await;

    let category_b = create_category(&app, &manager_b, org_b, "B utilities").await;
    let budget_b = create_budget(&app, &manager_b, org_b, 2026).await;
    let resp = add_item(&app, &manager_b, org_b, budget_b, category_b).await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let item_b = id_of(&resp);

    // Manager of A, acting in A, addressing B's ids.
    let qa = format!("organization_id={org_a}");
    let not_found = [
        get(
            &app,
            &manager_a,
            org_a,
            &format!("/api/v1/budgets/{budget_b}?{qa}"),
        )
        .await,
        get(
            &app,
            &manager_a,
            org_a,
            &format!("/api/v1/budgets/{budget_b}/items"),
        )
        .await,
        get(
            &app,
            &manager_a,
            org_a,
            &format!("/api/v1/budgets/{budget_b}/summary"),
        )
        .await,
        get(
            &app,
            &manager_a,
            org_a,
            &format!("/api/v1/budgets/{budget_b}/variance"),
        )
        .await,
        get(
            &app,
            &manager_a,
            org_a,
            &format!("/api/v1/budgets/{budget_b}/alerts"),
        )
        .await,
        get(
            &app,
            &manager_a,
            org_a,
            &format!("/api/v1/budgets/items/{item_b}/actuals"),
        )
        .await,
        post_empty(
            &app,
            &manager_a,
            org_a,
            &format!("/api/v1/budgets/{budget_b}/submit?{qa}"),
        )
        .await,
        delete(
            &app,
            &manager_a,
            org_a,
            &format!("/api/v1/budgets/{budget_b}?{qa}"),
        )
        .await,
        add_item(&app, &manager_a, org_a, budget_b, category_b).await,
        put(
            &app,
            &manager_a,
            org_a,
            &format!("/api/v1/budgets/items/{item_b}"),
            json!({ "name": "Hijacked" }),
        )
        .await,
        delete(
            &app,
            &manager_a,
            org_a,
            &format!("/api/v1/budgets/items/{item_b}"),
        )
        .await,
        post(
            &app,
            &manager_a,
            org_a,
            &format!("/api/v1/budgets/items/{item_b}/actuals"),
            actual_body(),
        )
        .await,
    ];
    for (i, resp) in not_found.iter().enumerate() {
        assert_eq!(resp.status, StatusCode::NOT_FOUND, "#{i}: {}", resp.text());
    }

    // Manager of A naming org B explicitly while acting in A → 403.
    let qb = format!("organization_id={org_b}");
    let resp = get(&app, &manager_a, org_a, &format!("/api/v1/budgets?{qb}")).await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let resp = post_empty(
        &app,
        &manager_a,
        org_a,
        &format!("/api/v1/budgets/{budget_b}/submit?{qb}"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let resp = post(
        &app,
        &manager_a,
        org_a,
        "/api/v1/budgets",
        json!({ "organization_id": org_b, "fiscal_year": 2027, "name": "Planted" }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    // A's own draft budget cannot reference B's category.
    let budget_a = create_budget(&app, &manager_a, org_a, 2026).await;
    let resp = add_item(&app, &manager_a, org_a, budget_a, category_b).await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());

    // B's data is untouched.
    let (status, name): (String, String) = sqlx::query_as(
        "SELECT b.status, bi.name FROM budgets b \
         JOIN budget_items bi ON bi.budget_id = b.id WHERE bi.id = $1",
    )
    .bind(item_b)
    .fetch_one(&app.pool)
    .await
    .expect("item row");
    assert_eq!(status, "draft");
    assert_eq!(name, "Line");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn status_machine_and_self_approval_are_enforced(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&app.pool, "budauthz-sm").await;
    let author = member(&app, org_id, "manager").await;
    let approver = member(&app, org_id, "org_admin").await;
    let q = format!("organization_id={org_id}");

    let category_id = create_category(&app, &author, org_id, "Maintenance").await;
    let budget_id = create_budget(&app, &author, org_id, 2026).await;

    // Unknown category → 400 (was a 500 FK violation).
    let resp = add_item(&app, &author, org_id, budget_id, Uuid::new_v4()).await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());

    // Draft: lines editable, actuals rejected.
    let resp = add_item(&app, &author, org_id, budget_id, category_id).await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let item_id = id_of(&resp);
    let resp = add_item(&app, &author, org_id, budget_id, category_id).await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let spare_item = id_of(&resp);
    let resp = put(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/items/{item_id}"),
        json!({ "budgeted_amount": "250.00" }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    let resp = delete(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/items/{spare_item}"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "{}", resp.text());
    let resp = post(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/items/{item_id}/actuals"),
        actual_body(),
    )
    .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    assert_eq!(resp.json_value()["code"], "INVALID_STATE");

    // Submit → pending_approval: lines frozen.
    let resp = post_empty(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/{budget_id}/submit?{q}"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    let resp = add_item(&app, &author, org_id, budget_id, category_id).await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    assert_eq!(resp.json_value()["code"], "INVALID_STATE");
    let resp = put(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/items/{item_id}"),
        json!({ "budgeted_amount": "1.00" }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    let resp = delete(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/items/{item_id}"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());

    // Creator cannot approve their own budget.
    let resp = post_empty(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/{budget_id}/approve?{q}"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());

    // A different manager approves; then activate.
    let resp = post_empty(
        &app,
        &approver,
        org_id,
        &format!("/api/v1/budgets/{budget_id}/approve?{q}"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    assert_eq!(resp.json_value()["approved_by"], json!(approver.id));
    // Approved but not yet active: actuals still rejected.
    let resp = post(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/items/{item_id}/actuals"),
        actual_body(),
    )
    .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());
    let resp = post_empty(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/{budget_id}/activate?{q}"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());

    // Active: actuals accepted, lines still frozen.
    let resp = post(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/items/{item_id}/actuals"),
        actual_body(),
    )
    .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let resp = put(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/items/{item_id}"),
        json!({ "budgeted_amount": "1.00" }),
    )
    .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());

    // Closed: actuals rejected again.
    let resp = post_empty(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/{budget_id}/close?{q}"),
    )
    .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    let resp = post(
        &app,
        &author,
        org_id,
        &format!("/api/v1/budgets/items/{item_id}/actuals"),
        actual_body(),
    )
    .await;
    assert_eq!(resp.status, StatusCode::BAD_REQUEST, "{}", resp.text());

    let actuals: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM budget_actuals WHERE budget_item_id = $1")
            .bind(item_id)
            .fetch_one(&app.pool)
            .await
            .expect("count actuals");
    assert_eq!(actuals, 1);
}
