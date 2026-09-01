//! Building Certifications routes for Epic 137: Smart Building Certification.
//!
//! # RLS (PAP-67 / PAP-80)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on the building-certification
//! cluster (8 tables), so every query MUST run on a connection that has
//! `app.current_org_id` set or it collapses to deny-all. Each handler therefore
//! acquires an [`RlsConnection`] (which validates tenant membership and sets the
//! org/user GUCs on a dedicated connection) and passes that connection to the
//! repository (`&mut **rls.conn()` to the generic-`Executor` methods, `rls.conn()`
//! to the `&mut PgConnection` ones, which deref-coerce). The authoritative
//! organization is `rls.tenant_id()` — the tenant the caller was validated
//! against — not a client-supplied `organization_id`, so the SQL org filter and
//! the RLS context can never disagree. Cross-tenant access is blocked by RLS: a
//! by-id read of another org's row returns no row (`404`), and a write targeting
//! another org fails the policy's `WITH CHECK`. `rls.release()` clears the
//! context before the connection returns to the pool (on both the Ok and Err
//! paths).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use api_core::extractors::RlsConnection;
use db::models::building_certification::{
    BuildingCertification, CertificationBenchmark, CertificationCost, CertificationCredit,
    CertificationDashboard, CertificationDocument, CertificationFilters, CertificationLevel,
    CertificationMilestone, CertificationProgram, CertificationReminder, CertificationStatus,
    CertificationWithCredits, CreateBuildingCertification, CreateCertificationBenchmark,
    CreateCertificationCost, CreateCertificationCredit, CreateCertificationDocument,
    CreateCertificationMilestone, CreateCertificationReminder, UpdateBuildingCertification,
    UpdateCertificationCredit, UpdateCertificationMilestone,
};

use crate::routes::pagination::clamp_limit;
use crate::state::AppState;

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

fn internal_error(e: sqlx::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

// ==================== Dashboard ====================

/// Get certification dashboard summary.
async fn get_dashboard(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<CertificationDashboard>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .get_dashboard(rls.conn(), org_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

// ==================== Certifications ====================

/// List certifications with filters.
async fn list_certifications(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListCertificationsQuery>,
) -> Result<Json<Vec<BuildingCertification>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();

    let filters = CertificationFilters {
        building_id: query.building_id,
        program: query.program,
        level: query.level,
        status: query.status,
        expiring_within_days: query.expiring_within_days,
    };

    let out = state
        .building_certification_repo
        .list_certifications(
            &mut **rls.conn(),
            org_id,
            filters,
            clamp_limit(query.limit, 50),
            query.offset.unwrap_or(0),
        )
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Create a new certification.
async fn create_certification(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(input): Json<CreateBuildingCertification>,
) -> Result<(StatusCode, Json<BuildingCertification>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let out = state
        .building_certification_repo
        .create_certification(&mut **rls.conn(), org_id, input, Some(user_id))
        .await
        .map(|cert| (StatusCode::CREATED, Json(cert)))
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Get a certification by ID.
async fn get_certification(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<BuildingCertification>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .get_certification(&mut **rls.conn(), org_id, cert_id)
        .await
        .map_err(internal_error)
        .and_then(|c| {
            c.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Certification not found".to_string()))
        });
    rls.release().await;
    out
}

/// Get certification with credits summary.
async fn get_certification_with_credits(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<CertificationWithCredits>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .get_certification_with_credits(rls.conn(), org_id, cert_id)
        .await
        .map_err(internal_error)
        .and_then(|c| {
            c.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Certification not found".to_string()))
        });
    rls.release().await;
    out
}

/// Update a certification.
async fn update_certification(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
    Json(input): Json<UpdateBuildingCertification>,
) -> Result<Json<BuildingCertification>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .update_certification(&mut **rls.conn(), org_id, cert_id, input)
        .await
        .map_err(internal_error)
        .and_then(|c| {
            c.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Certification not found".to_string()))
        });
    rls.release().await;
    out
}

/// Delete a certification.
async fn delete_certification(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .delete_certification(&mut **rls.conn(), org_id, cert_id)
        .await
        .map_err(internal_error)
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((StatusCode::NOT_FOUND, "Certification not found".to_string()))
            }
        });
    rls.release().await;
    out
}

/// Get certifications expiring soon.
async fn get_expiring_certifications(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ExpiringQuery>,
) -> Result<Json<Vec<BuildingCertification>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .get_expiring_certifications(&mut **rls.conn(), org_id, query.days.unwrap_or(90))
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

// ==================== Credits ====================

/// List credits for a certification.
async fn list_credits(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<Vec<CertificationCredit>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .list_credits(&mut **rls.conn(), org_id, cert_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Create a certification credit.
async fn create_credit(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
    Json(mut input): Json<CreateCertificationCredit>,
) -> Result<(StatusCode, Json<CertificationCredit>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    input.certification_id = cert_id;

    let out = state
        .building_certification_repo
        .create_credit(&mut **rls.conn(), org_id, input)
        .await
        .map(|credit| (StatusCode::CREATED, Json(credit)))
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Get a credit by ID.
async fn get_credit(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_cert_id, credit_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<CertificationCredit>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .get_credit(&mut **rls.conn(), org_id, credit_id)
        .await
        .map_err(internal_error)
        .and_then(|c| {
            c.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Credit not found".to_string()))
        });
    rls.release().await;
    out
}

/// Update a credit.
async fn update_credit(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_cert_id, credit_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateCertificationCredit>,
) -> Result<Json<CertificationCredit>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .update_credit(&mut **rls.conn(), org_id, credit_id, input)
        .await
        .map_err(internal_error)
        .and_then(|c| {
            c.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Credit not found".to_string()))
        });
    rls.release().await;
    out
}

/// Delete a credit.
async fn delete_credit(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_cert_id, credit_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .delete_credit(&mut **rls.conn(), org_id, credit_id)
        .await
        .map_err(internal_error)
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((StatusCode::NOT_FOUND, "Credit not found".to_string()))
            }
        });
    rls.release().await;
    out
}

// ==================== Documents ====================

/// List documents for a certification.
async fn list_documents(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<Vec<CertificationDocument>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .list_documents(&mut **rls.conn(), org_id, cert_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Create a certification document.
async fn create_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
    Json(mut input): Json<CreateCertificationDocument>,
) -> Result<(StatusCode, Json<CertificationDocument>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    input.certification_id = cert_id;

    let out = state
        .building_certification_repo
        .create_document(&mut **rls.conn(), org_id, input, Some(user_id))
        .await
        .map(|doc| (StatusCode::CREATED, Json(doc)))
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Delete a document.
async fn delete_document(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_cert_id, doc_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .delete_document(&mut **rls.conn(), org_id, doc_id)
        .await
        .map_err(internal_error)
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((StatusCode::NOT_FOUND, "Document not found".to_string()))
            }
        });
    rls.release().await;
    out
}

// ==================== Milestones ====================

/// List milestones for a certification.
async fn list_milestones(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<Vec<CertificationMilestone>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .list_milestones(&mut **rls.conn(), org_id, cert_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Create a certification milestone.
async fn create_milestone(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
    Json(mut input): Json<CreateCertificationMilestone>,
) -> Result<(StatusCode, Json<CertificationMilestone>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    input.certification_id = cert_id;

    let out = state
        .building_certification_repo
        .create_milestone(&mut **rls.conn(), org_id, input)
        .await
        .map(|milestone| (StatusCode::CREATED, Json(milestone)))
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Update a milestone.
async fn update_milestone(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_cert_id, milestone_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateCertificationMilestone>,
) -> Result<Json<CertificationMilestone>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .update_milestone(&mut **rls.conn(), org_id, milestone_id, input)
        .await
        .map_err(internal_error)
        .and_then(|m| {
            m.map(Json)
                .ok_or((StatusCode::NOT_FOUND, "Milestone not found".to_string()))
        });
    rls.release().await;
    out
}

/// Delete a milestone.
async fn delete_milestone(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_cert_id, milestone_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .delete_milestone(&mut **rls.conn(), org_id, milestone_id)
        .await
        .map_err(internal_error)
        .and_then(|deleted| {
            if deleted {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((StatusCode::NOT_FOUND, "Milestone not found".to_string()))
            }
        });
    rls.release().await;
    out
}

// ==================== Benchmarks ====================

/// List benchmarks for a certification.
async fn list_benchmarks(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<Vec<CertificationBenchmark>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .list_benchmarks(&mut **rls.conn(), org_id, cert_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Create a certification benchmark.
async fn create_benchmark(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
    Json(mut input): Json<CreateCertificationBenchmark>,
) -> Result<(StatusCode, Json<CertificationBenchmark>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    input.certification_id = cert_id;

    let out = state
        .building_certification_repo
        .create_benchmark(&mut **rls.conn(), org_id, input)
        .await
        .map(|benchmark| (StatusCode::CREATED, Json(benchmark)))
        .map_err(internal_error);
    rls.release().await;
    out
}

// ==================== Costs ====================

/// List costs for a certification.
async fn list_costs(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<Vec<CertificationCost>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .list_costs(&mut **rls.conn(), org_id, cert_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Create a certification cost.
async fn create_cost(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
    Json(mut input): Json<CreateCertificationCost>,
) -> Result<(StatusCode, Json<CertificationCost>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    input.certification_id = cert_id;

    let out = state
        .building_certification_repo
        .create_cost(&mut **rls.conn(), org_id, input, Some(user_id))
        .await
        .map(|cost| (StatusCode::CREATED, Json(cost)))
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Get total costs for a certification.
async fn get_total_costs(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<rust_decimal::Decimal>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .get_total_costs(&mut **rls.conn(), org_id, cert_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

// ==================== Reminders ====================

/// List reminders for a certification.
async fn list_reminders(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
) -> Result<Json<Vec<CertificationReminder>>, (StatusCode, String)> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .list_reminders(&mut **rls.conn(), org_id, cert_id)
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}

/// Create a certification reminder.
async fn create_reminder(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
    Json(mut input): Json<CreateCertificationReminder>,
) -> Result<(StatusCode, Json<CertificationReminder>), (StatusCode, String)> {
    let org_id = rls.tenant_id();
    input.certification_id = cert_id;

    let out = state
        .building_certification_repo
        .create_reminder(&mut **rls.conn(), org_id, input)
        .await
        .map(|reminder| (StatusCode::CREATED, Json(reminder)))
        .map_err(internal_error);
    rls.release().await;
    out
}

// ==================== Audit Logs ====================

/// List audit logs for a certification.
async fn list_audit_logs(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(cert_id): Path<Uuid>,
    Query(query): Query<AuditLogQuery>,
) -> Result<
    Json<Vec<db::models::building_certification::CertificationAuditLog>>,
    (StatusCode, String),
> {
    let org_id = rls.tenant_id();
    let out = state
        .building_certification_repo
        .list_audit_logs(
            &mut **rls.conn(),
            org_id,
            cert_id,
            clamp_limit(query.limit, 100),
        )
        .await
        .map(Json)
        .map_err(internal_error);
    rls.release().await;
    out
}
