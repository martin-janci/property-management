//! Round-5 security regressions (sec-a).
//!
//! * developer API keys: revoke / rotate only touch a key of the developer in
//!   the path, owned by the caller — anything else is 404 and inert;
//! * package / visitor building settings: a building of another org is 404 and
//!   nothing is written;
//! * announcement attachments: `file_key` must be in the org's storage
//!   namespace (`{org_id}/…`) — another org's namespace is 404, junk is 400;
//! * rental reads validate DB membership of the request org (a non-member
//!   naming another org via `X-Tenant-ID` is refused).
//!
//! Every caller holds a login-format access token (no `tenant_id` / `role`
//! claim), so each decision comes from the database.

#![allow(dead_code)]

use api_server::services::JwtService;
use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig, TestResponse};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

struct Actor {
    id: Uuid,
    token: String,
}

async fn user(pool: &PgPool, label: &str) -> Actor {
    let email = format!("r5a-{label}-{}@r5-sec-a.test", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'hash', 'R5 Sec A', 'active', NOW()) RETURNING id"#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    let token = JwtService::new(&TestConfig::default().jwt_secret)
        .expect("jwt service")
        .generate_access_token(id, &email, "R5 Sec A", None, None)
        .expect("mint token");
    Actor { id, token }
}

async fn member(pool: &PgPool, org: Uuid, role: &str) -> Actor {
    let actor = user(pool, role).await;
    seed_membership(pool, org, actor.id, role).await;
    actor
}

async fn call(
    app: &TestApp,
    method: &str,
    uri: &str,
    actor: &Actor,
    org: Option<Uuid>,
    body: Option<serde_json::Value>,
) -> TestResponse {
    let mut builder = match method {
        "GET" => app.get(uri),
        "POST" => app.post(uri),
        "PUT" => app.put(uri),
        "DELETE" => app.delete(uri),
        other => panic!("unsupported method {other}"),
    }
    .bearer(&actor.token);
    if let Some(org) = org {
        builder = builder.tenant(org);
    }
    if let Some(b) = body {
        builder = builder.json(b);
    }
    builder.send().await
}

fn expect(resp: &TestResponse, status: StatusCode, ctx: &str) {
    assert_eq!(
        resp.status,
        status,
        "{ctx}: expected {status}, got {}: {}",
        resp.status,
        resp.text()
    );
}

async fn seed_building(pool: &PgPool, org: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'R5 St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

// ---------------------------------------------------------------------------
// 1. Developer API keys: revoke / rotate are owner-scoped
// ---------------------------------------------------------------------------

async fn developer_account(pool: &PgPool, owner: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO developer_accounts (user_id, email, status)
           VALUES ($1, $2, 'approved') RETURNING id"#,
    )
    .bind(owner)
    .bind(format!("dev-{}@r5-sec-a.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed developer account")
}

async fn developer_key(pool: &PgPool, developer: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO developer_api_keys (developer_id, name, key_prefix, key_hash)
           VALUES ($1, 'k', 'ppt_live', 'sha256:x') RETURNING id"#,
    )
    .bind(developer)
    .fetch_one(pool)
    .await
    .expect("seed developer api key")
}

async fn key_is_active(pool: &PgPool, key: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT revoked_at IS NULL AND is_active FROM developer_api_keys WHERE id = $1",
    )
    .bind(key)
    .fetch_one(pool)
    .await
    .expect("key state")
}

async fn key_count(pool: &PgPool, developer: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM developer_api_keys WHERE developer_id = $1")
        .bind(developer)
        .fetch_one(pool)
        .await
        .expect("key count")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn developer_key_revoke_and_rotate_are_owner_scoped(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let alice = user(&pool, "dev-alice").await;
    let mallory = user(&pool, "dev-mallory").await;
    let dev_a = developer_account(&pool, alice.id).await;
    let dev_m = developer_account(&pool, mallory.id).await;
    let key_a = developer_key(&pool, dev_a).await;

    let base = "/api/v1/ecosystem/developers";
    // Foreign developer id + its key, and own developer id + foreign key: 404.
    for dev in [dev_a, dev_m] {
        let revoke = call(
            &app,
            "DELETE",
            &format!("{base}/{dev}/keys/{key_a}"),
            &mallory,
            None,
            None,
        )
        .await;
        expect(&revoke, StatusCode::NOT_FOUND, "revoke someone else's key");
        let rotate = call(
            &app,
            "POST",
            &format!("{base}/{dev}/keys/{key_a}/rotate"),
            &mallory,
            None,
            None,
        )
        .await;
        expect(&rotate, StatusCode::NOT_FOUND, "rotate someone else's key");
    }
    assert!(key_is_active(&pool, key_a).await, "victim key must survive");
    assert_eq!(
        key_count(&pool, dev_a).await,
        1,
        "no replacement key minted"
    );
    assert_eq!(
        key_count(&pool, dev_m).await,
        0,
        "nothing minted for caller"
    );

    // The owner can still rotate and revoke their own key.
    let rotated = call(
        &app,
        "POST",
        &format!("{base}/{dev_a}/keys/{key_a}/rotate"),
        &alice,
        None,
        None,
    )
    .await;
    expect(&rotated, StatusCode::OK, "owner rotates own key");
    assert!(!key_is_active(&pool, key_a).await, "rotated key is revoked");
    let new_key: Uuid = rotated.json_value()["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .expect("rotated key id");
    let revoked = call(
        &app,
        "DELETE",
        &format!("{base}/{dev_a}/keys/{new_key}"),
        &alice,
        None,
        None,
    )
    .await;
    expect(&revoked, StatusCode::NO_CONTENT, "owner revokes own key");
    assert!(!key_is_active(&pool, new_key).await);
}

// ---------------------------------------------------------------------------
// 2. Package / visitor building settings: building must be in the org
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn building_settings_of_foreign_building_are_404_and_not_written(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "r5a-pv-a").await;
    let org_b = seed_org(&pool, "r5a-pv-b").await;
    let manager = member(&pool, org_a, "manager").await;
    let own = seed_building(&pool, org_a).await;
    let foreign = seed_building(&pool, org_b).await;

    for (kind, body) in [
        ("packages", json!({"max_storage_days": 10})),
        ("visitors", json!({"code_length": 8})),
    ] {
        let resp = call(
            &app,
            "PUT",
            &format!("/api/v1/{kind}/buildings/{foreign}/settings"),
            &manager,
            Some(org_a),
            Some(body.clone()),
        )
        .await;
        expect(
            &resp,
            StatusCode::NOT_FOUND,
            &format!("{kind}: foreign building"),
        );

        let resp = call(
            &app,
            "PUT",
            &format!("/api/v1/{kind}/buildings/{own}/settings"),
            &manager,
            Some(org_a),
            Some(body),
        )
        .await;
        expect(&resp, StatusCode::OK, &format!("{kind}: own building"));
    }

    for (table, sql) in [
        (
            "building_package_settings",
            "SELECT COUNT(*) FROM building_package_settings WHERE building_id = $1",
        ),
        (
            "building_visitor_settings",
            "SELECT COUNT(*) FROM building_visitor_settings WHERE building_id = $1",
        ),
    ] {
        let rows: i64 = sqlx::query_scalar(sql)
            .bind(foreign)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 0, "{table}: nothing written for a foreign building");
    }
}

// ---------------------------------------------------------------------------
// 3. Announcement attachments: file_key org prefix
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn announcement_attachment_file_key_must_be_in_org_namespace(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "r5a-ann").await;
    let other_org = seed_org(&pool, "r5a-ann-other").await;
    let manager = member(&pool, org, "manager").await;
    let announcement = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO announcements (organization_id, author_id, title, content, target_type, status)
           VALUES ($1, $2, 'T', 'C', 'all'::announcement_target_type, 'draft'::announcement_status)
           RETURNING id"#,
    )
    .bind(org)
    .bind(manager.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let uri = format!("/api/v1/announcements/{announcement}/attachments");
    let body = |key: String| json!({"file_key": key, "file_name": "a.pdf", "file_type": "application/pdf", "file_size": 1});

    for (key, status, ctx) in [
        (
            format!("{other_org}/2026/11/x_a.pdf"),
            StatusCode::NOT_FOUND,
            "another org's namespace",
        ),
        (
            "messages/thread/a.pdf".to_string(),
            StatusCode::BAD_REQUEST,
            "no org prefix",
        ),
        (
            format!("{org}/../{other_org}/a.pdf"),
            StatusCode::BAD_REQUEST,
            "traversal",
        ),
    ] {
        let resp = call(&app, "POST", &uri, &manager, Some(org), Some(body(key))).await;
        expect(&resp, status, ctx);
    }
    let stored: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM announcement_attachments WHERE announcement_id = $1",
    )
    .bind(announcement)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored, 0, "rejected keys must not be stored");

    let ok = call(
        &app,
        "POST",
        &uri,
        &manager,
        Some(org),
        Some(body(format!("{org}/2026/11/x_a.pdf"))),
    )
    .await;
    expect(&ok, StatusCode::CREATED, "own namespace");
}

// ---------------------------------------------------------------------------
// 4. Rental reads validate DB membership
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn rental_reads_require_membership_of_request_org(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "r5a-rent-a").await;
    let org_b = seed_org(&pool, "r5a-rent-b").await;
    let outsider = member(&pool, org_b, "manager").await;
    let insider = member(&pool, org_a, "manager").await;

    for uri in [
        "/api/v1/rentals/statistics",
        "/api/v1/rentals/sync-status",
        "/api/v1/rentals/connections",
        "/api/v1/rentals/bookings",
        "/api/v1/rentals/guests",
    ] {
        let resp = call(&app, "GET", uri, &outsider, Some(org_a), None).await;
        expect(
            &resp,
            StatusCode::FORBIDDEN,
            &format!("non-member GET {uri}"),
        );
        let resp = call(&app, "GET", uri, &insider, Some(org_a), None).await;
        expect(&resp, StatusCode::OK, &format!("member GET {uri}"));
    }
}
