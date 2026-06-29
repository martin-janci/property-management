// Epic 135: Enhanced Tenant Screening with AI Risk Scoring
// REST API routes for AI-powered tenant screening
//
// The handlers are split into cohesive sub-modules (one per resource group) to
// keep each unit small; `router()` below remains the single wiring point and
// preserves the exact public route table.
//
// # RLS (PAP-67 / PAP-74)
//
// Migration `00179` put `FORCE ROW LEVEL SECURITY` on the screening tables, so
// every query MUST run on a connection that has `app.current_org_id` set or it
// collapses to deny-all. Each handler therefore acquires an [`RlsConnection`]
// (which validates tenant membership and sets the org/user GUCs on a dedicated
// connection) and passes `&mut **rls.conn()` to the repository. The authoritative
// organization is `rls.tenant_id()` — the tenant the caller was validated
// against — not a client-supplied `organization_id`, so the SQL org filter and
// the RLS context can never disagree. Cross-tenant access is blocked by RLS: a
// by-id read of another org's row returns no row (`404`), and a write targeting
// another org fails the policy's `WITH CHECK`. `rls.release()` clears the context
// before the connection returns to the pool (called on every Ok/Err path).

use axum::{
    routing::{get, post, put},
    Router,
};

use crate::state::AppState;

mod models;
mod providers;
mod queue;
mod reporting;
mod results;
mod screening_results;

/// Create the enhanced tenant screening router.
pub fn router() -> Router<AppState> {
    Router::new()
        // AI Risk Scoring Models
        .route(
            "/models",
            get(models::list_risk_models).post(models::create_risk_model),
        )
        .route(
            "/models/{id}",
            get(models::get_risk_model).delete(models::delete_risk_model),
        )
        .route("/models/{id}/activate", post(models::activate_risk_model))
        // Provider Configs
        .route(
            "/providers",
            get(providers::list_provider_configs).post(providers::create_provider_config),
        )
        .route(
            "/providers/{id}",
            get(providers::get_provider_config).delete(providers::delete_provider_config),
        )
        .route(
            "/providers/{id}/status",
            put(providers::update_provider_status),
        )
        // AI Results
        .route("/results", get(results::list_ai_results))
        .route("/results/{screening_id}", get(results::get_ai_result))
        .route(
            "/results/{screening_id}/factors",
            get(results::get_risk_factors),
        )
        .route(
            "/results/{screening_id}/complete",
            get(results::get_complete_screening_data),
        )
        // Run AI Scoring
        .route("/score", post(results::run_ai_scoring))
        // Credit Results
        .route(
            "/credit/{screening_id}",
            get(screening_results::get_credit_result),
        )
        .route("/credit", post(screening_results::create_credit_result))
        // Background Results
        .route(
            "/background/{screening_id}",
            get(screening_results::get_background_result),
        )
        .route(
            "/background",
            post(screening_results::create_background_result),
        )
        // Eviction Results
        .route(
            "/eviction/{screening_id}",
            get(screening_results::get_eviction_result),
        )
        .route("/eviction", post(screening_results::create_eviction_result))
        // Queue
        .route(
            "/queue",
            get(queue::get_pending_queue).post(queue::create_queue_item),
        )
        .route("/queue/{id}/status", put(queue::update_queue_status))
        // Reports
        .route("/reports/{screening_id}", get(reporting::get_reports))
        .route("/reports", post(reporting::create_report))
        // Statistics
        .route("/statistics", get(reporting::get_statistics))
        .route("/distribution", get(reporting::get_risk_distribution))
}
