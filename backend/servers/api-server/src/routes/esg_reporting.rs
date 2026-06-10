// Epic 136: ESG Reporting Dashboard
// API routes for ESG (Environmental, Social, Governance) reporting and compliance
//
// # RLS (PAP-139 / PAP-72)
//
// Migration `00179` put `FORCE ROW LEVEL SECURITY` on the ESG tables, so every
// query MUST run on a connection that has `app.current_org_id` set or it
// collapses to deny-all. Each handler therefore acquires an [`RlsConnection`]
// (which validates tenant membership and sets the org/user GUCs on a dedicated
// connection) and passes `&mut **rls.conn()` to the repository. The
// authoritative organization is `rls.tenant_id()` — the tenant the caller was
// validated against — never a client-supplied value, so the SQL org filter and
// the RLS context can never disagree. By-id repo queries stay keyed on
// `(id, organization_id)` as defense-in-depth, so a cross-tenant probe
// resolves to no row → `404` even on an RLS-exempt pool. `rls.release()`
// clears the context before the connection returns to the pool.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use db::models::esg_reporting::*;
use uuid::Uuid;

use crate::state::AppState;
use api_core::extractors::RlsConnection;

/// Create ESG reporting router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Configuration
        .route("/configuration", get(get_configuration))
        .route("/configuration", post(upsert_configuration))
        // Metrics
        .route("/metrics", get(list_metrics))
        .route("/metrics", post(create_metric))
        .route("/metrics/{id}", get(get_metric))
        .route("/metrics/{id}", put(update_metric))
        .route("/metrics/{id}/verify", post(verify_metric))
        .route("/metrics/{id}/delete", post(delete_metric))
        // Carbon Footprint
        .route("/carbon", get(list_carbon_footprints))
        .route("/carbon", post(create_carbon_footprint))
        .route("/carbon/summary/{year}", get(get_carbon_summary))
        .route("/carbon/{id}", get(get_carbon_footprint))
        .route("/carbon/{id}/delete", post(delete_carbon_footprint))
        // Benchmarks
        .route("/benchmarks", get(list_benchmarks))
        .route("/benchmarks", post(create_benchmark))
        .route("/benchmarks/{id}/delete", post(delete_benchmark))
        // Targets
        .route("/targets", get(list_targets))
        .route("/targets", post(create_target))
        .route("/targets/{id}", get(get_target))
        .route("/targets/{id}", put(update_target))
        .route("/targets/{id}/delete", post(delete_target))
        // Reports
        .route("/reports", get(list_reports))
        .route("/reports", post(create_report))
        .route("/reports/{id}", get(get_report))
        .route("/reports/{id}", put(update_report))
        .route("/reports/{id}/submit", post(submit_report))
        .route("/reports/{id}/approve", post(approve_report))
        .route("/reports/{id}/delete", post(delete_report))
        // EU Taxonomy
        .route("/eu-taxonomy", get(list_eu_taxonomy_assessments))
        .route("/eu-taxonomy", post(create_eu_taxonomy_assessment))
        .route("/eu-taxonomy/{id}", get(get_eu_taxonomy_assessment))
        .route("/eu-taxonomy/{id}", put(update_eu_taxonomy_assessment))
        // Dashboard
        .route("/dashboard/{year}", get(get_dashboard))
        .route("/dashboard/{year}/refresh", post(refresh_dashboard))
        // Import Jobs
        .route("/imports", get(list_import_jobs))
        .route("/imports", post(create_import_job))
        .route("/imports/{id}", get(get_import_job))
        // Statistics
        .route("/statistics", get(get_statistics))
}

// =============================================================================
// Configuration Handlers
// =============================================================================

async fn get_configuration(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Option<EsgConfiguration>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .get_configuration(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn upsert_configuration(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(input): Json<CreateEsgConfiguration>,
) -> Result<Json<EsgConfiguration>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .upsert_configuration(&mut **rls.conn(), org_id, input)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

// =============================================================================
// Metrics Handlers
// =============================================================================

async fn list_metrics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<EsgMetricsQuery>,
) -> Result<Json<Vec<EsgMetric>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .list_metrics(&mut **rls.conn(), org_id, query)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn create_metric(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(input): Json<CreateEsgMetric>,
) -> Result<(StatusCode, Json<EsgMetric>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .esg_reporting_repo
        .create_metric(&mut **rls.conn(), org_id, user_id, input)
        .await
        .map(|m| (StatusCode::CREATED, Json(m)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn get_metric(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<EsgMetric>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .get_metric(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|m| {
            m.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Metric not found".to_string()))
        });
    rls.release().await;
    out
}

async fn update_metric(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateEsgMetric>,
) -> Result<Json<EsgMetric>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .update_metric(&mut **rls.conn(), id, org_id, input)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|m| {
            m.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Metric not found".to_string()))
        });
    rls.release().await;
    out
}

#[derive(serde::Deserialize)]
pub struct VerifyMetricRequest {
    pub status: String,
}

async fn verify_metric(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<VerifyMetricRequest>,
) -> Result<Json<EsgMetric>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .esg_reporting_repo
        .verify_metric(&mut **rls.conn(), id, org_id, user_id, &input.status)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|m| {
            m.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Metric not found".to_string()))
        });
    rls.release().await;
    out
}

async fn delete_metric(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .delete_metric(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((StatusCode::NOT_FOUND, "Metric not found".to_string()))
            }
        });
    rls.release().await;
    out
}

// =============================================================================
// Carbon Footprint Handlers
// =============================================================================

async fn list_carbon_footprints(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<CarbonFootprintQuery>,
) -> Result<Json<Vec<CarbonFootprint>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .list_carbon_footprints(&mut **rls.conn(), org_id, query)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn create_carbon_footprint(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(input): Json<CreateCarbonFootprint>,
) -> Result<(StatusCode, Json<CarbonFootprint>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .create_carbon_footprint(&mut **rls.conn(), org_id, input)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn get_carbon_footprint(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<CarbonFootprint>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .get_carbon_footprint(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|c| {
            c.map(Json).ok_or((
                StatusCode::NOT_FOUND,
                "Carbon footprint not found".to_string(),
            ))
        });
    rls.release().await;
    out
}

async fn get_carbon_summary(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(year): Path<i32>,
) -> Result<Json<Option<CarbonFootprintSummary>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .get_carbon_summary(&mut **rls.conn(), org_id, year)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn delete_carbon_footprint(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .delete_carbon_footprint(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((
                    StatusCode::NOT_FOUND,
                    "Carbon footprint not found".to_string(),
                ))
            }
        });
    rls.release().await;
    out
}

// =============================================================================
// Benchmark Handlers
// =============================================================================

#[derive(serde::Deserialize)]
pub struct BenchmarkQueryParams {
    pub category: Option<EsgBenchmarkCategory>,
}

async fn list_benchmarks(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(params): Query<BenchmarkQueryParams>,
) -> Result<Json<Vec<EsgBenchmark>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .list_benchmarks(&mut **rls.conn(), org_id, params.category)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn create_benchmark(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(input): Json<CreateEsgBenchmark>,
) -> Result<(StatusCode, Json<EsgBenchmark>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .create_benchmark(&mut **rls.conn(), org_id, input)
        .await
        .map(|b| (StatusCode::CREATED, Json(b)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn delete_benchmark(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .delete_benchmark(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((StatusCode::NOT_FOUND, "Benchmark not found".to_string()))
            }
        });
    rls.release().await;
    out
}

// =============================================================================
// Target Handlers
// =============================================================================

#[derive(serde::Deserialize)]
pub struct TargetQueryParams {
    pub building_id: Option<Uuid>,
}

async fn list_targets(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(params): Query<TargetQueryParams>,
) -> Result<Json<Vec<EsgTarget>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .list_targets(&mut **rls.conn(), org_id, params.building_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn create_target(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(input): Json<CreateEsgTarget>,
) -> Result<(StatusCode, Json<EsgTarget>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .create_target(&mut **rls.conn(), org_id, input)
        .await
        .map(|t| (StatusCode::CREATED, Json(t)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn get_target(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<EsgTarget>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .get_target(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|t| {
            t.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Target not found".to_string()))
        });
    rls.release().await;
    out
}

async fn update_target(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateEsgTarget>,
) -> Result<Json<EsgTarget>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .update_target(&mut **rls.conn(), id, org_id, input)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|t| {
            t.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Target not found".to_string()))
        });
    rls.release().await;
    out
}

async fn delete_target(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .delete_target(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((StatusCode::NOT_FOUND, "Target not found".to_string()))
            }
        });
    rls.release().await;
    out
}

// =============================================================================
// Report Handlers
// =============================================================================

#[derive(serde::Deserialize)]
pub struct ReportQueryParams {
    pub status: Option<EsgReportStatus>,
}

async fn list_reports(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(params): Query<ReportQueryParams>,
) -> Result<Json<Vec<EsgReport>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .list_reports(&mut **rls.conn(), org_id, params.status)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn create_report(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(input): Json<CreateEsgReport>,
) -> Result<(StatusCode, Json<EsgReport>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .esg_reporting_repo
        .create_report(&mut **rls.conn(), org_id, user_id, input)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn get_report(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<EsgReport>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .get_report(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|r| {
            r.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Report not found".to_string()))
        });
    rls.release().await;
    out
}

async fn update_report(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateEsgReport>,
) -> Result<Json<EsgReport>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .update_report(&mut **rls.conn(), id, org_id, input)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|r| {
            r.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Report not found".to_string()))
        });
    rls.release().await;
    out
}

async fn submit_report(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<EsgReport>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .submit_report(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|r| {
            r.map(Json).ok_or((
                StatusCode::BAD_REQUEST,
                "Report cannot be submitted (not in draft status)".to_string(),
            ))
        });
    rls.release().await;
    out
}

async fn approve_report(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<EsgReport>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .esg_reporting_repo
        .approve_report(&mut **rls.conn(), id, org_id, user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|r| {
            r.map(Json).ok_or((
                StatusCode::BAD_REQUEST,
                "Report cannot be approved (not pending review)".to_string(),
            ))
        });
    rls.release().await;
    out
}

async fn delete_report(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .delete_report(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((
                    StatusCode::BAD_REQUEST,
                    "Report cannot be deleted (only draft reports can be deleted)".to_string(),
                ))
            }
        });
    rls.release().await;
    out
}

// =============================================================================
// EU Taxonomy Handlers
// =============================================================================

#[derive(serde::Deserialize)]
pub struct EuTaxonomyQueryParams {
    pub year: Option<i32>,
}

async fn list_eu_taxonomy_assessments(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(params): Query<EuTaxonomyQueryParams>,
) -> Result<Json<Vec<EuTaxonomyAssessment>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .list_eu_taxonomy_assessments(&mut **rls.conn(), org_id, params.year)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn create_eu_taxonomy_assessment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(input): Json<CreateEuTaxonomyAssessment>,
) -> Result<(StatusCode, Json<EuTaxonomyAssessment>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .create_eu_taxonomy_assessment(&mut **rls.conn(), org_id, input)
        .await
        .map(|a| (StatusCode::CREATED, Json(a)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn get_eu_taxonomy_assessment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<EuTaxonomyAssessment>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .get_eu_taxonomy_assessment(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|a| {
            a.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Assessment not found".to_string()))
        });
    rls.release().await;
    out
}

async fn update_eu_taxonomy_assessment(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateEuTaxonomyAssessment>,
) -> Result<Json<EuTaxonomyAssessment>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .update_eu_taxonomy_assessment(&mut **rls.conn(), id, org_id, input)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|a| {
            a.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Assessment not found".to_string()))
        });
    rls.release().await;
    out
}

// =============================================================================
// Dashboard Handlers
// =============================================================================

#[derive(serde::Deserialize)]
pub struct DashboardQueryParams {
    pub building_id: Option<Uuid>,
}

async fn get_dashboard(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(year): Path<i32>,
    Query(params): Query<DashboardQueryParams>,
) -> Result<Json<Option<EsgDashboardMetrics>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .get_dashboard_metrics(&mut **rls.conn(), org_id, year, params.building_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn refresh_dashboard(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(year): Path<i32>,
    Query(params): Query<DashboardQueryParams>,
) -> Result<Json<EsgDashboardMetrics>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .refresh_dashboard_metrics(rls.conn(), org_id, year, params.building_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

// =============================================================================
// Import Job Handlers
// =============================================================================

async fn list_import_jobs(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<EsgImportJob>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .list_import_jobs(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn create_import_job(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(input): Json<CreateEsgImportJob>,
) -> Result<(StatusCode, Json<EsgImportJob>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .esg_reporting_repo
        .create_import_job(&mut **rls.conn(), org_id, user_id, input)
        .await
        .map(|j| (StatusCode::CREATED, Json(j)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}

async fn get_import_job(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<EsgImportJob>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .get_import_job(&mut **rls.conn(), id, org_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        .and_then(|j| {
            j.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Import job not found".to_string()))
        });
    rls.release().await;
    out
}

// =============================================================================
// Statistics Handler
// =============================================================================

async fn get_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<EsgStatistics>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .esg_reporting_repo
        .get_statistics(&mut **rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    rls.release().await;
    out
}
