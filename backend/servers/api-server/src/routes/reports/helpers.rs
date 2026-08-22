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
use db::models::VoteParticipationDetail;
use uuid::Uuid;

use crate::routes::admin::audit::sanitize_csv_cell;
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

/// Render a single voting-participation row for the CSV export.
///
/// The vote `title` is the only user-authored, free-form string in the row
/// (every other column is a UUID, a system enum, a date, or a number), so it
/// is routed through the shared [`sanitize_csv_cell`] guard to neutralize
/// spreadsheet formula injection: a title beginning with `=`, `+`, `-`, or
/// `@` is prefixed with `'` so Excel/Sheets treat it as text rather than a
/// formula. The shared guard also collapses any embedded `\r`/`\n` to a space
/// so a crafted title cannot inject a fabricated *record* into this hand-rolled
/// export (which terminates rows with a raw `\n` and does not use the `csv`
/// crate's quoting — see #2822). Commas are then replaced with `;` so the cell
/// cannot spill into adjacent columns. Extracted into its own function so the
/// neutralization is unit testable without a database.
fn voting_csv_row(v: &VoteParticipationDetail) -> String {
    format!(
        "{},{},{},{},{},{},{},{:.1}%,{},{}\n",
        v.vote_id,
        sanitize_csv_cell(&v.title).replace(',', ";"),
        v.status,
        v.start_at.as_deref().unwrap_or("N/A"),
        v.end_at,
        v.eligible_count,
        v.response_count,
        v.participation_rate,
        v.quorum_required
            .map_or("N/A".to_string(), |q| format!("{:.0}%", q)),
        if v.quorum_reached { "Yes" } else { "No" }
    )
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
                csv.push_str(&voting_csv_row(v));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vote(title: &str) -> VoteParticipationDetail {
        VoteParticipationDetail {
            vote_id: Uuid::nil(),
            title: title.to_string(),
            status: "closed".to_string(),
            start_at: Some("2026-01-01".to_string()),
            end_at: "2026-01-31".to_string(),
            eligible_count: 10,
            response_count: 8,
            participation_rate: 80.0,
            quorum_required: Some(50),
            quorum_reached: true,
        }
    }

    /// The title cell is the 2nd comma-separated field of the row.
    fn title_cell(row: &str) -> String {
        row.trim_end_matches('\n')
            .split(',')
            .nth(1)
            .expect("row has a title cell")
            .to_string()
    }

    /// Regression: a user-authored vote title that begins with a spreadsheet
    /// formula trigger must be neutralized in the exported CSV (leading `'`
    /// prefix) instead of being written verbatim, closing the CSV/formula
    /// injection vector that bypassed the repo's `sanitize_csv_cell`.
    #[test]
    fn voting_csv_row_neutralizes_formula_trigger_titles() {
        for trigger in ["=cmd|'/c calc'!A1", "+1+1", "-2+3", "@SUM(A1)"] {
            let row = voting_csv_row(&sample_vote(trigger));
            let cell = title_cell(&row);
            assert!(
                cell.starts_with('\''),
                "title {trigger:?} must be prefixed with a quote in cell {cell:?}"
            );
            // The dangerous payload must not appear as a leading formula.
            assert!(
                !cell.starts_with(trigger.chars().next().unwrap()),
                "cell {cell:?} still begins with the formula trigger"
            );
        }
    }

    /// A plain title is written through unchanged (aside from comma escaping).
    #[test]
    fn voting_csv_row_leaves_plain_titles_untouched() {
        let row = voting_csv_row(&sample_vote("Annual Budget Vote"));
        assert_eq!(title_cell(&row), "Annual Budget Vote");
    }

    /// Commas in a title are escaped to `;` so the cell cannot spill into
    /// adjacent CSV columns.
    #[test]
    fn voting_csv_row_escapes_commas_in_title() {
        let row = voting_csv_row(&sample_vote("Roof, Facade, Windows"));
        assert_eq!(title_cell(&row), "Roof; Facade; Windows");
    }

    /// Regression (#2822): a title carrying an embedded CR/LF must not inject a
    /// fabricated record. The only newline in the rendered row is the trailing
    /// terminator, the title cell contains no raw CR/LF, and it does not spill
    /// into adjacent columns.
    #[test]
    fn voting_csv_row_neutralizes_crlf_in_title() {
        let row = voting_csv_row(&sample_vote("Roof\r\nInjected,row"));

        // Exactly one newline — the row terminator at the very end.
        assert_eq!(
            row.matches('\n').count(),
            1,
            "crafted CR/LF must not add extra records: {row:?}"
        );
        assert!(row.ends_with('\n'));
        assert!(!row.trim_end_matches('\n').contains('\r'));

        // The row still has exactly 10 comma-separated fields (no spill).
        assert_eq!(row.trim_end_matches('\n').split(',').count(), 10);

        // The title cell has the separators neutralized to spaces and the
        // comma escaped to `;`.
        assert_eq!(title_cell(&row), "Roof  Injected;row");
    }
}
