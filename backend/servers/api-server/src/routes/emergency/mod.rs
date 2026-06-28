//! Emergency management routes for Epic 23.
//!
//! Handles emergency protocols, contacts, incidents, broadcasts, and drills.
//! Split by surface so each concern lives in its own module:
//!
//! | Surface     | Module       | Handlers |
//! |-------------|--------------|----------|
//! | shared      | `shared`     | Request/query DTOs, `From` conversions, manager-gate helper |
//! | protocols   | `protocols`  | create, list, get, update, delete |
//! | contacts    | `contacts`   | create, list, get, update, delete |
//! | incidents   | `incidents`  | CRUD + acknowledge/resolve/close + attachments + updates |
//! | broadcasts  | `broadcasts` | create, list, get, deactivate, acknowledge, acknowledgments |
//! | drills      | `drills`     | CRUD + start/complete/cancel + upcoming |
//! | statistics  | `statistics` | statistics, incidents-by-type, incidents-by-severity |
//!
//! Public DTO types are re-exported here so `routes::emergency::*` references
//! continue to resolve unchanged.
//!
//! # RLS routing (PAP-80)
//!
//! Every handler acquires an [`RlsConnection`](api_core::extractors::RlsConnection),
//! which validates the caller's JWT + org membership and opens a pooled
//! connection with the RLS context (`app.current_org_id` / user GUCs) bound to
//! it. All `emergency_*` tables run under `FORCE ROW LEVEL SECURITY` (migration
//! `00179`), so the connection is the only thing that scopes a query to the
//! caller's tenant — the repository holds no pool of its own.
//!
//! The authoritative organization is therefore `rls.tenant_id()` (the
//! membership-validated org from the session), **not** any client-supplied
//! `organization_id`. The org-keyed repo queries combine that value with the
//! RLS context, so the explicit `organization_id = $N` SQL filter and the
//! policy can never disagree; a foreign-tenant id resolves to `None` → `404`.
//! Membership is enforced by the extractor, so the previous `verify_org_access`
//! helper is no longer needed; manager-gated actions still check `rls.role()`.
//!
//! **IMPORTANT**: every path calls `rls.release().await` before returning so the
//! RLS context is cleared and the connection returns clean to the pool.

mod broadcasts;
mod contacts;
mod drills;
mod incidents;
mod protocols;
pub mod shared;
mod statistics;

use axum::Router;

use crate::state::AppState;

// Re-export all public DTO types so external references via
// `routes::emergency::<Type>` keep working after the split.
pub use drills::CompleteDrillRequest;
pub use shared::{
    BroadcastListQuery, ContactListQuery, CreateBroadcastRequest, CreateContactRequest,
    CreateDrillRequest, CreateIncidentRequest, CreateProtocolRequest, DrillListQuery,
    IncidentListQuery, OrgQuery, ProtocolListQuery, UpdateContactRequest, UpdateDrillRequest,
    UpdateIncidentRequest, UpdateProtocolRequest,
};

/// Create the emergency router.
pub fn router() -> Router<AppState> {
    Router::new()
        .merge(protocols::router())
        .merge(contacts::router())
        .merge(incidents::router())
        .merge(broadcasts::router())
        .merge(drills::router())
        .merge(statistics::router())
}
