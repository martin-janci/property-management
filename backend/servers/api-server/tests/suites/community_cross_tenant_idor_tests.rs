//! Cross-tenant IDOR regression coverage for the community `community.rs`
//! write handlers (code-review: api-handlers/community cross-tenant IDOR).
//!
//! Five authenticated write handlers take a *resource* id from the URL path
//! (`group_id`, `post_id`, `event_id`, `item_id`) and used to bind it straight
//! into the repository write with no tenant check. Any authenticated user of
//! org A could POST into org B's group/post/event/item by guessing an id.
//!
//! These tests seed org B's resources directly, then drive them as an
//! authenticated org-A caller and assert **404** (resource absent OR owned by
//! another tenant — same opaque body, no existence oracle) with no row written.
//! Same-tenant happy paths are asserted alongside so the guard cannot regress
//! legitimate writes.
//!
//! Setup mirrors `community_happy_path_tests.rs`: the community handlers use
//! [`RequestPrincipal`], which needs a `ResolvedTenant` request extension
//! (via `inject_tenant`) and an active `user_memberships` row (via
//! `seed_user_membership`).

#![allow(dead_code)]

use axum::{body::Body, http::Request};
use serde_json::{json, Value};
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
           VALUES ($1, 'IDOR St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
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

async fn seed_group_member(pool: &PgPool, group_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO community_group_members (group_id, user_id, role)
           VALUES ($1, $2, 'member')
           ON CONFLICT (group_id, user_id) DO NOTHING"#,
    )
    .bind(group_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed group member");
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

async fn seed_event(pool: &PgPool, building_id: Uuid, organizer: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO community_events
               (building_id, organizer_id, title, start_time, end_time)
           VALUES ($1, $2, 'seeded event', NOW() + INTERVAL '7 days',
                   NOW() + INTERVAL '7 days 2 hours') RETURNING id"#,
    )
    .bind(building_id)
    .bind(organizer)
    .fetch_one(pool)
    .await
    .expect("seed event")
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

/// The attacker/context: an authenticated org-A caller, plus the victim org B
/// with fully-seeded community resources that org A must not be able to touch.
struct Ctx {
    app: TestApp,
    pool: PgPool,
    org_a: Uuid,
    user_a: Uuid,
    // org-A resources (legitimate — the caller owns these)
    group_a: Uuid,
    post_a: Uuid,
    token: String,
    // org-B victim resources
    group_b: Uuid,
    post_b: Uuid,
    event_b: Uuid,
    item_b: Uuid,
}

async fn setup(pool: PgPool) -> Ctx {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_a) = create_authenticated_user_with_org(&app, &user, "idor-a").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&pool)
        .await
        .expect("resolve user id");
    seed_user_membership(&pool, user_id, org_a).await;

    // Org A: a real building + group + post the caller legitimately owns.
    let building_a = seed_building(&pool, org_a).await;
    let group_a = seed_group(&pool, building_a, user_id, "own-group-a").await;
    let post_a = seed_post(&pool, group_a, user_id).await;

    // Org B (victim): a separate org with its own building + resources. The
    // seeded owner FK reuses `user_id` (FK is to users(id), not org-scoped);
    // tenant ownership is established purely through `building_id -> org B`.
    let org_b = seed_org(&pool, "idor-b").await;
    let building_b = seed_building(&pool, org_b).await;
    let group_b = seed_group(&pool, building_b, user_id, "victim-group-b").await;
    let post_b = seed_post(&pool, group_b, user_id).await;
    let event_b = seed_event(&pool, building_b, user_id).await;
    let item_b = seed_item(&pool, building_b, user_id).await;

    Ctx {
        app,
        pool,
        org_a,
        user_a: user_id,
        group_a,
        post_a,
        token,
        group_b,
        post_b,
        event_b,
        item_b,
    }
}

impl Ctx {
    /// POST as the org-A caller, with the org-A tenant context resolved.
    async fn post(&self, uri: &str, body: Value) -> TestResponse {
        let rb = self.app.post(uri).bearer(&self.token).json(body);
        let req = inject_tenant(rb.build(), self.org_a);
        self.app.execute(req).await
    }

    async fn count(&self, sql: &'static str, id: Uuid) -> i64 {
        sqlx::query_scalar::<_, i64>(sql)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .expect("count")
    }
}

fn assert_404(resp: &TestResponse, what: &str) {
    assert_eq!(
        resp.status,
        axum::http::StatusCode::NOT_FOUND,
        "{what} cross-tenant must be 404, got {}: {}",
        resp.status,
        resp.text()
    );
}

// ============================================================================
// Cross-tenant (must be 404 + no write)
// ============================================================================

/// IG3 regression: an authenticated org-A caller POSTing into org B's group
/// must get 404 and insert NO post. Fails on pre-fix `dev` (returns 201 and
/// the post row lands under org B).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_post_cross_tenant_404(pool: PgPool) {
    let ctx = setup(pool).await;

    let before = ctx
        .count(
            "SELECT COUNT(*) FROM community_posts WHERE group_id = $1",
            ctx.group_b,
        )
        .await;

    let resp = ctx
        .post(
            &format!("/api/v1/community/groups/{}/posts", ctx.group_b),
            json!({ "content": "cross-tenant intrusion" }),
        )
        .await;
    assert_404(&resp, "create_post");

    let after = ctx
        .count(
            "SELECT COUNT(*) FROM community_posts WHERE group_id = $1",
            ctx.group_b,
        )
        .await;
    assert_eq!(
        before, after,
        "no post row may be written into org B's group"
    );
}

/// IG3 regression (issue #2529, follow-up to PR #2498): an authenticated org-A
/// caller joining org B's group must get 404 and insert NO membership row.
/// Fails on pre-fix `dev` (returns 200 and a `community_group_members` row for
/// the org-A user lands under org B's group).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn join_group_cross_tenant_404(pool: PgPool) {
    let ctx = setup(pool).await;

    let resp = ctx
        .post(
            &format!("/api/v1/community/groups/{}/join", ctx.group_b),
            json!({}),
        )
        .await;
    assert_404(&resp, "join_group");

    let members = ctx
        .count(
            "SELECT COUNT(*) FROM community_group_members WHERE group_id = $1",
            ctx.group_b,
        )
        .await;
    assert_eq!(members, 0, "no membership may land on org B's group");
}

/// IG3 regression (issue #2529): an authenticated org-A caller leaving org B's
/// group must get 404 and DELETE no membership row. We seed a membership into
/// org B's group first and assert it survives the cross-tenant leave attempt.
/// Fails on pre-fix `dev` (returns 200 and deletes the seeded row).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn leave_group_cross_tenant_404(pool: PgPool) {
    let ctx = setup(pool).await;

    // A pre-existing membership on org B's group (same user id satisfies the FK;
    // tenant ownership flows through group_b -> org B).
    seed_group_member(&ctx.pool, ctx.group_b, ctx.user_a).await;

    let resp = ctx
        .post(
            &format!("/api/v1/community/groups/{}/leave", ctx.group_b),
            json!({}),
        )
        .await;
    assert_404(&resp, "leave_group");

    let members = ctx
        .count(
            "SELECT COUNT(*) FROM community_group_members WHERE group_id = $1",
            ctx.group_b,
        )
        .await;
    assert_eq!(
        members, 1,
        "org B's membership row must not be deleted cross-tenant"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_reaction_cross_tenant_404(pool: PgPool) {
    let ctx = setup(pool).await;

    let resp = ctx
        .post(
            &format!("/api/v1/community/posts/{}/reactions", ctx.post_b),
            json!({ "reaction_type": "like" }),
        )
        .await;
    assert_404(&resp, "add_reaction");

    let reactions = ctx
        .count(
            "SELECT COUNT(*) FROM community_post_reactions WHERE post_id = $1",
            ctx.post_b,
        )
        .await;
    assert_eq!(reactions, 0, "no reaction may land on org B's post");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_comment_cross_tenant_404(pool: PgPool) {
    let ctx = setup(pool).await;

    let resp = ctx
        .post(
            &format!("/api/v1/community/posts/{}/comments", ctx.post_b),
            json!({ "content": "cross-tenant comment" }),
        )
        .await;
    assert_404(&resp, "create_comment");

    let comments = ctx
        .count(
            "SELECT COUNT(*) FROM community_comments WHERE post_id = $1",
            ctx.post_b,
        )
        .await;
    assert_eq!(comments, 0, "no comment may land on org B's post");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rsvp_event_cross_tenant_404(pool: PgPool) {
    let ctx = setup(pool).await;

    let resp = ctx
        .post(
            &format!("/api/v1/community/events/{}/rsvp", ctx.event_b),
            json!({ "status": "going" }),
        )
        .await;
    assert_404(&resp, "rsvp_event");

    let rsvps = ctx
        .count(
            "SELECT COUNT(*) FROM community_event_rsvps WHERE event_id = $1",
            ctx.event_b,
        )
        .await;
    assert_eq!(rsvps, 0, "no RSVP may land on org B's event");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_inquiry_cross_tenant_404(pool: PgPool) {
    let ctx = setup(pool).await;

    let resp = ctx
        .post(
            &format!("/api/v1/community/marketplace/{}/inquiries", ctx.item_b),
            json!({ "message": "cross-tenant inquiry" }),
        )
        .await;
    assert_404(&resp, "create_inquiry");

    let inquiries = ctx
        .count(
            "SELECT COUNT(*) FROM marketplace_inquiries WHERE item_id = $1",
            ctx.item_b,
        )
        .await;
    assert_eq!(inquiries, 0, "no inquiry may land on org B's item");
}

// ============================================================================
// Same-tenant (must still succeed — guard doesn't block legitimate writes)
// ============================================================================

/// The tenant guard must not regress the happy path: a caller can still join a
/// group its own org owns (2xx + membership row written).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn join_group_same_tenant_200(pool: PgPool) {
    let ctx = setup(pool).await;

    let resp = ctx
        .post(
            &format!("/api/v1/community/groups/{}/join", ctx.group_a),
            json!({}),
        )
        .await;
    assert!(
        resp.status.is_success(),
        "same-tenant join_group should 2xx, got {}: {}",
        resp.status,
        resp.text()
    );

    let members = ctx
        .count(
            "SELECT COUNT(*) FROM community_group_members WHERE group_id = $1",
            ctx.group_a,
        )
        .await;
    assert_eq!(members, 1, "the legitimate membership must be written");
}

/// The tenant guard must not regress the happy path: a caller can still react
/// to a post its own org owns.
///
/// We assert against `add_reaction` (not `create_post`) deliberately —
/// `add_post_reaction` returns `()` (no struct decode), whereas `create_post`'s
/// `RETURNING *` decode of `CommunityPost` trips a *pre-existing* enum-vs-String
/// model bug (the reason `community_happy_path_tests` is quarantined under
/// BIT-440). Reacting keeps this no-regression check honest and independent of
/// that unrelated bug: it proves `verify_post_access` lets a same-tenant write
/// through (200), not 404/403/500.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_reaction_same_tenant_200(pool: PgPool) {
    let ctx = setup(pool).await;

    let resp = ctx
        .post(
            &format!("/api/v1/community/posts/{}/reactions", ctx.post_a),
            json!({ "reaction_type": "like" }),
        )
        .await;
    assert!(
        resp.status.is_success(),
        "same-tenant add_reaction should 2xx, got {}: {}",
        resp.status,
        resp.text()
    );

    let reactions = ctx
        .count(
            "SELECT COUNT(*) FROM community_post_reactions WHERE post_id = $1",
            ctx.post_a,
        )
        .await;
    assert_eq!(reactions, 1, "the legitimate reaction must be written");
}
