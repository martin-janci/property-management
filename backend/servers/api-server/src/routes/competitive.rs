//! Competitive feature routes (Epic 70) - Virtual tours, dynamic pricing, neighborhood insights, comparables.
//!
//! # Authorization & data model (issue #895)
//!
//! Two security defects were closed here:
//!
//! 1. **Cross-tenant IDOR.** Every handler accepted a `listing_id` path param and
//!    a `TenantExtractor` but discarded both (`_tenant`, `_listing`, unused
//!    `State`). There was no ownership check, so any authenticated caller could
//!    target any listing across organizations. Every handler now resolves the
//!    listing scoped to the caller's tenant via
//!    [`ListingRepository::find_by_id_and_org`] and returns `404` for a missing
//!    or foreign-org listing *before* doing anything else. This mirrors the
//!    standard listing-ownership gate in `routes/listings.rs`.
//!
//! 2. **Fabricated data.** Every handler previously returned hard-coded mock
//!    data (e.g. `https://my.matterport.com/show/?m=example`, invented prices
//!    and amenities) while the OpenAPI implied production behavior. There is no
//!    persistence for any competitive feature — no `virtual_tours`,
//!    `tour_hotspots`, `pricing_suggestions`, `neighborhood_insights`,
//!    `nearby_amenities` tables and no repository. Rather than fabricate values,
//!    every handler now returns `501 Not Implemented` after the org-ownership
//!    gate, so callers get an honest signal. This mirrors the vendor-portal fix
//!    (#828) and developer-portal fix (#851). The org-ownership gate runs first
//!    regardless, so even the `501` path is tenant-checked (and a foreign-org
//!    listing still yields `404`, never `501`).

use crate::state::AppState;
use api_core::extractors::TenantExtractor;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use db::models::{
    ComparablesRequest, ComparablesResponse, CompetitiveAnalysis, CompetitiveFeaturesStatus,
    CreateTourHotspot, CreateVirtualTour, NearbyAmenity, NeighborhoodInsightsRequest,
    NeighborhoodInsightsResponse, PriceHistory, PricingAnalysisRequest, PricingAnalysisResponse,
    PricingSuggestion, ReorderTours, TourHotspot, UpdateVirtualTour, VirtualTour,
    VirtualTourWithHotspots,
};
use serde::{Deserialize, Serialize};
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
// Shared authorization + stub helpers (#895)
// ============================================

/// Resolve a listing scoped to the caller's tenant, or fail closed.
///
/// SECURITY (#895): every competitive handler takes a `listing_id` path param.
/// Before doing any work, the listing MUST be confirmed to belong to the
/// caller's organization. A missing listing OR a listing owned by a different
/// org both return `404` — never leaking existence across tenants and never
/// reaching the feature stub. Mirrors the ownership gate in `routes/listings.rs`.
async fn ensure_listing_in_org(
    state: &AppState,
    listing_id: Uuid,
    tenant: &TenantExtractor,
) -> Result<(), (StatusCode, String)> {
    state
        .listing_repo
        .find_by_id_and_org(listing_id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Listing not found".to_string()))?;
    Ok(())
}

/// Uniform "not implemented" error for competitive endpoints whose persistence
/// does not exist yet (#895).
///
/// These handlers previously fabricated mock data. Until the competitive-feature
/// schema lands (`virtual_tours`, `tour_hotspots`, `pricing_suggestions`,
/// `neighborhood_insights`, `nearby_amenities`, comparables stores), every
/// endpoint returns `501` after the org-ownership gate so callers get an honest
/// signal rather than fabricated values. Mirrors the vendor-portal (#828) and
/// developer-portal (#851) fixes.
fn not_implemented(feature: &str) -> (StatusCode, String) {
    (
        StatusCode::NOT_IMPLEMENTED,
        format!(
            "{feature} is not implemented yet (no competitive-feature persistence; see issue #895)"
        ),
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
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_virtual_tours(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<VirtualToursResponse>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Virtual tours"))
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
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_virtual_tour(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
    Json(_data): Json<CreateVirtualTour>,
) -> Result<Json<VirtualTour>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Creating virtual tours"))
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
        (status = 404, description = "Listing or tour not found"),
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_virtual_tour(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path((listing_id, _tour_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<VirtualTourWithHotspots>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Virtual tours"))
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
        (status = 404, description = "Listing or tour not found"),
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn update_virtual_tour(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path((listing_id, _tour_id)): Path<(Uuid, Uuid)>,
    Json(_data): Json<UpdateVirtualTour>,
) -> Result<Json<VirtualTour>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Updating virtual tours"))
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
        (status = 404, description = "Listing or tour not found"),
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn delete_virtual_tour(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path((listing_id, _tour_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Deleting virtual tours"))
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
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn reorder_virtual_tours(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
    Json(_data): Json<ReorderTours>,
) -> Result<StatusCode, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Reordering virtual tours"))
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
        (status = 404, description = "Listing or tour not found"),
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_tour_hotspots(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path((listing_id, _tour_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<TourHotspot>>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Tour hotspots"))
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
        (status = 404, description = "Listing or tour not found"),
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_tour_hotspot(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path((listing_id, _tour_id)): Path<(Uuid, Uuid)>,
    Json(_data): Json<CreateTourHotspot>,
) -> Result<Json<TourHotspot>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Creating tour hotspots"))
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
        (status = 404, description = "Listing or hotspot not found"),
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn delete_tour_hotspot(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path((listing_id, _tour_id, _hotspot_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Deleting tour hotspots"))
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
        (status = 404, description = "Listing not found"),
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_pricing_suggestion(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<PricingSuggestion>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Pricing suggestions"))
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
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn analyze_pricing(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
    Json(_data): Json<PricingAnalysisRequest>,
) -> Result<Json<PricingAnalysisResponse>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Pricing analysis"))
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
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_price_history(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<Vec<PriceHistory>>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Price history"))
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
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_neighborhood_insights(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<NeighborhoodInsightsResponse>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Neighborhood insights"))
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
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn refresh_neighborhood_insights(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
    Json(_data): Json<NeighborhoodInsightsRequest>,
) -> Result<Json<NeighborhoodInsightsResponse>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Refreshing neighborhood insights"))
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
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_nearby_amenities(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<Vec<NearbyAmenity>>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Nearby amenities"))
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
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_comparables(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
    Query(_params): Query<ComparablesRequest>,
) -> Result<Json<ComparablesResponse>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Comparable properties"))
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
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn refresh_comparables(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<ComparablesResponse>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Refreshing comparable properties"))
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
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_competitive_analysis(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<CompetitiveAnalysis>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Competitive analysis"))
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
        (status = 501, description = "Not implemented (no competitive-feature persistence)"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_competitive_status(
    State(state): State<AppState>,
    tenant: TenantExtractor,
    Path(listing_id): Path<Uuid>,
) -> Result<Json<CompetitiveFeaturesStatus>, (StatusCode, String)> {
    ensure_listing_in_org(&state, listing_id, &tenant).await?;
    Err(not_implemented("Competitive features status"))
}
