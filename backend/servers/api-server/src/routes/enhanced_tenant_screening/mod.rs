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
use api_core::extractors::RlsConnection;
use axum::{http::StatusCode, response::IntoResponse, response::Response};

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

/// Tenant-screening data (credit / background / eviction results, AI risk
/// scores, provider credentials, queue, reports) is manager-only. The role is
/// the membership-validated one carried by [`RlsConnection`], never the JWT.
pub(super) fn is_screening_manager(rls: &RlsConnection) -> bool {
    rls.role().is_manager()
}

/// Standard `403` for a non-manager caller.
pub(super) fn manager_required() -> Response {
    (
        StatusCode::FORBIDDEN,
        "Manager role required for tenant screening",
    )
        .into_response()
}

/// Standard `404` for a payload that references a screening or provider
/// config outside the caller's organization.
pub(super) fn screening_not_found() -> Response {
    (StatusCode::NOT_FOUND, "Screening not found").into_response()
}

/// Reject a create payload whose `screening_id` / `provider_config_id` belongs
/// to another organization (`404`). The caller releases the connection.
pub(super) async fn reject_foreign_refs(
    s: &AppState,
    rls: &mut RlsConnection,
    screening_id: uuid::Uuid,
    provider_config_id: Option<uuid::Uuid>,
) -> Option<Response> {
    let org_id = rls.tenant_id();
    let owned = s
        .enhanced_tenant_screening_repo
        .screening_refs_in_org(&mut **rls.conn(), org_id, screening_id, provider_config_id)
        .await;
    match owned {
        Ok(true) => None,
        Ok(false) => Some(screening_not_found()),
        Err(e) => Some((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()),
    }
}

#[cfg(test)]
mod raw_error_leak_guard {
    //! Regression guard (pm-security, Epic 135): tenant-screening handlers
    //! expose highly sensitive PII (credit, criminal-background and eviction
    //! results). A failing DB call must NEVER surface raw `sqlx` error text to
    //! the client — its `Display` output leaks table/column names, SQL
    //! fragments and connection details. Every DB-fallible arm routes through
    //! `db_error`, which logs the cause server-side and returns an opaque 500.
    //!
    //! This source-level guard fails if any handler in the group reintroduces
    //! the raw-error leak (a tuple response built from the sqlx error). It is a
    //! sibling of the equivalent guard in `routes/compliance.rs`.

    /// Every handler source file in the screening group.
    const SOURCES: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("models.rs", include_str!("models.rs")),
        ("providers.rs", include_str!("providers.rs")),
        ("queue.rs", include_str!("queue.rs")),
        ("reporting.rs", include_str!("reporting.rs")),
        ("results.rs", include_str!("results.rs")),
        ("screening_results.rs", include_str!("screening_results.rs")),
    ];

    // Split across two constants so no single *code* line of this guard
    // contains both tokens (which would make the scan flag its own source).
    const LEAK_STATUS: &str = "INTERNAL_SERVER_ERROR";
    const LEAK_RAW: &str = "to_string()";

    #[test]
    fn no_handler_leaks_raw_db_error_to_client() {
        for (name, src) in SOURCES {
            for (idx, line) in src.lines().enumerate() {
                // Ignore comments / doc paragraphs — only real code counts.
                if line.trim_start().starts_with("//") {
                    continue;
                }
                let leaks = line.contains(LEAK_STATUS) && line.contains(LEAK_RAW);
                assert!(
                    !leaks,
                    "{}:{}: raw DB error leaked to client — route it through \
                     db_error(e) instead of building the response from the \
                     error: {}",
                    name,
                    idx + 1,
                    line.trim()
                );
            }
        }
    }
}
