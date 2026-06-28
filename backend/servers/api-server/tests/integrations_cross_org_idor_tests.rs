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

mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, seed_membership, TestApp, TestUser};

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
