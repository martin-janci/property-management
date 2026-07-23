#![allow(clippy::doc_overindented_list_items)]

//! API Server - Property Management System
//!
//! Consolidated backend for all property management operations.
//! Handles authentication, organizations, buildings, faults, voting,
//! rentals, listings, and external integrations.
//!
//! Package: ppt::api_server

// Allow dead code for stub implementations during development
#![allow(dead_code)]

use axum::{extract::DefaultBodyLimit, http, routing::get, Extension};
use http::HeaderValue;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

// The binary shares the `api_server` library crate's modules instead of
// compiling its own copies. This is the same single-source-of-truth principle
// the router unification (#867) applies to the route table: a binary-local
// `mod routes;` would be a second compilation of the route handlers that could
// drift from the library the integration tests exercise. Pull everything from
// `api_server::` so the binary and the tests share one set of types — in
// particular ONE `AppState`, so `api_server::route_table()` (a
// `Router<api_server::state::AppState>`) accepts the state built here.
use api_server::{observability, routes, services, state};

use db::repositories::AnnouncementRepository;
use services::{
    EmailService, JwtService, NotificationDigestConfig, NotificationDigestWorker, PushFanoutConfig,
    PushFanoutWorker, QuietHoursDrainConfig, QuietHoursDrainWorker, Scheduler, SchedulerConfig,
};
use state::AppState;

// Phase 5 — admin extension wiring helpers (B7). Both `main.rs` (production
// binary) and `lib.rs::create_router` (tests) build the same admin-core
// dependency chain via these helpers so they cannot drift.
use api_server::{attach_admin_extensions, build_admin_extensions, route_table};

// Boot-time preflight for required production env vars (issue #951 recurrence
// guard). Binary-local — it gates on `RUST_ENV` and fails fast before heavy
// init, so it lives next to `main` rather than in the shared library crate.
mod preflight;

/// Default CORS allowed origins for api-server.
/// Includes development origins and production domains.
const DEFAULT_CORS_ORIGINS: &[&str] = &[
    "http://localhost:3000",             // ppt-web dev
    "http://localhost:3001",             // reality-web dev
    "http://localhost:8080",             // api-server dev (swagger-ui)
    "http://localhost:8081",             // mobile dev / reality-server
    "https://ppt.three-two-bit.com",     // production
    "https://reality.three-two-bit.com", // reality production
];

/// Parse CORS allowed origins from environment variable.
///
/// Reads `CORS_ALLOWED_ORIGINS` environment variable as a comma-separated list of origins.
/// Falls back to default origins if not set.
///
/// # Example
/// ```bash
/// CORS_ALLOWED_ORIGINS=https://example.com,https://api.example.com
/// ```
fn get_cors_allowed_origins() -> Vec<HeaderValue> {
    match std::env::var("CORS_ALLOWED_ORIGINS") {
        Ok(origins_str) if !origins_str.is_empty() => {
            let origins: Vec<HeaderValue> = origins_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .filter_map(|origin| {
                    origin.parse::<HeaderValue>().ok().or_else(|| {
                        tracing::warn!("Invalid CORS origin '{}', skipping", origin);
                        None
                    })
                })
                .collect();

            if origins.is_empty() {
                tracing::warn!(
                    "CORS_ALLOWED_ORIGINS is set but no valid origins found, using defaults"
                );
                parse_default_origins()
            } else {
                tracing::info!("Using {} configured CORS origins", origins.len());
                origins
            }
        }
        _ => {
            tracing::info!("CORS_ALLOWED_ORIGINS not set, using default origins");
            parse_default_origins()
        }
    }
}

/// Parse the default origins into HeaderValue vector.
fn parse_default_origins() -> Vec<HeaderValue> {
    DEFAULT_CORS_ORIGINS
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect()
}

/// Build the production CORS layer.
///
/// Origins are configurable via the `CORS_ALLOWED_ORIGINS` env var (falling
/// back to [`DEFAULT_CORS_ORIGINS`]). `allow_headers` MUST be an explicit list
/// when paired with `allow_credentials(true)`: per the CORS spec browsers
/// reject the wildcard with credentials, and tower-http panics at layer
/// construction if the two are combined.
///
/// Extracted from `main()` so CORS-policy edits (a recurring churn point) live
/// in one cohesive function instead of inline in the `main()` middleware chain.
fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        // Allow requests from configured origins (env var or defaults)
        .allow_origin(get_cors_allowed_origins())
        // Allow common HTTP methods
        .allow_methods([
            http::Method::GET,
            http::Method::POST,
            http::Method::PUT,
            http::Method::PATCH,
            http::Method::DELETE,
            http::Method::OPTIONS,
        ])
        // Allow common headers
        .allow_headers([
            http::header::AUTHORIZATION,
            http::header::CONTENT_TYPE,
            http::header::ACCEPT,
            http::header::ORIGIN,
            http::HeaderName::from_static("x-requested-with"),
        ])
        // Allow credentials (cookies, authorization headers)
        .allow_credentials(true)
        // Cache preflight response for 1 hour
        .max_age(std::time::Duration::from_secs(3600))
}

/// Assemble the production router: the shared [`route_table`] plus the
/// production-only surface layered on top.
///
/// The application route table is sourced from the SINGLE source of truth,
/// `api_server::route_table()` (issue #867). The production binary used to
/// hand-maintain a parallel chain of `.nest(...)` calls here, which drifted
/// from `lib.rs::create_router` (used by the integration tests): PR #836 had
/// to retro-fit five routers that existed only in the test-side router and
/// were therefore unreachable in production. Building both routers from the
/// same `route_table()` makes that class of divergence impossible — a route
/// added to `route_table()` is reachable in production AND exercised by the
/// tests, and the guard test `route_table_is_single_source_of_truth` fails
/// the build if `main.rs` ever reverts to hand-listing API routes.
///
/// Only production-only surface is added on top here:
///   - `/metrics`            — needs the production observability registry.
///   - Swagger UI            — dev/ops doc endpoint.
///   - Phase-3 host routing  — `/tenant-config`, `/admin/tenants/...`,
///                             `/internal/caddy-ask`; these depend on the
///                             live host-tenant middleware and are covered
///                             by their own focused tests
///                             (`tenant_config_tests.rs`, `caddy_ask_tests.rs`).
///
/// Extracted from `main()` so router-assembly edits stop colliding with the
/// rest of startup.
fn build_app_router() -> axum::Router<AppState> {
    route_table()
        // Prometheus metrics endpoint (Epic 95.4) — production-only.
        .route("/metrics", get(metrics_endpoint))
        // Phase 3: per-host tenant config (read by reality-web at request
        // time). Mounted at the root because reality-web hits the same host
        // Caddy serves the app on, and the path must match exactly.
        .nest("/tenant-config", routes::tenant_config::router())
        // Phase 3: per-tenant admin endpoints (branding + feature flags).
        // Stub auth via `require_platform_principal`; Phase 5 swaps to a
        // real capability registry.
        .nest(
            "/admin/tenants/{org_id}/branding",
            routes::admin_tenants::branding_router(),
        )
        .nest(
            "/admin/tenants/{org_id}/feature-flags",
            routes::admin_tenants::feature_flags_router(),
        )
        // Phase 3: Caddy on-demand TLS ask-endpoint. Mounted at `/internal`,
        // which is in PUBLIC_ALLOWLIST so host_tenant_middleware skips it
        // entirely — by definition this endpoint runs BEFORE TLS / a
        // host -> tenant mapping exists. Auth is enforced inside the handler
        // (loopback bind in dev, X-Internal-Token shared secret in prod).
        .nest("/internal/caddy-ask", routes::caddy_ask::router())
        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
}

/// Layer production middleware onto the assembled router and bind the app
/// state, yielding the final `Router` ready to serve.
///
/// Layer order is significant — Tower layers execute outside-in, so the chain
/// below is preserved EXACTLY as it was inline in `main()`:
///   1. admin-core extensions (must be visible to every nested route)
///   2. global request body cap
///   3. baseline security headers
///   4. tracing
///   5. host-tenant resolution (runs FIRST on the request pipeline)
///   6. CORS
///   7. application state
///
/// Mirrors the chain in `lib.rs::create_router` via the shared
/// `attach_admin_extensions` helper so production and tests cannot drift.
/// Extracted from `main()` so middleware/security-header edits (a recurring
/// churn point — #954 security headers, #989 gap-sweep) live in one function.
fn apply_middleware(
    app: axum::Router<AppState>,
    admin_ext: &api_server::AdminExtensions,
    host_tenant_config: api_core::middleware::HostTenantConfig,
    state: AppState,
) -> axum::Router {
    attach_admin_extensions(app, admin_ext)
        // P0-15: global request body cap. Default Axum limit is 2 MiB which
        // is fine for JSON but exposes every multipart handler to memory
        // abuse when the handler itself forgets to limit. Upload routes
        // that legitimately need more (restore, migration import) raise
        // this per-route via `DefaultBodyLimit::max(...)` and stream
        // chunks instead of buffering full payloads. Cap here: 16 MiB.
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
        // Baseline security headers (HSTS, nosniff, frame-deny, referrer) on
        // every response — the API edge previously shipped none (#954).
        .layer(axum::middleware::from_fn(
            api_core::middleware::security_headers,
        ))
        // Middleware
        .layer(TraceLayer::new_for_http())
        // Phase 1: Host-resolution (tenant-resolution) middleware. Runs FIRST
        // on the request pipeline (layers execute outside-in), inspecting the
        // Host header to inject a `ResolvedTenant` extension before any
        // handler/extractor runs. Public-allowlist paths (`/health`, etc.)
        // bypass resolution; unknown hosts fail closed with 404.
        .layer(axum::middleware::from_fn_with_state(
            host_tenant_config,
            api_core::middleware::host_tenant_middleware,
        ))
        // CORS configuration - origins configurable via CORS_ALLOWED_ORIGINS env var
        .layer(cors_layer())
        .layer(Extension(state.clone()))
        // Application state
        .with_state(state)
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Property Management API",
        version = "1.0.0",
        description = "API for Property Management System (PPT)",
        contact(name = "PPT Team", email = "api@ppt.example.com"),
        license(name = "MIT")
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development"),
        (url = "https://api.ppt.example.com", description = "Production")
    ),
    paths(
        routes::health::liveness,
        routes::health::readiness,
        routes::auth::login,
        routes::auth::register,
        routes::auth::logout,
        routes::auth::verify_email,
        routes::auth::resend_verification,
        routes::auth::refresh_token,
        routes::auth::forgot_password,
        routes::auth::reset_password,
        routes::auth::list_sessions,
        routes::auth::revoke_session,
        routes::auth::revoke_all_sessions,
        routes::admin::users_lifecycle::list_users,
        routes::admin::users_lifecycle::get_user,
        routes::admin::users_lifecycle::suspend_user,
        routes::admin::users_lifecycle::reactivate_user,
        routes::admin::users_lifecycle::delete_user,
        routes::organizations::create_organization,
        routes::organizations::list_organizations,
        routes::organizations::list_my_organizations,
        routes::organizations::get_organization,
        routes::organizations::update_organization,
        routes::organizations::delete_organization,
        routes::organizations::list_organization_members,
        routes::organizations::add_organization_member,
        routes::organizations::update_organization_member,
        routes::organizations::remove_organization_member,
        routes::organizations::list_organization_roles,
        routes::organizations::create_organization_role,
        routes::organizations::get_organization_role,
        routes::organizations::update_organization_role,
        routes::organizations::delete_organization_role,
        routes::organizations::get_organization_settings,
        routes::organizations::update_organization_settings,
        routes::organizations::get_organization_branding,
        routes::organizations::update_organization_branding,
        routes::organizations::export_organization_data,
        routes::messaging::delete_message,
        routes::accounting::invoices::list_invoices,
        routes::accounting::invoices::get_invoice,
        routes::accounting::invoices::list_invoice_items,
        routes::accounting::invoices::create_invoice,
        routes::accounting::invoices::update_invoice,
        routes::accounting::invoices::delete_invoice,
        routes::accounting::contacts::list_contacts,
        routes::layout::tenant::get_tenant_override,
        routes::layout::tenant::put_tenant_override,
        routes::layout::resolved::get_resolved,
        routes::layout::admin::list_screens,
        routes::layout::admin::get_config,
        routes::layout::admin::put_draft,
        routes::layout::admin::put_rails,
        routes::layout::admin::list_manifests,
        routes::layout::admin::put_manifest,
        routes::layout::admin::publish,
        routes::layout::admin::rollback,
        routes::layout::admin::kill,
        routes::layout::admin::unkill,
        routes::layout::admin::preview_resolve,
    ),
    components(schemas(
        routes::health::HealthResponse,
        routes::health::LivenessResponse,
        routes::auth::LoginRequest,
        routes::auth::LoginResponse,
        routes::auth::RegisterRequest,
        routes::auth::RegisterResponse,
        routes::auth::VerifyEmailQuery,
        routes::auth::VerifyEmailResponse,
        routes::auth::ResendVerificationRequest,
        routes::auth::ResendVerificationResponse,
        routes::auth::RefreshTokenRequest,
        routes::auth::LogoutRequest,
        routes::auth::LogoutResponse,
        routes::auth::ForgotPasswordRequest,
        routes::auth::ForgotPasswordResponse,
        routes::auth::ResetPasswordRequest,
        routes::auth::ResetPasswordResponse,
        routes::auth::SessionInfo,
        routes::auth::ListSessionsResponse,
        routes::auth::RevokeSessionRequest,
        routes::auth::RevokeSessionResponse,
        routes::auth::RevokeAllSessionsResponse,
        routes::admin::AdminUserInfo,
        routes::admin::ListUsersQuery,
        routes::admin::ListUsersResponse,
        routes::admin::UserActionRequest,
        routes::admin::AdminActionResponse,
        routes::organizations::CreateOrganizationRequest,
        routes::organizations::OrganizationResponse,
        routes::organizations::ListOrganizationsResponse,
        routes::organizations::UpdateOrganizationRequest,
        routes::organizations::DeleteOrganizationResponse,
        routes::organizations::MemberResponse,
        routes::organizations::ListMembersResponse,
        routes::organizations::AddMemberRequest,
        routes::organizations::AddMemberResponse,
        routes::organizations::UpdateMemberRequest,
        routes::organizations::UpdateMemberResponse,
        routes::organizations::RemoveMemberResponse,
        routes::organizations::RoleResponse,
        routes::organizations::ListRolesResponse,
        routes::organizations::CreateRoleRequest,
        routes::organizations::CreateRoleResponse,
        routes::organizations::GetRoleResponse,
        routes::organizations::UpdateRoleRequest,
        routes::organizations::UpdateRoleResponse,
        routes::organizations::DeleteRoleResponse,
        routes::organizations::OrganizationSettingsResponse,
        routes::organizations::UpdateOrganizationSettingsRequest,
        routes::organizations::OrganizationBrandingResponse,
        routes::organizations::UpdateOrganizationBrandingRequest,
        routes::organizations::ExportMember,
        routes::organizations::ExportRole,
        routes::organizations::ExportQuery,
        routes::organizations::OrganizationExportResponse,
        db::models::accounting::Invoice,
        db::models::accounting::InvoiceItem,
        db::models::accounting::InvoiceStatus,
        db::models::accounting::CreateInvoice,
        db::models::accounting::CreateInvoiceItem,
        db::models::accounting::UpdateInvoice,
        routes::layout::types::ScreenQuery,
        routes::layout::types::PutDraftRequest,
        routes::layout::types::PutRailsRequest,
        routes::layout::types::PutManifestRequest,
        routes::layout::types::ValidationErrorsResponse,
        routes::layout::types::PublishRequest,
        routes::layout::types::RollbackRequest,
        routes::layout::types::KillRequest,
        routes::layout::types::PutTenantOverrideRequest,
        routes::layout::types::ResolvedQuery,
        routes::layout::types::PreviewResolveRequest,
        common::errors::ErrorResponse,
        common::errors::ValidationError,
        common::tenant::TenantContext,
        common::tenant::TenantRole,
    )),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Authentication", description = "User authentication and authorization"),
        (name = "Admin", description = "Administrative user management"),
        (name = "Organizations", description = "Multi-tenant organization management"),
        (name = "Buildings", description = "Building and unit management"),
        (name = "Delegations", description = "Ownership delegation management"),
        (name = "Facilities", description = "Common area and facility management"),
        (name = "Facility Bookings", description = "Facility booking and reservation management"),
        (name = "Faults", description = "Fault reporting and tracking"),
        (name = "Voting", description = "Voting and polls"),
        (name = "Announcements", description = "Announcements and communication"),
        (name = "Documents", description = "Document management and sharing"),
        (name = "E-Signatures", description = "Electronic signature workflows for documents"),
        (name = "Notification Preferences", description = "User notification channel preferences"),
        (name = "Critical Notifications", description = "Critical notifications that bypass user preferences"),
        (name = "Multi-Factor Authentication", description = "Two-factor authentication setup and management"),
        (name = "GDPR", description = "GDPR compliance: data export, deletion, and privacy settings"),
        (name = "Compliance", description = "Compliance reports: audit logs, security, and GDPR reports"),
        (name = "Rentals", description = "Short-term rental integrations (Airbnb, Booking)"),
        (name = "Listings", description = "Real estate listing management"),
        (name = "Integrations", description = "External portal integrations"),
        (name = "financial", description = "Financial management: accounts, invoices, payments"),
        (name = "meters", description = "Meter readings and utility management"),
        (name = "AI Chat", description = "AI-powered chat assistant"),
        (name = "Sentiment Analysis", description = "Resident sentiment tracking"),
        (name = "Equipment", description = "Equipment and predictive maintenance"),
        (name = "Workflows", description = "Automated workflow management"),
        (name = "IoT Sensors", description = "IoT sensor registration and management"),
        (name = "Community", description = "Community & social features: groups, posts, events, marketplace"),
        (name = "Automation", description = "Workflow automation rules, templates, and execution"),
        (name = "energy", description = "Energy & sustainability: EPCs, carbon footprint, sustainability scores, benchmarking"),
        (name = "Marketplace", description = "Service provider marketplace: providers, bookings, reviews, tenant insurance"),
        (name = "Competitive Features", description = "Competitive enhancements: dynamic pricing, neighborhood data, virtual tours, market trends"),
        (name = "Regional Compliance", description = "Slovak & Czech regional legal compliance: voting quorum, accounting exports, GDPR, SVJ"),
        (name = "Layout", description = "Layout & content manager: tenant overrides and resolved screens"),
        (name = "Layout Admin", description = "Layout & content manager: platform-admin control plane (drafts, rails, manifests, publish, kill switches)")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Recurrence guard for #951: fail fast if a required production secret is
    // missing, listing ALL of them at once, BEFORE any heavy init (DB pool,
    // migrations, server bind). In development this is a no-op — the per-var
    // dev fallbacks below apply. Outside development a missing var returns an
    // Err that propagates out of `main`, exiting non-zero before the server can
    // boot half-broken.
    if let Err(message) = preflight::run() {
        return Err(anyhow::anyhow!("{message}"));
    }

    // Initialize observability (Epic 95)
    // This sets up OpenTelemetry tracing, Sentry error tracking, and Prometheus metrics
    // IMPORTANT: This guard MUST remain in scope for the entire application lifetime.
    // Dropping it will shut down the Sentry client and stop error reporting.
    #[allow(unused_variables)]
    let observability_guard = observability::init_observability(
        observability::OtelConfig::default(),
        observability::SentryConfig::default(),
        observability::MetricsConfig::default(),
    );

    tracing::info!(
        "API Server v{} starting with observability enabled",
        env!("CARGO_PKG_VERSION")
    );

    // Get database URL from environment. A hardcoded fallback to a local dev
    // database is only permitted when RUST_ENV=development to support local
    // development; production/staging must set DATABASE_URL explicitly.
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        let is_development = std::env::var("RUST_ENV").unwrap_or_default() == "development";
        if is_development {
            tracing::warn!(
                "DATABASE_URL not set, using development default (DEVELOPMENT MODE ONLY)"
            );
            "postgres://postgres:postgres@localhost:5432/ppt".to_string()
        } else {
            panic!(
                "DATABASE_URL environment variable is required in non-development environments. \
                 Set RUST_ENV=development to use the dev default."
            );
        }
    });

    // Create RLS-safe database pool with automatic context cleanup
    // The after_release hook ensures RLS context is cleared before connections
    // are returned to the pool, providing defense-in-depth against context bleeding
    let db_pool = db::create_rls_safe_pool(&database_url).await?;
    tracing::info!("Connected to database with RLS-safe pool");

    // Apply any pending migrations from `backend/crates/db/migrations/`. The
    // sqlx migrator is idempotent and uses a Postgres advisory lock, so a
    // concurrent `reality-server` startup against the same DB is safe — the
    // second caller blocks until the first finishes, then sees zero pending.
    // Critical on first deploy of a fresh target: `ppt_prod` / `ppt_staging`
    // are empty databases (created from `ppt_dev_template` which is also
    // empty); without this call the first request would fail with
    // `relation "users" does not exist`.
    // `.context()` keeps the original `MigrateError` as the source so a
    // failure surfaces with both the human-readable context and the
    // chained underlying cause (e.g. `permission denied for relation X`)
    // when anyhow renders the error chain — `map_err(|e| anyhow!("{e}"))`
    // would have flattened that into a single string.
    use anyhow::Context;
    db::run_migrations(&db_pool)
        .await
        .context("DB migration failed")?;
    tracing::info!("Database migrations applied (or already current)");

    // Create email service (development mode by default)
    let email_enabled = std::env::var("EMAIL_ENABLED")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    let base_url =
        std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let email_service = EmailService::new(base_url, email_enabled);

    // SECURITY: Create JWT service with strict secret validation
    // - Production: JWT_SECRET required with minimum 64 characters (recommended)
    // - Development: Falls back to dev default when RUST_ENV=development
    let is_development = std::env::var("RUST_ENV").unwrap_or_default() == "development";
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        if is_development {
            tracing::warn!("JWT_SECRET not set, using development default (DEVELOPMENT MODE ONLY)");
            "development-secret-key-that-is-at-least-64-characters-long-for-testing".to_string()
        } else {
            panic!("JWT_SECRET environment variable is required. Set RUST_ENV=development to use dev defaults.");
        }
    });

    // SECURITY: Validate JWT secret strength
    // Production: Warn if less than 64 characters (security best practice)
    // All environments: Fail if less than 32 characters (minimum security)
    if jwt_secret.len() < 32 {
        panic!("JWT_SECRET must be at least 32 characters long for minimum security");
    }
    if !is_development && jwt_secret.len() < 64 {
        tracing::warn!(
            "JWT_SECRET is {} characters (minimum 64 recommended for production security)",
            jwt_secret.len()
        );
    }

    let jwt_service = JwtService::new(&jwt_secret).expect("Failed to create JWT service");

    // SECURITY (#527 findings #1 & #4): Validate e-signature secrets at
    // startup so the binary refuses to boot in production without them,
    // instead of falling back to a hardcoded dev token secret or silently
    // disabling the webhook auth check on first use. Mirrors the JWT_SECRET
    // handling above.
    if let Err(e) = integrations::LightweightProvider::from_env() {
        panic!(
            "E-signature config invalid: {e}. \
             Set RUST_ENV=development to use dev defaults."
        );
    }
    let webhook_secret_set = std::env::var("ESIGN_WEBHOOK_SECRET")
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    if !webhook_secret_set {
        if is_development {
            tracing::warn!(
                "ESIGN_WEBHOOK_SECRET not set, webhook receiver will reject all events (DEVELOPMENT MODE ONLY)"
            );
        } else {
            panic!(
                "ESIGN_WEBHOOK_SECRET environment variable is required (got missing or empty). \
                 Set RUST_ENV=development to use dev defaults."
            );
        }
    }

    // Phase 1: Build the host-resolution config + shared tenant-resolution
    // cache ONCE, then clone the `Arc` into both the middleware config and the
    // AppState so domain-management handlers can invalidate cache entries via
    // `state.tenant_resolution_cache`.
    //
    // Phase 5.5: same pattern for the per-tenant rate limiter set (defense
    // leak #15) — the middleware enforces the limit and admin handlers
    // install per-tenant overrides via
    // `state.tenant_rate_limiters.set_override(org, rpm)`.
    let host_tenant_config = api_core::middleware::HostTenantConfig::new(db_pool.clone());
    let tenant_resolution_cache = host_tenant_config.cache.clone();
    let tenant_rate_limiters = host_tenant_config.rate_limiters.clone();

    // Create application state. Clone the EmailService into AppState so the
    // scheduler (built below) can also receive it via `.with_email_service`.
    let state = AppState::new(
        db_pool.clone(),
        email_service.clone(),
        jwt_service,
        tenant_resolution_cache,
        tenant_rate_limiters,
    );

    // gap-84-1: Wire the S3 / MinIO storage service so document download &
    // preview endpoints can mint short-lived presigned URLs. The constructor
    // (`from_env_async`) initialises the real aws-sdk-s3 client — for an
    // S3-compatible endpoint like MinIO it picks up S3_ENDPOINT and uses
    // path-style addressing. Fail-soft: a missing/invalid config logs a
    // warning and leaves `storage_service = None`, in which case the download
    // endpoint returns 503 STORAGE_NOT_CONFIGURED rather than crashing the
    // whole server (mirrors the optional-service pattern used elsewhere).
    let state = match integrations::StorageService::from_env_async().await {
        Ok(storage) => {
            tracing::info!(
                bucket = %storage.bucket(),
                region = %storage.region(),
                download_ttl_secs = storage.download_ttl_secs(),
                "S3 storage service initialised — document downloads enabled"
            );
            state.with_storage(storage)
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "S3 storage not configured — document download/preview endpoints \
                 will return 503 until S3_BUCKET / AWS_* env vars are set"
            );
            state
        }
    };

    // Start background scheduler for scheduled announcements
    let scheduler_enabled = std::env::var("SCHEDULER_ENABLED")
        .map(|v| v != "false" && v != "0")
        .unwrap_or(true);
    let scheduler_config = SchedulerConfig {
        interval_secs: std::env::var("SCHEDULER_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60),
        enabled: scheduler_enabled,
        vote_reminder_days_before: std::env::var("VOTE_REMINDER_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        meter_reminder_days_before: std::env::var("METER_REMINDER_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3),
        payment_reminder_days_before: std::env::var("PAYMENT_REMINDER_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7),
        overdue_grace_period_days: std::env::var("OVERDUE_GRACE_PERIOD_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        signature_reminder_days_before: std::env::var("SIGNATURE_REMINDER_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3),
        pin_max_age_days: std::env::var("PIN_MAX_AGE_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
        support_tooling_retention_days: std::env::var("SUPPORT_TOOLING_RETENTION_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(730),
    };
    let scheduler_pool = state.db.clone();
    let announcement_repo = AnnouncementRepository::new(scheduler_pool.clone());
    // SECURITY (#527 finding #9): Wire the real EmailService into the
    // scheduler. Without this, `Scheduler::new` defaulted to
    // `EmailService::development()` (enabled=false, base_url=localhost:3000),
    // which silently logged signature reminders instead of sending them and
    // baked localhost links into any URL the scheduler generated.
    let scheduler = Scheduler::new(scheduler_pool, announcement_repo, scheduler_config)
        .with_email_service(email_service.clone());
    let _scheduler_handle = scheduler.start();

    // Epic 8A-3: start the push notification fanout background worker.
    // The worker fans out pending push notifications to FCM/APNs device tokens.
    // If FCM credentials are not configured it runs in heartbeat-only mode and
    // does NOT crash the server.
    let push_fanout_config = PushFanoutConfig::from_env();
    let push_fanout_worker = PushFanoutWorker::new(
        db_pool.clone(),
        state.pubsub_service.clone(),
        push_fanout_config,
    );
    let _push_fanout_handle = push_fanout_worker.start();

    // Story 8B.3 / #980: release notifications held during quiet hours once the
    // user's quiet-hours window ends. Polls `held_notifications` on an interval;
    // disabled-safe (no crash when env vars are unset).
    let quiet_hours_drain_worker = QuietHoursDrainWorker::new(
        db_pool.clone(),
        email_service.clone(),
        state.pubsub_service.clone(),
        QuietHoursDrainConfig::from_env(),
    );
    let _quiet_hours_drain_handle = quiet_hours_drain_worker.start();

    // Story 2B.3 / PM #969 gap 1: email a notification digest to users who have
    // been inactive for >24h and have unread (pending) notifications. Drives the
    // previously-uncalled `create_digest` write path. Polls on an interval and is
    // disabled-safe (no crash when env vars are unset; gated per-user behind the
    // `digest_enabled` preference).
    let notification_digest_worker = NotificationDigestWorker::new(
        db_pool.clone(),
        email_service.clone(),
        NotificationDigestConfig::from_env(),
    );
    let _notification_digest_handle = notification_digest_worker.start();

    // Phase 5 — admin dependency injection (B7).
    //
    // Build the admin-core dep bundle (capability registry init + grants /
    // mfa / audit / impersonation services backed by the live `db_pool`) and
    // attach it to the router below via `attach_admin_extensions`. The same
    // helpers are called from `lib.rs::create_router` so the production binary
    // and the test-side router stay in sync; before this fix only `create_router`
    // wired admin extensions, so production hit 500 on every `/admin/*` call
    // because `RequireCapability` could not find `AdminDeps` in extensions.
    let admin_ext = build_admin_extensions(db_pool.clone());

    // Build the production router (shared route table + production-only
    // surface) then layer production middleware and bind the app state. The
    // two cohesive blocks live in `build_app_router` / `apply_middleware` so
    // concurrent PRs editing routing vs. middleware no longer collide here.
    let app = build_app_router();
    let app = apply_middleware(app, &admin_ext, host_tenant_config, state);

    // Run server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("API server (Property Management) listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}

/// Prometheus metrics endpoint (Epic 95.4).
async fn metrics_endpoint() -> impl axum::response::IntoResponse {
    let metrics = observability::get_metrics_text();
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        metrics,
    )
}
