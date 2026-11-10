//! Real-JWT regression tests for the integration-route cross-org IDOR fix
//! (closes #765).
//!
//! # Background
//!
//! The org-scoped Airbnb / Booking.com integration routes in
//! `routes/integrations/install.rs` (`get_airbnb_status`, `connect_airbnb`,
//! `sync_airbnb`, `disconnect_airbnb`, and the Booking.com equivalents)
//! extracted the authenticated caller (`AuthUser`) purely for logging and
//! then operated on the `{org_id}` taken from the URL path WITHOUT ever
//! verifying that the caller belonged to that organization. Any authenticated
//! user could therefore read another org's connection status, kick off a
//! sync, or disconnect another org's Airbnb / Booking.com integration by
//! guessing the org UUID — a textbook IDOR.
//!
//! The Airbnb OAuth callback (`routes/integrations/oauth.rs`) had a related
//! hole: it matched the org id embedded in the (client-visible, forgeable)
//! `state` parameter against the path, but never checked org membership, so a
//! caller could bind Airbnb tokens to an arbitrary org.
//!
//! # The fix
//!
//! Every org-scoped handler now calls `verify_org_access(&state,
//! auth.user_id, path.org_id)` — the same membership guard already used by
//! the calendar / accounting / e-signature handlers in `sync.rs` and the
//! webhook handlers in `webhook.rs`. A caller who is not a member of the
//! target org receives `403 FORBIDDEN`.
//!
//! # What these tests verify
//!
//! Using a *real* JWT (minted via `/api/v1/auth/register` +
//! `/api/v1/auth/login`):
//!
//!   - A user who is NOT a member of Org A is rejected with `403` on every
//!     org-scoped Airbnb / Booking.com route (the IDOR is closed).
//!   - A legitimate member of Org A passes the guard (NOT `403`) — proving
//!     the fix does not lock out authorized callers.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user, seed_membership, TestApp, TestUser};

// Must match `TestConfig::default().jwt_secret` (see
// `airbnb_connections_routes_tests.rs`, which uses the same constant). Lets the
// manager-gate tests mint a JWT whose `role` claim deliberately *disagrees* with
// the caller's DB `role_type`, proving the gate reads the DB, not the token.
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

/// Mirror of `api_core::extractors::auth::Claims`.
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

/// Mint an access token whose `role` claim is `manager` (the JWT-claim role the
/// pre-#1787 handlers trusted), regardless of the caller's real DB membership.
fn mint_manager_claim_token(user_id: Uuid, tenant_id: Uuid) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(tenant_id),
        role: Some("manager".to_string()),
        email: "booking-gate-test@test.local".to_string(),
        name: "Booking Gate Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint access token")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'hash', 'Booking Gate User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

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
    .bind(format!("Integrations IDOR {slug}"))
    .bind(format!("integrations-idor-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@integrations-idor.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("fetch user id")
}

fn authed_req(
    method: Method,
    uri: &str,
    access_token: &str,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {}", access_token));
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let payload = body
        .map(|v| Body::from(v.to_string()))
        .unwrap_or_else(Body::empty);
    b.body(payload).expect("build request")
}

/// The org-scoped Airbnb / Booking.com routes a non-member must be barred
/// from. Each tuple is (method, path-suffix, optional-body).
fn org_scoped_routes(org_id: Uuid) -> Vec<(Method, String, Option<serde_json::Value>)> {
    let base = format!("/api/v1/integrations/organizations/{org_id}");
    vec![
        (Method::GET, format!("{base}/airbnb/status"), None),
        (
            Method::POST,
            format!("{base}/airbnb/connect"),
            Some(serde_json::json!({})),
        ),
        (Method::POST, format!("{base}/airbnb/sync"), None),
        (Method::DELETE, format!("{base}/airbnb"), None),
        (Method::GET, format!("{base}/booking/status"), None),
        (
            Method::POST,
            format!("{base}/booking/connect"),
            Some(serde_json::json!({
                "hotel_id": "H1",
                "username": "u",
                "password": "p"
            })),
        ),
        (Method::POST, format!("{base}/booking/sync"), None),
        (Method::DELETE, format!("{base}/booking"), None),
    ]
}

// ---------------------------------------------------------------------------
// Test 1 — a non-member is rejected with 403 on every org-scoped route
// ---------------------------------------------------------------------------

/// #765: a real authenticated user who is NOT a member of Org A must be
/// rejected with `403 FORBIDDEN` on every org-scoped Airbnb / Booking.com
/// integration route. Before the fix these handlers ignored the caller's
/// org entirely and proceeded against Org A's data.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_member_is_forbidden_on_org_integration_routes(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "victim").await;
    let org_b = seed_org(&pool, "attacker").await;

    // Attacker: a real authenticated user who belongs to Org B only.
    let attacker = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &attacker).await;
    let attacker_id = user_id_for(&pool, &attacker.email).await;
    seed_membership(&pool, org_b, attacker_id, "manager").await;

    for (method, uri, body) in org_scoped_routes(org_a) {
        let response = app
            .execute(authed_req(method.clone(), &uri, &access_token, body))
            .await;

        assert_eq!(
            response.status,
            StatusCode::FORBIDDEN,
            "#765: {method} {uri} from a non-member must return 403, got {} body={}",
            response.status,
            response.text()
        );
    }
}

// ---------------------------------------------------------------------------
// Test 2 — a legitimate org member passes the org-access guard
// ---------------------------------------------------------------------------

/// #765 (no-lockout sanity): a real authenticated member of Org A must NOT
/// be rejected with `403` by the new org-access guard. `get_airbnb_status`
/// returns `200` with `connected: false` when no connection exists, so we can
/// assert the exact success status for the read path.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn member_passes_org_access_guard(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_a = seed_org(&pool, "member-org").await;

    // A real authenticated user who IS a member of Org A.
    let member = TestUser::new();
    let (access_token, _) = create_authenticated_user(&app, &member).await;
    let member_id = user_id_for(&pool, &member.email).await;
    seed_membership(&pool, org_a, member_id, "manager").await;

    let uri = format!("/api/v1/integrations/organizations/{org_a}/airbnb/status");
    let response = app
        .execute(authed_req(Method::GET, &uri, &access_token, None))
        .await;

    assert_ne!(
        response.status,
        StatusCode::FORBIDDEN,
        "#765: a legitimate org member must NOT be blocked by the org-access guard; body={}",
        response.text()
    );
    assert_eq!(
        response.status,
        StatusCode::OK,
        "#765: airbnb/status for a member with no connection should be 200; body={}",
        response.text()
    );
    let json = response.json_value();
    assert_eq!(
        json["connected"], false,
        "expected connected=false for an org with no Airbnb connection"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — Booking.com manager gate reads the DB role, not the JWT claim
// ---------------------------------------------------------------------------
//
// #1787: `get_booking_conflicts` and `push_booking_listing`
// (`routes/integrations/booking_channel.rs`) previously authorized off the JWT
// `role`/`tenant_id` claim with a hand-rolled `matches!(role, …)`. A user who is
// only a `resident` in the path org — but carries a `manager` role claim in
// their token — would have been admitted. After the fix both handlers run
// `verify_org_access` + `verify_manager_role_in_org`, which read
// `organization_members.role_type` for the PATH org, so the resident is rejected
// with `403` despite the `manager` JWT claim.
//
// These tests mint a token whose `role` claim is `manager` while the DB
// membership is `resident`: on the pre-#1787 (JWT-claim) code they would PASS the
// gate (200 / 404), so the assertion FAILS; on the DB-backed code they return
// `403`, so the assertion PASSES — the IG3 failing-on-old-code property.

/// `GET /booking/conflicts`: a `resident` member is rejected with `403` even
/// though the JWT carries a `manager` role claim (gate reads the DB role).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn booking_conflicts_manager_gate_rejects_resident(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_x = seed_org(&pool, "booking-resident").await;
    let user_id = seed_user(&pool, "booking-conflicts-resident@test.local").await;
    seed_membership(&pool, org_x, user_id, "resident").await;

    let token = mint_manager_claim_token(user_id, org_x);
    let uri = format!("/api/v1/integrations/organizations/{org_x}/booking/conflicts");
    let response = app
        .execute(authed_req(Method::GET, &uri, &token, None))
        .await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "#1787: a resident member must be 403 on booking/conflicts despite a manager JWT claim; got {}: {}",
        response.status,
        response.text()
    );
}

/// `POST /booking/listing-push` (the byte-identical write gate): a `resident`
/// member is rejected with `403` despite a `manager` JWT claim. The gate fires
/// before payload validation, so a minimal body still 403s.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn booking_listing_push_manager_gate_rejects_resident(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_x = seed_org(&pool, "booking-push-resident").await;
    let user_id = seed_user(&pool, "booking-push-resident@test.local").await;
    seed_membership(&pool, org_x, user_id, "resident").await;

    let token = mint_manager_claim_token(user_id, org_x);
    let uri = format!("/api/v1/integrations/organizations/{org_x}/booking/listing-push");
    let body = serde_json::json!({
        "unit_id": Uuid::new_v4(),
        "room_type_id": "RT-1",
        "availability": [],
        "rates": [],
    });
    let response = app
        .execute(authed_req(Method::POST, &uri, &token, Some(body)))
        .await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "#1787: a resident member must be 403 on booking/listing-push despite a manager JWT claim; got {}: {}",
        response.status,
        response.text()
    );
}

/// No-lockout sanity: a `manager` DB member passes the gate on
/// `GET /booking/conflicts` (NOT `403`). With no Booking.com connection the
/// handler returns `404`, so we assert only that the manager gate does not block.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn booking_conflicts_manager_member_passes_gate(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_x = seed_org(&pool, "booking-manager").await;
    let user_id = seed_user(&pool, "booking-conflicts-manager@test.local").await;
    seed_membership(&pool, org_x, user_id, "manager").await;

    let token = mint_manager_claim_token(user_id, org_x);
    let uri = format!("/api/v1/integrations/organizations/{org_x}/booking/conflicts");
    let response = app
        .execute(authed_req(Method::GET, &uri, &token, None))
        .await;

    assert_ne!(
        response.status,
        StatusCode::FORBIDDEN,
        "#1787: a legitimate manager member must NOT be blocked by the manager gate; got {}: {}",
        response.status,
        response.text()
    );
}

// ---------------------------------------------------------------------------
// Test 4 — direct-connect credential writers on the install surface enforce
// the manager gate (install.rs `connect_booking` / `disconnect_booking` /
// `disconnect_airbnb`, and oauth.rs `airbnb_oauth_callback`).
// ---------------------------------------------------------------------------
//
// SECURITY (install.rs:907 non-manager-hijack): these direct-connect handlers
// previously gated only on `verify_org_access`, which passes for ANY org member
// — including a plain resident. A resident could therefore POST attacker-owned
// Booking.com credentials (persisted as the org's active OTA integration) or
// `DELETE` the legitimate connection, mirroring the #1787 gap on the
// `booking_channel.rs` push surface. The fix adds
// `verify_manager_role_in_org` after `verify_org_access`, matching the sibling
// gated writers (`booking_token_exchange`, `direct_connect_airbnb`).
//
// Each test mints a token whose `role` claim is `manager` while the DB
// membership is `resident`: on the pre-fix code the handler admits the caller
// and proceeds past the gate (network/encryption error, 404, or 503 — never a
// `403`), so `assert_eq!(status, FORBIDDEN)` FAILS on old code and PASSES on the
// DB-backed manager gate — the IG3 failing-on-main property. The gate fires
// before any network call or DB lookup, so the resident rejection is
// deterministic and hermetic.

/// `POST /booking/connect`: a `resident` member is rejected with `403` even
/// though the JWT carries a `manager` role claim. Before the fix this stored
/// the caller's Booking.com credentials as the org's active connection.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connect_booking_manager_gate_rejects_resident(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_x = seed_org(&pool, "connect-booking-resident").await;
    let user_id = seed_user(&pool, "connect-booking-resident@test.local").await;
    seed_membership(&pool, org_x, user_id, "resident").await;

    let token = mint_manager_claim_token(user_id, org_x);
    let uri = format!("/api/v1/integrations/organizations/{org_x}/booking/connect");
    let body = serde_json::json!({
        "hotel_id": "ATTACKER-HOTEL",
        "username": "attacker",
        "password": "attacker-secret",
    });
    let response = app
        .execute(authed_req(Method::POST, &uri, &token, Some(body)))
        .await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "a resident must be 403 on booking/connect despite a manager JWT claim; got {}: {}",
        response.status,
        response.text()
    );
}

/// `POST /booking/connect`: EVERY non-manager `role_type` is rejected with
/// `403`, not merely the low-privilege `resident` covered above (refs #2821).
///
/// The manager gate derives its decision from `TenantRole::is_manager()`
/// (`sync.rs::verify_manager_role_in_org`), whose `false` set is broader than
/// `resident`: it includes the high-privilege `owner` (hierarchy level 60, well
/// above `resident`'s 30) and the deceptively-named `property_manager` — a
/// short-term-rental role whose *name* contains "manager" but which is NOT a
/// manager tier. This test pins that whole set so a future refactor of the gate
/// (e.g. a naive substring/`role_type LIKE '%manager%'` check, or one that only
/// special-cases `resident`) that let any of these hijack the org's OTA
/// credentials would fail here.
///
/// Each caller carries a `manager` JWT role claim (which the pre-#1787 handler
/// trusted) while its DB membership is the non-manager role under test, so the
/// gate must read the DB, not the token. The gate fires before any Booking.com
/// network call, so the rejection is deterministic and hermetic.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn connect_booking_manager_gate_rejects_all_non_manager_roles(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Non-manager `role_type`s (snake_case per `TenantRole`'s serde mapping).
    // `is_manager()` is `false` for all of these; the gate must 403 each one.
    let non_manager_roles = [
        "owner",
        "owner_delegate",
        "tenant",
        "property_manager",
        "real_estate_agent",
        "guest",
    ];

    for role in non_manager_roles {
        let org_x = seed_org(&pool, &format!("connect-booking-{role}")).await;
        let user_id = seed_user(&pool, &format!("connect-booking-{role}@test.local")).await;
        seed_membership(&pool, org_x, user_id, role).await;

        let token = mint_manager_claim_token(user_id, org_x);
        let uri = format!("/api/v1/integrations/organizations/{org_x}/booking/connect");
        let body = serde_json::json!({
            "hotel_id": "ATTACKER-HOTEL",
            "username": "attacker",
            "password": "attacker-secret",
        });
        let response = app
            .execute(authed_req(Method::POST, &uri, &token, Some(body)))
            .await;

        assert_eq!(
            response.status,
            StatusCode::FORBIDDEN,
            "a `{role}` member must be 403 on booking/connect despite a manager \
             JWT claim (non-manager role must not hijack OTA credentials); got {}: {}",
            response.status,
            response.text()
        );
    }
}

/// `DELETE /booking`: a `resident` member is rejected with `403` — they cannot
/// wipe the org's legitimate Booking.com connection.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disconnect_booking_manager_gate_rejects_resident(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_x = seed_org(&pool, "disconnect-booking-resident").await;
    let user_id = seed_user(&pool, "disconnect-booking-resident@test.local").await;
    seed_membership(&pool, org_x, user_id, "resident").await;

    let token = mint_manager_claim_token(user_id, org_x);
    let uri = format!("/api/v1/integrations/organizations/{org_x}/booking");
    let response = app
        .execute(authed_req(Method::DELETE, &uri, &token, None))
        .await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "a resident must be 403 on DELETE booking despite a manager JWT claim; got {}: {}",
        response.status,
        response.text()
    );
}

/// `DELETE /airbnb`: a `resident` member is rejected with `403` — they cannot
/// tear down the org's legitimate Airbnb connection.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disconnect_airbnb_manager_gate_rejects_resident(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_x = seed_org(&pool, "disconnect-airbnb-resident").await;
    let user_id = seed_user(&pool, "disconnect-airbnb-resident@test.local").await;
    seed_membership(&pool, org_x, user_id, "resident").await;

    let token = mint_manager_claim_token(user_id, org_x);
    let uri = format!("/api/v1/integrations/organizations/{org_x}/airbnb");
    let response = app
        .execute(authed_req(Method::DELETE, &uri, &token, None))
        .await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "a resident must be 403 on DELETE airbnb despite a manager JWT claim; got {}: {}",
        response.status,
        response.text()
    );
}

/// `GET /airbnb/callback`: a `resident` member is rejected with `403` before
/// any token exchange — they cannot bind Airbnb OAuth tokens to the org. The
/// state is well-formed (`{org_id}:{nonce}`) so it clears the CSRF/state checks
/// and reaches the membership + manager gate; with no state store wired the
/// consume path is `StoreUnavailable` (proceeds), so the manager gate is what
/// short-circuits.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn airbnb_oauth_callback_manager_gate_rejects_resident(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_x = seed_org(&pool, "airbnb-callback-resident").await;
    let user_id = seed_user(&pool, "airbnb-callback-resident@test.local").await;
    seed_membership(&pool, org_x, user_id, "resident").await;

    let token = mint_manager_claim_token(user_id, org_x);
    let state = format!("{org_x}:{}", Uuid::new_v4());
    let uri = format!(
        "/api/v1/integrations/organizations/{org_x}/airbnb/callback?code=attacker-code&state={state}"
    );
    let response = app
        .execute(authed_req(Method::GET, &uri, &token, None))
        .await;

    assert_eq!(
        response.status,
        StatusCode::FORBIDDEN,
        "a resident must be 403 on airbnb/callback despite a manager JWT claim; got {}: {}",
        response.status,
        response.text()
    );
}

/// No-lockout sanity for the install surface: a `manager` DB member passes the
/// `DELETE /booking` gate (NOT `403`). With no Booking.com connection the
/// handler returns `404`, so we assert only that the manager gate does not block.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn disconnect_booking_manager_member_passes_gate(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org_x = seed_org(&pool, "disconnect-booking-manager").await;
    let user_id = seed_user(&pool, "disconnect-booking-manager@test.local").await;
    seed_membership(&pool, org_x, user_id, "manager").await;

    let token = mint_manager_claim_token(user_id, org_x);
    let uri = format!("/api/v1/integrations/organizations/{org_x}/booking");
    let response = app
        .execute(authed_req(Method::DELETE, &uri, &token, None))
        .await;

    assert_ne!(
        response.status,
        StatusCode::FORBIDDEN,
        "a legitimate manager member must NOT be blocked by the manager gate on DELETE booking; got {}: {}",
        response.status,
        response.text()
    );
}
