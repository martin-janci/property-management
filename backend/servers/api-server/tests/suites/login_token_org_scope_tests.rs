//! Real-login org resolution for the analytics / finance / migration route
//! groups (round-3 #2 follow-up, PR #3282).
//!
//! These handlers used to read `AuthUser.tenant_id` (and migration
//! `AuthUser.role`) straight from the JWT. Login access tokens carry neither
//! claim, so every call from the app was rejected ("No organization context")
//! or, for owner-analytics' portfolio summary, ran unscoped. They now take
//! `TenantAuthUser`, which resolves org + role from the caller's active
//! membership in the `X-Tenant-ID` org.
//!
//! Every test logs in through `POST /api/v1/auth/login` like the app does.

use axum::http::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::common::{
    create_authenticated_user, seed_membership, seed_org, TestApp, TestResponse, TestUser,
};

async fn login(app: &TestApp) -> (String, Uuid) {
    let user = TestUser::new();
    let (token, _refresh) = create_authenticated_user(app, &user).await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");
    (token, user_id)
}

async fn get(app: &TestApp, token: &str, org: Option<Uuid>, uri: &str) -> TestResponse {
    let mut builder = app.get(uri).bearer(token);
    if let Some(org) = org {
        builder = builder.tenant(org);
    }
    app.execute(builder.build()).await
}

/// One read endpoint per module group touched by the fix. Multi-currency is
/// not listed: it now uses `ValidatedTenantExtractor` (#3285), which answers a
/// missing `X-Tenant-ID` with 400, and its cross-org/role behaviour is pinned
/// by `multi_currency_cross_org_idor_tests` and the explicit checks in
/// `login_token_without_membership_is_forbidden`.
const ENDPOINTS: &[&str] = &[
    "/api/v1/market-pricing/regions",
    "/api/v1/portfolio-analytics/benchmarks",
    "/api/v1/owner-analytics/expense-rules",
    "/api/v1/owner-analytics/portfolio",
    "/api/v1/lease-abstraction/documents",
    "/api/v1/portfolio-performance/portfolios",
    "/api/v1/property-valuations/models",
    "/api/v1/investor-portal/investors",
];

/// A manager's login token plus `X-Tenant-ID` reaches every group (used to be
/// 400/403 "No organization context").
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_login_token_reaches_every_org_scoped_group(pool: sqlx::PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "ltos-mgr").await;
    let (token, user_id) = login(&app).await;
    seed_membership(&pool, org, user_id, "manager").await;

    for uri in ENDPOINTS
        .iter()
        .copied()
        .chain(["/api/v1/multi-currency/properties"])
    {
        let resp = get(&app, &token, Some(org), uri).await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "manager GET {uri}: got {} body={}",
            resp.status,
            resp.text()
        );
    }
}

/// Without a tenant, or naming an org the caller does not belong to, every
/// group refuses with 403 instead of serving (or scoping to) anything.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_token_without_membership_is_forbidden(pool: sqlx::PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let own = seed_org(&pool, "ltos-own").await;
    let foreign = seed_org(&pool, "ltos-foreign").await;
    let (token, user_id) = login(&app).await;
    seed_membership(&pool, own, user_id, "manager").await;

    for uri in ENDPOINTS {
        let none = get(&app, &token, None, uri).await;
        assert_eq!(
            none.status,
            StatusCode::FORBIDDEN,
            "no tenant GET {uri}: got {} body={}",
            none.status,
            none.text()
        );
        let other = get(&app, &token, Some(foreign), uri).await;
        assert_eq!(
            other.status,
            StatusCode::FORBIDDEN,
            "foreign tenant GET {uri}: got {} body={}",
            other.status,
            other.text()
        );
    }

    // Multi-currency (ValidatedTenantExtractor): no tenant is a 400, a foreign
    // org the caller is not a member of is still refused with 403.
    let mc = "/api/v1/multi-currency/properties";
    let none = get(&app, &token, None, mc).await;
    assert_eq!(
        none.status,
        StatusCode::BAD_REQUEST,
        "no tenant GET {mc}: got {} body={}",
        none.status,
        none.text()
    );
    let other = get(&app, &token, Some(foreign), mc).await;
    assert_eq!(
        other.status,
        StatusCode::FORBIDDEN,
        "foreign tenant GET {mc}: got {} body={}",
        other.status,
        other.text()
    );
}

/// Lists are scoped to the caller's org: a pricing region created by org A's
/// manager is invisible to org B's manager.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn login_token_lists_are_org_scoped(pool: sqlx::PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "ltos-a").await;
    let org_b = seed_org(&pool, "ltos-b").await;
    let (token_a, user_a) = login(&app).await;
    let (token_b, user_b) = login(&app).await;
    seed_membership(&pool, org_a, user_a, "manager").await;
    seed_membership(&pool, org_b, user_b, "manager").await;

    let created = app
        .execute(
            app.post("/api/v1/market-pricing/regions")
                .bearer(&token_a)
                .tenant(org_a)
                .json(json!({ "name": "Region A", "country_code": "SK", "city": "Bratislava" }))
                .build(),
        )
        .await;
    assert!(
        created.status.is_success(),
        "manager A create region: {} {}",
        created.status,
        created.text()
    );
    let body = created.json_value();
    let region_a = body["id"]
        .as_str()
        .or_else(|| body["region"]["id"].as_str())
        .unwrap_or_else(|| panic!("region id in {body}"))
        .to_string();

    let list_a = get(
        &app,
        &token_a,
        Some(org_a),
        "/api/v1/market-pricing/regions",
    )
    .await;
    assert_eq!(list_a.status, StatusCode::OK, "{}", list_a.text());
    assert!(list_a.text().contains(&region_a), "{}", list_a.text());

    let list_b = get(
        &app,
        &token_b,
        Some(org_b),
        "/api/v1/market-pricing/regions",
    )
    .await;
    assert_eq!(list_b.status, StatusCode::OK, "{}", list_b.text());
    assert!(!list_b.text().contains(&region_a), "{}", list_b.text());

    let get_b = get(
        &app,
        &token_b,
        Some(org_b),
        &format!("/api/v1/market-pricing/regions/{region_a}"),
    )
    .await;
    assert_eq!(get_b.status, StatusCode::NOT_FOUND, "{}", get_b.text());
}

/// Migration is platform-admin only. With a login token the org now resolves
/// from the membership, so a manager is refused at the role gate (not the org
/// gate). A `super_admin` membership row must NOT unlock it: org admins can
/// write any `role_type` on their own members, so the platform tier is never
/// taken from a membership (`TenantAuthUser` caps it at `OrgAdmin`).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn migration_role_gate_ignores_membership_platform_tier(pool: sqlx::PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "ltos-mig").await;
    let (mgr_token, mgr) = login(&app).await;
    let (fake_admin_token, fake_admin) = login(&app).await;
    seed_membership(&pool, org, mgr, "manager").await;
    seed_membership(&pool, org, fake_admin, "super_admin").await;

    for (who, token) in [
        ("manager", &mgr_token),
        ("super_admin member", &fake_admin_token),
    ] {
        let resp = get(&app, token, Some(org), "/api/v1/migration/templates").await;
        assert_eq!(resp.status, StatusCode::FORBIDDEN, "{who}: {}", resp.text());
        assert!(
            resp.text().contains("platform administrators"),
            "{who} must hit the role gate, not the org gate: {}",
            resp.text()
        );
    }
}

/// Voice account linking binds the device to the caller's org; with a login
/// token that org now comes from the membership instead of a JWT claim.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn voice_exchange_accepts_login_token_with_membership(pool: sqlx::PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "ltos-voice").await;
    let (token, user_id) = login(&app).await;

    let body = json!({
        "auth_code": "test-authorization-code",
        "platform": "alexa",
        "redirect_uri": "https://ppt.three-two-bit.com/api/v1/webhooks/voice/oauth/callback",
        "state": null
    });
    let send = |org: Option<Uuid>| {
        let mut b = app
            .post("/api/v1/webhooks/voice/oauth/exchange")
            .bearer(&token)
            .json(body.clone());
        if let Some(org) = org {
            b = b.tenant(org);
        }
        b.build()
    };

    let not_member = app.execute(send(Some(org))).await;
    assert_eq!(
        not_member.status,
        StatusCode::FORBIDDEN,
        "{}",
        not_member.text()
    );

    seed_membership(&pool, org, user_id, "owner").await;
    let member = app.execute(send(Some(org))).await;
    assert_ne!(member.status, StatusCode::UNAUTHORIZED, "{}", member.text());
    assert_ne!(member.status, StatusCode::FORBIDDEN, "{}", member.text());
}

/// AML/DSA moderation (manager tier) resolves org + role from the membership:
/// a manager's login token reaches the queue, a tenant's is refused.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn moderation_queue_role_comes_from_membership(pool: sqlx::PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "ltos-mod").await;
    let (mgr_token, mgr) = login(&app).await;
    let (tenant_token, tenant) = login(&app).await;
    seed_membership(&pool, org, mgr, "manager").await;
    seed_membership(&pool, org, tenant, "tenant").await;

    let ok = get(
        &app,
        &mgr_token,
        Some(org),
        "/api/v1/aml-dsa/moderation/queue",
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", ok.text());

    let denied = get(
        &app,
        &tenant_token,
        Some(org),
        "/api/v1/aml-dsa/moderation/queue",
    )
    .await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN, "{}", denied.text());
}
