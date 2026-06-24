//! IoT sensor repository (Epic 14).
//!
//! # RLS Integration (PAP-67)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` + the canonical
//! `get_current_org_id()` policy on every sensor table (`sensors`,
//! `sensor_readings`, `sensor_alerts`, `sensor_thresholds`,
//! `sensor_threshold_templates`, `sensor_fault_correlations`). Under `FORCE`
//! the api-server's owner connection is no longer exempt, so a query issued on
//! a connection without `app.current_org_id` set collapses to deny-all (own-org
//! reads return empty, writes fail).
//!
//! Every method therefore takes an **executor whose connection already has RLS
//! context set** (org + user GUCs) — in handlers this comes from the
//! `RlsConnection` extractor via `&mut **rls.conn()`. The repository holds **no
//! pool**, so there is no way to issue a query that bypasses RLS. This mirrors
//! the `work_order.rs` / `budget.rs` precedent.
//!
//! # Module layout
//!
//! The repository surface is large (sensor CRUD, readings, thresholds, alerts,
//! fault correlations and a dashboard aggregate). To keep each area readable and
//! to reduce the churn surface of any single file, the `impl SensorRepository`
//! block is split across cohesive sub-modules. Every sub-module adds methods to
//! the *same* [`SensorRepository`] struct defined here (same pattern as the
//! `document/` repository split):
//!
//! - [`sensors`]      — sensor CRUD, status, counts & building lookup
//! - [`readings`]     — reading writes (single + batch), listing & aggregation
//! - [`thresholds`]   — threshold CRUD, templates & template application
//! - [`alerts`]       — alert CRUD, resolve/acknowledge & counts
//! - [`correlations`] — sensor-fault correlation CRUD & lookups
//! - [`dashboard`]    — organization dashboard aggregate

mod alerts;
mod correlations;
mod dashboard;
mod readings;
mod sensors;
mod thresholds;

/// Repository for IoT sensor operations.
///
/// Stateless: every method receives an RLS-context-bearing executor. The repo
/// holds no pool so it cannot issue an un-scoped (deny-all under `FORCE`) query.
#[derive(Clone, Default)]
pub struct SensorRepository;

impl SensorRepository {
    pub fn new() -> Self {
        Self
    }
}
