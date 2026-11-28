//! Helper functions for the reports routes (Epic 55 / Story 88.5).
//!
//! Row-count estimation (sync vs async export) and the RLS-scoped
//! building-name lookup. Report rendering lives in
//! `services::report_generator`.

use api_core::extractors::RlsConnection;
use chrono::NaiveDate;
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
        let org_id = rls.tenant_id();
        let is_super_admin = rls.is_super_admin();
        let result = state
            .building_repo
            .find_by_id_rls(&mut **rls.conn(), id)
            .await;
        // Explicit org filter: `buildings` RLS is not FORCEd, so the owner
        // role would otherwise see (and leak the name of) any org's building.
        result
            .ok()
            .flatten()
            .filter(|b| is_super_admin || b.organization_id == org_id)
            .and_then(|b| b.name)
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
