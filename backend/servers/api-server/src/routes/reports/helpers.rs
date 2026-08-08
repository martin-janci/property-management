//! Helper functions for the reports routes (Epic 55 / Story 88.5).
//!
//! Extracted from `mod.rs` (behavior-preserving refactor): row-count
//! estimation, synchronous CSV generation, RLS-scoped building-name lookup,
//! and content-type mapping. These are internal to the `reports` module and
//! shared across the report handlers.

use api_core::extractors::RlsConnection;
use axum::{http::StatusCode, Json};
use chrono::NaiveDate;
use common::errors::ErrorResponse;
use uuid::Uuid;

use crate::state::AppState;

/// Get building name by ID if provided.
///
/// RLS-scoped: callers pass their request's `RlsConnection` so the lookup is
/// constrained to buildings the authenticated tenant can see (prevents leaking
/// a building name across tenants via an arbitrary id).
pub(super) async fn get_building_name(
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
pub(super) async fn estimate_report_row_count(
    state: &AppState,
    report_type: &str,
    organization_id: Uuid,
    building_id: Option<Uuid>,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
) -> i64 {
    // Default date range for estimation
    let to = to_date.unwrap_or_else(|| chrono::Utc::now().date_naive());
    let from =
        from_date.unwrap_or_else(|| to - chrono::Duration::days(super::DEFAULT_FAULT_REPORT_DAYS));

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
pub(super) async fn generate_sync_csv_report(
    state: &AppState,
    report_type: &str,
    organization_id: Uuid,
    building_id: Option<Uuid>,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    let to = to_date.unwrap_or_else(|| chrono::Utc::now().date_naive());
    let from =
        from_date.unwrap_or_else(|| to - chrono::Duration::days(super::DEFAULT_FAULT_REPORT_DAYS));

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
pub(super) fn get_content_type_for_format(format: &str) -> &'static str {
    match format {
        "pdf" => "application/pdf",
        "excel" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "csv" => "text/csv",
        _ => "application/octet-stream",
    }
}
