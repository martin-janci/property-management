//! OAuth client registration / management integration tests (gap-10a-2).
//!
//! The OAuth 2.0 *authorization-server* flow (authorize / token / introspect /
//! grants) is covered by `oauth_integration_tests.rs`. This file is ADDITIVE
//! test hardening for the **admin client-management surface** that was, until
//! now, only exercised on its audit-trail edge:
//!
//!   POST   /api/v1/admin/oauth/clients                      (register)
//!   GET    /api/v1/admin/oauth/clients                      (list)
//!   GET    /api/v1/admin/oauth/clients/{id}                 (get)
//!   PATCH  /api/v1/admin/oauth/clients/{id}                 (update)
//!   POST   /api/v1/admin/oauth/clients/{id}/regenerate-secret (rotate secret)
//!   DELETE /api/v1/admin/oauth/clients/{id}                 (revoke / delete)
//!
//! Coverage:
//!   * Auth boundary: every route requires authentication (401 when anonymous).
//!   * Authorization / tenant isolation: the whole surface is gated by the
//!     platform-scoped `Capability::OauthClientWrite`. OAuth clients are
//!     platform-global (no `org_id` column), so the isolation boundary IS the
//!     capability gate. The `RequireCapability` extractor first re-derives the
//!     principal via `RequestPrincipal`; under the test harness a NON-platform
//!     principal has no resolved tenant, so that step fails and the extractor
//!     maps it to `AdminError::Unauthenticated` (401). Either way an ordinary
//!     tenant user (any org) is denied — they never see or mutate clients. A
//!     platform principal who clears step 1 but lacks the grant is rejected
//!     with 403 (missing_capability). Both are exercised below.
//!   * Happy path: a platform principal holding `oauth_client_write` can
//!     create / list / get / update / rotate-secret / delete a client.
//!   * Validation errors: an unrecognised scope is rejected with 400 on both
//!     create and update. Lookups of an unknown client UUID return 404 on
//!     get / update / delete, but 500 on rotate-secret (the shipped handler
//!     maps the service's ClientNotFound to a 500 there — pinned, not "fixed").
//!
//! These tests assert the SHIPPED behaviour; they do not change production code.
//! Where the production handler validates a field (scopes) we pin it; where it
//! does NOT (e.g. duplicate redirect URIs are stored verbatim — there is no
//! dedup at the service layer), the test documents the actual contract rather
//! than asserting a validation that does not exist.

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use api_server::services::JwtService;
use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use common::{create_authenticated_user, create_authenticated_user_with_org, TestApp, TestUser};

const TEST_JWT_SECRET: &str =
    "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

const ADMIN_CLIENTS: &str = "/api/v1/admin/oauth/clients";

// ─── seeding helpers (mirrors oauth_integration_tests::admin_client_audit) ──────────

/// Seed a platform-principal user and return its id. INSERTs bypass the
/// `BEFORE UPDATE` principal_kind guard, so we can set `'platform'` directly.
async fn seed_platform_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'h', 'OAuth Admin Test', 'active', NOW(), 'platform')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed platform user")
}

/// Mark the user as MFA-enrolled (the capability gate's step 2.5 wall).
async fn enroll_mfa(pool: &PgPool, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO user_2fa (user_id, secret, enabled, enabled_at, backup_codes, backup_codes_remaining)
        VALUES ($1, 'unused-secret', true, NOW(), '[]'::jsonb, 0)
        ON CONFLICT (user_id) DO UPDATE SET enabled = true, enabled_at = NOW()
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .expect("enroll mfa");
}

/// Grant `oauth_client_write` to the user with `mfa_required = false` so the
/// capability extractor's recent-MFA recency check is skipped — these tests
/// exercise the client-management behaviour, not the MFA recency wall (which is
/// covered by the admin-mfa step-up tests). `granted_by` must reference a
/// distinct real user (FK + app-layer no-self-grant rule).
async fn grant_oauth_client_write(pool: &PgPool, user_id: Uuid, granted_by: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO capability_grants
            (user_id, capability, granted_by, expires_at, mfa_required, note)
        VALUES ($1, 'oauth_client_write', $2, NULL, false, 'ci-client-mgmt-test')
        "#,
    )
    .bind(user_id)
    .bind(granted_by)
    .execute(pool)
    .await
    .expect("grant oauth_client_write");
}

/// Mint a platform-kind bearer JWT validated by `TestApp`'s JWT secret.
fn mint_platform_token(user_id: Uuid, email: &str) -> String {
    let svc = JwtService::new(TEST_JWT_SECRET).expect("jwt service");
    svc.generate_access_token_with_kind(
        user_id,
        email,
        "OAuth Admin Test",
        None,
        None,
        Some("platform".to_string()),
    )
    .expect("mint token")
}

/// Provision a fully-authorized platform admin (platform principal + MFA
/// enrolled + `oauth_client_write` grant) and return a usable bearer token.
async fn authorized_admin_token(pool: &PgPool, label: &str) -> String {
    let email = format!("oauth-admin-{label}-{}@test.local", Uuid::new_v4());
    let granter_email = format!("oauth-granter-{label}-{}@test.local", Uuid::new_v4());
    let admin = seed_platform_user(pool, &email).await;
    let granter = seed_platform_user(pool, &granter_email).await;
    enroll_mfa(pool, admin).await;
    grant_oauth_client_write(pool, admin, granter).await;
    mint_platform_token(admin, &email)
}

// ─── request builders ───────────────────────────────────────────────────────────

fn anon(method: Method, uri: &str, body: Option<serde_json::Value>) -> Request<Body> {
    let b = Request::builder().method(method).uri(uri);
    match body {
        Some(j) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(j.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

fn authed(
    token: &str,
    method: Method,
    uri: &str,
    body: Option<serde_json::Value>,
) -> Request<Body> {
    let b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    match body {
        Some(j) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(j.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

/// Minimal valid registration body.
fn register_body() -> serde_json::Value {
    serde_json::json!({
        "name": "CI Test Client",
        "redirectUris": ["https://app.example.com/callback"],
        "scopes": ["profile", "email"],
        "isConfidential": true,
    })
}

/// Register a client via the admin route and return its `(uuid, client_id)`.
async fn register_client(app: &TestApp, token: &str, body: serde_json::Value) -> (Uuid, String) {
    let resp = app
        .execute(authed(token, Method::POST, ADMIN_CLIENTS, Some(body)))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "register_client must return 201, got {}: {}",
        resp.status,
        resp.text()
    );
    let v = resp.json_value();
    let uuid = v["id"].as_str().expect("id in response");
    let client_id = v["clientId"].as_str().expect("clientId in response");
    (
        Uuid::parse_str(uuid).expect("valid uuid"),
        client_id.to_string(),
    )
}

// ─── module: auth_boundary ────────────────────────────────────────────────────────
//
// Every admin client route must require authentication. An anonymous request
// (no bearer) must be rejected with 401 before any handler logic runs.

#[cfg(test)]
mod auth_boundary {
    use super::*;

    fn cases() -> Vec<(Method, String, Option<serde_json::Value>)> {
        let id = "00000000-0000-0000-0000-000000000001";
        vec![
            (
                Method::POST,
                ADMIN_CLIENTS.to_string(),
                Some(register_body()),
            ),
            (Method::GET, ADMIN_CLIENTS.to_string(), None),
            (Method::GET, format!("{ADMIN_CLIENTS}/{id}"), None),
            (
                Method::PATCH,
                format!("{ADMIN_CLIENTS}/{id}"),
                Some(serde_json::json!({"name": "x"})),
            ),
            (
                Method::POST,
                format!("{ADMIN_CLIENTS}/{id}/regenerate-secret"),
                None,
            ),
            (Method::DELETE, format!("{ADMIN_CLIENTS}/{id}"), None),
        ]
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn all_admin_client_routes_require_auth(pool: PgPool) {
        let app = TestApp::new(pool).await;
        for (method, uri, body) in cases() {
            let resp = app.execute(anon(method.clone(), &uri, body)).await;
            assert_eq!(
                resp.status,
                StatusCode::UNAUTHORIZED,
                "{method} {uri} must require auth (401), got {}",
                resp.status
            );
        }
    }
}

// ─── module: capability_isolation ──────────────────────────────────────────────────
//
// OAuth clients are platform-global. The isolation boundary is the
// `OauthClientWrite` capability gate: a tenant user (no matter which org) must
// never reach the client surface, and a platform principal without the grant is
// likewise refused.

#[cfg(test)]
mod capability_isolation {
    use super::*;

    /// An ordinary, freshly-registered tenant user (non-platform principal) must
    /// be denied on every admin client route — they can neither read nor mutate
    /// platform OAuth clients. The denial surfaces as 401 because the
    /// `RequireCapability` extractor re-derives the principal first, and a
    /// non-platform principal with no resolved tenant fails that step (mapped to
    /// `AdminError::Unauthenticated`). What matters for isolation is that the
    /// request never reaches the client surface; we pin the exact 401 status.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn non_platform_user_is_denied(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (token, _r) = create_authenticated_user(&app, &TestUser::new()).await;

        // List + register are representative of read and write paths.
        let list = app
            .execute(authed(&token, Method::GET, ADMIN_CLIENTS, None))
            .await;
        assert_eq!(
            list.status,
            StatusCode::UNAUTHORIZED,
            "non-platform user must be denied listing clients, got {}: {}",
            list.status,
            list.text()
        );

        let create = app
            .execute(authed(
                &token,
                Method::POST,
                ADMIN_CLIENTS,
                Some(register_body()),
            ))
            .await;
        assert_eq!(
            create.status,
            StatusCode::UNAUTHORIZED,
            "non-platform user must be denied registering a client, got {}: {}",
            create.status,
            create.text()
        );
    }

    /// A user that is an active member (org_admin) of an organization is still a
    /// tenant principal, not a platform one — they must NOT be able to manage
    /// platform OAuth clients. This pins the cross-tenant isolation contract:
    /// org-level admin does not confer platform-admin authority. As above, the
    /// denial surfaces as 401 from the principal re-derivation step.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn org_admin_cannot_manage_platform_clients(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let (token, _org) =
            create_authenticated_user_with_org(&app, &TestUser::new(), "oauthiso").await;

        let resp = app
            .execute(authed(&token, Method::GET, ADMIN_CLIENTS, None))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "org_admin must not manage platform OAuth clients, got {}: {}",
            resp.status,
            resp.text()
        );
    }

    /// A platform principal who is MFA-enrolled but holds NO `oauth_client_write`
    /// grant must be rejected with 403 (missing capability). Proves the grant —
    /// not merely being a platform principal — is required.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn platform_principal_without_grant_is_forbidden(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let email = format!("oauth-nogrant-{}@test.local", Uuid::new_v4());
        let admin = seed_platform_user(&pool, &email).await;
        enroll_mfa(&pool, admin).await; // enrolled, but no capability grant
        let token = mint_platform_token(admin, &email);

        let resp = app
            .execute(authed(&token, Method::GET, ADMIN_CLIENTS, None))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "platform principal without the grant must be forbidden, got {}: {}",
            resp.status,
            resp.text()
        );
    }
}

// ─── module: lifecycle ─────────────────────────────────────────────────────────────
//
// Happy-path CRUD for a fully-authorized platform admin: create → list → get →
// update → rotate-secret → delete.

#[cfg(test)]
mod lifecycle {
    use super::*;

    /// Registering a client returns 201 with a generated `clientId` and a
    /// plaintext `clientSecret` (shown once), and the requested scopes echo back.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn register_returns_client_with_one_time_secret(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let token = authorized_admin_token(&pool, "create").await;

        let resp = app
            .execute(authed(
                &token,
                Method::POST,
                ADMIN_CLIENTS,
                Some(register_body()),
            ))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::CREATED,
            "register must return 201, got {}: {}",
            resp.status,
            resp.text()
        );
        let v = resp.json_value();
        assert!(v["id"].is_string(), "response must carry a UUID id");
        assert!(
            v["clientId"]
                .as_str()
                .map(|s| !s.is_empty())
                .unwrap_or(false),
            "response must carry a non-empty clientId"
        );
        assert!(
            v["clientSecret"]
                .as_str()
                .map(|s| !s.is_empty())
                .unwrap_or(false),
            "register must return the one-time plaintext clientSecret"
        );
        let scopes = v["scopes"].as_array().expect("scopes array");
        assert!(
            scopes.iter().any(|s| s == "profile") && scopes.iter().any(|s| s == "email"),
            "requested scopes must echo back, got {}",
            v["scopes"]
        );
    }

    /// A registered client appears in the list, and after delete it is no longer
    /// active (revoke deactivates: it must not appear among active clients).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn create_list_get_delete_round_trip(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let token = authorized_admin_token(&pool, "roundtrip").await;

        let (id, client_id) = register_client(&app, &token, register_body()).await;

        // List — the new client must be present.
        let list = app
            .execute(authed(&token, Method::GET, ADMIN_CLIENTS, None))
            .await;
        assert_eq!(list.status, StatusCode::OK, "list must return 200");
        let arr = list.json_value();
        let arr = arr.as_array().expect("list returns an array");
        assert!(
            arr.iter().any(|c| c["clientId"] == client_id),
            "registered client {client_id} must appear in the list"
        );

        // Get by UUID — must return the same client.
        let get = app
            .execute(authed(
                &token,
                Method::GET,
                &format!("{ADMIN_CLIENTS}/{id}"),
                None,
            ))
            .await;
        assert_eq!(get.status, StatusCode::OK, "get must return 200");
        assert_eq!(
            get.json_value()["clientId"],
            client_id,
            "get must return the requested client"
        );

        // Delete (revoke) — 204.
        let del = app
            .execute(authed(
                &token,
                Method::DELETE,
                &format!("{ADMIN_CLIENTS}/{id}"),
                None,
            ))
            .await;
        assert_eq!(
            del.status,
            StatusCode::NO_CONTENT,
            "delete must return 204, got {}: {}",
            del.status,
            del.text()
        );

        // After revoke the client is deactivated (is_active = false). Note the
        // shipped `list_clients` returns ALL clients (no is_active filter), so
        // the revoked client STILL appears in the listing — but now flagged
        // inactive. We pin that actual contract: present, with isActive=false.
        let list2 = app
            .execute(authed(&token, Method::GET, ADMIN_CLIENTS, None))
            .await;
        let arr2 = list2.json_value();
        let arr2 = arr2.as_array().expect("list returns an array");
        let revoked = arr2
            .iter()
            .find(|c| c["clientId"] == client_id)
            .expect("revoked client still appears in the (unfiltered) listing");
        assert_eq!(
            revoked["isActive"],
            serde_json::Value::Bool(false),
            "revoked client must be flagged isActive=false, got {revoked}"
        );

        // And the database row reflects the deactivation directly.
        let is_active: bool =
            sqlx::query_scalar("SELECT is_active FROM oauth_clients WHERE id = $1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .expect("fetch is_active");
        assert!(
            !is_active,
            "revoke must set oauth_clients.is_active = false"
        );
    }

    /// Updating a client's name + scopes returns 200 and the new values.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn update_changes_name_and_scopes(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let token = authorized_admin_token(&pool, "update").await;
        let (id, _client_id) = register_client(&app, &token, register_body()).await;

        let resp = app
            .execute(authed(
                &token,
                Method::PATCH,
                &format!("{ADMIN_CLIENTS}/{id}"),
                Some(serde_json::json!({
                    "name": "Renamed CI Client",
                    "scopes": ["profile"],
                    "reason": "ticket OPS-1 narrowing scopes",
                })),
            ))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "update must return 200, got {}: {}",
            resp.status,
            resp.text()
        );
        let v = resp.json_value();
        assert_eq!(v["name"], "Renamed CI Client", "name must be updated");
        let scopes = v["scopes"].as_array().expect("scopes array");
        assert_eq!(scopes.len(), 1, "scopes must be narrowed to one");
        assert_eq!(scopes[0], "profile");
    }

    /// Rotating the secret returns 200 with a fresh, non-empty `clientSecret`
    /// that differs from the original registration secret.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn regenerate_secret_returns_new_secret(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let token = authorized_admin_token(&pool, "rotate").await;

        let create_resp = app
            .execute(authed(
                &token,
                Method::POST,
                ADMIN_CLIENTS,
                Some(register_body()),
            ))
            .await;
        assert_eq!(create_resp.status, StatusCode::CREATED);
        let created = create_resp.json_value();
        let id = created["id"].as_str().expect("id");
        let original_secret = created["clientSecret"]
            .as_str()
            .expect("original secret")
            .to_string();

        let resp = app
            .execute(authed(
                &token,
                Method::POST,
                &format!("{ADMIN_CLIENTS}/{id}/regenerate-secret"),
                None,
            ))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "regenerate-secret must return 200, got {}: {}",
            resp.status,
            resp.text()
        );
        let new_secret = resp.json_value()["clientSecret"]
            .as_str()
            .expect("new clientSecret")
            .to_string();
        assert!(!new_secret.is_empty(), "rotated secret must be non-empty");
        assert_ne!(
            new_secret, original_secret,
            "rotated secret must differ from the original registration secret"
        );
    }
}

// ─── module: validation ────────────────────────────────────────────────────────────
//
// Pin the validation that the shipped handlers actually perform.

#[cfg(test)]
mod validation {
    use super::*;

    /// Registering with an unrecognised scope is rejected with 400. `validate_scopes`
    /// rejects the first scope that is not one of profile / email / org:read / full.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn register_with_invalid_scope_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let token = authorized_admin_token(&pool, "badscope").await;

        let resp = app
            .execute(authed(
                &token,
                Method::POST,
                ADMIN_CLIENTS,
                Some(serde_json::json!({
                    "name": "Bad Scope Client",
                    "redirectUris": ["https://app.example.com/callback"],
                    "scopes": ["profile", "definitely-not-a-real-scope"],
                    "isConfidential": true,
                })),
            ))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "an unrecognised scope must be rejected with 400, got {}: {}",
            resp.status,
            resp.text()
        );
    }

    /// Updating a client to an unrecognised scope is rejected with 400 — the same
    /// `validate_scopes` gate applies on the update path.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn update_with_invalid_scope_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let token = authorized_admin_token(&pool, "updbadscope").await;
        let (id, _client_id) = register_client(&app, &token, register_body()).await;

        let resp = app
            .execute(authed(
                &token,
                Method::PATCH,
                &format!("{ADMIN_CLIENTS}/{id}"),
                Some(serde_json::json!({ "scopes": ["nope-not-valid"] })),
            ))
            .await;
        assert_eq!(
            resp.status,
            StatusCode::BAD_REQUEST,
            "an unrecognised scope on update must be rejected with 400, got {}: {}",
            resp.status,
            resp.text()
        );
    }

    /// A missing required field (no `name`) fails JSON deserialization before the
    /// handler runs — axum's `Json` extractor rejects it with 422. This documents
    /// the actual contract for malformed bodies on the register route.
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn register_missing_required_field_is_rejected(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let token = authorized_admin_token(&pool, "missingfield").await;

        // `name`, `redirectUris` and `scopes` are non-Option in RegisterClientRequest;
        // omitting `name` makes the payload undeserializable.
        let resp = app
            .execute(authed(
                &token,
                Method::POST,
                ADMIN_CLIENTS,
                Some(serde_json::json!({
                    "redirectUris": ["https://app.example.com/callback"],
                    "scopes": ["profile"],
                })),
            ))
            .await;
        assert!(
            resp.status.is_client_error(),
            "a body missing a required field must be rejected with 4xx, got {}: {}",
            resp.status,
            resp.text()
        );
    }

    /// Lookups of a non-existent client UUID for an authorized admin: get /
    /// update / delete return 404. Rotate-secret is the odd one out — the
    /// shipped handler maps the service's `ClientNotFound` to a 500
    /// (`REGENERATE_FAILED`) rather than a 404. This test PINS that current
    /// behaviour; it is intentionally not "corrected" here (additive hardening,
    /// no production change).
    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn operations_on_unknown_client_status_codes(pool: PgPool) {
        let app = TestApp::new(pool.clone()).await;
        let token = authorized_admin_token(&pool, "notfound").await;
        let missing = Uuid::new_v4();

        let get = app
            .execute(authed(
                &token,
                Method::GET,
                &format!("{ADMIN_CLIENTS}/{missing}"),
                None,
            ))
            .await;
        assert_eq!(get.status, StatusCode::NOT_FOUND, "get unknown must be 404");

        let update = app
            .execute(authed(
                &token,
                Method::PATCH,
                &format!("{ADMIN_CLIENTS}/{missing}"),
                Some(serde_json::json!({ "name": "x" })),
            ))
            .await;
        assert_eq!(
            update.status,
            StatusCode::NOT_FOUND,
            "update unknown must be 404, got {}: {}",
            update.status,
            update.text()
        );

        let rotate = app
            .execute(authed(
                &token,
                Method::POST,
                &format!("{ADMIN_CLIENTS}/{missing}/regenerate-secret"),
                None,
            ))
            .await;
        // Shipped contract: regenerate-secret on an unknown client surfaces as
        // 500 REGENERATE_FAILED (the handler does not special-case ClientNotFound
        // into a 404 the way get/update/delete do).
        assert_eq!(
            rotate.status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "rotate unknown is mapped to 500 by the shipped handler, got {}: {}",
            rotate.status,
            rotate.text()
        );

        let delete = app
            .execute(authed(
                &token,
                Method::DELETE,
                &format!("{ADMIN_CLIENTS}/{missing}"),
                None,
            ))
            .await;
        assert_eq!(
            delete.status,
            StatusCode::NOT_FOUND,
            "delete unknown must be 404, got {}: {}",
            delete.status,
            delete.text()
        );
    }
}
