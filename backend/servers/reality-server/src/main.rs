#![allow(clippy::doc_overindented_list_items)]

//! Reality Server - Public Real Estate Portal
//!
//! Public-facing API for the Reality Portal.
//! Serves property listings, search, favorites, and inquiries.
//! Supports SSO with Property Management system.
//!
//! Package: ppt::reality_server

// Allow dead code for stub implementations during development
#![allow(dead_code)]

use axum::{extract::DefaultBodyLimit, http, routing::get, Router};
use db::models::{
    AddFavorite, CreateAgencyInvitation, CreateFeedSubscription, CreateListingInquiry,
    CreatePortalImportJob, CreatePortalSavedSearch, CreateRealityAgency, CreateRealtorProfile,
    InquiryMessage, ListingInquiry, PortalFavorite, PortalFavoriteWithListing, PortalImportJob,
    PortalImportJobWithStats, PortalSavedSearch, PublicListingSearchResponse, RealityAgency,
    RealityAgencyInvitation, RealityAgencyMember, RealityFeedSubscription, RealtorProfile,
    SendInquiryMessage, UpdateAgencyBranding, UpdateFeedSubscription, UpdatePortalImportJob,
    UpdatePortalSavedSearch, UpdateRealityAgency, UpdateRealtorProfile,
};
use http::HeaderValue;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod extractors;
mod handlers;
mod observability;
mod routes;
mod services;
pub mod state;
mod util;

use state::AppState;

/// Default CORS allowed origins for reality-server.
/// Includes development origins and production domains for all regional portals.
///
/// # Production Deployment
/// In production, set the `CORS_ALLOWED_ORIGINS` environment variable to
/// restrict origins to only the necessary production domains:
/// ```bash
/// CORS_ALLOWED_ORIGINS=https://reality-portal.sk,https://reality-portal.cz,https://reality-portal.eu
/// ```
/// This prevents localhost origins from being accepted in production.
const DEFAULT_CORS_ORIGINS: &[&str] = &[
    "http://localhost:3000",             // ppt-web dev
    "http://localhost:3001",             // reality-web dev
    "http://localhost:8080",             // api-server dev
    "http://localhost:8081",             // reality-server dev (swagger-ui)
    "https://ppt.three-two-bit.com",     // production
    "https://reality.three-two-bit.com", // reality production
    "https://reality-portal.sk",         // Slovakia portal
    "https://reality-portal.cz",         // Czech portal
    "https://reality-portal.eu",         // EU portal
];

/// Check if an origin string contains a wildcard pattern.
/// Wildcards are not allowed with allow_credentials(true).
fn is_wildcard_origin(origin: &str) -> bool {
    origin == "*" || origin.contains("*.")
}

/// Parse CORS allowed origins from environment variable.
///
/// Reads `CORS_ALLOWED_ORIGINS` environment variable as a comma-separated list of origins.
/// Falls back to default origins if not set.
///
/// # Security Note
/// Wildcard origins ("*" or "*.example.com") are explicitly rejected when
/// credentials are enabled. This prevents security vulnerabilities where
/// any origin could access authenticated resources.
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
                    // Security: Reject wildcard origins when credentials are enabled
                    if is_wildcard_origin(origin) {
                        tracing::error!(
                            "Wildcard CORS origin '{}' rejected - not allowed with credentials",
                            origin
                        );
                        return None;
                    }
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
        title = "Reality Portal API",
        version = "1.0.0",
        description = "Public API for Reality Portal - Real Estate Listings",
        contact(name = "PPT Team", email = "reality@ppt.example.com"),
        license(name = "MIT")
    ),
    servers(
        (url = "http://localhost:8081", description = "Local development"),
        (url = "https://api.reality-portal.sk", description = "Slovakia"),
        (url = "https://api.reality-portal.cz", description = "Czech Republic"),
        (url = "https://api.reality-portal.eu", description = "EU-wide")
    ),
    paths(
        routes::health::liveness,
        routes::health::readiness,
        routes::listings::search,
        routes::listings::get_listing,
        routes::listings::get_suggestions,
        routes::listings::get_featured,
        routes::listings::get_categories,
        routes::listings::record_view,
        routes::favorites::list_favorites,
        routes::favorites::list_favorite_ids,
        routes::favorites::add_favorite,
        routes::favorites::remove_favorite,
        routes::favorites::check_favorite,
        routes::saved_searches::list_saved_searches,
        routes::saved_searches::create_saved_search,
        routes::saved_searches::get_saved_search,
        routes::saved_searches::update_saved_search,
        routes::saved_searches::delete_saved_search,
        routes::saved_searches::run_saved_search,
        routes::sso::sso_login,
        routes::sso::sso_callback,
        routes::sso::sso_logout,
        routes::sso::create_mobile_sso_token,
        routes::sso::validate_mobile_sso_token,
        routes::sso::get_session,
        routes::sso::refresh_session,
        // Epic 32: Agencies
        routes::agencies::create_agency,
        routes::agencies::get_agency,
        routes::agencies::get_agency_by_slug,
        routes::agencies::update_agency,
        routes::agencies::update_branding,
        routes::agencies::list_members,
        routes::agencies::create_invitation,
        routes::agencies::accept_invitation,
        // Epic 33: Realtors
        routes::realtors::get_my_profile,
        routes::realtors::get_profile,
        routes::realtors::create_profile,
        routes::realtors::update_profile,
        routes::realtors::list_inquiries,
        routes::realtors::mark_inquiry_read,
        routes::realtors::respond_to_inquiry,
        // Epic 34: Imports
        routes::imports::list_import_jobs,
        routes::imports::create_import_job,
        routes::imports::get_import_job,
        routes::imports::update_import_job,
        routes::imports::start_import_job,
        routes::imports::cancel_import_job,
        routes::imports::list_feeds,
        routes::imports::create_feed,
        routes::imports::get_feed,
        routes::imports::update_feed,
        routes::imports::sync_feed,
        // Portal listing CRUD (Epic 15.1/15.2)
        routes::portal_listings::create_listing,
        routes::portal_listings::get_my_listing,
        routes::portal_listings::update_listing,
        routes::portal_listings::list_my_listings,
        routes::portal_listings::get_my_listing_analytics,
        // Compare (UC-48)
        routes::compare::get_compare_list,
        routes::compare::add_to_compare,
        routes::compare::remove_from_compare,
        // Reports (UC-23)
        routes::reports::submit_report,
        routes::reports::list_my_reports,
        // Agent reviews (UC-49, UC-51)
        routes::agent_reviews::list_reviews,
        routes::agent_reviews::create_review,
        // Agency branding (UC-49)
        routes::agency_branding::get_branding,
        routes::agency_branding::update_branding,
        // Agency imports (UC-50)
        routes::agency_imports::list_import_history,
        routes::agency_imports::test_connection,
        routes::agency_imports::run_import,
        routes::agency_imports::get_import_job_status,
        // Price map (UC-31)
        routes::price_map::get_price_map,
        // Articles / Journal (UC-13)
        routes::articles::list_articles,
        routes::articles::get_article,
        routes::articles::list_comments,
        routes::articles::create_comment,
    ),
    components(schemas(
        routes::health::HealthResponse,
        routes::health::LivenessResponse,
        routes::health::CacheMetricsResponse,
        routes::health::CacheMetricsDetail,
        routes::listings::ListingSearchRequest,
        routes::listings::ListingSearchResponse,
        routes::listings::ListingSummary,
        routes::listings::ListingDetail,
        routes::listings::SuggestionsResponse,
        routes::listings::FeaturedListingsResponse,
        routes::listings::RichListingSummary,
        routes::listings::RichListingAddress,
        routes::listings::RichListingPhoto,
        routes::listings::CategoryCount,
        routes::favorites::CheckFavoriteResponse,
        routes::favorites::FavoritesResponse,
        routes::saved_searches::SavedSearchesResponse,
        routes::saved_searches::RunSavedSearchResponse,
        AddFavorite,
        PortalFavorite,
        PortalFavoriteWithListing,
        PublicListingSearchResponse,
        CreatePortalSavedSearch,
        UpdatePortalSavedSearch,
        PortalSavedSearch,
        routes::sso::SsoError,
        routes::sso::SsoUserInfo,
        routes::sso::SessionInfo,
        routes::sso::CreateMobileSsoTokenRequest,
        routes::sso::MobileSsoTokenResponse,
        routes::sso::ValidateMobileSsoTokenRequest,
        routes::sso::SessionResponse,
        // Epic 32: Agencies
        routes::agencies::AgencyResponse,
        routes::agencies::MembersResponse,
        routes::agencies::AcceptInvitationRequest,
        CreateRealityAgency,
        UpdateRealityAgency,
        UpdateAgencyBranding,
        CreateAgencyInvitation,
        RealityAgency,
        RealityAgencyMember,
        RealityAgencyInvitation,
        // Epic 33: Realtors
        routes::realtors::ProfileResponse,
        routes::realtors::InquiriesResponse,
        routes::realtors::InquiriesQuery,
        CreateRealtorProfile,
        UpdateRealtorProfile,
        RealtorProfile,
        CreateListingInquiry,
        SendInquiryMessage,
        ListingInquiry,
        InquiryMessage,
        // Epic 34: Imports
        routes::imports::ImportJobsResponse,
        routes::imports::ImportJobResponse,
        routes::imports::ImportJobsQuery,
        routes::imports::FeedsResponse,
        routes::imports::FeedResponse,
        CreatePortalImportJob,
        UpdatePortalImportJob,
        PortalImportJob,
        PortalImportJobWithStats,
        CreateFeedSubscription,
        UpdateFeedSubscription,
        RealityFeedSubscription,
        // Portal listing CRUD (Epic 15.1/15.2)
        routes::portal_listings::PortalListingResponse,
        routes::portal_listings::CreatePortalListingRequest,
        routes::portal_listings::UpdatePortalListingRequest,
        routes::portal_listings::MyListingsResponse,
        routes::portal_listings::ListingAnalyticsResponse,
        routes::portal_listings::DailyListingAnalytics,
        // Compare (UC-48)
        routes::compare::CompareEntry,
        routes::compare::CompareListResponse,
        routes::compare::AddCompareResponse,
        // Reports (UC-23)
        routes::reports::SubmitReportRequest,
        routes::reports::ListingReport,
        routes::reports::SubmitReportResponse,
        routes::reports::MyReportsResponse,
        routes::reports::ProblemType,
        // Agent reviews (UC-49, UC-51)
        routes::agent_reviews::RealtorReview,
        routes::agent_reviews::CreateReviewRequest,
        routes::agent_reviews::ReviewsResponse,
        // Agency branding (UC-49)
        routes::agency_branding::AgencyBranding,
        routes::agency_branding::UpdateBrandingRequest,
        routes::agency_branding::BrandingResponse,
        routes::agency_branding::WatermarkPosition,
        routes::agency_branding::WatermarkStyle,
        // Agency imports (UC-50)
        routes::agency_imports::ImportJobSummary,
        routes::agency_imports::ImportHistoryResponse,
        routes::agency_imports::TestConnectionRequest,
        routes::agency_imports::TestConnectionResponse,
        routes::agency_imports::RunImportRequest,
        routes::agency_imports::RunImportResponse,
        routes::agency_imports::ImportJobDetail,
        routes::agency_imports::ImportJobDetailResponse,
        routes::agency_imports::ImportProvider,
        // Price map (UC-31)
        routes::price_map::DistrictPriceData,
        routes::price_map::PriceMapResponse,
        // Articles / Journal (UC-13)
        routes::articles::ArticleSummary,
        routes::articles::ArticleDetail,
        routes::articles::RelatedArticle,
        routes::articles::ArticlesListResponse,
        routes::articles::ArticleDetailResponse,
        routes::articles::ArticleComment,
        routes::articles::CommentsResponse,
        routes::articles::CreateCommentRequest,
    )),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Listings", description = "Public listing search and detail"),
        (name = "SSO", description = "Single Sign-On with Property Management"),
        (name = "Users", description = "Portal user accounts (separate from PM)"),
        (name = "Favorites", description = "Save and manage favorite listings"),
        (name = "SavedSearches", description = "Saved search criteria and alerts"),
        (name = "Inquiries", description = "Contact and viewing requests"),
        (name = "Agencies", description = "Real estate agency management (Epic 32)"),
        (name = "Realtors", description = "Realtor profiles and tools (Epic 33)"),
        (name = "Imports", description = "Property import and feed management (Epic 34)"),
        (name = "PortalListings", description = "Owner/realtor listing CRUD (Epic 15.1/15.2)"),
        (name = "Compare", description = "Compare up to 4 listings side-by-side (UC-48)"),
        (name = "Reports", description = "Report problematic listings (UC-23)"),
        (name = "AgentReviews", description = "Realtor reviews and ratings (UC-49, UC-51)"),
        (name = "AgencyBranding", description = "Agency branding settings (UC-49)"),
        (name = "AgencyImport", description = "Per-agency import management (UC-50)"),
        (name = "PriceMap", description = "District price aggregations (UC-31)"),
        (name = "Articles", description = "Journal and news articles (UC-13)")
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
        "Reality Server v{} starting with observability enabled",
        env!("CARGO_PKG_VERSION")
    );

    // Get database URL
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost:5432/ppt".to_string());

    // Create RLS-safe database pool with automatic context cleanup
    let db = db::create_rls_safe_pool(&database_url).await?;
    tracing::info!("Connected to database with RLS-safe pool");

    // Apply any pending migrations. Same migration set as api-server (both
    // share the per-target Postgres database). Concurrency-safe via sqlx's
    // advisory lock — if api-server happens to be migrating in parallel
    // (typical blue/green spin-up), this call blocks then sees zero pending.
    // Required on first deploy of a fresh target where the database was
    // created empty from `ppt_dev_template`.
    // `.context()` preserves the underlying `MigrateError` as the source
    // for anyhow's chained-error rendering — `map_err(|e| anyhow!("{e}"))`
    // would have flattened the cause into a single string and lost the
    // backtrace.
    use anyhow::Context;
    db::run_migrations(&db)
        .await
        .context("DB migration failed")?;
    tracing::info!("Database migrations applied (or already current)");

    // Phase 1: Build the host-resolution config + shared tenant-resolution
    // cache ONCE, then clone the `Arc` into both the middleware config and the
    // AppState (the field name `tenant_resolution_cache` is a contract with
    // platform-admin handlers that invalidate cache entries).
    //
    // Phase 5.5: same pattern for the per-tenant rate limiter set (defense
    // leak #15) — the middleware enforces the limit and admin handlers
    // install per-tenant overrides via
    // `state.tenant_rate_limiters.set_override(org, rpm)`.
    let host_tenant_config = api_core::middleware::HostTenantConfig::new(db.clone());
    let tenant_resolution_cache = host_tenant_config.cache.clone();
    let tenant_rate_limiters = host_tenant_config.rate_limiters.clone();

    // Story 16.3 / #983: start the saved-search alert matching engine. It polls
    // alert-enabled saved searches against newly published listings and enqueues
    // alerts. Disabled-safe via SAVED_SEARCH_ALERT_* env.
    let alert_worker = services::SavedSearchAlertWorker::new(
        db.clone(),
        services::SavedSearchAlertConfig::from_env(),
    );
    let _alert_worker_handle = alert_worker.start();

    // BIT-139 / Epic 16: drain enqueued alerts to the email + push transports.
    // Reads the same `search_alert_queue`, fans out to each owner's
    // `device_push_tokens`, and marks delivery via `notified_at` (independent of
    // the in-app read `status`). Transports are logging stubs until a real
    // email/push service is wired; disabled-safe via SEARCH_ALERT_DRAINER_* env.
    let drainer_worker = services::SearchAlertDrainerWorker::new(
        db.clone(),
        services::SearchAlertDrainerConfig::from_env(),
    );
    let _drainer_worker_handle = drainer_worker.start();

    let favorite_alert_worker =
        services::FavoriteAlertWorker::new(db.clone(), services::FavoriteAlertConfig::from_env());
    let _favorite_alert_worker_handle = favorite_alert_worker.start();

    // Create application state
    let state = AppState::new(db, tenant_resolution_cache, tenant_rate_limiters);

    // Build router with state
    let app = Router::new()
        // Health (liveness) — shallow, no deps. Docker HEALTHCHECK target.
        .route("/health", get(routes::health::liveness))
        // Readiness — deep dep check (DB + PM API). Operator dashboards.
        .route("/readiness", get(routes::health::readiness))
        // Prometheus metrics endpoint (Epic 95.4)
        .route("/metrics", get(metrics_endpoint))
        // Public listing routes
        .nest("/api/v1/listings", routes::listings::router())
        // Portal user routes
        .nest("/api/v1/users", routes::users::router())
        // Favorites routes
        .nest("/api/v1/favorites", routes::favorites::router())
        // Saved searches routes
        .nest("/api/v1/saved-searches", routes::saved_searches::router())
        // Inquiries routes
        .nest("/api/v1/inquiries", routes::inquiries::router())
        // SSO routes (Epic 10A-SSO)
        .nest("/api/v1/sso", routes::sso::router())
        // Agency routes (Epic 32)
        .nest("/api/v1/agencies", routes::agencies::router())
        // Realtor routes (Epic 33)
        .nest("/api/v1/realtors", routes::realtors::router())
        // Import routes (Epic 34)
        .nest("/api/v1/imports", routes::imports::router())
        // Portal listing CRUD (Epic 15.1/15.2 — owner/realtor edit)
        .nest("/api/v1/my/listings", routes::portal_listings::router())
        // Compare routes (UC-48)
        .nest("/api/v1/compare", routes::compare::router())
        // Reports routes (UC-23)
        .nest("/api/v1/reports", routes::reports::router())
        // Agent reviews — nested under realtors (UC-49, UC-51)
        .nest(
            "/api/v1/realtors/{id}/reviews",
            routes::agent_reviews::router(),
        )
        // Agency branding (UC-49) — nested under /:id/branding to avoid prefix
        // collision with routes::agencies (Axum .nest() shadows on same prefix).
        .nest(
            "/api/v1/agencies/{id}/branding",
            routes::agency_branding::router(),
        )
        // Agency imports (UC-50)
        .nest(
            "/api/v1/agencies/{id}/imports",
            routes::agency_imports::router(),
        )
        // Price map aggregations (UC-31)
        .nest("/api/v1/price-map", routes::price_map::router())
        // Journal / News articles (UC-13)
        .nest("/api/v1/articles", routes::articles::router())
        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Add state
        .with_state(state)
        // P0-15: global request body cap. Reality-server is public-facing
        // and accepts no large uploads; 4 MiB is more than enough for
        // JSON+forms.
        .layer(DefaultBodyLimit::max(4 * 1024 * 1024))
        // Baseline security headers (HSTS, nosniff, frame-deny, referrer) on
        // every response — the API edge previously shipped none (#954).
        .layer(axum::middleware::from_fn(
            api_core::middleware::security_headers,
        ))
        // Middleware
        .layer(TraceLayer::new_for_http())
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
        // Phase 1: Host-resolution (tenant-resolution) middleware. Runs FIRST
        // on the request pipeline (layers execute outside-in), inspecting the
        // Host header to inject a `ResolvedTenant` extension before any
        // handler/extractor runs. Public-allowlist paths (`/health`, etc.)
        // bypass resolution; unknown hosts fail closed with 404.
        .layer(axum::middleware::from_fn_with_state(
            host_tenant_config.clone(),
            api_core::middleware::host_tenant_middleware,
        ));

    // Run server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
    tracing::info!("Reality server (Public Portal) listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

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
