//! Route-level pagination clamp tests for `GET /api/v1/migration/templates`
//! (Epic 66, Story 66.1 — follow-up #2051 to PR #2038).
//!
//! The list handler (`routes/migration.rs::list_templates`) sanitises the
//! caller-supplied paging params before touching the database:
//!
//! ```ignore
//! let page = query.page.unwrap_or(1).max(1);
//! let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
//! let offset = (page - 1) as i64 * per_page as i64;
//! ```
//!
//! PR #2038 added this clamp but shipped without a test that actually drives
//! the route with hostile paging input. These tests close that gap: they assert
//! the clamp bounds (`per_page` -> `[1, 100]`, `page` -> `>= 1`) via the
//! `per_page` / `page` values echoed back in [`ListTemplatesResponse`], and that
//! the resulting `LIMIT`/`OFFSET` slice matches what the clamped values imply.
//!
//! These tests do NOT share the BIT-351 quarantine of `migration_db_tests.rs`:
//! that quarantine is scoped to the create/validate/preview/approve lifecycle,
//! whereas this file only seeds rows directly and reads them back through the
//! list endpoint — the same happy-path listing surface that the un-ignored
//! `infra_migration_platform_admin_tests.rs` already exercises.

#![allow(dead_code)]

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, seed_membership, seed_org, TestApp, TestUser};

/// Subset of `routes::migration::ListTemplatesResponse` needed for assertions.
/// (The route type is not exported, so mirror the wire shape here.)
#[derive(Debug, Deserialize)]
struct ListTemplatesResponse {
    templates: Vec<TemplateSummary>,
    total: i64,
    page: i32,
    per_page: i32,
}

#[derive(Debug, Deserialize)]
struct TemplateSummary {
    id: Uuid,
    name: String,
}

/// Build a GET request carrying the platform-admin bearer token and the
/// `X-Tenant-ID` header both required to reach the `list_templates` handler.
fn get(token: &str, tenant_id: Uuid, uri: &str) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", tenant_id.to_string())
        .body(Body::empty())
        .unwrap()
}

/// Mint an access token that carries the `platform_admin` role and a
/// `tenant_id` claim pinned to `org_id`.
///
/// The `list_templates` handler reads BOTH of these off the JWT *claims*:
/// `require_platform_admin` gates on `AuthUser::is_platform_admin()` (the
/// `role` claim) and the org scope comes from `user.tenant_id` (the
/// `tenant_id` claim). The `/auth/login` flow used by `create_authenticated_user`
/// mints a token with `role = None` / `tenant_id = None` ("roles are set when
/// org context is selected", see `routes/auth.rs`), so a login token can never
/// clear the platform-admin gate — it 403s before the query runs. We therefore
/// mint the token directly, mirroring the green `accounting_happy_path` pattern.
/// The signing secret matches `TestConfig::default().jwt_secret`, which
/// `TestApp` installs into `JWT_SECRET`.
///
/// `role` serialises via `TenantRole`'s `rename_all = "snake_case"`, so the
/// literal wire value is `"platform_admin"`.
fn mint_platform_admin_token(user_id: Uuid, org_id: Uuid, email: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Claims {
        sub: String,
        exp: i64,
        iat: i64,
        token_type: String,
        tenant_id: String,
        role: String,
        email: String,
        name: String,
    }

    let now = chrono::Utc::now().timestamp();
    encode(
        &Header::default(),
        &Claims {
            sub: user_id.to_string(),
            exp: now + 900,
            iat: now,
            token_type: "access".into(),
            tenant_id: org_id.to_string(),
            role: "platform_admin".into(),
            email: email.into(),
            name: "Migration Pagination Admin".into(),
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint platform-admin token")
}

/// Register + verify a user, seed a fresh org with an active `platform_admin`
/// membership for that user, then return a directly-minted platform-admin
/// access token scoped to the org (plus the org id).
///
/// The membership row is seeded to keep the fixture faithful to the real
/// authorization model (an admin *is* a member); the token's `platform_admin`
/// role claim is what actually clears `ValidatedTenantExtractor`'s membership
/// bypass and the `require_platform_admin` gate.
async fn create_platform_admin(app: &TestApp, user: &TestUser, slug: &str) -> (String, Uuid) {
    let (_login_token, _refresh) = create_authenticated_user(app, user).await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let org_id = seed_org(&app.pool, slug).await;
    seed_membership(&app.pool, org_id, user_id, "platform_admin").await;

    let token = mint_platform_admin_token(user_id, org_id, &user.email);
    (token, org_id)
}

/// Insert `count` custom `buildings` templates for `org_id` with strictly
/// increasing `updated_at` (index `i` = `NOW() + i seconds`), so the
/// `ORDER BY updated_at DESC, id DESC` list order is fully deterministic:
/// `pg-tpl-{count-1}` (newest) down to `pg-tpl-0` (oldest).
///
/// Returns the template names in that expected descending list order.
async fn seed_buildings_templates(pool: &PgPool, org_id: Uuid, count: i32) -> Vec<String> {
    for i in 0..count {
        sqlx::query(
            r#"INSERT INTO import_templates
                   (organization_id, name, description, data_type,
                    field_mappings, is_system_template, updated_at)
               VALUES ($1, $2, NULL, 'buildings', '[]'::jsonb, false,
                       NOW() + make_interval(secs => $3))"#,
        )
        .bind(org_id)
        .bind(format!("pg-tpl-{i}"))
        .bind(i as f64)
        .execute(pool)
        .await
        .expect("seed buildings template");
    }

    // Descending updated_at => highest index first.
    (0..count).rev().map(|i| format!("pg-tpl-{i}")).collect()
}

/// `per_page=0` and `per_page=-5` must both clamp up to 1.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn per_page_zero_and_negative_clamp_to_one(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_id) = create_platform_admin(&app, &TestUser::new(), "pgclamp1").await;
    seed_buildings_templates(&pool, org_id, 3).await;

    for raw in ["0", "-5"] {
        let uri = format!("/api/v1/migration/templates?include_system=false&per_page={raw}");
        let resp = app.execute(get(&token, org_id, &uri)).await;
        assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
        let body: ListTemplatesResponse = resp.json();

        assert_eq!(body.per_page, 1, "per_page={raw} must clamp to 1");
        assert_eq!(body.page, 1, "page defaults to 1");
        assert_eq!(body.total, 3, "total ignores paging window");
        assert_eq!(
            body.templates.len(),
            1,
            "per_page clamped to 1 => at most one row for per_page={raw}"
        );
    }
}

/// `per_page=500` must clamp down to 100. With only a handful of rows the
/// returned slice equals the full set, and the echoed `per_page` reflects the
/// clamp (not the requested 500).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn per_page_above_max_clamps_to_hundred(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_id) = create_platform_admin(&app, &TestUser::new(), "pgclamp2").await;
    seed_buildings_templates(&pool, org_id, 5).await;

    let uri = "/api/v1/migration/templates?include_system=false&per_page=500";
    let resp = app.execute(get(&token, org_id, uri)).await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body: ListTemplatesResponse = resp.json();

    assert_eq!(body.per_page, 100, "per_page=500 must clamp to 100");
    assert_eq!(body.total, 5);
    assert_eq!(
        body.templates.len(),
        5,
        "clamp to 100 still returns all 5 seeded rows (5 < 100)"
    );
}

/// `page=0` must clamp up to 1 (offset 0 => first page).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn page_zero_clamps_to_one(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_id) = create_platform_admin(&app, &TestUser::new(), "pgclamp3").await;
    let expected_order = seed_buildings_templates(&pool, org_id, 5).await;

    let uri = "/api/v1/migration/templates?include_system=false&page=0&per_page=2";
    let resp = app.execute(get(&token, org_id, uri)).await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body: ListTemplatesResponse = resp.json();

    assert_eq!(body.page, 1, "page=0 must clamp to 1");
    assert_eq!(body.per_page, 2);
    assert_eq!(body.total, 5);
    // Offset 0 => the first two rows in updated_at DESC, id DESC order.
    let names: Vec<&str> = body.templates.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, vec![&expected_order[0][..], &expected_order[1][..]]);
}

/// `page=2&per_page=2` selects offset `(2-1)*2 = 2` — the third and fourth
/// rows in `updated_at DESC, id DESC` order.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn page_two_returns_expected_offset_slice(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org_id) = create_platform_admin(&app, &TestUser::new(), "pgclamp4").await;
    let expected_order = seed_buildings_templates(&pool, org_id, 5).await;

    let uri = "/api/v1/migration/templates?include_system=false&page=2&per_page=2";
    let resp = app.execute(get(&token, org_id, uri)).await;
    assert_eq!(resp.status, StatusCode::OK, "body: {}", resp.text());
    let body: ListTemplatesResponse = resp.json();

    assert_eq!(body.page, 2);
    assert_eq!(body.per_page, 2);
    assert_eq!(body.total, 5);

    // offset = (page - 1) * per_page = 2 => rows at indices 2 and 3.
    let names: Vec<&str> = body.templates.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(
        names,
        vec![&expected_order[2][..], &expected_order[3][..]],
        "page 2 must be the offset-2 slice of the DESC ordering"
    );
}
