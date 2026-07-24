//! Common test utilities for integration tests.
//!
//! Provides test helpers for:
//! - Test application builder with mock database
//! - Request helpers
//! - Response extractors
//! - Test fixtures for users and organizations
//!
//! Helpers here are shared across many integration-test binaries; each binary
//! `mod common;`-includes the whole module but uses only a subset, so unused
//! items are expected per-binary. Allow dead_code so the shared harness does
//! not trip the `-Dwarnings` test gate (BIT-345).
#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

/// Test configuration.
pub struct TestConfig {
    /// JWT secret for test tokens
    pub jwt_secret: String,
    /// Base URL for test server
    pub base_url: String,
    /// Email service in test mode
    pub email_enabled: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            // WARNING: Test-only JWT secret. This value must NEVER be used in production.
            // It is only used for integration testing with isolated test databases.
            jwt_secret: "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes"
                .to_string(),
            base_url: "http://localhost:8080".to_string(),
            email_enabled: false,
        }
    }
}

/// Test application wrapper.
pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
    pub config: TestConfig,
}

impl TestApp {
    /// Create a new test application with the given database pool.
    pub async fn new(pool: PgPool) -> Self {
        Self::with_config(pool, TestConfig::default()).await
    }

    /// Create a new test application with custom configuration.
    pub async fn with_config(pool: PgPool, config: TestConfig) -> Self {
        use api_server::services::{EmailService, JwtService};
        use api_server::state::AppState;

        // Tests need two env vars set before `AppState::new` runs:
        //
        // - `JWT_SECRET` — the `AuthUser` extractor reads it to validate
        //   bearer tokens. Without it, any request carrying a Bearer would
        //   surface as 500 instead of the expected 401/403.
        // - `RUST_ENV=development` — `TotpService::new` (called inside
        //   `AppState::new`) panics when `TOTP_ENCRYPTION_KEY` is absent
        //   unless we're in development mode.
        //
        // CI sets `RUST_ENV` via the workflow env block but not `JWT_SECRET`;
        // local `cargo test` typically has neither. We seed both here so
        // every test binary works in both environments without callers
        // remembering to export them.
        //
        // `set_var` is not thread-safe on glibc when called concurrently
        // with `getenv`, so we gate both writes behind a single `Once`.
        static TEST_ENV_ONCE: std::sync::Once = std::sync::Once::new();
        TEST_ENV_ONCE.call_once(|| {
            if std::env::var("JWT_SECRET").is_err() {
                std::env::set_var("JWT_SECRET", &config.jwt_secret);
            }
            if std::env::var("RUST_ENV").is_err() {
                std::env::set_var("RUST_ENV", "development");
            }
        });

        let email_service = EmailService::new(config.base_url.clone(), config.email_enabled);
        let jwt_service =
            JwtService::new(&config.jwt_secret).expect("Failed to create JWT service for tests");

        let tenant_cache = std::sync::Arc::new(api_core::middleware::TenantResolutionCache::new(
            300, 30, 10_000,
        ));
        // Phase 5.5: per-tenant rate limiter set. Mirrors the `TenantResolutionCache`
        // wiring above — a fresh per-test instance with the default 600 rpm
        // baseline. Integration tests that need a tighter quota can swap this
        // out by constructing a custom `HostTenantConfig` directly.
        let tenant_rate_limiters =
            std::sync::Arc::new(api_core::middleware::TenantRateLimiterSet::new());
        let state = AppState::new(
            pool.clone(),
            email_service,
            jwt_service,
            tenant_cache,
            tenant_rate_limiters,
        );

        // Build the router with all routes.
        // MockConnectInfo injects a synthetic SocketAddr so handlers that
        // use axum::extract::ConnectInfo don't return 500 when called via
        // oneshot (which does not call into_make_service_with_connect_info).
        let router =
            api_server::create_router(state).layer(axum::extract::connect_info::MockConnectInfo(
                std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
            ));

        Self {
            router,
            pool,
            config,
        }
    }

    /// Create a test application whose `AppState` carries a custom Airbnb
    /// integration configuration (issue #2240).
    ///
    /// Production loads `airbnb_config` from the environment. This lets a test
    /// inject non-empty credentials plus an `api_base` pointing at a `wiremock`
    /// stub so the `direct_connect_airbnb` success/write path can be driven
    /// network-free — without setting process-global `AIRBNB_*` env vars, which
    /// would race the sibling `#[sqlx::test]` cases in the same binary (notably
    /// the NOT_CONFIGURED test, which requires the credentials to stay empty).
    pub async fn with_airbnb_config(
        pool: PgPool,
        airbnb_config: api_server::state::AirbnbAppConfig,
    ) -> Self {
        use api_server::services::{EmailService, JwtService};
        use api_server::state::AppState;

        let config = TestConfig::default();

        // Seed JWT_SECRET / RUST_ENV exactly like `with_config` so bearer
        // tokens validate and `TotpService::new` doesn't panic.
        static TEST_ENV_ONCE: std::sync::Once = std::sync::Once::new();
        TEST_ENV_ONCE.call_once(|| {
            if std::env::var("JWT_SECRET").is_err() {
                std::env::set_var("JWT_SECRET", &config.jwt_secret);
            }
            if std::env::var("RUST_ENV").is_err() {
                std::env::set_var("RUST_ENV", "development");
            }
        });

        let email_service = EmailService::new(config.base_url.clone(), config.email_enabled);
        let jwt_service =
            JwtService::new(&config.jwt_secret).expect("Failed to create JWT service for tests");
        let tenant_cache = std::sync::Arc::new(api_core::middleware::TenantResolutionCache::new(
            300, 30, 10_000,
        ));
        let tenant_rate_limiters =
            std::sync::Arc::new(api_core::middleware::TenantRateLimiterSet::new());

        let state = AppState::new(
            pool.clone(),
            email_service,
            jwt_service,
            tenant_cache,
            tenant_rate_limiters,
        )
        .with_airbnb_config(airbnb_config);

        let router =
            api_server::create_router(state).layer(axum::extract::connect_info::MockConnectInfo(
                std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
            ));

        Self {
            router,
            pool,
            config,
        }
    }

    /// Create a test application whose `AppState` has a
    /// [`PreferenceEventRecorder`](api_server::state::PreferenceEventRecorder)
    /// installed, returning the app plus the recorder handle (issue #1376).
    ///
    /// This mirrors the `.with_redis(...)` builder the `#[ignore]`d S4 test
    /// uses, but instead of a live Redis it captures the `preference.updated`
    /// events the notification-preference handler would publish into an
    /// in-memory sink — so the publish contract is assertable in CI (which has
    /// no Redis daemon) without flakiness.
    pub async fn with_recording_pubsub(
        pool: PgPool,
    ) -> (Self, api_server::state::PreferenceEventRecorder) {
        use api_server::services::{EmailService, JwtService};
        use api_server::state::AppState;

        let config = TestConfig::default();

        // Seed JWT_SECRET / RUST_ENV exactly like `with_config` so bearer
        // tokens validate and `TotpService::new` doesn't panic.
        static TEST_ENV_ONCE: std::sync::Once = std::sync::Once::new();
        TEST_ENV_ONCE.call_once(|| {
            if std::env::var("JWT_SECRET").is_err() {
                std::env::set_var("JWT_SECRET", &config.jwt_secret);
            }
            if std::env::var("RUST_ENV").is_err() {
                std::env::set_var("RUST_ENV", "development");
            }
        });

        let email_service = EmailService::new(config.base_url.clone(), config.email_enabled);
        let jwt_service =
            JwtService::new(&config.jwt_secret).expect("Failed to create JWT service for tests");
        let tenant_cache = std::sync::Arc::new(api_core::middleware::TenantResolutionCache::new(
            300, 30, 10_000,
        ));
        let tenant_rate_limiters =
            std::sync::Arc::new(api_core::middleware::TenantRateLimiterSet::new());

        let (state, recorder) = AppState::new(
            pool.clone(),
            email_service,
            jwt_service,
            tenant_cache,
            tenant_rate_limiters,
        )
        .with_pref_event_recorder();

        let router =
            api_server::create_router(state).layer(axum::extract::connect_info::MockConnectInfo(
                std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
            ));

        (
            Self {
                router,
                pool,
                config,
            },
            recorder,
        )
    }

    /// Create a test application whose `AppState` has an in-memory OAuth
    /// `state` store installed, returning the app plus the store handle
    /// (issue #2203).
    ///
    /// Mirrors [`with_recording_pubsub`](Self::with_recording_pubsub): instead
    /// of a live Redis, the store is an in-process map with the same single-use
    /// consume semantics, so a test can seed a freshly-issued state and then
    /// drive the Airbnb-callback consume path (`Consumed` on first use,
    /// `Rejected → 400 INVALID_STATE` on replay) deterministically in CI.
    pub async fn with_oauth_state_store(
        pool: PgPool,
    ) -> (
        Self,
        api_server::routes::integrations::oauth_state::OAuthStateStore,
    ) {
        use api_server::services::{EmailService, JwtService};
        use api_server::state::AppState;

        let config = TestConfig::default();

        // Seed JWT_SECRET / RUST_ENV exactly like `with_config` so bearer
        // tokens validate and `TotpService::new` doesn't panic.
        static TEST_ENV_ONCE: std::sync::Once = std::sync::Once::new();
        TEST_ENV_ONCE.call_once(|| {
            if std::env::var("JWT_SECRET").is_err() {
                std::env::set_var("JWT_SECRET", &config.jwt_secret);
            }
            if std::env::var("RUST_ENV").is_err() {
                std::env::set_var("RUST_ENV", "development");
            }
        });

        let email_service = EmailService::new(config.base_url.clone(), config.email_enabled);
        let jwt_service =
            JwtService::new(&config.jwt_secret).expect("Failed to create JWT service for tests");
        let tenant_cache = std::sync::Arc::new(api_core::middleware::TenantResolutionCache::new(
            300, 30, 10_000,
        ));
        let tenant_rate_limiters =
            std::sync::Arc::new(api_core::middleware::TenantRateLimiterSet::new());

        let (state, store) = AppState::new(
            pool.clone(),
            email_service,
            jwt_service,
            tenant_cache,
            tenant_rate_limiters,
        )
        .with_oauth_state_store();

        let router =
            api_server::create_router(state).layer(axum::extract::connect_info::MockConnectInfo(
                std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
            ));

        (
            Self {
                router,
                pool,
                config,
            },
            store,
        )
    }

    /// Execute a request against the test application.
    pub async fn execute(&self, request: Request<Body>) -> TestResponse {
        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("Failed to execute request");

        TestResponse::from_response(response).await
    }

    /// Create a JSON POST request.
    pub fn post(&self, uri: &str) -> RequestBuilder {
        RequestBuilder::new(Method::POST, uri).with_router(self.router.clone())
    }

    /// Create a JSON GET request.
    pub fn get(&self, uri: &str) -> RequestBuilder {
        RequestBuilder::new(Method::GET, uri).with_router(self.router.clone())
    }

    /// Create a JSON PUT request.
    pub fn put(&self, uri: &str) -> RequestBuilder {
        RequestBuilder::new(Method::PUT, uri).with_router(self.router.clone())
    }

    /// Create a JSON DELETE request.
    pub fn delete(&self, uri: &str) -> RequestBuilder {
        RequestBuilder::new(Method::DELETE, uri).with_router(self.router.clone())
    }

    /// Create a JSON PATCH request.
    pub fn patch(&self, uri: &str) -> RequestBuilder {
        RequestBuilder::new(Method::PATCH, uri).with_router(self.router.clone())
    }

    /// Create an authenticated session for the given token and org (Story 1370).
    pub fn session(&self, token: String, org_id: Uuid) -> AuthenticatedSession<'_> {
        AuthenticatedSession::new(self, token, org_id)
    }
}

/// A session authenticated for a specific user and tenant (Story 1370).
///
/// Use this to build requests that automatically include both the Bearer
/// token and the `X-Tenant-ID` header, preventing "forgotten header" bugs in
/// RLS-aware integration tests.
pub struct AuthenticatedSession<'a> {
    app: &'a TestApp,
    token: String,
    org_id: Uuid,
}

impl<'a> AuthenticatedSession<'a> {
    pub fn new(app: &'a TestApp, token: String, org_id: Uuid) -> Self {
        Self { app, token, org_id }
    }

    /// Create a GET request with session credentials.
    pub fn get(&self, uri: &str) -> RequestBuilder {
        self.app.get(uri).bearer(&self.token).tenant(self.org_id)
    }

    /// Create a POST request with session credentials.
    pub fn post(&self, uri: &str) -> RequestBuilder {
        self.app.post(uri).bearer(&self.token).tenant(self.org_id)
    }

    /// Create a PUT request with session credentials.
    pub fn put(&self, uri: &str) -> RequestBuilder {
        self.app.put(uri).bearer(&self.token).tenant(self.org_id)
    }

    /// Create a PATCH request with session credentials.
    pub fn patch(&self, uri: &str) -> RequestBuilder {
        self.app.patch(uri).bearer(&self.token).tenant(self.org_id)
    }

    /// Create a DELETE request with session credentials.
    pub fn delete(&self, uri: &str) -> RequestBuilder {
        self.app.delete(uri).bearer(&self.token).tenant(self.org_id)
    }

    pub fn org_id(&self) -> Uuid {
        self.org_id
    }
}

/// Request builder for test requests.
pub struct RequestBuilder {
    method: Method,
    uri: String,
    body: Option<Value>,
    auth_token: Option<String>,
    headers: Vec<(String, String)>,
    router: Option<Router>,
}

impl RequestBuilder {
    pub fn new(method: Method, uri: &str) -> Self {
        Self {
            method,
            uri: uri.to_string(),
            body: None,
            auth_token: None,
            headers: Vec::new(),
            router: None,
        }
    }

    /// Bind the test router so this builder can `send()` itself.
    ///
    /// Set automatically by the `TestApp::{get,post,put,patch,delete}`
    /// helpers; callers that construct a builder directly via
    /// [`RequestBuilder::new`] keep using `app.execute(builder.build())`.
    fn with_router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    /// Set JSON body.
    pub fn json<T: Serialize>(mut self, body: T) -> Self {
        self.body = Some(serde_json::to_value(body).expect("Failed to serialize body"));
        self
    }

    /// Set authorization bearer token.
    pub fn bearer(mut self, token: &str) -> Self {
        self.auth_token = Some(token.to_string());
        self
    }

    /// Set the tenant scope via the `X-Tenant-ID` header (Story 1370).
    pub fn tenant(mut self, org_id: Uuid) -> Self {
        self.headers
            .push(("X-Tenant-ID".to_string(), org_id.to_string()));
        self
    }

    /// Add a custom header.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    /// Build the request.
    pub fn build(self) -> Request<Body> {
        let mut builder = Request::builder().method(self.method).uri(&self.uri);

        // Add content type if we have a body
        if self.body.is_some() {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
        }

        // Add auth header if present
        if let Some(token) = &self.auth_token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", token));
        }

        // Add custom headers
        for (name, value) in &self.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }

        // Build body
        let body = match self.body {
            Some(v) => Body::from(v.to_string()),
            None => Body::empty(),
        };

        builder.body(body).expect("Failed to build request")
    }

    /// Build and execute this request against the bound test router.
    ///
    /// Convenience equivalent of `app.execute(builder.build())`. The router
    /// is captured when the builder is created via a `TestApp` request helper
    /// (`app.get(...)`, `app.post(...)`, …) or an `AuthenticatedSession`.
    pub async fn send(self) -> TestResponse {
        let router = self
            .router
            .clone()
            .expect("send() requires a builder created via TestApp request helpers");
        let request = self.build();
        let response = router
            .oneshot(request)
            .await
            .expect("Failed to execute request");
        TestResponse::from_response(response).await
    }
}

/// Test response wrapper with helpers for extracting data.
pub struct TestResponse {
    pub status: StatusCode,
    pub headers: axum::http::HeaderMap,
    pub body: Vec<u8>,
}

impl TestResponse {
    /// Create from an axum response.
    pub async fn from_response(response: axum::response::Response) -> Self {
        let status = response.status();
        let headers = response.headers().clone();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body")
            .to_vec();

        Self {
            status,
            headers,
            body,
        }
    }

    /// Status code of the response.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Parse body as JSON.
    pub fn json<T: DeserializeOwned>(&self) -> T {
        serde_json::from_slice(&self.body).expect("Failed to parse JSON response")
    }

    /// Get body as JSON value.
    pub fn json_value(&self) -> Value {
        serde_json::from_slice(&self.body).expect("Failed to parse JSON response")
    }

    /// Get body as string.
    pub fn text(&self) -> String {
        String::from_utf8(self.body.clone()).expect("Response is not valid UTF-8")
    }

    /// Assert status code.
    pub fn assert_status(&self, expected: StatusCode) -> &Self {
        assert_eq!(
            self.status,
            expected,
            "Expected status {}, got {}. Body: {}",
            expected,
            self.status,
            self.text()
        );
        self
    }

    /// Assert JSON field exists.
    pub fn assert_json_field(&self, field: &str) -> &Self {
        let json = self.json_value();
        assert!(
            json.get(field).is_some(),
            "Expected field '{}' in response: {}",
            field,
            json
        );
        self
    }

    /// Assert JSON field value.
    pub fn assert_json_value(&self, field: &str, expected: &Value) -> &Self {
        let json = self.json_value();
        let actual = json.get(field);
        assert_eq!(
            actual,
            Some(expected),
            "Expected field '{}' to be {:?}, got {:?}",
            field,
            expected,
            actual
        );
        self
    }
}

/// Test fixture for creating test users.
pub struct TestUser {
    pub email: String,
    pub password: String,
    pub name: String,
}

impl TestUser {
    /// Create a new test user with random email.
    pub fn new() -> Self {
        let random_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        Self {
            email: format!("test-{}@example.com", random_id),
            password: "SecurePassword123!".to_string(),
            name: "Test User".to_string(),
        }
    }

    /// Create with specific email.
    pub fn with_email(email: &str) -> Self {
        Self {
            email: email.to_string(),
            ..Self::new()
        }
    }

    /// Get registration request body.
    pub fn registration_body(&self) -> Value {
        json!({
            "email": self.email,
            "password": self.password,
            "name": self.name
        })
    }

    /// Get login request body.
    pub fn login_body(&self) -> Value {
        json!({
            "email": self.email,
            "password": self.password
        })
    }
}

impl Default for TestUser {
    fn default() -> Self {
        Self::new()
    }
}

/// Test helper to clean up test data.
pub async fn cleanup_test_user(pool: &PgPool, email: &str) {
    if let Err(err) = sqlx::query("DELETE FROM users WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await
    {
        eprintln!("Warning: Failed to clean up test user '{}': {}", email, err);
    }
}

/// Test helper to verify user directly in database.
pub async fn verify_user_email(pool: &PgPool, email: &str) {
    sqlx::query("UPDATE users SET email_verified_at = NOW(), status = 'active' WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await
        .expect("Failed to verify user email");
}

/// Test helper to create a verified user and return auth tokens.
pub async fn create_authenticated_user(app: &TestApp, user: &TestUser) -> (String, String) {
    // Register user
    let register_req = app
        .post("/api/v1/auth/register")
        .json(user.registration_body())
        .build();
    let register_resp = app.execute(register_req).await;
    assert_eq!(register_resp.status, StatusCode::CREATED);

    // Verify email in database
    verify_user_email(&app.pool, &user.email).await;

    // Login to get tokens
    let login_req = app
        .post("/api/v1/auth/login")
        .json(user.login_body())
        .build();
    let login_resp = app.execute(login_req).await;
    assert_eq!(login_resp.status, StatusCode::OK);

    let json = login_resp.json_value();
    let access_token = json["accessToken"]
        .as_str()
        .expect("Missing accessToken")
        .to_string();
    let refresh_token = json["refreshToken"]
        .as_str()
        .expect("Missing refreshToken")
        .to_string();

    (access_token, refresh_token)
}

// ----------------------------------------------------------------------------
// Tenant-seeding fixtures.
//
// Tenant-scoped routes use `RlsConnection`, which resolves tenant context via
// `ValidatedTenantExtractor`. Under the `TestApp` harness (no
// `host_tenant_middleware`), that extractor requires BOTH an `X-Tenant-ID`
// header AND a matching active `organization_members` row — otherwise the
// request is rejected with 400 (missing header) or 403 (not a member) before
// the handler ever runs. A freshly-registered user from
// `create_authenticated_user` has no org membership, so tests provision one
// with these helpers.
//
// Historically every tenant-scoped test file re-declared its own near-identical
// `seed_org` / `seed_membership` (~22 copies). These shared versions promote
// the canonical pattern so new tests import one helper instead of copying ~30
// lines of seeding boilerplate. (See issue #1090.)
// ----------------------------------------------------------------------------

/// Insert a fresh active organization and return its id.
///
/// `slug` is a short, stable label chosen by the caller (e.g. `"s1"`). It is
/// embedded into a randomized slug/email so concurrent `#[sqlx::test]` runs (or
/// multiple orgs within one test) never collide on the `organizations` unique
/// constraints, while staying human-readable in failures.
pub async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("Test Org {slug}"))
    .bind(format!("test-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@test-org.internal", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

/// Make `user_id` an active member of `org_id` with the given `role` (the
/// `role_type` column, e.g. `"org_admin"`, `"resident"`).
///
/// `id` is omitted (the column defaults to `gen_random_uuid()`), and both
/// `created_at` (defaults to `NOW()`) and `joined_at` (nullable) are left for
/// the database — the minimal insert needed to satisfy `ValidatedTenantExtractor`.
pub async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid, role: &str) {
    sqlx::query(
        r#"INSERT INTO organization_members
               (organization_id, user_id, role_type, status)
           VALUES ($1, $2, $3, 'active')
           ON CONFLICT DO NOTHING"#,
    )
    .bind(org_id)
    .bind(user_id)
    .bind(role)
    .execute(pool)
    .await
    .expect("seed membership");
}

/// Register + verify + log in a user, then make them an active `org_admin`
/// member of a fresh org so `RlsConnection` can resolve tenant context.
///
/// Returns `(access_token, org_id)`. Pass `org_id.to_string()` as the
/// `X-Tenant-ID` header on subsequent requests.
pub async fn create_authenticated_user_with_org(
    app: &TestApp,
    user: &TestUser,
    slug: &str,
) -> (String, Uuid) {
    let (access_token, _refresh) = create_authenticated_user(app, user).await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let org_id = seed_org(&app.pool, slug).await;
    seed_membership(&app.pool, org_id, user_id, "org_admin").await;

    (access_token, org_id)
}

/// Mint a JWT in `api_core::Claims` format with the given tenant role.
///
/// Login tokens from `create_authenticated_user` carry `roles: [...]` (plural,
/// the services::JwtService format). `TenantExtractor` reads `role` (singular,
/// `api_core::Claims` format) — so manager-gated handlers return 403 for Guest
/// when given a plain login token. Use this helper (or `create_manager_with_org`)
/// when the handler checks `tenant.role.is_manager()`.
pub fn mint_tenant_token(
    user_id: Uuid,
    email: &str,
    org_id: Uuid,
    role: common::tenant::TenantRole,
) -> String {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

    const TEST_JWT_SECRET: &str =
        "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

    let now = chrono::Utc::now().timestamp();
    let claims = api_core::Claims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some(role),
        email: email.to_string(),
        name: "Test User".to_string(),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .expect("mint tenant token")
}

/// Register + org-seed a user (like `create_authenticated_user_with_org`),
/// but return a JWT with `role: "org_admin"` in `api_core::Claims` format so
/// `TenantExtractor` resolves the role correctly for manager-gated handlers.
pub async fn create_manager_with_org(app: &TestApp, user: &TestUser, slug: &str) -> (String, Uuid) {
    let (_login_token, org_id) = create_authenticated_user_with_org(app, user, slug).await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let token = mint_tenant_token(
        user_id,
        &user.email,
        org_id,
        common::tenant::TenantRole::OrgAdmin,
    );
    (token, org_id)
}
