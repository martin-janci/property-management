//! Real-login authorization for disputes, violations and news (round-3 #2, #10).
//!
//! The production login flow mints access tokens WITHOUT a `tenant_id` or
//! `role` claim: the org is chosen per request via `X-Tenant-ID`. Handlers that
//! read `AuthUser.tenant_id` / `AuthUser.role` straight from the JWT therefore
//! answered 403 ("No organization context", "Only managers can create
//! articles") for every real manager. They now resolve both from the caller's
//! active `organization_members` row via `TenantAuthUser`.
//!
//! Every test here logs in through `POST /api/v1/auth/login`, exactly like the
//! app, and proves:
//! - a manager can create a news article and list disputes / violations;
//! - a tenant still gets 403 on the manager-only actions;
//! - ids from another org are 404;
//! - a non-member cannot borrow another org's `X-Tenant-ID`.

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
    create_authenticated_user, seed_membership, seed_org, TestApp, TestResponse, TestUser,
};

/// A logged-in user (plain login token) with an active membership in `org`.
struct Member {
    token: String,
    user_id: Uuid,
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
    Member {
        token,
        user_id,
        org,
    }
}

async fn send(
    app: &TestApp,
    m: &Member,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> TestResponse {
    let builder = match method {
        "GET" => app.get(uri),
        "POST" => app.post(uri),
        "PUT" => app.put(uri),
        "PATCH" => app.patch(uri),
        "DELETE" => app.delete(uri),
        other => panic!("unsupported method {other}"),
    };
    let builder = builder.bearer(&m.token).tenant(m.org);
    let builder = match body {
        Some(b) => builder.json(b),
        None => builder,
    };
    app.execute(builder.build()).await
}

fn assert_status(resp: &TestResponse, expected: StatusCode, what: &str) {
    assert_eq!(
        resp.status,
        expected,
        "{what}: expected {expected}, got {} body={}",
        resp.status,
        resp.text()
    );
}

/// Guard: the login token really carries no tenant/role claim, so these tests
/// exercise the membership lookup rather than a JWT shortcut.
fn assert_login_token_has_no_tenant_or_role(token: &str) {
    use base64::Engine;
    let payload = token.split('.').nth(1).expect("jwt payload");
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .expect("decode jwt payload");
    let claims: Value = serde_json::from_slice(&bytes).expect("jwt json");
    assert!(
        claims.get("tenant_id").is_none_or(Value::is_null),
        "{claims}"
    );
    assert!(claims.get("role").is_none_or(Value::is_null), "{claims}");
}

async fn seed_dispute(pool: &PgPool, org: Uuid, filed_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO disputes (organization_id, reference_number, category, title,
                                 description, status, priority, filed_by)
           VALUES ($1, $2, 'noise', 'Dispute', 'Desc', 'under_review', 'medium', $3)
           RETURNING id"#,
    )
    .bind(org)
    .bind(format!("LTR-{}", Uuid::new_v4()))
    .bind(filed_by)
    .fetch_one(pool)
    .await
    .expect("seed dispute")
}

async fn seed_violation(pool: &PgPool, org: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO violations (organization_id, violation_number, category, title,
                                   description, occurred_at)
           VALUES ($1, $2, 'noise', 'Loud party', 'After 22:00', NOW())
           RETURNING id"#,
    )
    .bind(org)
    .bind(format!("V-{}", &Uuid::new_v4().to_string()[..8]))
    .fetch_one(pool)
    .await
    .expect("seed violation")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_with_login_token_can_create_news_article(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "ltr-news").await;
    let manager = login_member(&app, org, "manager").await;
    assert_login_token_has_no_tenant_or_role(&manager.token);

    let created = send(
        &app,
        &manager,
        "POST",
        "/api/v1/news",
        Some(json!({ "title": "Water outage", "content": "Tomorrow 8-12." })),
    )
    .await;
    assert_status(&created, StatusCode::CREATED, "manager create article");
    let body = created.json_value();
    let id = body["id"]
        .as_str()
        .or_else(|| body["article"]["id"].as_str())
        .unwrap_or_else(|| panic!("article id in {body}"))
        .to_string();

    let publish = send(
        &app,
        &manager,
        "POST",
        &format!("/api/v1/news/{id}/publish"),
        Some(json!({})),
    )
    .await;
    assert_status(&publish, StatusCode::OK, "manager publish article");

    let delete = send(
        &app,
        &manager,
        "DELETE",
        &format!("/api/v1/news/{id}"),
        None,
    )
    .await;
    assert_status(&delete, StatusCode::OK, "manager delete article");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_with_login_token_cannot_manage_news(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "ltr-news-t").await;
    let manager = login_member(&app, org, "manager").await;
    let tenant = login_member(&app, org, "tenant").await;

    let create = send(
        &app,
        &tenant,
        "POST",
        "/api/v1/news",
        Some(json!({ "title": "Nope", "content": "Nope" })),
    )
    .await;
    assert_status(&create, StatusCode::FORBIDDEN, "tenant create article");

    let created = send(
        &app,
        &manager,
        "POST",
        "/api/v1/news",
        Some(json!({ "title": "Notice", "content": "Body" })),
    )
    .await;
    assert_status(&created, StatusCode::CREATED, "manager create article");
    let body = created.json_value();
    let id = body["id"]
        .as_str()
        .or_else(|| body["article"]["id"].as_str())
        .expect("article id")
        .to_string();

    // Delete used to have no role check at all.
    let delete = send(&app, &tenant, "DELETE", &format!("/api/v1/news/{id}"), None).await;
    assert_status(&delete, StatusCode::FORBIDDEN, "tenant delete article");
    let publish = send(
        &app,
        &tenant,
        "POST",
        &format!("/api/v1/news/{id}/publish"),
        Some(json!({})),
    )
    .await;
    assert_status(&publish, StatusCode::FORBIDDEN, "tenant publish article");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn foreign_news_article_is_404_and_foreign_tenant_header_is_403(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "ltr-news-a").await;
    let org_b = seed_org(&pool, "ltr-news-b").await;
    let manager_a = login_member(&app, org_a, "manager").await;
    let manager_b = login_member(&app, org_b, "manager").await;

    let created = send(
        &app,
        &manager_a,
        "POST",
        "/api/v1/news",
        Some(json!({ "title": "A only", "content": "Body" })),
    )
    .await;
    assert_status(&created, StatusCode::CREATED, "manager A create article");
    let body = created.json_value();
    let id = body["id"]
        .as_str()
        .or_else(|| body["article"]["id"].as_str())
        .expect("article id")
        .to_string();

    for (method, uri, body) in [
        ("GET", format!("/api/v1/news/{id}"), None),
        (
            "POST",
            format!("/api/v1/news/{id}/publish"),
            Some(json!({})),
        ),
        ("DELETE", format!("/api/v1/news/{id}"), None),
    ] {
        let resp = send(&app, &manager_b, method, &uri, body).await;
        assert_status(&resp, StatusCode::NOT_FOUND, &format!("B {method} {uri}"));
    }

    // Manager B naming org A in X-Tenant-ID is not a member there.
    let spoof = Member {
        token: manager_b.token.clone(),
        user_id: manager_b.user_id,
        org: org_a,
    };
    let resp = send(&app, &spoof, "DELETE", &format!("/api/v1/news/{id}"), None).await;
    assert_status(&resp, StatusCode::FORBIDDEN, "B spoofing A's tenant header");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_with_login_token_reaches_disputes(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "ltr-disp").await;
    let manager = login_member(&app, org, "manager").await;
    let dispute = seed_dispute(&pool, org, manager.user_id).await;

    let list = send(
        &app,
        &manager,
        "GET",
        &format!("/api/v1/disputes?organization_id={org}"),
        None,
    )
    .await;
    assert_status(&list, StatusCode::OK, "manager list disputes");

    // Used to be 403 "No organization context" for every login token.
    let get = send(
        &app,
        &manager,
        "GET",
        &format!("/api/v1/disputes/{dispute}"),
        None,
    )
    .await;
    assert_status(&get, StatusCode::OK, "manager get dispute");

    // Manager-only status change (used to be 403 from the JWT role check).
    let status = send(
        &app,
        &manager,
        "PATCH",
        &format!("/api/v1/disputes/{dispute}"),
        Some(json!({ "status": "mediation" })),
    )
    .await;
    assert_status(&status, StatusCode::OK, "manager update dispute status");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_and_foreign_org_are_kept_out_of_disputes(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "ltr-disp-a").await;
    let org_b = seed_org(&pool, "ltr-disp-b").await;
    let manager_a = login_member(&app, org_a, "manager").await;
    let tenant_a = login_member(&app, org_a, "tenant").await;
    let manager_b = login_member(&app, org_b, "manager").await;
    let dispute = seed_dispute(&pool, org_a, manager_a.user_id).await;

    let status = send(
        &app,
        &tenant_a,
        "PATCH",
        &format!("/api/v1/disputes/{dispute}"),
        Some(json!({ "status": "mediation" })),
    )
    .await;
    assert_status(
        &status,
        StatusCode::FORBIDDEN,
        "tenant update dispute status",
    );

    let get = send(
        &app,
        &manager_b,
        "GET",
        &format!("/api/v1/disputes/{dispute}"),
        None,
    )
    .await;
    assert_status(&get, StatusCode::NOT_FOUND, "foreign manager get dispute");
    let status = send(
        &app,
        &manager_b,
        "PATCH",
        &format!("/api/v1/disputes/{dispute}"),
        Some(json!({ "status": "mediation" })),
    )
    .await;
    assert_status(
        &status,
        StatusCode::NOT_FOUND,
        "foreign manager update dispute",
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_with_login_token_reaches_violations(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "ltr-viol").await;
    let manager = login_member(&app, org, "manager").await;
    let violation = seed_violation(&pool, org).await;

    let list = send(&app, &manager, "GET", "/api/v1/violations", None).await;
    assert_status(&list, StatusCode::OK, "manager list violations");
    assert!(
        list.json_value()
            .as_array()
            .is_some_and(|v| v.iter().any(|row| row["id"] == json!(violation))),
        "seeded violation listed: {}",
        list.text()
    );

    let rule = send(
        &app,
        &manager,
        "POST",
        "/api/v1/violations/rules",
        Some(json!({ "rule_code": "LTR-1", "title": "Quiet hours", "category": "noise" })),
    )
    .await;
    assert_status(&rule, StatusCode::OK, "manager create rule");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn tenant_and_foreign_org_are_kept_out_of_violations(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "ltr-viol-a").await;
    let org_b = seed_org(&pool, "ltr-viol-b").await;
    let tenant_a = login_member(&app, org_a, "tenant").await;
    let manager_b = login_member(&app, org_b, "manager").await;
    let violation = seed_violation(&pool, org_a).await;

    let rule = send(
        &app,
        &tenant_a,
        "POST",
        "/api/v1/violations/rules",
        Some(json!({ "rule_code": "LTR-2", "title": "Nope", "category": "noise" })),
    )
    .await;
    assert_status(&rule, StatusCode::FORBIDDEN, "tenant create rule");

    let get = send(
        &app,
        &manager_b,
        "GET",
        &format!("/api/v1/violations/{violation}"),
        None,
    )
    .await;
    assert_status(&get, StatusCode::NOT_FOUND, "foreign manager get violation");
}
