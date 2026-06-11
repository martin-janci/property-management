// Epic 135: Enhanced Tenant Screening with AI Risk Scoring
// REST API routes for AI-powered tenant screening
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
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use api_core::extractors::RlsConnection;
use db::models::enhanced_tenant_screening::*;
use db::repositories::enhanced_tenant_screening::ComponentScores;

use crate::state::AppState;

/// Create the enhanced tenant screening router.
pub fn router() -> Router<AppState> {
    Router::new()
        // AI Risk Scoring Models
        .route("/models", get(list_risk_models).post(create_risk_model))
        .route(
            "/models/{id}",
            get(get_risk_model).delete(delete_risk_model),
        )
        .route("/models/{id}/activate", post(activate_risk_model))
        // Provider Configs
        .route(
            "/providers",
            get(list_provider_configs).post(create_provider_config),
        )
        .route(
            "/providers/{id}",
            get(get_provider_config).delete(delete_provider_config),
        )
        .route("/providers/{id}/status", put(update_provider_status))
        // AI Results
        .route("/results", get(list_ai_results))
        .route("/results/{screening_id}", get(get_ai_result))
        .route("/results/{screening_id}/factors", get(get_risk_factors))
        .route(
            "/results/{screening_id}/complete",
            get(get_complete_screening_data),
        )
        // Run AI Scoring
        .route("/score", post(run_ai_scoring))
        // Credit Results
        .route("/credit/{screening_id}", get(get_credit_result))
        .route("/credit", post(create_credit_result))
        // Background Results
        .route("/background/{screening_id}", get(get_background_result))
        .route("/background", post(create_background_result))
        // Eviction Results
        .route("/eviction/{screening_id}", get(get_eviction_result))
        .route("/eviction", post(create_eviction_result))
        // Queue
        .route("/queue", get(get_pending_queue).post(create_queue_item))
        .route("/queue/{id}/status", put(update_queue_status))
        // Reports
        .route("/reports/{screening_id}", get(get_reports))
        .route("/reports", post(create_report))
        // Statistics
        .route("/statistics", get(get_statistics))
        .route("/distribution", get(get_risk_distribution))
}

// =============================================================================
// AI Risk Scoring Models
// =============================================================================

/// List all risk models for organization.
async fn list_risk_models(State(s): State<AppState>, mut rls: RlsConnection) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .list_risk_models(&mut **rls.conn(), org_id)
        .await;
    rls.release().await;

    match result {
        Ok(models) => Json(models).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create a new AI risk scoring model.
async fn create_risk_model(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateAiRiskScoringModel>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_risk_model(&mut **rls.conn(), org_id, user_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(model) => (StatusCode::CREATED, Json(model)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get a risk model by ID.
async fn get_risk_model(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_risk_model(&mut **rls.conn(), org_id, id)
        .await;
    rls.release().await;

    match result {
        Ok(Some(model)) => Json(model).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Risk model not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Delete a risk model.
async fn delete_risk_model(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let result = s
        .enhanced_tenant_screening_repo
        .delete_risk_model(&mut **rls.conn(), id)
        .await;
    rls.release().await;

    match result {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Risk model not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Activate a risk model.
async fn activate_risk_model(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .activate_risk_model(rls.conn(), org_id, id)
        .await;
    rls.release().await;

    match result {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// =============================================================================
// Provider Configs
// =============================================================================

/// List provider configs.
async fn list_provider_configs(
    State(s): State<AppState>,
    mut rls: RlsConnection,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .list_provider_configs(&mut **rls.conn(), org_id)
        .await;
    rls.release().await;

    match result {
        Ok(configs) => Json(configs).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create provider config.
async fn create_provider_config(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScreeningProviderConfig>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_provider_config(&mut **rls.conn(), org_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(config) => (StatusCode::CREATED, Json(config)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get provider config.
async fn get_provider_config(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_provider_config(&mut **rls.conn(), org_id, id)
        .await;
    rls.release().await;

    match result {
        Ok(Some(config)) => Json(config).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Provider config not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Delete provider config.
async fn delete_provider_config(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let result = s
        .enhanced_tenant_screening_repo
        .delete_provider_config(&mut **rls.conn(), id)
        .await;
    rls.release().await;

    match result {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Provider config not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct UpdateProviderStatusRequest {
    status: ProviderIntegrationStatus,
    error_message: Option<String>,
}

/// Update provider status.
async fn update_provider_status(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProviderStatusRequest>,
) -> impl IntoResponse {
    let result = s
        .enhanced_tenant_screening_repo
        .update_provider_status(
            &mut **rls.conn(),
            id,
            req.status,
            req.error_message.as_deref(),
        )
        .await;
    rls.release().await;

    match result {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// =============================================================================
// AI Results
// =============================================================================

#[derive(Debug, Deserialize)]
struct ListResultsQuery {
    limit: Option<i32>,
    offset: Option<i32>,
}

/// List AI results.
async fn list_ai_results(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Query(q): Query<ListResultsQuery>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .list_ai_results(
            &mut **rls.conn(),
            org_id,
            q.limit.unwrap_or(50),
            q.offset.unwrap_or(0),
        )
        .await;
    rls.release().await;

    match result {
        Ok(results) => Json(results).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get AI result for screening.
async fn get_ai_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(screening_id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_ai_result_by_screening(&mut **rls.conn(), org_id, screening_id)
        .await;
    rls.release().await;

    match result {
        Ok(Some(result)) => Json(result).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "AI result not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get risk factors for screening.
async fn get_risk_factors(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(screening_id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();

    // First get the (org-scoped) AI result to get its ID, then its factors.
    let ai_result = s
        .enhanced_tenant_screening_repo
        .get_ai_result_by_screening(&mut **rls.conn(), org_id, screening_id)
        .await;

    let response = match ai_result {
        Ok(Some(ai_result)) => {
            match s
                .enhanced_tenant_screening_repo
                .get_risk_factors(&mut **rls.conn(), ai_result.id)
                .await
            {
                Ok(factors) => Json(factors).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, "AI result not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    rls.release().await;
    response
}

/// Get complete screening data.
async fn get_complete_screening_data(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(screening_id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_complete_screening_data(rls.conn(), org_id, screening_id)
        .await;
    rls.release().await;

    match result {
        Ok(data) => Json(data).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Run AI scoring on a screening.
async fn run_ai_scoring(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<RunAiScoringRequest>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();

    // Get the active model or specified model.
    let model_res = if let Some(model_id) = req.model_id {
        s.enhanced_tenant_screening_repo
            .get_risk_model(&mut **rls.conn(), org_id, model_id)
            .await
    } else {
        s.enhanced_tenant_screening_repo
            .get_active_risk_model(&mut **rls.conn(), org_id)
            .await
    };

    let response = match model_res {
        Ok(Some(model)) => {
            // Get credit, background, and eviction results to compute component scores.
            let credit = s
                .enhanced_tenant_screening_repo
                .get_credit_result_by_screening(&mut **rls.conn(), org_id, req.screening_id)
                .await
                .ok()
                .flatten();
            let background = s
                .enhanced_tenant_screening_repo
                .get_background_result_by_screening(&mut **rls.conn(), org_id, req.screening_id)
                .await
                .ok()
                .flatten();
            let eviction = s
                .enhanced_tenant_screening_repo
                .get_eviction_result_by_screening(&mut **rls.conn(), org_id, req.screening_id)
                .await
                .ok()
                .flatten();

            // Calculate component scores.
            let component_scores = ComponentScores {
                credit_history: credit.as_ref().and_then(|c| {
                    c.credit_score.map(|score| {
                        // Convert FICO score (300-850) to 0-100 scale
                        ((score - 300) * 100 / 550).clamp(0, 100)
                    })
                }),
                rental_history: None,   // Would need rental history integration
                income_stability: None, // Would need income verification integration
                employment_stability: None, // Would need employment verification integration
                eviction_history: eviction.as_ref().map(|e| {
                    if e.eviction_count.unwrap_or(0) == 0 {
                        100
                    } else {
                        (100 - e.eviction_count.unwrap_or(0) * 25).clamp(0, 100)
                    }
                }),
                criminal_background: background.as_ref().map(|b| {
                    let felony_penalty = b.felony_count.unwrap_or(0) * 30;
                    let misdemeanor_penalty = b.misdemeanor_count.unwrap_or(0) * 10;
                    (100 - felony_penalty - misdemeanor_penalty).clamp(0, 100)
                }),
                identity_verification: background.as_ref().map(|b| {
                    if b.identity_verified.unwrap_or(false) && b.ssn_verified.unwrap_or(false) {
                        100
                    } else if b.identity_verified.unwrap_or(false) {
                        75
                    } else {
                        25
                    }
                }),
                reference_quality: None, // Would need reference check integration
            };

            // Create AI result.
            match s
                .enhanced_tenant_screening_repo
                .create_ai_result(
                    &mut **rls.conn(),
                    org_id,
                    req.screening_id,
                    &model,
                    component_scores,
                )
                .await
            {
                Ok(result) => (StatusCode::CREATED, Json(result)).into_response(),
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            }
        }
        Ok(None) => {
            if req.model_id.is_some() {
                (StatusCode::NOT_FOUND, "Specified model not found").into_response()
            } else {
                (
                    StatusCode::BAD_REQUEST,
                    "No active risk model configured for organization",
                )
                    .into_response()
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    rls.release().await;
    response
}

// =============================================================================
// Credit Results
// =============================================================================

/// Get credit result.
async fn get_credit_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(screening_id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_credit_result_by_screening(&mut **rls.conn(), org_id, screening_id)
        .await;
    rls.release().await;

    match result {
        Ok(Some(result)) => Json(result).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Credit result not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create credit result.
async fn create_credit_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScreeningCreditResult>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_credit_result(&mut **rls.conn(), org_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(result) => (StatusCode::CREATED, Json(result)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// =============================================================================
// Background Results
// =============================================================================

/// Get background result.
async fn get_background_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(screening_id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_background_result_by_screening(&mut **rls.conn(), org_id, screening_id)
        .await;
    rls.release().await;

    match result {
        Ok(Some(result)) => Json(result).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Background result not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create background result.
async fn create_background_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScreeningBackgroundResult>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_background_result(&mut **rls.conn(), org_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(result) => (StatusCode::CREATED, Json(result)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// =============================================================================
// Eviction Results
// =============================================================================

/// Get eviction result.
async fn get_eviction_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(screening_id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_eviction_result_by_screening(&mut **rls.conn(), org_id, screening_id)
        .await;
    rls.release().await;

    match result {
        Ok(Some(result)) => Json(result).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Eviction result not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create eviction result.
async fn create_eviction_result(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScreeningEvictionResult>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_eviction_result(&mut **rls.conn(), org_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(result) => (StatusCode::CREATED, Json(result)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// =============================================================================
// Queue
// =============================================================================

/// Get pending queue items.
async fn get_pending_queue(State(s): State<AppState>, mut rls: RlsConnection) -> impl IntoResponse {
    let result = s
        .enhanced_tenant_screening_repo
        .get_pending_queue_items(&mut **rls.conn(), 50)
        .await;
    rls.release().await;

    match result {
        Ok(items) => Json(items).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create queue item.
async fn create_queue_item(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScreeningQueueItem>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_queue_item(&mut **rls.conn(), org_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(item) => (StatusCode::CREATED, Json(item)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Debug, Deserialize)]
struct UpdateQueueStatusRequest {
    status: String,
    error: Option<String>,
}

/// Update queue item status.
async fn update_queue_status(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateQueueStatusRequest>,
) -> impl IntoResponse {
    let result = s
        .enhanced_tenant_screening_repo
        .update_queue_item_status(&mut **rls.conn(), id, &req.status, req.error.as_deref())
        .await;
    rls.release().await;

    match result {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// =============================================================================
// Reports
// =============================================================================

/// Get reports for screening.
async fn get_reports(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Path(screening_id): Path<Uuid>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_reports_by_screening(&mut **rls.conn(), org_id, screening_id)
        .await;
    rls.release().await;

    match result {
        Ok(reports) => Json(reports).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Create report.
async fn create_report(
    State(s): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScreeningReport>,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let result = s
        .enhanced_tenant_screening_repo
        .create_report(&mut **rls.conn(), org_id, user_id, req)
        .await;
    rls.release().await;

    match result {
        Ok(report) => (StatusCode::CREATED, Json(report)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// =============================================================================
// Statistics
// =============================================================================

/// Get screening statistics.
async fn get_statistics(State(s): State<AppState>, mut rls: RlsConnection) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_statistics(&mut **rls.conn(), org_id)
        .await;
    rls.release().await;

    match result {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Get risk distribution.
async fn get_risk_distribution(
    State(s): State<AppState>,
    mut rls: RlsConnection,
) -> impl IntoResponse {
    let org_id = rls.tenant_id();
    let result = s
        .enhanced_tenant_screening_repo
        .get_risk_distribution(&mut **rls.conn(), org_id)
        .await;
    rls.release().await;

    match result {
        Ok(dist) => Json(dist).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
