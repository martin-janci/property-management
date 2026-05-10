// Epic 136: ESG Reporting Dashboard
// API routes for ESG (Environmental, Social, Governance) reporting and compliance

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use db::models::esg_reporting::*;
use uuid::Uuid;

use crate::state::AppState;
use api_core::extractors::AuthUser;

/// Helper to extract organization ID from auth user.
fn get_org_id(auth: &AuthUser) -> Result<Uuid, (StatusCode, String)> {
    auth.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "No organization context".to_string(),
    ))
}

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
    auth: AuthUser,
) -> Result<Json<Option<EsgConfiguration>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .get_configuration(org_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn upsert_configuration(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateEsgConfiguration>,
) -> Result<Json<EsgConfiguration>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .upsert_configuration(org_id, input)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// =============================================================================
// Metrics Handlers
// =============================================================================

async fn list_metrics(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<EsgMetricsQuery>,
) -> Result<Json<Vec<EsgMetric>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .list_metrics(org_id, query)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn create_metric(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateEsgMetric>,
) -> Result<(StatusCode, Json<EsgMetric>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .create_metric(org_id, auth.user_id, input)
        .await
        .map(|m| (StatusCode::CREATED, Json(m)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn get_metric(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EsgMetric>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .get_metric(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Metric not found".to_string()))
}

async fn update_metric(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateEsgMetric>,
) -> Result<Json<EsgMetric>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .update_metric(id, input)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Metric not found".to_string()))
}

#[derive(serde::Deserialize)]
pub struct VerifyMetricRequest {
    pub status: String,
}

async fn verify_metric(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<VerifyMetricRequest>,
) -> Result<Json<EsgMetric>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .verify_metric(id, auth.user_id, &input.status)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Metric not found".to_string()))
}

async fn delete_metric(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .delete_metric(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// Carbon Footprint Handlers
// =============================================================================

async fn list_carbon_footprints(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<CarbonFootprintQuery>,
) -> Result<Json<Vec<CarbonFootprint>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .list_carbon_footprints(org_id, query)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn create_carbon_footprint(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateCarbonFootprint>,
) -> Result<(StatusCode, Json<CarbonFootprint>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .create_carbon_footprint(org_id, input)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn get_carbon_footprint(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<CarbonFootprint>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .get_carbon_footprint(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            "Carbon footprint not found".to_string(),
        ))
}

async fn get_carbon_summary(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(year): Path<i32>,
) -> Result<Json<Option<CarbonFootprintSummary>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .get_carbon_summary(org_id, year)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn delete_carbon_footprint(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .delete_carbon_footprint(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
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
    auth: AuthUser,
    Query(params): Query<BenchmarkQueryParams>,
) -> Result<Json<Vec<EsgBenchmark>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .list_benchmarks(org_id, params.category)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn create_benchmark(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateEsgBenchmark>,
) -> Result<(StatusCode, Json<EsgBenchmark>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .create_benchmark(org_id, input)
        .await
        .map(|b| (StatusCode::CREATED, Json(b)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn delete_benchmark(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .delete_benchmark(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
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
    auth: AuthUser,
    Query(params): Query<TargetQueryParams>,
) -> Result<Json<Vec<EsgTarget>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .list_targets(org_id, params.building_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn create_target(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateEsgTarget>,
) -> Result<(StatusCode, Json<EsgTarget>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .create_target(org_id, input)
        .await
        .map(|t| (StatusCode::CREATED, Json(t)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn get_target(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EsgTarget>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .get_target(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Target not found".to_string()))
}

async fn update_target(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateEsgTarget>,
) -> Result<Json<EsgTarget>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .update_target(id, input)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Target not found".to_string()))
}

async fn delete_target(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .delete_target(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
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
    auth: AuthUser,
    Query(params): Query<ReportQueryParams>,
) -> Result<Json<Vec<EsgReport>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .list_reports(org_id, params.status)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn create_report(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateEsgReport>,
) -> Result<(StatusCode, Json<EsgReport>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .create_report(org_id, auth.user_id, input)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn get_report(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EsgReport>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .get_report(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Report not found".to_string()))
}

async fn update_report(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateEsgReport>,
) -> Result<Json<EsgReport>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .update_report(id, input)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Report not found".to_string()))
}

async fn submit_report(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EsgReport>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .submit_report(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Report cannot be submitted (not in draft status)".to_string(),
        ))
}

async fn approve_report(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EsgReport>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .approve_report(id, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Report cannot be approved (not pending review)".to_string(),
        ))
}

async fn delete_report(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = state
        .esg_reporting_repo
        .delete_report(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            "Report cannot be deleted (only draft reports can be deleted)".to_string(),
        ))
    }
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
    auth: AuthUser,
    Query(params): Query<EuTaxonomyQueryParams>,
) -> Result<Json<Vec<EuTaxonomyAssessment>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .list_eu_taxonomy_assessments(org_id, params.year)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn create_eu_taxonomy_assessment(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateEuTaxonomyAssessment>,
) -> Result<(StatusCode, Json<EuTaxonomyAssessment>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .create_eu_taxonomy_assessment(org_id, input)
        .await
        .map(|a| (StatusCode::CREATED, Json(a)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn get_eu_taxonomy_assessment(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EuTaxonomyAssessment>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .get_eu_taxonomy_assessment(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Assessment not found".to_string()))
}

async fn update_eu_taxonomy_assessment(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateEuTaxonomyAssessment>,
) -> Result<Json<EuTaxonomyAssessment>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .update_eu_taxonomy_assessment(id, input)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Assessment not found".to_string()))
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
    auth: AuthUser,
    Path(year): Path<i32>,
    Query(params): Query<DashboardQueryParams>,
) -> Result<Json<Option<EsgDashboardMetrics>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .get_dashboard_metrics(org_id, year, params.building_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn refresh_dashboard(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(year): Path<i32>,
    Query(params): Query<DashboardQueryParams>,
) -> Result<Json<EsgDashboardMetrics>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .refresh_dashboard_metrics(org_id, year, params.building_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// =============================================================================
// Import Job Handlers
// =============================================================================

async fn list_import_jobs(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<EsgImportJob>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .list_import_jobs(org_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn create_import_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateEsgImportJob>,
) -> Result<(StatusCode, Json<EsgImportJob>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .create_import_job(org_id, auth.user_id, input)
        .await
        .map(|j| (StatusCode::CREATED, Json(j)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn get_import_job(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<EsgImportJob>, (StatusCode, String)> {
    state
        .esg_reporting_repo
        .get_import_job(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Import job not found".to_string()))
}

// =============================================================================
// Statistics Handler
// =============================================================================

async fn get_statistics(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<EsgStatistics>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .esg_reporting_repo
        .get_statistics(org_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
