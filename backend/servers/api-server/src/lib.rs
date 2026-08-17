//! API Server library for Property Management System.
//!
//! This module exposes the application components for integration testing.

// Allow dead code in stub route/service implementations during development
#![allow(clippy::doc_overindented_list_items)]
#![allow(dead_code)]

pub mod client_ip;
pub mod observability;
pub mod routes;
pub mod services;
pub mod state;

use axum::{http, routing::get, Extension, Router};
use http::HeaderValue;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

// Phase 5 — Admin extension wiring.
use admin_core::{
    AdminDeps, AlwaysEnrolledMfa, AuditWriter, CapabilityGrantsRepository, CapabilityRegistry,
    ImpersonationService, MfaEnrollment, MfaRecency, NoopAuditWriter, NoopMfaRecency,
    PgAuditWriter, PgCapabilityGrantsRepository, PgImpersonationService, PgMfaEnrollment,
    PgMfaRecency,
};

/// Phase 5 — admin dependency injection bundle.
///
/// Holds the trio of services (`grants`, `mfa`, `audit`) the
/// `RequireCapability` extractor needs, plus the impersonation service.
/// Construct via [`build_admin_extensions`] and apply with
/// [`attach_admin_extensions`] so the production binary in `main.rs::serve`
/// and the test-side `create_router` agree exactly.
#[derive(Clone)]
pub struct AdminExtensions {
    pub deps: AdminDeps,
    pub grants: Arc<dyn CapabilityGrantsRepository>,
    pub mfa: Arc<dyn MfaRecency>,
    pub enrollment: Arc<dyn MfaEnrollment>,
    pub audit: Arc<dyn AuditWriter>,
    pub impersonation: Arc<dyn ImpersonationService>,
}

/// Build the admin dependency bundle from a database pool.
///
/// Initialises the capability registry as a side effect (idempotent — the
/// registry is a `OnceLock` under the hood).
pub fn build_admin_extensions(pool: db::DbPool) -> AdminExtensions {
    CapabilityRegistry::init(admin_core::Capability::ALL.iter().copied());
    let grants: Arc<dyn CapabilityGrantsRepository> =
        Arc::new(PgCapabilityGrantsRepository::new(pool.clone()));
    let mfa: Arc<dyn MfaRecency> = Arc::new(PgMfaRecency::new(pool.clone()));
    let enrollment: Arc<dyn MfaEnrollment> = Arc::new(PgMfaEnrollment::new(pool.clone()));
    let audit: Arc<dyn AuditWriter> = Arc::new(PgAuditWriter::new(pool.clone()));
    let deps = AdminDeps::new(
        grants.clone(),
        mfa.clone(),
        enrollment.clone(),
        audit.clone(),
    );
    let impersonation: Arc<dyn ImpersonationService> =
        Arc::new(PgImpersonationService::new(pool, audit.clone()));
    // Suppress unused warnings for noop fixtures (used by tests, not by the
    // production binary).
    let _ = (NoopAuditWriter, NoopMfaRecency, AlwaysEnrolledMfa);
    AdminExtensions {
        deps,
        grants,
        mfa,
        enrollment,
        audit,
        impersonation,
    }
}

/// Layer the admin-core extensions onto a router.
///
/// Order mirrors the pre-existing chain in `create_router` so behaviour is
/// identical between the test-side and production paths. Layered before
/// `TraceLayer` (the caller is responsible for that ordering) so every nested
/// route inherits the extensions.
pub fn attach_admin_extensions<S>(router: Router<S>, ext: &AdminExtensions) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .layer(Extension(ext.deps.clone()))
        .layer(Extension(ext.grants.clone()))
        .layer(Extension(ext.mfa.clone()))
        .layer(Extension(ext.enrollment.clone()))
        .layer(Extension(ext.audit.clone()))
        .layer(Extension(ext.impersonation.clone()))
}

/// Default CORS allowed origins for api-server.
const DEFAULT_CORS_ORIGINS: &[&str] = &[
    "http://localhost:3000",
    "http://localhost:3001",
    "http://localhost:8080",
    "http://localhost:8081",
    "https://ppt.three-two-bit.com",
    "https://reality.three-two-bit.com",
];

/// Parse default origins into HeaderValue vector.
fn parse_default_origins() -> Vec<HeaderValue> {
    DEFAULT_CORS_ORIGINS
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect()
}

/// Build the application route table — the SINGLE source of truth for every
/// HTTP route the api-server exposes.
///
/// Returns a state-less [`Router<AppState>`] with no middleware and no admin
/// extensions layered. Both [`create_router`] (integration tests) and the
/// production binary's `main()` build their router from THIS function, so the
/// two can never silently diverge again (issue #867). Before this refactor,
/// `main.rs` hand-maintained its own parallel chain of `.nest(...)` calls;
/// PR #836 had to retro-fit five routers (`push-tokens`, `mfa/recovery-codes`,
/// `pricing`, `property-valuations`, `data-residency`) that existed only in
/// the test-side router and were therefore unreachable in production.
///
/// Production-only surface — the Prometheus `/metrics` endpoint, Swagger UI,
/// and the Phase-3 host-routing endpoints (`/tenant-config`,
/// `/admin/tenants/{org_id}/...`, `/internal/caddy-ask`) — is intentionally
/// NOT included here. Those depend on production-only state (the metrics
/// registry, the live host-tenant middleware) and are layered onto the shared
/// table by `main.rs`. They are covered by their own focused tests
/// (`tenant_config_tests.rs`, `caddy_ask_tests.rs`).
pub fn route_table() -> Router<AppState> {
    Router::new()
        // Health (liveness) — shallow, no deps. Docker HEALTHCHECK target.
        .route("/health", get(routes::health::liveness))
        // Readiness — deep dep check (DB + Redis). Operator dashboards.
        .route("/readiness", get(routes::health::readiness))
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
        // Resident-facing "My Unit" view (Epic 3, Story 3.6)
        .nest("/api/v1/users/me/units", routes::my_units::router())
        // Delegations routes
        .nest("/api/v1/delegations", routes::delegations::router())
        // Facilities routes
        .nest("/api/v1", routes::facilities::router())
        // Faults routes
        .nest("/api/v1/faults", routes::faults::router())
        // Voting routes
        .nest("/api/v1/voting", routes::voting::router())
        // Announcements routes
        .nest("/api/v1/announcements", routes::announcements::router())
        // Documents routes
        .nest("/api/v1/documents", routes::documents::router())
        .merge(routes::documents::public_router())
        // Templates routes
        .nest("/api/v1/templates", routes::templates::router())
        // E-Signature routes
        .nest("/api/v1/signature-requests", routes::signatures::router())
        // Signer-facing public consumer endpoint (issue #761 part 2): the
        // emailed HMAC link lands the signer here. No auth — all authority is
        // in the verified token. GET render-context + POST record-signature.
        .nest(
            "/api/v1/signatures",
            routes::signatures::public_sign_router(),
        )
        // Messaging routes
        .nest("/api/v1/messages", routes::messaging::router())
        // Neighbor routes
        .nest("/api/v1", routes::neighbors::router())
        // Notification preferences routes
        .nest(
            "/api/v1/users/me/notification-preferences",
            routes::notification_preferences::router(),
        )
        // Mobile OS push token registration (Epic 8A-3)
        .nest(
            "/api/v1/users/me/push-tokens",
            routes::push_tokens::router(),
        )
        // WebSocket realtime notification sync (Epic 8A, Story 8A.3)
        .nest(
            "/api/v1/users/me/notifications",
            routes::ws_notifications::router(),
        )
        // Granular notification preferences routes
        .nest(
            "/api/v1/users/me/notification-preferences/granular",
            routes::granular_notifications::router(),
        )
        // Critical notifications routes
        .nest(
            "/api/v1/organizations/{org_id}/critical-notifications",
            routes::critical_notifications::router(),
        )
        // MFA routes (auth flow)
        .nest("/api/v1/auth/mfa", routes::mfa::router())
        // MFA recovery-code consumption (Story 9.2)
        .nest(
            "/api/v1/users/me/mfa/recovery-codes",
            routes::mfa::recovery_codes_router(),
        )
        // OAuth routes
        .nest("/api/v1/oauth", routes::oauth::router())
        .nest("/api/v1/admin/oauth", routes::oauth::admin_router())
        // Platform Admin routes
        .nest("/api/v1/platform-admin", routes::platform_admin::router())
        .nest(
            "/api/v1/platform-admin/layout",
            routes::layout::admin_router(),
        )
        .nest("/api/v1/layout", routes::layout::router())
        .nest(
            "/api/v1/feature-flags",
            routes::platform_admin::public_feature_flags_router(),
        )
        .nest(
            "/api/v1/system-announcements",
            routes::platform_admin::public_announcements_router(),
        )
        .nest(
            "/api/v1/maintenance",
            routes::platform_admin::public_maintenance_router(),
        )
        // Onboarding routes
        .nest("/api/v1/onboarding", routes::onboarding::router())
        // Help routes
        .nest("/api/v1/help", routes::help::router())
        // GDPR routes
        .nest("/api/v1/gdpr", routes::gdpr::router())
        // Compliance routes
        .nest("/api/v1/compliance", routes::compliance::router())
        // Rentals routes
        .nest("/api/v1/rentals", routes::rentals::router())
        // Listings routes
        .nest("/api/v1/listings", routes::listings::router())
        // Integration routes
        .nest("/api/v1/integrations", routes::integrations::router())
        // Financial routes
        .nest("/api/v1/financial", routes::financial::router())
        // Accounting routes
        .nest("/api/v1/accounting", routes::accounting::router())
        // Meters routes
        .nest("/api/v1/meters", routes::meters::router())
        // AI routes
        .nest("/api/v1/ai/chat", routes::ai::ai_chat_router())
        .nest("/api/v1/ai/sentiment", routes::ai::sentiment_router())
        .nest("/api/v1/ai/equipment", routes::ai::equipment_router())
        .nest("/api/v1/ai/workflows", routes::ai::workflow_router())
        .nest("/api/v1/ai/llm", routes::ai::llm_router())
        .nest("/api/v1/ai/ocr", routes::ai::ocr_router())
        // IoT routes — REST + realtime sensor WS (`GET /api/v1/iot/sensors/ws`)
        .nest("/api/v1/iot/sensors", routes::iot::sensor_router())
        // Agency routes
        .nest("/api/v1/agencies", routes::agencies::router())
        // Lease routes
        .nest("/api/v1/leases", routes::leases::router())
        // Work Orders routes
        .nest("/api/v1/work-orders", routes::work_orders::router())
        // Vendor routes
        .nest("/api/v1/vendors", routes::vendors::router())
        // Insurance routes
        .nest("/api/v1/insurance", routes::insurance::router())
        // Emergency routes
        .nest("/api/v1/emergency", routes::emergency::router())
        // Budget routes
        .nest("/api/v1/budgets", routes::budgets::router())
        // Legal routes
        .nest("/api/v1/legal", routes::legal::router())
        // Subscription routes
        .nest("/api/v1/subscriptions", routes::subscriptions::router())
        .nest(
            "/api/v1/admin/subscriptions",
            routes::subscriptions::admin_router(),
        )
        // Government Portal routes
        .nest(
            "/api/v1/government-portal",
            routes::government_portal::router(),
        )
        // Community routes
        .nest("/api/v1/community", routes::community::router())
        // Automation routes
        .nest("/api/v1/automation", routes::automation::router())
        // Forms routes
        .nest("/api/v1/forms", routes::forms::router())
        // Reports routes
        .nest("/api/v1/reports", routes::reports::router())
        // Package routes
        .nest(
            "/api/v1/packages",
            routes::package_visitor::packages_router(),
        )
        // Visitor routes
        .nest(
            "/api/v1/visitors",
            routes::package_visitor::visitors_router(),
        )
        // News routes
        .nest("/api/v1/news", routes::news_articles::router())
        // Energy routes
        .nest("/api/v1/energy", routes::energy::router())
        // Regional Compliance routes
        .nest(
            "/api/v1/regional-compliance",
            routes::regional_compliance::router(),
        )
        // Migration routes
        .nest("/api/v1/migration", routes::migration::router())
        // AML/DSA Compliance routes
        .nest("/api/v1/aml-dsa", routes::aml_dsa::router())
        // Marketplace routes
        .nest("/api/v1/marketplace", routes::marketplace::router())
        // Competitive Features routes removed (PAP-33): dead scaffold, no migration, no product backing
        // Infrastructure routes
        .nest("/api/v1/infrastructure", routes::infrastructure::router())
        // Operations routes
        .nest("/api/v1/operations", routes::operations::router())
        // Owner Analytics routes
        .nest("/api/v1/owner-analytics", routes::owner_analytics::router())
        // Dispute Resolution routes
        .nest("/api/v1/disputes", routes::disputes::router())
        // Registry routes
        .nest("/api/v1/registry", routes::registry::router())
        // Multi-Currency routes (Epic 145)
        .nest("/api/v1/multi-currency", routes::multi_currency::router())
        // Voice Webhooks routes
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
        // Market Pricing routes (Epic 132). Mounted under both `/pricing` (the
        // canonical path) and the `/market-pricing` alias that the production
        // binary historically exposed; keeping both preserves backwards-compat
        // for clients on either path while the route table is unified (#867).
        .nest("/api/v1/pricing", routes::market_pricing::router())
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
        // Building Certification routes (Epic 137)
        .nest(
            "/api/v1/building-certifications",
            routes::building_certifications::router(),
        )
        // Property Valuation routes (Epic 138). Mounted under both the plural
        // `/property-valuations` (the canonical path) and the singular
        // `/property-valuation` alias the production binary historically
        // exposed; keeping both preserves backwards-compat while the route
        // table is unified (#867).
        .nest(
            "/api/v1/property-valuations",
            routes::property_valuation::router(),
        )
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
        // Reserve Fund routes (Epic 141)
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
        // Data Residency routes (Epic 146)
        .nest("/api/v1/data-residency", routes::data_residency::router())
}

/// Create the application router with all routes.
///
/// This function is exposed for integration testing. It builds on the shared
/// [`route_table`] (the single source of truth for the route set — see #867),
/// then layers the admin-core extensions, tracing, a development CORS policy,
/// and the application state. The production binary in `main.rs` builds from
/// the same [`route_table`] but layers production-only middleware (env-driven
/// CORS, the host-tenant resolution layer, a global body cap) and
/// production-only routes (`/metrics`, Swagger UI, Phase-3 host routing).
pub fn create_router(state: AppState) -> Router {
    // Phase 5 — Admin dependency wiring. Built via [`build_admin_extensions`]
    // so `main.rs::serve` and this test-side router stay in sync; an extension
    // chain divergence between them previously meant 500s on every `/admin/*`
    // call in production while tests passed.
    let admin_ext = build_admin_extensions(state.db.clone());

    // Phase 5 — admin dependency injection. Layered before TraceLayer so every
    // nested route inherits these extensions. Production binary applies the
    // same chain from `main.rs::serve` via `attach_admin_extensions` so the
    // two paths cannot drift.
    attach_admin_extensions(route_table(), &admin_ext)
        // Baseline security headers (HSTS, nosniff, frame-deny, referrer, CSP)
        // on every response (#954). This layer is also applied by the
        // production binary in `main.rs::apply_middleware`; it MUST be mirrored
        // here so the integration-test path (which exercises `create_router`,
        // not `main.rs::serve`) actually verifies the header wiring. Before
        // PR #963's follow-up the layer lived only in `main.rs`, so every
        // integration test ran without security headers and the wiring was
        // never exercised end-to-end (regression guarded by
        // `security_headers_tests.rs` + `router_single_source_tests.rs`).
        .layer(axum::middleware::from_fn(
            api_core::middleware::security_headers,
        ))
        // Middleware
        .layer(TraceLayer::new_for_http())
        // CORS configuration
        // NOTE: `allow_headers` MUST be an explicit list when paired with
        // `allow_credentials(true)`. Per the CORS spec, browsers reject the
        // wildcard with credentials, and tower-http panics at layer
        // construction if the two are combined.
        .layer(
            CorsLayer::new()
                .allow_origin(parse_default_origins())
                .allow_methods([
                    http::Method::GET,
                    http::Method::POST,
                    http::Method::PUT,
                    http::Method::PATCH,
                    http::Method::DELETE,
                    http::Method::OPTIONS,
                ])
                // SECURITY: `x-tenant-context` was removed from the allowlist
                // alongside the deletion of `routes::ai::extract_tenant_context`.
                // Tenancy is now derived from the verified server-side
                // `RequestPrincipal`; trusting a client-supplied header was the
                // root cause of the AI router auth-bypass advisory.
                .allow_headers([
                    http::header::AUTHORIZATION,
                    http::header::CONTENT_TYPE,
                    http::header::ACCEPT,
                    http::header::ORIGIN,
                    http::HeaderName::from_static("x-requested-with"),
                    http::HeaderName::from_static("x-tenant-id"),
                ])
                .allow_credentials(true)
                .max_age(std::time::Duration::from_secs(3600)),
        )
        .layer(Extension(state.clone()))
        // Application state
        .with_state(state)
}
