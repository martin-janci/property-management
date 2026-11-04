//! Round-4 comms cluster: org scope + manager gates.
//!
//! Covers the communication / document / board surfaces whose handlers looked
//! rows up by id only (relying on non-FORCEd RLS that the owner-role pool
//! bypasses), trusted `TenantExtractor` (no membership check, JWT role), or had
//! no role gate on manager mutations:
//!
//! - outages: every by-id handler + list/statistics are org-filtered in SQL.
//! - signature requests: get / remind / cancel compare the request org;
//!   create / remind / cancel are manager-only.
//! - announcements: every by-id handler (crud, lifecycle, comments,
//!   attachments, read/ack) goes through `load_announcement_in_org`; update
//!   re-validates target ids; attachment delete is scoped to its announcement.
//! - board meetings: mutations are manager-only; votes are cast as the
//!   caller's own board-member row; nested user ids must be org members.
//! - documents: update/delete/move/access compare the document org; folder ids
//!   from the body must belong to the org; version reads are org-checked.
//! - templates / granular role defaults / packages: membership is validated
//!   (`ValidatedTenantExtractor`), so a foreign `X-Tenant-ID` is rejected.
//! - reports: generation is manager-only.
//! - automation: rule mutations are manager-only (DB role).
//! - news: a reply's parent comment must belong to the same article.
//!
//! The test pool is a superuser (RLS bypassed), so every cross-org assertion
//! here exercises the explicit application-layer org checks.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, RequestBuilder, TestApp, TestConfig, TestResponse};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Comms User', 'active', NOW()) RETURNING id"#,
    )
    .bind(format!("{tag}-{}@comms-scope.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Login-style access token (no `role` claim): every role decision under
/// test must come from the DB membership.
fn mint_token(user_id: Uuid, org_id: Uuid) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, "comms@test.test", "Comms User", Some(org_id), None)
        .expect("mint access token")
}

#[derive(Clone)]
struct Actor {
    user_id: Uuid,
    org_id: Uuid,
    token: String,
}

async fn actor(pool: &PgPool, org_id: Uuid, role: &str) -> Actor {
    let user_id = seed_user(pool, role).await;
    seed_membership(pool, org_id, user_id, role).await;
    Actor {
        user_id,
        org_id,
        token: mint_token(user_id, org_id),
    }
}

/// Two orgs: a manager (`org_admin`) and a `tenant` in A, a manager in B.
struct World {
    app: TestApp,
    a: Actor,
    a_tenant: Actor,
    b: Actor,
}

async fn world(pool: PgPool, tag: &str) -> World {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, &format!("{tag}-a")).await;
    let org_b = seed_org(&pool, &format!("{tag}-b")).await;
    World {
        a: actor(&pool, org_a, "org_admin").await,
        a_tenant: actor(&pool, org_a, "tenant").await,
        b: actor(&pool, org_b, "org_admin").await,
        app,
    }
}

impl World {
    fn req(&self, who: &Actor, b: RequestBuilder) -> Request<Body> {
        b.bearer(&who.token).tenant(who.org_id).build()
    }

    async fn get(&self, who: &Actor, uri: &str) -> TestResponse {
        self.app.execute(self.req(who, self.app.get(uri))).await
    }

    async fn delete(&self, who: &Actor, uri: &str) -> TestResponse {
        self.app.execute(self.req(who, self.app.delete(uri))).await
    }

    async fn post(&self, who: &Actor, uri: &str, body: Value) -> TestResponse {
        self.app
            .execute(self.req(who, self.app.post(uri).json(body)))
            .await
    }

    async fn put(&self, who: &Actor, uri: &str, body: Value) -> TestResponse {
        self.app
            .execute(self.req(who, self.app.put(uri).json(body)))
            .await
    }
}

fn id_of(resp: &TestResponse) -> Uuid {
    let v = resp.json_value();
    let raw = v["id"]
        .as_str()
        .or_else(|| v["announcement"]["id"].as_str())
        .or_else(|| v["document"]["id"].as_str())
        .or_else(|| v["signature_request"]["id"].as_str())
        .or_else(|| v["signatureRequest"]["id"].as_str())
        .unwrap_or_else(|| panic!("no id in {}", resp.text()));
    Uuid::parse_str(raw).expect("uuid")
}

fn assert_status(resp: &TestResponse, expected: StatusCode, what: &str) {
    assert_eq!(resp.status, expected, "{what}: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Outages
// ---------------------------------------------------------------------------

async fn create_outage(w: &World) -> Uuid {
    let resp = w
        .post(
            &w.a,
            "/api/v1/outages",
            json!({
                "title": "Water shutoff",
                "commodity": "water",
                "severity": "medium",
                "scheduled_start": "2026-12-01T08:00:00Z",
            }),
        )
        .await;
    assert_status(&resp, StatusCode::CREATED, "create outage");
    id_of(&resp)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn outages_cross_org_by_id_is_404(pool: PgPool) {
    let w = world(pool, "out-x").await;
    let id = create_outage(&w).await;
    let base = format!("/api/v1/outages/{id}");

    assert_status(&w.get(&w.b, &base).await, StatusCode::NOT_FOUND, "get");
    assert_status(
        &w.put(&w.b, &base, json!({"title": "pwned"})).await,
        StatusCode::NOT_FOUND,
        "update",
    );
    for action in ["start", "resolve", "cancel"] {
        assert_status(
            &w.post(&w.b, &format!("{base}/{action}"), json!({})).await,
            StatusCode::NOT_FOUND,
            action,
        );
    }
    assert_status(
        &w.delete(&w.b, &base).await,
        StatusCode::NOT_FOUND,
        "delete",
    );

    // Listing / statistics in org B never see org A's outage.
    let list = w.get(&w.b, "/api/v1/outages").await;
    assert_status(&list, StatusCode::OK, "list");
    assert_eq!(
        list.json_value()["total"].as_i64(),
        Some(0),
        "{}",
        list.text()
    );
    let stats = w.get(&w.b, "/api/v1/outages/statistics").await;
    assert_status(&stats, StatusCode::OK, "statistics");
    assert_eq!(
        stats.json_value()["statistics"]["total"].as_i64(),
        Some(0),
        "{}",
        stats.text()
    );
    let dash = w.get(&w.b, "/api/v1/outages/dashboard").await;
    assert_status(&dash, StatusCode::OK, "dashboard");
    assert_eq!(
        dash.json_value()["dashboard"]["active_count"].as_i64(),
        Some(0),
        "{}",
        dash.text()
    );

    // Org A's outage is untouched.
    let own = w.get(&w.a, &base).await;
    assert_status(&own, StatusCode::OK, "owner get");
    assert_eq!(own.json_value()["outage"]["title"], "Water shutoff");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn outages_tenant_mutation_is_403_owner_succeeds(pool: PgPool) {
    let w = world(pool, "out-r").await;
    let id = create_outage(&w).await;
    let base = format!("/api/v1/outages/{id}");

    assert_status(
        &w.put(&w.a_tenant, &base, json!({"title": "x"})).await,
        StatusCode::FORBIDDEN,
        "tenant update",
    );
    assert_status(
        &w.put(&w.a, &base, json!({"title": "Renamed"})).await,
        StatusCode::OK,
        "manager update",
    );
    assert_status(
        &w.post(&w.a, &format!("{base}/start"), json!({})).await,
        StatusCode::OK,
        "manager start",
    );
    let list = w.get(&w.a, "/api/v1/outages").await;
    assert_eq!(
        list.json_value()["total"].as_i64(),
        Some(1),
        "{}",
        list.text()
    );
    assert_status(
        &w.delete(&w.a, &base).await,
        StatusCode::NO_CONTENT,
        "delete",
    );
}

// ---------------------------------------------------------------------------
// Documents + signature requests
// ---------------------------------------------------------------------------

async fn create_document(w: &World, who: &Actor, folder_id: Option<Uuid>) -> TestResponse {
    let key = Uuid::new_v4();
    let mut body = json!({
        "title": "Lease",
        "category": "contracts",
        "file_key": format!("{}/contracts/{key}.pdf", who.org_id),
        "file_name": format!("{key}.pdf"),
        "mime_type": "application/pdf",
        "size_bytes": 4096
    });
    if let Some(f) = folder_id {
        body["folder_id"] = json!(f);
    }
    w.post(who, "/api/v1/documents", body).await
}

async fn document_in_a(w: &World) -> Uuid {
    let resp = create_document(w, &w.a, None).await;
    assert_status(&resp, StatusCode::CREATED, "create document");
    id_of(&resp)
}

async fn create_folder(w: &World, who: &Actor) -> Uuid {
    let resp = w
        .post(
            who,
            "/api/v1/documents/folders",
            json!({"name": format!("F-{}", Uuid::new_v4())}),
        )
        .await;
    assert_status(&resp, StatusCode::CREATED, "create folder");
    let v = resp.json_value();
    let raw = v["id"]
        .as_str()
        .or_else(|| v["folder"]["id"].as_str())
        .unwrap_or_else(|| panic!("no folder id: {}", resp.text()));
    Uuid::parse_str(raw).unwrap()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn document_mutations_cross_org_are_404(pool: PgPool) {
    let w = world(pool, "doc-x").await;
    let doc = document_in_a(&w).await;
    let base = format!("/api/v1/documents/{doc}");

    assert_status(
        &w.put(&w.b, &base, json!({"title": "pwned"})).await,
        StatusCode::NOT_FOUND,
        "update",
    );
    assert_status(
        &w.post(&w.b, &format!("{base}/move"), json!({"folder_id": null}))
            .await,
        StatusCode::NOT_FOUND,
        "move",
    );
    assert_status(
        &w.put(
            &w.b,
            &format!("{base}/access"),
            json!({"access_scope": "organization"}),
        )
        .await,
        StatusCode::NOT_FOUND,
        "access",
    );
    assert_status(
        &w.get(&w.b, &format!("{base}/versions")).await,
        StatusCode::NOT_FOUND,
        "version history",
    );
    assert_status(
        &w.get(&w.b, &format!("{base}/shares")).await,
        StatusCode::NOT_FOUND,
        "list shares",
    );
    assert_status(
        &w.delete(&w.b, &base).await,
        StatusCode::NOT_FOUND,
        "delete",
    );

    // Owner org: a manager who is not the creator can still manage it (the
    // role now comes from the DB membership, not the JWT).
    let other_manager = actor(&w.app.pool, w.a.org_id, "manager").await;
    assert_status(
        &w.put(&other_manager, &base, json!({"title": "Renamed"}))
            .await,
        StatusCode::OK,
        "non-creator manager update",
    );
    assert_status(
        &w.get(&other_manager, &format!("{base}/versions")).await,
        StatusCode::OK,
        "version history",
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn document_folder_ids_must_belong_to_org(pool: PgPool) {
    let w = world(pool, "doc-f").await;
    let foreign_folder = create_folder(&w, &w.b).await;
    let own_folder = create_folder(&w, &w.a).await;

    assert_status(
        &create_document(&w, &w.a, Some(foreign_folder)).await,
        StatusCode::NOT_FOUND,
        "create in foreign folder",
    );
    assert_status(
        &w.post(
            &w.a,
            "/api/v1/documents/folders",
            json!({"name": "child", "parent_id": foreign_folder}),
        )
        .await,
        StatusCode::NOT_FOUND,
        "folder under foreign parent",
    );

    let doc = document_in_a(&w).await;
    assert_status(
        &w.post(
            &w.a,
            &format!("/api/v1/documents/{doc}/move"),
            json!({"folder_id": foreign_folder}),
        )
        .await,
        StatusCode::NOT_FOUND,
        "move into foreign folder",
    );
    assert_status(
        &w.post(
            &w.a,
            &format!("/api/v1/documents/{doc}/move"),
            json!({"folder_id": own_folder}),
        )
        .await,
        StatusCode::OK,
        "move into own folder",
    );
    assert_status(
        &create_document(&w, &w.a, Some(own_folder)).await,
        StatusCode::CREATED,
        "create in own folder",
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn classification_feedback_requires_creator_or_manager(pool: PgPool) {
    let w = world(pool, "doc-cf").await;
    let doc = document_in_a(&w).await;
    let uri = format!("/api/v1/documents/{doc}/classification/feedback");
    let body = json!({"accepted": false, "correct_category": "invoices"});

    assert_status(
        &w.post(&w.a_tenant, &uri, body.clone()).await,
        StatusCode::FORBIDDEN,
        "tenant feedback",
    );
    assert_status(
        &w.post(&w.b, &uri, body.clone()).await,
        StatusCode::NOT_FOUND,
        "cross-org feedback",
    );
    assert_status(
        &w.post(&w.a, &uri, body).await,
        StatusCode::OK,
        "creator feedback",
    );
}

async fn create_signature_request(w: &World, who: &Actor, doc: Uuid) -> TestResponse {
    w.post(
        who,
        &format!("/api/v1/documents/{doc}/signature-requests"),
        json!({
            "signers": [{ "email": "signer@example.com", "name": "Sam Signer" }],
            "subject": "Please sign"
        }),
    )
    .await
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn signature_requests_org_scope_and_manager_gate(pool: PgPool) {
    let w = world(pool, "sig").await;
    let doc = document_in_a(&w).await;

    // Residents / tenants cannot start a signing workflow.
    assert_status(
        &create_signature_request(&w, &w.a_tenant, doc).await,
        StatusCode::FORBIDDEN,
        "tenant create",
    );

    let created = create_signature_request(&w, &w.a, doc).await;
    assert_status(&created, StatusCode::CREATED, "manager create");
    let id = id_of(&created);
    let base = format!("/api/v1/signature-requests/{id}");

    // Cross-org: indistinguishable from missing.
    assert_status(&w.get(&w.b, &base).await, StatusCode::NOT_FOUND, "get");
    assert_status(
        &w.post(&w.b, &format!("{base}/remind"), json!({})).await,
        StatusCode::NOT_FOUND,
        "remind",
    );
    assert_status(
        &w.post(&w.b, &format!("{base}/cancel"), json!({"reason": "x"}))
            .await,
        StatusCode::NOT_FOUND,
        "cancel",
    );
    assert_status(
        &w.get(&w.b, &format!("/api/v1/documents/{doc}/signature-requests"))
            .await,
        StatusCode::NOT_FOUND,
        "list for foreign document",
    );

    // Same org, non-manager: may read, may not cancel.
    assert_status(
        &w.post(&w.a_tenant, &format!("{base}/cancel"), json!({}))
            .await,
        StatusCode::FORBIDDEN,
        "tenant cancel",
    );

    assert_status(&w.get(&w.a, &base).await, StatusCode::OK, "owner get");
    assert_status(
        &w.post(&w.a, &format!("{base}/cancel"), json!({"reason": "done"}))
            .await,
        StatusCode::OK,
        "owner cancel",
    );
}

// ---------------------------------------------------------------------------
// Announcements
// ---------------------------------------------------------------------------

async fn create_announcement(w: &World) -> Uuid {
    let resp = w
        .post(
            &w.a,
            "/api/v1/announcements",
            json!({"title": "Hello", "content": "Body", "target_type": "all"}),
        )
        .await;
    assert_status(&resp, StatusCode::CREATED, "create announcement");
    id_of(&resp)
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Main 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn announcements_cross_org_by_id_is_404(pool: PgPool) {
    let w = world(pool, "ann-x").await;
    let id = create_announcement(&w).await;
    let base = format!("/api/v1/announcements/{id}");

    assert_status(&w.get(&w.b, &base).await, StatusCode::NOT_FOUND, "get");
    assert_status(
        &w.put(&w.b, &base, json!({"title": "pwned"})).await,
        StatusCode::NOT_FOUND,
        "update",
    );
    for action in ["publish", "archive"] {
        assert_status(
            &w.post(&w.b, &format!("{base}/{action}"), json!({})).await,
            StatusCode::NOT_FOUND,
            action,
        );
    }
    assert_status(
        &w.post(
            &w.b,
            &format!("{base}/schedule"),
            json!({"scheduled_at": "2027-01-01T00:00:00Z"}),
        )
        .await,
        StatusCode::NOT_FOUND,
        "schedule",
    );
    assert_status(
        &w.post(&w.b, &format!("{base}/pin"), json!({"pinned": false}))
            .await,
        StatusCode::NOT_FOUND,
        "pin",
    );
    assert_status(
        &w.get(&w.b, &format!("{base}/attachments")).await,
        StatusCode::NOT_FOUND,
        "list attachments",
    );
    assert_status(
        &w.post(
            &w.b,
            &format!("{base}/attachments"),
            json!({"file_key": "x", "file_name": "x.pdf", "file_type": "application/pdf", "file_size": 1}),
        )
        .await,
        StatusCode::NOT_FOUND,
        "add attachment",
    );
    assert_status(
        &w.get(&w.b, &format!("{base}/comments")).await,
        StatusCode::NOT_FOUND,
        "list comments",
    );
    assert_status(
        &w.post(&w.b, &format!("{base}/comments"), json!({"content": "hi"}))
            .await,
        StatusCode::NOT_FOUND,
        "create comment",
    );
    assert_status(
        &w.post(&w.b, &format!("{base}/read"), json!({})).await,
        StatusCode::NOT_FOUND,
        "mark read",
    );
    assert_status(
        &w.post(&w.b, &format!("{base}/acknowledge"), json!({}))
            .await,
        StatusCode::NOT_FOUND,
        "acknowledge",
    );
    assert_status(
        &w.delete(&w.b, &base).await,
        StatusCode::NOT_FOUND,
        "delete",
    );

    // Still a draft in org A.
    let own = w.get(&w.a, &base).await;
    assert_status(&own, StatusCode::OK, "owner get");
    assert_eq!(
        own.json_value()["announcement"]["title"],
        "Hello",
        "{}",
        own.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn announcements_role_gates_and_target_validation(pool: PgPool) {
    let w = world(pool, "ann-r").await;
    let id = create_announcement(&w).await;
    let base = format!("/api/v1/announcements/{id}");

    // Tenants cannot publish, and cannot read an unpublished draft.
    assert_status(
        &w.post(&w.a_tenant, &format!("{base}/publish"), json!({}))
            .await,
        StatusCode::FORBIDDEN,
        "tenant publish",
    );
    assert_status(
        &w.get(&w.a_tenant, &base).await,
        StatusCode::NOT_FOUND,
        "tenant draft get",
    );

    // Retargeting onto another org's building is rejected; own building is fine.
    let foreign_building = seed_building(&w.app.pool, w.b.org_id).await;
    let own_building = seed_building(&w.app.pool, w.a.org_id).await;
    assert_status(
        &w.put(
            &w.a,
            &base,
            json!({"target_type": "building", "target_ids": [foreign_building]}),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "foreign target",
    );
    assert_status(
        &w.put(
            &w.a,
            &base,
            json!({"target_type": "building", "target_ids": [own_building]}),
        )
        .await,
        StatusCode::OK,
        "own target",
    );
    assert_status(
        &w.put(&w.a, &base, json!({"target_type": "all", "target_ids": []}))
            .await,
        StatusCode::OK,
        "retarget all",
    );

    // Manager-only stats now work for a real (DB-role) manager.
    assert_status(
        &w.get(&w.a, "/api/v1/announcements/statistics").await,
        StatusCode::OK,
        "manager statistics",
    );
    assert_status(
        &w.get(&w.a_tenant, "/api/v1/announcements/statistics").await,
        StatusCode::FORBIDDEN,
        "tenant statistics",
    );

    assert_status(
        &w.post(&w.a, &format!("{base}/publish"), json!({})).await,
        StatusCode::OK,
        "manager publish",
    );
    assert_status(
        &w.get(&w.a_tenant, &base).await,
        StatusCode::OK,
        "tenant published get",
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn announcement_attachment_delete_is_scoped_to_parent(pool: PgPool) {
    let w = world(pool, "ann-att").await;
    let first = create_announcement(&w).await;
    let second = create_announcement(&w).await;

    let added = w
        .post(
            &w.a,
            &format!("/api/v1/announcements/{first}/attachments"),
            json!({
                "file_key": format!("{}/announcements/a.pdf", w.a.org_id),
                "file_name": "a.pdf",
                "file_type": "application/pdf",
                "file_size": 10
            }),
        )
        .await;
    assert_status(&added, StatusCode::CREATED, "add attachment");
    let attachment = id_of(&added);

    // Wrong parent (even within the same org) → 404, attachment survives.
    assert_status(
        &w.delete(
            &w.a,
            &format!("/api/v1/announcements/{second}/attachments/{attachment}"),
        )
        .await,
        StatusCode::NOT_FOUND,
        "mismatched parent",
    );
    // Foreign org → 404.
    assert_status(
        &w.delete(
            &w.b,
            &format!("/api/v1/announcements/{first}/attachments/{attachment}"),
        )
        .await,
        StatusCode::NOT_FOUND,
        "cross-org",
    );
    let listed = w
        .get(&w.a, &format!("/api/v1/announcements/{first}/attachments"))
        .await;
    assert_status(&listed, StatusCode::OK, "list");
    assert_eq!(
        listed.json_value()["attachments"].as_array().map(Vec::len),
        Some(1)
    );
    assert_status(
        &w.delete(
            &w.a,
            &format!("/api/v1/announcements/{first}/attachments/{attachment}"),
        )
        .await,
        StatusCode::NO_CONTENT,
        "owner delete",
    );
}

// ---------------------------------------------------------------------------
// Board meetings
// ---------------------------------------------------------------------------

fn meeting_body() -> Value {
    json!({
        "title": "Board",
        "meeting_type": "regular",
        "scheduled_start": "2026-12-10T18:00:00Z"
    })
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn board_meetings_manager_gate_and_votes(pool: PgPool) {
    let w = world(pool, "board").await;

    assert_status(
        &w.post(&w.a_tenant, "/api/v1/board-meetings", meeting_body())
            .await,
        StatusCode::FORBIDDEN,
        "tenant create meeting",
    );
    let meeting = w.post(&w.a, "/api/v1/board-meetings", meeting_body()).await;
    assert_status(&meeting, StatusCode::OK, "manager create meeting");
    let meeting_id = id_of(&meeting);

    // Cross-org mutation on the meeting stays 404.
    assert_status(
        &w.put(
            &w.b,
            &format!("/api/v1/board-meetings/{meeting_id}"),
            json!({"title": "x"}),
        )
        .await,
        StatusCode::NOT_FOUND,
        "cross-org update",
    );

    // A foreign user cannot be appointed to the board.
    assert_status(
        &w.post(
            &w.a,
            "/api/v1/board-meetings/members",
            json!({"user_id": w.b.user_id, "role": "member_at_large", "term_start": "2026-01-01"}),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "foreign board member",
    );
    assert_status(
        &w.post(
            &w.a_tenant,
            "/api/v1/board-meetings/members",
            json!({"user_id": w.a_tenant.user_id, "role": "member_at_large", "term_start": "2026-01-01"}),
        )
        .await,
        StatusCode::FORBIDDEN,
        "tenant appoints self",
    );

    let motion = w
        .post(
            &w.a,
            &format!("/api/v1/board-meetings/{meeting_id}/motions"),
            json!({"meeting_id": meeting_id, "title": "Paint", "motion_text": "Paint the hall"}),
        )
        .await;
    assert_status(&motion, StatusCode::OK, "create motion");
    let motion_id = id_of(&motion);
    let vote_uri = format!("/api/v1/board-meetings/motions/{motion_id}/vote");
    let vote = json!({"motion_id": motion_id, "vote": "in_favor"});

    // Not a board member → 403 (previously a placeholder id → 500).
    assert_status(
        &w.post(&w.a_tenant, &vote_uri, vote.clone()).await,
        StatusCode::FORBIDDEN,
        "non-board-member vote",
    );

    // Appoint the tenant to the board; now they vote as their own row.
    assert_status(
        &w.post(
            &w.a,
            "/api/v1/board-meetings/members",
            json!({"user_id": w.a_tenant.user_id, "role": "member_at_large", "term_start": "2026-01-01"}),
        )
        .await,
        StatusCode::OK,
        "appoint tenant",
    );
    let cast = w.post(&w.a_tenant, &vote_uri, vote).await;
    assert_status(&cast, StatusCode::OK, "board member vote");

    // A cross-org manager cannot vote on org A's motion.
    assert_status(
        &w.post(
            &w.b,
            &vote_uri,
            json!({"motion_id": motion_id, "vote": "opposed"}),
        )
        .await,
        StatusCode::NOT_FOUND,
        "cross-org vote",
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn board_nested_ids_must_belong_to_meeting(pool: PgPool) {
    let w = world(pool, "board-n").await;
    let m1 = id_of(&w.post(&w.a, "/api/v1/board-meetings", meeting_body()).await);
    let m2 = id_of(&w.post(&w.a, "/api/v1/board-meetings", meeting_body()).await);

    let agenda = w
        .post(
            &w.a,
            &format!("/api/v1/board-meetings/{m1}/agenda"),
            json!({"meeting_id": m1, "item_number": "1", "title": "Item"}),
        )
        .await;
    assert_status(&agenda, StatusCode::OK, "agenda item");
    let agenda_id = id_of(&agenda);

    assert_status(
        &w.post(
            &w.a,
            &format!("/api/v1/board-meetings/{m2}/motions"),
            json!({"meeting_id": m2, "agenda_item_id": agenda_id, "title": "M", "motion_text": "t"}),
        )
        .await,
        StatusCode::NOT_FOUND,
        "motion with other meeting's agenda item",
    );
    assert_status(
        &w.post(
            &w.a,
            &format!("/api/v1/board-meetings/{m2}/actions"),
            json!({"meeting_id": m2, "agenda_item_id": agenda_id, "title": "Do it"}),
        )
        .await,
        StatusCode::NOT_FOUND,
        "action item with other meeting's agenda item",
    );
    assert_status(
        &w.post(
            &w.a,
            &format!("/api/v1/board-meetings/{m1}/actions"),
            json!({"meeting_id": m1, "title": "Do it", "assigned_to": w.b.user_id}),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "foreign assignee",
    );
    assert_status(
        &w.post(
            &w.a,
            &format!("/api/v1/board-meetings/{m1}/attendance"),
            json!({"meeting_id": m1, "user_id": w.b.user_id, "status": "present"}),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "foreign attendee",
    );
    assert_status(
        &w.post(
            &w.a,
            &format!("/api/v1/board-meetings/{m1}/actions"),
            json!({"meeting_id": m1, "agenda_item_id": agenda_id, "title": "Do it", "assigned_to": w.a_tenant.user_id}),
        )
        .await,
        StatusCode::OK,
        "valid action item",
    );
}

// ---------------------------------------------------------------------------
// Membership-validated tenant (templates, granular role defaults, packages)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn foreign_tenant_header_is_rejected(pool: PgPool) {
    let w = world(pool, "hdr").await;
    // Org B's manager names org A in X-Tenant-ID.
    let spoof = Actor {
        org_id: w.a.org_id,
        ..w.b.clone()
    };
    for uri in [
        "/api/v1/templates",
        "/api/v1/users/me/notification-preferences/granular/roles",
        "/api/v1/packages",
        "/api/v1/visitors",
    ] {
        let resp = w.get(&spoof, uri).await;
        assert_status(&resp, StatusCode::FORBIDDEN, uri);
        // A real member passes the membership gate. (The package / visitor
        // list queries have an unrelated pre-existing failure on the test
        // schema, so only assert the gate itself there.)
        let own = w.get(&w.a, uri).await;
        assert_ne!(own.status, StatusCode::FORBIDDEN, "{uri}: {}", own.text());
        assert_ne!(own.status, StatusCode::BAD_REQUEST, "{uri}: {}", own.text());
    }
    for uri in [
        "/api/v1/templates",
        "/api/v1/users/me/notification-preferences/granular/roles",
    ] {
        assert_status(&w.get(&w.a, uri).await, StatusCode::OK, uri);
    }

    // Role-default writes now honour the DB role (manager OK, tenant 403).
    let uri = "/api/v1/users/me/notification-preferences/granular/roles/tenant";
    let body = json!({"eventPreferences": {}});
    assert_status(
        &w.put(&w.a_tenant, uri, body.clone()).await,
        StatusCode::FORBIDDEN,
        "tenant role-defaults write",
    );
    assert_status(
        &w.put(&w.a, uri, body).await,
        StatusCode::OK,
        "manager role-defaults write",
    );
}

// ---------------------------------------------------------------------------
// Reports
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reports_require_manager(pool: PgPool) {
    let w = world(pool, "rep").await;
    let org = w.a.org_id;
    for uri in [
        format!("/api/v1/reports/occupancy?organization_id={org}&year=2026"),
        format!("/api/v1/reports/consumption?organization_id={org}&from_date=2026-01-01&to_date=2026-02-01"),
        format!("/api/v1/reports/faults?organization_id={org}"),
        format!("/api/v1/reports/voting?organization_id={org}"),
    ] {
        assert_status(&w.get(&w.a_tenant, &uri).await, StatusCode::FORBIDDEN, &uri);
        assert_status(&w.get(&w.a, &uri).await, StatusCode::OK, &uri);
    }
    assert_status(
        &w.post(
            &w.a_tenant,
            "/api/v1/reports/export",
            json!({"organization_id": org, "report_type": "occupancy", "format": "csv"}),
        )
        .await,
        StatusCode::FORBIDDEN,
        "tenant export",
    );
}

// ---------------------------------------------------------------------------
// Automation
// ---------------------------------------------------------------------------

async fn seed_user_membership(pool: &PgPool, user_id: Uuid, org_id: Uuid, role: &str) {
    sqlx::query(
        r#"INSERT INTO user_memberships (user_id, organization_id, role)
           VALUES ($1, $2, $3) ON CONFLICT DO NOTHING"#,
    )
    .bind(user_id)
    .bind(org_id)
    .bind(role)
    .execute(pool)
    .await
    .expect("seed user_membership");
}

fn inject_tenant(mut req: Request<Body>, org_id: Uuid) -> Request<Body> {
    use api_core::middleware::host_tenant::{ResolvedTenant, TenantSource};
    req.extensions_mut().insert(ResolvedTenant {
        organization_id: org_id,
        source: TenantSource::Subdomain,
    });
    req
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn automation_rule_mutations_require_manager(pool: PgPool) {
    let w = world(pool.clone(), "auto").await;
    seed_user_membership(&pool, w.a.user_id, w.a.org_id, "org_admin").await;
    seed_user_membership(&pool, w.a_tenant.user_id, w.a.org_id, "tenant").await;
    seed_user_membership(&pool, w.b.user_id, w.b.org_id, "org_admin").await;

    let rule_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO workflow_automation_rules
               (organization_id, name, trigger_type, trigger_config, actions, created_by)
           VALUES ($1, 'Rule', 'schedule', '{"cron":"0 9 * * 1"}'::jsonb,
                   '[{"type":"send_notification","config":{}}]'::jsonb, $2)
           RETURNING id"#,
    )
    .bind(w.a.org_id)
    .bind(w.a.user_id)
    .fetch_one(&pool)
    .await
    .expect("seed rule");
    let toggle = format!("/api/v1/automation/rules/{rule_id}/toggle");

    let send = |who: &Actor, org: Uuid| {
        inject_tenant(
            w.app
                .post(&toggle)
                .json(json!({"is_active": false}))
                .bearer(&who.token)
                .build(),
            org,
        )
    };
    assert_status(
        &w.app.execute(send(&w.a_tenant, w.a.org_id)).await,
        StatusCode::FORBIDDEN,
        "tenant toggle",
    );
    assert_status(
        &w.app.execute(send(&w.b, w.b.org_id)).await,
        StatusCode::NOT_FOUND,
        "cross-org toggle",
    );
    assert_status(
        &w.app.execute(send(&w.a, w.a.org_id)).await,
        StatusCode::OK,
        "manager toggle",
    );
}

// ---------------------------------------------------------------------------
// News comments
// ---------------------------------------------------------------------------

async fn seed_article(pool: &PgPool, org_id: Uuid, author: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO news_articles (organization_id, author_id, title, content, status)
           VALUES ($1, $2, 'News', 'Body', 'published') RETURNING id"#,
    )
    .bind(org_id)
    .bind(author)
    .fetch_one(pool)
    .await
    .expect("seed article")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn news_reply_parent_must_be_on_same_article(pool: PgPool) {
    let w = world(pool.clone(), "news").await;
    let art1 = seed_article(&pool, w.a.org_id, w.a.user_id).await;
    let art2 = seed_article(&pool, w.a.org_id, w.a.user_id).await;

    let parent = w
        .post(
            &w.a,
            &format!("/api/v1/news/{art1}/comments"),
            json!({"content": "first"}),
        )
        .await;
    assert_status(&parent, StatusCode::CREATED, "parent comment");
    let parent_id = id_of(&parent);

    assert_status(
        &w.post(
            &w.a,
            &format!("/api/v1/news/{art2}/comments"),
            json!({"content": "reply", "parent_id": parent_id}),
        )
        .await,
        StatusCode::NOT_FOUND,
        "reply on other article",
    );
    assert_status(
        &w.post(
            &w.a,
            &format!("/api/v1/news/{art1}/comments"),
            json!({"content": "reply", "parent_id": parent_id}),
        )
        .await,
        StatusCode::CREATED,
        "reply on same article",
    );
}
