//! Reports routes (Epic 55: Advanced Reporting & Analytics).
//!
//! Provides comprehensive reporting endpoints for:
//! - Story 55.1: Fault Statistics Report
//! - Story 55.2: Voting Participation Report
//! - Story 55.3: Occupancy Report
//! - Story 55.4: Consumption Report
//! - Story 55.5: Export Reports to PDF/Excel

use api_core::extractors::AuthUser;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::NaiveDate;
use common::errors::ErrorResponse;
use db::models::{
    ConsumptionAnomaly, ConsumptionSummary, DateRange, FaultStatistics, FaultTrends,
    OccupancySummary, OccupancyTrends, UnitConsumption, UnitOccupancy, UtilityTypeConsumption,
    VoteParticipationDetail, VotingParticipationSummary, YearComparison,
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
}

// ============================================================================
// Helper functions
// ============================================================================

/// Get building name by ID if provided.
/// Note: This helper function is called from report handlers. RLS migration
/// would require passing RlsConnection through all callers.
async fn get_building_name(state: &AppState, building_id: Option<Uuid>) -> Option<String> {
    if let Some(id) = building_id {
        // TODO: Migrate to find_by_id_rls when handlers pass RlsConnection
        #[allow(deprecated)]
        let result = state.building_repo.find_by_id(id).await;
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

    // Get building name if building_id is provided
    let building_name = get_building_name(&state, query.building_id).await;

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

    // Get building name if building_id is provided
    let building_name = get_building_name(&state, query.building_id).await;

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

    // Get building name if building_id is provided
    let building_name = get_building_name(&state, query.building_id).await;

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

    // Get building name if building_id is provided
    let building_name = get_building_name(&state, query.building_id).await;

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
