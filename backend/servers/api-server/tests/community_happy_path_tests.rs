//! Happy-path (2xx) coverage for the governance `community.rs` endpoints
//! (BIT-408 Wave 8, governance batch 3).
//!
//! The community handlers use [`RequestPrincipal`], which (unlike the
//! `RlsConnection`/`AuthUser` paths) needs two things the bare `TestApp`
//! harness does not provide:
//!   1. a `ResolvedTenant` request extension (injected via `inject_tenant`), and
//!   2. an active `user_memberships` row (seeded by `seed_user_membership`).
//! This mirrors the proven `automation_workflow_batch3_tests.rs` setup.

#![allow(dead_code)]

mod common;

use axum::{body::Body, http::Request};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user_with_org, TestApp, TestResponse, TestUser};

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
           VALUES ($1, 'Community St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

fn inject_tenant(mut req: Request<Body>, org_id: Uuid) -> Request<Body> {
    use api_core::middleware::host_tenant::{ResolvedTenant, TenantSource};
    req.extensions_mut().insert(ResolvedTenant {
        organization_id: org_id,
        source: TenantSource::Subdomain,
    });
    req
}

struct Ctx {
    app: TestApp,
    org_id: Uuid,
    building_id: Uuid,
    token: String,
}

async fn setup(pool: PgPool, slug: &str) -> Ctx {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, slug).await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("resolve user id");
    seed_user_membership(&pool, user_id, org_id).await;
    let building_id = seed_building(&pool, org_id).await;
    Ctx {
        app,
        org_id,
        building_id,
        token,
    }
}

impl Ctx {
    async fn exec(&self, rb: common::RequestBuilder) -> TestResponse {
        let req = inject_tenant(rb.build(), self.org_id);
        self.app.execute(req).await
    }
    fn get(&self, uri: &str) -> common::RequestBuilder {
        self.app.get(uri).bearer(&self.token)
    }
    fn post(&self, uri: &str, body: Value) -> common::RequestBuilder {
        self.app.post(uri).bearer(&self.token).json(body)
    }
}

fn ok(resp: &TestResponse, what: &str) {
    assert!(
        resp.status.is_success(),
        "{what} should 2xx, got {}: {}",
        resp.status,
        resp.text()
    );
}

fn id_of(v: &Value) -> Uuid {
    v.get("id")
        .and_then(|x| x.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("expected id field in {v}"))
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn community_groups_posts_happy_path(pool: PgPool) {
    let ctx = setup(pool, "grp").await;
    let bid = ctx.building_id;

    let group = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/buildings/{bid}/groups"),
            json!({ "name": "Gardening Club" }),
        ))
        .await;
    ok(&group, "create_group");
    let group_id = id_of(&group.json_value());

    ok(
        &ctx.exec(ctx.get(&format!("/api/v1/community/buildings/{bid}/groups")))
            .await,
        "list_groups",
    );
    ok(
        &ctx.exec(ctx.get(&format!("/api/v1/community/groups/{group_id}")))
            .await,
        "get_group",
    );

    // posts
    let post = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/groups/{group_id}/posts"),
            json!({ "content": "Welcome to the group!" }),
        ))
        .await;
    ok(&post, "create_post");
    let post_id = id_of(&post.json_value());

    ok(
        &ctx.exec(ctx.get(&format!("/api/v1/community/groups/{group_id}/posts")))
            .await,
        "list_posts",
    );
    ok(
        &ctx.exec(ctx.post(
            &format!("/api/v1/community/posts/{post_id}/reactions"),
            json!({ "reaction_type": "like" }),
        ))
        .await,
        "add_reaction",
    );
    ok(
        &ctx.exec(ctx.post(
            &format!("/api/v1/community/posts/{post_id}/comments"),
            json!({ "content": "Great idea!" }),
        ))
        .await,
        "create_comment",
    );

    // membership toggle (creator is already owner; join is idempotent)
    ok(
        &ctx.exec(ctx.post(&format!("/api/v1/community/groups/{group_id}/join"), json!({})))
            .await,
        "join_group",
    );
    ok(
        &ctx.exec(ctx.post(&format!("/api/v1/community/groups/{group_id}/leave"), json!({})))
            .await,
        "leave_group",
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn community_events_happy_path(pool: PgPool) {
    let ctx = setup(pool, "evt").await;
    let bid = ctx.building_id;

    let event = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/buildings/{bid}/events"),
            json!({
                "title": "Building Picnic",
                "start_time": (chrono::Utc::now() + chrono::Duration::days(7)).to_rfc3339(),
                "end_time": (chrono::Utc::now() + chrono::Duration::days(7) + chrono::Duration::hours(2)).to_rfc3339()
            }),
        ))
        .await;
    ok(&event, "create_event");
    let event_id = id_of(&event.json_value());

    ok(
        &ctx.exec(ctx.get(&format!("/api/v1/community/buildings/{bid}/events")))
            .await,
        "list_events",
    );
    ok(
        &ctx.exec(ctx.post(
            &format!("/api/v1/community/events/{event_id}/rsvp"),
            json!({ "status": "going" }),
        ))
        .await,
        "rsvp_event",
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn community_marketplace_happy_path(pool: PgPool) {
    let ctx = setup(pool, "mkt").await;
    let bid = ctx.building_id;

    let item = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/buildings/{bid}/marketplace"),
            json!({ "title": "Wooden Chair", "category": "furniture", "condition": "good" }),
        ))
        .await;
    ok(&item, "create_item");
    let item_id = id_of(&item.json_value());

    ok(
        &ctx.exec(ctx.get(&format!("/api/v1/community/buildings/{bid}/marketplace")))
            .await,
        "list_items",
    );
    ok(
        &ctx.exec(ctx.get(&format!("/api/v1/community/marketplace/{item_id}")))
            .await,
        "get_item",
    );
    ok(
        &ctx.exec(ctx.post(
            &format!("/api/v1/community/marketplace/{item_id}/inquiries"),
            json!({ "message": "Is this still available?" }),
        ))
        .await,
        "create_inquiry",
    );
}
