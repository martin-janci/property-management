//! Multi-Currency & Cross-Border Support routes (Epic 145).
//! Provides currency configuration, exchange rate management,
//! cross-currency transactions, cross-border lease management, and reporting.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use chrono::NaiveDate;
use common::errors::ErrorResponse;
use db::models::multi_currency::{
    CreateCrossBorderLease, CreateCurrencyConfig, CreateExchangeRate,
    CreateMultiCurrencyTransaction, CreatePropertyCurrencyConfig, CreateReportConfig,
    CrossBorderLeaseQuery, ExchangeRateQuery, GenerateReportRequest, OverrideExchangeRate,
    SupportedCurrency, TransactionQuery, UpdateCrossBorderLease, UpdateCurrencyConfig,
    UpdatePropertyCurrencyConfig, UpdateTransactionRate,
};
use serde::Deserialize;
use serde_json::json;
use utoipa::IntoParams;
use uuid::Uuid;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

/// Helper to serialize a value to JSON.
fn to_json_value<T: serde::Serialize>(value: T) -> ApiResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&format!(
                "Failed to serialize: {}",
                e
            ))),
        )
    })
}

pub fn router() -> Router<AppState> {
    Router::new()
        // Currency Configuration
        .route("/config", get(get_currency_config))
        .route("/config", post(create_or_update_currency_config))
        .route("/config", put(update_currency_config))
        // Property Currency Configuration
        .route("/properties", get(list_property_currency_configs))
        .route("/properties", post(create_property_currency_config))
        .route(
            "/properties/{building_id}",
            get(get_property_currency_config),
        )
        .route(
            "/properties/{building_id}",
            put(update_property_currency_config),
        )
        // Exchange Rates
        .route("/rates", get(list_exchange_rates))
        .route("/rates", post(create_exchange_rate))
        .route("/rates/latest", get(get_latest_exchange_rate))
        .route("/rates/override", post(override_exchange_rate))
        .route("/rates/fetch", post(fetch_exchange_rates))
        // Cross-Currency Transactions
        .route("/transactions", get(list_transactions))
        .route("/transactions", post(create_transaction))
        .route("/transactions/{id}", get(get_transaction))
        .route("/transactions/{id}/rate", put(update_transaction_rate))
        // Cross-Border Leases
        .route("/cross-border", get(list_cross_border_leases))
        .route("/cross-border", post(create_cross_border_lease))
        .route("/cross-border/{lease_id}", get(get_cross_border_lease))
        .route("/cross-border/{lease_id}", put(update_cross_border_lease))
        .route(
            "/cross-border/compliance/{country}",
            get(get_compliance_requirements),
        )
        // Reports
        .route("/reports/configs", get(list_report_configs))
        .route("/reports/configs", post(create_report_config))
        .route("/reports/generate", post(generate_report))
        .route("/reports/snapshots", get(list_report_snapshots))
        .route("/reports/exposure", get(get_currency_exposure))
        // Dashboard & Statistics
        .route("/dashboard", get(get_dashboard))
        .route("/statistics", get(get_statistics))
}

// =============================================================================
// STORY 145.1: CURRENCY CONFIGURATION
// =============================================================================

/// Get organization currency configuration
async fn get_currency_config(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .get_or_create_currency_config(org_id)
        .await
    {
        Ok(config) => Ok(Json(to_json_value(config)?)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Create or update organization currency configuration
async fn create_or_update_currency_config(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateCurrencyConfig>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .upsert_currency_config(org_id, user.user_id, req)
        .await
    {
        Ok(config) => Ok(Json(to_json_value(config)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// Update organization currency configuration
async fn update_currency_config(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<UpdateCurrencyConfig>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .update_currency_config(org_id, req)
        .await
    {
        Ok(Some(config)) => Ok(Json(to_json_value(config)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Currency config not found")),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// List property currency configurations
async fn list_property_currency_configs(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .list_property_currency_configs(org_id)
        .await
    {
        Ok(configs) => Ok(Json(json!({ "property_configs": configs }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Create property currency configuration
async fn create_property_currency_config(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreatePropertyCurrencyConfig>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .create_property_currency_config(org_id, req)
        .await
    {
        Ok(config) => Ok(Json(to_json_value(config)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// Get property currency configuration
async fn get_property_currency_config(
    State(s): State<AppState>,
    _user: AuthUser,
    Path(building_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    match s
        .multi_currency_repo
        .get_property_currency_config(building_id)
        .await
    {
        Ok(Some(config)) => Ok(Json(to_json_value(config)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found(
                "Property currency config not found",
            )),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Update property currency configuration
async fn update_property_currency_config(
    State(s): State<AppState>,
    user: AuthUser,
    Path(building_id): Path<Uuid>,
    Json(req): Json<UpdatePropertyCurrencyConfig>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .update_property_currency_config(building_id, org_id, req)
        .await
    {
        Ok(Some(config)) => Ok(Json(to_json_value(config)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found(
                "Property currency config not found",
            )),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

// =============================================================================
// STORY 145.2: EXCHANGE RATE MANAGEMENT
// =============================================================================

#[derive(Debug, Deserialize, IntoParams)]
pub struct LatestRateQuery {
    pub from_currency: SupportedCurrency,
    pub to_currency: SupportedCurrency,
}

/// List exchange rates with optional filtering
async fn list_exchange_rates(
    State(s): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ExchangeRateQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    match s.multi_currency_repo.list_exchange_rates(query).await {
        Ok(rates) => Ok(Json(json!({ "exchange_rates": rates }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Get latest exchange rate for a currency pair
async fn get_latest_exchange_rate(
    State(s): State<AppState>,
    _user: AuthUser,
    Query(query): Query<LatestRateQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    match s
        .multi_currency_repo
        .get_latest_exchange_rate(query.from_currency, query.to_currency)
        .await
    {
        Ok(Some(rate)) => Ok(Json(to_json_value(rate)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Exchange rate not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Create exchange rate
async fn create_exchange_rate(
    State(s): State<AppState>,
    _user: AuthUser,
    Json(req): Json<CreateExchangeRate>,
) -> ApiResult<Json<serde_json::Value>> {
    match s.multi_currency_repo.create_exchange_rate(req).await {
        Ok(rate) => Ok(Json(to_json_value(rate)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// Override exchange rate manually
async fn override_exchange_rate(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<OverrideExchangeRate>,
) -> ApiResult<Json<serde_json::Value>> {
    match s
        .multi_currency_repo
        .override_exchange_rate(user.user_id, req)
        .await
    {
        Ok(rate) => Ok(Json(to_json_value(rate)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// Trigger exchange rate fetch from external source
async fn fetch_exchange_rates(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Get organization config to determine source
    let config = s
        .multi_currency_repo
        .get_or_create_currency_config(org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    // In a real implementation, this would call external APIs (ECB, XE)
    // For now, we just log the attempt
    match s
        .multi_currency_repo
        .log_rate_fetch(Some(org_id), config.exchange_rate_source, true, 0, None)
        .await
    {
        Ok(log) => {
            // Update last rate update timestamp
            let _ = s.multi_currency_repo.update_last_rate_update(org_id).await;
            Ok(Json(json!({
                "success": true,
                "message": "Exchange rate fetch initiated",
                "log": to_json_value(log)?
            })))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// STORY 145.3: CROSS-CURRENCY TRANSACTIONS
// =============================================================================

/// List multi-currency transactions
async fn list_transactions(
    State(s): State<AppState>,
    user: AuthUser,
    Query(query): Query<TransactionQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.multi_currency_repo.list_transactions(org_id, query).await {
        Ok(transactions) => Ok(Json(json!({ "transactions": transactions }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Create multi-currency transaction
async fn create_transaction(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateMultiCurrencyTransaction>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Get organization base currency
    let config = s
        .multi_currency_repo
        .get_or_create_currency_config(org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    match s
        .multi_currency_repo
        .create_transaction(org_id, config.base_currency, req)
        .await
    {
        Ok(tx) => Ok(Json(to_json_value(tx)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// Get multi-currency transaction
async fn get_transaction(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.multi_currency_repo.get_transaction(id, org_id).await {
        Ok(Some(tx)) => Ok(Json(to_json_value(tx)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Transaction not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Update transaction exchange rate
async fn update_transaction_rate(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTransactionRate>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .update_transaction_rate(id, org_id, user.user_id, req)
        .await
    {
        Ok(Some(tx)) => Ok(Json(to_json_value(tx)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Transaction not found")),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

// =============================================================================
// STORY 145.4: CROSS-BORDER LEASE MANAGEMENT
// =============================================================================

/// List cross-border leases
async fn list_cross_border_leases(
    State(s): State<AppState>,
    user: AuthUser,
    Query(query): Query<CrossBorderLeaseQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .list_cross_border_leases(org_id, query)
        .await
    {
        Ok(leases) => Ok(Json(json!({ "cross_border_leases": leases }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Create cross-border lease
async fn create_cross_border_lease(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateCrossBorderLease>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .create_cross_border_lease(org_id, req)
        .await
    {
        Ok(lease) => Ok(Json(to_json_value(lease)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// Get cross-border lease
async fn get_cross_border_lease(
    State(s): State<AppState>,
    user: AuthUser,
    Path(lease_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .get_cross_border_lease(lease_id, org_id)
        .await
    {
        Ok(Some(lease)) => Ok(Json(to_json_value(lease)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Cross-border lease not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Update cross-border lease
async fn update_cross_border_lease(
    State(s): State<AppState>,
    user: AuthUser,
    Path(lease_id): Path<Uuid>,
    Json(req): Json<UpdateCrossBorderLease>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .update_cross_border_lease(lease_id, org_id, req)
        .await
    {
        Ok(Some(lease)) => Ok(Json(to_json_value(lease)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Cross-border lease not found")),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct CountryParam {
    pub country: db::models::multi_currency::CountryCode,
}

/// Get compliance requirements for a country
async fn get_compliance_requirements(
    State(s): State<AppState>,
    _user: AuthUser,
    Path(country): Path<db::models::multi_currency::CountryCode>,
) -> ApiResult<Json<serde_json::Value>> {
    match s
        .multi_currency_repo
        .get_compliance_requirements(country)
        .await
    {
        Ok(requirements) => Ok(Json(json!({
            "country": country,
            "requirements": requirements
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// STORY 145.5: CONSOLIDATED MULTI-CURRENCY REPORTING
// =============================================================================

/// List report configurations
async fn list_report_configs(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.multi_currency_repo.list_report_configs(org_id).await {
        Ok(configs) => Ok(Json(json!({ "report_configs": configs }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Create report configuration
async fn create_report_config(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateReportConfig>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .create_report_config(org_id, user.user_id, req)
        .await
    {
        Ok(config) => Ok(Json(to_json_value(config)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// Generate report snapshot
async fn generate_report(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<GenerateReportRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .generate_report_snapshot(org_id, user.user_id, req)
        .await
    {
        Ok(snapshot) => Ok(Json(to_json_value(snapshot)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// List report snapshots
async fn list_report_snapshots(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .multi_currency_repo
        .list_report_snapshots(org_id, 50)
        .await
    {
        Ok(snapshots) => Ok(Json(json!({ "report_snapshots": snapshots }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ExposureQuery {
    pub date: Option<NaiveDate>,
}

/// Get currency exposure analysis
async fn get_currency_exposure(
    State(s): State<AppState>,
    user: AuthUser,
    Query(query): Query<ExposureQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    let date = query
        .date
        .unwrap_or_else(|| chrono::Utc::now().date_naive());

    match s
        .multi_currency_repo
        .get_currency_exposure(org_id, date)
        .await
    {
        Ok(exposures) => Ok(Json(json!({
            "date": date,
            "exposures": exposures
        }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// DASHBOARD & STATISTICS
// =============================================================================

/// Get multi-currency dashboard
async fn get_dashboard(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Get currency config
    let config = s
        .multi_currency_repo
        .get_or_create_currency_config(org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    // Get recent transactions
    let transactions = s
        .multi_currency_repo
        .list_transactions(org_id, TransactionQuery::default())
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    // Get statistics
    let stats = s
        .multi_currency_repo
        .get_statistics(org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    Ok(Json(json!({
        "config": config,
        "recent_transactions": transactions.into_iter().take(10).collect::<Vec<_>>(),
        "statistics": stats
    })))
}

/// Get multi-currency statistics
async fn get_statistics(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.multi_currency_repo.get_statistics(org_id).await {
        Ok(stats) => Ok(Json(to_json_value(stats)?)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}
