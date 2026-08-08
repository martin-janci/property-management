//! Reports routes (Epic 55: Advanced Reporting & Analytics).
//!
//! Provides comprehensive reporting endpoints for:
//! - Story 55.1: Fault Statistics Report
//! - Story 55.2: Voting Participation Report
//! - Story 55.3: Occupancy Report
//! - Story 55.4: Consumption Report
//! - Story 55.5: Export Reports to PDF/Excel

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use chrono_tz::Tz;
use common::errors::ErrorResponse;
use db::models::{
    report_schedule::ExecutionHistoryQuery, ConsumptionAnomaly, ConsumptionSummary, DateRange,
    ExecutionDownloadUrl, ExecutionHistoryResponse, FaultStatistics, FaultTrends, OccupancySummary,
    OccupancyTrends, ReportExecution, ReportSchedule, UnitConsumption, UnitOccupancy,
    UtilityTypeConsumption, VoteParticipationDetail, VotingParticipationSummary, YearComparison,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

mod schedule_cadence;

pub(crate) use schedule_cadence::compute_next_run_for_schedule;
use schedule_cadence::{
    compute_first_next_run, compute_next_run_from_cron, parse_time_hhmm, validate_cron_expression,
    DEFAULT_SCHEDULE_TIME, VALID_SCHEDULE_FORMATS, VALID_SCHEDULE_FREQUENCIES,
};

// ============================================================================
// Constants
// ============================================================================

/// Maximum row count for synchronous report generation (Story 88.5).
/// Reports with fewer rows than this threshold are generated immediately.
/// Larger reports use async background job processing.
const SYNC_REPORT_ROW_THRESHOLD: i64 = 1000;

// ============================================================================
// Response Types
// ============================================================================

/// Fault statistics report response (Story 55.1).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FaultStatisticsReportResponse {
    pub building_id: Option<Uuid>,
    pub building_name: Option<String>,
    pub date_range: DateRange,
    pub statistics: FaultStatistics,
    pub trends: FaultTrends,
}

/// Voting participation report response (Story 55.2).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VotingParticipationReportResponse {
    pub building_id: Option<Uuid>,
    pub building_name: Option<String>,
    pub date_range: DateRange,
    pub summary: VotingParticipationSummary,
    pub votes: Vec<VoteParticipationDetail>,
}

/// Occupancy report response (Story 55.3).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OccupancyReportResponse {
    pub building_id: Option<Uuid>,
    pub building_name: Option<String>,
    pub date_range: DateRange,
    pub summary: OccupancySummary,
    pub by_unit: Vec<UnitOccupancy>,
    pub trends: OccupancyTrends,
}

/// Consumption report response (Story 55.4).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ConsumptionReportResponse {
    pub building_id: Option<Uuid>,
    pub building_name: Option<String>,
    pub date_range: DateRange,
    pub summary: ConsumptionSummary,
    pub by_utility_type: Vec<UtilityTypeConsumption>,
    pub by_unit: Vec<UnitConsumption>,
    pub anomalies: Vec<ConsumptionAnomaly>,
}

/// Export report response (Story 55.5).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExportReportResponse {
    pub download_url: String,
    pub format: String,
    pub expires_at: String,
}

/// Synchronous export response with inline data (Story 88.5).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SyncExportReportResponse {
    /// Base64-encoded file content for immediate download
    pub data: String,
    /// Suggested filename for the download
    pub filename: String,
    /// MIME type for the content
    pub content_type: String,
    /// Export format used
    pub format: String,
}

// ============================================================================
// Request Types
// ============================================================================

/// Default report date range in days.
const DEFAULT_FAULT_REPORT_DAYS: i64 = 30;
const DEFAULT_VOTING_REPORT_DAYS: i64 = 365;

/// Query parameters for fault statistics report.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct FaultStatisticsQuery {
    /// Organization ID (required for multi-tenant filtering)
    pub organization_id: Uuid,
    /// Building ID (optional, filters to specific building)
    pub building_id: Option<Uuid>,
    /// Start date for report period
    pub from_date: Option<NaiveDate>,
    /// End date for report period
    pub to_date: Option<NaiveDate>,
}

/// Query parameters for voting participation report.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct VotingParticipationQuery {
    /// Organization ID (required for multi-tenant filtering)
    pub organization_id: Uuid,
    /// Building ID (optional, filters to specific building)
    pub building_id: Option<Uuid>,
    /// Start date for report period
    pub from_date: Option<NaiveDate>,
    /// End date for report period
    pub to_date: Option<NaiveDate>,
}

/// Query parameters for occupancy report.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct OccupancyReportQuery {
    /// Organization ID (required for multi-tenant filtering)
    pub organization_id: Uuid,
    /// Building ID (optional, filters to specific building)
    pub building_id: Option<Uuid>,
    /// Year for report
    pub year: i32,
    /// Month (optional, if not provided returns full year)
    pub month: Option<i32>,
    /// Include comparison with previous year
    #[serde(default)]
    pub include_comparison: bool,
}

/// Query parameters for consumption report.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ConsumptionReportQuery {
    /// Organization ID (required for multi-tenant filtering)
    pub organization_id: Uuid,
    /// Building ID (optional, filters to specific building)
    pub building_id: Option<Uuid>,
    /// Utility type filter (water, electricity, gas, heating)
    pub utility_type: Option<String>,
    /// Start date for report period
    pub from_date: NaiveDate,
    /// End date for report period
    pub to_date: NaiveDate,
    /// Include anomaly detection
    #[serde(default)]
    pub include_anomalies: bool,
}

/// Request for exporting a report.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ExportReportRequest {
    /// Organization ID (required for multi-tenant filtering)
    pub organization_id: Uuid,
    /// Report type to export
    pub report_type: String,
    /// Export format (pdf, excel, csv)
    pub format: String,
    /// Building ID (optional)
    pub building_id: Option<Uuid>,
    /// Start date for report period
    pub from_date: Option<NaiveDate>,
    /// End date for report period
    pub to_date: Option<NaiveDate>,
    /// Additional parameters based on report type
    pub params: Option<serde_json::Value>,
}

// ============================================================================
// Router
// ============================================================================

/// Create reports router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Story 55.1: Fault Statistics Report
        .route("/faults", get(get_fault_statistics_report))
        // Story 55.2: Voting Participation Report
        .route("/voting", get(get_voting_participation_report))
        // Story 55.3: Occupancy Report
        .route("/occupancy", get(get_occupancy_report))
        // Story 55.4: Consumption Report
        .route("/consumption", get(get_consumption_report))
        // Story 55.5: Export Reports (Story 84.1: Background job implementation)
        .route("/export", axum::routing::post(export_report))
        .route("/export/{job_id}/status", get(get_export_job_status))
        // gap-81-1: Create a new report schedule; issue #2324: list schedules
        .route(
            "/schedules",
            axum::routing::post(create_schedule).get(list_schedules),
        )
        // gap-81-1: Update schedule (cron expression, recipients, enabled flag)
        .route("/schedules/{id}", axum::routing::put(update_schedule))
        // Epic 81: Story 81.1 — Schedule pause/resume
        .route("/schedules/{id}/pause", axum::routing::put(pause_schedule))
        .route(
            "/schedules/{id}/resume",
            axum::routing::put(resume_schedule),
        )
        // Epic 81: Story 81.2 — Execution history
        .route("/schedules/{id}/executions", get(list_schedule_executions))
        .route("/executions/{id}", get(get_execution))
        .route("/executions/{id}/download", get(get_execution_download_url))
        .route(
            "/executions/{id}/retry",
            axum::routing::post(retry_execution),
        )
}

// ============================================================================
// Helper functions (extracted to `helpers.rs`)
// ============================================================================

mod helpers;

use helpers::{
    estimate_report_row_count, generate_sync_csv_report, get_building_name,
    get_content_type_for_format,
};

// ============================================================================
// Handlers
// ============================================================================

/// Validate date range and return error if invalid.
fn validate_date_range(
    from_date: NaiveDate,
    to_date: NaiveDate,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if from_date > to_date {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_DATE_RANGE",
                "from_date must be before or equal to to_date",
            )),
        ));
    }
    // Limit range to 5 years max for performance
    let max_days = 365 * 5;
    if (to_date - from_date).num_days() > max_days {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "DATE_RANGE_TOO_LARGE",
                "Date range cannot exceed 5 years",
            )),
        ));
    }
    Ok(())
}

/// Get fault statistics report (Story 55.1).
#[utoipa::path(
    get,
    path = "/api/v1/reports/faults",
    params(FaultStatisticsQuery),
    responses(
        (status = 200, description = "Fault statistics report", body = FaultStatisticsReportResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Reports"
)]
pub async fn get_fault_statistics_report(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<FaultStatisticsQuery>,
) -> Result<Json<FaultStatisticsReportResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get date range (default to last 30 days if not specified)
    let to_date = query
        .to_date
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let from_date = query
        .from_date
        .unwrap_or_else(|| to_date - chrono::Duration::days(DEFAULT_FAULT_REPORT_DAYS));

    // Validate date range
    validate_date_range(from_date, to_date)?;

    // Enforce that the requested organization matches the authenticated tenant
    // (super-admins may query across orgs). Closes a cross-tenant IDOR where a
    // caller could request another org's report by supplying its UUID.
    if !rls.is_super_admin() && query.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "organization_id does not match the authenticated tenant",
            )),
        ));
    }

    // Get building name if building_id is provided
    let building_name = get_building_name(&state, &mut rls, query.building_id).await;
    // RLS lookup complete — clear context and return the connection to the pool.
    rls.release().await;

    // Get fault statistics from repository (using organization_id for multi-tenant filtering)
    let statistics = state
        .fault_repo
        .get_statistics(query.organization_id, query.building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get fault statistics");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to get fault statistics",
                )),
            )
        })?;

    // Get monthly trend data
    let monthly_counts = state
        .fault_repo
        .get_monthly_fault_counts(query.organization_id, query.building_id, from_date, to_date)
        .await
        .unwrap_or_default();

    // Get resolution time trend
    let resolution_time_trend = state
        .fault_repo
        .get_monthly_resolution_times(query.organization_id, query.building_id, from_date, to_date)
        .await
        .unwrap_or_default();

    // Build response
    let response = FaultStatisticsReportResponse {
        building_id: query.building_id,
        building_name,
        date_range: DateRange {
            from: from_date,
            to: to_date,
        },
        statistics,
        trends: FaultTrends {
            monthly_counts,
            resolution_time_trend,
            category_trend: vec![],
        },
    };

    Ok(Json(response))
}

/// Get voting participation report (Story 55.2).
#[utoipa::path(
    get,
    path = "/api/v1/reports/voting",
    params(VotingParticipationQuery),
    responses(
        (status = 200, description = "Voting participation report", body = VotingParticipationReportResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Reports"
)]
pub async fn get_voting_participation_report(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<VotingParticipationQuery>,
) -> Result<Json<VotingParticipationReportResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get date range (default to last 12 months if not specified)
    let to_date = query
        .to_date
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let from_date = query
        .from_date
        .unwrap_or_else(|| to_date - chrono::Duration::days(DEFAULT_VOTING_REPORT_DAYS));

    // Validate date range
    validate_date_range(from_date, to_date)?;

    // Enforce that the requested organization matches the authenticated tenant
    // (super-admins may query across orgs). Closes a cross-tenant IDOR where a
    // caller could request another org's report by supplying its UUID.
    if !rls.is_super_admin() && query.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "organization_id does not match the authenticated tenant",
            )),
        ));
    }

    // Get building name if building_id is provided
    let building_name = get_building_name(&state, &mut rls, query.building_id).await;
    // RLS lookup complete — clear context and return the connection to the pool.
    rls.release().await;

    // Query voting participation data from repository
    let participation_data = state
        .vote_repo
        .get_participation_report(query.organization_id, query.building_id, from_date, to_date)
        .await
        .unwrap_or_default();

    // Calculate summary from participation data
    let total_votes = participation_data.len() as i64;
    let votes_with_quorum = participation_data
        .iter()
        .filter(|v| v.quorum_reached)
        .count() as i64;
    let votes_without_quorum = total_votes - votes_with_quorum;
    let total_responses: i64 = participation_data.iter().map(|v| v.response_count).sum();
    let total_eligible: i64 = participation_data.iter().map(|v| v.eligible_count).sum();
    let average_participation_rate = if total_eligible > 0 {
        (total_responses as f64 / total_eligible as f64) * 100.0
    } else {
        0.0
    };

    let response = VotingParticipationReportResponse {
        building_id: query.building_id,
        building_name,
        date_range: DateRange {
            from: from_date,
            to: to_date,
        },
        summary: VotingParticipationSummary {
            total_votes,
            votes_with_quorum,
            votes_without_quorum,
            average_participation_rate,
            total_eligible_voters: total_eligible,
            total_responses,
        },
        votes: participation_data,
    };

    Ok(Json(response))
}

/// Get occupancy report (Story 55.3).
#[utoipa::path(
    get,
    path = "/api/v1/reports/occupancy",
    params(OccupancyReportQuery),
    responses(
        (status = 200, description = "Occupancy report", body = OccupancyReportResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Reports"
)]
pub async fn get_occupancy_report(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<OccupancyReportQuery>,
) -> Result<Json<OccupancyReportResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate month if provided
    if let Some(month) = query.month {
        if !(1..=12).contains(&month) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_MONTH",
                    "Month must be between 1 and 12",
                )),
            ));
        }
    }

    // Calculate date range
    let from_date = NaiveDate::from_ymd_opt(query.year, query.month.unwrap_or(1) as u32, 1)
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let to_date = if let Some(month) = query.month {
        // Last day of the specified month
        let next_month = if month == 12 {
            NaiveDate::from_ymd_opt(query.year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(query.year, (month + 1) as u32, 1)
        };
        next_month
            .map(|d| d - chrono::Duration::days(1))
            .unwrap_or(from_date)
    } else {
        // Last day of the year
        NaiveDate::from_ymd_opt(query.year, 12, 31).unwrap_or(from_date)
    };

    // Enforce that the requested organization matches the authenticated tenant
    // (super-admins may query across orgs). Closes a cross-tenant IDOR where a
    // caller could request another org's report by supplying its UUID.
    if !rls.is_super_admin() && query.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "organization_id does not match the authenticated tenant",
            )),
        ));
    }

    // Get building name if building_id is provided
    let building_name = get_building_name(&state, &mut rls, query.building_id).await;
    // RLS lookup complete — clear context and return the connection to the pool.
    rls.release().await;

    // Query occupancy data from person_month repository
    let occupancy_data = state
        .person_month_repo
        .get_occupancy_report(query.organization_id, query.building_id, from_date, to_date)
        .await
        .unwrap_or_default();

    // Build year-over-year comparison if requested
    let year_over_year_comparison = if query.include_comparison {
        let prev_from = NaiveDate::from_ymd_opt(query.year - 1, query.month.unwrap_or(1) as u32, 1)
            .unwrap_or(from_date);
        let prev_to = NaiveDate::from_ymd_opt(
            query.year - 1,
            query.month.unwrap_or(12) as u32,
            if query.month.is_some() { 28 } else { 31 },
        )
        .unwrap_or(to_date);

        let prev_data = state
            .person_month_repo
            .get_occupancy_report(query.organization_id, query.building_id, prev_from, prev_to)
            .await
            .unwrap_or_default();

        let current_total = occupancy_data.summary.total_person_months;
        let previous_total = prev_data.summary.total_person_months;
        let change_percentage = if previous_total > 0 {
            ((current_total - previous_total) as f64 / previous_total as f64) * 100.0
        } else {
            0.0
        };

        Some(YearComparison {
            current_year: query.year,
            previous_year: query.year - 1,
            current_total,
            previous_total,
            change_percentage,
        })
    } else {
        None
    };

    let response = OccupancyReportResponse {
        building_id: query.building_id,
        building_name,
        date_range: DateRange {
            from: from_date,
            to: to_date,
        },
        summary: occupancy_data.summary,
        by_unit: occupancy_data.by_unit,
        trends: OccupancyTrends {
            monthly_total: occupancy_data.monthly_totals,
            year_over_year_comparison,
        },
    };

    Ok(Json(response))
}

/// Get consumption report (Story 55.4).
#[utoipa::path(
    get,
    path = "/api/v1/reports/consumption",
    params(ConsumptionReportQuery),
    responses(
        (status = 200, description = "Consumption report", body = ConsumptionReportResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Reports"
)]
pub async fn get_consumption_report(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    Query(query): Query<ConsumptionReportQuery>,
) -> Result<Json<ConsumptionReportResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate date range
    validate_date_range(query.from_date, query.to_date)?;

    // Validate utility type if provided
    if let Some(ref utility_type) = query.utility_type {
        let valid_types = ["water", "electricity", "gas", "heating"];
        if !valid_types.contains(&utility_type.to_lowercase().as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_UTILITY_TYPE",
                    "Utility type must be water, electricity, gas, or heating",
                )),
            ));
        }
    }

    // Enforce that the requested organization matches the authenticated tenant
    // (super-admins may query across orgs). Closes a cross-tenant IDOR where a
    // caller could request another org's report by supplying its UUID.
    if !rls.is_super_admin() && query.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "organization_id does not match the authenticated tenant",
            )),
        ));
    }

    // Get building name if building_id is provided
    let building_name = get_building_name(&state, &mut rls, query.building_id).await;
    // RLS lookup complete — clear context and return the connection to the pool.
    rls.release().await;

    // Query consumption data from meter repository
    let consumption_data = state
        .meter_repo
        .get_consumption_report(
            query.organization_id,
            query.building_id,
            query.utility_type.as_deref(),
            query.from_date,
            query.to_date,
        )
        .await
        .unwrap_or_default();

    // Get anomalies if requested
    let anomalies = if query.include_anomalies {
        state
            .meter_repo
            .detect_consumption_anomalies(
                query.organization_id,
                query.building_id,
                query.from_date,
                query.to_date,
            )
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    let response = ConsumptionReportResponse {
        building_id: query.building_id,
        building_name,
        date_range: DateRange {
            from: query.from_date,
            to: query.to_date,
        },
        summary: consumption_data.summary,
        by_utility_type: consumption_data.by_utility_type,
        by_unit: consumption_data.by_unit,
        anomalies,
    };

    Ok(Json(response))
}

/// Export job status response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExportJobStatusResponse {
    pub job_id: Uuid,
    pub status: String,
    pub download_url: Option<String>,
    pub expires_at: Option<String>,
    pub error_message: Option<String>,
    pub progress_percent: Option<i32>,
}

/// Export report response union type for OpenAPI (Story 88.5).
///
/// This enum represents the two possible response types:
/// - Async: Returns job ID for large reports (202 Accepted)
/// - Sync: Returns inline data for small reports (200 OK)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum ExportReportResponseUnion {
    /// Async response with job tracking URL
    Async(ExportReportResponse),
    /// Sync response with inline data
    Sync(SyncExportReportResponse),
}

/// Export report to PDF/Excel/CSV (Story 55.5 + Story 88.5).
///
/// For small reports (< 1000 rows), generates synchronously and returns inline data.
/// For large reports, creates a background job and returns a tracking URL.
///
/// The decision is made based on estimated row count:
/// - Faults: Total fault count for organization
/// - Voting: Number of votes in date range
/// - Occupancy: Number of units
/// - Consumption: Number of meters
#[utoipa::path(
    post,
    path = "/api/v1/reports/export",
    request_body = ExportReportRequest,
    responses(
        (status = 200, description = "Report generated synchronously (small report)", body = SyncExportReportResponse),
        (status = 202, description = "Report export job created (large report)", body = ExportReportResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Reports"
)]
pub async fn export_report(
    State(state): State<AppState>,
    auth: AuthUser,
    mut rls: RlsConnection,
    Json(req): Json<ExportReportRequest>,
) -> Result<(StatusCode, Json<ExportReportResponseUnion>), (StatusCode, Json<ErrorResponse>)> {
    // Validate format
    let format = req.format.to_lowercase();
    if !["pdf", "excel", "csv"].contains(&format.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_FORMAT",
                "Format must be pdf, excel, or csv",
            )),
        ));
    }

    // Validate report type
    let report_type = req.report_type.to_lowercase();
    if !["faults", "voting", "occupancy", "consumption"].contains(&report_type.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_REPORT_TYPE",
                "Report type must be faults, voting, occupancy, or consumption",
            )),
        ));
    }

    // Enforce that the requested organization matches the authenticated tenant
    // (super-admins may export across orgs). Closes a cross-tenant IDOR where a
    // caller could export another org's data by supplying its UUID (#832).
    // Mirrors get_fault_statistics_report and the other report handlers.
    if !rls.is_super_admin() && req.organization_id != rls.tenant_id() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "organization_id does not match the authenticated tenant",
            )),
        ));
    }
    rls.release().await;

    // Story 88.5: Estimate row count to decide sync vs async processing
    let estimated_rows = estimate_report_row_count(
        &state,
        &report_type,
        req.organization_id,
        req.building_id,
        req.from_date,
        req.to_date,
    )
    .await;

    tracing::debug!(
        report_type = %report_type,
        estimated_rows = %estimated_rows,
        threshold = %SYNC_REPORT_ROW_THRESHOLD,
        "Estimating report size for sync/async decision"
    );

    // Story 88.5: For small reports, generate synchronously
    // Note: Currently only CSV format is supported for sync generation
    // PDF and Excel require additional libraries and are always async
    if estimated_rows < SYNC_REPORT_ROW_THRESHOLD && format == "csv" {
        tracing::info!(
            report_type = %report_type,
            format = %format,
            estimated_rows = %estimated_rows,
            organization_id = %req.organization_id,
            "Generating small report synchronously (Story 88.5)"
        );

        // Generate CSV content synchronously
        let csv_content = generate_sync_csv_report(
            &state,
            &report_type,
            req.organization_id,
            req.building_id,
            req.from_date,
            req.to_date,
        )
        .await?;

        // Encode content as base64 for JSON response
        use base64::{engine::general_purpose::STANDARD, Engine};
        let encoded_data = STANDARD.encode(csv_content.as_bytes());

        // Generate filename
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.csv", report_type, timestamp);

        return Ok((
            StatusCode::OK,
            Json(ExportReportResponseUnion::Sync(SyncExportReportResponse {
                data: encoded_data,
                filename,
                content_type: get_content_type_for_format(&format).to_string(),
                format,
            })),
        ));
    }

    // For large reports or non-CSV formats, use async processing
    // Build job payload with all export parameters
    let job_payload = serde_json::json!({
        "report_type": report_type,
        "format": format,
        "organization_id": req.organization_id,
        "building_id": req.building_id,
        "from_date": req.from_date,
        "to_date": req.to_date,
        "params": req.params,
        "requested_by": auth.user_id,
    });

    // Create a unique job ID for tracking
    let job_id = Uuid::new_v4();
    let timestamp = chrono::Utc::now().timestamp();

    // Create background job for report generation
    // The job worker will:
    // 1. Fetch data from appropriate repository
    // 2. Generate file (CSV is simplest, PDF/Excel require additional libs)
    // 3. Upload to S3 and create presigned URL
    let job_result = state
        .operations_repo
        .create_background_job(
            job_id,
            "report_export".to_string(),
            "reports".to_string(),
            job_payload,
            Some(req.organization_id),
            Some(auth.user_id),
        )
        .await;

    match job_result {
        Ok(_) => {
            tracing::info!(
                job_id = %job_id,
                report_type = %report_type,
                format = %format,
                estimated_rows = %estimated_rows,
                organization_id = %req.organization_id,
                "Report export job created for large report"
            );

            // Return accepted status with job tracking URL
            let download_url = format!("/api/v1/reports/export/{}/status", job_id);

            // Job expiration is 24 hours from creation
            let expires_at = (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339();

            Ok((
                StatusCode::ACCEPTED,
                Json(ExportReportResponseUnion::Async(ExportReportResponse {
                    download_url,
                    format,
                    expires_at,
                })),
            ))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to create report export job");

            // Story 88.5: Try synchronous generation as fallback when job creation fails
            // This provides better UX than returning an error for small reports
            if format == "csv" {
                tracing::info!(
                    report_type = %report_type,
                    "Attempting synchronous CSV generation as fallback after job creation failure"
                );

                match generate_sync_csv_report(
                    &state,
                    &report_type,
                    req.organization_id,
                    req.building_id,
                    req.from_date,
                    req.to_date,
                )
                .await
                {
                    Ok(csv_content) => {
                        use base64::{engine::general_purpose::STANDARD, Engine};
                        let encoded_data = STANDARD.encode(csv_content.as_bytes());

                        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                        let filename = format!("{}_{}.csv", report_type, timestamp);

                        tracing::info!(
                            report_type = %report_type,
                            "Synchronous fallback generation succeeded"
                        );

                        return Ok((
                            StatusCode::OK,
                            Json(ExportReportResponseUnion::Sync(SyncExportReportResponse {
                                data: encoded_data,
                                filename,
                                content_type: get_content_type_for_format(&format).to_string(),
                                format,
                            })),
                        ));
                    }
                    Err(sync_err) => {
                        tracing::error!(
                            "Synchronous fallback also failed, returning original error"
                        );
                        return Err(sync_err);
                    }
                }
            }

            // For PDF/Excel, we cannot generate synchronously, return job URL as placeholder
            let download_url = format!(
                "/api/v1/reports/download/{}-{}.{}",
                report_type, timestamp, format
            );

            let expires_at = (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339();

            tracing::warn!(
                download_url = %download_url,
                "Returning fallback download URL - actual report file was not generated"
            );

            Ok((
                StatusCode::ACCEPTED,
                Json(ExportReportResponseUnion::Async(ExportReportResponse {
                    download_url,
                    format,
                    expires_at,
                })),
            ))
        }
    }
}

/// Path parameter for export job ID.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ExportJobPath {
    /// The job ID returned from export request
    pub job_id: Uuid,
}

/// Get export job status (Story 84.1).
///
/// Poll this endpoint to check the status of an export job.
/// When completed, the response includes a download URL.
#[utoipa::path(
    get,
    path = "/api/v1/reports/export/{job_id}/status",
    params(ExportJobPath),
    responses(
        (status = 200, description = "Job status retrieved", body = ExportJobStatusResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Reports"
)]
pub async fn get_export_job_status(
    State(state): State<AppState>,
    _auth: AuthUser,
    mut rls: RlsConnection,
    axum::extract::Path(path): axum::extract::Path<ExportJobPath>,
) -> Result<Json<ExportJobStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get job from repository
    let job = state
        .operations_repo
        .get_background_job(path.job_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, job_id = %path.job_id, "Failed to get export job");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get job status")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("JOB_NOT_FOUND", "Export job not found")),
            )
        })?;

    // Scope the job to the caller's tenant: export jobs record their org_id
    // (set from the export request), so only members of that org — or
    // super-admins — may read the status and download URL. Closes a
    // cross-tenant IDOR (#832). Return 404 to avoid leaking the existence of
    // another org's job.
    if !rls.is_super_admin() && job.org_id != Some(rls.tenant_id()) {
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("JOB_NOT_FOUND", "Export job not found")),
        ));
    }
    rls.release().await;

    // Map job status to response
    let status = match job.status {
        db::models::infrastructure::BackgroundJobStatus::Pending => "pending",
        db::models::infrastructure::BackgroundJobStatus::Scheduled => "scheduled",
        db::models::infrastructure::BackgroundJobStatus::Running => "running",
        db::models::infrastructure::BackgroundJobStatus::Completed => "completed",
        db::models::infrastructure::BackgroundJobStatus::Failed => "failed",
        db::models::infrastructure::BackgroundJobStatus::Retrying => "retrying",
        db::models::infrastructure::BackgroundJobStatus::Cancelled => "cancelled",
        db::models::infrastructure::BackgroundJobStatus::TimedOut => "timed_out",
    };

    // Extract download URL from result if completed
    let download_url = job.result.as_ref().and_then(|r| {
        r.get("download_url")
            .and_then(|v| v.as_str())
            .map(String::from)
    });

    // Calculate progress based on status
    let progress_percent = match job.status {
        db::models::infrastructure::BackgroundJobStatus::Pending => Some(0),
        db::models::infrastructure::BackgroundJobStatus::Running => Some(50),
        db::models::infrastructure::BackgroundJobStatus::Completed => Some(100),
        db::models::infrastructure::BackgroundJobStatus::Failed => None,
        _ => Some(25),
    };

    // Calculate expiration (24 hours from completion)
    let expires_at = job
        .completed_at
        .map(|completed| (completed + chrono::Duration::hours(24)).to_rfc3339());

    Ok(Json(ExportJobStatusResponse {
        job_id: job.id,
        status: status.to_string(),
        download_url,
        expires_at,
        error_message: job.error_message,
        progress_percent,
    }))
}

// ============================================================================
// Epic 81: Report Schedule Management & Execution History
// ============================================================================

/// Pause a report schedule (Story 81.1).
///
/// # Security (closes #646)
///
/// Uses `RlsConnection` so the caller's org membership is re-verified against
/// the database on every request (not JWT claims). The `caller_org_id` is
/// threaded into the repository UPDATE WHERE clause, preventing cross-tenant
/// IDOR: a principal in org B cannot pause a schedule belonging to org A.
/// Manager role or above is required to mutate schedules.
#[utoipa::path(
    put,
    path = "/api/v1/reports/schedules/{id}/pause",
    tag = "reports",
    params(("id" = Uuid, Path, description = "Schedule ID")),
    responses(
        (status = 200, description = "Schedule paused", body = ReportSchedule),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - manager role required", body = ErrorResponse),
        (status = 404, description = "Schedule not found", body = ErrorResponse),
    )
)]
pub async fn pause_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ReportSchedule>, (StatusCode, Json<ErrorResponse>)> {
    // RBAC: only manager-tier roles may mutate report schedules.
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Manager role or above required to modify report schedules",
            )),
        ));
    }
    let caller_org_id = rls.tenant_id();
    rls.release().await;

    state
        .report_schedule_repo
        .pause(id, caller_org_id)
        .await
        .map(Json)
        .map_err(|e| match e {
            common::errors::AppError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "SCHEDULE_NOT_FOUND",
                    "Report schedule not found",
                )),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to pause schedule")),
            ),
        })
}

/// Resume a paused report schedule (Story 81.1).
///
/// # Security (closes #646)
///
/// Uses `RlsConnection` so the caller's org membership is re-verified against
/// the database on every request (not JWT claims). The `caller_org_id` is
/// threaded into the repository UPDATE WHERE clause, preventing cross-tenant
/// IDOR: a principal in org B cannot resume a schedule belonging to org A.
/// Manager role or above is required to mutate schedules.
#[utoipa::path(
    put,
    path = "/api/v1/reports/schedules/{id}/resume",
    tag = "reports",
    params(("id" = Uuid, Path, description = "Schedule ID")),
    responses(
        (status = 200, description = "Schedule resumed", body = ReportSchedule),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - manager role required", body = ErrorResponse),
        (status = 404, description = "Schedule not found", body = ErrorResponse),
    )
)]
pub async fn resume_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ReportSchedule>, (StatusCode, Json<ErrorResponse>)> {
    // RBAC: only manager-tier roles may mutate report schedules.
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Manager role or above required to modify report schedules",
            )),
        ));
    }
    let caller_org_id = rls.tenant_id();
    rls.release().await;

    state
        .report_schedule_repo
        .resume(id, caller_org_id)
        .await
        .map(Json)
        .map_err(|e| match e {
            common::errors::AppError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "SCHEDULE_NOT_FOUND",
                    "Report schedule not found",
                )),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to resume schedule")),
            ),
        })
}

/// Query parameters for listing execution history (Story 81.2).
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListExecutionsParams {
    /// Filter by execution status (pending, running, completed, failed, cancelled, skipped).
    pub status: Option<String>,
    /// Filter executions started on or after this timestamp (RFC 3339).
    pub date_from: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter executions started on or before this timestamp (RFC 3339).
    pub date_to: Option<chrono::DateTime<chrono::Utc>>,
    /// Maximum number of executions to return (1–100, default 20).
    #[serde(default = "default_execution_limit")]
    pub limit: i64,
    /// Zero-based offset for pagination.
    #[serde(default)]
    pub offset: i64,
}

fn default_execution_limit() -> i64 {
    20
}

/// Valid execution status values for query filtering.
const VALID_EXECUTION_STATUSES: &[&str] = &[
    "pending",
    "running",
    "completed",
    "failed",
    "cancelled",
    "skipped",
];

/// Build a per-execution download URL when the execution has a file ready.
///
/// The URL points to `GET /api/v1/reports/executions/{id}/download` which
/// generates a presigned S3 URL. We only populate the field when `file_key`
/// is set (i.e. the execution actually produced a file).
fn execution_download_url(exec: &db::models::report_schedule::ReportExecution) -> Option<String> {
    if exec.file_key.is_some() {
        Some(format!("/api/v1/reports/executions/{}/download", exec.id))
    } else {
        None
    }
}

/// List execution history for a report schedule (Story 81.2).
///
/// Returns a paginated log of all past and current executions for the given
/// schedule, ordered by `started_at` descending (most recent first). Each
/// completed execution that produced a file includes a `download_url` pointing
/// to the presigned file-download endpoint.
///
/// # Security (closes #647)
///
/// Uses `RlsConnection` so the caller's org membership is re-verified against
/// the database. The schedule existence check uses the org-scoped
/// `get_by_id_scoped` query, preventing a principal in org B from listing
/// executions belonging to org A's schedule.
#[utoipa::path(
    get,
    path = "/api/v1/reports/schedules/{id}/executions",
    tag = "reports",
    params(
        ("id" = Uuid, Path, description = "Schedule ID"),
        ListExecutionsParams,
    ),
    responses(
        (status = 200, description = "Paginated execution history", body = ExecutionHistoryResponse),
        (status = 400, description = "Invalid query parameters", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Schedule not found", body = ErrorResponse),
    )
)]
pub async fn list_schedule_executions(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(params): Query<ListExecutionsParams>,
) -> Result<Json<ExecutionHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate status filter
    if let Some(ref s) = params.status {
        if !VALID_EXECUTION_STATUSES.contains(&s.to_lowercase().as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_STATUS",
                    "status must be one of: pending, running, completed, failed, cancelled, skipped",
                )),
            ));
        }
    }

    // Validate limit bounds
    if !(1_i64..=100).contains(&params.limit) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_LIMIT",
                "limit must be between 1 and 100",
            )),
        ));
    }

    // Validate offset
    if params.offset < 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_OFFSET",
                "offset must be non-negative",
            )),
        ));
    }

    // Validate date range when both are provided
    if let (Some(from), Some(to)) = (params.date_from, params.date_to) {
        if from > to {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_DATE_RANGE",
                    "date_from must be before or equal to date_to",
                )),
            ));
        }
    }

    // Capture the caller's org and release the RLS connection before the
    // schedule lookup (which uses the pool directly, not the RLS connection).
    let caller_org_id = rls.tenant_id();
    rls.release().await;

    // Verify the schedule exists AND belongs to the caller's org before
    // querying executions.  This prevents cross-tenant IDOR (#647): a principal
    // in org B cannot enumerate executions for org A's schedule even if they
    // know the UUID.  Returns the same 404 for "not found" and "wrong org".
    let schedule = state
        .report_schedule_repo
        .get_by_id_scoped(id, caller_org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, schedule_id = %id, "Failed to look up report schedule");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to look up schedule")),
            )
        })?;

    if schedule.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "SCHEDULE_NOT_FOUND",
                "Report schedule not found",
            )),
        ));
    }

    let query = ExecutionHistoryQuery {
        schedule_id: id,
        status: params.status.map(|s| s.to_lowercase()),
        date_from: params.date_from,
        date_to: params.date_to,
        limit: params.limit,
        offset: params.offset,
    };

    let mut response = state
        .report_schedule_repo
        .list_executions(query)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, schedule_id = %id, "Failed to list execution history");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list executions")),
            )
        })?;

    // Populate download_url for each execution that produced a file.
    for exec in &mut response.executions {
        let url = execution_download_url(exec);
        exec.download_url = url;
    }

    Ok(Json(response))
}

/// Get a single report execution by ID (Story 81.2).
///
/// # Security (closes #647)
///
/// Uses `RlsConnection` and `get_execution_scoped` which joins through
/// `report_schedules.organization_id` — a principal in org B cannot read
/// executions belonging to org A's schedules.
#[utoipa::path(
    get,
    path = "/api/v1/reports/executions/{id}",
    tag = "reports",
    params(("id" = Uuid, Path, description = "Execution ID")),
    responses(
        (status = 200, description = "Report execution", body = ReportExecution),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Execution not found", body = ErrorResponse),
    )
)]
pub async fn get_execution(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ReportExecution>, (StatusCode, Json<ErrorResponse>)> {
    let caller_org_id = rls.tenant_id();
    rls.release().await;

    state
        .report_schedule_repo
        .get_execution_scoped(id, caller_org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, execution_id = %id, "Failed to get execution");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get execution")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "EXECUTION_NOT_FOUND",
                    "Report execution not found",
                )),
            )
        })
        .map(Json)
}

/// Get presigned download URL for a completed report execution (Story 81.2).
///
/// # Security (closes #647)
///
/// Uses `RlsConnection` and validates the execution belongs to the caller's org
/// (via `get_execution_scoped`) before generating a download URL.  This prevents
/// cross-tenant IDOR where a principal in org B could obtain a file URL for
/// org A's execution by supplying a known execution UUID.
#[utoipa::path(
    get,
    path = "/api/v1/reports/executions/{id}/download",
    tag = "reports",
    params(("id" = Uuid, Path, description = "Execution ID")),
    responses(
        (status = 200, description = "Download URL", body = ExecutionDownloadUrl),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "Execution not found", body = ErrorResponse),
    )
)]
pub async fn get_execution_download_url(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ExecutionDownloadUrl>, (StatusCode, Json<ErrorResponse>)> {
    let caller_org_id = rls.tenant_id();
    rls.release().await;

    // Verify the execution belongs to the caller's org before generating a URL.
    let execution = state
        .report_schedule_repo
        .get_execution_scoped(id, caller_org_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                execution_id = %id,
                org_id = %caller_org_id,
                "Failed to look up execution for download URL"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to get execution")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "EXECUTION_NOT_FOUND",
                    "Report execution not found",
                )),
            )
        })?;

    // Derive MIME type from file extension.
    let file_name = execution
        .file_name
        .as_deref()
        .unwrap_or("report.pdf")
        .to_string();
    let content_type = if file_name.ends_with(".pdf") {
        "application/pdf"
    } else if file_name.ends_with(".xlsx") {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    } else {
        "text/csv"
    };

    let file_key = execution.file_key.ok_or_else(|| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse::new(
                "NO_FILE_YET",
                "Execution does not have a completed file yet",
            )),
        )
    })?;

    let url = format!("/api/v1/reports/files/{}", file_key);
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

    Ok(Json(ExecutionDownloadUrl {
        url,
        expires_at,
        file_name,
        content_type: content_type.to_string(),
    }))
}

/// Retry a failed report execution (Story 81.2).
///
/// # Security (closes #647)
///
/// Uses `RlsConnection` and `retry_execution_scoped` which verifies the
/// execution's parent schedule belongs to the caller's org before performing
/// the status reset.  Manager role or above is required — retrying an execution
/// is a mutating action equivalent to resuming a schedule.
#[utoipa::path(
    post,
    path = "/api/v1/reports/executions/{id}/retry",
    tag = "reports",
    params(("id" = Uuid, Path, description = "Execution ID")),
    responses(
        (status = 200, description = "Execution queued for retry", body = ReportExecution),
        (status = 400, description = "Execution is not in failed state", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - manager role required", body = ErrorResponse),
        (status = 404, description = "Execution not found", body = ErrorResponse),
    )
)]
pub async fn retry_execution(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<ReportExecution>, (StatusCode, Json<ErrorResponse>)> {
    // RBAC: only manager-tier roles may trigger re-execution.
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Manager role or above required to retry report executions",
            )),
        ));
    }
    let caller_org_id = rls.tenant_id();
    rls.release().await;

    state
        .report_schedule_repo
        .retry_execution_scoped(id, caller_org_id)
        .await
        .map(Json)
        .map_err(|e| match e {
            common::errors::AppError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "EXECUTION_NOT_FOUND",
                    "Report execution not found",
                )),
            ),
            common::errors::AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("INVALID_STATE", msg.as_str())),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to retry execution")),
            ),
        })
}

// ============================================================================
// gap-81-1: Create report schedule
// ============================================================================

/// Query parameters for `GET /api/v1/reports/schedules` (issue #2324).
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListSchedulesQuery {
    /// Accepted for client compatibility but **ignored for scoping** — the
    /// organisation is always taken from the authenticated `RlsConnection`
    /// tenant, never a client-supplied value, so a caller cannot list another
    /// org's schedules.
    #[serde(default)]
    #[allow(dead_code)]
    pub organization_id: Option<Uuid>,
    /// Optional filter: only schedules for this (opaque) `report_id`.
    pub report_id: Option<Uuid>,
    /// Optional filter: `true` = active only, `false` = paused/inactive only.
    pub is_active: Option<bool>,
}

/// Response for `GET /api/v1/reports/schedules` (issue #2324).
#[derive(Debug, Serialize, ToSchema)]
pub struct ListSchedulesResponse {
    /// Schedules belonging to the caller's organisation, newest first.
    pub schedules: Vec<ReportSchedule>,
    /// Number of schedules returned.
    pub total: usize,
}

/// List report schedules for the authenticated tenant (issue #2324).
///
/// Backs the ppt-web Schedules tab (`useReportSchedules` → `listSchedules()`),
/// which before this endpoint existed called a route the router only registered
/// as POST — so the list never loaded (405) and a freshly created schedule was
/// invisible. Any authenticated org member may read their org's schedules.
///
/// # Security
///
/// Uses `RlsConnection` so the tenant is re-derived from the database session,
/// and the repository query filters on `organization_id = rls.tenant_id()` —
/// a principal can never see another org's schedules even by guessing IDs.
#[utoipa::path(
    get,
    path = "/api/v1/reports/schedules",
    tag = "reports",
    params(ListSchedulesQuery),
    responses(
        (status = 200, description = "Schedules for the caller's organisation", body = ListSchedulesResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    )
)]
pub async fn list_schedules(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListSchedulesQuery>,
) -> Result<Json<ListSchedulesResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Tenant scoping comes from the authenticated context, not the query string.
    let caller_org_id = rls.tenant_id();
    rls.release().await;

    let schedules = state
        .report_schedule_repo
        .list_by_org(caller_org_id, query.report_id, query.is_active)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                org_id = %caller_org_id,
                "Failed to list report schedules"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to list report schedules",
                )),
            )
        })?;

    let total = schedules.len();
    Ok(Json(ListSchedulesResponse { schedules, total }))
}

/// Request body for `POST /api/v1/reports/schedules`.
///
/// `organization_id` is intentionally absent — it is derived from the
/// authenticated tenant so a caller cannot create a schedule in another org.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateScheduleRequest {
    /// Report definition this schedule generates.
    ///
    /// `report_id` is intentionally **opaque** (issue #2198): the reports this
    /// system generates (fault statistics, occupancy, consumption, …) are
    /// computed on demand from live data and are *not* rows in a
    /// report-definitions table, so there is nothing to validate `report_id`
    /// against for existence or ownership — and `report_schedules.report_id`
    /// carries no foreign key by design. The handler rejects only the nil UUID
    /// as an obvious client bug; any other value is stored verbatim and the
    /// report-type it names is resolved by the scheduler at fire time.
    pub report_id: Uuid,
    /// Human-readable schedule name.
    pub name: String,
    /// Delivery cadence: `daily`, `weekly`, or `monthly`.
    pub frequency: String,
    /// Day of week (0=Sunday … 6=Saturday). Required when `frequency = weekly`.
    pub day_of_week: Option<i32>,
    /// Day of month (1–31). Required when `frequency = monthly`.
    pub day_of_month: Option<i32>,
    /// Delivery time of day in `HH:MM` (24h). Defaults to `08:00`.
    pub time: Option<String>,
    /// IANA timezone for the delivery time. Defaults to `UTC`.
    pub timezone: Option<String>,
    /// Export format: `pdf`, `excel`, or `csv`. Defaults to `pdf`.
    pub format: Option<String>,
    /// Recipient email addresses (max 50).
    #[serde(default)]
    pub recipients: Vec<String>,
    /// Optional 5-field UNIX cron expression (issue #2303, finding 2).
    ///
    /// When provided it is the schedule's cadence source of truth: `next_run_at`
    /// is computed from it via the same cron walker `update_schedule` uses (not
    /// from `frequency`/`time`), and it is persisted so a row is never created
    /// with a cron/legacy split-brain. Omit it to use the legacy
    /// `frequency`/`day_of_week`/`day_of_month`/`time` cadence.
    pub cron_expression: Option<String>,
}

/// Create a new report schedule (gap-81-1).
///
/// Persists a new `report_schedules` row for the authenticated tenant and
/// returns it with `201 Created`. Replaces the previous stub which never
/// persisted the schedule.
///
/// # Security
///
/// Uses `RlsConnection` so the caller's role and tenant are re-verified against
/// the database (not JWT claims). Manager role or above is required, and the
/// new schedule's `organization_id` is taken from `rls.tenant_id()` — never the
/// request body — preventing cross-tenant creation.
#[utoipa::path(
    post,
    path = "/api/v1/reports/schedules",
    tag = "reports",
    request_body = CreateScheduleRequest,
    responses(
        (status = 201, description = "Schedule created", body = ReportSchedule),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - manager role required", body = ErrorResponse),
    )
)]
pub async fn create_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateScheduleRequest>,
) -> Result<(StatusCode, Json<ReportSchedule>), (StatusCode, Json<ErrorResponse>)> {
    // RBAC: only manager-tier roles may create report schedules. Role is derived
    // from the DB-backed `RlsConnection`, NOT from JWT claims.
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Manager role or above required to create report schedules",
            )),
        ));
    }
    // Tenant for the new schedule comes from the authenticated context.
    let caller_org_id = rls.tenant_id();
    rls.release().await;

    // --- Validation --------------------------------------------------------
    // `report_id` is opaque (no report-definitions table / no FK — see the
    // CreateScheduleRequest doc, issue #2198). Reject only the obviously-bogus
    // nil UUID; any other value is persisted as-is.
    if req.report_id.is_nil() {
        return Err(bad_request(
            "INVALID_REPORT_ID",
            "report_id must not be the nil UUID",
        ));
    }

    let name = req.name.trim();
    if name.is_empty() {
        return Err(bad_request("EMPTY_NAME", "Schedule name must not be empty"));
    }

    let frequency = req.frequency.to_lowercase();
    if !VALID_SCHEDULE_FREQUENCIES.contains(&frequency.as_str()) {
        return Err(bad_request(
            "INVALID_FREQUENCY",
            "frequency must be one of: daily, weekly, monthly",
        ));
    }

    // Weekly schedules need a day-of-week; monthly schedules need a day-of-month.
    match frequency.as_str() {
        "weekly" => match req.day_of_week {
            Some(d) if (0..=6).contains(&d) => {}
            _ => {
                return Err(bad_request(
                    "INVALID_DAY_OF_WEEK",
                    "weekly schedules require day_of_week in 0..=6 (0=Sunday)",
                ))
            }
        },
        "monthly" => match req.day_of_month {
            Some(d) if (1..=31).contains(&d) => {}
            _ => {
                return Err(bad_request(
                    "INVALID_DAY_OF_MONTH",
                    "monthly schedules require day_of_month in 1..=31",
                ))
            }
        },
        _ => {}
    }
    // Range-check any provided day fields even when not required by frequency.
    if let Some(d) = req.day_of_week {
        if !(0..=6).contains(&d) {
            return Err(bad_request(
                "INVALID_DAY_OF_WEEK",
                "day_of_week must be in 0..=6 (0=Sunday)",
            ));
        }
    }
    if let Some(d) = req.day_of_month {
        if !(1..=31).contains(&d) {
            return Err(bad_request(
                "INVALID_DAY_OF_MONTH",
                "day_of_month must be in 1..=31",
            ));
        }
    }

    let time = req
        .time
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or(DEFAULT_SCHEDULE_TIME)
        .to_string();
    let Some((time_hour, time_minute)) = parse_time_hhmm(&time) else {
        return Err(bad_request(
            "INVALID_TIME",
            "time must be a 24-hour HH:MM value (e.g. \"08:30\")",
        ));
    };

    let format = req
        .format
        .as_deref()
        .map(str::to_lowercase)
        .unwrap_or_else(|| "pdf".to_string());
    if !VALID_SCHEDULE_FORMATS.contains(&format.as_str()) {
        return Err(bad_request(
            "INVALID_FORMAT",
            "format must be one of: pdf, excel, csv",
        ));
    }

    if req.recipients.len() > 50 {
        return Err(bad_request(
            "TOO_MANY_RECIPIENTS",
            "A schedule may have at most 50 recipients",
        ));
    }
    for email in &req.recipients {
        if !crate::services::auth::AuthService::validate_email(email) {
            return Err(bad_request(
                "INVALID_RECIPIENT_EMAIL",
                format!("Invalid recipient email address: {email}"),
            ));
        }
    }

    let timezone = req
        .timezone
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or("UTC")
        .to_string();
    // The timezone is the basis for the scheduler's delivery-time math, so an
    // unrecognised value is a latent break — reject it at the edge (issue #2198)
    // instead of storing e.g. "Mars/Phobos".
    let Ok(tz) = timezone.parse::<Tz>() else {
        return Err(bad_request(
            "INVALID_TIMEZONE",
            "timezone must be a valid IANA timezone name (e.g. \"Europe/Bratislava\")",
        ));
    };

    // issue #2303 (finding 2): a caller may supply a `cron_expression` at create
    // time. When present it is the cadence source of truth — validate it and
    // compute the first fire from the cron walker (the SAME path edits use) so a
    // row is never created with a cron/legacy split-brain. The legacy
    // frequency/time columns are still stored (they are NOT NULL in the schema)
    // but `compute_next_run_for_schedule` treats cron_expression as authoritative
    // for all later scheduling math.
    let cron_expression = match req.cron_expression.as_deref().map(str::trim) {
        Some(cron) if !cron.is_empty() => {
            if !validate_cron_expression(cron) {
                return Err(bad_request(
                    "INVALID_CRON_EXPRESSION",
                    "cron_expression must be a valid 5-field UNIX cron expression \
                     (e.g. \"0 8 * * 1\")",
                ));
            }
            Some(cron.to_string())
        }
        _ => None,
    };

    // Compute the first fire time now so the row is schedulable immediately;
    // otherwise a due-work query (`WHERE next_run_at <= NOW()`) would never
    // pick up a freshly created schedule (issue #2198). Route through the cron
    // walker when a cron_expression was supplied, else the legacy walker.
    let next_run_at = match cron_expression.as_deref() {
        Some(cron) => compute_next_run_from_cron(Utc::now().with_timezone(&tz), cron),
        None => compute_first_next_run(
            &frequency,
            req.day_of_week,
            req.day_of_month,
            time_hour,
            time_minute,
            tz,
        ),
    };

    // --- Persist -----------------------------------------------------------
    let schedule = state
        .report_schedule_repo
        .create(db::models::report_schedule::NewReportSchedule {
            organization_id: caller_org_id,
            report_id: req.report_id,
            name: name.to_string(),
            frequency,
            day_of_week: req.day_of_week,
            day_of_month: req.day_of_month,
            time,
            timezone,
            format,
            recipients: req.recipients,
            next_run_at,
            cron_expression,
        })
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                org_id = %caller_org_id,
                report_id = %req.report_id,
                "Failed to create report schedule"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create schedule")),
            )
        })?;

    Ok((StatusCode::CREATED, Json(schedule)))
}

/// Small helper to build a `400 Bad Request` error tuple.
fn bad_request(
    code: &'static str,
    message: impl Into<String>,
) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::new(code, message.into())),
    )
}

// ============================================================================
// gap-81-1: Update report schedule (cron_expression, recipients, enabled)
// ============================================================================

/// Request body for `PUT /api/v1/reports/schedules/{id}`.
///
/// All fields are optional — omit a field to leave the current value unchanged.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateScheduleRequest {
    /// Cron expression (5-field UNIX syntax, e.g. `"0 8 * * 1"` = every Monday at 08:00).
    pub cron_expression: Option<String>,
    /// Recipient email addresses (max 50). Replaces the existing list.
    pub recipients: Option<Vec<String>>,
    /// `true` activates the schedule; `false` pauses it.
    pub enabled: Option<bool>,
}

/// Update a report schedule (gap-81-1).
///
/// Partial-update semantics: any field omitted from the request body is left unchanged.
///
/// # Security (closes #614, #624)
///
/// Uses `RlsConnection` (not the deprecated `AuthUser`) so that:
/// - The caller's tenant membership is **re-verified against the database** on
///   every request (defends against stale JWT role claims / leak #10).
/// - The RBAC check (`is_manager()`) is performed against the DB-derived role
///   stored in `RlsConnection`, not the JWT `role` claim.
/// - The caller's `tenant_id` is threaded into the repository UPDATE as
///   `organization_id`, preventing cross-tenant mutation (closes #624).
#[utoipa::path(
    put,
    path = "/api/v1/reports/schedules/{id}",
    tag = "reports",
    params(("id" = Uuid, Path, description = "Schedule ID")),
    request_body = UpdateScheduleRequest,
    responses(
        (status = 200, description = "Updated schedule", body = ReportSchedule),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - manager role required", body = ErrorResponse),
        (status = 404, description = "Schedule not found", body = ErrorResponse),
    )
)]
pub async fn update_schedule(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateScheduleRequest>,
) -> Result<Json<ReportSchedule>, (StatusCode, Json<ErrorResponse>)> {
    // RBAC: only manager-tier roles may mutate report schedules.
    // Role is derived from the DB-backed `RlsConnection`, NOT from JWT claims.
    // This closes #614: a user with a stale JWT claiming a higher role cannot
    // bypass the check because `rls.role()` reflects current DB state.
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Manager role or above required to modify report schedules",
            )),
        ));
    }

    // Capture the caller's tenant_id for cross-tenant WHERE scoping (closes #624).
    let caller_org_id = rls.tenant_id();

    // We no longer need the DB connection for further lookups in this handler —
    // the UPDATE in update_schedule() uses the pool directly.
    rls.release().await;

    // At least one field must be supplied.
    if req.cron_expression.is_none() && req.recipients.is_none() && req.enabled.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "EMPTY_UPDATE",
                "At least one of cron_expression, recipients, or enabled must be provided",
            )),
        ));
    }
    // Validate cron expression when present.
    if let Some(ref cron) = req.cron_expression {
        if !validate_cron_expression(cron) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "INVALID_CRON_EXPRESSION",
                    "cron_expression must be a valid 5-field UNIX cron expression \
                     (e.g. \"0 8 * * 1\")",
                )),
            ));
        }
    }
    // Validate recipient email addresses when present.
    if let Some(ref recipients) = req.recipients {
        if recipients.len() > 50 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "TOO_MANY_RECIPIENTS",
                    "A schedule may have at most 50 recipients",
                )),
            ));
        }
        for email in recipients {
            if !crate::services::auth::AuthService::validate_email(email) {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "INVALID_RECIPIENT_EMAIL",
                        format!("Invalid recipient email address: {email}"),
                    )),
                ));
            }
        }
    }
    // When the cron expression changes, recompute the next fire time from the
    // NEW expression so the schedule fires at the edited cadence instead of the
    // stale time carried over from the previous expression (issue #2242). The
    // cron is evaluated in the schedule's stored timezone, so load the existing
    // row to read it; an unknown/missing row surfaces as the same 404 the
    // UPDATE would return.
    let next_run_at = if let Some(ref cron) = req.cron_expression {
        let existing = state
            .report_schedule_repo
            .get_by_id_scoped(id, caller_org_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    schedule_id = %id,
                    org_id = %caller_org_id,
                    "Failed to load schedule for next_run_at recompute"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DB_ERROR", "Failed to update schedule")),
                )
            })?
            .ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(
                        "SCHEDULE_NOT_FOUND",
                        "Report schedule not found",
                    )),
                )
            })?;
        // A stored timezone always validates (create-time rejects bad ones),
        // but fall back to UTC rather than fail an edit if it somehow doesn't.
        let tz: Tz = existing.timezone.parse().unwrap_or(chrono_tz::UTC);
        compute_next_run_from_cron(Utc::now().with_timezone(&tz), cron)
    } else {
        None
    };

    // Apply updates and persist.
    // The repository enforces `AND organization_id = caller_org_id` in the
    // UPDATE WHERE clause so a cross-tenant attempt silently returns 404.
    let updated = state
        .report_schedule_repo
        .update_schedule(
            id,
            caller_org_id,
            req.cron_expression,
            req.recipients,
            req.enabled,
            next_run_at,
        )
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                schedule_id = %id,
                org_id = %caller_org_id,
                "Failed to update schedule"
            );
            match e {
                common::errors::AppError::NotFound(_) => (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(
                        "SCHEDULE_NOT_FOUND",
                        "Report schedule not found",
                    )),
                ),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("DB_ERROR", "Failed to update schedule")),
                ),
            }
        })?;
    Ok(Json(updated))
}
