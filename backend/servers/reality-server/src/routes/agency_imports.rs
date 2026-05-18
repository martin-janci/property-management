//! Agency import routes (UC-50: Agency Import Management).
//!
//! Per-agency import history, test-connection, run, and job status.
//! D1.2: handlers now use the unified `RequestPrincipal` extractor.

use crate::state::AppState;
use api_core::extractors::RequestPrincipal;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

/// Create agency imports router.
/// Mounted at /api/v1/agencies/:id/imports in main.rs.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_import_history))
        .route("/test-connection", post(test_connection))
        .route("/run", post(run_import))
        .route("/{job_id}", get(get_import_job_status))
}

/// Supported import providers.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImportProvider {
    Reas,
    Bazos,
    Topreality,
    CustomCrm,
    GenericXml,
}

impl std::fmt::Display for ImportProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ImportProvider::Reas => "reas",
            ImportProvider::Bazos => "bazos",
            ImportProvider::Topreality => "topreality",
            ImportProvider::CustomCrm => "custom_crm",
            ImportProvider::GenericXml => "generic_xml",
        };
        write!(f, "{}", s)
    }
}

/// Import job summary (for history list).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ImportJobSummary {
    pub id: Uuid,
    pub agency_id: Option<Uuid>,
    pub provider: String,
    pub status: String,
    pub total_records: i32,
    pub success_count: i32,
    pub skip_count: i32,
    pub failure_count: i32,
    /// Duration in seconds (null if still running).
    pub duration_seconds: Option<i64>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Import history response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ImportHistoryResponse {
    pub jobs: Vec<ImportJobSummary>,
    pub total: i64,
}

/// Test provider credentials request.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TestConnectionRequest {
    pub provider: ImportProvider,
    pub feed_url: Option<String>,
    pub api_key: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Test connection result.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TestConnectionResponse {
    pub success: bool,
    pub message: String,
    pub sample_record_count: Option<i32>,
}

/// Run import request.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RunImportRequest {
    pub provider: ImportProvider,
    pub feed_url: Option<String>,
    pub api_key: Option<String>,
}

/// Run import response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RunImportResponse {
    pub job_id: Uuid,
    pub status: String,
    pub message: String,
}

/// Import job detail with per-row skipped reasons.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ImportJobDetail {
    pub id: Uuid,
    pub agency_id: Option<Uuid>,
    pub provider: String,
    pub status: String,
    pub total_records: i32,
    pub processed_records: i32,
    pub success_count: i32,
    pub skip_count: i32,
    pub failure_count: i32,
    pub duration_seconds: Option<i64>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    /// Per-row skipped/failed reasons from error_log.
    pub skip_reasons: Option<serde_json::Value>,
}

/// Import job detail response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ImportJobDetailResponse {
    pub job: ImportJobDetail,
}

/// List import history for an agency.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}/imports",
    tag = "AgencyImport",
    params(("id" = Uuid, Path, description = "Agency ID")),
    responses(
        (status = 200, description = "Import history", body = ImportHistoryResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of this agency"),
        (status = 404, description = "Agency not found")
    ),
    security(("session_token" = []))
)]
pub async fn list_import_history(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(agency_id): Path<Uuid>,
) -> Result<Json<ImportHistoryResponse>, (axum::http::StatusCode, String)> {
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    check_agency_membership(&mut conn, agency_id, principal.user_id).await?;

    let rows = sqlx::query(
        r#"
        SELECT
            id, agency_id, source_type AS provider, status,
            total_records, success_count, skip_count, failure_count,
            EXTRACT(EPOCH FROM (completed_at - started_at))::bigint AS duration_seconds,
            started_at, completed_at, created_at
        FROM portal_import_jobs
        WHERE agency_id = $1
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .bind(agency_id)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("list import history", e))?;

    let total = rows.len() as i64;
    let jobs: Vec<ImportJobSummary> = rows
        .into_iter()
        .map(|r| ImportJobSummary {
            id: r.get("id"),
            agency_id: r.get("agency_id"),
            provider: r.get("provider"),
            status: r.get("status"),
            total_records: r.get("total_records"),
            success_count: r.get("success_count"),
            skip_count: r.get("skip_count"),
            failure_count: r.get("failure_count"),
            duration_seconds: r.get("duration_seconds"),
            started_at: r.get("started_at"),
            completed_at: r.get("completed_at"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(ImportHistoryResponse { jobs, total }))
}

/// Test provider credentials without starting a full import.
#[utoipa::path(
    post,
    path = "/api/v1/agencies/{id}/imports/test-connection",
    tag = "AgencyImport",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = TestConnectionRequest,
    responses(
        (status = 200, description = "Test result", body = TestConnectionResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of this agency")
    ),
    security(("session_token" = []))
)]
pub async fn test_connection(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(agency_id): Path<Uuid>,
    Json(data): Json<TestConnectionRequest>,
) -> Result<Json<TestConnectionResponse>, (axum::http::StatusCode, String)> {
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    check_agency_membership(&mut conn, agency_id, principal.user_id).await?;

    // Stub: real implementation would call the external provider.
    let response = TestConnectionResponse {
        success: true,
        message: format!(
            "Connection to provider '{}' successful (stub response)",
            data.provider
        ),
        sample_record_count: Some(42),
    };

    Ok(Json(response))
}

/// Kick off a new import job for the agency.
#[utoipa::path(
    post,
    path = "/api/v1/agencies/{id}/imports/run",
    tag = "AgencyImport",
    params(("id" = Uuid, Path, description = "Agency ID")),
    request_body = RunImportRequest,
    responses(
        (status = 202, description = "Import job started", body = RunImportResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of this agency"),
        (status = 404, description = "Agency not found")
    ),
    security(("session_token" = []))
)]
pub async fn run_import(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(agency_id): Path<Uuid>,
    Json(data): Json<RunImportRequest>,
) -> Result<(axum::http::StatusCode, Json<RunImportResponse>), (axum::http::StatusCode, String)> {
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    check_agency_membership(&mut conn, agency_id, principal.user_id).await?;

    let provider_str = data.provider.to_string();

    let row = sqlx::query(
        r#"
        INSERT INTO portal_import_jobs
            (agency_id, user_id, source_type, source_url, status,
             total_records, processed_records, success_count, skip_count, failure_count)
        VALUES ($1, $2, $3, $4, 'pending', 0, 0, 0, 0, 0)
        RETURNING id
        "#,
    )
    .bind(agency_id)
    .bind(principal.user_id)
    .bind(&provider_str)
    .bind(&data.feed_url)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("create import job", e))?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(RunImportResponse {
            job_id: row.get("id"),
            status: "pending".to_string(),
            message: "Import job queued".to_string(),
        }),
    ))
}

/// Get import job status and per-row skipped reasons.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}/imports/{job_id}",
    tag = "AgencyImport",
    params(
        ("id" = Uuid, Path, description = "Agency ID"),
        ("job_id" = Uuid, Path, description = "Import job ID")
    ),
    responses(
        (status = 200, description = "Import job detail", body = ImportJobDetailResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member of this agency"),
        (status = 404, description = "Job not found")
    ),
    security(("session_token" = []))
)]
pub async fn get_import_job_status(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path((agency_id, job_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ImportJobDetailResponse>, (axum::http::StatusCode, String)> {
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| crate::util::errors::db_error("database error", e))?;

    check_agency_membership(&mut conn, agency_id, principal.user_id).await?;

    let row = sqlx::query(
        r#"
        SELECT
            id, agency_id, source_type AS provider, status,
            total_records, processed_records, success_count, skip_count, failure_count,
            EXTRACT(EPOCH FROM (completed_at - started_at))::bigint AS duration_seconds,
            started_at, completed_at, created_at, error_log
        FROM portal_import_jobs
        WHERE id = $1 AND agency_id = $2
        "#,
    )
    .bind(job_id)
    .bind(agency_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| crate::util::errors::db_error("get import job", e))?
    .ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            "Import job not found".to_string(),
        )
    })?;

    let job = ImportJobDetail {
        id: row.get("id"),
        agency_id: row.get("agency_id"),
        provider: row.get("provider"),
        status: row.get("status"),
        total_records: row.get("total_records"),
        processed_records: row.get("processed_records"),
        success_count: row.get("success_count"),
        skip_count: row.get("skip_count"),
        failure_count: row.get("failure_count"),
        duration_seconds: row.get("duration_seconds"),
        started_at: row.get("started_at"),
        completed_at: row.get("completed_at"),
        created_at: row.get("created_at"),
        skip_reasons: row.get("error_log"),
    };

    Ok(Json(ImportJobDetailResponse { job }))
}

/// Helper: verify the user is an active member of the given agency.
async fn check_agency_membership(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    agency_id: Uuid,
    user_id: Uuid,
) -> Result<(), (axum::http::StatusCode, String)> {
    // Check agency exists
    let agency_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM reality_agencies WHERE id = $1)")
            .bind(agency_id)
            .fetch_one(&mut **conn)
            .await
            .map_err(|e| crate::util::errors::db_error("check agency", e))?;

    if !agency_exists {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Agency not found".to_string(),
        ));
    }

    let is_member: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM reality_agency_members
            WHERE agency_id = $1 AND user_id = $2 AND is_active = TRUE
        )
        "#,
    )
    .bind(agency_id)
    .bind(user_id)
    .fetch_one(&mut **conn)
    .await
    .map_err(|e| crate::util::errors::db_error("check membership", e))?;

    if !is_member {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            "Not a member of this agency".to_string(),
        ));
    }

    Ok(())
}
