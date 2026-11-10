//! Auth + cross-tenant regression coverage for the community `community.rs`
//! **read** handlers (code-review: api-handlers/community unauthenticated reads).
//!
//! Three read handlers took no `RequestPrincipal` extractor and therefore ran
//! *unauthenticated* — the `/api/v1/community` router carries no auth layer, so
//! a handler that omits `RequestPrincipal` is reachable anonymously and leaks
//! any tenant's row by UUID:
//!   * `get_group`   — `GET /api/v1/community/groups/{id}`
//!   * `list_posts`  — `GET /api/v1/community/groups/{group_id}/posts`
//!   * `get_item`    — `GET /api/v1/community/marketplace/{id}`
//!
//! The fix adds `RequestPrincipal` (anonymous → 401) and gates each read on the
//! resource's tenant via the existing `verify_group_access` / `verify_item_access`
//! helpers (cross-tenant → 404, same opaque body as a genuine miss so the id
//! space is not an existence oracle).
//!
//! These assertions decode no resource body: 401/404 short-circuit before the
//! repository row is loaded, and the same-tenant no-regression check reads an
//! *empty* group (list_posts → `[]`). That keeps this suite independent of the
//! unrelated BIT-440 `CommunityPost`/`CommunityGroup` model-decode bug that
//! quarantines `community_happy_path_tests.rs`.
//!
//! Setup mirrors `community_cross_tenant_idor_tests.rs`: the community handlers
//! use [`RequestPrincipal`], which needs a `ResolvedTenant` request extension
//! (via `inject_tenant`) and an active `user_memberships` row.

#![allow(dead_code)]

use axum::{body::Body, http::Request};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
    create_authenticated_user_with_org, seed_org, TestApp, TestResponse, TestUser,
};

async fn seed_user_membership(pool: &PgPool, user_id: Uuid, org_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO user_memberships (user_id, organization_id, role)
           VALUES ($1, $2, 'org_admin')
           ON CONFLICT DO NOTHING"#,
    )
    .bind(user_id)
    .bind(org_id)
    .execute(pool)
    .await
    .expect("seed user_membership");
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Unauth St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Seed a group owned by `building_id`. `owner` only satisfies the `created_by`
/// FK — tenant ownership flows through `building_id`.
async fn seed_group(pool: &PgPool, building_id: Uuid, owner: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO community_groups (building_id, name, slug, created_by)
           VALUES ($1, $2, $3, $4) RETURNING id"#,
    )
    .bind(building_id)
    .bind(format!("Group {slug}"))
    .bind(slug)
    .bind(owner)
    .fetch_one(pool)
    .await
    .expect("seed group")
}

async fn seed_post(pool: &PgPool, group_id: Uuid, author: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO community_posts (group_id, author_id, content)
           VALUES ($1, $2, 'seeded post') RETURNING id"#,
    )
    .bind(group_id)
    .bind(author)
    .fetch_one(pool)
    .await
    .expect("seed post")
}

async fn seed_item(pool: &PgPool, building_id: Uuid, seller: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO marketplace_items (building_id, seller_id, title, category)
           VALUES ($1, $2, 'seeded item', 'furniture') RETURNING id"#,
    )
    .bind(building_id)
    .bind(seller)
    .fetch_one(pool)
    .await
    .expect("seed item")
}

fn inject_tenant(mut req: Request<Body>, org_id: Uuid) -> Request<Body> {
    use api_core::middleware::host_tenant::{ResolvedTenant, TenantSource};
    req.extensions_mut().insert(ResolvedTenant {
        organization_id: org_id,
        source: TenantSource::Subdomain,
    });
    req
}

/// Context: an authenticated org-A caller, plus a victim org B whose community
/// group / post / marketplace item org A must not be able to read.
struct Ctx {
    app: TestApp,
    org_a: Uuid,
    // org-A resources (legitimate — the caller owns these)
    group_a_empty: Uuid,
    token: String,
    // org-B victim resources
    group_b: Uuid,
    item_b: Uuid,
}

async fn setup(pool: PgPool) -> Ctx {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_a) = create_authenticated_user_with_org(&app, &user, "unauth-a").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("resolve user id");
    seed_user_membership(&pool, user_id, org_a).await;

    // Org A: a building + an EMPTY group the caller legitimately owns. Empty so
    // the same-tenant list_posts no-regression check returns `[]` and decodes
    // no `CommunityPost` row (see module docstring re: BIT-440).
    let building_a = seed_building(&pool, org_a).await;
    let group_a_empty = seed_group(&pool, building_a, user_id, "own-empty-group-a").await;

    // Org B (victim): a separate org with its own building + resources. The
    // seeded owner FK reuses `user_id` (FK is to users(id), not org-scoped);
    // tenant ownership is established purely through `building_id -> org B`.
    let org_b = seed_org(&pool, "unauth-b").await;
    let building_b = seed_building(&pool, org_b).await;
    let group_b = seed_group(&pool, building_b, user_id, "victim-group-b").await;
    seed_post(&pool, group_b, user_id).await;
    let item_b = seed_item(&pool, building_b, user_id).await;

    Ctx {
        app,
        org_a,
        group_a_empty,
        token,
        group_b,
        item_b,
    }
}

impl Ctx {
    /// GET as an ANONYMOUS caller — no bearer, no resolved tenant. The
    /// `RequestPrincipal` extractor must reject before any DB read (401).
    async fn get_anon(&self, uri: &str) -> TestResponse {
        let req = self.app.get(uri).build();
        self.app.execute(req).await
    }

    /// GET as the authenticated org-A caller, with the org-A tenant resolved.
    async fn get_as_a(&self, uri: &str) -> TestResponse {
        let rb = self.app.get(uri).bearer(&self.token);
        let req = inject_tenant(rb.build(), self.org_a);
        self.app.execute(req).await
    }
}

fn assert_status(resp: &TestResponse, want: axum::http::StatusCode, what: &str) {
    assert_eq!(
        resp.status,
        want,
        "{what}: expected {want}, got {}: {}",
        resp.status,
        resp.text()
    );
}

// ============================================================================
// Anonymous (must be 401 — the handler no longer runs unauthenticated)
// ============================================================================

/// IG3 primary regression: an anonymous `GET /groups/{id}` must be 401. On
/// pre-fix `dev` the handler took no `RequestPrincipal`, so this returned 200
/// with the (any-tenant) group body.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_group_anonymous_401(pool: PgPool) {
    let ctx = setup(pool).await;
    let resp = ctx
        .get_anon(&format!("/api/v1/community/groups/{}", ctx.group_b))
        .await;
    assert_status(
        &resp,
        axum::http::StatusCode::UNAUTHORIZED,
        "get_group anon",
    );
}

/// IG3 regression: an anonymous `GET /groups/{id}/posts` must be 401. Pre-fix:
/// 200 with another tenant's posts.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_posts_anonymous_401(pool: PgPool) {
    let ctx = setup(pool).await;
    let resp = ctx
        .get_anon(&format!("/api/v1/community/groups/{}/posts", ctx.group_b))
        .await;
    assert_status(
        &resp,
        axum::http::StatusCode::UNAUTHORIZED,
        "list_posts anon",
    );
}

/// IG3 regression: an anonymous `GET /marketplace/{id}` must be 401. Pre-fix:
/// 200 with another tenant's marketplace item.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_item_anonymous_401(pool: PgPool) {
    let ctx = setup(pool).await;
    let resp = ctx
        .get_anon(&format!("/api/v1/community/marketplace/{}", ctx.item_b))
        .await;
    assert_status(&resp, axum::http::StatusCode::UNAUTHORIZED, "get_item anon");
}

// ============================================================================
// Cross-tenant (authenticated org A reading org B — must be 404)
// ============================================================================

/// An authenticated org-A caller reading org B's group must get 404 (same
/// opaque body as a genuine miss — no existence oracle).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_group_cross_tenant_404(pool: PgPool) {
    let ctx = setup(pool).await;
    let resp = ctx
        .get_as_a(&format!("/api/v1/community/groups/{}", ctx.group_b))
        .await;
    assert_status(
        &resp,
        axum::http::StatusCode::NOT_FOUND,
        "get_group x-tenant",
    );
}

/// An authenticated org-A caller reading org B's group posts must get 404 —
/// no post from another tenant's group may be returned even by a guessed id.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_posts_cross_tenant_404(pool: PgPool) {
    let ctx = setup(pool).await;
    let resp = ctx
        .get_as_a(&format!("/api/v1/community/groups/{}/posts", ctx.group_b))
        .await;
    assert_status(
        &resp,
        axum::http::StatusCode::NOT_FOUND,
        "list_posts x-tenant",
    );
}

/// An authenticated org-A caller reading org B's marketplace item must get 404.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_item_cross_tenant_404(pool: PgPool) {
    let ctx = setup(pool).await;
    let resp = ctx
        .get_as_a(&format!("/api/v1/community/marketplace/{}", ctx.item_b))
        .await;
    assert_status(
        &resp,
        axum::http::StatusCode::NOT_FOUND,
        "get_item x-tenant",
    );
}

// ============================================================================
// Same-tenant (must still succeed — the auth/tenant guard doesn't block reads)
// ============================================================================

/// No-regression: a caller can still list posts of a group its own org owns.
/// The group is empty, so this proves `verify_group_access` lets a same-tenant
/// read through (200 + `[]`) without decoding a `CommunityPost` row — keeping
/// the check independent of the unrelated BIT-440 model-decode bug.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_posts_same_tenant_200(pool: PgPool) {
    let ctx = setup(pool).await;
    let resp = ctx
        .get_as_a(&format!(
            "/api/v1/community/groups/{}/posts",
            ctx.group_a_empty
        ))
        .await;
    assert_status(&resp, axum::http::StatusCode::OK, "list_posts same-tenant");
    assert_eq!(
        resp.text().trim(),
        "[]",
        "an empty same-tenant group must list zero posts"
    );
}
