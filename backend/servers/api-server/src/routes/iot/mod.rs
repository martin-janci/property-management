//! IoT routes (Epic 14: IoT & Smart Building).
//!
//! Handles sensor registration, data ingestion, dashboards, alerts, and
//! correlations. Split by surface so each concern lives in its own module:
//!
//! | Surface       | Module         | Handlers |
//! |---------------|----------------|----------|
//! | shared        | `shared`       | Error-mapping helpers, `MessageResponse`, `PaginationQuery` |
//! | sensors       | `sensors`      | create, list, get, update, delete |
//! | readings      | `readings`     | list, add, batch, aggregated (publish realtime events) |
//! | thresholds    | `thresholds`   | CRUD + template list / apply |
//! | alerts        | `alerts`       | list, acknowledge, resolve |
//! | correlations  | `correlations` | list, create, delete |
//! | dashboard     | `dashboard`    | dashboard summary |
//! | realtime      | `realtime`     | sensor WS upgrade + pub/sub publish |
//!
//! Public DTO types and the WS handler are re-exported here so existing
//! `routes::iot::*` references continue to resolve unchanged.
//!
//! # RLS (PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on every sensor table
//! (`sensors`, `sensor_readings`, `sensor_alerts`, `sensor_thresholds`,
//! `sensor_threshold_templates`, `sensor_fault_correlations`), so every query
//! MUST run on a connection that has `app.current_org_id` set or it collapses to
//! deny-all. Each handler therefore acquires an
//! [`RlsConnection`](api_core::extractors::RlsConnection) (which validates tenant
//! membership and sets the org/user GUCs on a dedicated connection) and passes
//! `&mut **rls.conn()` to the repository. The authoritative organization is
//! `rls.tenant_id()` — the tenant the caller was validated against — not a
//! client-supplied `organization_id`, so the SQL org filter and the RLS context
//! can never disagree. Cross-tenant access is blocked by RLS: a by-id read of
//! another org's row returns no row (`404`), and a write targeting another org
//! fails the policy's `WITH CHECK`. `rls.release()` clears the context before the
//! connection returns to the pool.

mod alerts;
mod correlations;
mod dashboard;
mod readings;
mod realtime;
mod sensors;
pub mod shared;
mod thresholds;

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::state::AppState;

// Re-export public DTO types and the WS handler so external references via
// `routes::iot::<Type>` keep working after the split.
pub use alerts::ResolveAlertRequest;
pub use dashboard::DashboardQuery;
pub use realtime::{sensor_ws_handler, SensorWsEvent, SensorWsQuery};
pub use shared::{MessageResponse, PaginationQuery};
pub use thresholds::{ApplyTemplateRequest, TemplateQuery};

// ============================================================================
// Sensor Router (Story 14.1)
// ============================================================================

pub fn sensor_router() -> Router<AppState> {
    Router::new()
        .route("/", post(sensors::create_sensor))
        .route("/", get(sensors::list_sensors))
        .route("/{id}", get(sensors::get_sensor))
        .route("/{id}", put(sensors::update_sensor))
        .route("/{id}", delete(sensors::delete_sensor))
        .route("/{id}/readings", get(readings::list_readings))
        .route("/{id}/readings", post(readings::add_reading))
        .route("/{id}/readings/batch", post(readings::add_batch_readings))
        .route(
            "/{id}/readings/aggregated",
            get(readings::get_aggregated_readings),
        )
        .route("/{id}/thresholds", get(thresholds::list_thresholds))
        .route("/{id}/thresholds", post(thresholds::create_threshold))
        .route(
            "/thresholds/{threshold_id}",
            put(thresholds::update_threshold),
        )
        .route(
            "/thresholds/{threshold_id}",
            delete(thresholds::delete_threshold),
        )
        .route("/{id}/alerts", get(alerts::list_sensor_alerts))
        .route(
            "/alerts/{alert_id}/acknowledge",
            post(alerts::acknowledge_alert),
        )
        .route("/alerts/{alert_id}/resolve", post(alerts::resolve_alert))
        .route("/{id}/correlations", get(correlations::list_correlations))
        .route("/{id}/correlations", post(correlations::create_correlation))
        .route(
            "/correlations/{correlation_id}",
            delete(correlations::delete_correlation),
        )
        .route("/templates", get(thresholds::list_threshold_templates))
        .route(
            "/templates/{template_id}/apply",
            post(thresholds::apply_template),
        )
        .route("/dashboard", get(dashboard::get_dashboard))
        // Realtime sensor-reading push channel (Story 14.3, gap #985-2).
        .route("/ws", get(realtime::sensor_ws_handler))
}
