//! Cross-org and role tests for the analytics / valuation / investor modules
//! (round-3 audit-4 findings, PR #3282), using real login tokens.
//!
//! Each module group proves:
//! - another org's ids (nested or body-supplied) are 404 and nothing changes;
//! - mutations need a manager-class membership role (a tenant gets 403);
//! - the caller's own manager still gets through.

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
    create_authenticated_user, seed_membership, seed_org, TestApp, TestResponse, TestUser,
};

struct Member {
    token: String,
    org: Uuid,
}

async fn login_member(app: &TestApp, org: Uuid, role: &str) -> Member {
    let user = TestUser::new();
    let (token, _refresh) = create_authenticated_user(app, &user).await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");
    seed_membership(&app.pool, org, user_id, role).await;
    Member { token, org }
}

async fn call(
    app: &TestApp,
    m: &Member,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> TestResponse {
    let b = match method {
        "GET" => app.get(uri),
        "POST" => app.post(uri),
        "PUT" => app.put(uri),
        "DELETE" => app.delete(uri),
        other => panic!("method {other}"),
    }
    .bearer(&m.token)
    .tenant(m.org);
    let b = match body {
        Some(v) => b.json(v),
        None => b,
    };
    app.execute(b.build()).await
}

fn expect(resp: &TestResponse, status: StatusCode, what: &str) {
    assert_eq!(
        resp.status,
        status,
        "{what}: expected {status}, got {} body={}",
        resp.status,
        resp.text()
    );
}

async fn seed_building(pool: &PgPool, org: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Authz Street 1', 'Bratislava', '81101', 'SK') RETURNING id"#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation, floor, unit_type)
           VALUES ($1, '1A', 1, 'apartment') RETURNING id"#,
    )
    .bind(building)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

/// A tenant is refused on a manager-only mutation of every module group; the
/// manager reaches the handler (404 for an unknown id, not 403).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn mutations_require_manager_role(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "authz-role").await;
    let manager = login_member(&app, org, "manager").await;
    let tenant = login_member(&app, org, "tenant").await;
    let id = Uuid::new_v4();

    let routes = [
        format!("/api/v1/property-valuations/models/{id}"),
        format!("/api/v1/property-valuations/adjustments/{id}"),
        format!("/api/v1/portfolio-performance/portfolios/{id}"),
        format!("/api/v1/portfolio-analytics/benchmarks/{id}"),
        format!("/api/v1/market-pricing/regions/{id}"),
        format!("/api/v1/investor-portal/investors/{id}"),
        format!("/api/v1/lease-abstraction/documents/{id}"),
    ];
    for uri in &routes {
        let denied = call(&app, &tenant, "DELETE", uri, None).await;
        expect(
            &denied,
            StatusCode::FORBIDDEN,
            &format!("tenant DELETE {uri}"),
        );
        let allowed = call(&app, &manager, "DELETE", uri, None).await;
        assert_ne!(
            allowed.status,
            StatusCode::FORBIDDEN,
            "manager DELETE {uri} must pass the role gate: {}",
            allowed.text()
        );
        expect(
            &allowed,
            StatusCode::NOT_FOUND,
            &format!("manager DELETE unknown {uri}"),
        );
    }

    // Owner analytics: reviewing an expense is manager-only.
    let review = call(
        &app,
        &tenant,
        "POST",
        &format!("/api/v1/owner-analytics/expenses/{id}/review"),
        Some(json!({ "decision": "approved" })),
    )
    .await;
    expect(&review, StatusCode::FORBIDDEN, "tenant review expense");
}

/// Property valuation adjustments are keyed on a comparable id that used to
/// be trusted blindly: another org can neither list, add nor delete them.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn valuation_adjustments_are_org_scoped(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "authz-pv-a").await;
    let org_b = seed_org(&pool, "authz-pv-b").await;
    let manager_a = login_member(&app, org_a, "manager").await;
    let manager_b = login_member(&app, org_b, "manager").await;

    let comparable = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO valuation_comparables (organization_id, sale_date, sale_price)
           VALUES ($1, CURRENT_DATE, 100000) RETURNING id"#,
    )
    .bind(org_a)
    .fetch_one(&pool)
    .await
    .expect("seed comparable");
    let adjustment = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO comparable_adjustments
               (comparable_id, adjustment_type, adjustment_name, adjustment_amount)
           VALUES ($1, 'size', 'Bigger', 1000) RETURNING id"#,
    )
    .bind(comparable)
    .fetch_one(&pool)
    .await
    .expect("seed adjustment");

    let list_uri = format!("/api/v1/property-valuations/comparables/{comparable}/adjustments");
    let body = json!({
        "comparable_id": comparable,
        "adjustment_type": "size",
        "adjustment_name": "Injected",
        "adjustment_amount": 5.0
    });

    expect(
        &call(&app, &manager_b, "GET", &list_uri, None).await,
        StatusCode::NOT_FOUND,
        "B lists A's adjustments",
    );
    expect(
        &call(&app, &manager_b, "POST", &list_uri, Some(body.clone())).await,
        StatusCode::NOT_FOUND,
        "B adds an adjustment to A's comparable",
    );
    expect(
        &call(
            &app,
            &manager_b,
            "DELETE",
            &format!("/api/v1/property-valuations/adjustments/{adjustment}"),
            None,
        )
        .await,
        StatusCode::NOT_FOUND,
        "B deletes A's adjustment",
    );

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM comparable_adjustments WHERE comparable_id = $1")
            .bind(comparable)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(count, 1, "B must not have added or deleted anything");

    let own = call(&app, &manager_a, "GET", &list_uri, None).await;
    expect(&own, StatusCode::OK, "A lists own adjustments");
    assert_eq!(
        own.json_value().as_array().map(Vec::len),
        Some(1),
        "{}",
        own.text()
    );
    expect(
        &call(&app, &manager_a, "POST", &list_uri, Some(body)).await,
        StatusCode::CREATED,
        "A adds an adjustment",
    );
}

/// Valuation child rows (comparables, reports) cannot be attached to another
/// org's valuation, and a valuation cannot point at another org's unit.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn valuation_children_reject_foreign_parents(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "authz-pvc-a").await;
    let org_b = seed_org(&pool, "authz-pvc-b").await;
    let manager_b = login_member(&app, org_b, "manager").await;
    let unit_a = seed_unit(&pool, seed_building(&pool, org_a).await).await;

    let valuation_a = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO avm_property_valuations (organization_id, property_id, valuation_date,
                                                effective_date, estimated_value)
           VALUES ($1, $2, CURRENT_DATE, CURRENT_DATE, 100000) RETURNING id"#,
    )
    .bind(org_a)
    .bind(unit_a)
    .fetch_one(&pool)
    .await
    .expect("seed valuation");
    for (uri, body) in [
        (
            format!("/api/v1/property-valuations/{valuation_a}/comparables"),
            json!({ "sale_date": "2026-01-01", "sale_price": 90000.0 }),
        ),
        (
            format!("/api/v1/property-valuations/{valuation_a}/reports"),
            json!({ "valuation_id": valuation_a }),
        ),
    ] {
        let resp = call(&app, &manager_b, "POST", &uri, Some(body)).await;
        assert!(
            resp.status == StatusCode::NOT_FOUND,
            "B attaches a child to A's valuation via {uri}: {} {}",
            resp.status,
            resp.text()
        );
    }

    // Creating a valuation for A's unit from org B.
    let resp = call(
        &app,
        &manager_b,
        "POST",
        "/api/v1/property-valuations",
        Some(json!({
            "property_id": unit_a,
            "valuation_date": "2026-01-01",
            "effective_date": "2026-01-01",
            "estimated_value": 100000.0
        })),
    )
    .await;
    expect(&resp, StatusCode::NOT_FOUND, "B values A's unit");
    let leaked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM avm_property_valuations WHERE organization_id = $1",
    )
    .bind(org_b)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(leaked, 0, "no valuation of A's unit may be created in B");
}

/// Portfolio sub-resources were keyed on the path portfolio id alone.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn portfolio_performance_sub_resources_are_org_scoped(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "authz-pp-a").await;
    let org_b = seed_org(&pool, "authz-pp-b").await;
    let manager_a = login_member(&app, org_a, "manager").await;
    let manager_b = login_member(&app, org_b, "manager").await;
    let building_b = seed_building(&pool, org_b).await;

    let created = call(
        &app,
        &manager_a,
        "POST",
        "/api/v1/portfolio-performance/portfolios",
        Some(json!({ "name": "A fund" })),
    )
    .await;
    expect(&created, StatusCode::OK, "A creates portfolio");
    let portfolio = created.json_value()["id"].as_str().expect("id").to_string();

    for sub in ["properties", "transactions", "alerts", "comparisons"] {
        let uri = format!("/api/v1/portfolio-performance/portfolios/{portfolio}/{sub}");
        expect(
            &call(&app, &manager_b, "GET", &uri, None).await,
            StatusCode::NOT_FOUND,
            &format!("B GET {sub}"),
        );
        expect(
            &call(&app, &manager_a, "GET", &uri, None).await,
            StatusCode::OK,
            &format!("A GET {sub}"),
        );
    }

    // B cannot add a property to A's portfolio, and A cannot add B's building.
    let property = json!({
        "building_id": building_b,
        "acquisition_date": "2025-01-01",
        "acquisition_price": "100000"
    });
    expect(
        &call(
            &app,
            &manager_b,
            "POST",
            &format!("/api/v1/portfolio-performance/portfolios/{portfolio}/properties"),
            Some(property.clone()),
        )
        .await,
        StatusCode::NOT_FOUND,
        "B adds a property to A's portfolio",
    );
    expect(
        &call(
            &app,
            &manager_a,
            "POST",
            &format!("/api/v1/portfolio-performance/portfolios/{portfolio}/properties"),
            Some(property),
        )
        .await,
        StatusCode::NOT_FOUND,
        "A adds B's building to its portfolio",
    );
}

/// Body-supplied buildings (analytics) and query-supplied regions (pricing
/// dashboard) must belong to the caller's org.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn analytics_and_pricing_reject_foreign_body_ids(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "authz-pa-a").await;
    let org_b = seed_org(&pool, "authz-pa-b").await;
    let manager_a = login_member(&app, org_a, "manager").await;
    let manager_b = login_member(&app, org_b, "manager").await;
    let building_a = seed_building(&pool, org_a).await;

    let metrics = |building: Uuid| {
        json!({
            "building_id": building,
            "period_start": "2026-01-01",
            "period_end": "2026-01-31",
            "total_units": 10,
            "occupied_units": 9,
            "gross_rental_income": "9000"
        })
    };
    expect(
        &call(
            &app,
            &manager_b,
            "POST",
            "/api/v1/portfolio-analytics/properties/metrics",
            Some(metrics(building_a)),
        )
        .await,
        // Repo-level org guard (#2946) answers 400; pinned in
        // portfolio_analytics_cross_org_idor_tests as well.
        StatusCode::BAD_REQUEST,
        "B writes metrics for A's building",
    );
    let written: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM property_performance_metrics WHERE building_id = $1",
    )
    .bind(building_a)
    .fetch_one(&pool)
    .await
    .expect("count metrics");
    assert_eq!(written, 0, "no metrics row may be written for A's building");

    let rule = |building: Uuid| {
        json!({
            "name": "Low occupancy",
            "metric_name": "occupancy_rate",
            "category": "occupancy",
            "threshold_value": "80",
            "building_id": building
        })
    };
    expect(
        &call(
            &app,
            &manager_b,
            "POST",
            "/api/v1/portfolio-analytics/alerts/rules",
            Some(rule(building_a)),
        )
        .await,
        StatusCode::NOT_FOUND,
        "B creates an alert rule scoped to A's building",
    );
    let rules: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM portfolio_alert_rules WHERE organization_id = ANY($1)",
    )
    .bind(vec![org_a, org_b])
    .fetch_one(&pool)
    .await
    .expect("count alert rules");
    assert_eq!(rules, 0, "no alert rule may be written for A's building");
    // No same-org success case: the repository INSERT targets columns
    // (category, scope, building_id, ...) that `portfolio_alert_rules` does not
    // have, so rule creation currently fails for every caller. Tracked
    // separately; the org check above runs before the repository call.

    let region = call(
        &app,
        &manager_a,
        "POST",
        "/api/v1/market-pricing/regions",
        Some(json!({ "name": "A region", "country_code": "SK", "city": "Bratislava" })),
    )
    .await;
    assert!(
        region.status.is_success(),
        "{} {}",
        region.status,
        region.text()
    );
    let region_id = region.json_value()["id"]
        .as_str()
        .expect("region id")
        .to_string();

    expect(
        &call(
            &app,
            &manager_b,
            "GET",
            &format!("/api/v1/market-pricing/dashboard?region_id={region_id}"),
            None,
        )
        .await,
        StatusCode::NOT_FOUND,
        "B reads A's region trends via the dashboard",
    );
    expect(
        &call(
            &app,
            &manager_a,
            "GET",
            &format!("/api/v1/market-pricing/dashboard?region_id={region_id}"),
            None,
        )
        .await,
        StatusCode::OK,
        "A reads own dashboard",
    );
}

/// Owner-analytics portfolio summary used to run unscoped (every org's units)
/// whenever the JWT had no tenant claim, i.e. for every real login.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn owner_portfolio_summary_is_org_scoped(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "authz-oa-a").await;
    let org_b = seed_org(&pool, "authz-oa-b").await;
    let manager_a = login_member(&app, org_a, "manager").await;
    let manager_b = login_member(&app, org_b, "manager").await;
    let unit_a = seed_unit(&pool, seed_building(&pool, org_a).await).await;

    let a = call(
        &app,
        &manager_a,
        "GET",
        "/api/v1/owner-analytics/portfolio",
        None,
    )
    .await;
    expect(&a, StatusCode::OK, "A portfolio");
    assert!(a.text().contains(&unit_a.to_string()), "{}", a.text());

    let b = call(
        &app,
        &manager_b,
        "GET",
        "/api/v1/owner-analytics/portfolio",
        None,
    )
    .await;
    expect(&b, StatusCode::OK, "B portfolio");
    assert!(!b.text().contains(&unit_a.to_string()), "{}", b.text());

    // create_valuation now honours the path unit: B cannot value A's unit.
    let resp = call(
        &app,
        &manager_b,
        "POST",
        &format!("/api/v1/owner-analytics/units/{unit_a}/valuation"),
        Some(json!({
            "organization_id": org_b,
            "unit_id": Uuid::new_v4(),
            "estimated_value": "100000",
            "valuation_method": "manual"
        })),
    )
    .await;
    expect(
        &resp,
        StatusCode::BAD_REQUEST,
        "B values A's unit via owner analytics",
    );
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM property_valuations WHERE unit_id = $1")
        .bind(unit_a)
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(n, 0, "no valuation may be written for A's unit by B");
}
