//! Integration tests for the Airbnb direct-connect and availability-sync
//! enqueue routes (Gap 83-1).
//!
//! Route file: `src/routes/integrations/install.rs`
//!
//! | Route                                                      | Handler                            |
//! |-------------------------------------------------------------|-------------------------------------|
//! | `POST /organizations/{org_id}/airbnb/direct-connect`        | `direct_connect_airbnb`            |
//! | `POST /organizations/{org_id}/airbnb/availability-sync`     | `enqueue_airbnb_availability_sync` |
//!
//! # Why this file exists
//!
//! Both handlers were added in Gap 83-1 alongside the pre-existing
//! connect/sync/disconnect/listings/reservations surface (covered by
//! `airbnb_install_routes_tests.rs` / `airbnb_oauth_routes_tests.rs`) but
//! shipped with no regression coverage of their own.
//!
//! # Authorization model (different from the rest of the Airbnb surface!)
//!
//! Unlike `connect_airbnb` / `sync_airbnb` / `disconnect_airbnb` (which gate on
//! `verify_org_access`, an `organization_members` row), these two Gap 83-1
//! handlers gate directly on the JWT claims carried by `AuthUser`:
//! - `auth.tenant_id` must equal the path `{org_id}` — UNLESS the caller is a
//!   platform admin (`auth.is_platform_admin()`), which bypasses the check.
//! - `auth.role` must be one of `SuperAdmin` / `PlatformAdmin` / `OrgAdmin` /
//!   `Manager`.
//!
//! No DB membership check (`organization_members`) is made by either handler,
//! so these tests mint tokens with the desired `tenant_id` / `role` claims
//! directly and do not seed org membership rows.
//!
//! # Airbnb configuration in this binary
//!
//! Unlike `airbnb_install_routes_tests.rs`, this file deliberately does NOT
//! set `AIRBNB_CLIENT_ID` / `AIRBNB_CLIENT_SECRET`. `direct_connect_airbnb`
//! reads `state.airbnb_config` (loaded once per `AppState::new` from the
//! environment, Issue #711) and fails closed with `500 NOT_CONFIGURED` when
//! the client id/secret are empty — BEFORE ever calling the live Airbnb
//! Partner API (`fetch_listings`). That lets the authorized-happy-path test
//! exercise the handler all the way past the auth/IDOR/role gates
//! deterministically, without any outbound network call or a real Airbnb
//! token. `enqueue_airbnb_availability_sync` never touches `airbnb_config`
//! (it only reads/writes the DB), so its happy path is exercised fully
//! end-to-end, including the persisted job row.

#![allow(dead_code)]

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use common::{seed_org, TestApp};

// Must match `TestConfig::default().jwt_secret`.
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

// ---------------------------------------------------------------------------
// JWT helper — mirror of `api_core::extractors::auth::Claims`
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct AccessClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint_token_with_role(user_id: Uuid, tenant_id: Option<Uuid>, role: &str) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id,
        role: Some(role.to_string()),
        email: "gap83-1-direct-connect-test@test.local".to_string(),
        name: "Gap 83-1 Direct Connect Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint access token")
}

/// A manager token whose JWT `tenant_id` claim matches `org_id` — passes both
/// gates on the Gap 83-1 handlers.
fn mint_manager_token(user_id: Uuid, org_id: Uuid) -> String {
    mint_token_with_role(user_id, Some(org_id), "manager")
}

// ---------------------------------------------------------------------------
// Database fixtures
// ---------------------------------------------------------------------------

/// Seed an org-level (nil `unit_id`) Airbnb connection directly. No
/// building/unit graph is required: migration `00181_rental_org_level_platform_unique_idx.sql`
/// dropped the `unit_id` foreign key specifically so org-level connections can
/// use the nil-UUID sentinel (the same shape `direct_connect_airbnb` /
/// `upsert_airbnb_connection` write when no `unit_id` is supplied).
async fn seed_org_level_airbnb_connection(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_platform_connections (
            organization_id, unit_id, platform,
            access_token, refresh_token, is_active
        )
        VALUES ($1, '00000000-0000-0000-0000-000000000000', 'airbnb', 'enc:tok', 'enc:refresh', true)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed org-level airbnb connection")
}

/// Seed a real `users` row and return its id.
///
/// The availability-sync happy paths persist a `background_jobs` row whose
/// `created_by` column carries a foreign key to `users(id)`. The authenticated
/// caller must therefore be a real seeded user — a random `Uuid::new_v4()`
/// would violate the FK and turn the enqueue INSERT into a 500 instead of the
/// expected 202. (The rejection tests never reach the enqueue, so they keep
/// their throwaway ids.)
async fn seed_user(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'hash', 'Gap 83-1 Enqueue Caller', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(format!("gap83-1-enqueue-{}@test.local", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

// ---------------------------------------------------------------------------
// Request helpers
// ---------------------------------------------------------------------------

fn authed_request(
    method: Method,
    uri: &str,
    token: &str,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json");
    builder
        .body(match body {
            Some(b) => Body::from(b.to_string()),
            None => Body::empty(),
        })
        .unwrap()
}

fn anon_request(method: Method, uri: &str, body: Option<serde_json::Value>) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    builder
        .body(match body {
            Some(b) => Body::from(b.to_string()),
            None => Body::empty(),
        })
        .unwrap()
}

fn direct_connect_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/airbnb/direct-connect")
}

fn availability_sync_uri(org_id: Uuid) -> String {
    format!("/api/v1/integrations/organizations/{org_id}/airbnb/availability-sync")
}

/// Auth-rejection statuses (mirrors `is_protected` in the sibling suites).
fn is_protected(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::BAD_REQUEST
    )
}

// ===========================================================================
// Route-existence smoke test
// ===========================================================================

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn gap_83_1_routes_are_mounted(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = Uuid::new_v4(); // arbitrary — we only care about not-404/405

    let routes: &[(String, Method, Option<serde_json::Value>)] = &[
        (
            direct_connect_uri(org_id),
            Method::POST,
            Some(json!({"access_token": "tok"})),
        ),
        (availability_sync_uri(org_id), Method::POST, None),
    ];

    for (uri, method, body) in routes {
        let resp = app
            .execute(anon_request(method.clone(), uri, body.clone()))
            .await;
        assert_ne!(
            resp.status,
            StatusCode::NOT_FOUND,
            "route {method} {uri} must be mounted (got 404 — router wiring broken)"
        );
        assert_ne!(
            resp.status,
            StatusCode::METHOD_NOT_ALLOWED,
            "route {method} {uri} must accept the expected method (got 405)"
        );
    }
}

// ===========================================================================
// POST /organizations/{org_id}/airbnb/direct-connect
// ===========================================================================

/// Unauthenticated direct-connect must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn direct_connect_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "dc-unauth").await;
    let resp = app
        .execute(anon_request(
            Method::POST,
            &direct_connect_uri(org_id),
            Some(json!({"access_token": "tok"})),
        ))
        .await;
    assert!(
        is_protected(resp.status),
        "unauthenticated direct-connect must be rejected; got {}",
        resp.status
    );
}

/// IDOR guard: a caller whose JWT `tenant_id` claim does not match the path
/// org must get 403 — this handler checks `auth.tenant_id`, not an
/// `organization_members` row.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn direct_connect_rejects_mismatched_tenant(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "dc-idor-a").await; // target org
    let org_b = seed_org(&pool, "dc-idor-b").await; // caller's JWT tenant
    let user_id = Uuid::new_v4();

    let token = mint_manager_token(user_id, org_b);
    let resp = app
        .execute(authed_request(
            Method::POST,
            &direct_connect_uri(org_a),
            &token,
            Some(json!({"access_token": "tok"})),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "mismatched-tenant direct-connect must be 403; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("FORBIDDEN"),
        "error body should carry FORBIDDEN code: {}",
        resp.text()
    );

    // No connection row must have been written for either org.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rental_platform_connections WHERE organization_id IN ($1, $2)",
    )
    .bind(org_a)
    .bind(org_b)
    .fetch_one(&pool)
    .await
    .expect("count connections");
    assert_eq!(
        count, 0,
        "IDOR-rejected direct-connect must not write a row"
    );
}

/// A missing `tenant_id` claim (token never scoped to an org) must also be
/// rejected: `auth.tenant_id != Some(path.org_id)` is true when the claim is
/// `None`.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn direct_connect_rejects_missing_tenant_claim(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "dc-notenant").await;
    let user_id = Uuid::new_v4();

    let token = mint_token_with_role(user_id, None, "manager");
    let resp = app
        .execute(authed_request(
            Method::POST,
            &direct_connect_uri(org_id),
            &token,
            Some(json!({"access_token": "tok"})),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "direct-connect with no tenant_id claim must be 403; got {}: {}",
        resp.status,
        resp.text()
    );
}

/// Role gate: a caller whose JWT tenant matches but whose role is below
/// Manager must get 403 with the insufficient-permissions message — a
/// distinct branch from the tenant-mismatch guard above.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn direct_connect_rejects_insufficient_role(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "dc-role").await;
    let user_id = Uuid::new_v4();

    let token = mint_token_with_role(user_id, Some(org_id), "tenant");
    let resp = app
        .execute(authed_request(
            Method::POST,
            &direct_connect_uri(org_id),
            &token,
            Some(json!({"access_token": "tok"})),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-manager direct-connect must be 403; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("Insufficient permissions"),
        "error body should explain the role gate: {}",
        resp.text()
    );
}

/// Platform admins bypass the tenant-match check (`auth.is_platform_admin()`)
/// even when their JWT tenant differs from the path org — reaching the
/// handler body (NOT_CONFIGURED, since Airbnb creds are unset in this binary)
/// instead of being rejected at the IDOR guard.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn direct_connect_platform_admin_bypasses_tenant_mismatch(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "dc-pa-a").await; // target org
    let org_b = seed_org(&pool, "dc-pa-b").await; // admin's own JWT tenant
    let user_id = Uuid::new_v4();

    let token = mint_token_with_role(user_id, Some(org_b), "platform_admin");
    let resp = app
        .execute(authed_request(
            Method::POST,
            &direct_connect_uri(org_a),
            &token,
            Some(json!({"access_token": "tok"})),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "platform admin must pass the IDOR + role gates and reach the (unconfigured) handler body; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("NOT_CONFIGURED"),
        "expected NOT_CONFIGURED once past the auth gates: {}",
        resp.text()
    );
}

/// Once the auth/role gates pass, the handler reads `state.airbnb_config`
/// (Issue #711). This binary never sets `AIRBNB_CLIENT_ID` /
/// `AIRBNB_CLIENT_SECRET`, so the request must short-circuit with
/// `500 NOT_CONFIGURED` BEFORE any live call to the Airbnb Partner API is
/// attempted — proving the handler is reached past both gates
/// deterministically, without needing outbound network access or a real
/// Airbnb token.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn direct_connect_authorized_request_reaches_not_configured(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "dc-notconfigured").await;
    let user_id = Uuid::new_v4();

    let token = mint_manager_token(user_id, org_id);
    let resp = app
        .execute(authed_request(
            Method::POST,
            &direct_connect_uri(org_id),
            &token,
            Some(json!({
                "access_token": "pretend-valid-token",
                "refresh_token": "pretend-refresh",
                "airbnb_account_id": "ACCT-1"
            })),
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "authorized direct-connect without Airbnb credentials configured must be 500 NOT_CONFIGURED; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("NOT_CONFIGURED"),
        "error body should carry NOT_CONFIGURED code: {}",
        resp.text()
    );

    // The handler must fail closed before ever reaching the upsert.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rental_platform_connections WHERE organization_id = $1",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("count connections");
    assert_eq!(
        count, 0,
        "no connection row should be written when Airbnb is not configured"
    );
}

/// Happy path (issue #2240): an authorized manager on a configured org, whose
/// supplied access token validates against a stubbed Airbnb Partner API,
/// gets `200` + a fully-populated [`AirbnbDirectConnectResponse`] and exactly
/// one persisted `rental_platform_connections` row with the token encrypted at
/// rest.
///
/// This is the previously-missing success/write-path coverage: unlike the
/// NOT_CONFIGURED test above, this binary injects non-empty Airbnb credentials
/// **and** an `api_base` pointing at a `wiremock` server via
/// `TestApp::with_airbnb_config`, so the handler runs all the way through
/// `client.fetch_listings(...)` → `encrypt_required(...)` →
/// `rental_repo.upsert_airbnb_connection(...)` without any live network call.
/// The per-instance config injection (rather than process-global `AIRBNB_*`
/// env vars) keeps the sibling NOT_CONFIGURED case — which requires the
/// credentials to stay empty — passing under concurrent `#[sqlx::test]`
/// execution.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn direct_connect_happy_path_upserts_connection(pool: PgPool) {
    // `direct_connect_airbnb` encrypts the token before storage via
    // `IntegrationCrypto::try_from_env()` and fails closed (500
    // ENCRYPTION_REQUIRED) when `INTEGRATION_ENCRYPTION_KEY` is unset. Seed a
    // valid 64-hex-char (32-byte) key for this binary. No sibling test in this
    // file asserts the key is absent, so a one-time global set is safe here.
    static ENC_KEY_ONCE: std::sync::Once = std::sync::Once::new();
    ENC_KEY_ONCE.call_once(|| {
        if std::env::var("INTEGRATION_ENCRYPTION_KEY").is_err() {
            std::env::set_var("INTEGRATION_ENCRYPTION_KEY", "0".repeat(64));
        }
    });

    // Stub the Airbnb Partner API: GET {base}/listings → two listings. The
    // handler only counts the listings, so a minimal-but-valid `AirbnbListing`
    // shape (required non-Option fields + empty photos/amenities) is enough.
    let mock_server = MockServer::start().await;
    let listings_body = json!({
        "listings": [
            {
                "id": "L1", "name": "Listing One", "property_type": "apartment",
                "room_type": "entire_home", "bedrooms": 2, "bathrooms": 1.0,
                "person_capacity": 4, "status": "active",
                "photos": [], "amenities": []
            },
            {
                "id": "L2", "name": "Listing Two", "property_type": "house",
                "room_type": "private_room", "bedrooms": 1, "bathrooms": 1.5,
                "person_capacity": 2, "status": "active",
                "photos": [], "amenities": []
            }
        ]
    });
    Mock::given(method("GET"))
        .and(path("/listings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(listings_body))
        .mount(&mock_server)
        .await;

    let airbnb_config = api_server::state::AirbnbAppConfig {
        client_id: "test-client-id".to_string(),
        client_secret: "test-client-secret".to_string(),
        redirect_uri: "http://localhost/callback".to_string(),
        webhook_secret: String::new(),
        // Point the direct-connect client at the stub; the handler appends
        // `/listings`.
        api_base: mock_server.uri(),
    };

    let app = TestApp::with_airbnb_config(pool.clone(), airbnb_config).await;
    let org_id = seed_org(&pool, "dc-happy").await;
    let user_id = Uuid::new_v4();
    let token = mint_manager_token(user_id, org_id);

    let resp = app
        .execute(authed_request(
            Method::POST,
            &direct_connect_uri(org_id),
            &token,
            Some(json!({
                "access_token": "valid-airbnb-token",
                "refresh_token": "valid-airbnb-refresh",
                "airbnb_account_id": "ACCT-1"
            })),
        ))
        .await;

    assert_eq!(
        resp.status,
        StatusCode::OK,
        "configured direct-connect with a valid token must be 200; got {}: {}",
        resp.status,
        resp.text()
    );

    // Response DTO: success flag, a UUID connection_id, listings_count from the
    // stub, and a human-readable message referencing the count.
    let body: serde_json::Value =
        serde_json::from_str(&resp.text()).expect("direct-connect response must be JSON");
    assert_eq!(body["success"], true, "body: {body}");
    assert_eq!(body["listings_count"], 2, "body: {body}");
    let connection_id = body["connection_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .expect("connection_id must be a UUID string");
    assert!(
        body["message"]
            .as_str()
            .is_some_and(|m| m.contains("2 listing")),
        "message should reference the listing count: {body}"
    );

    // Exactly one Airbnb connection row is persisted for the org.
    let row = sqlx::query(
        r#"
        SELECT id, organization_id, platform, access_token, refresh_token, is_active
        FROM rental_platform_connections
        WHERE organization_id = $1 AND platform = 'airbnb'
        "#,
    )
    .bind(org_id)
    .fetch_all(&pool)
    .await
    .expect("query connection rows");
    assert_eq!(
        row.len(),
        1,
        "exactly one airbnb connection row must be persisted"
    );
    let row = &row[0];
    assert_eq!(row.get::<Uuid, _>("id"), connection_id);
    assert_eq!(row.get::<Uuid, _>("organization_id"), org_id);
    assert!(row.get::<bool, _>("is_active"));

    // Token is encrypted at rest (Issue #765 mandatory-encryption contract):
    // the persisted value carries the "enc:" prefix, never the plaintext token.
    let stored_access: String = row.get("access_token");
    assert!(
        stored_access.starts_with("enc:"),
        "access token must be stored encrypted, got: {stored_access}"
    );
    assert!(
        !stored_access.contains("valid-airbnb-token"),
        "plaintext token must never be persisted: {stored_access}"
    );
    let stored_refresh: Option<String> = row.get("refresh_token");
    assert!(
        stored_refresh.as_deref().is_some_and(|t| t.starts_with("enc:")),
        "refresh token must be stored encrypted: {stored_refresh:?}"
    );
}

// ===========================================================================
// POST /organizations/{org_id}/airbnb/availability-sync
// ===========================================================================

/// Unauthenticated availability-sync must be rejected.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn availability_sync_rejects_unauthenticated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "as-unauth").await;
    let resp = app
        .execute(anon_request(
            Method::POST,
            &availability_sync_uri(org_id),
            None,
        ))
        .await;
    assert!(
        is_protected(resp.status),
        "unauthenticated availability-sync must be rejected; got {}",
        resp.status
    );
}

/// IDOR guard: a caller whose JWT `tenant_id` does not match the path org
/// must get 403, even though a connection exists for the target org — and no
/// job may be queued.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn availability_sync_rejects_mismatched_tenant(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "as-idor-a").await;
    let org_b = seed_org(&pool, "as-idor-b").await;
    seed_org_level_airbnb_connection(&pool, org_a).await;
    let user_id = Uuid::new_v4();

    let token = mint_manager_token(user_id, org_b);
    let resp = app
        .execute(authed_request(
            Method::POST,
            &availability_sync_uri(org_a),
            &token,
            None,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "mismatched-tenant availability-sync must be 403; got {}: {}",
        resp.status,
        resp.text()
    );

    let job_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM background_jobs WHERE org_id = $1")
            .bind(org_a)
            .fetch_one(&pool)
            .await
            .expect("count jobs");
    assert_eq!(job_count, 0, "IDOR-rejected request must not enqueue a job");
}

/// Role gate: a caller whose JWT tenant matches but whose role is below
/// Manager must get 403 — no job may be queued.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn availability_sync_rejects_insufficient_role(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "as-role").await;
    seed_org_level_airbnb_connection(&pool, org_id).await;
    let user_id = Uuid::new_v4();

    let token = mint_token_with_role(user_id, Some(org_id), "resident");
    let resp = app
        .execute(authed_request(
            Method::POST,
            &availability_sync_uri(org_id),
            &token,
            None,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-manager availability-sync must be 403; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("Insufficient permissions"),
        "error body should explain the role gate: {}",
        resp.text()
    );
}

/// No Airbnb connection exists for the org → 404, and no job is created.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn availability_sync_returns_404_when_no_connection(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "as-noconn").await;
    let user_id = Uuid::new_v4();

    let token = mint_manager_token(user_id, org_id);
    let resp = app
        .execute(authed_request(
            Method::POST,
            &availability_sync_uri(org_id),
            &token,
            None,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "availability-sync with no connection must be 404; got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("NOT_FOUND"),
        "error body should carry NOT_FOUND code: {}",
        resp.text()
    );

    let job_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM background_jobs WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(&pool)
            .await
            .expect("count jobs");
    assert_eq!(
        job_count, 0,
        "no job should be queued when there is no connection"
    );
}

/// Happy path: an authorized manager on a connected org gets 202 + a job id,
/// and the job row is actually persisted with the expected shape (job_type,
/// queue, org scoping, attempt budget, and a payload referencing the
/// connection).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn availability_sync_happy_path_enqueues_job(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "as-happy").await;
    let conn_id = seed_org_level_airbnb_connection(&pool, org_id).await;
    let user_id = seed_user(&pool).await;

    let token = mint_manager_token(user_id, org_id);
    let resp = app
        .execute(authed_request(
            Method::POST,
            &availability_sync_uri(org_id),
            &token,
            None,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::ACCEPTED,
        "manager availability-sync must be 202; got {}: {}",
        resp.status,
        resp.text()
    );

    let body: serde_json::Value =
        serde_json::from_str(&resp.text()).expect("availability-sync response must be JSON");
    assert_eq!(body["queued"], true);
    assert!(
        body["message"].as_str().is_some_and(|m| !m.is_empty()),
        "response must carry a human-readable message: {body}"
    );
    let job_id = body["job_id"].as_str().expect("job_id must be a string");
    let job_id = Uuid::parse_str(job_id).expect("job_id must be a valid UUID");

    let row = sqlx::query(
        r#"
        SELECT job_type, queue, org_id, priority, max_attempts, payload, created_by
        FROM background_jobs WHERE id = $1
        "#,
    )
    .bind(job_id)
    .fetch_one(&pool)
    .await
    .expect("queued job row must exist");

    assert_eq!(row.get::<String, _>("job_type"), "sync_external");
    assert_eq!(row.get::<String, _>("queue"), "low");
    assert_eq!(row.get::<Option<Uuid>, _>("org_id"), Some(org_id));
    assert_eq!(row.get::<i32, _>("priority"), 1);
    assert_eq!(row.get::<i32, _>("max_attempts"), 3);
    assert_eq!(row.get::<Option<Uuid>, _>("created_by"), Some(user_id));

    let payload: serde_json::Value = row.get("payload");
    assert_eq!(payload["org_id"], org_id.to_string());
    assert_eq!(payload["connection_id"], conn_id.to_string());
    assert_eq!(payload["sync_type"], "availability");
}

/// Platform admins bypass the tenant-match check for availability-sync too,
/// and successfully enqueue the job for the (mismatched) target org.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn availability_sync_platform_admin_bypasses_tenant_mismatch(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "as-pa-a").await; // target org
    let org_b = seed_org(&pool, "as-pa-b").await; // admin's own JWT tenant
    seed_org_level_airbnb_connection(&pool, org_a).await;
    let user_id = seed_user(&pool).await;

    let token = mint_token_with_role(user_id, Some(org_b), "platform_admin");
    let resp = app
        .execute(authed_request(
            Method::POST,
            &availability_sync_uri(org_a),
            &token,
            None,
        ))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::ACCEPTED,
        "platform admin must pass the IDOR + role gates and enqueue the job; got {}: {}",
        resp.status,
        resp.text()
    );

    let job_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM background_jobs WHERE org_id = $1")
            .bind(org_a)
            .fetch_one(&pool)
            .await
            .expect("count jobs");
    assert_eq!(job_count, 1, "exactly one job must be queued for org_a");
}
