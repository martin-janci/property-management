//! Reserve Fund Management routes for Epic 141.
//!
//! REST API endpoints for HOA/Condo reserve fund management.
//!
//! Split by surface so each concern lives in its own module (move-only refactor
//! — handlers and behaviour are unchanged from the former single-file
//! `reserve_funds.rs`):
//!
//! | Surface       | Module         | Handlers |
//! |---------------|----------------|----------|
//! | shared        | `shared`       | `ApiResult`, query DTOs, error-mapping helpers |
//! | funds         | `funds`        | list, create, get, update, dashboard, health |
//! | schedules     | `schedules`    | list, create, update contribution schedules |
//! | transactions  | `transactions` | list, record, transfer |
//! | policies      | `policies`     | list, create, get-active investment policies |
//! | projections   | `projections`  | create, current, get-items, add-items |
//! | components    | `components`   | list, create, update |
//! | alerts        | `alerts`       | list, acknowledge, resolve |
//!
//! The query DTO types (`FundListQuery`, `ActiveOnlyQuery`) are re-exported here
//! so `routes::reserve_funds::*` references continue to resolve unchanged.
//!
//! # RLS (PAP-67 / PAP-79)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on the reserve-fund cluster,
//! so every query MUST run on a connection that has `app.current_org_id` set or
//! it collapses to deny-all. Each handler therefore acquires an [`RlsConnection`]
//! (which validates tenant membership and sets the org/user GUCs on a dedicated
//! connection) and passes that connection to the repository (`&mut **rls.conn()`
//! to the generic-`Executor` methods, `rls.conn()` to the `&mut PgConnection`
//! ones, which deref-coerce). The
//! authoritative organization is `rls.tenant_id()` — the tenant the caller was
//! validated against — not a client-supplied `organization_id`, so the SQL org
//! filter and the RLS context can never disagree. Cross-tenant access is blocked
//! by RLS: a by-id read of another org's row returns no row (`404`), and a write
//! targeting another org fails the policy's `WITH CHECK`. `rls.release()` clears
//! the context before the connection returns to the pool (both on the Ok and Err
//! paths).
//!
//! [`RlsConnection`]: api_core::extractors::RlsConnection

mod alerts;
mod components;
mod funds;
mod policies;
mod projections;
mod schedules;
pub mod shared;
mod transactions;

use axum::Router;

use crate::state::AppState;

// Re-export the public query DTOs so external references via
// `routes::reserve_funds::<Type>` keep working after the split.
pub use shared::{ActiveOnlyQuery, FundListQuery};

/// Create the reserve funds router.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(funds::router())
        .merge(schedules::router())
        .merge(transactions::router())
        .merge(policies::router())
        .merge(projections::router())
        .merge(components::router())
        .merge(alerts::router())
}
