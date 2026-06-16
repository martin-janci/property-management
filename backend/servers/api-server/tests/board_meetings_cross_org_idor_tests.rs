//! Regression tests for the cross-tenant IDOR fix on the board-meeting
//! endpoints (`/api/v1/board-meetings/*`, Epic 143 — PAP-137).
//!
//! Audit history (PAP-136): every by-id board-meeting handler carried
//! `_user: AuthUser` and performed no org scoping, and the repository ran on
//! the raw pool with no `organization_id` predicate anywhere. A foreign caller
//! could read/mutate/delete any other org's meetings, agenda items, motions,
//! minutes, action items and documents by UUID.
//!
//! Since the PAP-137 RLS conversion, every handler acquires an
//! `RlsConnection` (tenant validated against `organization_members`) and the
//! repository is stateless: queries run on the request's RLS-scoped
//! connection AND stay org-keyed — `board_meetings`/`board_members` directly
//! on `organization_id`, child tables through the root meeting via `EXISTS`.
//! A cross-tenant probe made with the attacker's own valid tenant context
//! resolves to no row → `404`; "missing" and "forbidden" are
//! indistinguishable. The CI test pool runs as superuser (bypasses FORCE
//! RLS), so these tests specifically prove the org-keyed SQL layer.
//!
//! These tests exercise the HTTP surface end-to-end with real HS256 JWTs:
//!   1. Seed two orgs (A, B), a member user in each, and a meeting (+ agenda
//!      item / motion) in Org A.
//!   2. Org B's member probes Org A's resources → rejected (4xx); no leak,
//!      no write.
//!   3. Org A's member reads its own meeting → allowed (2xx).

#[allow(dead_code)]
mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, RequestBuilder, TestApp, TestConfig};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("BoardIDOR Org {slug}"))
    .bind(format!("board-idor-org-{slug}"))
    .bind(format!("{slug}@board-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'BoardIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// Seed a board meeting in `org_id` created by `created_by`, return its id.
async fn seed_meeting(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO board_meetings (organization_id, title, scheduled_start, created_by)
        VALUES ($1, 'Annual Budget Meeting', NOW() + INTERVAL '7 days', $2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed meeting")
}

/// Seed an agenda item on `meeting_id`, return its id.
async fn seed_agenda_item(pool: &PgPool, meeting_id: Uuid, added_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO meeting_agenda_items (meeting_id, item_number, title, added_by)
        VALUES ($1, '1', 'Reserve fund review', $2)
        RETURNING id
        "#,
    )
    .bind(meeting_id)
    .bind(added_by)
    .fetch_one(pool)
    .await
    .expect("seed agenda item")
}

/// Seed a motion on `meeting_id`, return its id.
async fn seed_motion(pool: &PgPool, meeting_id: Uuid, proposed_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO meeting_motions (meeting_id, title, motion_text, proposed_by)
        VALUES ($1, 'Approve budget', 'Motion to approve the 2026 budget', $2)
        RETURNING id
        "#,
    )
    .bind(meeting_id)
    .bind(proposed_by)
    .fetch_one(pool)
    .await
    .expect("seed motion")
}

/// Mint a real HS256 access token for `user_id`, signed with the same secret
/// the TestApp configures into `JWT_SECRET`.
fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "BoardIDOR User", None, None)
        .expect("mint access token")
}

fn assert_rejected(status: StatusCode, ctx: &str) {
    let code = status.as_u16();
    assert!(
        (400..500).contains(&code),
        "{ctx}: cross-tenant/unauthenticated request must be rejected with 4xx, got {status}"
    );
}

/// Seed two orgs with one member each plus a meeting in Org A.
/// Returns (org_a, org_b, user_a, user_b, meeting_a).
async fn seed_two_org_fixture(pool: &PgPool, tag: &str) -> (Uuid, Uuid, Uuid, Uuid, Uuid) {
    let org_a = seed_org(pool, &format!("{tag}-a")).await;
    let org_b = seed_org(pool, &format!("{tag}-b")).await;
    let user_a = seed_user(pool, &format!("{tag}-a@board-idor.test")).await;
    let user_b = seed_user(pool, &format!("{tag}-b@board-idor.test")).await;
    seed_membership(pool, org_a, user_a, "org_admin").await;
    seed_membership(pool, org_b, user_b, "org_admin").await;
    let meeting_a = seed_meeting(pool, org_a, user_a).await;
    (org_a, org_b, user_a, user_b, meeting_a)
}

// ---------------------------------------------------------------------------
// T1 — unauthenticated get_meeting is rejected
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_meeting_without_auth_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "noauth-a").await;
    let user_a = seed_user(&pool, "noauth-a@board-idor.test").await;
    let meeting_a = seed_meeting(&pool, org_a, user_a).await;

    let uri = format!("/api/v1/board-meetings/{meeting_a}");
    let resp = app.execute(app.get(&uri).build()).await;

    assert_rejected(resp.status, "get_meeting without bearer token");
}

// ---------------------------------------------------------------------------
// T2 — cross-org get_meeting by UUID is rejected (IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_meeting_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, _user_a, user_b, meeting_a) = seed_two_org_fixture(&pool, "get").await;

    let token_b = mint_token(user_b, "get-b@board-idor.test");
    let uri = format!("/api/v1/board-meetings/{meeting_a}");
    // Valid context for the attacker's OWN org — the by-id probe must fail on
    // row scoping (404), not on a missing tenant header.
    let resp = app
        .execute(
            app.get(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;

    assert_rejected(resp.status, "get_meeting cross-tenant");
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org A meeting must not be readable by Org B"
    );
}

// ---------------------------------------------------------------------------
// T3 — cross-org update_meeting is rejected (mutate IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_meeting_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, _user_a, user_b, meeting_a) = seed_two_org_fixture(&pool, "upd").await;

    let token_b = mint_token(user_b, "upd-b@board-idor.test");
    let uri = format!("/api/v1/board-meetings/{meeting_a}");
    let body = json!({ "title": "Hijacked Meeting" });
    let resp = app
        .execute(
            RequestBuilder::new(Method::PUT, &uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .json(body)
                .build(),
        )
        .await;

    assert_rejected(resp.status, "update_meeting cross-tenant");

    // The meeting title must be unchanged.
    let title: String = sqlx::query_scalar("SELECT title FROM board_meetings WHERE id = $1")
        .bind(meeting_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        title, "Annual Budget Meeting",
        "Org A meeting must not be mutated cross-tenant"
    );
}

// ---------------------------------------------------------------------------
// T4 — cross-org delete_meeting is rejected (destructive IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_meeting_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, _user_a, user_b, meeting_a) = seed_two_org_fixture(&pool, "del").await;

    let token_b = mint_token(user_b, "del-b@board-idor.test");
    let uri = format!("/api/v1/board-meetings/{meeting_a}");
    let resp = app
        .execute(
            app.delete(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;

    assert_rejected(resp.status, "delete_meeting cross-tenant");
    assert_ne!(
        resp.status,
        StatusCode::NO_CONTENT,
        "Org A meeting must not be deletable by Org B"
    );

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM board_meetings WHERE id = $1")
        .bind(meeting_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "Org A meeting must not be deleted cross-tenant");
}

// ---------------------------------------------------------------------------
// T5 — cross-org get_agenda_item is rejected (child table, EXISTS keying)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_agenda_item_from_other_org_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, user_a, user_b, meeting_a) = seed_two_org_fixture(&pool, "agi").await;
    let agenda_a = seed_agenda_item(&pool, meeting_a, user_a).await;

    let token_b = mint_token(user_b, "agi-b@board-idor.test");
    let uri = format!("/api/v1/board-meetings/agenda/{agenda_a}");
    let resp = app
        .execute(
            app.get(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;

    assert_rejected(resp.status, "get_agenda_item cross-tenant");
    assert_ne!(
        resp.status,
        StatusCode::OK,
        "Org A agenda item must not be readable by Org B"
    );
}

// ---------------------------------------------------------------------------
// T6 — cross-org add_agenda_item is rejected (child insert, root-meeting
// validation)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_agenda_item_to_other_org_meeting_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, _user_a, user_b, meeting_a) = seed_two_org_fixture(&pool, "ins").await;

    let token_b = mint_token(user_b, "ins-b@board-idor.test");
    let uri = format!("/api/v1/board-meetings/{meeting_a}/agenda");
    let body = json!({
        "meeting_id": meeting_a,
        "item_number": "99",
        "title": "Injected agenda item",
    });
    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .json(body)
                .build(),
        )
        .await;

    assert_rejected(resp.status, "add_agenda_item cross-tenant");

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM meeting_agenda_items WHERE meeting_id = $1")
            .bind(meeting_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count, 0,
        "no agenda item may be inserted into another org's meeting"
    );
}

// ---------------------------------------------------------------------------
// T7 — cross-org motion mutation (start-voting) is rejected (grand-child IDOR)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn start_voting_on_other_org_motion_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_org_a, org_b, user_a, user_b, meeting_a) = seed_two_org_fixture(&pool, "mot").await;
    let motion_a = seed_motion(&pool, meeting_a, user_a).await;

    let token_b = mint_token(user_b, "mot-b@board-idor.test");
    let uri = format!("/api/v1/board-meetings/motions/{motion_a}/start-voting");
    let resp = app
        .execute(
            app.post(&uri)
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .json(json!({}))
                .build(),
        )
        .await;

    assert_rejected(resp.status, "start_motion_voting cross-tenant");

    let status: String =
        sqlx::query_scalar("SELECT status::text FROM meeting_motions WHERE id = $1")
            .bind(motion_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        status, "proposed",
        "Org A motion must not transition to voting cross-tenant"
    );
}

// ---------------------------------------------------------------------------
// T8 — legitimate same-org access succeeds
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_meeting_for_own_org_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "own-a").await;
    let user_a = seed_user(&pool, "own-a@board-idor.test").await;
    seed_membership(&pool, org_a, user_a, "org_admin").await;
    let meeting_a = seed_meeting(&pool, org_a, user_a).await;

    let token_a = mint_token(user_a, "own-a@board-idor.test");
    let uri = format!("/api/v1/board-meetings/{meeting_a}");
    let resp = app
        .execute(
            app.get(&uri)
                .bearer(&token_a)
                .header("X-Tenant-ID", &org_a.to_string())
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "Org A member must be able to read its own meeting: {}",
        resp.text()
    );
    let detail = resp.json_value();
    assert_eq!(
        detail
            .get("meeting")
            .and_then(|m| m.get("id"))
            .and_then(|v| v.as_str()),
        Some(meeting_a.to_string().as_str()),
        "expected the owning org's meeting, got {detail}"
    );
}
