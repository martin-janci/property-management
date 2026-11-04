//! Happy-path (2xx) coverage for the governance `community.rs` endpoints
//! (BIT-408 Wave 8, governance batch 3).
//!
//! The community handlers use [`RequestPrincipal`], which (unlike the
//! `RlsConnection`/`AuthUser` paths) needs two things the bare `TestApp`
//! harness does not provide:
//!   1. a `ResolvedTenant` request extension (injected via `inject_tenant`), and
//!   2. an active `user_memberships` row (seeded by `seed_user_membership`).
//!
//! This mirrors the proven `automation_workflow_batch3_tests.rs` setup.
//!
//! BIT-440: these were quarantined because every write returned 500 — the
//! enum columns (`group_type`, `visibility`, `post_type`, `event_type`, RSVP
//! `status`, `category`, `condition`) were bound as `TEXT` params (rejected by
//! Postgres: "column ... is of type marketplace_category but expression is of
//! type text") and read back into `String` via `SELECT *` (sqlx: mismatched
//! types). The repository now casts both ways; the tests below also pin the
//! response shapes the ppt-web Community UI consumes.

#![allow(dead_code)]

use crate::common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{
    create_authenticated_user, create_authenticated_user_with_org, TestApp, TestResponse, TestUser,
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
    pool: PgPool,
    org_id: Uuid,
    building_id: Uuid,
    user_id: Uuid,
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
        pool,
        org_id,
        building_id,
        user_id,
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
    fn get_as(&self, token: &str, uri: &str) -> common::RequestBuilder {
        self.app.get(uri).bearer(token)
    }
    fn post_as(&self, token: &str, uri: &str, body: Value) -> common::RequestBuilder {
        self.app.post(uri).bearer(token).json(body)
    }
    /// A second authenticated, active member of the same org. Returns
    /// `(token, user_id)`.
    async fn second_user(&self) -> (String, Uuid) {
        let user = TestUser::new();
        let (token, _refresh) = create_authenticated_user(&self.app, &user).await;
        let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
            .bind(&user.email)
            .fetch_one(&self.pool)
            .await
            .expect("resolve second user id");
        seed_user_membership(&self.pool, user_id, self.org_id).await;
        (token, user_id)
    }
    async fn create_group(&self, body: Value) -> Value {
        let bid = self.building_id;
        let resp = self
            .exec(self.post(&format!("/api/v1/community/buildings/{bid}/groups"), body))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::CREATED,
            "create_group: {}",
            resp.text()
        );
        resp.json_value()
    }
}

fn status(resp: &TestResponse, expected: StatusCode, what: &str) {
    assert_eq!(
        resp.status,
        expected,
        "{what} should be {expected}, got {}: {}",
        resp.status,
        resp.text()
    );
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
        .create_group(json!({
            "name": "Gardening Club",
            "description": "Plants",
            "group_type": "interest",
            "visibility": "public"
        }))
        .await;
    let group_id = id_of(&group);
    assert_eq!(group["group_type"], "interest");
    assert_eq!(group["visibility"], "public");
    assert_eq!(group["slug"], "gardening-club");
    // The creator is auto-enrolled as owner and counted.
    assert_eq!(group["member_count"], 1);

    // Defaults apply when the optional enum fields are omitted.
    let defaulted = ctx.create_group(json!({ "name": "Gardening Club" })).await;
    assert_eq!(defaulted["group_type"], "interest");
    assert_eq!(defaulted["visibility"], "public");
    assert_eq!(defaulted["slug"], "gardening-club-1");

    let list = ctx
        .exec(ctx.get(&format!("/api/v1/community/buildings/{bid}/groups")))
        .await;
    ok(&list, "list_groups");
    let list = list.json_value();
    let rows = list.as_array().expect("list_groups array");
    assert_eq!(rows.len(), 2);
    let row = rows
        .iter()
        .find(|r| r["group"]["id"] == group["id"])
        .expect("created group listed");
    assert_eq!(row["is_member"], true);
    assert_eq!(row["member_role"], "owner");
    assert_eq!(row["group"]["visibility"], "public");

    let detail = ctx
        .exec(ctx.get(&format!("/api/v1/community/groups/{group_id}")))
        .await;
    ok(&detail, "get_group");
    let detail = detail.json_value();
    assert_eq!(detail["id"], group["id"]);
    assert_eq!(detail["name"], "Gardening Club");
    assert_eq!(detail["is_member"], true);
    assert_eq!(detail["my_role"], "owner");

    // posts
    let post = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/groups/{group_id}/posts"),
            json!({ "content": "Welcome to the group!", "title": "Hi" }),
        ))
        .await;
    status(&post, StatusCode::CREATED, "create_post");
    let post = post.json_value();
    let post_id = id_of(&post);
    assert_eq!(post["post_type"], "text");

    let question = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/groups/{group_id}/posts"),
            json!({ "content": "Anyone have a ladder?", "post_type": "question", "is_anonymous": true }),
        ))
        .await;
    status(&question, StatusCode::CREATED, "create_post(question)");
    assert_eq!(question.json_value()["post_type"], "question");
    assert_eq!(question.json_value()["is_anonymous"], true);

    let posts = ctx
        .exec(ctx.get(&format!(
            "/api/v1/community/groups/{group_id}/posts?limit=10&offset=0"
        )))
        .await;
    ok(&posts, "list_posts");
    assert_eq!(posts.json_value().as_array().map(Vec::len), Some(2));

    ok(
        &ctx.exec(ctx.post(
            &format!("/api/v1/community/posts/{post_id}/reactions"),
            json!({ "reaction_type": "like" }),
        ))
        .await,
        "add_reaction",
    );
    // Idempotent per (post, user, type).
    ok(
        &ctx.exec(ctx.post(
            &format!("/api/v1/community/posts/{post_id}/reactions"),
            json!({ "reaction_type": "like" }),
        ))
        .await,
        "add_reaction (repeat)",
    );
    let like_count: i32 =
        sqlx::query_scalar("SELECT like_count FROM community_posts WHERE id = $1")
            .bind(post_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
    assert_eq!(like_count, 1);

    let comment = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/posts/{post_id}/comments"),
            json!({ "content": "Great idea!" }),
        ))
        .await;
    status(&comment, StatusCode::CREATED, "create_comment");
    let comment_id = id_of(&comment.json_value());
    let reply = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/posts/{post_id}/comments"),
            json!({ "content": "Agreed", "parent_id": comment_id }),
        ))
        .await;
    status(&reply, StatusCode::CREATED, "create_comment(reply)");

    // A second member joins (idempotently) and leaves again.
    let (token2, _) = ctx.second_user().await;
    for what in ["join_group", "join_group (repeat)"] {
        status(
            &ctx.exec(ctx.post_as(
                &token2,
                &format!("/api/v1/community/groups/{group_id}/join"),
                json!({}),
            ))
            .await,
            StatusCode::OK,
            what,
        );
    }
    let d2 = ctx
        .exec(ctx.get_as(&token2, &format!("/api/v1/community/groups/{group_id}")))
        .await;
    ok(&d2, "get_group as joined member");
    assert_eq!(d2.json_value()["is_member"], true);
    assert_eq!(d2.json_value()["my_role"], "member");
    assert_eq!(d2.json_value()["member_count"], 2);

    status(
        &ctx.exec(ctx.post_as(
            &token2,
            &format!("/api/v1/community/groups/{group_id}/leave"),
            json!({}),
        ))
        .await,
        StatusCode::OK,
        "leave_group",
    );
    let d2 = ctx
        .exec(ctx.get_as(&token2, &format!("/api/v1/community/groups/{group_id}")))
        .await;
    ok(&d2, "get_group after leave");
    assert_eq!(d2.json_value()["is_member"], false);
    assert_eq!(d2.json_value()["my_role"], Value::Null);
    assert_eq!(d2.json_value()["member_count"], 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn community_events_happy_path(pool: PgPool) {
    let ctx = setup(pool, "evt").await;
    let bid = ctx.building_id;
    let start = chrono::Utc::now() + chrono::Duration::days(7);
    let end = start + chrono::Duration::hours(2);

    let event = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/buildings/{bid}/events"),
            json!({
                "title": "Building Picnic",
                "start_time": start.to_rfc3339(),
                "end_time": end.to_rfc3339()
            }),
        ))
        .await;
    status(&event, StatusCode::CREATED, "create_event");
    let event = event.json_value();
    let event_id = id_of(&event);
    assert_eq!(event["event_type"], "social");
    assert_eq!(event["status"], "scheduled");

    // Linked to a group in the same building, with an explicit type + cost.
    let group = ctx.create_group(json!({ "name": "Yoga" })).await;
    let workshop = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/buildings/{bid}/events"),
            json!({
                "title": "Yoga Workshop",
                "group_id": group["id"],
                "event_type": "workshop",
                "location": "Courtyard",
                "start_time": (start + chrono::Duration::days(1)).to_rfc3339(),
                "end_time": (end + chrono::Duration::days(1)).to_rfc3339(),
                "max_attendees": 10,
                "cost_per_person": "5.50"
            }),
        ))
        .await;
    status(&workshop, StatusCode::CREATED, "create_event(workshop)");
    assert_eq!(workshop.json_value()["event_type"], "workshop");
    assert_eq!(workshop.json_value()["group_id"], group["id"]);
    assert_eq!(workshop.json_value()["cost_per_person"], "5.50");

    let list = ctx
        .exec(ctx.get(&format!(
            "/api/v1/community/buildings/{bid}/events?limit=20&offset=0"
        )))
        .await;
    ok(&list, "list_events");
    let list = list.json_value();
    let rows = list.as_array().expect("events array");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], event["id"], "ordered by start_time");
    let paged = ctx
        .exec(ctx.get(&format!(
            "/api/v1/community/buildings/{bid}/events?limit=20&offset=1"
        )))
        .await;
    ok(&paged, "list_events offset");
    assert_eq!(paged.json_value().as_array().map(Vec::len), Some(1));

    let rsvp = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/events/{event_id}/rsvp"),
            json!({ "status": "going", "guests": 2 }),
        ))
        .await;
    status(&rsvp, StatusCode::OK, "rsvp_event");
    assert_eq!(rsvp.json_value()["status"], "going");
    assert_eq!(rsvp.json_value()["guests"], 2);

    // Changing the RSVP upserts the same row.
    let rsvp2 = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/events/{event_id}/rsvp"),
            json!({ "status": "maybe" }),
        ))
        .await;
    status(&rsvp2, StatusCode::OK, "rsvp_event (update)");
    assert_eq!(rsvp2.json_value()["status"], "maybe");
    assert_eq!(rsvp2.json_value()["id"], rsvp.json_value()["id"]);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn community_marketplace_happy_path(pool: PgPool) {
    let ctx = setup(pool, "mkt").await;
    let bid = ctx.building_id;

    let item = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/buildings/{bid}/marketplace"),
            json!({ "title": "Wooden Chair", "category": "furniture", "condition": "good", "price": "25.00" }),
        ))
        .await;
    status(&item, StatusCode::CREATED, "create_item");
    let item = item.json_value();
    let item_id = id_of(&item);
    assert_eq!(item["category"], "furniture");
    assert_eq!(item["condition"], "good");
    assert_eq!(item["status"], "active");
    assert_eq!(item["price"], "25.00");

    let lamp = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/buildings/{bid}/marketplace"),
            json!({ "title": "Desk Lamp", "category": "electronics", "condition": "like_new", "is_free": true }),
        ))
        .await;
    status(&lamp, StatusCode::CREATED, "create_item(lamp)");

    let all = ctx
        .exec(ctx.get(&format!("/api/v1/community/buildings/{bid}/marketplace")))
        .await;
    ok(&all, "list_items");
    assert_eq!(all.json_value().as_array().map(Vec::len), Some(2));

    let furniture = ctx
        .exec(ctx.get(&format!(
            "/api/v1/community/buildings/{bid}/marketplace?category=furniture&limit=20&offset=0"
        )))
        .await;
    ok(&furniture, "list_items?category=furniture");
    let furniture = furniture.json_value();
    let rows = furniture.as_array().expect("items array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], item["id"]);

    let empty = ctx
        .exec(ctx.get(&format!(
            "/api/v1/community/buildings/{bid}/marketplace?category=books"
        )))
        .await;
    ok(&empty, "list_items?category=books");
    assert_eq!(empty.json_value().as_array().map(Vec::len), Some(0));

    let got = ctx
        .exec(ctx.get(&format!("/api/v1/community/marketplace/{item_id}")))
        .await;
    ok(&got, "get_item");
    assert_eq!(got.json_value()["title"], "Wooden Chair");
    assert_eq!(got.json_value()["category"], "furniture");

    let inquiry = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/marketplace/{item_id}/inquiries"),
            json!({ "message": "Is this still available?", "offer_price": "20" }),
        ))
        .await;
    status(&inquiry, StatusCode::CREATED, "create_inquiry");
    assert_eq!(inquiry.json_value()["status"], "pending");
    let count: i32 =
        sqlx::query_scalar("SELECT inquiry_count FROM marketplace_items WHERE id = $1")
            .bind(item_id)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

/// Invalid enum values are a 400, not a Postgres enum-cast 500.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn community_invalid_enum_values_400(pool: PgPool) {
    let ctx = setup(pool, "enum").await;
    let bid = ctx.building_id;
    let start = chrono::Utc::now() + chrono::Duration::days(1);

    let cases: Vec<(&str, String, Value)> = vec![
        (
            "group_type",
            format!("/api/v1/community/buildings/{bid}/groups"),
            json!({ "name": "X", "group_type": "bogus" }),
        ),
        (
            "visibility",
            format!("/api/v1/community/buildings/{bid}/groups"),
            json!({ "name": "X", "visibility": "hidden" }),
        ),
        (
            "event_type",
            format!("/api/v1/community/buildings/{bid}/events"),
            json!({ "title": "E", "event_type": "rave",
                    "start_time": start.to_rfc3339(), "end_time": start.to_rfc3339() }),
        ),
        (
            "end before start",
            format!("/api/v1/community/buildings/{bid}/events"),
            json!({ "title": "E", "start_time": start.to_rfc3339(),
                    "end_time": (start - chrono::Duration::hours(1)).to_rfc3339() }),
        ),
        (
            "category",
            format!("/api/v1/community/buildings/{bid}/marketplace"),
            json!({ "title": "T", "category": "cars", "condition": "good" }),
        ),
        (
            "condition",
            format!("/api/v1/community/buildings/{bid}/marketplace"),
            json!({ "title": "T", "category": "books", "condition": "mint" }),
        ),
    ];
    for (what, uri, body) in cases {
        status(
            &ctx.exec(ctx.post(&uri, body)).await,
            StatusCode::BAD_REQUEST,
            what,
        );
    }
    status(
        &ctx.exec(ctx.get(&format!(
            "/api/v1/community/buildings/{bid}/marketplace?category=cars"
        )))
        .await,
        StatusCode::BAD_REQUEST,
        "list_items bad category",
    );

    let group = ctx.create_group(json!({ "name": "G" })).await;
    let gid = id_of(&group);
    status(
        &ctx.exec(ctx.post(
            &format!("/api/v1/community/groups/{gid}/posts"),
            json!({ "content": "c", "post_type": "essay" }),
        ))
        .await,
        StatusCode::BAD_REQUEST,
        "post_type",
    );
    let post = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/groups/{gid}/posts"),
            json!({ "content": "c" }),
        ))
        .await;
    let post_id = id_of(&post.json_value());
    status(
        &ctx.exec(ctx.post(
            &format!("/api/v1/community/posts/{post_id}/reactions"),
            json!({ "reaction_type": "angry" }),
        ))
        .await,
        StatusCode::BAD_REQUEST,
        "reaction_type",
    );

    let event = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/buildings/{bid}/events"),
            json!({ "title": "E", "start_time": start.to_rfc3339(),
                    "end_time": (start + chrono::Duration::hours(1)).to_rfc3339() }),
        ))
        .await;
    let event_id = id_of(&event.json_value());
    status(
        &ctx.exec(ctx.post(
            &format!("/api/v1/community/events/{event_id}/rsvp"),
            json!({ "status": "attending" }),
        ))
        .await,
        StatusCode::BAD_REQUEST,
        "rsvp status",
    );
}

/// The sole owner cannot leave (the group would be orphaned); once a second
/// owner exists, leaving succeeds. A non-member leave stays an idempotent 200.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn leave_group_last_owner_409(pool: PgPool) {
    let ctx = setup(pool, "own").await;
    let group = ctx.create_group(json!({ "name": "Owners" })).await;
    let gid = id_of(&group);
    let leave = format!("/api/v1/community/groups/{gid}/leave");

    let resp = ctx.exec(ctx.post(&leave, json!({}))).await;
    status(&resp, StatusCode::CONFLICT, "last owner leave");
    assert_eq!(resp.json_value()["code"], "LAST_OWNER");
    let owners: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM community_group_members WHERE group_id = $1 AND role = 'owner'",
    )
    .bind(gid)
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(owners, 1, "owner membership must survive");

    // A non-member leaving is a no-op 200.
    let (token2, user2) = ctx.second_user().await;
    status(
        &ctx.exec(ctx.post_as(&token2, &leave, json!({}))).await,
        StatusCode::OK,
        "non-member leave",
    );

    // Promote a second owner; now the original owner may leave.
    sqlx::query(
        "INSERT INTO community_group_members (group_id, user_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(gid)
    .bind(user2)
    .execute(&ctx.pool)
    .await
    .unwrap();
    status(
        &ctx.exec(ctx.post(&leave, json!({}))).await,
        StatusCode::OK,
        "owner leave with co-owner",
    );
    let me: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM community_group_members WHERE group_id = $1 AND user_id = $2",
    )
    .bind(gid)
    .bind(ctx.user_id)
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    assert_eq!(me, 0);
    // ...and the remaining owner is now the last one.
    status(
        &ctx.exec(ctx.post_as(&token2, &leave, json!({}))).await,
        StatusCode::CONFLICT,
        "new last owner leave",
    );
}

/// Secret groups are invisible to non-members of the same org: absent from
/// the list and 404 on detail, join, and posts; members see them normally.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn secret_group_hidden_from_non_members_404(pool: PgPool) {
    let ctx = setup(pool, "sec").await;
    let bid = ctx.building_id;
    let group = ctx
        .create_group(json!({ "name": "Board", "visibility": "secret" }))
        .await;
    let gid = id_of(&group);
    assert_eq!(group["visibility"], "secret");
    let post = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/groups/{gid}/posts"),
            json!({ "content": "secret agenda" }),
        ))
        .await;
    status(&post, StatusCode::CREATED, "owner post in secret group");
    let post_id = id_of(&post.json_value());

    // Owner sees it.
    let d = ctx
        .exec(ctx.get(&format!("/api/v1/community/groups/{gid}")))
        .await;
    ok(&d, "owner get secret group");
    assert_eq!(d.json_value()["my_role"], "owner");

    let (token2, _) = ctx.second_user().await;
    let list = ctx
        .exec(ctx.get_as(
            &token2,
            &format!("/api/v1/community/buildings/{bid}/groups"),
        ))
        .await;
    ok(&list, "non-member list");
    assert_eq!(list.json_value().as_array().map(Vec::len), Some(0));

    for (what, resp) in [
        (
            "get_group",
            ctx.exec(ctx.get_as(&token2, &format!("/api/v1/community/groups/{gid}")))
                .await,
        ),
        (
            "join_group",
            ctx.exec(ctx.post_as(
                &token2,
                &format!("/api/v1/community/groups/{gid}/join"),
                json!({}),
            ))
            .await,
        ),
        (
            "list_posts",
            ctx.exec(ctx.get_as(&token2, &format!("/api/v1/community/groups/{gid}/posts")))
                .await,
        ),
        (
            "create_post",
            ctx.exec(ctx.post_as(
                &token2,
                &format!("/api/v1/community/groups/{gid}/posts"),
                json!({ "content": "let me in" }),
            ))
            .await,
        ),
        (
            "add_reaction",
            ctx.exec(ctx.post_as(
                &token2,
                &format!("/api/v1/community/posts/{post_id}/reactions"),
                json!({ "reaction_type": "like" }),
            ))
            .await,
        ),
        (
            "create_comment",
            ctx.exec(ctx.post_as(
                &token2,
                &format!("/api/v1/community/posts/{post_id}/comments"),
                json!({ "content": "hi" }),
            ))
            .await,
        ),
    ] {
        status(&resp, StatusCode::NOT_FOUND, what);
    }
    let members: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM community_group_members WHERE group_id = $1")
            .bind(gid)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
    assert_eq!(members, 1, "non-member must not join a secret group");
}

/// `?user_id=` on list_groups is ignored: membership flags always describe
/// the authenticated caller, so a caller cannot probe another user's groups.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_groups_ignores_user_id_query(pool: PgPool) {
    let ctx = setup(pool, "uid").await;
    let bid = ctx.building_id;
    ctx.create_group(json!({ "name": "Mine" })).await;
    let (token2, _) = ctx.second_user().await;

    let resp = ctx
        .exec(ctx.get_as(
            &token2,
            &format!(
                "/api/v1/community/buildings/{bid}/groups?user_id={}",
                ctx.user_id
            ),
        ))
        .await;
    ok(&resp, "list_groups?user_id=");
    let rows = resp.json_value();
    let rows = rows.as_array().expect("array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["is_member"], false);
    assert_eq!(rows[0]["member_role"], Value::Null);
}

/// An event may only link a group from the same building; a group in another
/// building (here: another org's) is rejected and no event row is written.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_event_foreign_group_400(pool: PgPool) {
    let ctx = setup(pool, "evg").await;
    let bid = ctx.building_id;
    let other_org = common::seed_org(&ctx.pool, "evg-b").await;
    let other_building = seed_building(&ctx.pool, other_org).await;
    let foreign_group = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO community_groups (building_id, name, slug) VALUES ($1, 'B', 'b') RETURNING id",
    )
    .bind(other_building)
    .fetch_one(&ctx.pool)
    .await
    .unwrap();
    let start = chrono::Utc::now() + chrono::Duration::days(1);
    status(
        &ctx.exec(ctx.post(
            &format!("/api/v1/community/buildings/{bid}/events"),
            json!({ "title": "E", "group_id": foreign_group,
                    "start_time": start.to_rfc3339(),
                    "end_time": (start + chrono::Duration::hours(1)).to_rfc3339() }),
        ))
        .await,
        StatusCode::BAD_REQUEST,
        "event with foreign group",
    );
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM community_events")
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(events, 0);
}

/// A reply's `parent_id` must be a comment on the same post.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_comment_foreign_parent_400(pool: PgPool) {
    let ctx = setup(pool, "cmt").await;
    let group = ctx.create_group(json!({ "name": "G" })).await;
    let gid = id_of(&group);
    let mut posts = Vec::new();
    for c in ["a", "b"] {
        let p = ctx
            .exec(ctx.post(
                &format!("/api/v1/community/groups/{gid}/posts"),
                json!({ "content": c }),
            ))
            .await;
        posts.push(id_of(&p.json_value()));
    }
    let c = ctx
        .exec(ctx.post(
            &format!("/api/v1/community/posts/{}/comments", posts[0]),
            json!({ "content": "on a" }),
        ))
        .await;
    let cid = id_of(&c.json_value());
    status(
        &ctx.exec(ctx.post(
            &format!("/api/v1/community/posts/{}/comments", posts[1]),
            json!({ "content": "reply", "parent_id": cid }),
        ))
        .await,
        StatusCode::BAD_REQUEST,
        "reply with parent from another post",
    );
}
