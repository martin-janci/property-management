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

use axum::{extract::DefaultBodyLimit, http, routing::get, Router};
use http::HeaderValue;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod observability;
mod routes;
mod services;
mod state;

use db::repositories::AnnouncementRepository;
use services::{EmailService, JwtService, Scheduler, SchedulerConfig};
use state::AppState;

// Phase 5 — admin extension wiring helpers (B7). Both `main.rs` (production
// binary) and `lib.rs::create_router` (tests) build the same admin-core
// dependency chain via these helpers so they cannot drift.
use api_server::{attach_admin_extensions, build_admin_extensions};

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
        (name = "Regional Compliance", description = "Slovak & Czech regional legal compliance: voting quorum, accounting exports, GDPR, SVJ")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

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
        signature_reminder_days_before: std::env::var("SIGNATURE_REMINDER_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3),
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

    // Build router
    let app = Router::new()
        // Health (liveness) — shallow, no deps. Docker HEALTHCHECK target.
        .route("/health", get(routes::health::liveness))
        // Readiness — deep dep check (DB + Redis). Operator dashboards.
        .route("/readiness", get(routes::health::readiness))
        // Prometheus metrics endpoint (Epic 95.4)
        .route("/metrics", get(metrics_endpoint))
        // Auth routes
        .nest("/api/v1/auth", routes::auth::router())
        // Admin routes
        .nest("/api/v1/admin", routes::admin::router())
        // Phase 5.5: tenant lifecycle (export / purge / restore) — platform-admin only.
        .nest("/api/v1/admin", routes::admin_tenant_lifecycle::router())
        // Organizations routes
        .nest("/api/v1/organizations", routes::organizations::router())
        // Buildings routes
        .nest("/api/v1/buildings", routes::buildings::router())
        // Delegations routes (Epic 3, Story 3.4)
        .nest("/api/v1/delegations", routes::delegations::router())
        // Facilities routes (Epic 3, Story 3.7)
        .nest("/api/v1", routes::facilities::router())
        // Faults routes
        .nest("/api/v1/faults", routes::faults::router())
        // Voting routes
        .nest("/api/v1/voting", routes::voting::router())
        // Announcements routes (Epic 6)
        .nest("/api/v1/announcements", routes::announcements::router())
        // Documents routes (Epic 7A)
        .nest("/api/v1/documents", routes::documents::router())
        // Public shared document routes (no auth required)
        .merge(routes::documents::public_router())
        // Templates routes (Epic 7B)
        .nest("/api/v1/templates", routes::templates::router())
        // E-Signature routes (Epic 7B, Story 7B.3)
        .nest("/api/v1/signature-requests", routes::signatures::router())
        // Messaging routes (Epic 6, Story 6.5)
        .nest("/api/v1/messages", routes::messaging::router())
        // Neighbor routes (Epic 6, Story 6.6)
        .nest("/api/v1", routes::neighbors::router())
        // Notification preferences routes (Epic 8A)
        .nest(
            "/api/v1/users/me/notification-preferences",
            routes::notification_preferences::router(),
        )
        // WebSocket realtime notification sync (Epic 8A, Story 8A.3)
        .nest(
            "/api/v1/users/me/notifications",
            routes::ws_notifications::router(),
        )
        // Granular notification preferences routes (Epic 8B)
        .nest(
            "/api/v1/users/me/notification-preferences/granular",
            routes::granular_notifications::router(),
        )
        // Critical notifications routes (Epic 8A, Story 8A.2)
        .nest(
            "/api/v1/organizations/{org_id}/critical-notifications",
            routes::critical_notifications::router(),
        )
        // MFA routes (Epic 9, Story 9.1)
        .nest("/api/v1/auth/mfa", routes::mfa::router())
        // OAuth 2.0 routes (Epic 10A)
        .nest("/api/v1/oauth", routes::oauth::router())
        .nest("/api/v1/admin/oauth", routes::oauth::admin_router())
        // Platform Admin routes (Epic 10B)
        .nest("/api/v1/platform-admin", routes::platform_admin::router())
        // Public feature flags (Epic 10B.2)
        .nest(
            "/api/v1/feature-flags",
            routes::platform_admin::public_feature_flags_router(),
        )
        // Public system announcements (Epic 10B.4)
        .nest(
            "/api/v1/system-announcements",
            routes::platform_admin::public_announcements_router(),
        )
        // Public maintenance (Epic 10B.4)
        .nest(
            "/api/v1/maintenance",
            routes::platform_admin::public_maintenance_router(),
        )
        // Onboarding routes (Epic 10B.6)
        .nest("/api/v1/onboarding", routes::onboarding::router())
        // Help routes (Epic 10B.7)
        .nest("/api/v1/help", routes::help::router())
        // GDPR routes (Epic 9, Stories 9.3-9.5)
        .nest("/api/v1/gdpr", routes::gdpr::router())
        // Compliance routes (Epic 9, Story 9.7)
        .nest("/api/v1/compliance", routes::compliance::router())
        // Rentals routes
        .nest("/api/v1/rentals", routes::rentals::router())
        // Listings routes (management side)
        .nest("/api/v1/listings", routes::listings::router())
        // Integration routes
        .nest("/api/v1/integrations", routes::integrations::router())
        // Financial routes (Epic 11)
        .nest("/api/v1/financial", routes::financial::router())
        // Meters routes (Epic 12)
        .nest("/api/v1/meters", routes::meters::router())
        // AI routes (Epic 13)
        .nest("/api/v1/ai/chat", routes::ai::ai_chat_router())
        .nest("/api/v1/ai/sentiment", routes::ai::sentiment_router())
        .nest("/api/v1/ai/equipment", routes::ai::equipment_router())
        .nest("/api/v1/ai/workflows", routes::ai::workflow_router())
        // AI LLM routes (Epic 64)
        .nest("/api/v1/ai/llm", routes::ai::llm_router())
        // IoT routes (Epic 14)
        .nest("/api/v1/iot/sensors", routes::iot::sensor_router())
        // Agency routes (Epic 17)
        .nest("/api/v1/agencies", routes::agencies::router())
        // Lease routes (Epic 19)
        .nest("/api/v1/leases", routes::leases::router())
        // Work Orders routes (Epic 20)
        .nest("/api/v1/work-orders", routes::work_orders::router())
        // Vendor routes (Epic 21)
        .nest("/api/v1/vendors", routes::vendors::router())
        // Insurance routes (Epic 22)
        .nest("/api/v1/insurance", routes::insurance::router())
        // Emergency routes (Epic 23)
        .nest("/api/v1/emergency", routes::emergency::router())
        // Budget routes (Epic 24)
        .nest("/api/v1/budgets", routes::budgets::router())
        // Legal routes (Epic 25)
        .nest("/api/v1/legal", routes::legal::router())
        // Subscription routes (Epic 26)
        .nest("/api/v1/subscriptions", routes::subscriptions::router())
        .nest(
            "/api/v1/admin/subscriptions",
            routes::subscriptions::admin_router(),
        )
        // Government Portal routes (Epic 30)
        .nest(
            "/api/v1/government-portal",
            routes::government_portal::router(),
        )
        // Community routes (Epic 37)
        .nest("/api/v1/community", routes::community::router())
        // Automation routes (Epic 38)
        .nest("/api/v1/automation", routes::automation::router())
        // Forms routes (Epic 54)
        .nest("/api/v1/forms", routes::forms::router())
        // Reports routes (Epic 55)
        .nest("/api/v1/reports", routes::reports::router())
        // Package routes (Epic 58)
        .nest(
            "/api/v1/packages",
            routes::package_visitor::packages_router(),
        )
        // Visitor routes (Epic 58)
        .nest(
            "/api/v1/visitors",
            routes::package_visitor::visitors_router(),
        )
        // News routes (Epic 59)
        .nest("/api/v1/news", routes::news_articles::router())
        // Energy routes (Epic 65)
        .nest("/api/v1/energy", routes::energy::router())
        // Regional Compliance routes (Epic 72)
        .nest(
            "/api/v1/regional-compliance",
            routes::regional_compliance::router(),
        )
        // Migration routes (Epic 66)
        .nest("/api/v1/migration", routes::migration::router())
        // AML/DSA Compliance routes (Epic 67)
        .nest("/api/v1/aml-dsa", routes::aml_dsa::router())
        // Marketplace routes (Epic 68)
        .nest("/api/v1/marketplace", routes::marketplace::router())
        // Public API / Developer routes (Epic 69)
        .nest("/api/v1/developer", routes::public_api::router())
        // Competitive Features routes (Epic 70)
        .nest("/api/v1/competitive", routes::competitive::router())
        // Infrastructure routes (Epic 71)
        .nest("/api/v1/infrastructure", routes::infrastructure::router())
        // Operations routes (Epic 73)
        .nest("/api/v1/operations", routes::operations::router())
        // Owner Analytics routes (Epic 74)
        .nest("/api/v1/owner-analytics", routes::owner_analytics::router())
        // Dispute Resolution routes (Epic 77)
        .nest("/api/v1/disputes", routes::disputes::router())
        // Vendor Portal routes (Epic 78)
        .nest("/api/v1/vendor-portal", routes::vendor_portal::router())
        // Registry routes (Epic 64)
        .nest("/api/v1/registry", routes::registry::router())
        // Multi-Currency routes (Epic 145)
        .nest("/api/v1/multi-currency", routes::multi_currency::router())
        // Voice Webhooks routes (Epic 93)
        .nest(
            "/api/v1/webhooks/voice",
            routes::voice_webhooks::voice_webhook_router(),
        )
        // Portal Webhooks routes (Epic 105)
        .nest(
            "/api/v1/webhooks/portals",
            routes::portal_webhooks::router(),
        )
        // Feature Packages routes (Epic 108)
        .nest(
            "/api/v1/feature-packages",
            routes::feature_packages::router(),
        )
        // Features routes (Epic 109)
        .nest("/api/v1/features", routes::features::router())
        // Outages routes (UC-12)
        .nest("/api/v1/outages", routes::outages::router())
        // Market Pricing routes (Epic 132)
        .nest("/api/v1/market-pricing", routes::market_pricing::router())
        // Lease Abstraction routes (Epic 133)
        .nest(
            "/api/v1/lease-abstraction",
            routes::lease_abstraction::router(),
        )
        // Predictive Maintenance routes (Epic 134)
        .nest(
            "/api/v1/predictive-maintenance",
            routes::predictive_maintenance::router(),
        )
        // Enhanced Tenant Screening routes (Epic 135)
        .nest(
            "/api/v1/tenant-screening",
            routes::enhanced_tenant_screening::router(),
        )
        // ESG Reporting routes (Epic 136)
        .nest("/api/v1/esg", routes::esg_reporting::router())
        // Building Certifications routes (Epic 137)
        .nest(
            "/api/v1/building-certifications",
            routes::building_certifications::router(),
        )
        // Property Valuation routes (Epic 138)
        .nest(
            "/api/v1/property-valuation",
            routes::property_valuation::router(),
        )
        // Investor Portal routes (Epic 139)
        .nest("/api/v1/investor-portal", routes::investor_portal::router())
        // Portfolio Analytics routes (Epic 140)
        .nest(
            "/api/v1/portfolio-analytics",
            routes::portfolio_analytics::router(),
        )
        // Reserve Funds routes (Epic 141)
        .nest("/api/v1/reserve-funds", routes::reserve_funds::router())
        // Violations routes (Epic 142)
        .nest("/api/v1/violations", routes::violations::router())
        // Board Meetings routes (Epic 143)
        .nest("/api/v1/board-meetings", routes::board_meetings::router())
        // Portfolio Performance routes (Epic 144)
        .nest(
            "/api/v1/portfolio-performance",
            routes::portfolio_performance::router(),
        )
        // API Ecosystem Expansion routes (Epic 150)
        .nest("/api/v1/ecosystem", routes::api_ecosystem::router())
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
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    // Phase 5 — admin dependency injection (B7). Layered before TraceLayer so
    // every nested route inherits these extensions. Mirrors the chain in
    // `lib.rs::create_router` exactly via the shared `attach_admin_extensions`
    // helper so production and tests cannot drift.
    let app = attach_admin_extensions(app, &admin_ext)
        // P0-15: global request body cap. Default Axum limit is 2 MiB which
        // is fine for JSON but exposes every multipart handler to memory
        // abuse when the handler itself forgets to limit. Upload routes
        // that legitimately need more (restore, migration import) raise
        // this per-route via `DefaultBodyLimit::max(...)` and stream
        // chunks instead of buffering full payloads. Cap here: 16 MiB.
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
        // Middleware
        .layer(TraceLayer::new_for_http())
        // Phase 1: Host-resolution (tenant-resolution) middleware. Runs FIRST
        // on the request pipeline (layers execute outside-in), inspecting the
        // Host header to inject a `ResolvedTenant` extension before any
        // handler/extractor runs. Public-allowlist paths (`/health`, etc.)
        // bypass resolution; unknown hosts fail closed with 404.
        .layer(axum::middleware::from_fn_with_state(
            host_tenant_config.clone(),
            api_core::middleware::host_tenant_middleware,
        ))
        // CORS configuration - origins configurable via CORS_ALLOWED_ORIGINS env var
        .layer(
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
                .max_age(std::time::Duration::from_secs(3600)),
        )
        // Application state
        .with_state(state);

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
