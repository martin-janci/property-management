//! IoT dashboard handler (Story 14.3).

use super::shared::db_error;
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(super) async fn get_dashboard(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<DashboardQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let out = state
        .sensor_repo
        .get_dashboard(rls.conn(), org_id, query.building_id)
        .await
        .map(|dashboard| Json(serde_json::json!(dashboard)))
        .map_err(|e| db_error("Failed to get dashboard", e));
    rls.release().await;
    out
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DashboardQuery {
    pub building_id: Option<Uuid>,
}
