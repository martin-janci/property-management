//! Building Certifications routes for Epic 137: Smart Building Certification.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use api_core::extractors::AuthUser;
use db::models::building_certification::{
    BuildingCertification, CertificationBenchmark, CertificationCost, CertificationCredit,
    CertificationDashboard, CertificationDocument, CertificationFilters, CertificationLevel,
    CertificationMilestone, CertificationProgram, CertificationReminder, CertificationStatus,
    CertificationWithCredits, CreateBuildingCertification, CreateCertificationBenchmark,
    CreateCertificationCost, CreateCertificationCredit, CreateCertificationDocument,
    CreateCertificationMilestone, CreateCertificationReminder, UpdateBuildingCertification,
    UpdateCertificationCredit, UpdateCertificationMilestone,
};

use crate::state::AppState;

/// Helper to get organization ID from auth user.
fn get_org_id(auth: &AuthUser) -> Result<Uuid, (StatusCode, String)> {
    auth.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "No organization context".to_string(),
    ))
}

/// Create the router for building certifications.
pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/dashboard", get(get_dashboard))
        // Certifications
        .route("/", get(list_certifications).post(create_certification))
        .route("/expiring", get(get_expiring_certifications))
        .route(
            "/{cert_id}",
            get(get_certification)
                .put(update_certification)
                .delete(delete_certification),
        )
        .route(
            "/{cert_id}/with-credits",
            get(get_certification_with_credits),
        )
        // Credits
        .route("/{cert_id}/credits", get(list_credits).post(create_credit))
        .route(
            "/{cert_id}/credits/{credit_id}",
            get(get_credit).put(update_credit).delete(delete_credit),
        )
        // Documents
        .route(
            "/{cert_id}/documents",
            get(list_documents).post(create_document),
        )
        .route("/{cert_id}/documents/{doc_id}", delete(delete_document))
        // Milestones
        .route(
            "/{cert_id}/milestones",
            get(list_milestones).post(create_milestone),
        )
        .route(
            "/{cert_id}/milestones/{milestone_id}",
            put(update_milestone).delete(delete_milestone),
        )
        // Benchmarks
        .route(
            "/{cert_id}/benchmarks",
            get(list_benchmarks).post(create_benchmark),
        )
        // Costs
        .route("/{cert_id}/costs", get(list_costs).post(create_cost))
        .route("/{cert_id}/costs/total", get(get_total_costs))
        // Reminders
        .route(
            "/{cert_id}/reminders",
            get(list_reminders).post(create_reminder),
        )
        // Audit Logs
        .route("/{cert_id}/audit-logs", get(list_audit_logs))
}

// ==================== Query Parameters ====================

#[derive(Debug, Deserialize)]
pub struct ListCertificationsQuery {
    building_id: Option<Uuid>,
    program: Option<CertificationProgram>,
    level: Option<CertificationLevel>,
    status: Option<CertificationStatus>,
    expiring_within_days: Option<i32>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ExpiringQuery {
    days: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    limit: Option<i64>,
}

// ==================== Dashboard ====================

/// Get certification dashboard summary.
async fn get_dashboard(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<CertificationDashboard>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .get_dashboard(org_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ==================== Certifications ====================

/// List certifications with filters.
async fn list_certifications(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListCertificationsQuery>,
) -> Result<Json<Vec<BuildingCertification>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    let filters = CertificationFilters {
        building_id: query.building_id,
        program: query.program,
        level: query.level,
        status: query.status,
        expiring_within_days: query.expiring_within_days,
    };

    state
        .building_certification_repo
        .list_certifications(
            org_id,
            filters,
            query.limit.unwrap_or(50),
            query.offset.unwrap_or(0),
        )
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a new certification.
async fn create_certification(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateBuildingCertification>,
) -> Result<(StatusCode, Json<BuildingCertification>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .create_certification(org_id, input, Some(auth.user_id))
        .await
        .map(|cert| (StatusCode::CREATED, Json(cert)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get a certification by ID.
async fn get_certification(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<BuildingCertification>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .get_certification(org_id, cert_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Certification not found".to_string()))
}

/// Get certification with credits summary.
async fn get_certification_with_credits(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<CertificationWithCredits>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .get_certification_with_credits(org_id, cert_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Certification not found".to_string()))
}

/// Update a certification.
async fn update_certification(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
    Json(input): Json<UpdateBuildingCertification>,
) -> Result<Json<BuildingCertification>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .update_certification(org_id, cert_id, input)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Certification not found".to_string()))
}

/// Delete a certification.
async fn delete_certification(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    let deleted = state
        .building_certification_repo
        .delete_certification(org_id, cert_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Certification not found".to_string()))
    }
}

/// Get certifications expiring soon.
async fn get_expiring_certifications(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ExpiringQuery>,
) -> Result<Json<Vec<BuildingCertification>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .get_expiring_certifications(org_id, query.days.unwrap_or(90))
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ==================== Credits ====================

/// List credits for a certification.
async fn list_credits(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<Vec<CertificationCredit>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .list_credits(org_id, cert_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a certification credit.
async fn create_credit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
    Json(mut input): Json<CreateCertificationCredit>,
) -> Result<(StatusCode, Json<CertificationCredit>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;
    input.certification_id = cert_id;

    state
        .building_certification_repo
        .create_credit(org_id, input)
        .await
        .map(|credit| (StatusCode::CREATED, Json(credit)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get a credit by ID.
async fn get_credit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_cert_id, credit_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<CertificationCredit>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .get_credit(org_id, credit_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Credit not found".to_string()))
}

/// Update a credit.
async fn update_credit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_cert_id, credit_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateCertificationCredit>,
) -> Result<Json<CertificationCredit>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .update_credit(org_id, credit_id, input)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Credit not found".to_string()))
}

/// Delete a credit.
async fn delete_credit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_cert_id, credit_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    let deleted = state
        .building_certification_repo
        .delete_credit(org_id, credit_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Credit not found".to_string()))
    }
}

// ==================== Documents ====================

/// List documents for a certification.
async fn list_documents(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<Vec<CertificationDocument>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .list_documents(org_id, cert_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a certification document.
async fn create_document(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
    Json(mut input): Json<CreateCertificationDocument>,
) -> Result<(StatusCode, Json<CertificationDocument>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;
    input.certification_id = cert_id;

    state
        .building_certification_repo
        .create_document(org_id, input, Some(auth.user_id))
        .await
        .map(|doc| (StatusCode::CREATED, Json(doc)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Delete a document.
async fn delete_document(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_cert_id, doc_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    let deleted = state
        .building_certification_repo
        .delete_document(org_id, doc_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Document not found".to_string()))
    }
}

// ==================== Milestones ====================

/// List milestones for a certification.
async fn list_milestones(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<Vec<CertificationMilestone>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .list_milestones(org_id, cert_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a certification milestone.
async fn create_milestone(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
    Json(mut input): Json<CreateCertificationMilestone>,
) -> Result<(StatusCode, Json<CertificationMilestone>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;
    input.certification_id = cert_id;

    state
        .building_certification_repo
        .create_milestone(org_id, input)
        .await
        .map(|milestone| (StatusCode::CREATED, Json(milestone)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Update a milestone.
async fn update_milestone(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_cert_id, milestone_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateCertificationMilestone>,
) -> Result<Json<CertificationMilestone>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .update_milestone(org_id, milestone_id, input)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Milestone not found".to_string()))
}

/// Delete a milestone.
async fn delete_milestone(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_cert_id, milestone_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    let deleted = state
        .building_certification_repo
        .delete_milestone(org_id, milestone_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Milestone not found".to_string()))
    }
}

// ==================== Benchmarks ====================

/// List benchmarks for a certification.
async fn list_benchmarks(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<Vec<CertificationBenchmark>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .list_benchmarks(org_id, cert_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a certification benchmark.
async fn create_benchmark(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
    Json(mut input): Json<CreateCertificationBenchmark>,
) -> Result<(StatusCode, Json<CertificationBenchmark>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;
    input.certification_id = cert_id;

    state
        .building_certification_repo
        .create_benchmark(org_id, input)
        .await
        .map(|benchmark| (StatusCode::CREATED, Json(benchmark)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ==================== Costs ====================

/// List costs for a certification.
async fn list_costs(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<Vec<CertificationCost>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .list_costs(org_id, cert_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a certification cost.
async fn create_cost(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
    Json(mut input): Json<CreateCertificationCost>,
) -> Result<(StatusCode, Json<CertificationCost>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;
    input.certification_id = cert_id;

    state
        .building_certification_repo
        .create_cost(org_id, input, Some(auth.user_id))
        .await
        .map(|cost| (StatusCode::CREATED, Json(cost)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get total costs for a certification.
async fn get_total_costs(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<rust_decimal::Decimal>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .get_total_costs(org_id, cert_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ==================== Reminders ====================

/// List reminders for a certification.
async fn list_reminders(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<Vec<CertificationReminder>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .list_reminders(org_id, cert_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a certification reminder.
async fn create_reminder(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
    Json(mut input): Json<CreateCertificationReminder>,
) -> Result<(StatusCode, Json<CertificationReminder>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;
    input.certification_id = cert_id;

    state
        .building_certification_repo
        .create_reminder(org_id, input)
        .await
        .map(|reminder| (StatusCode::CREATED, Json(reminder)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ==================== Audit Logs ====================

/// List audit logs for a certification.
async fn list_audit_logs(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(cert_id): Path<Uuid>,
    Query(query): Query<AuditLogQuery>,
) -> Result<
    Json<Vec<db::models::building_certification::CertificationAuditLog>>,
    (StatusCode, String),
> {
    let org_id = get_org_id(&auth)?;

    state
        .building_certification_repo
        .list_audit_logs(org_id, cert_id, query.limit.unwrap_or(100))
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
