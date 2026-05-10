//! Infrastructure & Operations routes (Epic 73).
//!
//! Implements blue-green deployment management, database migration safety,
//! disaster recovery, and cost monitoring endpoints.

use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::operations::{
    BackupQuery, CostAlertQuery, CostQuery, CreateBackup, CreateCostBudget, CreateDeployment,
    CreateMigration, DeploymentQuery, InitiateRecovery, MigrationQuery, RecordDrDrill,
    RecordInfrastructureCost, SwitchTraffic, UpdateDeploymentStatus, UpdateMigrationProgress,
};
use uuid::Uuid;

use crate::state::AppState;

/// Default page size for listings.
const DEFAULT_LIST_LIMIT: i64 = 50;

/// Maximum page size to prevent excessive data retrieval.
const MAX_LIST_LIMIT: i64 = 100;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

/// Apply pagination defaults and limits to a query limit.
/// Ensures limit does not exceed MAX_LIST_LIMIT.
fn apply_pagination_limit(limit: i64) -> i64 {
    limit.min(MAX_LIST_LIMIT)
}

/// Create operations routes router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Blue-Green Deployment (Story 73.1)
        .route("/deployments", get(list_deployments))
        .route("/deployments", post(create_deployment))
        .route("/deployments/dashboard", get(get_deployment_dashboard))
        .route("/deployments/{id}", get(get_deployment))
        .route("/deployments/{id}/status", put(update_deployment_status))
        .route("/deployments/{id}/switch", post(switch_traffic))
        .route("/deployments/{id}/rollback", post(rollback_deployment))
        .route(
            "/deployments/{id}/health-checks",
            get(list_deployment_health_checks),
        )
        .route("/deployments/{id}/health-checks", post(run_health_checks))
        // Database Migration Safety (Story 73.2)
        .route("/migrations", get(list_migrations))
        .route("/migrations", post(create_migration))
        .route("/migrations/{id}", get(get_migration))
        .route("/migrations/{id}/progress", put(update_migration_progress))
        .route("/migrations/{id}/logs", get(list_migration_logs))
        .route("/migrations/{id}/rollback", post(rollback_migration))
        .route("/migrations/{id}/safety-check", get(check_migration_safety))
        .route("/schema/versions", get(list_schema_versions))
        .route("/schema/current", get(get_current_schema_version))
        // Disaster Recovery (Story 73.3)
        .route("/backups", get(list_backups))
        .route("/backups", post(create_backup))
        .route("/backups/dashboard", get(get_dr_dashboard))
        .route("/backups/{id}", get(get_backup))
        .route("/backups/{id}/verify", post(verify_backup))
        .route("/recovery", post(initiate_recovery))
        .route("/recovery/{id}", get(get_recovery_status))
        .route("/dr/drills", get(list_dr_drills))
        .route("/dr/drills", post(record_dr_drill))
        // Cost Monitoring (Story 73.4)
        .route("/costs", get(list_costs))
        .route("/costs", post(record_cost))
        .route("/costs/dashboard", get(get_cost_dashboard))
        .route("/costs/budgets", get(list_budgets))
        .route("/costs/budgets", post(create_budget))
        .route("/costs/budgets/{id}", get(get_budget))
        .route("/costs/budgets/{id}", put(update_budget))
        .route("/costs/alerts", get(list_cost_alerts))
        .route(
            "/costs/alerts/{id}/acknowledge",
            post(acknowledge_cost_alert),
        )
        .route("/costs/utilization", get(list_resource_utilization))
        .route(
            "/costs/recommendations",
            get(list_optimization_recommendations),
        )
        .route(
            "/costs/recommendations/{id}/implement",
            post(mark_recommendation_implemented),
        )
}

fn internal_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new("DB_ERROR", msg)),
    )
}

fn not_found_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", msg)),
    )
}

fn forbidden_error(msg: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse::new("FORBIDDEN", msg)),
    )
}

// ==================== Blue-Green Deployment Handlers (Story 73.1) ====================

/// List deployments.
#[utoipa::path(
    get,
    path = "/api/v1/operations/deployments",
    params(DeploymentQuery),
    responses(
        (status = 200, description = "Deployments list", body = db::models::operations::ListDeploymentsResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn list_deployments(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<DeploymentQuery>,
) -> ApiResult<Json<db::models::operations::ListDeploymentsResponse>> {
    // Only platform admins can access operations endpoints
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access operations",
        ));
    }

    let repo = &state.operations_repo;
    let limit = apply_pagination_limit(query.limit);

    repo.list_deployments(query.environment, query.status, limit, query.offset)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list deployments: {:?}", e);
            internal_error("Failed to list deployments")
        })
}

/// Create a new deployment.
#[utoipa::path(
    post,
    path = "/api/v1/operations/deployments",
    request_body = CreateDeployment,
    responses(
        (status = 201, description = "Deployment created", body = db::models::operations::Deployment),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn create_deployment(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateDeployment>,
) -> ApiResult<(StatusCode, Json<db::models::operations::Deployment>)> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can create deployments",
        ));
    }

    let repo = &state.operations_repo;

    repo.create_deployment(auth.user_id, payload)
        .await
        .map(|deployment| (StatusCode::CREATED, Json(deployment)))
        .map_err(|e| {
            tracing::error!("Failed to create deployment: {:?}", e);
            internal_error("Failed to create deployment")
        })
}

/// Get deployment dashboard.
#[utoipa::path(
    get,
    path = "/api/v1/operations/deployments/dashboard",
    responses(
        (status = 200, description = "Deployment dashboard", body = db::models::operations::DeploymentDashboard),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn get_deployment_dashboard(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<db::models::operations::DeploymentDashboard>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access deployment dashboard",
        ));
    }

    let repo = &state.operations_repo;

    repo.get_deployment_dashboard()
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to get deployment dashboard: {:?}", e);
            internal_error("Failed to get deployment dashboard")
        })
}

/// Get deployment by ID.
#[utoipa::path(
    get,
    path = "/api/v1/operations/deployments/{id}",
    params(("id" = Uuid, Path, description = "Deployment ID")),
    responses(
        (status = 200, description = "Deployment details", body = db::models::operations::Deployment),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn get_deployment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::operations::Deployment>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access deployments",
        ));
    }

    let repo = &state.operations_repo;

    match repo.get_deployment(id).await {
        Ok(Some(deployment)) => Ok(Json(deployment)),
        Ok(None) => Err(not_found_error("Deployment not found")),
        Err(e) => {
            tracing::error!("Failed to get deployment: {:?}", e);
            Err(internal_error("Failed to get deployment"))
        }
    }
}

/// Update deployment status.
#[utoipa::path(
    put,
    path = "/api/v1/operations/deployments/{id}/status",
    params(("id" = Uuid, Path, description = "Deployment ID")),
    request_body = UpdateDeploymentStatus,
    responses(
        (status = 200, description = "Status updated", body = db::models::operations::Deployment),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn update_deployment_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateDeploymentStatus>,
) -> ApiResult<Json<db::models::operations::Deployment>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can update deployments",
        ));
    }

    let repo = &state.operations_repo;

    match repo.update_deployment_status(id, payload).await {
        Ok(deployment) => Ok(Json(deployment)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Deployment not found")),
        Err(e) => {
            tracing::error!("Failed to update deployment status: {:?}", e);
            Err(internal_error("Failed to update deployment status"))
        }
    }
}

/// Switch traffic to deployment.
#[utoipa::path(
    post,
    path = "/api/v1/operations/deployments/{id}/switch",
    params(("id" = Uuid, Path, description = "Deployment ID")),
    request_body = SwitchTraffic,
    responses(
        (status = 200, description = "Traffic switched", body = db::models::operations::Deployment),
        (status = 400, description = "Health checks not passed", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn switch_traffic(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<SwitchTraffic>,
) -> ApiResult<Json<db::models::operations::Deployment>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can switch traffic",
        ));
    }

    let repo = &state.operations_repo;

    match repo.switch_traffic(id, payload.force).await {
        Ok(deployment) => Ok(Json(deployment)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Deployment not found")),
        Err(e) => {
            tracing::error!("Failed to switch traffic: {:?}", e);
            Err(internal_error("Failed to switch traffic"))
        }
    }
}

/// Rollback deployment.
#[utoipa::path(
    post,
    path = "/api/v1/operations/deployments/{id}/rollback",
    params(("id" = Uuid, Path, description = "Deployment ID")),
    responses(
        (status = 200, description = "Deployment rolled back", body = db::models::operations::Deployment),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn rollback_deployment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::operations::Deployment>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can rollback deployments",
        ));
    }

    let repo = &state.operations_repo;

    match repo.rollback_deployment(id).await {
        Ok(deployment) => Ok(Json(deployment)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Deployment not found")),
        Err(e) => {
            tracing::error!("Failed to rollback deployment: {:?}", e);
            Err(internal_error("Failed to rollback deployment"))
        }
    }
}

/// List deployment health checks.
#[utoipa::path(
    get,
    path = "/api/v1/operations/deployments/{id}/health-checks",
    params(("id" = Uuid, Path, description = "Deployment ID")),
    responses(
        (status = 200, description = "Health checks list", body = Vec<db::models::operations::DeploymentHealthCheck>),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn list_deployment_health_checks(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::operations::DeploymentHealthCheck>>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can view health checks",
        ));
    }

    let repo = &state.operations_repo;

    repo.list_health_checks(id).await.map(Json).map_err(|e| {
        tracing::error!("Failed to list health checks: {:?}", e);
        internal_error("Failed to list health checks")
    })
}

/// Run health checks for deployment.
#[utoipa::path(
    post,
    path = "/api/v1/operations/deployments/{id}/health-checks",
    params(("id" = Uuid, Path, description = "Deployment ID")),
    responses(
        (status = 200, description = "Health checks executed", body = Vec<db::models::operations::DeploymentHealthCheck>),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn run_health_checks(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::operations::DeploymentHealthCheck>>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can run health checks",
        ));
    }

    let repo = &state.operations_repo;

    match repo.run_health_checks(id).await {
        Ok(checks) => Ok(Json(checks)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Deployment not found")),
        Err(e) => {
            tracing::error!("Failed to run health checks: {:?}", e);
            Err(internal_error("Failed to run health checks"))
        }
    }
}

// ==================== Database Migration Safety Handlers (Story 73.2) ====================

/// List migrations.
#[utoipa::path(
    get,
    path = "/api/v1/operations/migrations",
    params(MigrationQuery),
    responses(
        (status = 200, description = "Migrations list", body = db::models::operations::ListMigrationsResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn list_migrations(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<MigrationQuery>,
) -> ApiResult<Json<db::models::operations::ListMigrationsResponse>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access migrations",
        ));
    }

    let repo = &state.operations_repo;
    let limit = apply_pagination_limit(query.limit);

    repo.list_migrations(query.status, limit, query.offset)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list migrations: {:?}", e);
            internal_error("Failed to list migrations")
        })
}

/// Create a new migration record.
#[utoipa::path(
    post,
    path = "/api/v1/operations/migrations",
    request_body = CreateMigration,
    responses(
        (status = 201, description = "Migration created", body = db::models::operations::DatabaseMigration),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn create_migration(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateMigration>,
) -> ApiResult<(StatusCode, Json<db::models::operations::DatabaseMigration>)> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can create migrations",
        ));
    }

    let repo = &state.operations_repo;

    repo.create_migration(auth.user_id, payload)
        .await
        .map(|migration| (StatusCode::CREATED, Json(migration)))
        .map_err(|e| {
            tracing::error!("Failed to create migration: {:?}", e);
            internal_error("Failed to create migration")
        })
}

/// Get migration by ID.
#[utoipa::path(
    get,
    path = "/api/v1/operations/migrations/{id}",
    params(("id" = Uuid, Path, description = "Migration ID")),
    responses(
        (status = 200, description = "Migration details", body = db::models::operations::DatabaseMigration),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn get_migration(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::operations::DatabaseMigration>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access migrations",
        ));
    }

    let repo = &state.operations_repo;

    match repo.get_migration(id).await {
        Ok(Some(migration)) => Ok(Json(migration)),
        Ok(None) => Err(not_found_error("Migration not found")),
        Err(e) => {
            tracing::error!("Failed to get migration: {:?}", e);
            Err(internal_error("Failed to get migration"))
        }
    }
}

/// Update migration progress.
#[utoipa::path(
    put,
    path = "/api/v1/operations/migrations/{id}/progress",
    params(("id" = Uuid, Path, description = "Migration ID")),
    request_body = UpdateMigrationProgress,
    responses(
        (status = 200, description = "Progress updated", body = db::models::operations::DatabaseMigration),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn update_migration_progress(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateMigrationProgress>,
) -> ApiResult<Json<db::models::operations::DatabaseMigration>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can update migrations",
        ));
    }

    let repo = &state.operations_repo;

    match repo.update_migration_progress(id, payload).await {
        Ok(migration) => Ok(Json(migration)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Migration not found")),
        Err(e) => {
            tracing::error!("Failed to update migration progress: {:?}", e);
            Err(internal_error("Failed to update migration progress"))
        }
    }
}

/// List migration logs.
#[utoipa::path(
    get,
    path = "/api/v1/operations/migrations/{id}/logs",
    params(("id" = Uuid, Path, description = "Migration ID")),
    responses(
        (status = 200, description = "Migration logs", body = Vec<db::models::operations::MigrationLog>),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn list_migration_logs(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::operations::MigrationLog>>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can view migration logs",
        ));
    }

    let repo = &state.operations_repo;

    repo.list_migration_logs(id).await.map(Json).map_err(|e| {
        tracing::error!("Failed to list migration logs: {:?}", e);
        internal_error("Failed to list migration logs")
    })
}

/// Rollback migration.
#[utoipa::path(
    post,
    path = "/api/v1/operations/migrations/{id}/rollback",
    params(("id" = Uuid, Path, description = "Migration ID")),
    responses(
        (status = 200, description = "Migration rolled back", body = db::models::operations::DatabaseMigration),
        (status = 400, description = "Cannot rollback", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn rollback_migration(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::operations::DatabaseMigration>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can rollback migrations",
        ));
    }

    let repo = &state.operations_repo;

    match repo.rollback_migration(id).await {
        Ok(migration) => Ok(Json(migration)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Migration not found")),
        Err(e) => {
            tracing::error!("Failed to rollback migration: {:?}", e);
            Err(internal_error("Failed to rollback migration"))
        }
    }
}

/// Check migration safety.
#[utoipa::path(
    get,
    path = "/api/v1/operations/migrations/{id}/safety-check",
    params(("id" = Uuid, Path, description = "Migration ID")),
    responses(
        (status = 200, description = "Safety check result", body = db::models::operations::MigrationSafetyCheck),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn check_migration_safety(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::operations::MigrationSafetyCheck>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can check migration safety",
        ));
    }

    let repo = &state.operations_repo;

    match repo.check_migration_safety(id).await {
        Ok(check) => Ok(Json(check)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Migration not found")),
        Err(e) => {
            tracing::error!("Failed to check migration safety: {:?}", e);
            Err(internal_error("Failed to check migration safety"))
        }
    }
}

/// List schema versions.
#[utoipa::path(
    get,
    path = "/api/v1/operations/schema/versions",
    responses(
        (status = 200, description = "Schema versions", body = Vec<db::models::operations::SchemaVersion>),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn list_schema_versions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<db::models::operations::SchemaVersion>>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can view schema versions",
        ));
    }

    let repo = &state.operations_repo;

    repo.list_schema_versions().await.map(Json).map_err(|e| {
        tracing::error!("Failed to list schema versions: {:?}", e);
        internal_error("Failed to list schema versions")
    })
}

/// Get current schema version.
#[utoipa::path(
    get,
    path = "/api/v1/operations/schema/current",
    responses(
        (status = 200, description = "Current schema version", body = db::models::operations::SchemaVersion),
        (status = 404, description = "No schema version found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn get_current_schema_version(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<db::models::operations::SchemaVersion>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can view schema version",
        ));
    }

    let repo = &state.operations_repo;

    match repo.get_current_schema_version().await {
        Ok(Some(version)) => Ok(Json(version)),
        Ok(None) => Err(not_found_error("No schema version found")),
        Err(e) => {
            tracing::error!("Failed to get current schema version: {:?}", e);
            Err(internal_error("Failed to get current schema version"))
        }
    }
}

// ==================== Disaster Recovery Handlers (Story 73.3) ====================

/// List backups.
#[utoipa::path(
    get,
    path = "/api/v1/operations/backups",
    params(BackupQuery),
    responses(
        (status = 200, description = "Backups list", body = db::models::operations::ListBackupsResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn list_backups(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<BackupQuery>,
) -> ApiResult<Json<db::models::operations::ListBackupsResponse>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access backups",
        ));
    }

    let repo = &state.operations_repo;
    let limit = apply_pagination_limit(query.limit);

    repo.list_backups(query.backup_type, query.status, limit, query.offset)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list backups: {:?}", e);
            internal_error("Failed to list backups")
        })
}

/// Create a new backup.
#[utoipa::path(
    post,
    path = "/api/v1/operations/backups",
    request_body = CreateBackup,
    responses(
        (status = 201, description = "Backup initiated", body = db::models::operations::Backup),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn create_backup(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateBackup>,
) -> ApiResult<(StatusCode, Json<db::models::operations::Backup>)> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can create backups",
        ));
    }

    let repo = &state.operations_repo;

    repo.create_backup(payload)
        .await
        .map(|backup| (StatusCode::CREATED, Json(backup)))
        .map_err(|e| {
            tracing::error!("Failed to create backup: {:?}", e);
            internal_error("Failed to create backup")
        })
}

/// Get disaster recovery dashboard.
#[utoipa::path(
    get,
    path = "/api/v1/operations/backups/dashboard",
    responses(
        (status = 200, description = "DR dashboard", body = db::models::operations::DisasterRecoveryDashboard),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn get_dr_dashboard(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<db::models::operations::DisasterRecoveryDashboard>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access DR dashboard",
        ));
    }

    let repo = &state.operations_repo;

    repo.get_dr_dashboard().await.map(Json).map_err(|e| {
        tracing::error!("Failed to get DR dashboard: {:?}", e);
        internal_error("Failed to get DR dashboard")
    })
}

/// Get backup by ID.
#[utoipa::path(
    get,
    path = "/api/v1/operations/backups/{id}",
    params(("id" = Uuid, Path, description = "Backup ID")),
    responses(
        (status = 200, description = "Backup details", body = db::models::operations::Backup),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn get_backup(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::operations::Backup>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access backups",
        ));
    }

    let repo = &state.operations_repo;

    match repo.get_backup(id).await {
        Ok(Some(backup)) => Ok(Json(backup)),
        Ok(None) => Err(not_found_error("Backup not found")),
        Err(e) => {
            tracing::error!("Failed to get backup: {:?}", e);
            Err(internal_error("Failed to get backup"))
        }
    }
}

/// Verify backup integrity.
#[utoipa::path(
    post,
    path = "/api/v1/operations/backups/{id}/verify",
    params(("id" = Uuid, Path, description = "Backup ID")),
    responses(
        (status = 200, description = "Backup verified", body = db::models::operations::Backup),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn verify_backup(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::operations::Backup>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can verify backups",
        ));
    }

    let repo = &state.operations_repo;

    match repo.verify_backup(id).await {
        Ok(backup) => Ok(Json(backup)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Backup not found")),
        Err(e) => {
            tracing::error!("Failed to verify backup: {:?}", e);
            Err(internal_error("Failed to verify backup"))
        }
    }
}

/// Initiate recovery operation.
#[utoipa::path(
    post,
    path = "/api/v1/operations/recovery",
    request_body = InitiateRecovery,
    responses(
        (status = 201, description = "Recovery initiated", body = db::models::operations::RecoveryOperation),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn initiate_recovery(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<InitiateRecovery>,
) -> ApiResult<(StatusCode, Json<db::models::operations::RecoveryOperation>)> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can initiate recovery",
        ));
    }

    let repo = &state.operations_repo;

    repo.initiate_recovery(auth.user_id, payload)
        .await
        .map(|recovery| (StatusCode::CREATED, Json(recovery)))
        .map_err(|e| {
            tracing::error!("Failed to initiate recovery: {:?}", e);
            internal_error("Failed to initiate recovery")
        })
}

/// Get recovery operation status.
#[utoipa::path(
    get,
    path = "/api/v1/operations/recovery/{id}",
    params(("id" = Uuid, Path, description = "Recovery operation ID")),
    responses(
        (status = 200, description = "Recovery status", body = db::models::operations::RecoveryOperation),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn get_recovery_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::operations::RecoveryOperation>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can view recovery status",
        ));
    }

    let repo = &state.operations_repo;

    match repo.get_recovery_operation(id).await {
        Ok(Some(recovery)) => Ok(Json(recovery)),
        Ok(None) => Err(not_found_error("Recovery operation not found")),
        Err(e) => {
            tracing::error!("Failed to get recovery status: {:?}", e);
            Err(internal_error("Failed to get recovery status"))
        }
    }
}

/// List DR drills.
#[utoipa::path(
    get,
    path = "/api/v1/operations/dr/drills",
    responses(
        (status = 200, description = "DR drills list", body = Vec<db::models::operations::DisasterRecoveryDrill>),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn list_dr_drills(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<db::models::operations::DisasterRecoveryDrill>>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can view DR drills",
        ));
    }

    let repo = &state.operations_repo;

    repo.list_dr_drills().await.map(Json).map_err(|e| {
        tracing::error!("Failed to list DR drills: {:?}", e);
        internal_error("Failed to list DR drills")
    })
}

/// Record DR drill.
#[utoipa::path(
    post,
    path = "/api/v1/operations/dr/drills",
    request_body = RecordDrDrill,
    responses(
        (status = 201, description = "DR drill recorded", body = db::models::operations::DisasterRecoveryDrill),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn record_dr_drill(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<RecordDrDrill>,
) -> ApiResult<(
    StatusCode,
    Json<db::models::operations::DisasterRecoveryDrill>,
)> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can record DR drills",
        ));
    }

    let repo = &state.operations_repo;

    repo.record_dr_drill(auth.user_id, payload)
        .await
        .map(|drill| (StatusCode::CREATED, Json(drill)))
        .map_err(|e| {
            tracing::error!("Failed to record DR drill: {:?}", e);
            internal_error("Failed to record DR drill")
        })
}

// ==================== Cost Monitoring Handlers (Story 73.4) ====================

/// List infrastructure costs.
#[utoipa::path(
    get,
    path = "/api/v1/operations/costs",
    params(CostQuery),
    responses(
        (status = 200, description = "Costs list", body = db::models::operations::ListCostsResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn list_costs(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<CostQuery>,
) -> ApiResult<Json<db::models::operations::ListCostsResponse>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access cost data",
        ));
    }

    let repo = &state.operations_repo;
    let limit = apply_pagination_limit(query.limit);

    repo.list_costs(
        query.period_start,
        query.period_end,
        query.service_type,
        limit,
        query.offset,
    )
    .await
    .map(Json)
    .map_err(|e| {
        tracing::error!("Failed to list costs: {:?}", e);
        internal_error("Failed to list costs")
    })
}

/// Record infrastructure cost.
#[utoipa::path(
    post,
    path = "/api/v1/operations/costs",
    request_body = RecordInfrastructureCost,
    responses(
        (status = 201, description = "Cost recorded", body = db::models::operations::InfrastructureCost),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn record_cost(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<RecordInfrastructureCost>,
) -> ApiResult<(StatusCode, Json<db::models::operations::InfrastructureCost>)> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can record costs",
        ));
    }

    let repo = &state.operations_repo;

    repo.record_cost(payload)
        .await
        .map(|cost| (StatusCode::CREATED, Json(cost)))
        .map_err(|e| {
            tracing::error!("Failed to record cost: {:?}", e);
            internal_error("Failed to record cost")
        })
}

/// Get cost dashboard.
#[utoipa::path(
    get,
    path = "/api/v1/operations/costs/dashboard",
    responses(
        (status = 200, description = "Cost dashboard", body = db::models::operations::CostDashboard),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn get_cost_dashboard(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<db::models::operations::CostDashboard>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access cost dashboard",
        ));
    }

    let repo = &state.operations_repo;

    repo.get_cost_dashboard().await.map(Json).map_err(|e| {
        tracing::error!("Failed to get cost dashboard: {:?}", e);
        internal_error("Failed to get cost dashboard")
    })
}

/// List cost budgets.
#[utoipa::path(
    get,
    path = "/api/v1/operations/costs/budgets",
    responses(
        (status = 200, description = "Budgets list", body = db::models::operations::ListBudgetsResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn list_budgets(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<db::models::operations::ListBudgetsResponse>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access budgets",
        ));
    }

    let repo = &state.operations_repo;

    repo.list_budgets().await.map(Json).map_err(|e| {
        tracing::error!("Failed to list budgets: {:?}", e);
        internal_error("Failed to list budgets")
    })
}

/// Create cost budget.
#[utoipa::path(
    post,
    path = "/api/v1/operations/costs/budgets",
    request_body = CreateCostBudget,
    responses(
        (status = 201, description = "Budget created", body = db::models::operations::CostBudget),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn create_budget(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateCostBudget>,
) -> ApiResult<(StatusCode, Json<db::models::operations::CostBudget>)> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can create budgets",
        ));
    }

    let repo = &state.operations_repo;

    repo.create_budget(payload)
        .await
        .map(|budget| (StatusCode::CREATED, Json(budget)))
        .map_err(|e| {
            tracing::error!("Failed to create budget: {:?}", e);
            internal_error("Failed to create budget")
        })
}

/// Get budget by ID.
#[utoipa::path(
    get,
    path = "/api/v1/operations/costs/budgets/{id}",
    params(("id" = Uuid, Path, description = "Budget ID")),
    responses(
        (status = 200, description = "Budget details", body = db::models::operations::CostBudget),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn get_budget(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::operations::CostBudget>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access budgets",
        ));
    }

    let repo = &state.operations_repo;

    match repo.get_budget(id).await {
        Ok(Some(budget)) => Ok(Json(budget)),
        Ok(None) => Err(not_found_error("Budget not found")),
        Err(e) => {
            tracing::error!("Failed to get budget: {:?}", e);
            Err(internal_error("Failed to get budget"))
        }
    }
}

/// Update budget.
#[utoipa::path(
    put,
    path = "/api/v1/operations/costs/budgets/{id}",
    params(("id" = Uuid, Path, description = "Budget ID")),
    request_body = CreateCostBudget,
    responses(
        (status = 200, description = "Budget updated", body = db::models::operations::CostBudget),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn update_budget(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateCostBudget>,
) -> ApiResult<Json<db::models::operations::CostBudget>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can update budgets",
        ));
    }

    let repo = &state.operations_repo;

    match repo.update_budget(id, payload).await {
        Ok(budget) => Ok(Json(budget)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Budget not found")),
        Err(e) => {
            tracing::error!("Failed to update budget: {:?}", e);
            Err(internal_error("Failed to update budget"))
        }
    }
}

/// List cost alerts.
#[utoipa::path(
    get,
    path = "/api/v1/operations/costs/alerts",
    params(CostAlertQuery),
    responses(
        (status = 200, description = "Cost alerts list", body = db::models::operations::ListCostAlertsResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn list_cost_alerts(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<CostAlertQuery>,
) -> ApiResult<Json<db::models::operations::ListCostAlertsResponse>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access cost alerts",
        ));
    }

    let repo = &state.operations_repo;
    let limit = apply_pagination_limit(query.limit);

    repo.list_cost_alerts(
        query.unacknowledged_only,
        query.severity,
        limit,
        query.offset,
    )
    .await
    .map(Json)
    .map_err(|e| {
        tracing::error!("Failed to list cost alerts: {:?}", e);
        internal_error("Failed to list cost alerts")
    })
}

/// Acknowledge cost alert.
#[utoipa::path(
    post,
    path = "/api/v1/operations/costs/alerts/{id}/acknowledge",
    params(("id" = Uuid, Path, description = "Alert ID")),
    responses(
        (status = 200, description = "Alert acknowledged", body = db::models::operations::CostAlert),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn acknowledge_cost_alert(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::operations::CostAlert>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can acknowledge cost alerts",
        ));
    }

    let repo = &state.operations_repo;

    match repo.acknowledge_cost_alert(id, auth.user_id).await {
        Ok(alert) => Ok(Json(alert)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Cost alert not found")),
        Err(e) => {
            tracing::error!("Failed to acknowledge cost alert: {:?}", e);
            Err(internal_error("Failed to acknowledge cost alert"))
        }
    }
}

/// List resource utilization.
#[utoipa::path(
    get,
    path = "/api/v1/operations/costs/utilization",
    responses(
        (status = 200, description = "Resource utilization list", body = db::models::operations::ListUtilizationResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn list_resource_utilization(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<db::models::operations::ListUtilizationResponse>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access utilization data",
        ));
    }

    let repo = &state.operations_repo;

    repo.list_resource_utilization()
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list resource utilization: {:?}", e);
            internal_error("Failed to list resource utilization")
        })
}

/// List cost optimization recommendations.
#[utoipa::path(
    get,
    path = "/api/v1/operations/costs/recommendations",
    responses(
        (status = 200, description = "Recommendations list", body = db::models::operations::ListRecommendationsResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn list_optimization_recommendations(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<db::models::operations::ListRecommendationsResponse>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can access recommendations",
        ));
    }

    let repo = &state.operations_repo;

    repo.list_optimization_recommendations()
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("Failed to list optimization recommendations: {:?}", e);
            internal_error("Failed to list optimization recommendations")
        })
}

/// Mark recommendation as implemented.
#[utoipa::path(
    post,
    path = "/api/v1/operations/costs/recommendations/{id}/implement",
    params(("id" = Uuid, Path, description = "Recommendation ID")),
    responses(
        (status = 200, description = "Recommendation marked implemented", body = db::models::operations::CostOptimizationRecommendation),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 403, description = "Not authorized", body = ErrorResponse),
    ),
    tag = "operations"
)]
async fn mark_recommendation_implemented(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<db::models::operations::CostOptimizationRecommendation>> {
    if !auth.is_platform_admin() {
        return Err(forbidden_error(
            "Only platform administrators can update recommendations",
        ));
    }

    let repo = &state.operations_repo;

    match repo.mark_recommendation_implemented(id, auth.user_id).await {
        Ok(rec) => Ok(Json(rec)),
        Err(sqlx::Error::RowNotFound) => Err(not_found_error("Recommendation not found")),
        Err(e) => {
            tracing::error!("Failed to mark recommendation implemented: {:?}", e);
            Err(internal_error("Failed to mark recommendation implemented"))
        }
    }
}
