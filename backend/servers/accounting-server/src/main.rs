#![allow(clippy::doc_overindented_list_items)]

//! Accounting Server — Invoicing & Accounting MVP (ACC).
//!
//! Tenant-scoped resource server (architecture option C). Binds 0.0.0.0:8082.
//! VALIDATES JWTs issued by api-server (via `api_core::extractors`) and shares
//! the same Postgres database + RLS. Issues NO tokens of its own.
//!
//! Package: ppt::accounting_server

// Allow dead code for the Phase 0a skeleton — the route/domain modules are
// stubs until Phase 1 coders implement them. Mirrors reality-server / api-server.
#![allow(dead_code)]
#![allow(unused)]

use axum::{extract::DefaultBodyLimit, http, routing::get, Router};
use http::HeaderValue;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod observability;
mod routes;
pub mod state;

use state::AppState;

/// Default CORS allowed origins for accounting-server.
///
/// Includes development origins (accounting-web on :3002, this server's
/// Swagger UI on :8082) plus production placeholders. In production, set
/// `CORS_ALLOWED_ORIGINS` to restrict to the necessary domains.
const DEFAULT_CORS_ORIGINS: &[&str] = &[
    "http://localhost:3002",         // accounting-web dev
    "http://localhost:8082",         // accounting-server dev (swagger-ui)
    "https://acc.three-two-bit.com", // production placeholder
];

/// Check if an origin string contains a wildcard pattern.
/// Wildcards are not allowed with allow_credentials(true).
fn is_wildcard_origin(origin: &str) -> bool {
    origin == "*" || origin.contains("*.")
}

/// Parse CORS allowed origins from the `CORS_ALLOWED_ORIGINS` env var
/// (comma-separated), falling back to [`DEFAULT_CORS_ORIGINS`]. Wildcard
/// origins are rejected because credentials are enabled.
fn get_cors_allowed_origins() -> Vec<HeaderValue> {
    match std::env::var("CORS_ALLOWED_ORIGINS") {
        Ok(origins_str) if !origins_str.is_empty() => {
            let origins: Vec<HeaderValue> = origins_str
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .filter_map(|origin| {
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

/// Parse the default origins into a `HeaderValue` vector.
fn parse_default_origins() -> Vec<HeaderValue> {
    DEFAULT_CORS_ORIGINS
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect()
}

/// Build the production CORS layer. `allow_headers` MUST be an explicit list
/// when paired with `allow_credentials(true)` — tower-http panics otherwise.
fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(get_cors_allowed_origins())
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
        ])
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(3600))
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Accounting API",
        version = "1.0.0",
        description = "Invoicing & Accounting MVP API (ACC)",
        contact(name = "PPT Team", email = "accounting@ppt.example.com"),
        license(name = "MIT")
    ),
    servers(
        (url = "http://localhost:8082", description = "Local development")
    ),
    paths(
        routes::health::liveness,
        routes::health::readiness,
        // EPIC-ACC-01 — Accounts
        routes::accounts::list_my_companies,
        routes::accounts::switch_company,
        routes::accounts::get_company,
        routes::accounts::create_company,
        routes::accounts::invite_user,
        routes::accounts::set_member_role,
        routes::accounts::deactivate_member,
        routes::accounts::grant_accountant_access,
        routes::accounts::revoke_accountant_access,
        routes::accounts::get_billing,
        // EPIC-ACC-02 — Config
        routes::config::get_company_settings,
        routes::config::upsert_company_settings,
        routes::config::list_numbering_series,
        routes::config::create_numbering_series,
        routes::config::list_units,
        routes::config::list_vat_rates,
        routes::config::list_bank_accounts,
        // EPIC-ACC-03 — Contacts
        routes::contacts::list_contacts,
        routes::contacts::get_contact,
        routes::contacts::create_contact,
        routes::contacts::update_contact,
        routes::contacts::deactivate_contact,
        routes::contacts::merge_contact,
        routes::contacts::list_addresses,
        // EPIC-ACC-04 — Catalog
        routes::catalog::list_items,
        routes::catalog::get_item,
        routes::catalog::create_item,
        routes::catalog::update_item,
        routes::catalog::list_categories,
        routes::catalog::list_price_levels,
        // EPIC-ACC-05 — Invoices
        routes::invoices::list_invoices,
        routes::invoices::get_invoice,
        routes::invoices::list_invoice_items,
        routes::invoices::get_invoice_qr,
        routes::invoices::create_invoice,
        routes::invoices::update_invoice,
        routes::invoices::delete_invoice,
        routes::invoices::issue_invoice,
        routes::invoices::set_exchange_rate,
        routes::invoices::create_credit_note,
        routes::invoices::duplicate_invoice,
        routes::invoices::list_links,
        // EPIC-ACC-16 — Platform
        routes::platform::list_audit_log,
        routes::platform::add_tag,
        routes::platform::create_share_link,
        routes::platform::revoke_share_link,
        routes::platform::view_shared_invoice,
        routes::platform::enroll_two_factor,
        routes::platform::confirm_two_factor,
    ),
    components(schemas(
        routes::health::LivenessResponse,
        routes::health::HealthResponse,
        routes::health::HealthStatus,
        routes::health::DependencyHealth,
        // EPIC-ACC-01
        routes::accounts::CompanyAccessDto,
        routes::accounts::MyCompaniesResponse,
        routes::accounts::SwitchCompanyRequest,
        routes::accounts::CreateCompanyRequest,
        routes::accounts::InviteUserRequest,
        routes::accounts::SetMemberRoleRequest,
        routes::accounts::GrantAccountantRequest,
        routes::accounts::MembershipMutationResponse,
        routes::accounts::BillingOverviewResponse,
        // EPIC-ACC-02
        routes::config::UpsertCompanySettingsRequest,
        routes::config::CreateNumberingSeriesRequest,
        db::models::acc_config::AccCompanySettings,
        db::models::acc_config::AccNumberingSeries,
        db::models::acc_config::AccUnit,
        db::models::acc_config::AccVatRate,
        db::models::acc_config::AccBankAccount,
        // EPIC-ACC-03
        routes::contacts::ContactRequest,
        routes::contacts::MergeContactRequest,
        db::models::acc_contacts_ext::AccContactExt,
        db::models::acc_contacts_ext::AccContactAddress,
        // EPIC-ACC-04
        routes::catalog::CatalogItemRequest,
        routes::catalog::PriceLevelRequest,
        db::models::acc_catalog::AccCatalogItem,
        db::models::acc_catalog::AccItemCategory,
        db::models::acc_catalog::AccPriceLevel,
        // EPIC-ACC-05
        routes::invoices::PaymentQrResponse,
        routes::invoices::IssueInvoiceRequest,
        routes::invoices::CreateCreditNoteRequest,
        routes::invoices::SetExchangeRateRequest,
        routes::invoices::CreateLinkRequest,
        db::models::accounting::Invoice,
        db::models::accounting::InvoiceItem,
        db::models::accounting::CreateInvoice,
        db::models::accounting::UpdateInvoice,
        db::models::accounting::CreateInvoiceItem,
        db::models::acc_invoicing_ext::AccInvoiceExt,
        db::models::acc_invoicing_ext::AccDocumentLink,
        // EPIC-ACC-16
        routes::platform::CreateShareLinkRequest,
        routes::platform::SharedInvoiceView,
        routes::platform::AddTagRequest,
        routes::platform::TwoFactorEnrollResponse,
        routes::platform::TwoFactorConfirmRequest,
        db::models::acc_platform::AccAuditLog,
        db::models::acc_platform::AccShareLink,
        db::models::acc_platform::AccTag,
    )),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Accounts", description = "Accounts, companies & access (EPIC-ACC-01)"),
        (name = "Config", description = "Company & document configuration (EPIC-ACC-02)"),
        (name = "Contacts", description = "Contacts & CRM (EPIC-ACC-03)"),
        (name = "Catalog", description = "Product & price-list catalog (EPIC-ACC-04)"),
        (name = "Invoices", description = "Sales invoicing (EPIC-ACC-05)"),
        (name = "Platform", description = "Security, audit, sharing & 2FA (EPIC-ACC-16)")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // OpenAPI export mode (DB-free): `accounting-server --export-openapi [path]`
    // dumps the utoipa `ApiDoc` spec as JSON and exits. Used by the
    // `@ppt/accounting-api-client` SDK generation step (CONTRACT §8). Runs before
    // any DB connection so it works in CI / fresh worktrees with no Postgres.
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--export-openapi") {
        let out_path = args
            .iter()
            .position(|a| a == "--export-openapi")
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| "openapi.json".to_string());
        let json = ApiDoc::openapi().to_pretty_json()?;
        std::fs::write(&out_path, json)?;
        eprintln!("Wrote OpenAPI spec to {out_path}");
        return Ok(());
    }

    // Load .env file if present.
    dotenvy::dotenv().ok();

    // Initialize observability (Epic 95). The guard MUST remain in scope for
    // the entire application lifetime — dropping it shuts down Sentry.
    #[allow(unused_variables)]
    let observability_guard = observability::init_observability(
        observability::OtelConfig::default(),
        observability::SentryConfig::default(),
        observability::MetricsConfig::default(),
    );

    tracing::info!(
        "Accounting Server v{} starting with observability enabled",
        env!("CARGO_PKG_VERSION")
    );

    // Get database URL (shared with api-server / reality-server).
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost:5432/ppt".to_string());

    // Create RLS-safe database pool with automatic context cleanup.
    let db = db::create_rls_safe_pool(&database_url).await?;
    tracing::info!("Connected to database with RLS-safe pool");

    // Apply any pending migrations. Same migration set as api-server (all
    // servers share the per-target Postgres database). Concurrency-safe via
    // sqlx's advisory lock.
    use anyhow::Context;
    db::run_migrations(&db)
        .await
        .context("DB migration failed")?;
    tracing::info!("Database migrations applied (or already current)");

    // Create application state.
    let state = AppState::new(db);

    // Build router with state.
    let app = Router::new()
        // Health (liveness) — shallow, no deps. Docker HEALTHCHECK target.
        .route("/health", get(routes::health::liveness))
        // Readiness — deep dep check (DB). Operator dashboards.
        .route("/readiness", get(routes::health::readiness))
        // Prometheus metrics endpoint (Epic 95.4).
        .route("/metrics", get(metrics_endpoint))
        // ACC REST surface (EPIC-ACC-01..05, 16). Each handler is a todo!()
        // stub in Phase 0b; Phase 1 coders fill bodies in their own route file.
        .nest("/api/v1", routes::api_router())
        // Swagger UI.
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Bind state.
        .with_state(state)
        // Global request body cap (JSON-only API; 4 MiB is generous).
        .layer(DefaultBodyLimit::max(4 * 1024 * 1024))
        // Baseline security headers (HSTS, nosniff, frame-deny, referrer).
        .layer(axum::middleware::from_fn(
            api_core::middleware::security_headers,
        ))
        // Tracing.
        .layer(TraceLayer::new_for_http())
        // CORS (origins configurable via CORS_ALLOWED_ORIGINS env var).
        .layer(cors_layer());

    // Run server.
    let addr = SocketAddr::from(([0, 0, 0, 0], 8082));
    tracing::info!("Accounting server listening on {}", addr);

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
