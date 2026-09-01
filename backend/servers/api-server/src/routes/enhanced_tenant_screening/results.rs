// Epic 135: Enhanced Tenant Screening — AI Results and scoring run
//
// See the module-level RLS note in `mod.rs` (PAP-67 / PAP-74).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use api_core::extractors::RlsConnection;
use db::models::enhanced_tenant_screening::*;
use db::repositories::enhanced_tenant_screening::ComponentScores;

use crate::routes::pagination::clamp_limit;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub(super) struct ListResultsQuery {
    limit: Option<i32>,
    offset: Option<i32>,
}

/// List AI results.
pub(super) async fn list_ai_results(
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
            clamp_limit(q.limit.map(i64::from), 50) as i32,
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
pub(super) async fn get_ai_result(
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
pub(super) async fn get_risk_factors(
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
pub(super) async fn get_complete_screening_data(
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
pub(super) async fn run_ai_scoring(
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
