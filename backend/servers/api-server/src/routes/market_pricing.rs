//! Market Pricing & Analytics routes (Epic 132).
//! Dynamic Rent Pricing, Market Data Collection, and Comparative Market Analysis.
//! Story 132.2: AI Pricing Model Integration using LLM for intelligent pricing.

use crate::state::AppState;
use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::market_pricing::{
    AcceptPricingRecommendation, AddCmaProperty, CreateComparativeMarketAnalysis,
    CreateMarketDataPoint, CreateMarketRegion, ExportPricingDataRequest, GenerateStatisticsRequest,
    MarketDataQuery, PricingDashboard, PricingDashboardQuery, RecordPriceChange,
    RejectPricingRecommendation, RequestPricingRecommendation, UpdateComparativeMarketAnalysis,
    UpdateMarketRegion,
};
use integrations::{ChatCompletionRequest, ChatMessage};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;
use std::str::FromStr;
use tracing::{info, warn};
use utoipa::IntoParams;
use uuid::Uuid;

type ApiResult<T> = Result<T, (StatusCode, Json<ErrorResponse>)>;

/// Helper to serialize a value to JSON, returning a 500 error on failure.
fn to_json_value<T: serde::Serialize>(value: T) -> ApiResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&format!(
                "Failed to serialize response: {}",
                e
            ))),
        )
    })
}

pub fn router() -> Router<AppState> {
    Router::new()
        // Market Regions
        .route("/regions", get(list_regions))
        .route("/regions", post(create_region))
        .route("/regions/{id}", get(get_region))
        .route("/regions/{id}", put(update_region))
        .route("/regions/{id}", delete(delete_region))
        // Market Data Points
        .route("/data", get(list_data_points))
        .route("/data", post(add_data_point))
        // Market Statistics
        .route("/statistics/{region_id}", get(get_statistics))
        .route("/statistics/generate", post(generate_statistics))
        // Pricing Recommendations
        .route("/recommendations", get(list_recommendations))
        .route("/recommendations/request", post(request_recommendation))
        .route("/recommendations/{id}", get(get_recommendation))
        .route(
            "/recommendations/{id}/details",
            get(get_recommendation_details),
        )
        .route("/recommendations/{id}/accept", post(accept_recommendation))
        .route("/recommendations/{id}/reject", post(reject_recommendation))
        // Unit Pricing History
        .route("/units/{unit_id}/history", get(get_pricing_history))
        .route("/units/{unit_id}/price", post(record_price_change))
        .route("/units/{unit_id}/current-rent", get(get_current_rent))
        // Comparative Market Analysis
        .route("/cma", get(list_cmas))
        .route("/cma", post(create_cma))
        .route("/cma/{id}", get(get_cma))
        .route("/cma/{id}", put(update_cma))
        .route("/cma/{id}", delete(delete_cma))
        .route("/cma/{id}/details", get(get_cma_details))
        .route("/cma/{id}/properties", get(get_cma_properties))
        .route("/cma/{id}/properties", post(add_cma_property))
        .route(
            "/cma/{cma_id}/properties/{property_id}",
            delete(remove_cma_property),
        )
        .route("/cma/{id}/recalculate", post(recalculate_cma))
        // Comparables lookup
        .route("/comparables", get(get_comparables))
        // Dashboard (Story 132.3)
        .route("/dashboard", get(get_pricing_dashboard))
        .route("/dashboard/export", post(export_pricing_data))
}

// =============================================================================
// MARKET REGIONS
// =============================================================================

async fn list_regions(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.market_pricing_repo.list_regions(org_id).await {
        Ok(regions) => Ok(Json(json!({ "regions": regions }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn create_region(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateMarketRegion>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.market_pricing_repo.create_region(org_id, req).await {
        Ok(region) => Ok(Json(to_json_value(region)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn get_region(
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

    match s.market_pricing_repo.get_region(id, org_id).await {
        Ok(Some(region)) => Ok(Json(to_json_value(region)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Region not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn update_region(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateMarketRegion>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.market_pricing_repo.update_region(id, org_id, req).await {
        Ok(Some(region)) => Ok(Json(to_json_value(region)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Region not found")),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn delete_region(
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

    match s.market_pricing_repo.delete_region(id, org_id).await {
        Ok(true) => Ok(Json(json!({ "deleted": true }))),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Region not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// MARKET DATA POINTS
// =============================================================================

async fn list_data_points(
    State(s): State<AppState>,
    user: AuthUser,
    Query(query): Query<MarketDataQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.market_pricing_repo.list_data_points(org_id, query).await {
        Ok(data_points) => Ok(Json(json!({ "data_points": data_points }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn add_data_point(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateMarketDataPoint>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate region belongs to user's organization
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify region ownership before adding data point
    let region = s
        .market_pricing_repo
        .get_region(req.region_id, org_id)
        .await;
    if matches!(region, Ok(None) | Err(_)) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found(
                "Region not found or not accessible",
            )),
        ));
    }

    match s.market_pricing_repo.add_data_point(req).await {
        Ok(data_point) => Ok(Json(to_json_value(data_point)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

// =============================================================================
// MARKET STATISTICS
// =============================================================================

#[derive(Debug, Deserialize, IntoParams)]
pub struct StatisticsParams {
    pub property_type: Option<String>,
}

async fn get_statistics(
    State(s): State<AppState>,
    user: AuthUser,
    Path(region_id): Path<Uuid>,
    Query(params): Query<StatisticsParams>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate region belongs to user's organization
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify region ownership
    let region = s.market_pricing_repo.get_region(region_id, org_id).await;
    if matches!(region, Ok(None) | Err(_)) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found(
                "Region not found or not accessible",
            )),
        ));
    }

    match s
        .market_pricing_repo
        .get_market_statistics(region_id, params.property_type)
        .await
    {
        Ok(Some(stats)) => Ok(Json(to_json_value(stats)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("No statistics available")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn generate_statistics(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<GenerateStatisticsRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate region belongs to user's organization
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify region ownership
    let region = s
        .market_pricing_repo
        .get_region(req.region_id, org_id)
        .await;
    if matches!(region, Ok(None) | Err(_)) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found(
                "Region not found or not accessible",
            )),
        ));
    }

    match s.market_pricing_repo.generate_statistics(req).await {
        Ok(stats) => Ok(Json(to_json_value(stats)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

// =============================================================================
// PRICING RECOMMENDATIONS
// =============================================================================

async fn list_recommendations(
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
        .market_pricing_repo
        .list_pending_recommendations(org_id)
        .await
    {
        Ok(recs) => Ok(Json(json!({ "recommendations": recs }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Request AI-powered pricing recommendation for a unit.
///
/// Story 132.2: AI Pricing Model Integration
/// This endpoint analyzes unit characteristics, market data, and comparable properties
/// to generate an optimal price recommendation with confidence scoring.
async fn request_recommendation(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<RequestPricingRecommendation>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Check if AI pricing is enabled via feature flag
    let ai_pricing_enabled =
        std::env::var("LLM_PRICING_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true";

    // Fetch unit details to analyze characteristics
    // TODO: Migrate to find_by_id_rls when RlsConnection is added to this handler
    #[allow(deprecated)]
    let unit = s.unit_repo.find_by_id(req.unit_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )
    })?;

    let unit = unit.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Unit not found")),
        )
    })?;

    // Fetch building details for location context
    // TODO: Migrate to find_by_id_rls when RlsConnection is added to this handler
    #[allow(deprecated)]
    let building = s
        .building_repo
        .find_by_id(unit.building_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    let building = building.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Building not found")),
        )
    })?;

    // Try to find a market region for this location
    let regions = s
        .market_pricing_repo
        .list_regions(org_id)
        .await
        .unwrap_or_default();

    let matching_region = regions
        .iter()
        .find(|r| r.city.to_lowercase() == building.city.to_lowercase() && r.is_active);

    // Get market comparables if we have a matching region
    let comparables = if let Some(region) = matching_region {
        let size = unit.size_sqm.unwrap_or(Decimal::new(50, 0));
        s.market_pricing_repo
            .get_market_comparables(region.id, &unit.unit_type, size, 10, None)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    // Calculate comparables count for recommendation
    let comparables_count = comparables.len() as i32;

    // Get market statistics if available
    let market_stats_id = if let Some(region) = matching_region {
        s.market_pricing_repo
            .get_market_statistics(region.id, Some(unit.unit_type.clone()))
            .await
            .ok()
            .flatten()
            .map(|stats| stats.id)
    } else {
        None
    };

    // Use AI pricing if enabled and we have sufficient data
    let (min_price, optimal_price, max_price, confidence, factors) = if ai_pricing_enabled {
        generate_ai_pricing_recommendation(&s, &unit, &building, &comparables, &req.currency).await
    } else {
        // Fallback to basic statistical pricing
        generate_statistical_pricing(&comparables, &unit)
    };

    match s
        .market_pricing_repo
        .create_recommendation(
            req.unit_id,
            min_price,
            optimal_price,
            max_price,
            &req.currency,
            confidence,
            factors,
            comparables_count,
            market_stats_id,
        )
        .await
    {
        Ok(rec) => Ok(Json(to_json_value(rec)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

/// Generate AI-powered pricing recommendation using LLM.
///
/// Story 132.2: This function constructs a prompt with unit characteristics,
/// market data, and comparable properties for the LLM to analyze and suggest
/// optimal pricing with confidence scoring.
async fn generate_ai_pricing_recommendation(
    state: &AppState,
    unit: &db::models::Unit,
    building: &db::models::Building,
    comparables: &[db::models::market_pricing::MarketComparable],
    currency: &str,
) -> (Decimal, Decimal, Decimal, Decimal, serde_json::Value) {
    // Select LLM provider and model
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| match provider.as_str() {
        "openai" => "gpt-4o-mini".to_string(),
        "azure_openai" => "gpt-4o-mini".to_string(),
        _ => "claude-3-5-haiku-20241022".to_string(),
    });

    // Build the pricing analysis prompt
    let system_prompt = r#"You are an expert real estate pricing analyst. Your task is to analyze property characteristics and market data to recommend optimal rental pricing.

You must respond with ONLY a valid JSON object in this exact format, with no additional text:
{
  "min_price": <number>,
  "optimal_price": <number>,
  "max_price": <number>,
  "confidence": <number between 0 and 100>,
  "factors": {
    "location_score": <number -10 to +10>,
    "size_adjustment": <number -10 to +10>,
    "amenities_premium": <number -10 to +10>,
    "market_positioning": <number -10 to +10>,
    "building_quality": <number -10 to +10>
  },
  "reasoning": "<brief explanation of key pricing factors>"
}

Guidelines:
- Prices should be realistic monthly rental amounts
- Confidence reflects data quality (more comparables = higher confidence)
- Factor scores indicate impact direction (+/- from base market rate)
- Consider location, size, floor, amenities, building age
- min_price = competitive rate to ensure occupancy
- optimal_price = balanced rate for best revenue/occupancy
- max_price = premium rate if demand is high"#;

    // Build user prompt with property data
    let size_str = unit
        .size_sqm
        .map(|s| format!("{} sqm", s))
        .unwrap_or_else(|| "Unknown".to_string());

    let rooms_str = unit
        .rooms
        .map(|r| format!("{} rooms", r))
        .unwrap_or_else(|| "Unknown".to_string());

    let comparables_summary = if comparables.is_empty() {
        "No market comparables available. Use general market knowledge.".to_string()
    } else {
        let avg_rent: Decimal = comparables.iter().map(|c| c.monthly_rent).sum::<Decimal>()
            / Decimal::from(comparables.len() as u32);
        let min_rent = comparables
            .iter()
            .map(|c| c.monthly_rent)
            .min()
            .unwrap_or(avg_rent);
        let max_rent = comparables
            .iter()
            .map(|c| c.monthly_rent)
            .max()
            .unwrap_or(avg_rent);

        format!(
            "Market comparables ({} properties):\n- Average rent: {} {}\n- Range: {} - {} {}\n- Comparable details:\n{}",
            comparables.len(),
            avg_rent,
            currency,
            min_rent,
            max_rent,
            currency,
            comparables
                .iter()
                .take(5)
                .map(|c| format!(
                    "  * {} sqm, {}: {} {} (similarity: {}%)",
                    c.size_sqm, c.property_type, c.monthly_rent, currency, c.similarity_score
                ))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };

    let user_prompt = format!(
        r#"Analyze this property and recommend rental pricing in {}:

**Property Details:**
- Type: {}
- Size: {}
- Rooms: {}
- Floor: {}
- Description: {}

**Building Information:**
- Location: {}, {}, {}
- Year Built: {}
- Total Floors: {}
- Amenities: {}

**Market Data:**
{}

Provide your pricing recommendation in the specified JSON format."#,
        currency,
        unit.unit_type,
        size_str,
        rooms_str,
        unit.floor,
        unit.description.as_deref().unwrap_or("N/A"),
        building.street,
        building.city,
        building.country,
        building
            .year_built
            .map(|y| y.to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
        building.total_floors,
        building.amenities,
        comparables_summary
    );

    // Make LLM request
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        },
    ];

    let request = ChatCompletionRequest {
        model,
        messages,
        temperature: Some(0.3), // Lower temperature for more consistent pricing
        max_tokens: Some(500),
    };

    match state.llm_client.chat(&provider, &request).await {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                // Parse the LLM response
                if let Ok(pricing) =
                    serde_json::from_str::<serde_json::Value>(&choice.message.content)
                {
                    let min_price = pricing["min_price"]
                        .as_f64()
                        .map(|v| {
                            Decimal::from_str(&format!("{:.2}", v)).unwrap_or(Decimal::new(800, 0))
                        })
                        .unwrap_or(Decimal::new(800, 0));
                    let optimal_price = pricing["optimal_price"]
                        .as_f64()
                        .map(|v| {
                            Decimal::from_str(&format!("{:.2}", v)).unwrap_or(Decimal::new(950, 0))
                        })
                        .unwrap_or(Decimal::new(950, 0));
                    let max_price = pricing["max_price"]
                        .as_f64()
                        .map(|v| {
                            Decimal::from_str(&format!("{:.2}", v)).unwrap_or(Decimal::new(1100, 0))
                        })
                        .unwrap_or(Decimal::new(1100, 0));
                    let confidence = pricing["confidence"]
                        .as_f64()
                        .map(|v| {
                            Decimal::from_str(&format!("{:.0}", v.clamp(0.0, 100.0)))
                                .unwrap_or(Decimal::new(75, 0))
                        })
                        .unwrap_or(Decimal::new(75, 0));

                    let factors = json!({
                        "ai_generated": true,
                        "provider": provider,
                        "factors": pricing.get("factors").cloned().unwrap_or(json!({})),
                        "reasoning": pricing.get("reasoning").cloned().unwrap_or(json!("AI analysis complete")),
                        "comparables_used": comparables.len()
                    });

                    info!(
                        "AI pricing recommendation generated: optimal={}, confidence={}%",
                        optimal_price, confidence
                    );

                    return (min_price, optimal_price, max_price, confidence, factors);
                }
            }
            warn!("Failed to parse LLM pricing response, falling back to statistical method");
        }
        Err(e) => {
            warn!(
                "LLM pricing request failed: {}, falling back to statistical method",
                e
            );
        }
    }

    // Fallback to statistical pricing if AI fails
    generate_statistical_pricing(comparables, unit)
}

/// Generate statistical pricing based on market comparables.
///
/// Fallback method when AI pricing is disabled or fails.
fn generate_statistical_pricing(
    comparables: &[db::models::market_pricing::MarketComparable],
    unit: &db::models::Unit,
) -> (Decimal, Decimal, Decimal, Decimal, serde_json::Value) {
    if comparables.is_empty() {
        // No comparables - use basic estimation based on size
        // Note: This is a rough EUR-equivalent baseline. For non-EUR currencies,
        // the actual prices should be adjusted using market comparables when available.
        let base_price_per_sqm = Decimal::new(15, 0); // ~15 EUR-equivalent/sqm default
        let size = unit.size_sqm.unwrap_or(Decimal::new(50, 0));
        let base_price = base_price_per_sqm * size;

        let min_price = base_price * Decimal::new(85, 0) / Decimal::new(100, 0); // 85%
        let optimal_price = base_price;
        let max_price = base_price * Decimal::new(115, 0) / Decimal::new(100, 0); // 115%
        let confidence = Decimal::new(40, 0); // Low confidence without comparables

        let factors = json!({
            "ai_generated": false,
            "method": "size_based_estimation",
            "base_price_per_sqm": 15,
            "comparables_used": 0,
            "note": "No market comparables available, using size-based estimation with EUR-equivalent baseline. Prices may need adjustment for local markets."
        });

        return (min_price, optimal_price, max_price, confidence, factors);
    }

    // Calculate statistics from comparables
    let total_rent: Decimal = comparables.iter().map(|c| c.monthly_rent).sum();
    let avg_rent = total_rent / Decimal::from(comparables.len() as u32);
    let min_rent = comparables
        .iter()
        .map(|c| c.monthly_rent)
        .min()
        .unwrap_or(avg_rent);
    let max_rent = comparables
        .iter()
        .map(|c| c.monthly_rent)
        .max()
        .unwrap_or(avg_rent);

    // Weight by similarity score
    let weighted_sum: Decimal = comparables
        .iter()
        .map(|c| c.monthly_rent * c.similarity_score / Decimal::new(100, 0))
        .sum();
    let weight_total: Decimal = comparables
        .iter()
        .map(|c| c.similarity_score / Decimal::new(100, 0))
        .sum();

    let weighted_avg = if weight_total > Decimal::ZERO {
        weighted_sum / weight_total
    } else {
        avg_rent
    };

    // Calculate confidence based on comparables count and similarity
    let avg_similarity: Decimal = comparables
        .iter()
        .map(|c| c.similarity_score)
        .sum::<Decimal>()
        / Decimal::from(comparables.len() as u32);

    let base_confidence = match comparables.len() {
        0 => Decimal::new(30, 0),
        1..=2 => Decimal::new(50, 0),
        3..=5 => Decimal::new(65, 0),
        6..=10 => Decimal::new(80, 0),
        _ => Decimal::new(90, 0),
    };
    let confidence = base_confidence * avg_similarity / Decimal::new(100, 0);

    let factors = json!({
        "ai_generated": false,
        "method": "statistical_analysis",
        "comparables_used": comparables.len(),
        "average_rent": avg_rent.to_string(),
        "weighted_average": weighted_avg.to_string(),
        "average_similarity": avg_similarity.to_string(),
        "rent_range": {
            "min": min_rent.to_string(),
            "max": max_rent.to_string()
        }
    });

    // Set pricing bands around weighted average
    let min_price = weighted_avg * Decimal::new(90, 0) / Decimal::new(100, 0); // 90%
    let optimal_price = weighted_avg;
    let max_price = weighted_avg * Decimal::new(110, 0) / Decimal::new(100, 0); // 110%

    (min_price, optimal_price, max_price, confidence, factors)
}

async fn get_recommendation(
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

    match s.market_pricing_repo.get_recommendation(id, org_id).await {
        Ok(Some(rec)) => Ok(Json(to_json_value(rec)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Recommendation not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Get detailed recommendation with factor explanations.
///
/// Story 132.2: Returns the recommendation with detailed factor breakdown,
/// market comparables, and human-readable explanations of pricing influences.
async fn get_recommendation_details(
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

    // Get the base recommendation
    let rec = s
        .market_pricing_repo
        .get_recommendation(id, org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::not_found("Recommendation not found")),
            )
        })?;

    // Parse the factors JSON and create human-readable explanations
    let factors_explanation = parse_pricing_factors(&rec.factors);

    // Get comparables if we have market stats reference
    let comparables = if let Some(stats_id) = rec.market_stats_id {
        // Try to get stats to find the region
        if let Ok(Some(stats)) = s
            .market_pricing_repo
            .get_market_statistics_by_id(stats_id)
            .await
        {
            // Get unit to determine property type
            // TODO: Migrate to find_by_id_rls when RlsConnection is added to this handler
            #[allow(deprecated)]
            if let Ok(Some(unit)) = s.unit_repo.find_by_id(rec.unit_id).await {
                let size = unit.size_sqm.unwrap_or(Decimal::new(50, 0));
                s.market_pricing_repo
                    .get_market_comparables(stats.region_id, &unit.unit_type, size, 5, None)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Build the detailed response
    let response = json!({
        "recommendation": rec,
        "factors_explanation": factors_explanation,
        "comparables": comparables,
        "market_stats": null // Can be enhanced to include full stats
    });

    Ok(Json(response))
}

/// Parse pricing factors JSON and generate human-readable explanations.
fn parse_pricing_factors(factors: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut explanations = vec![];

    let is_ai_generated = factors
        .get("ai_generated")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if is_ai_generated {
        // AI-generated factors
        if let Some(ai_factors) = factors.get("factors") {
            if let Some(location) = ai_factors.get("location_score").and_then(|v| v.as_f64()) {
                explanations.push(json!({
                    "name": "Location",
                    "impact": location,
                    "description": format_factor_description("location", location)
                }));
            }
            if let Some(size) = ai_factors.get("size_adjustment").and_then(|v| v.as_f64()) {
                explanations.push(json!({
                    "name": "Property Size",
                    "impact": size,
                    "description": format_factor_description("size", size)
                }));
            }
            if let Some(amenities) = ai_factors.get("amenities_premium").and_then(|v| v.as_f64()) {
                explanations.push(json!({
                    "name": "Amenities",
                    "impact": amenities,
                    "description": format_factor_description("amenities", amenities)
                }));
            }
            if let Some(market) = ai_factors
                .get("market_positioning")
                .and_then(|v| v.as_f64())
            {
                explanations.push(json!({
                    "name": "Market Position",
                    "impact": market,
                    "description": format_factor_description("market", market)
                }));
            }
            if let Some(building) = ai_factors.get("building_quality").and_then(|v| v.as_f64()) {
                explanations.push(json!({
                    "name": "Building Quality",
                    "impact": building,
                    "description": format_factor_description("building", building)
                }));
            }
        }

        // Add AI reasoning if available
        if let Some(reasoning) = factors.get("reasoning") {
            explanations.push(json!({
                "name": "AI Analysis",
                "impact": 0,
                "description": reasoning.as_str().unwrap_or("AI analysis complete")
            }));
        }
    } else {
        // Statistical method explanations
        let method = factors
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match method {
            "size_based_estimation" => {
                explanations.push(json!({
                    "name": "Estimation Method",
                    "impact": 0,
                    "description": "Price estimated based on property size (no market comparables available)"
                }));
            }
            "statistical_analysis" => {
                let comparables_used = factors
                    .get("comparables_used")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                explanations.push(json!({
                    "name": "Statistical Analysis",
                    "impact": 0,
                    "description": format!("Price based on {} comparable properties with similarity-weighted averaging", comparables_used)
                }));

                if let Some(avg_rent) = factors.get("average_rent").and_then(|v| v.as_str()) {
                    explanations.push(json!({
                        "name": "Market Average",
                        "impact": 0,
                        "description": format!("Average comparable rent: {}", avg_rent)
                    }));
                }
            }
            _ => {
                explanations.push(json!({
                    "name": "Analysis Method",
                    "impact": 0,
                    "description": "Pricing recommendation generated using available data"
                }));
            }
        }
    }

    explanations
}

/// Generate human-readable description for a pricing factor.
fn format_factor_description(factor_type: &str, impact: f64) -> String {
    let direction = if impact > 0.0 {
        "increases"
    } else if impact < 0.0 {
        "decreases"
    } else {
        "has neutral effect on"
    };

    let magnitude = if impact.abs() > 7.0 {
        "significantly"
    } else if impact.abs() > 3.0 {
        "moderately"
    } else {
        "slightly"
    };

    match factor_type {
        "location" => format!("Location {} {} the recommended price", magnitude, direction),
        "size" => format!(
            "Property size {} {} the recommended price relative to market average",
            magnitude, direction
        ),
        "amenities" => format!(
            "Available amenities {} {} the price premium",
            magnitude, direction
        ),
        "market" => format!(
            "Current market conditions {} {} competitive pricing",
            magnitude, direction
        ),
        "building" => format!(
            "Building quality and age {} {} property value",
            magnitude, direction
        ),
        _ => format!("{} {} the price", factor_type, direction),
    }
}

async fn accept_recommendation(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AcceptPricingRecommendation>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    let user_id = user.user_id;

    match s
        .market_pricing_repo
        .accept_recommendation(id, org_id, user_id, req.accepted_price)
        .await
    {
        Ok(Some(rec)) => Ok(Json(to_json_value(rec)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Recommendation not found")),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn reject_recommendation(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RejectPricingRecommendation>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .market_pricing_repo
        .reject_recommendation(id, org_id, &req.rejection_reason)
        .await
    {
        Ok(Some(rec)) => Ok(Json(to_json_value(rec)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Recommendation not found")),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

// =============================================================================
// UNIT PRICING HISTORY
// =============================================================================

async fn get_pricing_history(
    State(s): State<AppState>,
    user: AuthUser,
    Path(unit_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s
        .market_pricing_repo
        .get_pricing_history(unit_id, org_id)
        .await
    {
        Ok(history) => Ok(Json(json!({ "history": history }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn record_price_change(
    State(s): State<AppState>,
    user: AuthUser,
    Path(unit_id): Path<Uuid>,
    Json(mut req): Json<RecordPriceChange>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate unit belongs to user's organization
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    let user_id = user.user_id;

    // Verify unit belongs to org via building
    // TODO: Migrate to find_by_id_rls when RlsConnection is added to this handler
    #[allow(deprecated)]
    let unit = s.unit_repo.find_by_id(unit_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )
    })?;
    let unit = unit.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Unit not found")),
        )
    })?;
    // TODO: Migrate to find_by_id_rls when RlsConnection is added to this handler
    #[allow(deprecated)]
    let building = s
        .building_repo
        .find_by_id(unit.building_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;
    let building = building.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Building not found")),
        )
    })?;
    if building.organization_id != org_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::forbidden(
                "Unit does not belong to your organization",
            )),
        ));
    }

    req.unit_id = unit_id;

    match s
        .market_pricing_repo
        .record_price_change(req, user_id)
        .await
    {
        Ok(history) => Ok(Json(to_json_value(history)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn get_current_rent(
    State(s): State<AppState>,
    user: AuthUser,
    Path(unit_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate unit belongs to user's organization
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify unit belongs to org via building
    // TODO: Migrate to find_by_id_rls when RlsConnection is added to this handler
    #[allow(deprecated)]
    let unit = s.unit_repo.find_by_id(unit_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )
    })?;
    let unit = unit.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Unit not found")),
        )
    })?;
    // TODO: Migrate to find_by_id_rls when RlsConnection is added to this handler
    #[allow(deprecated)]
    let building = s
        .building_repo
        .find_by_id(unit.building_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;
    let building = building.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Building not found")),
        )
    })?;
    if building.organization_id != org_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::forbidden(
                "Unit does not belong to your organization",
            )),
        ));
    }

    match s.market_pricing_repo.get_current_rent(unit_id).await {
        Ok(Some(rent)) => Ok(Json(json!({ "current_rent": rent }))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("No pricing history for unit")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// COMPARATIVE MARKET ANALYSIS
// =============================================================================

async fn list_cmas(
    State(s): State<AppState>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.market_pricing_repo.list_cmas(org_id).await {
        Ok(cmas) => Ok(Json(json!({ "analyses": cmas }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn create_cma(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateComparativeMarketAnalysis>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;
    let user_id = user.user_id;

    match s.market_pricing_repo.create_cma(org_id, user_id, req).await {
        Ok(cma) => Ok(Json(to_json_value(cma)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn get_cma(
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

    match s.market_pricing_repo.get_cma(id, org_id).await {
        Ok(Some(cma)) => Ok(Json(to_json_value(cma)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("CMA not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn update_cma(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateComparativeMarketAnalysis>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    match s.market_pricing_repo.update_cma(id, org_id, req).await {
        Ok(Some(cma)) => Ok(Json(to_json_value(cma)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("CMA not found")),
        )),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

async fn delete_cma(
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

    match s.market_pricing_repo.delete_cma(id, org_id).await {
        Ok(true) => Ok(Json(json!({ "deleted": true }))),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("CMA not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn get_cma_properties(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate CMA belongs to user's organization
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify CMA ownership
    let cma = s.market_pricing_repo.get_cma(id, org_id).await;
    if matches!(cma, Ok(None) | Err(_)) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("CMA not found or not accessible")),
        ));
    }

    match s.market_pricing_repo.get_cma_properties(id).await {
        Ok(props) => Ok(Json(json!({ "properties": props }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

async fn add_cma_property(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<AddCmaProperty>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate CMA belongs to user's organization
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify CMA ownership
    let cma = s.market_pricing_repo.get_cma(id, org_id).await;
    if matches!(cma, Ok(None) | Err(_)) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("CMA not found or not accessible")),
        ));
    }

    match s.market_pricing_repo.add_property_to_cma(id, req).await {
        Ok(prop) => Ok(Json(to_json_value(prop)?)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(&e.to_string())),
        )),
    }
}

// =============================================================================
// CMA WITH DETAILS (Story 132.4)
// =============================================================================

/// Get CMA with full analysis details.
///
/// Story 132.4: Returns the CMA with all properties and computed analysis summary
/// including average rent, price per sqm, price ranges, and rental yield ranges.
async fn get_cma_details(
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

    match s.market_pricing_repo.get_cma_with_details(id, org_id).await {
        Ok(Some(cma_details)) => Ok(Json(to_json_value(cma_details)?)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("CMA not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Remove a property from a CMA.
///
/// Story 132.4: Allows removing individual properties from a comparative analysis.
async fn remove_cma_property(
    State(s): State<AppState>,
    user: AuthUser,
    Path((cma_id, property_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate CMA belongs to user's organization
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify CMA ownership
    let cma = s.market_pricing_repo.get_cma(cma_id, org_id).await;
    if matches!(cma, Ok(None) | Err(_)) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("CMA not found or not accessible")),
        ));
    }

    match s
        .market_pricing_repo
        .remove_property_from_cma(cma_id, property_id)
        .await
    {
        Ok(true) => Ok(Json(json!({ "deleted": true }))),
        Ok(false) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("Property not found in CMA")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

/// Recalculate CMA aggregate metrics.
///
/// Story 132.4: Recalculates the average price per sqm and rental yield
/// based on current properties and updates the CMA record.
async fn recalculate_cma(
    State(s): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate CMA belongs to user's organization
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify CMA ownership
    let cma = s.market_pricing_repo.get_cma(id, org_id).await;
    if matches!(cma, Ok(None) | Err(_)) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found("CMA not found or not accessible")),
        ));
    }

    match s.market_pricing_repo.recalculate_cma_metrics(id).await {
        Ok(()) => Ok(Json(json!({ "recalculated": true }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// COMPARABLES LOOKUP
// =============================================================================

#[derive(Debug, Deserialize, IntoParams)]
pub struct ComparablesParams {
    pub region_id: Uuid,
    pub property_type: String,
    pub size_sqm: Decimal,
    pub limit: Option<i32>,
    /// Maximum age of market data in days (default: 90). Slower markets may need higher values.
    pub max_age_days: Option<i32>,
}

async fn get_comparables(
    State(s): State<AppState>,
    user: AuthUser,
    Query(params): Query<ComparablesParams>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate region belongs to user's organization
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Verify region ownership
    let region = s
        .market_pricing_repo
        .get_region(params.region_id, org_id)
        .await;
    if matches!(region, Ok(None) | Err(_)) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::not_found(
                "Region not found or not accessible",
            )),
        ));
    }

    let limit = params.limit.unwrap_or(10).min(50);

    match s
        .market_pricing_repo
        .get_market_comparables(
            params.region_id,
            &params.property_type,
            params.size_sqm,
            limit,
            params.max_age_days,
        )
        .await
    {
        Ok(comparables) => Ok(Json(json!({ "comparables": comparables }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::internal_error(&e.to_string())),
        )),
    }
}

// =============================================================================
// DASHBOARD (Story 132.3)
// =============================================================================

/// Get pricing dashboard data.
///
/// Story 132.3: Returns portfolio summary, market trends, units with recommendations,
/// and vacancy trends. Supports filtering by region, building, and date range.
async fn get_pricing_dashboard(
    State(s): State<AppState>,
    user: AuthUser,
    Query(query): Query<PricingDashboardQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Get portfolio summary
    let portfolio_summary = s
        .market_pricing_repo
        .get_portfolio_summary(org_id, query.building_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&format!(
                    "Failed to get portfolio summary: {}",
                    e
                ))),
            )
        })?;

    // Get market trends (default to 12 months)
    let market_trends = if let Some(region_id) = query.region_id {
        s.market_pricing_repo
            .get_market_trends(region_id, 12)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    // Get units with recommendations
    let units_with_recommendations = s
        .market_pricing_repo
        .get_units_with_recommendations(org_id, query.building_id)
        .await
        .unwrap_or_default();

    // Get vacancy trends (12 months)
    let vacancy_trends = s
        .market_pricing_repo
        .get_vacancy_trends(org_id, 12)
        .await
        .unwrap_or_default();

    let dashboard = PricingDashboard {
        portfolio_summary,
        market_trends,
        units_with_recommendations,
        vacancy_trends,
    };

    Ok(Json(to_json_value(dashboard)?))
}

/// Export pricing data preview (data-only endpoint).
///
/// Story 132.3: Export market data and recommendations for external analysis.
///
/// Note: This endpoint returns the data as JSON for preview purposes. The format
/// parameter indicates the intended export format but actual file generation
/// (CSV/XLSX download) is not yet implemented. Use the JSON response for
/// integration with external tools or for preview before full export support.
async fn export_pricing_data(
    State(s): State<AppState>,
    user: AuthUser,
    Json(req): Json<ExportPricingDataRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let org_id = user.tenant_id.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request("Tenant context required")),
        )
    })?;

    // Validate format
    if !["csv", "xlsx"].contains(&req.format.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::bad_request(
                "Invalid format. Supported formats: csv, xlsx",
            )),
        ));
    }

    // Get data for export
    let data_query = MarketDataQuery {
        region_id: req.region_id,
        property_type: None,
        min_size_sqm: None,
        max_size_sqm: None,
        min_rent: None,
        max_rent: None,
        rooms: None,
        // SAFETY: and_hms_opt always succeeds for valid time values (0:0:0 and 23:59:59)
        from_date: req.from_date.map(|d| {
            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                d.and_hms_opt(0, 0, 0).expect("00:00:00 is always valid"),
                chrono::Utc,
            )
        }),
        to_date: req.to_date.map(|d| {
            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                d.and_hms_opt(23, 59, 59).expect("23:59:59 is always valid"),
                chrono::Utc,
            )
        }),
        page: None,
        limit: Some(1000),
    };

    let data_points = s
        .market_pricing_repo
        .list_data_points(org_id, data_query)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::internal_error(&e.to_string())),
            )
        })?;

    // Return data as JSON for preview/integration (file generation not yet implemented)
    Ok(Json(json!({
        "export": {
            "format": req.format,
            "record_count": data_points.len(),
            "generated_at": chrono::Utc::now().to_rfc3339(),
            "status": "preview",
            "note": "File download not yet implemented. Use this JSON data for external processing."
        },
        "data": data_points
    })))
}
