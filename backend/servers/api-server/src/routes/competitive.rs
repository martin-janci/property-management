//! Competitive feature routes (Epic 70) - Virtual tours, dynamic pricing, neighborhood insights, comparables.

use crate::state::AppState;
use api_core::extractors::TenantExtractor;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use db::models::{
    amenity_category, confidence_level, tour_type, ComparablePropertySummary, ComparablesRequest,
    ComparablesResponse, ComparisonTableEntry, CompetitiveAnalysis, CompetitiveFeaturesStatus,
    CreateTourHotspot, CreateVirtualTour, NearbyAmenity, NeighborhoodInsights,
    NeighborhoodInsightsRequest, NeighborhoodInsightsResponse, PriceHistory, PriceRange,
    PricingAnalysisRequest, PricingAnalysisResponse, PricingFactor, PricingSuggestion,
    ReorderTours, TourHotspot, UpdateVirtualTour, VirtualTour, VirtualTourWithHotspots,
};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// Create competitive features router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Virtual Tours (Story 70.1)
        .route("/listings/{listing_id}/tours", get(list_virtual_tours))
        .route("/listings/{listing_id}/tours", post(create_virtual_tour))
        .route(
            "/listings/{listing_id}/tours/reorder",
            post(reorder_virtual_tours),
        )
        .route(
            "/listings/{listing_id}/tours/{tour_id}",
            get(get_virtual_tour),
        )
        .route(
            "/listings/{listing_id}/tours/{tour_id}",
            put(update_virtual_tour),
        )
        .route(
            "/listings/{listing_id}/tours/{tour_id}",
            delete(delete_virtual_tour),
        )
        .route(
            "/listings/{listing_id}/tours/{tour_id}/hotspots",
            get(list_tour_hotspots),
        )
        .route(
            "/listings/{listing_id}/tours/{tour_id}/hotspots",
            post(create_tour_hotspot),
        )
        .route(
            "/listings/{listing_id}/tours/{tour_id}/hotspots/{hotspot_id}",
            delete(delete_tour_hotspot),
        )
        // Dynamic Pricing (Story 70.2)
        .route(
            "/listings/{listing_id}/pricing",
            get(get_pricing_suggestion),
        )
        .route(
            "/listings/{listing_id}/pricing/analyze",
            post(analyze_pricing),
        )
        .route(
            "/listings/{listing_id}/pricing/history",
            get(get_price_history),
        )
        // Neighborhood Insights (Story 70.3)
        .route(
            "/listings/{listing_id}/neighborhood",
            get(get_neighborhood_insights),
        )
        .route(
            "/listings/{listing_id}/neighborhood/refresh",
            post(refresh_neighborhood_insights),
        )
        .route(
            "/listings/{listing_id}/neighborhood/amenities",
            get(get_nearby_amenities),
        )
        // Comparables (Story 70.4)
        .route("/listings/{listing_id}/comparables", get(get_comparables))
        .route(
            "/listings/{listing_id}/comparables/refresh",
            post(refresh_comparables),
        )
        // Combined Analysis
        .route(
            "/listings/{listing_id}/competitive-analysis",
            get(get_competitive_analysis),
        )
        .route(
            "/listings/{listing_id}/competitive-status",
            get(get_competitive_status),
        )
}

// ============================================
// Story 70.1: Virtual Tour Integration
// ============================================

/// List virtual tours response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VirtualToursResponse {
    pub tours: Vec<VirtualTourWithHotspots>,
    pub total: i64,
}

/// List all virtual tours for a listing.
#[utoipa::path(
    get,
    path = "/api/v1/competitive/listings/{listing_id}/tours",
    tag = "Competitive Features",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Virtual tours list", body = VirtualToursResponse),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_virtual_tours(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<VirtualToursResponse>, (axum::http::StatusCode, String)> {
    // Mock implementation - would query database
    let now = Utc::now();

    let tour = VirtualTour {
        id: Uuid::new_v4(),
        listing_id,
        tour_type: tour_type::MATTERPORT.to_string(),
        title: Some("Full Property Tour".to_string()),
        description: Some("Explore the entire property in 3D".to_string()),
        photo_url: None,
        embed_url: Some("https://my.matterport.com/show/?m=example".to_string()),
        external_id: Some("example-tour-id".to_string()),
        video_url: None,
        thumbnail_url: Some("https://example.com/tour-thumb.jpg".to_string()),
        display_order: 1,
        is_featured: true,
        created_at: now,
        updated_at: now,
    };

    let hotspot = TourHotspot {
        id: Uuid::new_v4(),
        tour_id: tour.id,
        label: "Living Room".to_string(),
        description: Some("Spacious living area with natural light".to_string()),
        position_x: dec!(45.5),
        position_y: dec!(30.2),
        link_to_tour_id: None,
        action_type: Some("info".to_string()),
        created_at: now,
    };

    let tour_with_hotspots = VirtualTourWithHotspots {
        tour,
        hotspots: vec![hotspot],
    };

    Ok(Json(VirtualToursResponse {
        tours: vec![tour_with_hotspots],
        total: 1,
    }))
}

/// Create a virtual tour for a listing.
#[utoipa::path(
    post,
    path = "/api/v1/competitive/listings/{listing_id}/tours",
    tag = "Competitive Features",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    request_body = CreateVirtualTour,
    responses(
        (status = 201, description = "Virtual tour created", body = VirtualTour),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_virtual_tour(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path(listing_id): Path<Uuid>,
    Json(data): Json<CreateVirtualTour>,
) -> Result<Json<VirtualTour>, (axum::http::StatusCode, String)> {
    // Validate tour type
    let valid_types = [
        tour_type::PHOTO_360,
        tour_type::MATTERPORT,
        tour_type::VIDEO,
        tour_type::EXTERNAL_EMBED,
    ];

    if !valid_types.contains(&data.tour_type.as_str()) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid tour type: {}", data.tour_type),
        ));
    }

    let now = Utc::now();
    let tour = VirtualTour {
        id: Uuid::new_v4(),
        listing_id,
        tour_type: data.tour_type,
        title: data.title,
        description: data.description,
        photo_url: data.photo_url,
        embed_url: data.embed_url,
        external_id: data.external_id,
        video_url: data.video_url,
        thumbnail_url: data.thumbnail_url,
        display_order: 1,
        is_featured: data.is_featured,
        created_at: now,
        updated_at: now,
    };

    Ok(Json(tour))
}

/// Get a single virtual tour.
#[utoipa::path(
    get,
    path = "/api/v1/competitive/listings/{listing_id}/tours/{tour_id}",
    tag = "Competitive Features",
    params(
        ("listing_id" = Uuid, Path, description = "Listing ID"),
        ("tour_id" = Uuid, Path, description = "Tour ID")
    ),
    responses(
        (status = 200, description = "Virtual tour details", body = VirtualTourWithHotspots),
        (status = 404, description = "Tour not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_virtual_tour(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path((listing_id, tour_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<VirtualTourWithHotspots>, (axum::http::StatusCode, String)> {
    let now = Utc::now();

    let tour = VirtualTour {
        id: tour_id,
        listing_id,
        tour_type: tour_type::MATTERPORT.to_string(),
        title: Some("Property Tour".to_string()),
        description: None,
        photo_url: None,
        embed_url: Some("https://my.matterport.com/show/?m=example".to_string()),
        external_id: None,
        video_url: None,
        thumbnail_url: None,
        display_order: 1,
        is_featured: true,
        created_at: now,
        updated_at: now,
    };

    Ok(Json(VirtualTourWithHotspots {
        tour,
        hotspots: vec![],
    }))
}

/// Update a virtual tour.
#[utoipa::path(
    put,
    path = "/api/v1/competitive/listings/{listing_id}/tours/{tour_id}",
    tag = "Competitive Features",
    params(
        ("listing_id" = Uuid, Path, description = "Listing ID"),
        ("tour_id" = Uuid, Path, description = "Tour ID")
    ),
    request_body = UpdateVirtualTour,
    responses(
        (status = 200, description = "Virtual tour updated", body = VirtualTour),
        (status = 404, description = "Tour not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn update_virtual_tour(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path((listing_id, tour_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<UpdateVirtualTour>,
) -> Result<Json<VirtualTour>, (axum::http::StatusCode, String)> {
    let now = Utc::now();

    let tour = VirtualTour {
        id: tour_id,
        listing_id,
        tour_type: tour_type::MATTERPORT.to_string(),
        title: data.title.or(Some("Updated Tour".to_string())),
        description: data.description,
        photo_url: data.photo_url,
        embed_url: data.embed_url,
        external_id: None,
        video_url: data.video_url,
        thumbnail_url: data.thumbnail_url,
        display_order: 1,
        is_featured: data.is_featured.unwrap_or(false),
        created_at: now,
        updated_at: now,
    };

    Ok(Json(tour))
}

/// Delete a virtual tour.
#[utoipa::path(
    delete,
    path = "/api/v1/competitive/listings/{listing_id}/tours/{tour_id}",
    tag = "Competitive Features",
    params(
        ("listing_id" = Uuid, Path, description = "Listing ID"),
        ("tour_id" = Uuid, Path, description = "Tour ID")
    ),
    responses(
        (status = 204, description = "Virtual tour deleted"),
        (status = 404, description = "Tour not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn delete_virtual_tour(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path((_listing_id, _tour_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Reorder virtual tours.
#[utoipa::path(
    post,
    path = "/api/v1/competitive/listings/{listing_id}/tours/reorder",
    tag = "Competitive Features",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    request_body = ReorderTours,
    responses(
        (status = 200, description = "Tours reordered"),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn reorder_virtual_tours(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path(_listing_id): Path<Uuid>,
    Json(_data): Json<ReorderTours>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    Ok(axum::http::StatusCode::OK)
}

/// List hotspots for a tour.
#[utoipa::path(
    get,
    path = "/api/v1/competitive/listings/{listing_id}/tours/{tour_id}/hotspots",
    tag = "Competitive Features",
    params(
        ("listing_id" = Uuid, Path, description = "Listing ID"),
        ("tour_id" = Uuid, Path, description = "Tour ID")
    ),
    responses(
        (status = 200, description = "Tour hotspots", body = Vec<TourHotspot>),
        (status = 404, description = "Tour not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_tour_hotspots(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path((_listing_id, tour_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<TourHotspot>>, (axum::http::StatusCode, String)> {
    let hotspot = TourHotspot {
        id: Uuid::new_v4(),
        tour_id,
        label: "Kitchen".to_string(),
        description: Some("Modern kitchen with appliances".to_string()),
        position_x: dec!(60.0),
        position_y: dec!(40.0),
        link_to_tour_id: None,
        action_type: Some("info".to_string()),
        created_at: Utc::now(),
    };

    Ok(Json(vec![hotspot]))
}

/// Create a hotspot on a tour.
#[utoipa::path(
    post,
    path = "/api/v1/competitive/listings/{listing_id}/tours/{tour_id}/hotspots",
    tag = "Competitive Features",
    params(
        ("listing_id" = Uuid, Path, description = "Listing ID"),
        ("tour_id" = Uuid, Path, description = "Tour ID")
    ),
    request_body = CreateTourHotspot,
    responses(
        (status = 201, description = "Hotspot created", body = TourHotspot),
        (status = 404, description = "Tour not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_tour_hotspot(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path((_listing_id, tour_id)): Path<(Uuid, Uuid)>,
    Json(data): Json<CreateTourHotspot>,
) -> Result<Json<TourHotspot>, (axum::http::StatusCode, String)> {
    let hotspot = TourHotspot {
        id: Uuid::new_v4(),
        tour_id,
        label: data.label,
        description: data.description,
        position_x: data.position_x,
        position_y: data.position_y,
        link_to_tour_id: data.link_to_tour_id,
        action_type: data.action_type,
        created_at: Utc::now(),
    };

    Ok(Json(hotspot))
}

/// Delete a hotspot from a tour.
#[utoipa::path(
    delete,
    path = "/api/v1/competitive/listings/{listing_id}/tours/{tour_id}/hotspots/{hotspot_id}",
    tag = "Competitive Features",
    params(
        ("listing_id" = Uuid, Path, description = "Listing ID"),
        ("tour_id" = Uuid, Path, description = "Tour ID"),
        ("hotspot_id" = Uuid, Path, description = "Hotspot ID")
    ),
    responses(
        (status = 204, description = "Hotspot deleted"),
        (status = 404, description = "Hotspot not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn delete_tour_hotspot(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path((_listing_id, _tour_id, _hotspot_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ============================================
// Story 70.2: Dynamic Pricing Suggestions
// ============================================

/// Get current pricing suggestion for a listing.
#[utoipa::path(
    get,
    path = "/api/v1/competitive/listings/{listing_id}/pricing",
    tag = "Competitive Features",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Pricing suggestion", body = PricingSuggestion),
        (status = 404, description = "No pricing data available"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_pricing_suggestion(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<PricingSuggestion>, (axum::http::StatusCode, String)> {
    let now = Utc::now();

    let suggestion = PricingSuggestion {
        id: Uuid::new_v4(),
        listing_id,
        suggested_price_low: dec!(185000),
        suggested_price_mid: dec!(195000),
        suggested_price_high: dec!(210000),
        currency: "EUR".to_string(),
        confidence_level: confidence_level::HIGH.to_string(),
        confidence_score: Some(85),
        comparables_count: 8,
        market_trend: "stable".to_string(),
        seasonal_adjustment: Some(dec!(1.02)),
        calculated_at: now,
        valid_until: now + chrono::Duration::days(7),
    };

    Ok(Json(suggestion))
}

/// Analyze pricing for a listing and generate suggestions.
#[utoipa::path(
    post,
    path = "/api/v1/competitive/listings/{listing_id}/pricing/analyze",
    tag = "Competitive Features",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    request_body = PricingAnalysisRequest,
    responses(
        (status = 200, description = "Pricing analysis", body = PricingAnalysisResponse),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn analyze_pricing(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path(listing_id): Path<Uuid>,
    Json(_data): Json<PricingAnalysisRequest>,
) -> Result<Json<PricingAnalysisResponse>, (axum::http::StatusCode, String)> {
    let now = Utc::now();

    let suggestion = PricingSuggestion {
        id: Uuid::new_v4(),
        listing_id,
        suggested_price_low: dec!(185000),
        suggested_price_mid: dec!(195000),
        suggested_price_high: dec!(210000),
        currency: "EUR".to_string(),
        confidence_level: confidence_level::HIGH.to_string(),
        confidence_score: Some(85),
        comparables_count: 8,
        market_trend: "stable".to_string(),
        seasonal_adjustment: Some(dec!(1.02)),
        calculated_at: now,
        valid_until: now + chrono::Duration::days(7),
    };

    let factors = vec![
        PricingFactor {
            id: Uuid::new_v4(),
            suggestion_id: suggestion.id,
            factor_type: "location".to_string(),
            factor_name: "Prime Location".to_string(),
            impact: dec!(5.0),
            explanation: "Located in a desirable neighborhood with good amenities".to_string(),
        },
        PricingFactor {
            id: Uuid::new_v4(),
            suggestion_id: suggestion.id,
            factor_type: "size".to_string(),
            factor_name: "Above Average Size".to_string(),
            impact: dec!(3.5),
            explanation: "Property size is larger than average for the area".to_string(),
        },
        PricingFactor {
            id: Uuid::new_v4(),
            suggestion_id: suggestion.id,
            factor_type: "condition".to_string(),
            factor_name: "Recently Renovated".to_string(),
            impact: dec!(4.0),
            explanation: "Recent renovations add value to the property".to_string(),
        },
    ];

    let price_history = vec![PriceHistory {
        id: Uuid::new_v4(),
        listing_id: Some(listing_id),
        property_type: "apartment".to_string(),
        city: "Bratislava".to_string(),
        postal_code: Some("81101".to_string()),
        transaction_type: "sale".to_string(),
        price: dec!(190000),
        price_per_sqm: Some(dec!(2800)),
        currency: "EUR".to_string(),
        recorded_at: now - chrono::Duration::days(30),
    }];

    let comparables_used = vec![ComparablePropertySummary {
        id: Uuid::new_v4(),
        property_type: "apartment".to_string(),
        city: "Bratislava".to_string(),
        size_sqm: dec!(68),
        rooms: Some(3),
        price: dec!(188000),
        price_per_sqm: dec!(2765),
        currency: "EUR".to_string(),
        distance_meters: dec!(500),
        similarity_score: 92,
        transaction_date: Some(now - chrono::Duration::days(45)),
        is_active: false,
    }];

    Ok(Json(PricingAnalysisResponse {
        suggestion,
        factors,
        price_history,
        comparables_used,
    }))
}

/// Get price history for a listing's area.
#[utoipa::path(
    get,
    path = "/api/v1/competitive/listings/{listing_id}/pricing/history",
    tag = "Competitive Features",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Price history", body = Vec<PriceHistory>),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_price_history(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<Vec<PriceHistory>>, (axum::http::StatusCode, String)> {
    let now = Utc::now();

    let history = vec![
        PriceHistory {
            id: Uuid::new_v4(),
            listing_id: Some(listing_id),
            property_type: "apartment".to_string(),
            city: "Bratislava".to_string(),
            postal_code: Some("81101".to_string()),
            transaction_type: "sale".to_string(),
            price: dec!(185000),
            price_per_sqm: Some(dec!(2720)),
            currency: "EUR".to_string(),
            recorded_at: now - chrono::Duration::days(90),
        },
        PriceHistory {
            id: Uuid::new_v4(),
            listing_id: None,
            property_type: "apartment".to_string(),
            city: "Bratislava".to_string(),
            postal_code: Some("81101".to_string()),
            transaction_type: "sale".to_string(),
            price: dec!(192000),
            price_per_sqm: Some(dec!(2825)),
            currency: "EUR".to_string(),
            recorded_at: now - chrono::Duration::days(60),
        },
    ];

    Ok(Json(history))
}

// ============================================
// Story 70.3: Neighborhood Insights
// ============================================

/// Get neighborhood insights for a listing.
#[utoipa::path(
    get,
    path = "/api/v1/competitive/listings/{listing_id}/neighborhood",
    tag = "Competitive Features",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Neighborhood insights", body = NeighborhoodInsightsResponse),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_neighborhood_insights(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<NeighborhoodInsightsResponse>, (axum::http::StatusCode, String)> {
    let now = Utc::now();
    let insights_id = Uuid::new_v4();

    let insights = NeighborhoodInsights {
        id: insights_id,
        listing_id: Some(listing_id),
        latitude: dec!(48.1486),
        longitude: dec!(17.1077),
        walk_score: Some(85),
        transit_score: Some(78),
        bike_score: Some(72),
        population: Some(450000),
        median_age: Some(dec!(38.5)),
        median_income: Some(dec!(1850)),
        crime_index: Some(25),
        safety_rating: Some("good".to_string()),
        data_sources: serde_json::json!({
            "walk_score": "Walk Score API",
            "amenities": "OpenStreetMap",
            "demographics": "Statistical Office SR"
        }),
        fetched_at: now,
        valid_until: now + chrono::Duration::days(30),
    };

    let amenities = vec![
        NearbyAmenity {
            id: Uuid::new_v4(),
            insights_id,
            category: amenity_category::SUPERMARKET.to_string(),
            name: "Tesco Express".to_string(),
            address: Some("Obchodna 15".to_string()),
            distance_meters: dec!(150),
            latitude: dec!(48.1490),
            longitude: dec!(17.1080),
            rating: Some(dec!(4.2)),
            details: None,
        },
        NearbyAmenity {
            id: Uuid::new_v4(),
            insights_id,
            category: amenity_category::TRANSIT_STOP.to_string(),
            name: "Hodzovo namestie (tram)".to_string(),
            address: None,
            distance_meters: dec!(200),
            latitude: dec!(48.1480),
            longitude: dec!(17.1070),
            rating: None,
            details: Some(serde_json::json!({"lines": ["1", "2", "4"]})),
        },
        NearbyAmenity {
            id: Uuid::new_v4(),
            insights_id,
            category: amenity_category::SCHOOL.to_string(),
            name: "Zakladna skola Grosslingova".to_string(),
            address: Some("Grosslingova 18".to_string()),
            distance_meters: dec!(450),
            latitude: dec!(48.1460),
            longitude: dec!(17.1100),
            rating: Some(dec!(4.5)),
            details: None,
        },
        NearbyAmenity {
            id: Uuid::new_v4(),
            insights_id,
            category: amenity_category::PARK.to_string(),
            name: "Medicka zahrada".to_string(),
            address: None,
            distance_meters: dec!(600),
            latitude: dec!(48.1450),
            longitude: dec!(17.1120),
            rating: Some(dec!(4.7)),
            details: None,
        },
    ];

    let mut amenities_by_category: HashMap<String, Vec<NearbyAmenity>> = HashMap::new();
    for amenity in &amenities {
        amenities_by_category
            .entry(amenity.category.clone())
            .or_default()
            .push(amenity.clone());
    }

    Ok(Json(NeighborhoodInsightsResponse {
        insights,
        amenities,
        amenities_by_category,
    }))
}

/// Refresh neighborhood insights for a listing.
#[utoipa::path(
    post,
    path = "/api/v1/competitive/listings/{listing_id}/neighborhood/refresh",
    tag = "Competitive Features",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    request_body = NeighborhoodInsightsRequest,
    responses(
        (status = 200, description = "Neighborhood insights refreshed", body = NeighborhoodInsightsResponse),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn refresh_neighborhood_insights(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
    Json(_data): Json<NeighborhoodInsightsRequest>,
) -> Result<Json<NeighborhoodInsightsResponse>, (axum::http::StatusCode, String)> {
    // Reuse get_neighborhood_insights with fresh data
    get_neighborhood_insights(State(state), tenant, Path(listing_id)).await
}

/// Get nearby amenities for a listing.
#[utoipa::path(
    get,
    path = "/api/v1/competitive/listings/{listing_id}/neighborhood/amenities",
    tag = "Competitive Features",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Nearby amenities", body = Vec<NearbyAmenity>),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_nearby_amenities(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path(_listing_id): Path<Uuid>,
) -> Result<Json<Vec<NearbyAmenity>>, (axum::http::StatusCode, String)> {
    let insights_id = Uuid::new_v4();

    let amenities = vec![NearbyAmenity {
        id: Uuid::new_v4(),
        insights_id,
        category: amenity_category::SUPERMARKET.to_string(),
        name: "Lidl".to_string(),
        address: Some("Stefanikova 25".to_string()),
        distance_meters: dec!(300),
        latitude: dec!(48.1495),
        longitude: dec!(17.1085),
        rating: Some(dec!(4.0)),
        details: None,
    }];

    Ok(Json(amenities))
}

// ============================================
// Story 70.4: Comparable Sales/Rentals
// ============================================

/// Get comparable properties for a listing.
#[utoipa::path(
    get,
    path = "/api/v1/competitive/listings/{listing_id}/comparables",
    tag = "Competitive Features",
    params(
        ("listing_id" = Uuid, Path, description = "Listing ID"),
        ComparablesRequest
    ),
    responses(
        (status = 200, description = "Comparable properties", body = ComparablesResponse),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_comparables(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path(listing_id): Path<Uuid>,
    Query(_params): Query<ComparablesRequest>,
) -> Result<Json<ComparablesResponse>, (axum::http::StatusCode, String)> {
    let now = Utc::now();

    let comparables = vec![
        ComparablePropertySummary {
            id: Uuid::new_v4(),
            property_type: "apartment".to_string(),
            city: "Bratislava".to_string(),
            size_sqm: dec!(72),
            rooms: Some(3),
            price: dec!(198000),
            price_per_sqm: dec!(2750),
            currency: "EUR".to_string(),
            distance_meters: dec!(350),
            similarity_score: 95,
            transaction_date: Some(now - chrono::Duration::days(20)),
            is_active: false,
        },
        ComparablePropertySummary {
            id: Uuid::new_v4(),
            property_type: "apartment".to_string(),
            city: "Bratislava".to_string(),
            size_sqm: dec!(68),
            rooms: Some(2),
            price: dec!(185000),
            price_per_sqm: dec!(2720),
            currency: "EUR".to_string(),
            distance_meters: dec!(500),
            similarity_score: 88,
            transaction_date: Some(now - chrono::Duration::days(45)),
            is_active: false,
        },
        ComparablePropertySummary {
            id: Uuid::new_v4(),
            property_type: "apartment".to_string(),
            city: "Bratislava".to_string(),
            size_sqm: dec!(75),
            rooms: Some(3),
            price: dec!(210000),
            price_per_sqm: dec!(2800),
            currency: "EUR".to_string(),
            distance_meters: dec!(800),
            similarity_score: 82,
            transaction_date: None,
            is_active: true,
        },
    ];

    let comparison_table = vec![
        ComparisonTableEntry {
            feature: "Size (sqm)".to_string(),
            source_value: "70".to_string(),
            comparable_values: vec!["72".to_string(), "68".to_string(), "75".to_string()],
        },
        ComparisonTableEntry {
            feature: "Rooms".to_string(),
            source_value: "3".to_string(),
            comparable_values: vec!["3".to_string(), "2".to_string(), "3".to_string()],
        },
        ComparisonTableEntry {
            feature: "Price/sqm".to_string(),
            source_value: "2785 EUR".to_string(),
            comparable_values: vec![
                "2750 EUR".to_string(),
                "2720 EUR".to_string(),
                "2800 EUR".to_string(),
            ],
        },
        ComparisonTableEntry {
            feature: "Distance".to_string(),
            source_value: "-".to_string(),
            comparable_values: vec!["350m".to_string(), "500m".to_string(), "800m".to_string()],
        },
    ];

    Ok(Json(ComparablesResponse {
        listing_id,
        comparables,
        comparison_table,
        average_price_per_sqm: dec!(2757),
        price_range: PriceRange {
            min: dec!(185000),
            max: dec!(210000),
            median: dec!(198000),
            currency: "EUR".to_string(),
        },
    }))
}

/// Refresh comparable properties for a listing.
#[utoipa::path(
    post,
    path = "/api/v1/competitive/listings/{listing_id}/comparables/refresh",
    tag = "Competitive Features",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Comparables refreshed", body = ComparablesResponse),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn refresh_comparables(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<ComparablesResponse>, (axum::http::StatusCode, String)> {
    // Reuse get_comparables with default params
    get_comparables(
        State(state),
        tenant,
        Path(listing_id),
        Query(ComparablesRequest::default()),
    )
    .await
}

// ============================================
// Combined Competitive Analysis
// ============================================

/// Get complete competitive analysis for a listing.
#[utoipa::path(
    get,
    path = "/api/v1/competitive/listings/{listing_id}/competitive-analysis",
    tag = "Competitive Features",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Complete competitive analysis", body = CompetitiveAnalysis),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_competitive_analysis(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<CompetitiveAnalysis>, (axum::http::StatusCode, String)> {
    // Fetch all competitive data
    let tours_response = list_virtual_tours(
        State(state.clone()),
        TenantExtractor(tenant.0.clone()),
        Path(listing_id),
    )
    .await?;

    let pricing_response = analyze_pricing(
        State(state.clone()),
        TenantExtractor(tenant.0.clone()),
        Path(listing_id),
        Json(PricingAnalysisRequest {
            listing_id,
            force_refresh: false,
        }),
    )
    .await
    .ok()
    .map(|r| r.0);

    let neighborhood_response = get_neighborhood_insights(
        State(state.clone()),
        TenantExtractor(tenant.0.clone()),
        Path(listing_id),
    )
    .await
    .ok()
    .map(|r| r.0);

    let comparables_response = get_comparables(
        State(state),
        tenant,
        Path(listing_id),
        Query(ComparablesRequest::default()),
    )
    .await
    .ok()
    .map(|r| r.0);

    Ok(Json(CompetitiveAnalysis {
        listing_id,
        virtual_tours: tours_response.0.tours,
        pricing_analysis: pricing_response,
        neighborhood: neighborhood_response,
        comparables: comparables_response,
        generated_at: Utc::now(),
    }))
}

/// Get competitive features status for a listing.
#[utoipa::path(
    get,
    path = "/api/v1/competitive/listings/{listing_id}/competitive-status",
    tag = "Competitive Features",
    params(("listing_id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Competitive features status", body = CompetitiveFeaturesStatus),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_competitive_status(
    State(_state): State<AppState>,
    TenantExtractor(_tenant): TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<CompetitiveFeaturesStatus>, (axum::http::StatusCode, String)> {
    // Mock implementation - would query actual data
    Ok(Json(CompetitiveFeaturesStatus {
        listing_id,
        has_virtual_tours: true,
        virtual_tour_count: 2,
        has_pricing_analysis: true,
        pricing_analysis_valid: true,
        has_neighborhood_insights: true,
        neighborhood_insights_valid: true,
        has_comparables: true,
        comparables_count: 5,
    }))
}
