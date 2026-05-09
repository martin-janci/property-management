//! Property Valuation API routes for Epic 138: Automated Property Valuation Model.
//!
//! Provides REST endpoints for property valuations, AVM models,
//! comparable sales, market data, and valuation reports.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use api_core::extractors::AuthUser;
use db::models::property_valuation::*;

use crate::state::AppState;

/// Query parameters for listing valuations
#[derive(Debug, Deserialize)]
pub struct ListValuationsQuery {
    pub property_id: Option<Uuid>,
    pub status: Option<ValuationStatus>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

/// Query parameters for listing models
#[derive(Debug, Deserialize)]
pub struct ListModelsQuery {
    #[serde(default)]
    pub active_only: bool,
}

/// Query parameters for listing requests
#[derive(Debug, Deserialize)]
pub struct ListRequestsQuery {
    pub status: Option<ValuationStatus>,
    pub assigned_to: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

/// Query parameters for market data
#[derive(Debug, Deserialize)]
pub struct MarketDataQuery {
    pub city: Option<String>,
    pub district: Option<String>,
    pub property_type: Option<String>,
}

/// Query parameters for value history
#[derive(Debug, Deserialize)]
pub struct ValueHistoryQuery {
    #[serde(default = "default_history_limit")]
    pub limit: i64,
}

/// Query parameters for expiring valuations
#[derive(Debug, Deserialize)]
pub struct ExpiringQuery {
    #[serde(default = "default_days")]
    pub days: i32,
}

/// Query parameters for audit logs
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

fn default_history_limit() -> i64 {
    24
}

fn default_days() -> i32 {
    30
}

/// Helper to get organization ID from auth
fn get_org_id(auth: &AuthUser) -> Result<Uuid, (StatusCode, String)> {
    auth.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        "No organization context".to_string(),
    ))
}

/// Create the property valuation router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/dashboard", get(get_dashboard))
        .route("/expiring", get(get_expiring_valuations))
        // Valuation Models
        .route("/models", get(list_models).post(create_model))
        .route(
            "/models/{model_id}",
            get(get_model).put(update_model).delete(delete_model),
        )
        // Valuations
        .route("/", get(list_valuations).post(create_valuation))
        .route(
            "/{valuation_id}",
            get(get_valuation)
                .put(update_valuation)
                .delete(delete_valuation),
        )
        .route("/{valuation_id}/approve", put(approve_valuation))
        // Comparables
        .route(
            "/{valuation_id}/comparables",
            get(list_comparables).post(create_comparable),
        )
        .route(
            "/comparables/{comparable_id}",
            put(update_comparable).delete(delete_comparable),
        )
        // Adjustments
        .route(
            "/comparables/{comparable_id}/adjustments",
            get(list_adjustments).post(create_adjustment),
        )
        .route("/adjustments/{adjustment_id}", delete(delete_adjustment))
        // Market Data
        .route(
            "/market-data",
            get(get_market_data).post(create_market_data),
        )
        .route("/market-data/{market_data_id}", put(update_market_data))
        // Value History
        .route(
            "/properties/{property_id}/history",
            get(get_value_history).post(create_value_history),
        )
        // Valuation Requests
        .route("/requests", get(list_requests).post(create_request))
        .route(
            "/requests/{request_id}",
            get(get_request).put(update_request),
        )
        // Property Features
        .route(
            "/properties/{property_id}/features",
            get(get_features).post(create_features),
        )
        .route("/features/{feature_id}", put(update_features))
        // Reports
        .route(
            "/{valuation_id}/reports",
            get(list_reports).post(create_report),
        )
        .route("/reports/{report_id}", put(update_report))
        .route("/reports/{report_id}/sign", put(sign_report))
        // Audit Logs
        .route("/{valuation_id}/audit-logs", get(get_audit_logs))
}

// ============================================================================
// Dashboard Handlers
// ============================================================================

/// Get valuation dashboard summary
async fn get_dashboard(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ValuationDashboard>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .get_dashboard(org_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get valuations expiring soon
async fn get_expiring_valuations(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ExpiringQuery>,
) -> Result<Json<Vec<PropertyValuation>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .get_expiring_valuations(org_id, query.days)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ============================================================================
// Valuation Model Handlers
// ============================================================================

/// List valuation models
async fn list_models(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListModelsQuery>,
) -> Result<Json<Vec<PropertyValuationModel>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .list_models(org_id, query.active_only)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a valuation model
async fn create_model(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateValuationModel>,
) -> Result<(StatusCode, Json<PropertyValuationModel>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .create_model(org_id, &data, auth.user_id)
        .await
        .map(|model| (StatusCode::CREATED, Json(model)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get a valuation model by ID
async fn get_model(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(model_id): Path<Uuid>,
) -> Result<Json<PropertyValuationModel>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .get_model(org_id, model_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Model not found".to_string()))
}

/// Update a valuation model
async fn update_model(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(model_id): Path<Uuid>,
    Json(data): Json<UpdateValuationModel>,
) -> Result<Json<PropertyValuationModel>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .update_model(org_id, model_id, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Model not found".to_string()))
}

/// Delete a valuation model
async fn delete_model(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(model_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    let deleted = state
        .property_valuation_repo
        .delete_model(org_id, model_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Model not found".to_string()))
    }
}

// ============================================================================
// Valuation Handlers
// ============================================================================

/// List valuations
async fn list_valuations(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListValuationsQuery>,
) -> Result<Json<Vec<PropertyValuation>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .list_valuations(
            org_id,
            query.property_id,
            query.status,
            query.limit,
            query.offset,
        )
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a valuation
async fn create_valuation(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreatePropertyValuation>,
) -> Result<(StatusCode, Json<PropertyValuation>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .create_valuation(org_id, &data, auth.user_id)
        .await
        .map(|v| (StatusCode::CREATED, Json(v)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get a valuation by ID
async fn get_valuation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(valuation_id): Path<Uuid>,
) -> Result<Json<PropertyValuation>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .get_valuation(org_id, valuation_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Valuation not found".to_string()))
}

/// Update a valuation
async fn update_valuation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(valuation_id): Path<Uuid>,
    Json(data): Json<UpdatePropertyValuation>,
) -> Result<Json<PropertyValuation>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .update_valuation(org_id, valuation_id, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Valuation not found".to_string()))
}

/// Delete a valuation
async fn delete_valuation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(valuation_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    let deleted = state
        .property_valuation_repo
        .delete_valuation(org_id, valuation_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Valuation not found".to_string()))
    }
}

/// Approve a valuation
#[derive(Debug, Deserialize)]
pub struct ApproveRequest {
    pub notes: Option<String>,
}

async fn approve_valuation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(valuation_id): Path<Uuid>,
    Json(data): Json<ApproveRequest>,
) -> Result<Json<PropertyValuation>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .approve_valuation(org_id, valuation_id, auth.user_id, data.notes)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Valuation not found".to_string()))
}

// ============================================================================
// Comparable Handlers
// ============================================================================

/// List comparables for a valuation
async fn list_comparables(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(valuation_id): Path<Uuid>,
) -> Result<Json<Vec<ValuationComparable>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .list_comparables(org_id, valuation_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a comparable
async fn create_comparable(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(valuation_id): Path<Uuid>,
    Json(mut data): Json<CreateComparable>,
) -> Result<(StatusCode, Json<ValuationComparable>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;
    data.valuation_id = Some(valuation_id);

    state
        .property_valuation_repo
        .create_comparable(org_id, &data)
        .await
        .map(|c| (StatusCode::CREATED, Json(c)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Update a comparable
async fn update_comparable(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(comparable_id): Path<Uuid>,
    Json(data): Json<UpdateComparable>,
) -> Result<Json<ValuationComparable>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .update_comparable(org_id, comparable_id, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Comparable not found".to_string()))
}

/// Delete a comparable
async fn delete_comparable(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(comparable_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    let deleted = state
        .property_valuation_repo
        .delete_comparable(org_id, comparable_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Comparable not found".to_string()))
    }
}

// ============================================================================
// Adjustment Handlers
// ============================================================================

/// List adjustments for a comparable
async fn list_adjustments(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(comparable_id): Path<Uuid>,
) -> Result<Json<Vec<ComparableAdjustment>>, (StatusCode, String)> {
    state
        .property_valuation_repo
        .list_adjustments(comparable_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create an adjustment
async fn create_adjustment(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(comparable_id): Path<Uuid>,
    Json(mut data): Json<CreateAdjustment>,
) -> Result<(StatusCode, Json<ComparableAdjustment>), (StatusCode, String)> {
    data.comparable_id = comparable_id;

    state
        .property_valuation_repo
        .create_adjustment(&data)
        .await
        .map(|a| (StatusCode::CREATED, Json(a)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Delete an adjustment
async fn delete_adjustment(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(adjustment_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let deleted = state
        .property_valuation_repo
        .delete_adjustment(adjustment_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Adjustment not found".to_string()))
    }
}

// ============================================================================
// Market Data Handlers
// ============================================================================

/// Get market data
async fn get_market_data(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<MarketDataQuery>,
) -> Result<Json<Vec<ValuationMarketData>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .get_market_data(
            org_id,
            query.city.as_deref(),
            query.district.as_deref(),
            query.property_type.as_deref(),
        )
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create market data
async fn create_market_data(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateMarketData>,
) -> Result<(StatusCode, Json<ValuationMarketData>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .create_market_data(org_id, &data)
        .await
        .map(|m| (StatusCode::CREATED, Json(m)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Update market data
async fn update_market_data(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(market_data_id): Path<Uuid>,
    Json(data): Json<UpdateMarketData>,
) -> Result<Json<ValuationMarketData>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .update_market_data(org_id, market_data_id, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Market data not found".to_string()))
}

// ============================================================================
// Value History Handlers
// ============================================================================

/// Get value history for a property
async fn get_value_history(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(property_id): Path<Uuid>,
    Query(query): Query<ValueHistoryQuery>,
) -> Result<Json<Vec<PropertyValueHistory>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .get_value_history(org_id, property_id, query.limit)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create value history entry
async fn create_value_history(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(property_id): Path<Uuid>,
    Json(mut data): Json<CreateValueHistory>,
) -> Result<(StatusCode, Json<PropertyValueHistory>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;
    data.property_id = property_id;

    state
        .property_valuation_repo
        .create_value_history(org_id, &data)
        .await
        .map(|h| (StatusCode::CREATED, Json(h)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ============================================================================
// Valuation Request Handlers
// ============================================================================

/// List valuation requests
async fn list_requests(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListRequestsQuery>,
) -> Result<Json<Vec<ValuationRequest>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .list_requests(
            org_id,
            query.status,
            query.assigned_to,
            query.limit,
            query.offset,
        )
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a valuation request
async fn create_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateValuationRequest>,
) -> Result<(StatusCode, Json<ValuationRequest>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .create_request(org_id, &data, auth.user_id)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Get a valuation request
async fn get_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(request_id): Path<Uuid>,
) -> Result<Json<ValuationRequest>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .get_request(org_id, request_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Request not found".to_string()))
}

/// Update a valuation request
async fn update_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(request_id): Path<Uuid>,
    Json(data): Json<UpdateValuationRequest>,
) -> Result<Json<ValuationRequest>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .update_request(org_id, request_id, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Request not found".to_string()))
}

// ============================================================================
// Property Features Handlers
// ============================================================================

/// Get property features
async fn get_features(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(property_id): Path<Uuid>,
) -> Result<Json<PropertyValuationFeatures>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .get_features(org_id, property_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Features not found".to_string()))
}

/// Create property features
async fn create_features(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(property_id): Path<Uuid>,
    Json(mut data): Json<CreatePropertyFeatures>,
) -> Result<(StatusCode, Json<PropertyValuationFeatures>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;
    data.property_id = property_id;

    state
        .property_valuation_repo
        .create_features(org_id, &data, auth.user_id)
        .await
        .map(|f| (StatusCode::CREATED, Json(f)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Update property features
async fn update_features(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(feature_id): Path<Uuid>,
    Json(data): Json<UpdatePropertyFeatures>,
) -> Result<Json<PropertyValuationFeatures>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .update_features(org_id, feature_id, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Features not found".to_string()))
}

// ============================================================================
// Report Handlers
// ============================================================================

/// List reports for a valuation
async fn list_reports(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(valuation_id): Path<Uuid>,
) -> Result<Json<Vec<ValuationReport>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .list_reports(org_id, valuation_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Create a report
async fn create_report(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(valuation_id): Path<Uuid>,
    Json(mut data): Json<CreateValuationReport>,
) -> Result<(StatusCode, Json<ValuationReport>), (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;
    data.valuation_id = valuation_id;

    state
        .property_valuation_repo
        .create_report(org_id, &data, auth.user_id)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Update a report
async fn update_report(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(report_id): Path<Uuid>,
    Json(data): Json<UpdateValuationReport>,
) -> Result<Json<ValuationReport>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .update_report(org_id, report_id, &data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Report not found".to_string()))
}

/// Sign a report
async fn sign_report(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(report_id): Path<Uuid>,
) -> Result<Json<ValuationReport>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .sign_report(org_id, report_id, auth.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Report not found".to_string()))
}

// ============================================================================
// Audit Log Handlers
// ============================================================================

/// Get audit logs for a valuation
async fn get_audit_logs(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(valuation_id): Path<Uuid>,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<Vec<ValuationAuditLog>>, (StatusCode, String)> {
    let org_id = get_org_id(&auth)?;

    state
        .property_valuation_repo
        .get_audit_logs(org_id, valuation_id, query.limit)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
