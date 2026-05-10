//! Investor Portal & ROI Reporting routes (Epic 139).
//! Provides investment tracking, ROI calculations, and investor dashboard features.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::investor_portal::{
    CreateCapitalCall, CreateDashboardMetrics, CreateDistribution, CreateInvestmentPortfolio,
    CreateInvestorPortfolioProperty, CreateInvestorProfile, CreateInvestorReport,
    CreateRoiCalculation, RoiCalculationQuery, UpdateCapitalCall, UpdateDistribution,
    UpdateInvestmentPortfolio, UpdateInvestorPortfolioProperty, UpdateInvestorProfile,
};
use uuid::Uuid;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

fn get_org_id(auth: &AuthUser) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    auth.tenant_id.ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse::bad_request("No organization context")),
    ))
}

pub fn router() -> Router<AppState> {
    Router::new()
        // Investor profiles
        .route("/investors", get(list_investors).post(create_investor))
        .route(
            "/investors/{investor_id}",
            get(get_investor)
                .put(update_investor)
                .delete(delete_investor),
        )
        .route(
            "/investors/{investor_id}/summary",
            get(get_investor_summary),
        )
        // Portfolios
        .route("/portfolios", get(list_portfolios).post(create_portfolio))
        .route(
            "/portfolios/{portfolio_id}",
            get(get_portfolio)
                .put(update_portfolio)
                .delete(delete_portfolio),
        )
        .route(
            "/investors/{investor_id}/portfolios",
            get(list_investor_portfolios),
        )
        // Portfolio properties
        .route(
            "/portfolios/{portfolio_id}/properties",
            get(list_portfolio_properties).post(add_portfolio_property),
        )
        .route(
            "/portfolios/{portfolio_id}/properties/{property_id}",
            put(update_portfolio_property).delete(remove_portfolio_property),
        )
        // ROI calculations
        .route(
            "/roi",
            get(list_roi_calculations).post(create_roi_calculation),
        )
        .route("/portfolios/{portfolio_id}/roi/latest", get(get_latest_roi))
        // Distributions
        .route("/distributions", post(create_distribution))
        .route(
            "/investors/{investor_id}/distributions",
            get(list_investor_distributions),
        )
        .route("/distributions/{distribution_id}", put(update_distribution))
        // Reports
        .route("/reports", post(create_report))
        .route(
            "/investors/{investor_id}/reports",
            get(list_investor_reports),
        )
        .route("/reports/{report_id}", get(get_report))
        // Capital calls
        .route("/capital-calls", post(create_capital_call))
        .route(
            "/investors/{investor_id}/capital-calls",
            get(list_investor_capital_calls),
        )
        .route("/capital-calls/{call_id}", put(update_capital_call))
        // Dashboard
        .route("/dashboard/{investor_id}", get(get_investor_dashboard))
        .route(
            "/dashboard/{investor_id}/metrics",
            post(upsert_dashboard_metrics),
        )
}

// =============================================================================
// INVESTOR PROFILE HANDLERS
// =============================================================================

async fn list_investors(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<db::models::investor_portal::InvestorProfile>>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .list_investor_profiles(org_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn create_investor(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateInvestorProfile>,
) -> ApiResult<Json<db::models::investor_portal::InvestorProfile>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .create_investor_profile(org_id, &data, auth.user_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn get_investor(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(investor_id): Path<Uuid>,
) -> ApiResult<Json<db::models::investor_portal::InvestorProfile>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .get_investor_profile(investor_id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Investor not found")),
        ))
}

async fn update_investor(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(investor_id): Path<Uuid>,
    Json(data): Json<UpdateInvestorProfile>,
) -> ApiResult<Json<db::models::investor_portal::InvestorProfile>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .update_investor_profile(investor_id, org_id, &data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Investor not found")),
        ))
}

async fn delete_investor(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(investor_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = get_org_id(&auth)?;

    let deleted = state
        .investor_portal_repo
        .delete_investor_profile(investor_id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Investor not found")),
        ))
    }
}

async fn get_investor_summary(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(investor_id): Path<Uuid>,
) -> ApiResult<Json<db::models::investor_portal::InvestorSummary>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .get_investor_summary(org_id, investor_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Investor not found")),
        ))
}

// =============================================================================
// PORTFOLIO HANDLERS
// =============================================================================

async fn list_portfolios(
    State(state): State<AppState>,
    auth: AuthUser,
) -> ApiResult<Json<Vec<db::models::investor_portal::InvestmentPortfolio>>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .list_portfolios(org_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn create_portfolio(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateInvestmentPortfolio>,
) -> ApiResult<Json<db::models::investor_portal::InvestmentPortfolio>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .create_portfolio(org_id, &data, auth.user_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn get_portfolio(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
) -> ApiResult<Json<db::models::investor_portal::InvestmentPortfolio>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .get_portfolio(portfolio_id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Portfolio not found")),
        ))
}

async fn update_portfolio(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Json(data): Json<UpdateInvestmentPortfolio>,
) -> ApiResult<Json<db::models::investor_portal::InvestmentPortfolio>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .update_portfolio(portfolio_id, org_id, &data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Portfolio not found")),
        ))
}

async fn delete_portfolio(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let org_id = get_org_id(&auth)?;

    let deleted = state
        .investor_portal_repo
        .delete_portfolio(portfolio_id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Portfolio not found")),
        ))
    }
}

async fn list_investor_portfolios(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(investor_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::investor_portal::InvestmentPortfolio>>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .list_portfolios_by_investor(org_id, investor_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

// =============================================================================
// PORTFOLIO PROPERTY HANDLERS
// =============================================================================

async fn list_portfolio_properties(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::investor_portal::InvestorPortfolioProperty>>> {
    state
        .investor_portal_repo
        .list_portfolio_properties(portfolio_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn add_portfolio_property(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
    Json(data): Json<CreateInvestorPortfolioProperty>,
) -> ApiResult<Json<db::models::investor_portal::InvestorPortfolioProperty>> {
    state
        .investor_portal_repo
        .add_portfolio_property(portfolio_id, &data)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn update_portfolio_property(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((_portfolio_id, property_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<UpdateInvestorPortfolioProperty>,
) -> ApiResult<Json<db::models::investor_portal::InvestorPortfolioProperty>> {
    state
        .investor_portal_repo
        .update_portfolio_property(property_id, &data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Property not found")),
        ))
}

async fn remove_portfolio_property(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((_portfolio_id, property_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    let deleted = state
        .investor_portal_repo
        .remove_portfolio_property(property_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Property not found")),
        ))
    }
}

// =============================================================================
// ROI CALCULATION HANDLERS
// =============================================================================

async fn list_roi_calculations(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<RoiCalculationQuery>,
) -> ApiResult<Json<Vec<db::models::investor_portal::RoiCalculation>>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .list_roi_calculations(org_id, &query)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn create_roi_calculation(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateRoiCalculation>,
) -> ApiResult<Json<db::models::investor_portal::RoiCalculation>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .create_roi_calculation(org_id, &data)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn get_latest_roi(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(portfolio_id): Path<Uuid>,
) -> ApiResult<Json<db::models::investor_portal::RoiCalculation>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .get_latest_roi(org_id, portfolio_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("No ROI calculation found")),
        ))
}

// =============================================================================
// DISTRIBUTION HANDLERS
// =============================================================================

async fn create_distribution(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateDistribution>,
) -> ApiResult<Json<db::models::investor_portal::InvestorDistribution>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .create_distribution(org_id, &data, auth.user_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn list_investor_distributions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(investor_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::investor_portal::InvestorDistribution>>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .list_distributions_by_investor(org_id, investor_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn update_distribution(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(distribution_id): Path<Uuid>,
    Json(data): Json<UpdateDistribution>,
) -> ApiResult<Json<db::models::investor_portal::InvestorDistribution>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .update_distribution(distribution_id, org_id, &data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Distribution not found")),
        ))
}

// =============================================================================
// REPORT HANDLERS
// =============================================================================

async fn create_report(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateInvestorReport>,
) -> ApiResult<Json<db::models::investor_portal::InvestorReport>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .create_report(org_id, &data, auth.user_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn list_investor_reports(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(investor_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::investor_portal::InvestorReport>>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .list_reports_by_investor(org_id, investor_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn get_report(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(report_id): Path<Uuid>,
) -> ApiResult<Json<db::models::investor_portal::InvestorReport>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .get_report(report_id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Report not found")),
        ))
}

// =============================================================================
// CAPITAL CALL HANDLERS
// =============================================================================

async fn create_capital_call(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(data): Json<CreateCapitalCall>,
) -> ApiResult<Json<db::models::investor_portal::CapitalCall>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .create_capital_call(org_id, &data, auth.user_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn list_investor_capital_calls(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(investor_id): Path<Uuid>,
) -> ApiResult<Json<Vec<db::models::investor_portal::CapitalCall>>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .list_capital_calls_by_investor(org_id, investor_id)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}

async fn update_capital_call(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(call_id): Path<Uuid>,
    Json(data): Json<UpdateCapitalCall>,
) -> ApiResult<Json<db::models::investor_portal::CapitalCall>> {
    let org_id = get_org_id(&auth)?;

    state
        .investor_portal_repo
        .update_capital_call(call_id, org_id, &data)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Capital call not found")),
        ))
}

// =============================================================================
// DASHBOARD HANDLERS
// =============================================================================

async fn get_investor_dashboard(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(investor_id): Path<Uuid>,
) -> ApiResult<Json<db::models::investor_portal::InvestorPortalDashboard>> {
    let org_id = get_org_id(&auth)?;

    // Get investor profile
    let investor = state
        .investor_portal_repo
        .get_investor_profile(investor_id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Investor not found")),
        ))?;

    // Get portfolios
    let portfolios = state
        .investor_portal_repo
        .list_portfolios_by_investor(org_id, investor_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    // Get recent distributions
    let distributions = state
        .investor_portal_repo
        .list_distributions_by_investor(org_id, investor_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    // Get pending capital calls
    let pending_calls = state
        .investor_portal_repo
        .get_pending_capital_calls_count(org_id, investor_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    // Calculate totals
    let total_invested: rust_decimal::Decimal =
        portfolios.iter().map(|p| p.initial_investment).sum();
    let total_value: rust_decimal::Decimal =
        portfolios.iter().filter_map(|p| p.current_value).sum();
    let total_distributions: rust_decimal::Decimal =
        distributions.iter().filter_map(|d| d.net_amount).sum();

    // Get metrics if available
    let metrics = state
        .investor_portal_repo
        .get_latest_dashboard_metrics(org_id, investor_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    // Build portfolio summaries
    let portfolio_summaries: Vec<db::models::investor_portal::InvestorPortfolioSummary> =
        portfolios
            .iter()
            .map(|p| db::models::investor_portal::InvestorPortfolioSummary {
                id: p.id,
                name: p.name.clone(),
                status: p.status.clone(),
                initial_investment: p.initial_investment,
                current_value: p.current_value,
                irr: p.irr,
                property_count: 0, // Would need a separate query
            })
            .collect();

    // Count properties
    let mut property_count = 0;
    for portfolio in &portfolios {
        let props = state
            .investor_portal_repo
            .list_portfolio_properties(portfolio.id)
            .await
            .unwrap_or_default();
        property_count += props.len();
    }

    let dashboard = db::models::investor_portal::InvestorPortalDashboard {
        investor_id,
        investor_name: investor.display_name,
        total_invested,
        total_current_value: total_value,
        total_distributions,
        overall_irr: metrics.as_ref().and_then(|m| m.irr),
        equity_multiple: metrics.as_ref().and_then(|m| m.equity_multiple),
        ytd_return: metrics.as_ref().and_then(|m| m.ytd_return),
        portfolio_count: portfolios.len() as i32,
        property_count: property_count as i32,
        pending_capital_calls: pending_calls as i32,
        recent_distributions: distributions.into_iter().take(5).collect(),
        portfolio_summaries,
    };

    Ok(Json(dashboard))
}

async fn upsert_dashboard_metrics(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(investor_id): Path<Uuid>,
    Json(mut data): Json<CreateDashboardMetrics>,
) -> ApiResult<Json<db::models::investor_portal::InvestorDashboardMetrics>> {
    let org_id = get_org_id(&auth)?;
    data.investor_id = investor_id;

    state
        .investor_portal_repo
        .upsert_dashboard_metrics(org_id, &data)
        .await
        .map(Json)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })
}
