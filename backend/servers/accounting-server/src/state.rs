//! Application state for the accounting server.
//!
//! Tenant-scoped resource server (architecture option C): this server VALIDATES
//! JWTs issued by api-server via `api_core::extractors::{AuthUser,
//! RlsConnection}` — it issues no tokens of its own. `AppState` therefore holds
//! only what the accounting handlers need:
//!   * `db`            — the shared RLS-safe Postgres pool,
//!   * `accounting_repo` — the RLS-aware [`db::repositories::AccountingRepository`]
//!     reused from the embedded MVP,
//!   * `config`        — boot-time configuration.
//!
//! Modeled on `api-server`'s `AppState` (the tenant-scoped resource server),
//! NOT `reality-server`'s SSO-consumer state. Phase 1 coders add per-epic
//! services/repos in their OWN modules and request any shared-field additions
//! via the contract-notes flow rather than editing this file.

use api_core::TenantMembershipProvider;
use db::repositories::{
    AccAccountsRepository, AccCatalogRepository, AccConfigRepository, AccContactsRepository,
    AccInvoicingRepository, AccPlatformRepository, AccountingRepository,
};
use db::DbPool;

/// Boot-time configuration for the accounting server.
///
/// Minimal for the Phase 0a skeleton. Mirrors the api-server convention of
/// loading from environment at startup so handlers never read the environment
/// on the hot path.
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// Deployment region label (for health/observability output).
    pub region: String,
}

impl AppConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        Self {
            region: std::env::var("REGION").unwrap_or_else(|_| "local".to_string()),
        }
    }
}

/// Application state shared across all accounting handlers.
///
/// Each per-epic repository is RLS-aware (generic-executor pattern). Handlers
/// take `RlsConnection` and pass `&mut **rls` / a transaction into these repos.
#[derive(Clone)]
pub struct AppState {
    /// Shared RLS-safe Postgres connection pool.
    pub db: DbPool,
    /// RLS-aware base accounting repository reused from the embedded MVP
    /// (base invoice/contact/bank CRUD on the 00184 tables).
    pub accounting_repo: AccountingRepository,
    /// EPIC-ACC-01 — accounts & company access (BE-Accounts).
    pub accounts_repo: AccAccountsRepository,
    /// EPIC-ACC-02 — company & document configuration (BE-Config).
    pub config_repo: AccConfigRepository,
    /// EPIC-ACC-03 — contacts & CRM (BE-Contacts+Catalog).
    pub contacts_repo: AccContactsRepository,
    /// EPIC-ACC-04 — product & price-list catalog (BE-Contacts+Catalog).
    pub catalog_repo: AccCatalogRepository,
    /// EPIC-ACC-05 — sales-invoicing extensions (BE-Invoicing).
    pub invoicing_repo: AccInvoicingRepository,
    /// EPIC-ACC-16 — platform/security (BE-Platform).
    pub platform_repo: AccPlatformRepository,
    /// Boot-time configuration.
    pub config: AppConfig,
}

impl AppState {
    /// Create a new [`AppState`] from the shared database pool.
    pub fn new(db: DbPool) -> Self {
        let accounting_repo = AccountingRepository::new(db.clone());
        let accounts_repo = AccAccountsRepository::new(db.clone());
        let config_repo = AccConfigRepository::new(db.clone());
        let contacts_repo = AccContactsRepository::new(db.clone());
        let catalog_repo = AccCatalogRepository::new(db.clone());
        let invoicing_repo = AccInvoicingRepository::new(db.clone());
        let platform_repo = AccPlatformRepository::new(db.clone());
        let config = AppConfig::from_env();
        Self {
            db,
            accounting_repo,
            accounts_repo,
            config_repo,
            contacts_repo,
            catalog_repo,
            invoicing_repo,
            platform_repo,
            config,
        }
    }
}

// SECURITY: Implement TenantMembershipProvider so the api-core auth/RLS
// extractors (`AuthUser`, `RlsConnection`) can obtain the db pool from this
// state to re-derive the request principal and enforce tenant membership —
// exactly as api-server does. The accounting server is a resource server:
// it validates the JWT and resolves tenant scope, but issues no tokens.
impl TenantMembershipProvider for AppState {
    fn db_pool(&self) -> &DbPool {
        &self.db
    }
}
