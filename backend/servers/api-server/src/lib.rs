//! API Server library for Property Management System.
//!
//! This module exposes the application components for integration testing.

// Allow dead code for stub implementations during development
#![allow(dead_code)]

pub mod handlers;
pub mod observability;
pub mod routes;
pub mod services;
pub mod state;

use axum::{http, routing::get, Router};
use http::HeaderValue;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

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

/// Create the application router with all routes.
///
/// This function is exposed for integration testing.
pub fn create_router(state: AppState) -> Router {
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
        .nest(
            "/api/v1/admin",
            routes::admin_tenant_lifecycle::router(),
        )
        // Organizations routes
        .nest("/api/v1/organizations", routes::organizations::router())
        // Buildings routes
        .nest("/api/v1/buildings", routes::buildings::router())
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
        // Messaging routes
        .nest("/api/v1/messages", routes::messaging::router())
        // Neighbor routes
        .nest("/api/v1", routes::neighbors::router())
        // Notification preferences routes
        .nest(
            "/api/v1/users/me/notification-preferences",
            routes::notification_preferences::router(),
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
        // MFA routes
        .nest("/api/v1/auth/mfa", routes::mfa::router())
        // OAuth routes
        .nest("/api/v1/oauth", routes::oauth::router())
        .nest("/api/v1/admin/oauth", routes::oauth::admin_router())
        // Platform Admin routes
        .nest("/api/v1/platform-admin", routes::platform_admin::router())
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
        // Meters routes
        .nest("/api/v1/meters", routes::meters::router())
        // AI routes
        .nest("/api/v1/ai/chat", routes::ai::ai_chat_router())
        .nest("/api/v1/ai/sentiment", routes::ai::sentiment_router())
        .nest("/api/v1/ai/equipment", routes::ai::equipment_router())
        .nest("/api/v1/ai/workflows", routes::ai::workflow_router())
        .nest("/api/v1/ai/llm", routes::ai::llm_router())
        // IoT routes
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
        // Public API routes
        .nest("/api/v1/developer", routes::public_api::router())
        // Competitive Features routes
        .nest("/api/v1/competitive", routes::competitive::router())
        // Infrastructure routes
        .nest("/api/v1/infrastructure", routes::infrastructure::router())
        // Operations routes
        .nest("/api/v1/operations", routes::operations::router())
        // Owner Analytics routes
        .nest("/api/v1/owner-analytics", routes::owner_analytics::router())
        // Dispute Resolution routes
        .nest("/api/v1/disputes", routes::disputes::router())
        // Vendor Portal routes
        .nest("/api/v1/vendor-portal", routes::vendor_portal::router())
        // Registry routes
        .nest("/api/v1/registry", routes::registry::router())
        // Voice Webhooks routes
        .nest(
            "/api/v1/webhooks/voice",
            routes::voice_webhooks::voice_webhook_router(),
        )
        // Features routes (Epic 109)
        .nest("/api/v1/features", routes::features::router())
        // Outages routes (UC-12)
        .nest("/api/v1/outages", routes::outages::router())
        // Market Pricing routes (Epic 132)
        .nest("/api/v1/pricing", routes::market_pricing::router())
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
        // Property Valuation routes (Epic 138)
        .nest(
            "/api/v1/property-valuations",
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
        // Data Residency routes (Epic 146)
        .nest("/api/v1/data-residency", routes::data_residency::router())
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
                .allow_headers([
                    http::header::AUTHORIZATION,
                    http::header::CONTENT_TYPE,
                    http::header::ACCEPT,
                    http::header::ORIGIN,
                    http::HeaderName::from_static("x-requested-with"),
                    http::HeaderName::from_static("x-tenant-id"),
                    http::HeaderName::from_static("x-tenant-context"),
                ])
                .allow_credentials(true)
                .max_age(std::time::Duration::from_secs(3600)),
        )
        // Application state
        .with_state(state)
}
