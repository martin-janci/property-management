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
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, TimeZone, Utc};
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
        // gap-81-1: Create a new report schedule
        .route("/schedules", axum::routing::post(create_schedule))
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
// Helper functions
// ============================================================================

/// Get building name by ID if provided.
///
/// RLS-scoped: callers pass their request's `RlsConnection` so the lookup is
/// constrained to buildings the authenticated tenant can see (prevents leaking
/// a building name across tenants via an arbitrary id).
async fn get_building_name(
    state: &AppState,
    rls: &mut RlsConnection,
    building_id: Option<Uuid>,
) -> Option<String> {
    if let Some(id) = building_id {
        let result = state
            .building_repo
            .find_by_id_rls(&mut **rls.conn(), id)
            .await;
        result.ok().flatten().and_then(|b| b.name)
    } else {
        None
    }
}

/// Estimate row count for a report based on type and parameters (Story 88.5).
///
/// Returns an estimated row count to decide between sync and async processing.
/// For sync generation, we want reports with less than SYNC_REPORT_ROW_THRESHOLD rows.
async fn estimate_report_row_count(
    state: &AppState,
    report_type: &str,
    organization_id: Uuid,
    building_id: Option<Uuid>,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
) -> i64 {
    // Default date range for estimation
    let to = to_date.unwrap_or_else(|| chrono::Utc::now().date_naive());
    let from = from_date.unwrap_or_else(|| to - chrono::Duration::days(DEFAULT_FAULT_REPORT_DAYS));

    match report_type {
        "faults" => {
            // Estimate based on fault count for the organization/building
            state
                .fault_repo
                .count_by_organization(organization_id)
                .await
                .unwrap_or(0)
        }
        "voting" => {
            // Get count of votes in the date range
            state
                .vote_repo
                .get_participation_report(organization_id, building_id, from, to)
                .await
                .map(|v| v.len() as i64)
                .unwrap_or(0)
        }
        "occupancy" => {
            // Estimate based on unit count (one row per unit in the report)
            state
                .unit_repo
                .count_by_organization(organization_id)
                .await
                .unwrap_or(0)
        }
        "consumption" => {
            // Estimate based on meter count (consumption data per meter)
            // This is a rough estimate; actual rows depend on date range granularity
            state
                .meter_repo
                .list_meters_for_building(building_id.unwrap_or(Uuid::nil()), 1, 0)
                .await
                .map(|r| r.total)
                .unwrap_or(0)
        }
        _ => 0,
    }
}

/// Generate CSV content for a report synchronously (Story 88.5).
///
/// Generates report data in CSV format for immediate download.
/// Only called for small reports (below SYNC_REPORT_ROW_THRESHOLD).
async fn generate_sync_csv_report(
    state: &AppState,
    report_type: &str,
    organization_id: Uuid,
    building_id: Option<Uuid>,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let to = to_date.unwrap_or_else(|| chrono::Utc::now().date_naive());
    let from = from_date.unwrap_or_else(|| to - chrono::Duration::days(DEFAULT_FAULT_REPORT_DAYS));

    match report_type {
        "faults" => {
            // Get fault statistics
            let stats = state
                .fault_repo
                .get_statistics(organization_id, building_id)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to get fault statistics for sync export");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            "REPORT_GENERATION_FAILED",
                            "Failed to generate fault report",
                        )),
                    )
                })?;

            // Generate CSV
            let mut csv = String::from(
                "Category,Status,Priority,Count,Average Resolution Hours,Average Rating\n",
            );
            csv.push_str(&format!(
                "Total,All,All,{},{},{}\n",
                stats.total_count,
                stats
                    .average_resolution_time_hours
                    .map_or("N/A".to_string(), |h| format!("{:.1}", h)),
                stats
                    .average_rating
                    .map_or("N/A".to_string(), |r| format!("{:.1}", r))
            ));
            csv.push_str(&format!("Open,All,All,{},N/A,N/A\n", stats.open_count));
            csv.push_str(&format!("Closed,All,All,{},N/A,N/A\n", stats.closed_count));

            // Add by status breakdown
            for status in &stats.by_status {
                csv.push_str(&format!(
                    "By Status,{},All,{},N/A,N/A\n",
                    status.status, status.count
                ));
            }

            // Add by category breakdown
            for cat in &stats.by_category {
                csv.push_str(&format!(
                    "By Category,All,{},{},N/A,N/A\n",
                    cat.category, cat.count
                ));
            }

            // Add by priority breakdown
            for priority in &stats.by_priority {
                csv.push_str(&format!(
                    "By Priority,All,{},{},N/A,N/A\n",
                    priority.priority, priority.count
                ));
            }

            Ok(csv)
        }
        "voting" => {
            // Get voting participation data
            let votes = state
                .vote_repo
                .get_participation_report(organization_id, building_id, from, to)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to get voting participation for sync export");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new("REPORT_GENERATION_FAILED", "Failed to generate voting report")),
                    )
                })?;

            // Generate CSV
            let mut csv = String::from("Vote ID,Title,Status,Start Date,End Date,Eligible Count,Response Count,Participation Rate,Quorum Required,Quorum Reached\n");
            for v in &votes {
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{},{:.1}%,{},{}\n",
                    v.vote_id,
                    v.title.replace(',', ";"),
                    v.status,
                    v.start_at.as_deref().unwrap_or("N/A"),
                    v.end_at,
                    v.eligible_count,
                    v.response_count,
                    v.participation_rate,
                    v.quorum_required
                        .map_or("N/A".to_string(), |q| format!("{:.0}%", q)),
                    if v.quorum_reached { "Yes" } else { "No" }
                ));
            }

            Ok(csv)
        }
        "occupancy" => {
            // Get occupancy data
            let data = state
                .person_month_repo
                .get_occupancy_report(organization_id, building_id, from, to)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to get occupancy data for sync export");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            "REPORT_GENERATION_FAILED",
                            "Failed to generate occupancy report",
                        )),
                    )
                })?;

            // Generate CSV
            let mut csv = String::from("Metric,Value\n");
            csv.push_str(&format!("Total Units,{}\n", data.summary.total_units));
            csv.push_str(&format!("Occupied Units,{}\n", data.summary.occupied_units));
            csv.push_str(&format!("Vacant Units,{}\n", data.summary.vacant_units));
            csv.push_str(&format!(
                "Occupancy Rate,{:.1}%\n",
                data.summary.occupancy_rate
            ));
            csv.push_str(&format!(
                "Total Person Months,{}\n",
                data.summary.total_person_months
            ));

            // Add monthly breakdown
            csv.push_str("\nMonth,Person Months\n");
            for monthly in &data.monthly_totals {
                csv.push_str(&format!(
                    "{}/{},{}\n",
                    monthly.year, monthly.month, monthly.count
                ));
            }

            Ok(csv)
        }
        "consumption" => {
            // Get consumption data
            let data = state
                .meter_repo
                .get_consumption_report(organization_id, building_id, None, from, to)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to get consumption data for sync export");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::new(
                            "REPORT_GENERATION_FAILED",
                            "Failed to generate consumption report",
                        )),
                    )
                })?;

            // Generate CSV
            let mut csv = String::from("Metric,Value\n");
            csv.push_str(&format!(
                "Total Consumption,{}\n",
                data.summary.total_consumption
            ));
            csv.push_str(&format!("Total Cost,{}\n", data.summary.total_cost));
            csv.push_str(&format!("Meter Count,{}\n", data.summary.meter_count));
            csv.push_str(&format!(
                "Average Consumption Per Unit,{}\n",
                data.summary.average_consumption_per_unit
            ));

            Ok(csv)
        }
        _ => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "INVALID_REPORT_TYPE",
                "Unsupported report type",
            )),
        )),
    }
}

/// Get MIME type for export format.
fn get_content_type_for_format(format: &str) -> &'static str {
    match format {
        "pdf" => "application/pdf",
        "excel" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "csv" => "text/csv",
        _ => "application/octet-stream",
    }
}

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

/// Allowed schedule frequencies (matches the `report_schedules.frequency`
/// CHECK constraint in migration 00162).
const VALID_SCHEDULE_FREQUENCIES: &[&str] = &["daily", "weekly", "monthly"];

/// Allowed export formats for a schedule.
const VALID_SCHEDULE_FORMATS: &[&str] = &["pdf", "excel", "csv"];

/// Default delivery time (HH:MM) when the caller omits `time`.
const DEFAULT_SCHEDULE_TIME: &str = "08:00";

/// Parse an `HH:MM` 24-hour time-of-day string into `(hour, minute)`.
///
/// Rejects empty components and non-numeric input; enforces 24h/60m bounds.
fn parse_time_hhmm(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.split_once(':')?;
    if h.is_empty() || m.is_empty() {
        return None;
    }
    let hour = h.parse::<u32>().ok()?;
    let minute = m.parse::<u32>().ok()?;
    (hour <= 23 && minute <= 59).then_some((hour, minute))
}

/// Validate an `HH:MM` 24-hour time-of-day string.
fn validate_time_hhmm(s: &str) -> bool {
    parse_time_hhmm(s).is_some()
}

/// Compute the first `next_run_at` (UTC) for a freshly created schedule
/// (issue #2198).
///
/// Walks forward from "now" in the schedule's `tz` to the first calendar day
/// that satisfies the cadence, then anchors the `HH:MM` delivery time on that
/// day and converts back to UTC. Returns `None` only if no matching day is
/// found within the search horizon (should not happen for valid input).
///
/// Cadence rules (mirrors the create-schedule validation):
/// - `daily`   — today if the time is still in the future, else tomorrow.
/// - `weekly`  — the next date whose weekday matches `day_of_week`
///   (0=Sunday … 6=Saturday), today included only if the time has not passed.
/// - `monthly` — the next date whose day-of-month equals `day_of_month`. Months
///   that lack that day (e.g. day 31 in April, or 29–31 in February) are
///   skipped to the next month that has it, rather than clamped.
///
/// DST handling mirrors `services::quiet_hours::local_to_utc`: ambiguous local
/// times (fall-back) take the earliest instant; a skipped local time
/// (spring-forward gap) nudges forward one hour.
fn compute_first_next_run(
    frequency: &str,
    day_of_week: Option<i32>,
    day_of_month: Option<i32>,
    hour: u32,
    minute: u32,
    tz: Tz,
) -> Option<DateTime<Utc>> {
    compute_next_run_after(
        Utc::now().with_timezone(&tz),
        frequency,
        day_of_week,
        day_of_month,
        hour,
        minute,
    )
}

/// Compute the first cadence-matching run strictly after `now` (UTC result).
///
/// This is the injectable core of [`compute_first_next_run`]: it takes the
/// reference instant explicitly (in the schedule's timezone) instead of reading
/// the wall clock, which makes the DST edge cases (spring-forward gap /
/// fall-back ambiguity) unit-testable and provides the reusable primitive a
/// post-fire `next_run_at` advance would call with `now = last fire` (issue
/// #2242, finding 1). The schedule timezone is taken from `now.timezone()`.
///
/// Cadence and DST semantics are identical to [`compute_first_next_run`].
fn compute_next_run_after(
    now: DateTime<Tz>,
    frequency: &str,
    day_of_week: Option<i32>,
    day_of_month: Option<i32>,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Utc>> {
    let tz = now.timezone();
    let today = now.date_naive();
    let time = NaiveTime::from_hms_opt(hour, minute, 0)?;

    // Horizon of ~2 years bounds the monthly day-of-month edge cases.
    for add in 0..=800u64 {
        let date = today.checked_add_days(chrono::Days::new(add))?;
        let matches = match frequency {
            "daily" => true,
            "weekly" => {
                day_of_week.is_some_and(|d| date.weekday().num_days_from_sunday() as i32 == d)
            }
            "monthly" => day_of_month.is_some_and(|d| date.day() as i32 == d),
            _ => false,
        };
        if !matches {
            continue;
        }

        let naive = date.and_time(time);
        let candidate = match tz.from_local_datetime(&naive).earliest() {
            Some(dt) => dt,
            // Spring-forward gap: nudge forward an hour; worst case anchor in UTC.
            None => tz
                .from_local_datetime(&(naive + chrono::Duration::hours(1)))
                .earliest()
                .unwrap_or_else(|| Utc.from_utc_datetime(&naive).with_timezone(&tz)),
        };

        if candidate > now {
            return Some(candidate.with_timezone(&Utc));
        }
    }
    None
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

    // Compute the first fire time now so the row is schedulable immediately;
    // otherwise a due-work query (`WHERE next_run_at <= NOW()`) would never
    // pick up a freshly created schedule (issue #2198).
    let next_run_at = compute_first_next_run(
        &frequency,
        req.day_of_week,
        req.day_of_month,
        time_hour,
        time_minute,
        tz,
    );

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

/// Validate a 5-field UNIX cron expression (minute hour dom month dow).
fn validate_cron_expression(expr: &str) -> bool {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return false;
    }
    fn valid_field(field: &str, min: u32, max: u32) -> bool {
        for part in field.split(',') {
            let (base, step) = if let Some((b, s)) = part.split_once('/') {
                (b, Some(s))
            } else {
                (part, None)
            };
            if let Some(s) = step {
                if s.parse::<u32>().map_or(true, |v| v == 0) {
                    return false;
                }
            }
            if base != "*" {
                if let Some((lo, hi)) = base.split_once('-') {
                    match (lo.parse::<u32>(), hi.parse::<u32>()) {
                        (Ok(l), Ok(h)) if l >= min && h <= max && l <= h => {}
                        _ => return false,
                    }
                } else {
                    match base.parse::<u32>() {
                        Ok(v) if (min..=max).contains(&v) => {}
                        _ => return false,
                    }
                }
            }
        }
        true
    }
    // minute(0-59) hour(0-23) dom(1-31) month(1-12) dow(0-7)
    valid_field(fields[0], 0, 59)
        && valid_field(fields[1], 0, 23)
        && valid_field(fields[2], 1, 31)
        && valid_field(fields[3], 1, 12)
        && valid_field(fields[4], 0, 7)
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

#[cfg(test)]
mod tests {
    use super::{
        compute_first_next_run, compute_next_run_after, parse_time_hhmm, validate_cron_expression,
        validate_time_hhmm,
    };
    use chrono::{Datelike, LocalResult, TimeZone, Timelike, Utc};
    use chrono_tz::Tz;

    // --- parse_time_hhmm (issue #2198) ---

    #[test]
    fn parse_time_returns_components() {
        assert_eq!(parse_time_hhmm("08:30"), Some((8, 30)));
        assert_eq!(parse_time_hhmm("0:0"), Some((0, 0)));
        assert_eq!(parse_time_hhmm("23:59"), Some((23, 59)));
        assert_eq!(parse_time_hhmm("24:00"), None);
        assert_eq!(parse_time_hhmm("aa:bb"), None);
    }

    // --- compute_first_next_run (issue #2198) ---

    #[test]
    fn next_run_daily_is_in_the_future() {
        let tz: Tz = "Europe/Bratislava".parse().unwrap();
        let next = compute_first_next_run("daily", None, None, 8, 30, tz)
            .expect("daily schedule must yield a next run");
        assert!(
            next > chrono::Utc::now(),
            "next_run_at must be in the future"
        );
        // Anchored at 08:30 local time.
        let local = next.with_timezone(&tz);
        assert_eq!((local.hour(), local.minute()), (8, 30));
    }

    #[test]
    fn next_run_weekly_lands_on_requested_weekday() {
        let tz: Tz = "UTC".parse().unwrap();
        // day_of_week = 3 (Wednesday, 0=Sunday).
        let next = compute_first_next_run("weekly", Some(3), None, 9, 0, tz)
            .expect("weekly schedule must yield a next run");
        let local = next.with_timezone(&tz);
        assert_eq!(
            local.weekday().num_days_from_sunday(),
            3,
            "weekly next run must fall on the requested weekday"
        );
        assert!(next > chrono::Utc::now());
    }

    #[test]
    fn next_run_monthly_lands_on_requested_day() {
        let tz: Tz = "UTC".parse().unwrap();
        // day_of_month = 15.
        let next = compute_first_next_run("monthly", None, Some(15), 6, 0, tz)
            .expect("monthly schedule must yield a next run");
        let local = next.with_timezone(&tz);
        assert_eq!(local.day(), 15, "monthly next run must fall on day 15");
        assert!(next > chrono::Utc::now());
    }

    #[test]
    fn next_run_monthly_day_31_skips_short_months() {
        let tz: Tz = "UTC".parse().unwrap();
        // Day 31 only exists in some months; the walker must still find one.
        let next = compute_first_next_run("monthly", None, Some(31), 0, 0, tz)
            .expect("day-31 schedule must resolve to a 31-day month");
        assert_eq!(next.with_timezone(&tz).day(), 31);
        assert!(next > chrono::Utc::now());
    }

    // --- compute_next_run_after: DST edge cases (issue #2242, finding 2) ---
    //
    // These exercise the branches in the walker that plain daily/weekly/monthly
    // tests can't reach, because the real-clock `compute_first_next_run` never
    // lands its search on a specific DST-transition date. `compute_next_run_after`
    // injects the reference instant so we can anchor a delivery time inside the
    // spring-forward gap / fall-back ambiguous hour.
    //
    // Europe/Bratislava (CET/CEST) transitions, verified against chrono-tz:
    //  - Spring forward: 2027-03-28, 02:00 -> 03:00 (02:00..03:00 does not exist).
    //  - Fall back:      2026-10-25, 03:00 -> 02:00 (02:00..03:00 occurs twice).

    #[test]
    fn next_run_spring_forward_gap_nudges_into_valid_instant() {
        let tz: Tz = "Europe/Bratislava".parse().unwrap();

        // Precondition: 02:30 on the transition date is a genuine gap (no local
        // instant). If chrono-tz ever changes its rules this asserts loudly
        // rather than silently testing the wrong branch.
        assert!(
            matches!(
                tz.with_ymd_and_hms(2027, 3, 28, 2, 30, 0),
                LocalResult::None
            ),
            "02:30 on 2027-03-28 must be a spring-forward gap"
        );

        // Reference instant: 00:30 local on the transition day, before the gap.
        let now = tz.with_ymd_and_hms(2027, 3, 28, 0, 30, 0).unwrap();

        // Daily schedule at 02:30 -> today's candidate falls in the gap.
        let next = compute_next_run_after(now, "daily", None, None, 2, 30)
            .expect("gap-anchored schedule must still yield a next run");

        // Must not panic, must be strictly after `now`, and must be a real
        // instant nudged forward out of the gap (03:30 local == 01:30 UTC).
        assert!(
            next > now.with_timezone(&Utc),
            "next run must be in the future"
        );
        let local = next.with_timezone(&tz);
        assert_eq!(
            (local.year(), local.month(), local.day()),
            (2027, 3, 28),
            "next run stays on the transition date"
        );
        assert_eq!(
            (local.hour(), local.minute()),
            (3, 30),
            "gap time 02:30 nudges forward one hour to 03:30 CEST"
        );
        assert_eq!(
            next,
            Utc.with_ymd_and_hms(2027, 3, 28, 1, 30, 0).unwrap(),
            "03:30 CEST is 01:30 UTC"
        );
    }

    #[test]
    fn next_run_fall_back_ambiguous_picks_earliest() {
        let tz: Tz = "Europe/Bratislava".parse().unwrap();

        // Precondition: 02:30 on the fall-back date is ambiguous (two instants).
        assert!(
            matches!(
                tz.with_ymd_and_hms(2026, 10, 25, 2, 30, 0),
                LocalResult::Ambiguous(_, _)
            ),
            "02:30 on 2026-10-25 must be a fall-back ambiguous time"
        );

        // Reference instant: 00:30 local, before the ambiguous hour.
        let now = tz.with_ymd_and_hms(2026, 10, 25, 0, 30, 0).unwrap();

        let next = compute_next_run_after(now, "daily", None, None, 2, 30)
            .expect("ambiguous-anchored schedule must still yield a next run");

        assert!(
            next > now.with_timezone(&Utc),
            "next run must be in the future"
        );
        let local = next.with_timezone(&tz);
        assert_eq!(
            (local.hour(), local.minute()),
            (2, 30),
            "fall-back time is kept at 02:30 local"
        );
        // The earliest of the two 02:30 instants is the CEST one (UTC+2),
        // i.e. 00:30 UTC — not the later CET (UTC+1) 01:30 UTC.
        assert_eq!(
            next,
            Utc.with_ymd_and_hms(2026, 10, 25, 0, 30, 0).unwrap(),
            "the earliest (CEST, +02:00) instant must be chosen"
        );
    }

    // --- validate_time_hhmm (gap-81-1: create schedule) ---

    #[test]
    fn time_valid_values() {
        for t in ["00:00", "08:00", "08:30", "23:59", "9:05", "0:0"] {
            assert!(validate_time_hhmm(t), "{t:?} should be a valid HH:MM time");
        }
    }

    #[test]
    fn time_invalid_values() {
        for t in [
            "", ":", "24:00", "08:60", "08", "08:", ":30", "aa:bb", "-1:00", "12:-5", "8:0:0",
        ] {
            assert!(!validate_time_hhmm(t), "{t:?} should be rejected");
        }
    }

    // --- valid expressions ---

    #[test]
    fn valid_every_minute() {
        assert!(validate_cron_expression("* * * * *"));
    }

    #[test]
    fn valid_specific_time() {
        // Every Monday at 08:00
        assert!(validate_cron_expression("0 8 * * 1"));
    }

    #[test]
    fn valid_ranges_and_lists() {
        assert!(validate_cron_expression("0,30 9-17 1-31 1-12 0-7"));
    }

    #[test]
    fn valid_step_syntax() {
        // Every 5 minutes
        assert!(validate_cron_expression("*/5 * * * *"));
    }

    #[test]
    fn valid_boundary_values() {
        assert!(validate_cron_expression("59 23 31 12 7"));
    }

    // --- invalid expressions ---

    #[test]
    fn invalid_too_few_fields() {
        assert!(!validate_cron_expression("* * * *"));
    }

    #[test]
    fn invalid_too_many_fields() {
        assert!(!validate_cron_expression("* * * * * *"));
    }

    #[test]
    fn invalid_empty_string() {
        assert!(!validate_cron_expression(""));
    }

    #[test]
    fn invalid_minute_out_of_range() {
        // minute must be 0-59
        assert!(!validate_cron_expression("60 * * * *"));
    }

    #[test]
    fn invalid_hour_out_of_range() {
        // hour must be 0-23
        assert!(!validate_cron_expression("* 24 * * *"));
    }

    #[test]
    fn invalid_dom_zero() {
        // day-of-month must be 1-31
        assert!(!validate_cron_expression("* * 0 * *"));
    }

    #[test]
    fn invalid_month_too_large() {
        // month must be 1-12
        assert!(!validate_cron_expression("* * * 13 *"));
    }

    #[test]
    fn invalid_dow_out_of_range() {
        // day-of-week must be 0-7
        assert!(!validate_cron_expression("* * * * 8"));
    }

    #[test]
    fn invalid_step_zero() {
        // step of 0 is meaningless
        assert!(!validate_cron_expression("*/0 * * * *"));
    }

    #[test]
    fn invalid_non_numeric_field() {
        assert!(!validate_cron_expression("foo * * * *"));
    }

    #[test]
    fn invalid_reversed_range() {
        // lo > hi in a range
        assert!(!validate_cron_expression("* * 31-1 * *"));
    }

    // --- macros / shorthand (gap-81-1: PR #531 review) ---
    //
    // The validator implements *only* the 5-field UNIX syntax. The `@reboot`,
    // `@daily`, `@hourly`, etc. macros are NOT supported: each macro is a single
    // whitespace-delimited token, so the field-count guard rejects them. These
    // tests pin that contract so a future refactor can't silently start
    // accepting (or mis-parsing) macros without updating the handler docs.

    #[test]
    fn invalid_reboot_macro_rejected() {
        // `@reboot` is one token -> field count is 1, not 5 -> rejected.
        assert!(!validate_cron_expression("@reboot"));
    }

    #[test]
    fn invalid_named_macros_rejected() {
        for macro_expr in [
            "@daily",
            "@hourly",
            "@weekly",
            "@monthly",
            "@yearly",
            "@annually",
        ] {
            assert!(
                !validate_cron_expression(macro_expr),
                "macro {macro_expr:?} must be rejected (macros are unsupported)"
            );
        }
    }

    #[test]
    fn invalid_macro_padded_to_five_tokens_rejected() {
        // Even if a macro string happens to contain five whitespace-separated
        // tokens, each token must still be a valid numeric/`*` field.
        assert!(!validate_cron_expression(
            "@hourly @daily @weekly @monthly @yearly"
        ));
    }

    // --- additional edge cases ---

    #[test]
    fn valid_range_with_step() {
        // A step applied to an explicit range: minutes 5..15 every 3.
        assert!(validate_cron_expression("5-15/3 * * * *"));
    }

    #[test]
    fn valid_step_on_wildcard_dow() {
        // `*/2` over the day-of-week wildcard.
        assert!(validate_cron_expression("* * * * */2"));
    }

    #[test]
    fn valid_explicit_value_list() {
        // Comma list of discrete minute values.
        assert!(validate_cron_expression("0,15,30,45 * * * *"));
    }

    #[test]
    fn valid_full_range_every_field() {
        // The inclusive bounds of every field expressed as ranges.
        assert!(validate_cron_expression("0-59 0-23 1-31 1-12 0-7"));
    }

    #[test]
    fn valid_tab_separated_fields() {
        // `split_whitespace` collapses any run of whitespace, so tabs work too.
        assert!(validate_cron_expression("0\t8\t*\t*\t1"));
    }

    #[test]
    fn invalid_whitespace_only() {
        // Only whitespace -> zero fields -> rejected.
        assert!(!validate_cron_expression("     "));
    }

    #[test]
    fn invalid_empty_list_element() {
        // A trailing/empty comma element (`1,,`) is not a valid sub-field.
        assert!(!validate_cron_expression("1,, * * * *"));
    }

    #[test]
    fn invalid_dangling_range_bound() {
        // A range missing its upper bound (`1-`) fails to parse.
        assert!(!validate_cron_expression("1- * * * *"));
    }

    #[test]
    fn invalid_negative_value() {
        // Leading `-` makes the field parse as a malformed range.
        assert!(!validate_cron_expression("-1 * * * *"));
    }

    #[test]
    fn invalid_non_numeric_step() {
        // Step value must be a positive integer.
        assert!(!validate_cron_expression("*/x * * * *"));
    }
}
